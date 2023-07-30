use crate::controlflow::Branch;
use crate::dataflow::AbstractBackwardState;
use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::{Class, Method, Repo};
use crate::typing::errors::TypeError;
use crate::typing::types::{
    AbstractType, JAVA_LANG_CLASS, JAVA_LANG_OBJECT, JAVA_LANG_STRING, JAVA_LANG_THROWABLE,
};
use crate::typing::{tc, State};
use dw_dex::instrs::Instr;
use dw_dex::types::Type;
use dw_dex::{Dex, DexIndex, WithDex};
use std::convert::TryFrom;

impl<'a> AbstractBackwardState<'a> for State {
    type Context<'c> = Repo<'c>;
    type Error = AnalysisError;

    fn init(method: &Method, _class: &Class) -> AnalysisResult<Self> {
        // Note1: method registers layout:
        // [...local registers ...]
        // ['this' register (if method is not static)]
        // [...parameters...]

        // Note2: nb_registers = local_regs + this_reg + param_regs

        let nb_registers = method.code().unwrap().read().unwrap().registers_size();

        let mut registers = Vec::new();
        for _ in 0..nb_registers {
            registers.push(AbstractType::Top);
        }

        let expected = match method.return_type() {
            Type::Void => None,
            t => {
                let s = AbstractType::try_from(t)?;
                Some(s)
            }
        };

        Ok(Self {
            registers,
            last_exception: None,
            last_result: None,
            expected,
        })
    }

    fn meet(&mut self, other: &Self, repo: &Repo) -> AnalysisResult<()> {
        if self.registers.len() != other.registers.len() {
            return Err(TypeError::IncompatibleStates.into());
        }

        for i in 0..self.registers.len() {
            let reg1 = self.registers[i].clone();
            let reg2 = other.registers[i].clone();
            self.registers[i] = reg1.meet(reg2, repo)?;
        }

        self.last_exception = match (&self.last_exception, &other.last_exception) {
            (Some(st1), Some(st2)) => {
                let t1 = st1.clone();
                let t2 = st2.clone();
                Some(t1.meet(t2, repo)?)
            }
            (t @ Some(_), _) | (_, t @ Some(_)) => t.clone(),
            _ => None,
        };

        self.last_result = match (&self.last_result, &other.last_result) {
            (Some(st1), Some(st2)) => {
                let t1 = st1.clone();
                let t2 = st2.clone();
                Some(t1.meet(t2, repo)?)
            }
            (t @ Some(_), _) | (_, t @ Some(_)) => t.clone(),
            _ => None,
        };

        assert!(self.expected == other.expected);

        Ok(())
    }

