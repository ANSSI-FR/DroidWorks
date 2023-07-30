//! Control flow graph representation.

use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::Method;
use dw_dex::code::CodeItem;
use dw_dex::instrs::{Instr, Instruction, LabeledInstr};
use dw_dex::registers::Reg;
use dw_dex::types::Type;
use dw_dex::{Addr, Dex, DexIndex, PrettyPrint};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::NodeRef;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write;

#[derive(Debug)]
pub struct Block<'a> {
    dex: &'a Dex,
    instrs: Vec<LabeledInstr>,
    can_throw: bool,
}

impl<'a> fmt::Display for Block<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.instrs.is_empty() {
            write!(f, "<END>")?;
            return Ok(());
        }
        for linstr in &self.instrs {
            write!(f, "{:5}: ", linstr.addr())?;
            linstr.instr().pp(f, self.dex).unwrap();
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<'a> Block<'a> {
    fn new(instrs: Vec<LabeledInstr>, dex: &'a Dex) -> Self {
        let can_throw = instruction_can_throw(&instrs[0]);
        Self {
            dex,
            instrs,
            can_throw,
        }
    }

    #[inline]
    pub fn instructions(&self) -> impl Iterator<Item = &LabeledInstr> {
        self.instrs.iter()
    }

    #[inline]
    pub fn rev_instructions(&self) -> impl Iterator<Item = &LabeledInstr> {
        self.instrs.iter().rev()
    }

    #[must_use]
    pub fn start_addr(&self) -> Addr {
        self.instrs.first().unwrap().addr()
    }

    #[inline]
    pub fn can_throw(&self) -> bool {
        self.can_throw
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Comp {
    Eq,
    Ne,
    Lt,
    Ge,
    Gt,
    Le,
}

#[derive(Debug, Clone, Copy)]
pub enum Operand {
    Register(Reg),
    Zero,
}

#[derive(Debug)]
pub enum Branch {
    IfTrue(Reg, Comp, Operand),
    IfFalse(Reg, Comp, Operand),
    Switch(Reg, i32),
    SwitchDefault,
    Jmp,
    Sequence,
    Catch(Type),
    CatchAll,
    ArrayAccessSuccess,
    InvokeSuccess,
    CastSuccess(Reg, Type),
    DivSuccess,
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IfTrue(_, _, _) => write!(f, "<true>"),
            Self::IfFalse(_, _, _) => write!(f, "<false>"),
            Self::Switch(_, _) => write!(f, "<switch>"),
            Self::SwitchDefault => write!(f, "<switch _>"),
            Self::Jmp => write!(f, "<jmp>"),
            Self::Sequence => write!(f, "<seq>"),
            Self::Catch(typ) => write!(f, "<catch {typ}>"),
            Self::CatchAll => write!(f, "<catch *>"),
            Self::ArrayAccessSuccess => write!(f, "<seq_array>"),
            Self::InvokeSuccess => write!(f, "<seq_invoke>"),
            Self::CastSuccess(_, _) => write!(f, "<seq_cast>"),
            Self::DivSuccess => write!(f, "<seq_div>"),
        }
    }
}

#[derive(Debug)]
pub struct Cfg<'a> {
    pub(crate) inner: DiGraph<Block<'a>, Branch>,
    node_ids: BTreeMap<Addr, NodeIndex>,
}

impl<'a> Cfg<'a> {
    pub(crate) fn start_index(&self) -> NodeIndex {
        *self.node_ids.get(&Addr::entry()).unwrap()
    }

