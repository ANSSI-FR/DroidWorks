//! Dalvik classes data structures.

use crate::annotations::AnnotationsDirectoryItem;
use crate::errors::{DexError, DexResult};
use crate::fields::EncodedField;
use crate::methods::EncodedMethod;
use crate::strings::StringIdItem;
use crate::types::{TypeIdItem, TypeList};
use crate::values::EncodedArrayItem;
use crate::{Dex, DexCollection, DexIndex, Index, Map};
use bitflags::bitflags;
use dw_utils::leb::Uleb128;
use std::fmt;

/// The Dalvik class definition.
#[derive(Debug)]
pub struct ClassDefItem {
    pub(crate) index: Index<ClassDefItem>,
    pub(crate) class_idx: Index<TypeIdItem>,
    pub(crate) access_flags: ClassFlags,
    pub(crate) superclass_idx: Option<Index<TypeIdItem>>,
    pub(crate) interfaces_off: Option<Index<TypeList>>,
    pub(crate) source_file_idx: Option<Index<StringIdItem>>,
    pub(crate) annotations_off: Option<Index<AnnotationsDirectoryItem>>,
    pub(crate) class_data_off: Option<Index<ClassDataItem>>,
    pub(crate) static_values_off: Option<Index<EncodedArrayItem>>,
}

impl DexIndex for Index<ClassDefItem> {
    type T = ClassDefItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.class_def_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("ClassDefItem".to_string()))
    }
}

impl DexCollection for ClassDefItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl ClassDefItem {
    /// Returns the name of the class.
    pub fn class_name(&self, dex: &Dex) -> DexResult<String> {
        Ok(self
            .class_idx
            .get(dex)?
            .to_type(dex)?
            .as_class_name()?
            .to_string())
    }

    /// Returns the flags of the class.
    #[inline]
    #[must_use]
    pub const fn flags(&self) -> ClassFlags {
        self.access_flags
    }

    /// Returns the name of the superclass if it exists, returns [`None`] otherwise.
    pub fn superclass(&self, dex: &Dex) -> DexResult<Option<String>> {
        self.superclass_idx
            .map(|idx| Ok(idx.get(dex)?.to_type(dex)?.as_class_name()?.to_string()))
            .transpose()
    }

    /// Returns a vec of the interface names that are implemented by the class.
    pub fn interfaces(&self, dex: &Dex) -> DexResult<Vec<String>> {
        if let Some(off) = self.interfaces_off {
            off.get(dex)?
                .to_types(dex)?
                .into_iter()
                .map(|t| Ok(t.as_class_name()?.to_string()))
                .collect()
        } else {
            Ok(Vec::new())
        }
    }

    /// Optionnaly returns the source file name of the class definition.
    pub fn source_file(&self, dex: &Dex) -> DexResult<Option<String>> {
        self.source_file_idx
            .map(|idx| idx.get(dex)?.to_string(dex))
            .transpose()
    }

    /// The class can be a simple declaration, in which case this methods
    /// returns [`None`]. If it contains data (methods, fields, etc.),
    /// returns it.
    pub fn data<'a>(&'a self, dex: &'a Dex) -> DexResult<Option<&'a ClassDataItem>> {
        self.class_data_off.map(|off| off.get(dex)).transpose()
    }

    pub(crate) fn size(&self) -> usize {
        32
    }
}

bitflags! {
    /// Dalvik class flags
    pub struct ClassFlags: u32 {
        const ACC_PUBLIC                = 0x00001;
        const ACC_PRIVATE               = 0x00002;
        const ACC_PROTECTED             = 0x00004;
        const ACC_STATIC                = 0x00008;
        const ACC_FINAL                 = 0x00010;
        const ACC_INTERFACE             = 0x00200;
        const ACC_ABSTRACT              = 0x00400;
        const ACC_SYNTHETIC             = 0x01000;
        const ACC_ANNOTATION            = 0x02000;
        const ACC_ENUM                  = 0x04000;
    }
}

impl fmt::Display for ClassFlags {
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
        if self.contains(Self::ACC_INTERFACE) {
            write!(f, "interface ")?;
        }
        if self.contains(Self::ACC_ABSTRACT) {
            write!(f, "abstract ")?;
        }
        if self.contains(Self::ACC_SYNTHETIC) {
            write!(f, "synthetic ")?;
        }
        if self.contains(Self::ACC_ANNOTATION) {
            write!(f, "annotation ")?;
        }
        if self.contains(Self::ACC_ENUM) {
            write!(f, "enum ")?;
        }
        Ok(())
    }
}