    fn transfer_branch(&mut self, branch: &Branch, repo: &Repo) -> AnalysisResult<()> {
        use AbstractType::Null;

        match branch {
            Branch::IfTrue(_, _, _)
            | Branch::IfFalse(_, _, _)
            | Branch::Switch(_, _)
            | Branch::SwitchDefault
            | Branch::Jmp
            | Branch::Sequence
            | Branch::ArrayAccessSuccess
            | Branch::InvokeSuccess
            | Branch::DivSuccess => Ok(()),
            Branch::Catch(e_catch) => match &self.last_exception {
                None => {
                    let e_catch = AbstractType::try_from(e_catch)?;
                    tc!(e_catch <: &JAVA_LANG_THROWABLE ; repo)?;
                    self.last_exception = Some(e_catch);
                    Ok(())
                }
                Some(e_throw) => {
                    let e_catch = AbstractType::try_from(e_catch)?;
                    tc!(e_catch <: &JAVA_LANG_THROWABLE ; repo)?;
                    let m_typ = e_throw.clone().meet(e_catch.clone(), repo)?;
                    tc!(Null <: &m_typ ; repo)?;
                    self.last_exception = Some(e_catch);
                    Ok(())
                }
            },
            Branch::CatchAll => match &self.last_exception {
                None => {
                    self.last_exception = Some(JAVA_LANG_THROWABLE.clone());
                    Ok(())
                }
                Some(e) => {
                    let m_typ = e.clone().meet(JAVA_LANG_THROWABLE.clone(), repo)?;
                    tc!(Null <: &m_typ ; repo)?;
                    self.last_exception = Some(JAVA_LANG_THROWABLE.clone());
                    Ok(())
                }
            },
            Branch::CastSuccess(ptr, cls) => {
                let cls_typ = AbstractType::try_from(cls)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                tc!(cls_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
        }
    }

    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::enum_glob_use)]
    fn transfer_instr(&mut self, instr: &Instr, dex: &Dex, repo: &Repo) -> AnalysisResult<()> {
        use AbstractType::*;

        // save last status registers, and reset them to the 'default' value (None)
        let last_exception = std::mem::replace(&mut self.last_exception, None);
        let last_result = std::mem::replace(&mut self.last_result, None);

        match instr {
            Instr::Nop | Instr::Goto(_) | Instr::Goto16(_) | Instr::Goto32(_) => Ok(()),

            Instr::Move(dst, src) | Instr::MoveFrom16(dst, src) | Instr::Move16(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Join32, repo)?;
                tc!(Meet32 <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(m_typ, repo)?;
                tc!(Meet32 <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::MoveWide(dst, src)
            | Instr::MoveWideFrom16(dst, src)
            | Instr::MoveWide16(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Join64, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(m_typ, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }

            Instr::MoveObject(dst, src)
            | Instr::MoveObjectFrom16(dst, src)
            | Instr::MoveObject16(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(m_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::MoveResult(dst) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Join32, repo)?;
                tc!(Meet32 <: &m_typ ; repo)?;
                self.last_result = Some(m_typ);
                self.write_reg(*dst, Top)
            }
            Instr::MoveResultWide(dst) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Join64, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.last_result = Some(m_typ);
                self.write_pair(*dst, Top)
            }
            Instr::MoveResultObject(dst) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.last_result = Some(m_typ);
                self.write_reg(*dst, Top)
            }
            Instr::MoveException(dst) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JAVA_LANG_THROWABLE.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.last_exception = Some(m_typ);
                self.write_reg(*dst, Top)
            }

            Instr::ReturnVoid => match &self.expected {
                None => Ok(()),
                Some(_) => Err(TypeError::BadReturnType.into()),
            },
            Instr::Return(ret) => match &self.expected {
                Some(exp_typ) => {
                    tc!(exp_typ <: &Join32 ; repo)?;
                    let ret_typ = self.read_reg(*ret)?.clone();
                    let m_typ = ret_typ.meet(exp_typ.clone(), repo)?;
                    tc!(exp_typ <: &m_typ ; repo)?;
                    self.write_reg(*ret, m_typ)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },
            Instr::ReturnWide(ret) => match &self.expected {
                Some(exp_typ) => {
                    tc!(exp_typ <: &Join64 ; repo)?;
                    let ret_typ = self.read_pair(*ret)?.clone();
                    let m_typ = ret_typ.meet(exp_typ.clone(), repo)?;
                    tc!(exp_typ <: &m_typ ; repo)?;
                    self.write_pair(*ret, m_typ)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },
            Instr::ReturnObject(ret) => match &self.expected {
                Some(exp_typ) => {
                    tc!(exp_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                    let ret_typ = self.read_reg(*ret)?.clone();
                    let m_typ = ret_typ.meet(exp_typ.clone(), repo)?;
                    tc!(exp_typ <: &m_typ ; repo)?;
                    self.write_reg(*ret, m_typ)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },

            Instr::Const4(dst, val) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                if *val == 0 {
                    let m_typ = dst_typ.meet(JoinZero, repo)?;
                    tc!(MeetZero <: &m_typ ; repo )?;
                } else {
                    let m_typ = dst_typ.meet(Join32, repo)?;
                    tc!(Meet32 <: &m_typ ; repo )?;
                }
                self.write_reg(*dst, Top)
            }
            Instr::Const16(dst, val) | Instr::ConstHigh16(dst, val) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                if *val == 0 {
                    let m_typ = dst_typ.meet(JoinZero, repo)?;
                    tc!(MeetZero <: &m_typ ; repo )?;
                } else {
                    let m_typ = dst_typ.meet(Join32, repo)?;
                    tc!(Meet32 <: &m_typ ; repo )?;
                }
                self.write_reg(*dst, Top)
            }
            Instr::Const(dst, val) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                if *val == 0 {
                    let m_typ = dst_typ.meet(JoinZero, repo)?;
                    tc!(MeetZero <: &m_typ ; repo )?;
                } else {
                    let m_typ = dst_typ.meet(Join32, repo)?;
                    tc!(Meet32 <: &m_typ ; repo )?;
                }
                self.write_reg(*dst, Top)
            }

            Instr::ConstWide16(dst, _)
            | Instr::ConstWideHigh16(dst, _)
            | Instr::ConstWide32(dst, _)
            | Instr::ConstWide(dst, _) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Join64, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)
            }

            Instr::ConstString(dst, _) | Instr::ConstStringJumbo(dst, _) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(JAVA_LANG_STRING <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }

            Instr::ConstClass(dst, cls) => {
                let cls_typ = match cls.get(dex)?.to_type(dex)? {
                    Type::Boolean => {
                        AbstractType::object_singleton("java/lang/Boolean".to_string())
                    }
                    Type::Byte => AbstractType::object_singleton("java/lang/Byte".to_string()),
                    Type::Short => AbstractType::object_singleton("java/lang/Short".to_string()),
                    Type::Char => AbstractType::object_singleton("java/lang/Character".to_string()),
                    Type::Int => AbstractType::object_singleton("java/lang/Integer".to_string()),
                    Type::Long => AbstractType::object_singleton("java/lang/Long".to_string()),
                    Type::Float => AbstractType::object_singleton("java/lang/Float".to_string()),
                    Type::Double => AbstractType::object_singleton("java/lang/Double".to_string()),
                    _ => AbstractType::try_from(WithDex::new(dex, *cls))?,
                };
                tc!(cls_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(JAVA_LANG_CLASS <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::MonitorEnter(ptr) | Instr::MonitorExit(ptr) => {
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::CheckCast(ptr, cls) => {
                let _ = AbstractType::try_from(WithDex::new(dex, *cls))?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, JAVA_LANG_OBJECT.clone())
            }
            Instr::InstanceOf(dst, ptr, cls) => {
                let _ = AbstractType::try_from(WithDex::new(dex, *cls))?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, JAVA_LANG_OBJECT.clone())
            }

            Instr::ArrayLength(dst, ptr) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(Array(1, Box::new(Top)), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }

            Instr::NewInstance(dst, descr) => {
                let typ = AbstractType::try_from(WithDex::new(dex, *descr))?;
                let dst_typ = self.read_reg(*dst)?.clone();
                match typ {
                    Object(_) => {
                        let m_typ = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                        tc!(typ <: &m_typ ; repo)?;
                        self.write_reg(*dst, Top)
                    }
                    _ => Err(TypeError::ExpectedClass.into()),
                }
            }
            Instr::NewArray(dst, siz, arr) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let arr_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                let m_typ = dst_typ.meet(arr_typ.clone(), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;

                match arr_typ {
                    Array(_, _) => {
                        // needs to come after the write of dst reg
                        // in case the same register is used
                        let siz_typ = self.read_reg(*siz)?.clone();
                        tc!(Integer <: &siz_typ ; repo)?;
                        self.write_reg(*siz, Integer)
                    }
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }

            Instr::FilledNewArray(args, arr) => match last_result {
                None => Err(TypeError::MissingResult.into()),
                Some(res_typ) => {
                    let arr_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                    let m_typa = arr_typ.meet(Array(1, Box::new(Top)), repo)?;
                    tc!(Null <: &m_typa ; repo)?;
                    let m_typr = res_typ.meet(m_typa.clone(), repo)?;
                    tc!(Null <: &m_typr ; repo)?;
                    tc!(m_typa <: &m_typr ; repo)?;
                    match m_typa {
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &JoinZero ; repo)?;
                            for arg in args.iter() {
                                let arg_typ = self.read_reg(arg)?.clone();
                                let m_typ = elt_typ.clone().meet(arg_typ, repo)?;
                                tc!(elt_typ <: &m_typ ; repo)?;
                                self.write_reg(arg, m_typ)?;
                            }
                            Ok(())
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
            },
            Instr::FilledNewArrayRange(args, arr) => match last_result {
                None => Err(TypeError::MissingResult.into()),
                Some(res_typ) => {
                    let arr_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                    let m_typa = arr_typ.meet(Array(1, Box::new(Top)), repo)?;
                    tc!(Null <: &m_typa ; repo)?;
                    let m_typr = res_typ.meet(m_typa.clone(), repo)?;
                    tc!(Null <: &m_typr ; repo)?;
                    tc!(m_typa <: &m_typr ; repo)?;
                    match m_typa {
                        Array(1, elt_typ) => {
                            tc!(elt_typ <: &JoinZero ; repo)?;
                            for arg in args.iter() {
                                let arg_typ = self.read_reg(arg)?.clone();
                                let m_typ = elt_typ.clone().meet(arg_typ, repo)?;
                                tc!(elt_typ <: &m_typ ; repo)?;
                                self.write_reg(arg, m_typ)?;
                            }
                            Ok(())
                        }
                        _ => Err(TypeError::ExpectedArray.into()),
                    }
                }
            },
            Instr::FillArrayData(arr, _) => {
                let arr_typ = self.read_reg(*arr)?.clone();
                let m_typ = arr_typ.meet(Array(1, Box::new(Join32)), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*arr, m_typ)
            }

            Instr::Throw(ptr) => match last_exception {
                None => self.write_reg(*ptr, JAVA_LANG_THROWABLE.clone()),
                Some(e_typ) => {
                    tc!(e_typ <: &JAVA_LANG_THROWABLE ; repo)?;
                    let ptr_typ = self.read_reg(*ptr)?.clone();
                    let m_typ = ptr_typ.meet(e_typ, repo)?;
                    tc!(Null <: &m_typ ; repo)?;
                    self.write_reg(*ptr, m_typ)
                }
            },

            Instr::PackedSwitch(src, _) | Instr::SparseSwitch(src, _) => {
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::CmplFloat(dst, src1, src2) | Instr::CmpgFloat(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ1 = self.read_reg(*src1)?.clone();
                let src_typ2 = self.read_reg(*src2)?.clone();
                let m_typ = src_typ1.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src1, m_typ)?;
                let m_typ = src_typ2.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src2, m_typ)
            }
            Instr::CmplDouble(dst, src1, src2) | Instr::CmpgDouble(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ1 = self.read_pair(*src1)?.clone();
                let src_typ2 = self.read_pair(*src2)?.clone();
                let m_typ = src_typ1.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src1, m_typ)?;
                let m_typ = src_typ2.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src2, m_typ)
            }
            Instr::CmpLong(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ1 = self.read_pair(*src1)?.clone();
                let src_typ2 = self.read_pair(*src2)?.clone();
                let m_typ = src_typ1.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src1, m_typ)?;
                let m_typ = src_typ2.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src2, m_typ)
            }

