//! Dalvik class methods data structures.

use crate::code::CodeItem;
use crate::errors::{DexError, DexResult};
use crate::strings::StringIdItem;
use crate::types::{ProtoIdItem, Type, TypeIdItem};
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use bitflags::bitflags;
use dw_utils::leb::Uleb128;
use std::fmt;
use std::sync::RwLock;

#[derive(Debug)]
pub struct MethodIdItem {
    pub(crate) index: Index<MethodIdItem>,
    pub(crate) class_idx: Index<TypeIdItem>,
    pub(crate) proto_idx: Index<ProtoIdItem>,
    pub(crate) name_idx: Index<StringIdItem>,
}

impl DexIndex for Index<MethodIdItem> {
    type T = MethodIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.method_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("MethodIdItem".to_string()))
    }
}

impl DexCollection for MethodIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl MethodIdItem {
    /// Type that defined the method.
    /// According to the Dalvik documentation, this must be a class type or an array type.
    pub fn definer(&self, dex: &Dex) -> DexResult<Type> {
        self.class_idx.get(dex)?.to_type(dex)
    }

    pub fn return_type(&self, dex: &Dex) -> DexResult<Type> {
        self.proto_idx.get(dex)?.return_type(dex)
    }

    pub fn parameters_types(&self, dex: &Dex) -> DexResult<Vec<Type>> {
        self.proto_idx.get(dex)?.parameters_types(dex)
    }

    pub fn name(&self, dex: &Dex) -> DexResult<String> {
        self.name_idx.get(dex)?.to_string(dex)
    }

    pub(crate) fn size(&self) -> usize {
        8
    }
}

impl PrettyPrint for MethodIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let definer = self.definer(dex)?;
        let name = self.name(dex)?;
        let proto = self.proto_idx.get(dex)?;
        write!(f, "{definer}->{name}")?;
        proto.pp(f, dex)
    }
}

#[derive(Debug, Clone)]
pub struct EncodedMethod {
    pub(crate) method_idx_diff: Uleb128,
    // The method_idx is not stored in dex as is, it's the cumulated sum
    // of diffs that is computed at parsing.
    pub(crate) method_idx: Index<MethodIdItem>,
    pub(crate) access_flags_repr: Uleb128,
    pub(crate) access_flags: MethodFlags,
    pub(crate) code_off: Option<Index<CodeItem>>,
}

impl EncodedMethod {
    pub fn descriptor<'a>(&'a self, dex: &'a Dex) -> DexResult<&'a MethodIdItem> {
        self.method_idx.get(dex)
    }

    #[inline]
    #[must_use]
    pub const fn flags(&self) -> MethodFlags {
        self.access_flags
    }

    pub fn code<'a>(&self, dex: &'a Dex) -> DexResult<Option<&'a RwLock<CodeItem>>> {
        self.code_off.map(|off| off.get(dex)).transpose()
    }
}

bitflags! {
    pub struct MethodFlags: u32 {
        const ACC_PUBLIC                = 0x00001;
        const ACC_PRIVATE               = 0x00002;
        const ACC_PROTECTED             = 0x00004;
        const ACC_STATIC                = 0x00008;
        const ACC_FINAL                 = 0x00010;
        const ACC_SYNCHRONIZED          = 0x00020;
        const ACC_BRIDGE                = 0x00040;
        const ACC_VARARGS               = 0x00080;
        const ACC_NATIVE                = 0x00100;
        const ACC_ABSTRACT              = 0x00400;
        const ACC_STRICT                = 0x00800;
        const ACC_SYNTHETIC             = 0x01000;
        const ACC_CONSTRUCTOR           = 0x10000;
        const ACC_DECLARED_SYNCHRONIZED = 0x20000;
    }
}

impl fmt::Display for MethodFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.contains(Self::ACC_PUBLIC) {
            write!(f, "public ")?;
        }
        if self.contains(Self::ACC_PRIVATE) {
            write!(f, "private ")?;
        }
        if self.contains(Self::ACC_PROTECTED) {
            write!(f, "protected ")?;
        }
        if self.contains(Self::ACC_STATIC) {
            write!(f, "static ")?;
        }
        if self.contains(Self::ACC_FINAL) {
            write!(f, "final ")?;
        }
        if self.contains(Self::ACC_SYNCHRONIZED) {
            write!(f, "synchronized ")?;
        }
        if self.contains(Self::ACC_BRIDGE) {
            write!(f, "bridge ")?;
        }
        if self.contains(Self::ACC_VARARGS) {
            write!(f, "varargs ")?;
        }
        if self.contains(Self::ACC_NATIVE) {
            write!(f, "native ")?;
        }
        if self.contains(Self::ACC_ABSTRACT) {
            write!(f, "abstract ")?;
        }
        if self.contains(Self::ACC_STRICT) {
            write!(f, "strict ")?;
        }
        if self.contains(Self::ACC_SYNTHETIC) {
            write!(f, "synthetic ")?;
        }
        if self.contains(Self::ACC_CONSTRUCTOR) {
            write!(f, "constructor ")?;
        }
        if self.contains(Self::ACC_DECLARED_SYNCHRONIZED) {
            write!(f, "declared_synchronized ")?;
        }
        Ok(())
    }
}
