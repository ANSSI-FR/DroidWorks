//! Dalvik class fields data structures.

use crate::errors::{DexError, DexResult};
use crate::strings::StringIdItem;
use crate::types::{Type, TypeIdItem};
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use bitflags::bitflags;
use dw_utils::leb::Uleb128;
use std::fmt;

#[derive(Debug)]
pub struct FieldIdItem {
    pub(crate) index: Index<FieldIdItem>,
    pub(crate) class_idx: Index<TypeIdItem>,
    pub(crate) type_idx: Index<TypeIdItem>,
    pub(crate) name_idx: Index<StringIdItem>,
}

impl DexIndex for Index<FieldIdItem> {
    type T = FieldIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.field_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("FieldIdItem".to_string()))
    }
}

impl DexCollection for FieldIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl FieldIdItem {
    /// Class type that defines the field.
    /// According to the Dalvik documentation, this must be a class type.
    pub fn class(&self, dex: &Dex) -> DexResult<Type> {
        self.class_idx.get(dex)?.to_type(dex)
    }

    pub fn class_name(&self, dex: &Dex) -> DexResult<String> {
        Ok(self.class(dex)?.as_class_name()?.to_string())
    }

    pub fn type_(&self, dex: &Dex) -> DexResult<Type> {
        self.type_idx.get(dex)?.to_type(dex)
    }

    pub fn name(&self, dex: &Dex) -> DexResult<String> {
        self.name_idx.get(dex)?.to_string(dex)
    }

    pub(crate) fn size(&self) -> usize {
        8
    }
}

impl PrettyPrint for FieldIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let class_name = self.class_name(dex)?;
        let name = self.name(dex)?;
        write!(f, "{class_name}.{name}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EncodedField {
    pub(crate) field_idx_diff: Uleb128,
    // The field_idx is not stored in dex as is, it's the cumulated sum
    // of diffs that is computed at parsing.
    pub(crate) field_idx: Index<FieldIdItem>,
    pub(crate) access_flags_repr: Uleb128,
    pub(crate) access_flags: FieldFlags,
}

impl EncodedField {
    pub fn descriptor<'a>(&'a self, dex: &'a Dex) -> DexResult<&'a FieldIdItem> {
        self.field_idx.get(dex)
    }

    #[inline]
    #[must_use]
    pub const fn flags(&self) -> FieldFlags {
        self.access_flags
    }
}

bitflags! {
    pub struct FieldFlags: u32 {
        const ACC_PUBLIC                = 0x00001;
        const ACC_PRIVATE               = 0x00002;
        const ACC_PROTECTED             = 0x00004;
        const ACC_STATIC                = 0x00008;
        const ACC_FINAL                 = 0x00010;
        const ACC_VOLATILE              = 0x00040;
        const ACC_TRANSIENT             = 0x00080;
        const ACC_SYNTHETIC             = 0x01000;
        const ACC_ENUM                  = 0x04000;
    }
}

impl fmt::Display for FieldFlags {
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
        if self.contains(Self::ACC_VOLATILE) {
            write!(f, "volatile ")?;
        }
        if self.contains(Self::ACC_TRANSIENT) {
            write!(f, "transient ")?;
        }
        if self.contains(Self::ACC_SYNTHETIC) {
            write!(f, "synthetic ")?;
        }
        if self.contains(Self::ACC_ENUM) {
            write!(f, "enum ")?;
        }
        Ok(())
    }
}
