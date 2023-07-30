//! Dalvik bytecode-related structures.

use crate::errors::{DexError, DexResult};
use crate::fields::FieldIdItem;
use crate::instrs::{Instr, Instruction, LabeledInstr};
use crate::methods::MethodIdItem;
use crate::strings::StringIdItem;
use crate::types::{Type, TypeIdItem};
use crate::values::{EncodedArray, EncodedArrayItem};
use crate::{Addr, Dex, DexCollection, DexIndex, Index, PrettyPrint};
use dw_utils::leb::{Sleb128, Uleb128};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::RwLock;

#[derive(Debug)]
pub struct CallSiteIdItem {
    pub(crate) index: Index<CallSiteIdItem>,
    pub(crate) call_site_off: Index<EncodedArrayItem>,
}

impl DexIndex for Index<CallSiteIdItem> {
    type T = CallSiteIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.call_site_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("CallSiteIdItem".to_string()))
    }
}

impl DexCollection for CallSiteIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl PrettyPrint for CallSiteIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        write!(f, "callsite")?;
        self.arguments(dex)?.pp(f, dex)?;
        Ok(())
    }
}

impl CallSiteIdItem {
    pub(crate) fn arguments<'a>(&self, dex: &'a Dex) -> DexResult<&'a EncodedArray> {
        Ok(&self.call_site_off.get(dex)?.value)
    }

    pub(crate) fn size(&self) -> usize {
        4
    }
}

#[derive(Debug)]
pub struct MethodHandleItem {
    pub(crate) index: Index<MethodHandleItem>,
    // Specs describes method handle as method_handle_type: u16 + field_or_method_id.
    // Parsing resolves it as:
    pub(crate) method_handle: MethodHandle,
}

impl DexIndex for Index<MethodHandleItem> {
    type T = MethodHandleItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.method_handle_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("MethodHandleItem".to_string()))
    }
}

impl DexCollection for MethodHandleItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl PrettyPrint for MethodHandleItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let n = match self.method_handle {
            MethodHandle::StaticPut(_) => "static-put",
            MethodHandle::StaticGet(_) => "static-get",
            MethodHandle::InstancePut(_) => "instance-put",
            MethodHandle::InstanceGet(_) => "instance-get",
            MethodHandle::InvokeStatic(_) => "invoke-static",
            MethodHandle::InvokeInstance(_) => "invoke-instance",
            MethodHandle::InvokeConstructor(_) => "invoke-constructor",
            MethodHandle::InvokeDirect(_) => "invoke-direct",
            MethodHandle::InvokeInterface(_) => "invoke-interface",
        };
        write!(f, "{n}")?;

        match self.method_handle {
            MethodHandle::StaticPut(field)
            | MethodHandle::StaticGet(field)
            | MethodHandle::InstancePut(field)
            | MethodHandle::InstanceGet(field) => {
                write!(f, ")")?;
                field.get(dex)?.pp(f, dex)?;
                write!(f, ")")?;
                Ok(())
            }
            MethodHandle::InvokeStatic(method)
            | MethodHandle::InvokeInstance(method)
            | MethodHandle::InvokeConstructor(method)
            | MethodHandle::InvokeDirect(method)
            | MethodHandle::InvokeInterface(method) => {
                write!(f, ")")?;
                method.get(dex)?.pp(f, dex)?;
                write!(f, ")")?;
                Ok(())
            }
        }
    }
}

impl MethodHandleItem {
    pub(crate) fn size(&self) -> usize {
        8
    }
}

#[derive(Debug)]
pub(crate) enum MethodHandle {
    StaticPut(Index<FieldIdItem>),
    StaticGet(Index<FieldIdItem>),
    InstancePut(Index<FieldIdItem>),
    InstanceGet(Index<FieldIdItem>),
    InvokeStatic(Index<MethodIdItem>),
    InvokeInstance(Index<MethodIdItem>),
    InvokeConstructor(Index<MethodIdItem>),
    InvokeDirect(Index<MethodIdItem>),
    InvokeInterface(Index<MethodIdItem>),
}

#[derive(Debug)]
pub struct CodeItem {
    pub(crate) index: Index<CodeItem>,
    pub(crate) registers_size: usize,
    pub(crate) ins_size: usize,
    pub(crate) outs_size: usize,
    pub(crate) debug_info_off: Option<Index<DebugInfoItem>>,
    pub(crate) insns: Vec<LabeledInstr>,
    pub(crate) tries: Vec<TryItem>,
    pub(crate) handlers: Option<EncodedCatchHandlerList>,
}

