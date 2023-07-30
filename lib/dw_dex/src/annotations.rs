//! Dalvik bytecode annotations data structures.

use crate::errors::{DexError, DexResult};
use crate::fields::FieldIdItem;
use crate::methods::MethodIdItem;
use crate::strings::StringIdItem;
use crate::types::TypeIdItem;
use crate::values::EncodedValue;
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use dw_utils::leb::Uleb128;
use std::fmt;

#[derive(Debug)]
pub(crate) struct AnnotationsDirectoryItem {
    pub(crate) index: Index<AnnotationsDirectoryItem>,
    pub(crate) class_annotations_off: Index<AnnotationSetItem>,
    pub(crate) field_annotations: Vec<FieldAnnotation>,
    pub(crate) method_annotations: Vec<MethodAnnotation>,
    pub(crate) parameter_annotations: Vec<ParameterAnnotation>,
}

impl DexIndex for Index<AnnotationsDirectoryItem> {
    type T = AnnotationsDirectoryItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.annotations_directory_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("AnnotationsDirectoryItem".to_string()))
    }
}

impl DexCollection for AnnotationsDirectoryItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl AnnotationsDirectoryItem {
    pub(crate) fn size(&self) -> usize {
        16 + self.field_annotations.len() * 8
            + self.method_annotations.len() * 8
            + self.parameter_annotations.len() * 8
    }
}

#[derive(Debug)]
pub(crate) struct FieldAnnotation {
    pub(crate) field_idx: Index<FieldIdItem>,
    pub(crate) annotations_off: Index<AnnotationSetItem>,
}

#[derive(Debug)]
pub(crate) struct MethodAnnotation {
    pub(crate) method_idx: Index<MethodIdItem>,
    pub(crate) annotations_off: Index<AnnotationSetItem>,
}

#[derive(Debug)]
pub(crate) struct ParameterAnnotation {
    pub(crate) method_idx: Index<MethodIdItem>,
    pub(crate) annotations_off: Index<AnnotationSetRefList>,
}

#[derive(Debug)]
pub(crate) struct AnnotationSetRefList {
    pub(crate) index: Index<AnnotationSetRefList>,
    pub(crate) list: Vec<AnnotationSetRefItem>,
}

impl DexIndex for Index<AnnotationSetRefList> {
    type T = AnnotationSetRefList;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.annotation_set_ref_lists
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("AnnotationSetRefList".to_string()))
    }
}

impl DexCollection for AnnotationSetRefList {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl AnnotationSetRefList {
    pub(crate) fn size(&self) -> usize {
        4 + 4 * self.list.len()
    }
}

#[derive(Debug)]
pub(crate) struct AnnotationSetRefItem {
    pub(crate) annotations_off: Index<AnnotationSetItem>,
}

#[derive(Debug)]
pub(crate) struct AnnotationSetItem {
    pub(crate) index: Index<AnnotationSetItem>,
    pub(crate) entries: Vec<AnnotationOffItem>,
}

impl DexIndex for Index<AnnotationSetItem> {
    type T = AnnotationSetItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.annotation_set_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("AnnotationSetItem".to_string()))
    }
}

impl DexCollection for AnnotationSetItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl AnnotationSetItem {
    pub(crate) fn size(&self) -> usize {
        4 + 4 * self.entries.len()
    }
}

#[derive(Debug)]
pub(crate) struct AnnotationOffItem {
    pub(crate) annotation_off: Index<AnnotationItem>,
}

#[derive(Debug)]
pub(crate) struct AnnotationItem {
    pub(crate) index: Index<AnnotationItem>,
    pub(crate) visibility: Visibility,
    pub(crate) annotation: EncodedAnnotation,
}

impl DexIndex for Index<AnnotationItem> {
    type T = AnnotationItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.annotation_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("AnnotationItem".to_string()))
    }
}

impl DexCollection for AnnotationItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl AnnotationItem {
    pub(crate) fn size(&self) -> usize {
        1 + self.annotation.size()
    }
}

#[derive(Debug)]
pub enum Visibility {
    Build,
    Runtime,
    System,
}

#[derive(Debug)]
pub(crate) struct EncodedAnnotation {
    pub(crate) type_idx: Index<TypeIdItem>,
    pub(crate) size: Uleb128,
    pub(crate) elements: Vec<AnnotationElement>,
}

impl PrettyPrint for EncodedAnnotation {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        write!(f, "@{}", self.type_idx.get(dex)?.to_type(dex)?)?;
        if !self.elements.is_empty() {
            write!(f, "(")?;
            for i in 0..self.elements.len() {
                self.elements[i].pp(f, dex)?;
                if i != self.elements.len() - 1 {
                    write!(f, ", ")?;
                }
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl EncodedAnnotation {
    pub(crate) fn size(&self) -> usize {
        let elements_size: usize = self.elements.iter().map(AnnotationElement::size).sum();
        self.type_idx.as_uleb().size() + self.size.size() + elements_size
    }
}

#[derive(Debug)]
pub(crate) struct AnnotationElement {
    pub(crate) name_idx: Index<StringIdItem>,
    pub(crate) value: EncodedValue,
}

impl PrettyPrint for AnnotationElement {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        write!(f, "{}=", self.name_idx.get(dex)?.to_string(dex)?)?;
        self.value.pp(f, dex)
    }
}

impl AnnotationElement {
    pub(crate) fn size(&self) -> usize {
        self.name_idx.as_uleb().size() + self.value.size()
    }
}