/// Dalvik class fields and methods data definition.
#[derive(Debug)]
pub struct ClassDataItem {
    pub(crate) index: Index<ClassDataItem>,
    pub(crate) static_fields_size: Uleb128,
    pub(crate) instance_fields_size: Uleb128,
    pub(crate) direct_methods_size: Uleb128,
    pub(crate) virtual_methods_size: Uleb128,
    pub(crate) static_fields: Vec<EncodedField>,
    pub(crate) instance_fields: Vec<EncodedField>,
    pub(crate) direct_methods: Vec<EncodedMethod>,
    pub(crate) virtual_methods: Vec<EncodedMethod>,
}

impl DexIndex for Index<ClassDataItem> {
    type T = ClassDataItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.class_data_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("ClassDataItem".to_string()))
    }
}

impl DexCollection for ClassDataItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl ClassDataItem {
    /// Returns an iterator over static fields of the class.
    #[inline]
    pub fn iter_static_fields(&self) -> impl Iterator<Item = &EncodedField> {
        self.static_fields.iter()
    }

    /// Returns an iterator over instance fields of the class.
    #[inline]
    pub fn iter_instance_fields(&self) -> impl Iterator<Item = &EncodedField> {
        self.instance_fields.iter()
    }

    /// Returns an iterator over all fields declared in the class.
    #[inline]
    pub fn iter_fields(&self) -> impl Iterator<Item = &EncodedField> {
        self.iter_static_fields().chain(self.iter_instance_fields())
    }

    /// Returns an iterator over direct methods of the class.
    #[inline]
    pub fn iter_direct_methods(&self) -> impl Iterator<Item = &EncodedMethod> {
        self.direct_methods.iter()
    }

    /// Returns an iterator over virtual methods of the class.
    #[inline]
    pub fn iter_virtual_methods(&self) -> impl Iterator<Item = &EncodedMethod> {
        self.virtual_methods.iter()
    }

    /// Returns an iterator over all methods declared in the class.
    #[inline]
    pub fn iter_methods(&self) -> impl Iterator<Item = &EncodedMethod> {
        self.iter_direct_methods()
            .chain(self.iter_virtual_methods())
    }

    pub(crate) fn size(&self) -> usize {
        let fields_size: usize = self
            .static_fields
            .iter()
            .chain(self.instance_fields.iter())
            .map(|field| field.field_idx_diff.size() + field.access_flags_repr.size())
            .sum();
        let methods_size: usize = self
            .direct_methods
            .iter()
            .chain(self.virtual_methods.iter())
            .map(|method| {
                method.method_idx_diff.size()
                    + method.access_flags_repr.size()
                    + (if let Some(off) = method.code_off {
                        off.as_uleb().size()
                    } else {
                        1 // need 1 byte to store value 0 as an uleb128
                    })
            })
            .sum();

        self.static_fields_size.size()
            + self.instance_fields_size.size()
            + self.direct_methods_size.size()
            + self.virtual_methods_size.size()
            + fields_size
            + methods_size
    }
}

#[derive(Debug)]
pub(crate) struct HiddenapiClassDataItem {
    pub(crate) offsets: Vec<Index<HiddenapiClassFlag>>,
    pub(crate) flags: Map<HiddenapiClassFlag>,
}

impl HiddenapiClassDataItem {
    pub(crate) fn size(&self) -> usize {
        let flags_len: usize = self.flags.values().map(|flag| flag.uleb_repr.size()).sum();
        self.offsets.len() * 4 + flags_len
    }
}

#[derive(Debug)]
pub struct HiddenapiClassFlag {
    pub(crate) uleb_repr: Uleb128,
    pub(crate) flag: HiddenapiFlag,
}

impl DexIndex for Index<HiddenapiClassFlag> {
    type T = HiddenapiClassFlag;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.hiddenapi_class_data_items[&0]
            .flags
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("HiddenapiClassFlag".to_string()))
    }
}

impl HiddenapiClassFlag {
    pub fn flag(&self) -> HiddenapiFlag {
        self.flag
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HiddenapiFlag {
    Whitelist,
    Greylist,
    Blacklist,
    GreylistMaxO,
    GreylistMaxP,
    GreylistMaxQ,
    GreylistMaxR,
}

impl TryFrom<u32> for HiddenapiFlag {
    type Error = DexError;

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(Self::Whitelist),
            0x0001 => Ok(Self::Greylist),
            0x0002 => Ok(Self::Blacklist),
            0x0003 => Ok(Self::GreylistMaxO),
            0x0004 => Ok(Self::GreylistMaxP),
            0x0005 => Ok(Self::GreylistMaxQ),
            0x0006 => Ok(Self::GreylistMaxR),
            _ => Err(DexError::Structure(format!(
                "unknown hiddenapi flag: '{v}'"
            ))),
        }
    }
}