impl DexIndex for Index<CodeItem> {
    type T = RwLock<CodeItem>;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.code_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("CodeItem".to_string()))
    }
}

impl DexCollection for CodeItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl CodeItem {
    #[inline]
    #[must_use]
    pub const fn registers_size(&self) -> usize {
        self.registers_size
    }

    #[inline]
    #[must_use]
    pub const fn ins_size(&self) -> usize {
        self.ins_size
    }

    #[inline]
    #[must_use]
    pub const fn outs_size(&self) -> usize {
        self.outs_size
    }

    pub fn debug_info<'a>(&'a self, dex: &'a Dex) -> DexResult<Option<&'a DebugInfoItem>> {
        self.debug_info_off.map(|off| off.get(dex)).transpose()
    }

    #[inline]
    #[must_use]
    pub fn instructions_count(&self) -> usize {
        self.insns.len()
    }

    #[inline]
    pub fn iter_instructions(&self) -> impl Iterator<Item = &LabeledInstr> {
        self.insns.iter()
    }

    #[inline]
    pub fn instruction_at(&self, addr: Addr) -> DexResult<&LabeledInstr> {
        let index = self
            .insns
            .binary_search_by(|probe| probe.addr().cmp(&addr))
            .map_err(|_| DexError::InstructionNotFound(addr))?;
        Ok(&self.insns[index])
    }

    pub fn patch_instruction_at(&mut self, addr: Addr, new_instrs: Vec<Instr>) -> DexResult<()> {
        let index = self
            .insns
            .binary_search_by(|probe| probe.addr().cmp(&addr))
            .map_err(|_| DexError::InstructionNotFound(addr))?;
        let old_instr = &self.insns[index];

        let mut addr = addr;
        let mut new_instrs: Vec<LabeledInstr> = new_instrs
            .into_iter()
            .map(|instr| {
                let linstr = LabeledInstr { addr, instr };
                addr = Addr(addr.0 + linstr.size());
                linstr
            })
            .collect();
        let new_size: usize = new_instrs.iter().map(Instruction::size).sum();
        if old_instr.size() < new_size {
            return Err(DexError::BadInstructionSize);
        }
        for _ in 0..(old_instr.size() - new_size) {
            new_instrs.push(LabeledInstr {
                addr,
                instr: Instr::Nop,
            });
            addr = Addr(addr.0 + 1);
        }

        self.insns.remove(index);
        for new_instr in new_instrs.into_iter().rev() {
            self.insns.insert(index, new_instr);
        }
        Ok(())
    }

    #[inline]
    pub fn iter_tries(&self) -> impl Iterator<Item = &TryItem> {
        self.tries.iter()
    }
}

#[derive(Debug)]
pub struct TryItem {
    pub(crate) start_addr: usize,
    pub(crate) insn_count: usize,
    // handler_off is not a des collection offset, it's an offset to
    // the BTreeMap from EncodedCatchHandlerList (this is why code
    // has to be passed to function handlers).
    pub(crate) handler_off: usize,
}

impl TryItem {
    #[inline]
    #[must_use]
    pub const fn start_addr(&self) -> Addr {
        Addr(self.start_addr)
    }

    #[inline]
    #[must_use]
    pub const fn insn_count(&self) -> usize {
        self.insn_count
    }

    #[inline]
    #[must_use]
    pub const fn end_addr(&self) -> Addr {
        Addr(self.start_addr + self.insn_count)
    }

    pub fn handlers<'a>(&'a self, code: &'a CodeItem) -> DexResult<&'a EncodedCatchHandler> {
        let handlers = match &code.handlers {
            Some(hs) => Ok(hs),
            None => Err(DexError::ResNotFound("EncodedCatchHandlerList".to_string())),
        }?;
        handlers
            .list
            .get(&self.handler_off)
            .ok_or_else(|| DexError::ResNotFound("EncodedCatchHandler".to_string()))
    }
}

#[derive(Debug)]
pub(crate) struct EncodedCatchHandlerList {
    pub(crate) size: Uleb128,
    pub(crate) list: BTreeMap<usize, EncodedCatchHandler>,
}