            Instr::IfEq(src1, src2, _) | Instr::IfNe(src1, src2, _) => {
                let src_typ1 = self.read_reg(*src1)?.clone();
                let src_typ2 = self.read_reg(*src2)?.clone();
                let m_typ1 = src_typ1.meet(JoinZero, repo)?;
                let m_typ2 = src_typ2.meet(JoinZero, repo)?;
                let m_typ = m_typ1.clone().meet(m_typ2.clone(), repo)?;
                tc!(MeetZero <: &m_typ; repo)?;
                self.write_reg(*src1, m_typ1)?;
                self.write_reg(*src2, m_typ2)
            }
            Instr::IfLt(src1, src2, _)
            | Instr::IfGe(src1, src2, _)
            | Instr::IfGt(src1, src2, _)
            | Instr::IfLe(src1, src2, _) => {
                let src_typ1 = self.read_reg(*src1)?.clone();
                let src_typ2 = self.read_reg(*src2)?.clone();
                let m_typ1 = src_typ1.meet(Integer, repo)?;
                let m_typ2 = src_typ2.meet(Integer, repo)?;
                let m_typ = m_typ1.clone().meet(m_typ2.clone(), repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*src1, m_typ1)?;
                self.write_reg(*src2, m_typ2)
            }

            Instr::IfEqz(src, _) | Instr::IfNez(src, _) => {
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IfLtz(src, _)
            | Instr::IfGez(src, _)
            | Instr::IfGtz(src, _)
            | Instr::IfLez(src, _) => {
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::Aget(dst, arr, idx) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Join32, repo)?;
                tc!(Meet32 <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;

                // needs to come after the write of dst reg
                // in case the same register is used
                let idx_typ = self.read_reg(*idx)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let arr_typ = self.read_reg(*arr)?.clone();
                let m_typ = arr_typ.meet(Array(1, Box::new(Join32)), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*arr, m_typ)
            }
            Instr::AgetBoolean(dst, arr, idx)
            | Instr::AgetByte(dst, arr, idx)
            | Instr::AgetChar(dst, arr, idx)
            | Instr::AgetShort(dst, arr, idx) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;

                // needs to come after the write of dst reg
                // in case the same register is used
                let idx_typ = self.read_reg(*idx)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let arr_typ = self.read_reg(*arr)?.clone();
                let m_typ = arr_typ.meet(Array(1, Box::new(Integer)), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*arr, m_typ)
            }
            Instr::AgetWide(dst, arr, idx) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Join64, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;