    pub fn iter_ordered_blocks(&self) -> impl Iterator<Item = &Block> {
        self.node_ids.values().map(move |id| &self.inner[*id])
    }

    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut res = String::new();
        res.push_str("digraph {\n");
        res.push_str("  splines=ortho;\n");
        res.push_str("  nodesep=2;\n");
        write!(
            res,
            "{}",
            Dot::with_attr_getters(
                &self.inner,
                &[Config::GraphContentOnly, Config::EdgeNoLabel],
                &|_, edge| {
                    let color = match edge.weight() {
                        Branch::IfTrue(_, _, _) => "green",
                        Branch::IfFalse(_, _, _) => "red",
                        Branch::Switch(_, _) | Branch::SwitchDefault => "purple",
                        Branch::Jmp => "blue",
                        Branch::Catch(_) | Branch::CatchAll => "orchid",
                        Branch::Sequence
                        | Branch::ArrayAccessSuccess
                        | Branch::InvokeSuccess
                        | Branch::CastSuccess(_, _)
                        | Branch::DivSuccess => "black",
                    };
                    format!("color={},xlabel=\"{}\"", color, edge.weight())
                },
                &|_, node| if node.weight().can_throw() {
                    String::from("shape=box,color=blue")
                } else {
                    String::from("shape=box,color=black")
                }
            )
        )
        .unwrap();
        res.push('}');
        res
    }

    pub fn build(method: &'a Method) -> AnalysisResult<Self> {
        let dex = method.dex();
        let code = method.code().ok_or(AnalysisError::NoCode)?;

        let mut cfgraph = DiGraph::new();
        let mut blocks_map = BTreeMap::new();

        let leaders = compute_block_leaders(&code.read().unwrap(), dex)?;
        for block in split_into_blocks(&code.read().unwrap(), dex, leaders) {
            blocks_map.insert(block.start_addr(), cfgraph.add_node(block));
        }

        let breakers: Vec<(Addr, LabeledInstr)> = cfgraph
            .node_indices()
            .filter_map(|id| {
                let block = &cfgraph[id];
                if block.instrs.is_empty() {
                    None
                } else {
                    Some((
                        block.instrs.first().unwrap().addr(),
                        block.instrs.last().unwrap().clone(),
                    ))
                }
            })
            .collect();
        for (leader_addr, linstr) in breakers {
            let src_id = blocks_map[&leader_addr];
            let branching = instruction_branching(&code.read().unwrap(), dex, &linstr)?;
            let block_tries = block_tries(&code.read().unwrap(), dex, &linstr)?;
            if branching.is_empty()
                && !instruction_can_throw(&linstr)
                && !instruction_does_return(&linstr)
            {
                if let Some(dst_id) = blocks_map.get(&linstr.next_addr()) {
                    cfgraph.add_edge(src_id, *dst_id, Branch::Sequence);
                }
            }
            branching
                .into_iter()
                .chain(block_tries.into_iter())
                .for_each(|(branch, dst)| {
                    let dst_id = blocks_map[&dst];
                    cfgraph.add_edge(src_id, dst_id, branch);
                });
        }

        Ok(Self {
            inner: cfgraph,
            node_ids: blocks_map,
        })
    }
}

// Block leaders are block first instructions addresses.
// Leaders can be caused by several cases:
//   - target address of a branching instruction is a leader
//   - address following a branching instruction is a leader
//   - throwable instruction is a leader (so that we can easily retrieve state before the
//     instruction when running a dataflow analysis)
//   - start of a catch block is a leader
fn compute_block_leaders(code: &CodeItem, dex: &Dex) -> AnalysisResult<BTreeSet<Addr>> {
    let mut leaders = BTreeSet::new();

    // collect leaders caused by instruction branching
    for linstr in code.iter_instructions() {
        let branching = instruction_branching(code, dex, linstr)?;
        if !branching.is_empty() || instruction_can_throw(linstr) || instruction_does_return(linstr)
        {
            leaders.insert(linstr.next_addr());
        }
        for (_, dst) in branching {
            leaders.insert(dst);
        }
        if instruction_can_throw(linstr) {
            leaders.insert(linstr.addr());
        }
    }

    // collect block leaders caused by try/catch dex descriptions
    for try_ in code.iter_tries() {
        leaders.insert(try_.start_addr());
        leaders.insert(try_.end_addr());

        let catches = try_.handlers(code)?;
        for handler in catches.iter_handlers() {
            leaders.insert(Addr(handler.catch_addr()));
        }
        if let Some(addr) = catches.catch_all_addr() {
            leaders.insert(Addr(addr));
        }
    }

    Ok(leaders)
}

fn split_into_blocks<'a>(
    code: &CodeItem,
    dex: &'a Dex,
    mut leaders: BTreeSet<Addr>,
) -> Vec<Block<'a>> {
    let mut instrs = Vec::new();
    let mut blocks = Vec::new();

    // remove 0 so we don't split at the beginning and don't create empty block
    leaders.remove(&Addr::entry());

    for linstr in code.iter_instructions() {
        if leaders.contains(&linstr.addr()) {
            blocks.push(Block::new(instrs, dex));
            instrs = Vec::new();
        }
        instrs.push(linstr.clone());
    }

    // push final block (cannot be empty due to pushing instruction after resetting instrs in the
    // for-loop)
    blocks.push(Block::new(instrs, dex));

    blocks
}

