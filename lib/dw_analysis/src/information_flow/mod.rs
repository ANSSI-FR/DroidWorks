use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use dw_dex::instrs::Instr;
use dw_dex::registers::Reg;
use dw_dex::types::Type;
use dw_dex::{Addr, Dex, DexIndex};

pub mod amg;
pub mod errors;
pub mod signature;

use crate::controlflow::{Branch, Cfg};
use crate::dataflow::{self, AbstractForwardState, Dataflow};
use crate::errors::{AnalysisError, AnalysisResult};
use crate::information_flow::errors::FlowError;
use crate::typing::types::{AbstractType, JAVA_LANG_REFLECT_ARRAY};
use crate::{repo::*, typing};

pub type InformationFlows = Dataflow<State>;

impl InformationFlows {
    pub(crate) fn compute(
        method: &Method,
        class: &Class,
        context: &StateContext,
    ) -> AnalysisResult<Self> {
        dataflow::forward(method, class, context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    registers: Vec<amg::Flows>,
    signature: signature::Signature,
    last_exception: Option<amg::Flows>,
    last_result: Option<amg::Flows>,
    conditions: BTreeSet<amg::VertexHash>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.registers.len() {
            write!(f, "    v{}: {}", i, self.registers[i])?;
        }
        if let Some(v) = &self.last_exception {
            write!(f, "    last_exception: {v}")?;
        }
        if let Some(v) = &self.last_result {
            write!(f, "    last_result: {v}")?;
        }
        Ok(())
    }
}

impl State {
    fn read_reg(&self, r: Reg) -> AnalysisResult<&amg::Flows> {
        self.registers
            .get(r.value() as usize)
            .ok_or_else(|| FlowError::OutOfBoundsRegister(r).into())
    }

    fn read_pair(&self, r: Reg) -> AnalysisResult<&amg::Flows> {
        let t1 = self.read_reg(r)?;
        let t2 = self.read_reg(r.next())?;
        if t1 == t2 {
            Ok(t1)
        } else {
            Err(FlowError::BadPairTypes {
                type1: format!("{t1}"),
                type2: format!("{t2}"),
            }
            .into())
        }
    }

    fn write_reg(&mut self, r: Reg, t: amg::Flows) -> AnalysisResult<()> {
        self.registers
            .get_mut(r.value() as usize)
            .map(|rt| *rt = t)
            .ok_or_else(|| FlowError::OutOfBoundsRegister(r).into())
    }

    fn write_pair(&mut self, r: Reg, t: amg::Flows) -> AnalysisResult<()> {
        self.write_reg(r, t.clone())?;
        self.write_reg(r.next(), t)
    }

    pub(crate) fn signature(&self) -> &signature::Signature {
        &self.signature
    }
}

pub struct StateContext<'a> {
    method: MethodUid,
    repo: &'a Repo<'a>,
    cfg: Cfg<'a>,
    typecheck: &'a Dataflow<typing::State>,
    signatures: &'a BTreeMap<MethodUid, signature::Signature>,
    contexts: BTreeMap<Addr, BTreeSet<Addr>>,
}

impl<'a> StateContext<'a> {
    pub(crate) fn cfg(&self) -> &Cfg {
        &self.cfg
    }
}

impl<'a>
    TryFrom<(
        &'a Method<'a>,
        &'a Repo<'a>,
        &'a Dataflow<typing::State>,
        &'a BTreeMap<MethodUid, signature::Signature>,
    )> for StateContext<'a>
{
    type Error = AnalysisError;

    fn try_from(
        mrepts: (
            &'a Method,
            &'a Repo,
            &'a Dataflow<typing::State>,
            &'a BTreeMap<MethodUid, signature::Signature>,
        ),
    ) -> AnalysisResult<Self> {
        let (m, repo, typecheck, signatures) = mrepts;

        let cfg = Cfg::build(m)?;

        let reachables = cfg.reachables();
        let ipd = cfg.immediate_postdominators();

        let mut regions = BTreeMap::new();

        for (n, nreach) in reachables.iter() {
            let mut reg = nreach.clone();
            if let Some(ipd) = ipd.get(n).unwrap() {
                let ipdreach = reachables.get(ipd).unwrap();
                reg.retain(|i| !ipdreach.contains(i));
            }
            regions.insert(n, reg);
        }

        let mut contexts = BTreeMap::new();
        for i in cfg.inner.node_indices() {
            let mut cxt = BTreeSet::new();
            for j in cfg.inner.node_indices() {
                let jreg = regions.get(&j).unwrap();
                if i != j && jreg.contains(&i) {
                    let jblock = cfg.inner.node_weight(j).unwrap();
                    cxt.insert(jblock.start_addr());
                }
            }
            let iblock = cfg.inner.node_weight(i).unwrap();
            contexts.insert(iblock.start_addr(), cxt);
        }

        Ok(Self {
            method: m.uid(),
            repo,
            cfg,
            typecheck,
            signatures,
            contexts,
        })
    }
}

impl<'a> fmt::Display for StateContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.method)
    }
}

impl<'a> AbstractForwardState<'a> for State {
    type Context = StateContext<'a>;
    type Error = AnalysisError;

    fn init(method: &Method, _class: &Class, context: &'a StateContext) -> AnalysisResult<Self> {
        let nb_registers = method.code().as_ref().unwrap().borrow().registers_size();

        let mut registers = Vec::with_capacity(nb_registers);
        for _ in 0..nb_registers {
            registers.push(amg::Flows::default());
        }

        let nb_param_registers: usize = method
            .parameters_types()
            .iter()
            .map(|t| match t {
                Type::Long | Type::Double => 2,
                _ => 1,
            })
            .sum();
        let first_param_reg = nb_registers - nb_param_registers;

        let mut param_count = 0;

        let mut amg = amg::Amg::default();

        if !method.is_static() {
            let param = amg::Vertex::parameter(context.method, param_count);
            let param = amg.add_vertex(param);
            registers[first_param_reg - 1] =
                amg::Flows::from(amg::Flow::new(param, amg::FlowType::Explicit));
            param_count += 1;
        }

        let mut param_reg = first_param_reg;

        for type_descr in method.parameters_types() {
            let param = amg::Vertex::parameter(context.method, param_count);
            let param = amg.add_vertex(param);
            let flow = amg::Flows::from(amg::Flow::new(param, amg::FlowType::Explicit));
            match type_descr {
                Type::Long | Type::Double => {
                    registers[param_reg] = flow.clone();
                    param_reg += 1;
                    registers[param_reg] = flow;
                    param_reg += 1;
                }
                _ => {
                    registers[param_reg] = flow;
                    param_reg += 1;
                }
            }
            param_count += 1;
        }

        Ok(Self {
            registers,
            signature: signature::Signature::new(amg),
            last_exception: None,
            last_result: None,
            conditions: BTreeSet::new(),
        })
    }