#[derive(Debug)]
pub struct EncodedCatchHandler {
    pub(crate) size: Sleb128,
    pub(crate) handlers: Vec<EncodedTypeAddrPair>,
    pub(crate) catch_all_addr: Option<Uleb128>,
}

impl EncodedCatchHandler {
    #[inline]
    pub fn iter_handlers(&self) -> impl Iterator<Item = &EncodedTypeAddrPair> {
        self.handlers.iter()
    }

    #[inline]
    #[must_use]
    pub fn catch_all_addr(&self) -> Option<usize> {
        self.catch_all_addr.map(|u| u.value() as usize)
    }
}

#[derive(Debug)]
pub struct EncodedTypeAddrPair {
    pub(crate) type_idx: Index<TypeIdItem>,
    pub(crate) addr: Uleb128,
}

impl EncodedTypeAddrPair {
    #[inline]
    pub fn catch_type(&self, dex: &Dex) -> DexResult<Type> {
        self.type_idx.get(dex)?.to_type(dex)
    }

    #[inline]
    #[must_use]
    pub fn catch_addr(&self) -> usize {
        self.addr.value() as usize
    }
}

#[derive(Debug)]
pub struct DebugInfoItem {
    pub(crate) index: Index<DebugInfoItem>,
    pub(crate) line_start: Uleb128,
    pub(crate) parameters_size: Uleb128,
    pub(crate) parameter_names: Vec<Option<Index<StringIdItem>>>,
    pub(crate) bytecode: Vec<DbgInstr>,
}

impl DexIndex for Index<DebugInfoItem> {
    type T = DebugInfoItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.debug_info_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("DebugInfoItem".to_string()))
    }
}

impl DexCollection for DebugInfoItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl DebugInfoItem {
    pub(crate) fn size(&self) -> usize {
        let parameter_names_size: usize = self
            .parameter_names
            .iter()
            .map(|opt_idx| opt_idx.map_or(1, |idx| idx.as_uleb().size()))
            .sum();
        let bytecode_size: usize = self.bytecode.iter().map(DbgInstr::size).sum();
        self.line_start.size() + self.parameters_size.size() + parameter_names_size + bytecode_size
    }
}

#[derive(Debug)]
pub(crate) enum DbgInstr {
    EndSequence,
    AdvancePc {
        addr_diff: Uleb128,
    },
    AdvanceLine {
        line_diff: Sleb128,
    },
    StartLocal {
        register_num: Uleb128,
        name_idx: Option<Index<StringIdItem>>,
        type_idx: Option<Index<TypeIdItem>>,
    },
    StartLocalExtended {
        register_num: Uleb128,
        name_idx: Option<Index<StringIdItem>>,
        type_idx: Option<Index<TypeIdItem>>,
        sig_idx: Option<Index<StringIdItem>>,
    },
    EndLocal {
        register_num: Uleb128,
    },
    RestartLocal {
        register_num: Uleb128,
    },
    SetPrologueEnd,
    SetEpilogueBegin,
    SetFile {
        name_idx: Option<Index<StringIdItem>>,
    },
    Special(u8),
}

impl DbgInstr {
    pub(crate) fn size(&self) -> usize {
        1 + match self {
            Self::EndSequence
            | Self::SetPrologueEnd
            | Self::SetEpilogueBegin
            | Self::Special(_) => 0,
            Self::AdvancePc { addr_diff } => addr_diff.size(),
            Self::AdvanceLine { line_diff } => line_diff.size(),
            Self::StartLocal {
                register_num,
                name_idx,
                type_idx,
            } => {
                register_num.size()
                    + name_idx.map_or(1, |idx| idx.as_uleb().size())
                    + type_idx.map_or(1, |idx| idx.as_uleb().size())
            }
            Self::StartLocalExtended {
                register_num,
                name_idx,
                type_idx,
                sig_idx,
            } => {
                register_num.size()
                    + name_idx.map_or(1, |idx| idx.as_uleb().size())
                    + type_idx.map_or(1, |idx| idx.as_uleb().size())
                    + sig_idx.map_or(1, |idx| idx.as_uleb().size())
            }
            Self::EndLocal { register_num } | Self::RestartLocal { register_num } => {
                register_num.size()
            }
            Self::SetFile { name_idx } => name_idx.map_or(1, |idx| idx.as_uleb().size()),
        }
    }
}