fn instruction_branching(
    code: &CodeItem,
    dex: &Dex,
    linstr: &LabeledInstr,
) -> AnalysisResult<Vec<(Branch, Addr)>> {
    match linstr.instr() {
        Instr::PackedSwitch(reg, a) => {
            let payload = code.instruction_at(linstr.addr().offset(*a))?;
            packed_switch_payload_branching(linstr, *reg, payload)
        }
        Instr::SparseSwitch(reg, a) => {
            let payload = code.instruction_at(linstr.addr().offset(*a))?;
            sparse_switch_payload_branching(linstr, *reg, payload)
        }
        Instr::Goto(a) => Ok(vec![(
            Branch::Jmp,
            Addr::from_offset(linstr.addr(), i32::from(*a)),
        )]),
        Instr::Goto16(a) => Ok(vec![(
            Branch::Jmp,
            Addr::from_offset(linstr.addr(), i32::from(*a)),
        )]),
        Instr::Goto32(a) => Ok(vec![(Branch::Jmp, linstr.addr().offset(*a))]),

        Instr::IfEq(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Eq, Operand::Register(*reg2))
        }
        Instr::IfNe(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Ne, Operand::Register(*reg2))
        }
        Instr::IfLt(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Lt, Operand::Register(*reg2))
        }
        Instr::IfGe(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Ge, Operand::Register(*reg2))
        }
        Instr::IfGt(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Gt, Operand::Register(*reg2))
        }
        Instr::IfLe(reg1, reg2, a) => {
            if_instr_branching(linstr, *a, *reg1, Comp::Le, Operand::Register(*reg2))
        }
        Instr::IfEqz(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Eq, Operand::Zero),
        Instr::IfNez(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Ne, Operand::Zero),
        Instr::IfLtz(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Lt, Operand::Zero),
        Instr::IfGez(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Ge, Operand::Zero),
        Instr::IfGtz(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Gt, Operand::Zero),
        Instr::IfLez(reg1, a) => if_instr_branching(linstr, *a, *reg1, Comp::Le, Operand::Zero),

        Instr::ArrayLength(_, _)
        | Instr::FilledNewArray(_, _)
        | Instr::FilledNewArrayRange(_, _)
        | Instr::FillArrayData(_, _)
        | Instr::Aget(_, _, _)
        | Instr::AgetWide(_, _, _)
        | Instr::AgetObject(_, _, _)
        | Instr::AgetBoolean(_, _, _)
        | Instr::AgetByte(_, _, _)
        | Instr::AgetChar(_, _, _)
        | Instr::AgetShort(_, _, _)
        | Instr::Aput(_, _, _)
        | Instr::AputWide(_, _, _)
        | Instr::AputObject(_, _, _)
        | Instr::AputBoolean(_, _, _)
        | Instr::AputByte(_, _, _)
        | Instr::AputChar(_, _, _)
        | Instr::AputShort(_, _, _) => Ok(vec![(Branch::ArrayAccessSuccess, linstr.next_addr())]),

        Instr::InvokeVirtual(_, _)
        | Instr::InvokeSuper(_, _)
        | Instr::InvokeDirect(_, _)
        | Instr::InvokeStatic(_, _)
        | Instr::InvokeInterface(_, _)
        | Instr::InvokeVirtualRange(_, _)
        | Instr::InvokeSuperRange(_, _)
        | Instr::InvokeDirectRange(_, _)
        | Instr::InvokeStaticRange(_, _)
        | Instr::InvokeInterfaceRange(_, _)
        | Instr::InvokePolymorphic(_, _, _)
        | Instr::InvokePolymorphicRange(_, _, _) => {
            Ok(vec![(Branch::InvokeSuccess, linstr.next_addr())])
        }

        Instr::DivInt(_, _, _)
        | Instr::RemInt(_, _, _)
        | Instr::DivLong(_, _, _)
        | Instr::RemLong(_, _, _)
        | Instr::DivInt2addr(_, _)
        | Instr::RemInt2addr(_, _)
        | Instr::DivLong2addr(_, _)
        | Instr::RemLong2addr(_, _)
        | Instr::DivIntLit16(_, _, _)
        | Instr::RemIntLit16(_, _, _)
        | Instr::DivIntLit8(_, _, _)
        | Instr::RemIntLit8(_, _, _) => Ok(vec![(Branch::DivSuccess, linstr.next_addr())]),

        Instr::CheckCast(reg, type_) => Ok(vec![(
            Branch::CastSuccess(*reg, type_.get(dex)?.to_type(dex)?),
            linstr.next_addr(),
        )]),
        _ => Ok(vec![]),
    }
}

fn if_instr_branching(
    linstr: &LabeledInstr,
    offset: impl Into<i32>,
    op1: Reg,
    comp: Comp,
    op2: Operand,
) -> AnalysisResult<Vec<(Branch, Addr)>> {
    Ok(vec![
        (
            Branch::IfTrue(op1, comp, op2),
            linstr.addr().offset(offset.into()),
        ),
        (Branch::IfFalse(op1, comp, op2), linstr.next_addr()),
    ])
}