    fn join(&mut self, other: &Self, _context: &StateContext) -> AnalysisResult<()> {
        if self.registers.len() != other.registers.len() {
            return Err(FlowError::IncompatibleStates.into());
        }
        for i in 0..self.registers.len() {
            self.registers[i] = self.registers[i].join(&other.registers[i]);
        }

        self.last_exception = match (&self.last_exception, &other.last_exception) {
            (Some(f1), Some(f2)) => Some(f1.join(f2)),
            _ => None,
        };

        self.last_result = match (&self.last_result, &other.last_result) {
            (Some(f1), Some(f2)) => Some(f1.join(f2)),
            _ => None,
        };

        self.signature.join(&other.signature);

        Ok(())
    }

    fn transfer_branch(&mut self, branch: &Branch, _context: &StateContext) -> AnalysisResult<()> {
        match branch {
            Branch::IfTrue(_, _, _)
            | Branch::IfFalse(_, _, _)
            | Branch::Switch(_, _)
            | Branch::SwitchDefault
            | Branch::Jmp
            | Branch::Sequence
            | Branch::ArrayAccessSuccess
            | Branch::InvokeSuccess
            | Branch::DivSuccess
            | Branch::Catch(_)
            | Branch::CatchAll
            | Branch::CastSuccess(_, _) => Ok(()),
            //_ => unimplemented!("information flow for branch {branch:?}"),
        }
    }