                // needs to come after the write of dst reg
                // in case the same register is used
                let idx_typ = self.read_reg(*idx)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let arr_typ = self.read_reg(*arr)?.clone();
                let m_typ = arr_typ.meet(Array(1, Box::new(Join64)), repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*arr, m_typ)
            }
            Instr::AgetObject(dst, arr, idx) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typd = dst_typ.meet(JAVA_LANG_OBJECT.clone(), repo)?;
                tc!(Null <: &m_typd ; repo)?;
                self.write_reg(*dst, Top)?;

                // needs to come after the write of dst reg
                // in case the same register is used
                let idx_typ = self.read_reg(*idx)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let arr_typ = self.read_reg(*arr)?.clone();
                let m_typ = match m_typd {
                    Array(n, elt_typ) => arr_typ.meet(Array(n + 1, elt_typ), repo)?,
                    _ => arr_typ.meet(Array(1, Box::new(m_typd)), repo)?,
                };
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*arr, m_typ)
            }
            Instr::Aput(src, arr, idx) => {
                let idx_typ = self.read_reg(*idx)?.clone();
                let arr_typ = self.read_reg(*arr)?.clone();
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let m_typ = arr_typ.meet(Array(1, Box::new(Join32)), repo)?;
                tc!(Null <: &m_typ ; repo)?;

                match m_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &Join32 ; repo)?;
                        let m_typ = elt_typ.meet(src_typ, repo)?;
                        tc!(Meet32 <: &m_typ ; repo)?;
                        self.write_reg(*src, m_typ)
                    }
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            Instr::AputBoolean(src, arr, idx)
            | Instr::AputByte(src, arr, idx)
            | Instr::AputChar(src, arr, idx)
            | Instr::AputShort(src, arr, idx) => {
                let idx_typ = self.read_reg(*idx)?.clone();
                let arr_typ = self.read_reg(*arr)?.clone();
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let m_typ = arr_typ.meet(Array(1, Box::new(Integer)), repo)?;
                tc!(Null <: &m_typ ; repo)?;

                match m_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &Integer ; repo)?;
                        let m_typ = elt_typ.meet(src_typ, repo)?;
                        tc!(Integer <: &m_typ ; repo)?;
                        self.write_reg(*src, m_typ)
                    }
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            Instr::AputWide(src, arr, idx) => {
                let idx_typ = self.read_reg(*idx)?.clone();
                let arr_typ = self.read_reg(*arr)?.clone();
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let m_typ = arr_typ.meet(Array(1, Box::new(Join64)), repo)?;
                tc!(Null <: &m_typ ; repo)?;

                match m_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &Join64 ; repo)?;
                        let m_typ = elt_typ.meet(src_typ, repo)?;
                        tc!(Meet64 <: &m_typ ; repo)?;
                        self.write_pair(*src, m_typ)
                    }
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }
            Instr::AputObject(src, arr, idx) => {
                let idx_typ = self.read_reg(*idx)?.clone();
                let arr_typ = self.read_reg(*arr)?.clone();
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = idx_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*idx, m_typ)?;

                let m_typ = arr_typ.meet(Array(1, Box::new(JAVA_LANG_OBJECT.clone())), repo)?;
                tc!(Null <: &m_typ ; repo)?;

                match m_typ {
                    Array(1, elt_typ) => {
                        tc!(elt_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                        let m_typ = elt_typ.meet(src_typ, repo)?;
                        tc!(Null <: &m_typ ; repo)?;
                        self.write_reg(*src, m_typ)
                    }
                    Array(n, elt_typ) => {
                        assert!(n > 1);
                        let m_typ = src_typ.meet(Array(n - 1, elt_typ), repo)?;
                        tc!(Null <: &m_typ ; repo)?;
                        self.write_reg(*src, m_typ)
                    }
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }

            Instr::Iget(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join32 ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetBoolean(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    Err(TypeError::InvalidFieldType)?;
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetByte(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    Err(TypeError::InvalidFieldType)?;
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetChar(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    Err(TypeError::InvalidFieldType)?;
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetShort(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    Err(TypeError::InvalidFieldType)?;
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetWide(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join64 ; repo)?;
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }
            Instr::IgetObject(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)
            }

            Instr::Iput(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(Meet32 <: &fld_typ ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IputBoolean(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IputByte(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IputChar(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IputShort(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IputWide(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(Meet64 <: &fld_typ ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::IputObject(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?.clone();
                let m_typ = ptr_typ.meet(cls_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*ptr, m_typ)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::Sget(dst, field) => {
                let fld_typ = AbstractType::try_from(&field.get(dex)?.type_(dex)?)?;
                tc!(fld_typ <: &Join32 ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Meet32 <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::SgetBoolean(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::SgetByte(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::SgetChar(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::SgetShort(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }
            Instr::SgetWide(dst, field) => {
                let fld_typ = AbstractType::try_from(&field.get(dex)?.type_(dex)?)?;
                tc!(fld_typ <: &Join64 ; repo)?;
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Meet64 <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)
            }
            Instr::SgetObject(dst, field) => {
                let fld_typ = AbstractType::try_from(&field.get(dex)?.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(fld_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)
            }

            Instr::Sput(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join32 ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::SputBoolean(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::SputByte(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::SputChar(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::SputShort(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::SputWide(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join64 ; repo)?;
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ.clone(), repo)?;
                tc!(fld_typ <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::SputObject(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(fld_typ, repo)?;
                tc!(Null <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::InvokeVirtual(args, meth)
            | Instr::InvokeSuper(args, meth)
            | Instr::InvokeDirect(args, meth)
            | Instr::InvokeInterface(args, meth) => {
                //typechecking 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(TypeError::MissingThisArgument)?;
                let this_reg_typ = self.read_reg(this_reg)?.clone();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let m_typ = definer_typ.meet(this_reg_typ, repo)?;
                tc!(Null <: &m_typ; repo)?;
                self.write_reg(this_reg, m_typ)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            let arg_reg_typ = self.read_pair(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_pair(arg_reg, m_typ)?;
                        }
                        Object(_) | Array(_, _) => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = p_typ.meet(arg_reg_typ, repo)?;
                            tc!(Null <: &m_typ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                        _ => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                    }
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                match method.return_type(dex)? {
                    Type::Void => match last_result {
                        None => Ok(()),
                        Some(_) => Err(TypeError::MissingResult.into()),
                    },
                    meth_descr => match last_result {
                        None => Ok(()), // result will not been used
                        Some(status_typ) => {
                            let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                            let m_typ = status_typ.meet(meth_ret_typ.clone(), repo)?;
                            tc!(meth_ret_typ <: &m_typ ; repo)
                        }
                    },
                }
            }
            Instr::InvokeStatic(args, meth) => {
                //typechecking 'this' argument
                let mut args_it = args.iter();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            let arg_reg_typ = self.read_pair(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_pair(arg_reg, m_typ)?;
                        }
                        Object(_) | Array(_, _) => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = p_typ.meet(arg_reg_typ, repo)?;
                            tc!(Null <: &m_typ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                        _ => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                    }
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                match method.return_type(dex)? {
                    Type::Void => match last_result {
                        None => Ok(()),
                        Some(_) => Err(TypeError::MissingResult.into()),
                    },
                    meth_descr => match last_result {
                        None => Ok(()), // result will not been used
                        Some(status_typ) => {
                            let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                            let m_typ = status_typ.meet(meth_ret_typ.clone(), repo)?;
                            tc!(meth_ret_typ <: &m_typ ; repo)
                        }
                    },
                }
            }

            Instr::InvokeVirtualRange(args, meth)
            | Instr::InvokeSuperRange(args, meth)
            | Instr::InvokeDirectRange(args, meth)
            | Instr::InvokeInterfaceRange(args, meth) => {
                //typechecking 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(TypeError::MissingThisArgument)?;
                let this_reg_typ = self.read_reg(this_reg)?.clone();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let m_typ = definer_typ.meet(this_reg_typ, repo)?;
                tc!(Null <: &m_typ; repo)?;
                self.write_reg(this_reg, m_typ)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            let arg_reg_typ = self.read_pair(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_pair(arg_reg, m_typ)?;
                        }
                        Object(_) | Array(_, _) => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = p_typ.meet(arg_reg_typ, repo)?;
                            tc!(Null <: &m_typ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                        _ => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                    }
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                match method.return_type(dex)? {
                    Type::Void => match last_result {
                        None => Ok(()),
                        Some(_) => Err(TypeError::MissingResult.into()),
                    },
                    meth_descr => match last_result {
                        None => Ok(()), // result will not been used
                        Some(status_typ) => {
                            let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                            let m_typ = status_typ.meet(meth_ret_typ.clone(), repo)?;
                            tc!(meth_ret_typ <: &m_typ ; repo)
                        }
                    },
                }
            }
            Instr::InvokeStaticRange(args, meth) => {
                //typechecking 'this' argument
                let mut args_it = args.iter();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            let arg_reg_typ = self.read_pair(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_pair(arg_reg, m_typ)?;
                        }
                        Object(_) | Array(_, _) => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = p_typ.meet(arg_reg_typ, repo)?;
                            tc!(Null <: &m_typ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                        _ => {
                            let arg_reg_typ = self.read_reg(arg_reg)?.clone();
                            let m_typ = arg_reg_typ.meet(p_typ.clone(), repo)?;
                            tc!(p_typ <: &m_typ ; repo)?;
                            self.write_reg(arg_reg, m_typ)?;
                        }
                    }
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                match method.return_type(dex)? {
                    Type::Void => match last_result {
                        None => Ok(()),
                        Some(_) => Err(TypeError::MissingResult.into()),
                    },
                    meth_descr => match last_result {
                        None => Ok(()), // result will not been used
                        Some(status_typ) => {
                            let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                            let m_typ = status_typ.meet(meth_ret_typ.clone(), repo)?;
                            tc!(meth_ret_typ <: &m_typ ; repo)
                        }
                    },
                }
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
            | Instr::UshrIntLit8(dst, src, _) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::AndIntLit16(dst, src, _)
            | Instr::OrIntLit16(dst, src, _)
            | Instr::XorIntLit16(dst, src, _)
            | Instr::AndIntLit8(dst, src, _)
            | Instr::OrIntLit8(dst, src, _)
            | Instr::XorIntLit8(dst, src, _) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::NegLong(dst, src) | Instr::NotLong(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::NegFloat(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::NegDouble(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }

            Instr::IntToLong(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IntToFloat(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::IntToDouble(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }
            Instr::LongToInt(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::LongToFloat(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::LongToDouble(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src, m_typ)
            }
            Instr::FloatToInt(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(JoinZero, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src, Float)
            }
            Instr::FloatToLong(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src, Float)
            }
            Instr::FloatToDouble(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src, Float)
            }
            Instr::DoubleToInt(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src, Double)
            }
            Instr::DoubleToLong(dst, src) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src, Double)
            }
            Instr::DoubleToFloat(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_pair(*src)?.clone();
                let m_typ = src_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src, Double)
            }
            Instr::IntToByte(dst, src)
            | Instr::IntToChar(dst, src)
            | Instr::IntToShort(dst, src) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src_typ = self.read_reg(*src)?.clone();
                let m_typ = src_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src, m_typ)
            }

            Instr::AddInt(dst, src1, src2)
            | Instr::SubInt(dst, src1, src2)
            | Instr::MulInt(dst, src1, src2)
            | Instr::DivInt(dst, src1, src2)
            | Instr::RemInt(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_reg(*src1)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ = src1_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src1, m_typ)?;
                let m_typ = src2_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src2, m_typ)
            }
            Instr::AndInt(dst, src1, src2)
            | Instr::OrInt(dst, src1, src2)
            | Instr::XorInt(dst, src1, src2)
            | Instr::ShlInt(dst, src1, src2)
            | Instr::ShrInt(dst, src1, src2)
            | Instr::UshrInt(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_reg(*src1)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ = src1_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src1, m_typ)?;
                let m_typ = src2_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ ; repo)?;
                self.write_reg(*src2, m_typ)
            }

            Instr::AddLong(dst, src1, src2)
            | Instr::SubLong(dst, src1, src2)
            | Instr::MulLong(dst, src1, src2)
            | Instr::DivLong(dst, src1, src2)
            | Instr::RemLong(dst, src1, src2) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_pair(*src1)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typ = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src1, m_typ)?;
                let m_typ = src2_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src2, m_typ)
            }
            Instr::AndLong(dst, src1, src2)
            | Instr::OrLong(dst, src1, src2)
            | Instr::XorLong(dst, src1, src2) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_pair(*src1)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typ = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src1, m_typ)?;
                let m_typ = src2_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*src2, m_typ)
            }
            Instr::ShlLong(dst, src1, src2)
            | Instr::ShrLong(dst, src1, src2)
            | Instr::UshrLong(dst, src1, src2) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_pair(*src1)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ1 = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ1 ; repo)?;
                let m_typ2 = src2_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ2 ; repo)?;
                self.write_pair(*src1, m_typ1)?;
                self.write_reg(*src2, m_typ2)
            }

            Instr::AddFloat(dst, src1, src2)
            | Instr::SubFloat(dst, src1, src2)
            | Instr::MulFloat(dst, src1, src2)
            | Instr::DivFloat(dst, src1, src2)
            | Instr::RemFloat(dst, src1, src2) => {
                let dst_typ = self.read_reg(*dst)?.clone();
                let m_typ = dst_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_reg(*src1)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ = src1_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src1, m_typ)?;
                let m_typ = src2_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ ; repo)?;
                self.write_reg(*src2, m_typ)
            }

            Instr::AddDouble(dst, src1, src2)
            | Instr::SubDouble(dst, src1, src2)
            | Instr::MulDouble(dst, src1, src2)
            | Instr::DivDouble(dst, src1, src2)
            | Instr::RemDouble(dst, src1, src2) => {
                let dst_typ = self.read_pair(*dst)?.clone();
                let m_typ = dst_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*dst, Top)?;
                // needs to come after the write of dst reg
                // in case the same register is used
                let src1_typ = self.read_pair(*src1)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typ = src1_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src1, m_typ)?;
                let m_typ = src2_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ ; repo)?;
                self.write_pair(*src2, m_typ)
            }

            Instr::AddInt2addr(bid, src2)
            | Instr::SubInt2addr(bid, src2)
            | Instr::MulInt2addr(bid, src2)
            | Instr::DivInt2addr(bid, src2)
            | Instr::RemInt2addr(bid, src2) => {
                let bid_typ = self.read_reg(*bid)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typb = bid_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typb ; repo)?;
                let m_typ2 = src2_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ2 ; repo)?;
                self.write_reg(*bid, m_typb)?;
                self.write_reg(*src2, m_typ2)
            }
            Instr::AndInt2addr(bid, src2)
            | Instr::OrInt2addr(bid, src2)
            | Instr::XorInt2addr(bid, src2)
            | Instr::ShlInt2addr(bid, src2)
            | Instr::ShrInt2addr(bid, src2)
            | Instr::UshrInt2addr(bid, src2) => {
                let src1_typ = self.read_reg(*bid)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ1 = src1_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ1 ; repo)?;
                let m_typ2 = src2_typ.meet(Integer, repo)?;
                tc!(Integer <: &m_typ2 ; repo)?;
                self.write_reg(*bid, m_typ1)?;
                self.write_reg(*src2, m_typ2)
            }

            Instr::AddLong2addr(bid, src2)
            | Instr::SubLong2addr(bid, src2)
            | Instr::MulLong2addr(bid, src2)
            | Instr::DivLong2addr(bid, src2)
            | Instr::RemLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typ1 = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ1 ; repo)?;
                let m_typ2 = src2_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ2 ; repo)?;
                self.write_pair(*bid, m_typ1)?;
                self.write_pair(*src2, m_typ2)
            }
            Instr::AndLong2addr(bid, src2)
            | Instr::OrLong2addr(bid, src2)
            | Instr::XorLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typ1 = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ1 ; repo)?;
                let m_typ2 = src2_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ2 ; repo)?;
                self.write_pair(*bid, m_typ1)?;
                self.write_pair(*src2, m_typ2)
            }
            Instr::ShlLong2addr(bid, src2)
            | Instr::ShrLong2addr(bid, src2)
            | Instr::UshrLong2addr(bid, src2) => {
                let src1_typ = self.read_pair(*bid)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typ1 = src1_typ.meet(Long, repo)?;
                tc!(Long <: &m_typ1 ; repo)?;
                let m_typ2 = src2_typ.meet(JoinZero, repo)?;
                tc!(MeetZero <: &m_typ2 ; repo)?;
                self.write_pair(*bid, m_typ1)?;
                self.write_reg(*src2, m_typ2)
            }

            Instr::AddFloat2addr(bid, src2)
            | Instr::SubFloat2addr(bid, src2)
            | Instr::MulFloat2addr(bid, src2)
            | Instr::DivFloat2addr(bid, src2)
            | Instr::RemFloat2addr(bid, src2) => {
                let bid_typ = self.read_reg(*bid)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                let m_typb = bid_typ.meet(Float, repo)?;
                tc!(Float <: &m_typb ; repo)?;
                let m_typ2 = src2_typ.meet(Float, repo)?;
                tc!(Float <: &m_typ2 ; repo)?;
                self.write_reg(*bid, m_typb)?;
                self.write_reg(*src2, m_typ2)
            }

            Instr::AddDouble2addr(bid, src2)
            | Instr::SubDouble2addr(bid, src2)
            | Instr::MulDouble2addr(bid, src2)
            | Instr::DivDouble2addr(bid, src2)
            | Instr::RemDouble2addr(bid, src2) => {
                let bid_typ = self.read_pair(*bid)?.clone();
                let src2_typ = self.read_pair(*src2)?.clone();
                let m_typb = bid_typ.meet(Double, repo)?;
                tc!(Double <: &m_typb ; repo)?;
                let m_typ2 = src2_typ.meet(Double, repo)?;
                tc!(Double <: &m_typ2 ; repo)?;
                self.write_pair(*bid, m_typb)?;
                self.write_pair(*src2, m_typ2)
            }

            Instr::InvokeCustom(_, _)
            | Instr::InvokeCustomRange(_, _)
            | Instr::InvokePolymorphic(_, _, _)
            | Instr::InvokePolymorphicRange(_, _, _)
            | Instr::ConstMethodHandle(_, _)
            | Instr::ConstMethodType(_, _) => {
                unimplemented!("support for {:?}", instr)
            }

            Instr::PackedSwitchPayload(_, _)
            | Instr::SparseSwitchPayload(_, _)
            | Instr::FillArrayDataPayload(_) => panic!("internal error"),
        }
    }

    fn entry_reached(&self, class: &Class, method: &Method, repo: &Repo) -> AnalysisResult<()> {
        let nb_registers = method
            .code()
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .registers_size();

        let mut registers = Vec::new();
        for _ in 0..nb_registers {
            registers.push(AbstractType::Bottom);
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

        if !method.is_static() {
            let this_name = class.name().to_string();
            registers[first_param_reg - 1] = AbstractType::object_singleton(this_name);
        }

        let mut param_reg = first_param_reg;

        for type_descr in method.parameters_types() {
            let typ = AbstractType::try_from(type_descr)?;
            match typ {
                AbstractType::Long | AbstractType::Double => {
                    registers[param_reg] = typ.clone();
                    param_reg += 1;
                    registers[param_reg] = typ;
                    param_reg += 1;
                }
                _ => {
                    registers[param_reg] = typ;
                    param_reg += 1;
                }
            }
        }

        if self.registers.len() != nb_registers {
            return Err(AnalysisError::Internal(
                "invalid number of registers".to_string(),
            ));
        }

        for (computed, expected) in self.registers.iter().zip(registers) {
            tc!(expected <: computed ; repo)?;
        }

        Ok(())
    }
}
