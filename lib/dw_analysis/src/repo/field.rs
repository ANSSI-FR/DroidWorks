use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::FieldUid;
use dw_dex::fields::{EncodedField, FieldFlags, FieldIdItem};
use dw_dex::types::Type;
use dw_dex::Dex;
use std::fmt;

/// The enriched field definition.
#[derive(Debug, Clone)]
pub struct Field<'a> {
    // Unique identifier in the repository
    uid: FieldUid,
    // Dex field data
    content: &'a EncodedField,
    // Dex reference to resolve dex data
    dex: &'a Dex,
    // Cache of names and type that identify the field
    descriptor: FieldDescr,
}

impl<'a> Field<'a> {
    pub(crate) fn new(
        uid: FieldUid,
        encoded_field: &'a EncodedField,
        dex: &'a Dex,
    ) -> AnalysisResult<Self> {
        let descriptor = FieldDescr::try_from((dex, encoded_field.descriptor(dex)?))?;
        Ok(Self {
            uid,
            content: encoded_field,
            dex,
            descriptor,
        })
    }

    #[inline]
    pub fn uid(&self) -> FieldUid {
        self.uid
    }

    #[must_use]
    pub const fn dex(&self) -> &Dex {
        self.dex
    }

    #[inline]
    pub fn descriptor(&self) -> &FieldDescr {
        &self.descriptor
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.descriptor().name()
    }

    #[inline]
    pub fn type_(&self) -> &Type {
        self.descriptor().type_()
    }

    #[inline]
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_PUBLIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_private(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_PRIVATE)
    }

    #[inline]
    #[must_use]
    pub const fn is_protected(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_PROTECTED)
    }

    #[inline]
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_STATIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_FINAL)
    }

    #[inline]
    #[must_use]
    pub const fn is_volatile(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_VOLATILE)
    }

    #[inline]
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_TRANSIENT)
    }

    #[inline]
    #[must_use]
    pub const fn is_synthetic(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_SYNTHETIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_enum(&self) -> bool {
        self.content.flags().contains(FieldFlags::ACC_ENUM)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldDescr {
    class: String,
    name: String,
    type_: Type,
}

impl TryFrom<(&Dex, &FieldIdItem)> for FieldDescr {
    type Error = AnalysisError;

    fn try_from((dex, field): (&Dex, &FieldIdItem)) -> Result<Self, Self::Error> {
        Ok(Self {
            class: field.class_name(dex)?,
            name: field.name(dex)?,
            type_: field.type_(dex)?,
        })
    }
}

impl fmt::Display for FieldDescr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}->{}", self.type_, self.class, self.name)
    }
}

impl FieldDescr {
    #[inline]
    pub fn class_name(&self) -> &str {
        &self.class
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn type_(&self) -> &Type {
        &self.type_
    }
}