    fn transfer_instr(
        &mut self,
        pc: Addr,
        instr: &Instr,
        dex: &Dex,
        context: &'a StateContext,
    ) -> AnalysisResult<()> {
        // save last status registers, and reset them to the 'default' value (None)
        let last_exception = std::mem::replace(&mut self.last_exception, None);
        let last_result = std::mem::replace(&mut self.last_result, None);

        match instr {
            Instr::Nop | Instr::Goto(_) | Instr::Goto16(_) | Instr::Goto32(_) => Ok(()),

            Instr::Move(dst, src) | Instr::MoveFrom16(dst, src) | Instr::Move16(dst, src) => {
                let mut f = self.read_reg(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }

            Instr::MoveWide(dst, src)
            | Instr::MoveWideFrom16(dst, src)
            | Instr::MoveWide16(dst, src) => {
                let mut f = self.read_pair(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_pair(*dst, f)
            }

            Instr::MoveObject(dst, src)
            | Instr::MoveObjectFrom16(dst, src)
            | Instr::MoveObject16(dst, src) => {
                let mut f = self.read_reg(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }

            Instr::MoveResult(dst) => match last_result {
                Some(mut f) => {
                    f.apply_context(&self.conditions);
                    self.write_reg(*dst, f)
                }
                _ => Err(FlowError::MissingResult.into()),
            },
            Instr::MoveResultWide(dst) => match last_result {
                Some(mut f) => {
                    f.apply_context(&self.conditions);
                    self.write_pair(*dst, f)
                }
                _ => Err(FlowError::MissingResult.into()),
            },
            Instr::MoveResultObject(dst) => match last_result {
                Some(mut f) => {
                    f.apply_context(&self.conditions);
                    self.write_reg(*dst, f)
                }
                _ => Err(FlowError::MissingResult.into()),
            },
            Instr::MoveException(dst) => match last_exception {
                Some(mut f) => {
                    f.apply_context(&self.conditions);
                    self.write_reg(*dst, f)
                }
                _ => Err(FlowError::MissingException.into()),
            },

            Instr::ReturnVoid => {
                let mut ret_flows = amg::Flows::default();
                ret_flows.apply_context(&self.conditions);
                self.signature.join_return(&ret_flows);
                Ok(())
            }
            Instr::Return(ret) | Instr::ReturnObject(ret) => {
                let mut ret_flows = self.read_reg(*ret)?.clone();
                ret_flows.apply_context(&self.conditions);
                self.signature.join_return(&ret_flows);
                Ok(())
            }
            Instr::ReturnWide(ret) => {
                let mut ret_flows = self.read_pair(*ret)?.clone();
                ret_flows.apply_context(&self.conditions);
                self.signature.join_return(&ret_flows);
                Ok(())
            }

            Instr::Const4(dst, val) => {
                let cst = amg::Vertex::constant(context.method, pc);
                let cst = self.signature.amg_mut().add_vertex(cst);
                let mut f = amg::Flows::default();
                f.add(amg::Flow::new(cst, amg::FlowType::Explicit));
                if *val == 0 {
                    let null = amg::Vertex::null();
                    let null = self.signature.amg_mut().add_vertex(null);
                    f.add(amg::Flow::new(null, amg::FlowType::Explicit));
                }
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::Const16(dst, val) | Instr::ConstHigh16(dst, val) => {
                let cst = amg::Vertex::constant(context.method, pc);
                let cst = self.signature.amg_mut().add_vertex(cst);
                let mut f = amg::Flows::default();
                f.add(amg::Flow::new(cst, amg::FlowType::Explicit));
                if *val == 0 {
                    let null = amg::Vertex::null();
                    let null = self.signature.amg_mut().add_vertex(null);
                    f.add(amg::Flow::new(null, amg::FlowType::Explicit));
                }
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::Const(dst, val) => {
                let cst = amg::Vertex::constant(context.method, pc);
                let cst = self.signature.amg_mut().add_vertex(cst);
                let mut f = amg::Flows::default();
                f.add(amg::Flow::new(cst, amg::FlowType::Explicit));
                if *val == 0 {
                    let null = amg::Vertex::null();
                    let null = self.signature.amg_mut().add_vertex(null);
                    f.add(amg::Flow::new(null, amg::FlowType::Explicit));
                }
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::ConstString(dst, _)
            | Instr::ConstStringJumbo(dst, _)
            | Instr::ConstClass(dst, _) => {
                let cst = amg::Vertex::constant(context.method, pc);
                let cst = self.signature.amg_mut().add_vertex(cst);
                let mut f = amg::Flows::default();
                f.add(amg::Flow::new(cst, amg::FlowType::Explicit));
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }

            Instr::ConstWide16(dst, _)
            | Instr::ConstWideHigh16(dst, _)
            | Instr::ConstWide32(dst, _)
            | Instr::ConstWide(dst, _) => {
                let cst = amg::Vertex::constant(context.method, pc);
                let cst = self.signature.amg_mut().add_vertex(cst);
                let mut f = amg::Flows::from(amg::Flow::new(cst, amg::FlowType::Explicit));
                f.apply_context(&self.conditions);
                self.write_pair(*dst, f)
            }

            Instr::MonitorEnter(_ptr) | Instr::MonitorExit(_ptr) => Ok(()),
            Instr::CheckCast(ptr, _cls) => {
                let mut ptr_f = self.read_reg(*ptr)?.clone();
                ptr_f.apply_context(&self.conditions);
                self.write_reg(*ptr, ptr_f)
            }
            /*Instr::InstanceOf(dst, ptr, cls) => {
                let cls_typ = AbstractType::try_from(WithDex::new(dex, *cls))?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(cls_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                tc!(ptr_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                self.write_reg(*dst, Integer)
            }*/
            Instr::ArrayLength(dst, ptr) => {
                let mut f = self.read_reg(*ptr)?.clone().into_implicit();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }

            Instr::NewInstance(dst, _descr) => {
                let alloc = amg::Vertex::instance(context.method, pc);
                let alloc = self.signature.amg_mut().add_vertex(alloc);

                /*
                let class = descr.get(dex)?.to_type(dex)?;
                let class = context.repo.get_class_by_name(&class.to_definer_name()?).ok_or(FlowError::UnknownClass)?;
                for field in class.iter_fields() {
                    let fdescr = field.descriptor();
                    let fname = field.name().to_string();
                    let field = amg::Vertex::field(context.descriptor.clone(), pc, fname.clone());
                    let field = self.signature.amg_mut().add_vertex(field);
                    if fdescr.type_().is_object() {
                        let null = amg::Vertex::null();
                        let null = self.signature.amg_mut().add_vertex(null);
                        self.signature.amg_mut().add_edge(&field, &null, amg::Edge::field(fname.clone(), amg::FlowType::Explicit))?;
                    }
                    self.signature.amg_mut().add_edge(&alloc, &field, amg::Edge::field(fname, amg::FlowType::Explicit))?;
                }
                */

                let mut f = amg::Flows::from(amg::Flow::new(alloc, amg::FlowType::Explicit));
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::NewArray(dst, siz, _arr) => {
                let alloc = amg::Vertex::instance(context.method, pc);
                let alloc = self.signature.amg_mut().add_vertex(alloc);
                let siz_f = self.read_reg(*siz)?;
                let mut f = siz_f.clone().into_implicit();
                f.add(amg::Flow::new(alloc, amg::FlowType::Explicit));
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }

            /*
            Instr::FilledNewArray(args, arr) => {
                let array_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                match &array_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &JoinZero ; repo)?;
                        for arg in args.iter() {
                            let arg_typ = self.read_reg(arg)?;
                            tc!(arg_typ <: elt_typ ; repo)?;
                        }
                        self.last_result = Some(array_typ);
                        Ok(())
                    }
                    _ if array_typ.subseteq(&Null, repo)? => Ok(()),
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            Instr::FilledNewArrayRange(args, arr) => {
                let array_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                match &array_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &JoinZero ; repo)?;
                        for arg in args.iter() {
                            let arg_typ = self.read_reg(arg)?;
                            tc!(arg_typ <: elt_typ ; repo)?;
                        }
                        self.last_result = Some(array_typ);
                        Ok(())
                    }
                    _ if array_typ.subseteq(&Null, repo)? => Ok(()),
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            Instr::FillArrayData(arr, _) => {
                let arr_typ = self.read_reg(*arr)?;
                match &arr_typ {
                    Array(1, _) => Ok(()),
                    _ if arr_typ.subseteq(&Null, repo)? => Ok(()),
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            */
            Instr::Throw(ptr) => {
                let mut ptr_f = self.read_reg(*ptr)?.clone();
                ptr_f.apply_context(&self.conditions);
                self.last_exception = Some(ptr_f);
                // TODO: join signature throw_flows ?
                Ok(())
            }

            /*
            Instr::PackedSwitch(src, _) | Instr::SparseSwitch(src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)
            }
             */
            Instr::CmplFloat(dst, src1, src2) | Instr::CmpgFloat(dst, src1, src2) => {
                let src_if1 = self.read_reg(*src1)?;
                let src_if2 = self.read_reg(*src2)?;
                let mut dst_if = src_if1.join(src_if2);
                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }
            Instr::CmplDouble(dst, src1, src2)
            | Instr::CmpgDouble(dst, src1, src2)
            | Instr::CmpLong(dst, src1, src2) => {
                let src_if1 = self.read_pair(*src1)?;
                let src_if2 = self.read_pair(*src2)?;
                let mut dst_if = src_if1.join(src_if2);
                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }

            Instr::IfEq(_, _, _)
            | Instr::IfNe(_, _, _)
            | Instr::IfLt(_, _, _)
            | Instr::IfGe(_, _, _)
            | Instr::IfGt(_, _, _)
            | Instr::IfLe(_, _, _)
            | Instr::IfEqz(_, _)
            | Instr::IfNez(_, _)
            | Instr::IfLtz(_, _)
            | Instr::IfGez(_, _)
            | Instr::IfGtz(_, _)
            | Instr::IfLez(_, _) => Ok(()),

            /*
                Instr::Aget(dst, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    match arr_typ {
                        Array(1, elt_typ) if elt_typ.as_ref() == &Integer => {
                            self.write_reg(*dst, Integer)
                        }
                        Array(1, elt_typ) if elt_typ.as_ref() == &Float => self.write_reg(*dst, Float),
                        Null => self.write_reg(*dst, Integer),
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AgetBoolean(dst, arr, idx)
                | Instr::AgetByte(dst, arr, idx)
                | Instr::AgetChar(dst, arr, idx)
                | Instr::AgetShort(dst, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    match arr_typ {
                        Array(1, elt_typ) if elt_typ.as_ref() == &Integer => {
                            self.write_reg(*dst, Integer)
                        }
                        Null => self.write_reg(*dst, Integer),
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AgetWide(dst, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    match arr_typ {
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &Join64 ; repo)?;
                            self.write_pair(*dst, *elt_typ.clone())
                        }
                        _ if arr_typ.subseteq(&Null, repo)? => self.write_pair(*dst, Meet64),
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AgetObject(dst, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    match arr_typ.clone() {
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                            self.write_reg(*dst, *elt_typ)
                        }
                        Array(n, elt_typ) => {
                            assert!(n > 1);
                            self.write_reg(*dst, Array(n - 1, elt_typ))
                        }
                        _ if arr_typ.subseteq(&Null, repo)? => self.write_reg(*dst, Null),
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::Aput(src, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    let src_typ = self.read_reg(*src)?;
                    match &arr_typ {
                        Null => {
                            tc!(src_typ <: &Join32 ; repo)
                        }
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &Join32 ; repo)?;
                            tc!(src_typ <: elt_typ ; repo)
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AputBoolean(src, arr, idx)
                | Instr::AputByte(src, arr, idx)
                | Instr::AputChar(src, arr, idx)
                | Instr::AputShort(src, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    let src_typ = self.read_reg(*src)?;
                    match &arr_typ {
                        Null => {
                            tc!(src_typ <: &Integer ; repo)
                        }
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &Integer ; repo)?;
                            tc!(src_typ <: elt_typ ; repo)
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AputWide(src, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    let arr_typ = self.read_reg(*arr)?;
                    let src_typ = self.read_pair(*src)?;
                    match &arr_typ {
                        Null => {
                            tc!(src_typ <: &Join64 ; repo)
                        }
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &Join64 ; repo)?;
                            tc!(src_typ <: elt_typ ; repo)
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
                Instr::AputObject(src, arr, idx) => {
                    let idx_typ = self.read_reg(*idx)?;
                    let arr_typ = self.read_reg(*arr)?;
                    let src_typ = self.read_reg(*src)?;
                    tc!(idx_typ <: &Integer ; repo)?;
                    match &arr_typ {
                        Null => {
                            tc!(src_typ <: &*JAVA_LANG_OBJECT ; repo)
                        }
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                            tc!(src_typ <: elt_typ ; repo)
                        }
                        Array(n, elt_typ) => {
                            assert!(n > &1);
                            tc!(src_typ <: &Array(n - 1, elt_typ.clone()) ; repo)
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
            }*/
            Instr::Iget(dst, ptr, field)
            | Instr::IgetBoolean(dst, ptr, field)
            | Instr::IgetByte(dst, ptr, field)
            | Instr::IgetChar(dst, ptr, field)
            | Instr::IgetShort(dst, ptr, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let ptr_if = self.read_reg(*ptr)?.clone();
                let (mut ptr_if_e, ptr_if_i) = ptr_if.split();
                ptr_if_e.retain(|f| f.vertex_hash() != null);

                let mut dst_if = ptr_if_i;

                for ptr in ptr_if_e {
                    let ptr_hash = ptr.vertex_hash();
                    let ptr_vertex = self.signature.amg().vertex(ptr_hash);
                    let fnode = ptr_vertex.field(field_uid)?;
                    let fnode = self.signature.amg_mut().add_vertex(fnode);

                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        fnode,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );

                    for (edgew, succ) in self.signature.amg().successors(ptr_hash) {
                        for edge in edgew {
                            if edge.field_uid() == field_uid {
                                dst_if.add(amg::Flow::new(succ, ptr.flow_type()));
                            }
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }
            Instr::IgetWide(dst, ptr, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let ptr_if = self.read_reg(*ptr)?.clone();
                let (mut ptr_if_e, ptr_if_i) = ptr_if.split();
                ptr_if_e.retain(|f| f.vertex_hash() != null);

                let mut dst_if = ptr_if_i;

                for ptr in ptr_if_e {
                    let ptr_hash = ptr.vertex_hash();
                    let ptr_vertex = self.signature.amg().vertex(ptr_hash);
                    let fnode = ptr_vertex.field(field_uid)?;
                    let fnode = self.signature.amg_mut().add_vertex(fnode);

                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        fnode,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );

                    for (edgew, succ) in self.signature.amg().successors(ptr_hash) {
                        for edge in edgew {
                            if edge.field_uid() == field_uid {
                                dst_if.add(amg::Flow::new(succ, ptr.flow_type()));
                            }
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_pair(*dst, dst_if)
            }
            Instr::IgetObject(dst, ptr, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let ptr_if = self.read_reg(*ptr)?.clone();
                let (mut ptr_if_e, ptr_if_i) = ptr_if.split();
                ptr_if_e.retain(|f| f.vertex_hash() != null);

                let mut dst_if = ptr_if_i;

                for ptr in ptr_if_e {
                    let ptr_hash = ptr.vertex_hash();
                    let ptr_vertex = self.signature.amg().vertex(ptr_hash);
                    let fnode = ptr_vertex.field(field_uid)?;
                    let fnode = self.signature.amg_mut().add_vertex(fnode);

                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        fnode,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );

                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        null,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );

                    for (edgew, succ) in self.signature.amg().successors(ptr_hash) {
                        for edge in edgew {
                            if edge.field_uid() == field_uid {
                                dst_if.add(amg::Flow::new(succ, ptr.flow_type()));
                            }
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }

            Instr::Iput(src, ptr, field)
            | Instr::IputBoolean(src, ptr, field)
            | Instr::IputByte(src, ptr, field)
            | Instr::IputChar(src, ptr, field)
            | Instr::IputShort(src, ptr, field)
            | Instr::IputObject(src, ptr, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let ptr_if = self.read_reg(*ptr)?.clone();
                let (mut ptr_if_e, ptr_if_i) = ptr_if.split();
                ptr_if_e.retain(|f| f.vertex_hash() != null);

                let src_if = self.read_reg(*src)?.clone();

                for ptr in ptr_if_e {
                    let ptr_hash = ptr.vertex_hash();
                    let ptr_vertex = self.signature.amg().vertex(ptr_hash);
                    let ptr_field = ptr_vertex.field(field_uid)?;
                    let ptr_field_hash = self.signature.amg_mut().add_vertex(ptr_field);
                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        ptr_field_hash,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );
                    for srcf in &src_if {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            srcf.vertex_hash(),
                            amg::Edge::field(field_uid, srcf.flow_type()),
                        );
                    }
                    for srcf in &ptr_if_i {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            srcf.vertex_hash(),
                            amg::Edge::field(field_uid, amg::FlowType::Implicit),
                        );
                    }
                    for srchash in &self.conditions {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            *srchash,
                            amg::Edge::field(field_uid, amg::FlowType::Implicit),
                        );
                    }
                }

                Ok(())
            }
            Instr::IputWide(src, ptr, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let ptr_if = self.read_reg(*ptr)?.clone();
                let (mut ptr_if_e, ptr_if_i) = ptr_if.split();
                ptr_if_e.retain(|f| f.vertex_hash() != null);

                let src_if = self.read_pair(*src)?.clone();

                for ptr in ptr_if_e {
                    let ptr_hash = ptr.vertex_hash();
                    let ptr_vertex = self.signature.amg().vertex(ptr_hash);
                    let ptr_field = ptr_vertex.field(field_uid)?;
                    let ptr_field_hash = self.signature.amg_mut().add_vertex(ptr_field);
                    self.signature.amg_mut().add_edge(
                        ptr_hash,
                        ptr_field_hash,
                        amg::Edge::field(field_uid, amg::FlowType::Explicit),
                    );
                    for srcf in &src_if {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            srcf.vertex_hash(),
                            amg::Edge::field(field_uid, srcf.flow_type()),
                        );
                    }
                    for srcf in &ptr_if_i {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            srcf.vertex_hash(),
                            amg::Edge::field(field_uid, amg::FlowType::Implicit),
                        );
                    }
                    for srchash in &self.conditions {
                        self.signature.amg_mut().add_edge(
                            ptr_hash,
                            *srchash,
                            amg::Edge::field(field_uid, amg::FlowType::Implicit),
                        );
                    }
                }

                Ok(())
            }

            Instr::Sget(dst, field)
            | Instr::SgetBoolean(dst, field)
            | Instr::SgetByte(dst, field)
            | Instr::SgetChar(dst, field)
            | Instr::SgetShort(dst, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_static_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let class = context
                    .repo
                    .get_class_by_name(field.descriptor().class_name())
                    .ok_or(FlowError::UnknownClass)?;
                let static_vertex = amg::Vertex::static_(class.uid());
                let fnode = static_vertex.field(field_uid)?;
                let static_hash = self.signature.amg_mut().add_vertex(static_vertex);
                let fnode = self.signature.amg_mut().add_vertex(fnode);

                self.signature.amg_mut().add_edge(
                    static_hash,
                    fnode,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                let mut dst_if = amg::Flows::default();

                for (edgew, succ) in self.signature.amg().successors(static_hash) {
                    for edge in edgew {
                        if edge.field_uid() == field_uid {
                            dst_if.add(amg::Flow::new(succ, edge.flow_type()));
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }
            Instr::SgetWide(dst, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_static_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let class = context
                    .repo
                    .get_class_by_name(field.descriptor().class_name())
                    .ok_or(FlowError::UnknownClass)?;
                let static_vertex = amg::Vertex::static_(class.uid());
                let fnode = static_vertex.field(field_uid)?;
                let static_hash = self.signature.amg_mut().add_vertex(static_vertex);
                let fnode = self.signature.amg_mut().add_vertex(fnode);

                self.signature.amg_mut().add_edge(
                    static_hash,
                    fnode,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                let mut dst_if = amg::Flows::default();

                for (edgew, succ) in self.signature.amg().successors(static_hash) {
                    for edge in edgew {
                        if edge.field_uid() == field_uid {
                            dst_if.add(amg::Flow::new(succ, edge.flow_type()));
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_pair(*dst, dst_if)
            }
            Instr::SgetObject(dst, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_static_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let class = context
                    .repo
                    .get_class_by_name(field.descriptor().class_name())
                    .ok_or(FlowError::UnknownClass)?;
                let static_vertex = amg::Vertex::static_(class.uid());
                let fnode = static_vertex.field(field_uid)?;
                let static_hash = self.signature.amg_mut().add_vertex(static_vertex);
                let fnode = self.signature.amg_mut().add_vertex(fnode);

                self.signature.amg_mut().add_edge(
                    static_hash,
                    fnode,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                self.signature.amg_mut().add_edge(
                    static_hash,
                    null,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                let mut dst_if = amg::Flows::default();

                for (edgew, succ) in self.signature.amg().successors(static_hash) {
                    for edge in edgew {
                        if edge.field_uid() == field_uid {
                            dst_if.add(amg::Flow::new(succ, edge.flow_type()));
                        }
                    }
                }

                dst_if.apply_context(&self.conditions);
                self.write_reg(*dst, dst_if)
            }

            Instr::Sput(src, field)
            | Instr::SputBoolean(src, field)
            | Instr::SputByte(src, field)
            | Instr::SputChar(src, field)
            | Instr::SputShort(src, field)
            | Instr::SputObject(src, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let class = context
                    .repo
                    .get_class_by_name(field.descriptor().class_name())
                    .ok_or(FlowError::UnknownClass)?;
                let static_vertex = amg::Vertex::static_(class.uid());
                let fnode = static_vertex.field(field_uid)?;
                let static_hash = self.signature.amg_mut().add_vertex(static_vertex);
                let fnode = self.signature.amg_mut().add_vertex(fnode);

                self.signature.amg_mut().add_edge(
                    static_hash,
                    fnode,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                let src_if = self.read_reg(*src)?.clone();

                for srcf in &src_if {
                    self.signature.amg_mut().add_edge(
                        static_hash,
                        srcf.vertex_hash(),
                        amg::Edge::field(field_uid, srcf.flow_type()),
                    );
                }
                for srchash in &self.conditions {
                    self.signature.amg_mut().add_edge(
                        static_hash,
                        *srchash,
                        amg::Edge::field(field_uid, amg::FlowType::Implicit),
                    );
                }

                Ok(())
            }
            Instr::SputWide(src, field) => {
                let field = field.get(dex)?;
                let fname = field.name(dex)?;
                let ftype = field.type_(dex)?;
                let field =
                    context
                        .repo
                        .lookup_instance_field(&fname, &ftype, &field.class_name(dex)?)?;
                let field_uid = field.uid();

                let class = context
                    .repo
                    .get_class_by_name(field.descriptor().class_name())
                    .ok_or(FlowError::UnknownClass)?;
                let static_vertex = amg::Vertex::static_(class.uid());
                let fnode = static_vertex.field(field_uid)?;
                let static_hash = self.signature.amg_mut().add_vertex(static_vertex);
                let fnode = self.signature.amg_mut().add_vertex(fnode);

                self.signature.amg_mut().add_edge(
                    static_hash,
                    fnode,
                    amg::Edge::field(field_uid, amg::FlowType::Explicit),
                );

                let src_if = self.read_pair(*src)?.clone();

                for srcf in &src_if {
                    self.signature.amg_mut().add_edge(
                        static_hash,
                        srcf.vertex_hash(),
                        amg::Edge::field(field_uid, srcf.flow_type()),
                    );
                }
                for srchash in &self.conditions {
                    self.signature.amg_mut().add_edge(
                        static_hash,
                        *srchash,
                        amg::Edge::field(field_uid, amg::FlowType::Implicit),
                    );
                }

                Ok(())
            }

            Instr::InvokeVirtual(args, meth) | Instr::InvokeInterface(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let tc_lubs = &context.typecheck.entries[&pc];
                let this_lub = tc_lubs.read_reg(this_reg)?;
                let this_lubs = match this_lub {
                    AbstractType::Object(objs) => objs,
                    AbstractType::Array(_, _) => match &*JAVA_LANG_REFLECT_ARRAY {
                        AbstractType::Object(obj) => obj,
                        _ => unreachable!(),
                    },
                    _ => unimplemented!("this is not an object: {this_lub}"),
                };

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow);
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let mut result_flows = amg::Flows::default();

                let meth_uids = context.repo.lookup_virtual_call(&meth, this_lubs)?;
                log::debug!("{} candidate implementations to inject", meth_uids.len());
                if meth_uids.is_empty() {
                    log::warn!("no candidate implementations found for {meth}");
                }
                for meth_uid in meth_uids {
                    let meth = &context.repo[meth_uid];
                    match context.signatures.get(&meth_uid) {
                        None => {
                            if meth_ret != &Type::Void {
                                log::error!("no signature for {meth_uid} = {}", meth.descriptor());
                            } else {
                                log::warn!(
                                    "missing signature for {meth_uid} = {}",
                                    meth.descriptor()
                                );
                            }
                        }
                        Some(sig) => {
                            /*
                            log::info!(
                                "\nCURRENT AMG =\n{}",
                                self.signature.amg().to_dot(context.repo)?
                            );
                            log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                            */

                            let mut out = self.signature.inject(sig, param_flows.clone())?;
                            if !matches!(meth_ret, Type::Void) {
                                out.return_flows.apply_context(&self.conditions);
                                result_flows = result_flows.join(&out.return_flows);
                            }
                        } // TODO: throw_flows ?
                    };
                }

                if matches!(meth_ret, Type::Void) {
                    self.last_result = None;
                } else {
                    self.last_result = Some(result_flows);
                }

                Ok(())
            }
            Instr::InvokeSuper(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let tc_lubs = &context.typecheck.entries[&pc];
                let this_lub = tc_lubs.read_reg(this_reg)?;
                let this_lubs = match this_lub {
                    AbstractType::Object(objs) => objs,
                    AbstractType::Array(_, _) => match &*JAVA_LANG_REFLECT_ARRAY {
                        AbstractType::Object(obj) => obj,
                        _ => unreachable!(),
                    },
                    _ => unimplemented!("this is not an object: {this_lub}"),
                };

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let mut result_flows = amg::Flows::default();

                let meth_uids = context.repo.lookup_super_call(&meth, this_lubs)?;
                log::debug!("{} candidate implementations to inject", meth_uids.len());
                if meth_uids.is_empty() {
                    log::warn!("no candidate implementation found for {meth}");
                }
                for meth_uid in meth_uids {
                    let meth = &context.repo[meth_uid];
                    match context.signatures.get(&meth_uid) {
                        None => {
                            if meth_ret != &Type::Void {
                                log::error!("no signature for {meth_uid} = {}", meth.descriptor());
                            } else {
                                log::warn!(
                                    "missing signature for {meth_uid} = {}",
                                    meth.descriptor()
                                );
                            }
                        }
                        Some(sig) => {
                            /*
                            log::info!(
                                "\nCURRENT AMG =\n{}",
                                self.signature.amg().to_dot(context.repo)?
                            );
                            log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                            */

                            let mut out = self.signature.inject(sig, param_flows.clone())?;
                            if !matches!(meth_ret, Type::Void) {
                                out.return_flows.apply_context(&self.conditions);
                                result_flows = result_flows.join(&out.return_flows);
                            }
                        } // TODO: throw_flows ?
                    };
                }

                if matches!(meth_ret, Type::Void) {
                    self.last_result = None;
                } else {
                    self.last_result = Some(result_flows);
                }

                Ok(())
            }
            Instr::InvokeDirect(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;
                let meth_uid = context
                    .repo
                    .find_method_by_descriptor(&meth)
                    .ok_or(FlowError::UnknownClass)?
                    .uid();

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let result_flows = match context.signatures.get(&meth_uid) {
                    None => {
                        if meth_ret != &Type::Void {
                            log::error!("no signature for {meth_uid} = {}", meth);
                        } else {
                            log::warn!("missing signature for {meth_uid} = {}", meth);
                        }
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            Some(amg::Flows::default())
                        }
                    }
                    Some(sig) => {
                        /*
                        log::info!(
                            "\nCURRENT AMG =\n{}",
                            self.signature.amg().to_dot(context.repo)?
                        );
                        log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                        */
                        let mut out = self.signature.inject(sig, param_flows)?;
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            out.return_flows.apply_context(&self.conditions);
                            Some(out.return_flows)
                        }
                    } // TODO: throw_flows ?
                };

                self.last_result = result_flows;

                Ok(())
            }
            Instr::InvokeStatic(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                let mut args_it = args.iter();

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;
                let meth_uid = context
                    .repo
                    .find_method_by_descriptor(&meth)
                    .ok_or(FlowError::UnknownClass)?
                    .uid();

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let result_flows = match context.signatures.get(&meth_uid) {
                    None => {
                        if meth_ret != &Type::Void {
                            log::error!("no signature for {meth_uid} = {}", meth);
                        } else {
                            log::warn!("missing signature for {meth_uid} = {}", meth);
                        }
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            Some(amg::Flows::default())
                        }
                    }
                    Some(sig) => {
                        /*
                        log::info!(
                            "\nCURRENT AMG =\n{}",
                            self.signature.amg().to_dot(context.repo)?
                        );
                        log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                        */
                        let mut out = self.signature.inject(sig, param_flows)?;
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            out.return_flows.apply_context(&self.conditions);
                            Some(out.return_flows)
                        }
                    } // TODO: throw_flows ?
                };

                self.last_result = result_flows;

                Ok(())
            }

            Instr::InvokeVirtualRange(args, meth) | Instr::InvokeInterfaceRange(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let tc_lubs = &context.typecheck.entries[&pc];
                let this_lub = tc_lubs.read_reg(this_reg)?;
                let this_lubs = match this_lub {
                    AbstractType::Object(objs) => objs,
                    AbstractType::Array(_, _) => match &*JAVA_LANG_REFLECT_ARRAY {
                        AbstractType::Object(obj) => obj,
                        _ => unreachable!(),
                    },
                    _ => unimplemented!("this is not an object: {this_lub}"),
                };

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let mut result_flows = amg::Flows::default();

                let meth_uids = context.repo.lookup_virtual_call(&meth, this_lubs)?;
                log::debug!("{} candidate implementations to inject", meth_uids.len());
                if meth_uids.is_empty() {
                    log::warn!("no candidate implementations found for {meth}");
                }
                for meth_uid in meth_uids {
                    let meth = &context.repo[meth_uid];
                    match context.signatures.get(&meth_uid) {
                        None => {
                            if meth_ret != &Type::Void {
                                log::error!("no signature for {meth_uid} = {}", meth.descriptor());
                            } else {
                                log::warn!(
                                    "missing signature for {meth_uid} = {}",
                                    meth.descriptor()
                                );
                            }
                        }
                        Some(sig) => {
                            /*
                            log::info!(
                                "\nCURRENT AMG =\n{}",
                                self.signature.amg().to_dot(context.repo)?
                            );
                            log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                            */
                            let mut out = self.signature.inject(sig, param_flows.clone())?;
                            if !matches!(meth_ret, Type::Void) {
                                out.return_flows.apply_context(&self.conditions);
                                result_flows = result_flows.join(&out.return_flows);
                            }
                        } // TODO: throw_flows ?
                    };
                }

                if matches!(meth_ret, Type::Void) {
                    self.last_result = None;
                } else {
                    self.last_result = Some(result_flows);
                }

                Ok(())
            }
            Instr::InvokeSuperRange(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let tc_lubs = &context.typecheck.entries[&pc];
                let this_lub = tc_lubs.read_reg(this_reg)?;
                let this_lubs = match this_lub {
                    AbstractType::Object(objs) => objs,
                    AbstractType::Array(_, _) => match &*JAVA_LANG_REFLECT_ARRAY {
                        AbstractType::Object(obj) => obj,
                        _ => unreachable!(),
                    },
                    _ => unimplemented!("this is not an object: {this_lub}"),
                };

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let mut result_flows = amg::Flows::default();

                let meth_uids = context.repo.lookup_super_call(&meth, this_lubs)?;
                log::debug!("{} candidate implementations to inject", meth_uids.len());
                if meth_uids.is_empty() {
                    log::warn!("no candidate implementations found for {meth}");
                }
                for meth_uid in meth_uids {
                    let meth = &context.repo[meth_uid];
                    match context.signatures.get(&meth_uid) {
                        None => {
                            if meth_ret != &Type::Void {
                                log::error!("no signature for {meth_uid} = {}", meth.descriptor());
                            } else {
                                log::warn!(
                                    "missing signature for {meth_uid} = {}",
                                    meth.descriptor()
                                );
                            }
                        }
                        Some(sig) => {
                            /*
                            log::info!(
                                "\nCURRENT AMG =\n{}",
                                self.signature.amg().to_dot(context.repo)?
                            );
                            log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                            */

                            let mut out = self.signature.inject(sig, param_flows.clone())?;
                            if !matches!(meth_ret, Type::Void) {
                                out.return_flows.apply_context(&self.conditions);
                                result_flows = result_flows.join(&out.return_flows);
                            }
                        } // TODO: throw_flows ?
                    };
                }

                if matches!(meth_ret, Type::Void) {
                    self.last_result = None;
                } else {
                    self.last_result = Some(result_flows);
                }

                Ok(())
            }
            Instr::InvokeDirectRange(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                // 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(FlowError::MissingThisArgument)?;
                let mut this_reg_flow = self.read_reg(this_reg)?.clone();
                this_reg_flow.retain(|f| f.vertex_hash() != null);
                param_flows.push(this_reg_flow);

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;
                let meth_uid = context
                    .repo
                    .find_method_by_descriptor(&meth)
                    .ok_or(FlowError::UnknownClass)?
                    .uid();

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let result_flows = match context.signatures.get(&meth_uid) {
                    None => {
                        if meth_ret != &Type::Void {
                            log::error!("no signature for {meth_uid} = {}", meth);
                        } else {
                            log::warn!("missing signature for {meth_uid} = {}", meth);
                        }
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            Some(amg::Flows::default())
                        }
                    }
                    Some(sig) => {
                        /*
                        log::info!(
                            "\nCURRENT AMG =\n{}",
                            self.signature.amg().to_dot(context.repo)?
                        );
                        log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                        */
                        let mut out = self.signature.inject(sig, param_flows)?;
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            out.return_flows.apply_context(&self.conditions);
                            Some(out.return_flows)
                        }
                    } // TODO: throw_flows ?
                };

                self.last_result = result_flows;

                Ok(())
            }
            Instr::InvokeStaticRange(args, meth) => {
                let null = amg::Vertex::null();
                let null = self.signature.amg_mut().add_vertex(null);

                let mut param_flows = Vec::new();

                let mut args_it = args.iter();

                let meth = meth.get(dex)?;
                let meth = MethodDescr::try_from((dex, meth))?;
                let meth_uid = context
                    .repo
                    .find_method_by_descriptor(&meth)
                    .ok_or(FlowError::UnknownClass)?
                    .uid();

                // arguments against expected parameters
                for p_typ in meth.parameters_types() {
                    let arg_reg = args_it.next().ok_or(FlowError::BadArity)?;
                    let arg_flow = match p_typ {
                        Type::Double | Type::Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    let mut arg_flow = arg_flow.clone();
                    arg_flow.retain(|f| f.vertex_hash() != null);
                    param_flows.push(arg_flow.clone());
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(FlowError::BadArity.into());
                }

                let meth_ret = meth.return_type();

                let result_flows = match context.signatures.get(&meth_uid) {
                    None => {
                        if meth_ret != &Type::Void {
                            log::error!("no signature for {meth_uid} = {}", meth);
                        } else {
                            log::warn!("missing signature for {meth_uid} = {}", meth);
                        }
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            Some(amg::Flows::default())
                        }
                    }
                    Some(sig) => {
                        /*
                        log::info!(
                            "\nCURRENT AMG =\n{}",
                            self.signature.amg().to_dot(context.repo)?
                        );
                        log::info!("\nTO INJECT =\n{}", sig.amg().to_dot(context.repo)?);
                        */
                        let mut out = self.signature.inject(sig, param_flows)?;
                        if matches!(meth_ret, Type::Void) {
                            None
                        } else {
                            out.return_flows.apply_context(&self.conditions);
                            Some(out.return_flows)
                        }
                    } // TODO: throw_flows ?
                };

                self.last_result = result_flows;

                Ok(())
            }

            Instr::NegInt(dst, src)
            | Instr::NotInt(dst, src)
            | Instr::AddIntLit16(dst, src, _)
            | Instr::RsubInt(dst, src, _)
            | Instr::MulIntLit16(dst, src, _)
            | Instr::DivIntLit16(dst, src, _)
            | Instr::RemIntLit16(dst, src, _)
            | Instr::AddIntLit8(dst, src, _)
            | Instr::RsubIntLit8(dst, src, _)
            | Instr::MulIntLit8(dst, src, _)
            | Instr::DivIntLit8(dst, src, _)
            | Instr::RemIntLit8(dst, src, _)
            | Instr::ShlIntLit8(dst, src, _)
            | Instr::ShrIntLit8(dst, src, _)
            | Instr::UshrIntLit8(dst, src, _)
            | Instr::AndIntLit16(dst, src, _)
            | Instr::OrIntLit16(dst, src, _)
            | Instr::XorIntLit16(dst, src, _)
            | Instr::AndIntLit8(dst, src, _)
            | Instr::OrIntLit8(dst, src, _)
            | Instr::XorIntLit8(dst, src, _)
            | Instr::NegFloat(dst, src) => {
                let mut f = self.read_reg(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)?;
                Ok(())
            }

            Instr::NegLong(dst, src) | Instr::NotLong(dst, src) | Instr::NegDouble(dst, src) => {
                let mut f = self.read_pair(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_pair(*dst, f)?;
                Ok(())
            }

            Instr::IntToLong(dst, src)
            | Instr::IntToDouble(dst, src)
            | Instr::FloatToLong(dst, src)
            | Instr::FloatToDouble(dst, src) => {
                let mut f = self.read_reg(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_pair(*dst, f)
            }
            Instr::IntToFloat(dst, src)
            | Instr::FloatToInt(dst, src)
            | Instr::IntToByte(dst, src)
            | Instr::IntToChar(dst, src)
            | Instr::IntToShort(dst, src) => {
                let mut f = self.read_reg(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::LongToInt(dst, src)
            | Instr::LongToFloat(dst, src)
            | Instr::DoubleToInt(dst, src)
            | Instr::DoubleToFloat(dst, src) => {
                let mut f = self.read_pair(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            Instr::LongToDouble(dst, src) | Instr::DoubleToLong(dst, src) => {
                let mut f = self.read_pair(*src)?.clone();
                f.apply_context(&self.conditions);
                self.write_pair(*dst, f)
            }

            Instr::AddInt(dst, src1, src2)
            | Instr::SubInt(dst, src1, src2)
            | Instr::MulInt(dst, src1, src2)
            | Instr::DivInt(dst, src1, src2)
            | Instr::RemInt(dst, src1, src2)
            | Instr::AndInt(dst, src1, src2)
            | Instr::OrInt(dst, src1, src2)
            | Instr::XorInt(dst, src1, src2)
            | Instr::ShlInt(dst, src1, src2)
            | Instr::ShrInt(dst, src1, src2)
            | Instr::UshrInt(dst, src1, src2)
            | Instr::AddFloat(dst, src1, src2)
            | Instr::SubFloat(dst, src1, src2)
            | Instr::MulFloat(dst, src1, src2)
            | Instr::DivFloat(dst, src1, src2)
            | Instr::RemFloat(dst, src1, src2) => {
                let f1 = self.read_reg(*src1)?;
                let f2 = self.read_reg(*src2)?;
                let mut f = f1.join(f2);
                f.apply_context(&self.conditions);
                self.write_reg(*dst, f)
            }
            /*
            Instr::AddLong(dst, src1, src2)
            | Instr::SubLong(dst, src1, src2)
            | Instr::MulLong(dst, src1, src2)
            | Instr::DivLong(dst, src1, src2)
            | Instr::RemLong(dst, src1, src2) => {
                let src1_typ = self.read_pair(*src1)?;
                let src2_typ = self.read_pair(*src2)?;
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Long ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::AndLong(dst, src1, src2)
            | Instr::OrLong(dst, src1, src2)
            | Instr::XorLong(dst, src1, src2) => {
                let src1_typ = self.read_pair(*src1)?;
                let src2_typ = self.read_pair(*src2)?;
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Long ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::ShlLong(dst, src1, src2)
            | Instr::ShrLong(dst, src1, src2)
            | Instr::UshrLong(dst, src1, src2) => {
                let src1_typ = self.read_pair(*src1)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_pair(*dst, Long)
            }

                let src1_typ = self.read_reg(*src1)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Float ; repo)?;
                tc!(src2_typ <: &Float ; repo)?;
                self.write_reg(*dst, Float)
            }

            Instr::AddDouble(dst, src1, src2)
            | Instr::SubDouble(dst, src1, src2)
            | Instr::MulDouble(dst, src1, src2)
            | Instr::DivDouble(dst, src1, src2)
            | Instr::RemDouble(dst, src1, src2) => {
                let src1_typ = self.read_pair(*src1)?;
                let src2_typ = self.read_pair(*src2)?;
                tc!(src1_typ <: &Double ; repo)?;
                tc!(src2_typ <: &Double ; repo)?;
                self.write_pair(*dst, Double)
            }*/
            Instr::AddInt2addr(bid, src2)
            | Instr::SubInt2addr(bid, src2)
            | Instr::MulInt2addr(bid, src2)
            | Instr::DivInt2addr(bid, src2)
            | Instr::RemInt2addr(bid, src2)
            | Instr::AndInt2addr(bid, src2)
            | Instr::OrInt2addr(bid, src2)
            | Instr::XorInt2addr(bid, src2)
            | Instr::ShlInt2addr(bid, src2)
            | Instr::ShrInt2addr(bid, src2)
            | Instr::UshrInt2addr(bid, src2)
            | Instr::AddFloat2addr(bid, src2)
            | Instr::SubFloat2addr(bid, src2)
            | Instr::MulFloat2addr(bid, src2)
            | Instr::DivFloat2addr(bid, src2)
            | Instr::RemFloat2addr(bid, src2) => {
                let f1 = self.read_reg(*bid)?;
                let f2 = self.read_reg(*src2)?;
                let mut f = f1.join(f2);
                f.apply_context(&self.conditions);
                self.write_reg(*bid, f)
            }

            /*
            Instr::AddLong2addr(bid, src2)
            | Instr::SubLong2addr(bid, src2)
            | Instr::MulLong2addr(bid, src2)
            | Instr::DivLong2addr(bid, src2)
            | Instr::RemLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?;
                let src2_typ = self.read_pair(*src2)?;
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Long ; repo)?;
                self.write_pair(*bid, Long)
            }
            Instr::AndLong2addr(bid, src2)
            | Instr::OrLong2addr(bid, src2)
            | Instr::XorLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Long ; repo)?;
                self.write_pair(*bid, Long)
            }
            Instr::ShlLong2addr(bid, src2)
            | Instr::ShrLong2addr(bid, src2)
            | Instr::UshrLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Long ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_pair(*bid, Long)
            }


            Instr::AddDouble2addr(bid, src2)
            | Instr::SubDouble2addr(bid, src2)
            | Instr::MulDouble2addr(bid, src2)
            | Instr::DivDouble2addr(bid, src2)
            | Instr::RemDouble2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?;
                let src2_typ = self.read_pair(*src2)?;
                tc!(src1_typ <: &Double ; repo)?;
                tc!(src2_typ <: &Double ; repo)?;
                self.write_pair(*bid, Double)
            }

            Instr::InvokeCustom(_, _)
            | Instr::InvokeCustomRange(_, _)
            | Instr::InvokePolymorphic(_, _, _)
            | Instr::InvokePolymorphicRange(_, _, _)
            | Instr::ConstMethodHandle(_, _)
            | Instr::ConstMethodType(_, _) => {
                unimplemented!("{:?} transfer in forward mode", instr)
            }

            Instr::PackedSwitchPayload(_, _)
            | Instr::SparseSwitchPayload(_, _)
            | Instr::FillArrayDataPayload(_) => panic!("internal error"),
             */
            _ => unimplemented!("information flow for instruction {instr:?}"),
        }
    }
}