fn packed_switch_payload_branching(
    linstr: &LabeledInstr,
    reg: Reg,
    payload: &LabeledInstr,
) -> AnalysisResult<Vec<(Branch, Addr)>> {
    if let Instr::PackedSwitchPayload(first_key, targets) = payload.instr() {
        Ok(std::iter::once((Branch::SwitchDefault, linstr.next_addr()))
            .chain(targets.iter().enumerate().map(|(i, target)| {
                (
                    Branch::Switch(reg, first_key + i as i32),
                    Addr::from_offset(linstr.addr(), *target),
                )
            }))
            .collect())
    } else {
        Err(AnalysisError::InstructionNotFound("payload".to_string()))
    }
}

fn sparse_switch_payload_branching(
    linstr: &LabeledInstr,
    reg: Reg,
    payload: &LabeledInstr,
) -> AnalysisResult<Vec<(Branch, Addr)>> {
    if let Instr::SparseSwitchPayload(keys, targets) = payload.instr() {
        assert_eq!(keys.len(), targets.len());
        Ok(std::iter::once((Branch::SwitchDefault, linstr.next_addr()))
            .chain(keys.iter().zip(targets.iter()).map(|(key, target)| {
                (
                    Branch::Switch(reg, *key),
                    Addr::from_offset(linstr.addr(), *target),
                )
            }))
            .collect())
    } else {
        Err(AnalysisError::InstructionNotFound("payload".to_string()))
    }
}

fn instruction_does_return(linstr: &LabeledInstr) -> bool {
    matches!(
        linstr.instr(),
        Instr::ReturnVoid | Instr::Return(_) | Instr::ReturnWide(_) | Instr::ReturnObject(_)
    )
}

fn instruction_can_throw(linstr: &LabeledInstr) -> bool {
    matches!(
        linstr.instr(),
        Instr::Throw(_)
            | Instr::ArrayLength(_, _)
            | Instr::FilledNewArray(_, _)
            | Instr::FilledNewArrayRange(_, _)
            | Instr::FillArrayData(_, _)
            | Instr::Aget(_, _, _)
            | Instr::AgetWide(_, _, _)
            | Instr::AgetObject(_, _, _)
            | Instr::AgetBoolean(_, _, _)
            | Instr::AgetByte(_, _, _)
            | Instr::AgetChar(_, _, _)
            | Instr::AgetShort(_, _, _)
            | Instr::Aput(_, _, _)
            | Instr::AputWide(_, _, _)
            | Instr::AputObject(_, _, _)
            | Instr::AputBoolean(_, _, _)
            | Instr::AputByte(_, _, _)
            | Instr::AputChar(_, _, _)
            | Instr::AputShort(_, _, _)
            | Instr::InvokeVirtual(_, _)
            | Instr::InvokeSuper(_, _)
            | Instr::InvokeDirect(_, _)
            | Instr::InvokeStatic(_, _)
            | Instr::InvokeInterface(_, _)
            | Instr::InvokeVirtualRange(_, _)
            | Instr::InvokeSuperRange(_, _)
            | Instr::InvokeDirectRange(_, _)
            | Instr::InvokeStaticRange(_, _)
            | Instr::InvokeInterfaceRange(_, _)
            | Instr::InvokePolymorphic(_, _, _)
            | Instr::InvokePolymorphicRange(_, _, _)
            | Instr::DivInt(_, _, _)
            | Instr::RemInt(_, _, _)
            | Instr::DivLong(_, _, _)
            | Instr::RemLong(_, _, _)
            | Instr::DivInt2addr(_, _)
            | Instr::RemInt2addr(_, _)
            | Instr::DivLong2addr(_, _)
            | Instr::RemLong2addr(_, _)
            | Instr::DivIntLit16(_, _, _)
            | Instr::RemIntLit16(_, _, _)
            | Instr::DivIntLit8(_, _, _)
            | Instr::RemIntLit8(_, _, _)
            | Instr::CheckCast(_, _)
    )
}

fn block_tries(
    code: &CodeItem,
    dex: &Dex,
    linstr: &LabeledInstr,
) -> AnalysisResult<Vec<(Branch, Addr)>> {
    if !linstr.can_throw() {
        return Ok(vec![]);
    }
    for try_ in code.iter_tries() {
        // TODO: this loop can be optimized (early break if tries are sorted)
        let try_beg = try_.start_addr();
        let try_end = try_.end_addr();
        if linstr.addr() >= try_beg && linstr.addr() < try_end {
            let catches = try_.handlers(code)?;
            let mut v: Vec<(Branch, Addr)> = catches
                .iter_handlers()
                .map(|h| {
                    let catch_type = h.catch_type(dex)?;
                    Ok((Branch::Catch(catch_type), Addr(h.catch_addr())))
                })
                .collect::<AnalysisResult<_>>()?;
            if let Some(a) = catches.catch_all_addr() {
                v.push((Branch::CatchAll, Addr(a)));
            }
            return Ok(v);
        }
    }
    Ok(vec![])
}
