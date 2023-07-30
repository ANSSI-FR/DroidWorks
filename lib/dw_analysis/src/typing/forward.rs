use crate::controlflow::Branch;
use crate::dataflow::AbstractForwardState;
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

impl<'a> AbstractForwardState<'a> for State {
    type Context<'c> = Repo<'c>;
    type Error = AnalysisError;

    fn init(method: &Method, class: &Class) -> AnalysisResult<Self> {
        // Note1: method registers layout:
        // [...local registers ...]
        // ['this' register (if method is not static)]
        // [...parameters...]

        // Note2: nb_registers = local_regs + this_reg + param_regs

        let nb_registers = method
            .code()
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .registers_size();

        let mut registers = Vec::new();
        for _ in 0..nb_registers {
            registers.push(AbstractType::Top);
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

    fn join(&mut self, other: &Self, repo: &Repo) -> AnalysisResult<()> {
        if self.registers.len() != other.registers.len() {
            return Err(TypeError::IncompatibleStates.into());
        }
        for i in 0..self.registers.len() {
            let reg1 = self.registers[i].clone();
            let reg2 = other.registers[i].clone();
            self.registers[i] = reg1.join(reg2, repo)?;
        }

        self.last_exception = match (&self.last_exception, &other.last_exception) {
            (Some(st1), Some(st2)) => {
                let t1 = st1.clone();
                let t2 = st2.clone();
                Some(t1.join(t2, repo)?)
            }
            _ => None,
        };

        self.last_result = match (&self.last_result, &other.last_result) {
            (Some(st1), Some(st2)) => {
                let t1 = st1.clone();
                let t2 = st2.clone();
                Some(t1.join(t2, repo)?)
            }
            _ => None,
        };

        assert!(self.expected == other.expected);

        Ok(())
    }

    fn transfer_branch(&mut self, branch: &Branch, repo: &Repo) -> AnalysisResult<()> {
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
            Branch::Catch(t) => {
                let typ = AbstractType::try_from(t)?;
                tc!(typ <: &*JAVA_LANG_THROWABLE ; repo)?;
                self.last_exception = Some(typ);
                Ok(())
            }
            Branch::CatchAll => {
                self.last_exception = Some((*JAVA_LANG_THROWABLE).clone());
                Ok(())
            }
            Branch::CastSuccess(ptr, cls) => {
                let cls_typ = AbstractType::try_from(cls)?;
                self.write_reg(*ptr, cls_typ)
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
                let src_typ = self.read_reg(*src)?.clone();
                tc!(src_typ <: &Join32 ; repo)?;
                self.write_reg(*dst, src_typ)
            }

            Instr::MoveWide(dst, src)
            | Instr::MoveWideFrom16(dst, src)
            | Instr::MoveWide16(dst, src) => {
                let src_typ = self.read_pair(*src)?.clone();
                tc!(src_typ <: &Join64 ; repo)?;
                self.write_pair(*dst, src_typ)
            }

            Instr::MoveObject(dst, src)
            | Instr::MoveObjectFrom16(dst, src)
            | Instr::MoveObject16(dst, src) => {
                let src_typ = self.read_reg(*src)?.clone();
                tc!(src_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                self.write_reg(*dst, src_typ)
            }

            Instr::MoveResult(dst) => match last_result {
                Some(src_typ) => {
                    tc!(src_typ <: &Join32 ; repo)?;
                    self.write_reg(*dst, src_typ)
                }
                _ => Err(TypeError::MissingResult.into()),
            },
            Instr::MoveResultWide(dst) => match last_result {
                Some(src_typ) => {
                    tc!(src_typ <: &Join64 ; repo)?;
                    self.write_pair(*dst, src_typ)
                }
                _ => Err(TypeError::MissingResult.into()),
            },
            Instr::MoveResultObject(dst) => match last_result {
                Some(src_typ) => {
                    tc!(src_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                    self.write_reg(*dst, src_typ)
                }
                _ => Err(TypeError::MissingResult.into()),
            },
            Instr::MoveException(dst) => match last_exception {
                Some(src_typ) => {
                    tc!(src_typ <: &*JAVA_LANG_THROWABLE ; repo)?;
                    self.write_reg(*dst, src_typ)
                }
                _ => Err(TypeError::MissingException.into()),
            },

            Instr::ReturnVoid => match &self.expected {
                None => Ok(()),
                Some(_) => Err(TypeError::BadReturnType.into()),
            },
            Instr::Return(ret) => match &self.expected {
                Some(exp_typ) => {
                    let ret_typ = self.read_reg(*ret)?;
                    tc!(exp_typ <: &Join32 ; repo)?;
                    tc!(ret_typ <: exp_typ ; repo)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },
            Instr::ReturnWide(ret) => match &self.expected {
                Some(exp_typ) => {
                    let ret_typ = self.read_pair(*ret)?;
                    tc!(exp_typ <: &Join64 ; repo)?;
                    tc!(ret_typ <: exp_typ ; repo)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },
            Instr::ReturnObject(ret) => match &self.expected {
                Some(exp_typ) => {
                    tc!(exp_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                    let ret_typ = self.read_reg(*ret)?;
                    tc!(ret_typ <: exp_typ ; repo)
                }
                _ => Err(TypeError::BadReturnType.into()),
            },

            Instr::Const4(dst, val) => {
                self.write_reg(*dst, if *val == 0 { MeetZero } else { Meet32 })
            }
            Instr::Const16(dst, val) | Instr::ConstHigh16(dst, val) => {
                self.write_reg(*dst, if *val == 0 { MeetZero } else { Meet32 })
            }
            Instr::Const(dst, val) => {
                self.write_reg(*dst, if *val == 0 { MeetZero } else { Meet32 })
            }

            Instr::ConstWide16(dst, _)
            | Instr::ConstWideHigh16(dst, _)
            | Instr::ConstWide32(dst, _)
            | Instr::ConstWide(dst, _) => self.write_pair(*dst, Meet64),

            Instr::ConstString(dst, _) | Instr::ConstStringJumbo(dst, _) => {
                self.write_reg(*dst, JAVA_LANG_STRING.clone())
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
                self.write_reg(*dst, JAVA_LANG_CLASS.clone())
            }
            Instr::MonitorEnter(ptr) | Instr::MonitorExit(ptr) => {
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &*JAVA_LANG_OBJECT ; repo)
            }
            Instr::CheckCast(ptr, cls) => {
                let _ = AbstractType::try_from(WithDex::new(dex, *cls))?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &*JAVA_LANG_OBJECT ; repo)
            }
            Instr::InstanceOf(dst, ptr, cls) => {
                let cls_typ = AbstractType::try_from(WithDex::new(dex, *cls))?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(cls_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                tc!(ptr_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                self.write_reg(*dst, Integer)
            }

            Instr::ArrayLength(dst, ptr) => {
                let ptr_typ = self.read_reg(*ptr)?;
                if matches!(ptr_typ, Array(_, _)) || ptr_typ.subseteq(&Null, repo)? {
                    self.write_reg(*dst, Integer)
                } else {
                    Err(TypeError::ExpectedArray.into())
                }
            }

            Instr::NewInstance(dst, descr) => {
                let typ = AbstractType::try_from(WithDex::new(dex, *descr))?;
                match typ {
                    Object(_) => self.write_reg(*dst, typ),
                    _ => Err(TypeError::ExpectedClass.into()),
                }
            }
            Instr::NewArray(dst, siz, arr) => {
                let arr_typ = AbstractType::try_from(WithDex::new(dex, *arr))?;
                let siz_typ = self.read_reg(*siz)?;
                tc!(siz_typ <: &Integer ; repo)?;
                match arr_typ {
                    Array(_, _) => self.write_reg(*dst, arr_typ),
                    _ => Err(TypeError::ExpectedArray.into()),
                }
            }

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

            Instr::Throw(ptr) => {
                let ptr_typ = self.read_reg(*ptr)?.clone();
                tc!(ptr_typ <: &*JAVA_LANG_THROWABLE ; repo)?;
                self.last_exception = Some(ptr_typ);
                Ok(())
            }

            Instr::PackedSwitch(src, _) | Instr::SparseSwitch(src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)
            }

            Instr::CmplFloat(dst, src1, src2) | Instr::CmpgFloat(dst, src1, src2) => {
                let src_typ1 = self.read_reg(*src1)?;
                let src_typ2 = self.read_reg(*src2)?;
                tc!(src_typ1 <: &Float ; repo)?;
                tc!(src_typ2 <: &Float ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::CmplDouble(dst, src1, src2) | Instr::CmpgDouble(dst, src1, src2) => {
                let src_typ1 = self.read_pair(*src1)?;
                let src_typ2 = self.read_pair(*src2)?;
                tc!(src_typ1 <: &Double ; repo)?;
                tc!(src_typ2 <: &Double ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::CmpLong(dst, src1, src2) => {
                let src_typ1 = self.read_pair(*src1)?;
                let src_typ2 = self.read_pair(*src2)?;
                tc!(src_typ1 <: &Long ; repo)?;
                tc!(src_typ2 <: &Long ; repo)?;
                self.write_reg(*dst, Integer)
            }

            Instr::IfEq(src1, src2, _) | Instr::IfNe(src1, src2, _) => {
                let src_typ1 = self.read_reg(*src1)?.clone();
                let src_typ2 = self.read_reg(*src2)?.clone();
                let src_typ = src_typ1.join(src_typ2, repo)?;
                tc!(src_typ <: &JoinZero; repo)
            }
            Instr::IfLt(src1, src2, _)
            | Instr::IfGe(src1, src2, _)
            | Instr::IfGt(src1, src2, _)
            | Instr::IfLe(src1, src2, _) => {
                let src_typ1 = self.read_reg(*src1)?.clone();
                let src_typ2 = self.read_reg(*src2)?.clone();
                let src_typ = src_typ1.join(src_typ2, repo)?;
                tc!(src_typ <: &Integer; repo)
            }

            Instr::IfEqz(src, _) | Instr::IfNez(src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &JoinZero; repo)
            }
            Instr::IfLtz(src, _)
            | Instr::IfGez(src, _)
            | Instr::IfGtz(src, _)
            | Instr::IfLez(src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer; repo)
            }

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
            }

            Instr::Iget(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(Meet32 <: &fld_typ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::IgetBoolean(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                //let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::IgetByte(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                //let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::IgetChar(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                //let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::IgetShort(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                //let fld_typ = AbstractType::try_from(&fld_typ)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::IgetWide(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(Meet64 <: &fld_typ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_pair(*dst, fld_typ)
            }
            Instr::IgetObject(dst, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                self.write_reg(*dst, fld_typ)
            }

            Instr::Iput(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join32 ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
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
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
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
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
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
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
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
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::IputWide(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Join64 ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::IputObject(src, ptr, field) => {
                let field = field.get(dex)?;
                let class_name = field.class_name(dex)?;
                let cls_typ = AbstractType::object_singleton(class_name);
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let ptr_typ = self.read_reg(*ptr)?;
                tc!(ptr_typ <: &cls_typ ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }

            Instr::Sget(dst, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &Join32 ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::SgetBoolean(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Integer ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::SgetByte(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Integer ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::SgetChar(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Integer ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::SgetShort(dst, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&field.type_(dex)?)?;
                tc!(fld_typ <: &Integer ; repo)?;
                self.write_reg(*dst, fld_typ)
            }
            Instr::SgetWide(dst, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &Join64 ; repo)?;
                self.write_pair(*dst, fld_typ)
            }
            Instr::SgetObject(dst, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                self.write_reg(*dst, fld_typ)
            }

            Instr::Sput(src, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &Join32 ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputBoolean(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Boolean {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputByte(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Byte {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputChar(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Char {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputShort(src, field) => {
                let field = field.get(dex)?;
                let fld_typ = field.type_(dex)?;
                if fld_typ != Type::Short {
                    return Err(TypeError::InvalidFieldType.into());
                }
                let fld_typ = AbstractType::try_from(&fld_typ)?;
                tc!(fld_typ <: &Integer ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputWide(src, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &Join64 ; repo)?;
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }
            Instr::SputObject(src, field) => {
                let fld_typ = AbstractType::try_from(WithDex::new(dex, *field))?;
                tc!(fld_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &fld_typ ; repo)
            }

            Instr::InvokeVirtual(args, meth)
            | Instr::InvokeSuper(args, meth)
            | Instr::InvokeDirect(args, meth)
            | Instr::InvokeInterface(args, meth) => {
                // typechecking 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(TypeError::MissingThisArgument)?;
                let this_reg_typ = self.read_reg(this_reg)?;

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                tc!(this_reg_typ <: &definer_typ ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    let arg_typ = match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    tc!(arg_typ <: &p_typ ; repo)?;
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                // saving return type if non void
                let meth_descr = method.return_type(dex)?;
                if meth_descr != Type::Void {
                    let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                    self.last_result = Some(meth_ret_typ);
                }

                Ok(())
            }
            Instr::InvokeStatic(args, meth) => {
                // typechecking 'this' argument
                let mut args_it = args.iter();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    let arg_typ = match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    tc!(arg_typ <: &p_typ ; repo)?;
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                // saving return type if non void
                let meth_descr = method.return_type(dex)?;
                if meth_descr != Type::Void {
                    let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                    self.last_result = Some(meth_ret_typ);
                }

                Ok(())
            }

            Instr::InvokeVirtualRange(args, meth)
            | Instr::InvokeSuperRange(args, meth)
            | Instr::InvokeDirectRange(args, meth)
            | Instr::InvokeInterfaceRange(args, meth) => {
                // typechecking 'this' argument
                let mut args_it = args.iter();
                let this_reg = args_it.next().ok_or(TypeError::MissingThisArgument)?;
                let this_reg_typ = self.read_reg(this_reg)?;

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;
                tc!(this_reg_typ <: &definer_typ ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    let arg_typ = match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    tc!(arg_typ <: &p_typ ; repo)?;
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                // saving return type if non void
                let meth_descr = method.return_type(dex)?;
                if meth_descr != Type::Void {
                    let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                    self.last_result = Some(meth_ret_typ);
                }

                Ok(())
            }
            Instr::InvokeStaticRange(args, meth) => {
                // typechecking 'this' argument
                let mut args_it = args.iter();

                let method = meth.get(dex)?;
                let definer_typ = AbstractType::try_from(&method.definer(dex)?)?;
                tc!(definer_typ <: &*JAVA_LANG_OBJECT ; repo)?;

                // typechecking arguments against expected parameters
                for p_descr in &method.parameters_types(dex)? {
                    let p_typ = AbstractType::try_from(p_descr)?;
                    let arg_reg = args_it.next().ok_or(TypeError::BadArity)?;
                    let arg_typ = match p_typ {
                        Double | Long => {
                            args_it.next(); // consume the following register as it's a pair
                            self.read_pair(arg_reg)?
                        }
                        _ => self.read_reg(arg_reg)?,
                    };
                    tc!(arg_typ <: &p_typ ; repo)?;
                }
                // all args should have been consumed
                if args_it.next().is_some() {
                    return Err(TypeError::BadArity.into());
                }

                // saving return type if non void
                let meth_descr = method.return_type(dex)?;
                if meth_descr != Type::Void {
                    let meth_ret_typ = AbstractType::try_from(&meth_descr)?;
                    self.last_result = Some(meth_ret_typ);
                }

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
            | Instr::UshrIntLit8(dst, src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Integer)
            }

            Instr::AndIntLit16(dst, src, _)
            | Instr::OrIntLit16(dst, src, _)
            | Instr::XorIntLit16(dst, src, _)
            | Instr::AndIntLit8(dst, src, _)
            | Instr::OrIntLit8(dst, src, _)
            | Instr::XorIntLit8(dst, src, _) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Integer)
            }

            Instr::NegLong(dst, src) | Instr::NotLong(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Long ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::NegFloat(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Float ; repo)?;
                self.write_reg(*dst, Float)
            }
            Instr::NegDouble(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Double ; repo)?;
                self.write_pair(*dst, Double)
            }

            Instr::IntToLong(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::IntToFloat(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Float)
            }
            Instr::IntToDouble(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_pair(*dst, Double)
            }
            Instr::LongToInt(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Long ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::LongToFloat(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Long ; repo)?;
                self.write_reg(*dst, Float)
            }
            Instr::LongToDouble(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Long ; repo)?;
                self.write_pair(*dst, Double)
            }
            Instr::FloatToInt(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Float ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::FloatToLong(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Float ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::FloatToDouble(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Float ; repo)?;
                self.write_pair(*dst, Double)
            }
            Instr::DoubleToInt(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Double ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::DoubleToLong(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Double ; repo)?;
                self.write_pair(*dst, Long)
            }
            Instr::DoubleToFloat(dst, src) => {
                let src_typ = self.read_pair(*src)?;
                tc!(src_typ <: &Double ; repo)?;
                self.write_reg(*dst, Float)
            }
            Instr::IntToByte(dst, src)
            | Instr::IntToChar(dst, src)
            | Instr::IntToShort(dst, src) => {
                let src_typ = self.read_reg(*src)?;
                tc!(src_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Integer)
            }

            Instr::AddInt(dst, src1, src2)
            | Instr::SubInt(dst, src1, src2)
            | Instr::MulInt(dst, src1, src2)
            | Instr::DivInt(dst, src1, src2)
            | Instr::RemInt(dst, src1, src2) => {
                let src1_typ = self.read_reg(*src1)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Integer ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Integer)
            }
            Instr::AndInt(dst, src1, src2)
            | Instr::OrInt(dst, src1, src2)
            | Instr::XorInt(dst, src1, src2)
            | Instr::ShlInt(dst, src1, src2)
            | Instr::ShrInt(dst, src1, src2)
            | Instr::UshrInt(dst, src1, src2) => {
                let src1_typ = self.read_reg(*src1)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Integer ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_reg(*dst, Integer)
            }

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

            Instr::AddFloat(dst, src1, src2)
            | Instr::SubFloat(dst, src1, src2)
            | Instr::MulFloat(dst, src1, src2)
            | Instr::DivFloat(dst, src1, src2)
            | Instr::RemFloat(dst, src1, src2) => {
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
            }

            Instr::AddInt2addr(bid, src2)
            | Instr::SubInt2addr(bid, src2)
            | Instr::MulInt2addr(bid, src2)
            | Instr::DivInt2addr(bid, src2)
            | Instr::RemInt2addr(bid, src2) => {
                let src1_typ = self.read_reg(*bid)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Integer ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_reg(*bid, Integer)
            }
            Instr::AndInt2addr(bid, src2)
            | Instr::OrInt2addr(bid, src2)
            | Instr::XorInt2addr(bid, src2)
            | Instr::ShlInt2addr(bid, src2)
            | Instr::ShrInt2addr(bid, src2)
            | Instr::UshrInt2addr(bid, src2) => {
                let src1_typ = self.read_reg(*bid)?.clone();
                let src2_typ = self.read_reg(*src2)?.clone();
                tc!(src1_typ <: &Integer ; repo)?;
                tc!(src2_typ <: &Integer ; repo)?;
                self.write_reg(*bid, Integer)
            }

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

            Instr::AddFloat2addr(bid, src2)
            | Instr::SubFloat2addr(bid, src2)
            | Instr::MulFloat2addr(bid, src2)
            | Instr::DivFloat2addr(bid, src2)
            | Instr::RemFloat2addr(bid, src2) => {
                let src1_typ = self.read_reg(*bid)?;
                let src2_typ = self.read_reg(*src2)?;
                tc!(src1_typ <: &Float ; repo)?;
                tc!(src2_typ <: &Float ; repo)?;
                self.write_reg(*bid, Float)
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
                unimplemented!("support for {:?}", instr)
            }

            Instr::PackedSwitchPayload(_, _)
            | Instr::SparseSwitchPayload(_, _)
            | Instr::FillArrayDataPayload(_) => panic!("internal error"),
        }
    }
}
