use crate::annotations::EncodedAnnotation;
use crate::code::MethodHandleItem;
use crate::errors::{DexError, DexResult};
use crate::fields::FieldIdItem;
use crate::methods::MethodIdItem;
use crate::strings::StringIdItem;
use crate::types::{ProtoIdItem, TypeIdItem};
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use dw_utils::leb::Uleb128;
use std::fmt;

#[derive(Debug)]
pub(crate) enum EncodedValue {
    Byte(i8),
    Short(usize, i16),
    Char(usize, u16),
    Int(usize, i32),
    Long(usize, i64),
    Float(usize, f32),
    Double(usize, f64),
    MethodType(usize, Index<ProtoIdItem>),
    MethodHandle(usize, Index<MethodHandleItem>),
    String(usize, Index<StringIdItem>),
    Type(usize, Index<TypeIdItem>),
    Field(usize, Index<FieldIdItem>),
    Method(usize, Index<MethodIdItem>),
    Enum(usize, Index<FieldIdItem>),
    Array(EncodedArray),
    Annotation(EncodedAnnotation),
    Null,
    Boolean(bool),
}

impl PrettyPrint for EncodedValue {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        match self {
            Self::Byte(v) => write!(f, "{v}")?,
            Self::Short(_, v) => write!(f, "{v}")?,
            Self::Char(_, v) => write!(f, "{v}")?,
            Self::Int(_, v) => write!(f, "{v}")?,
            Self::Long(_, v) => write!(f, "{v}")?,
            Self::Float(_, v) => write!(f, "{v}")?,
            Self::Double(_, v) => write!(f, "{v}")?,
            Self::MethodType(_, proto) => proto.get(dex)?.pp(f, dex)?,
            Self::MethodHandle(_, handle) => handle.get(dex)?.pp(f, dex)?,
            Self::String(_, s) => {
                write!(f, "\"")?;
                s.get(dex)?.pp(f, dex)?;
                write!(f, "\"")?;
            }
            Self::Type(_, typ) => typ.get(dex)?.pp(f, dex)?,
            Self::Field(_, field) => field.get(dex)?.pp(f, dex)?,
            Self::Method(_, method) => method.get(dex)?.pp(f, dex)?,
            Self::Enum(_, field) => field.get(dex)?.pp(f, dex)?,
            Self::Array(arr) => arr.pp(f, dex)?,
            Self::Annotation(ann) => ann.pp(f, dex)?,
            Self::Null => write!(f, "null")?,
            Self::Boolean(b) => write!(f, "{b}")?,
        };
        Ok(())
    }
}

impl EncodedValue {
    pub(crate) fn size(&self) -> usize {
        1 + // value_tag
        match self {
            Self::Byte(_) => 0,
            Self::Short(siz, _)
            | Self::Char(siz, _)
            | Self::Int(siz, _)
            | Self::Long(siz, _)
            | Self::Float(siz, _)
            | Self::Double(siz, _)
            | Self::MethodType(siz, _)
            | Self::MethodHandle(siz, _)
            | Self::String(siz, _)
            | Self::Type(siz, _)
            | Self::Field(siz, _)
            | Self::Method(siz, _)
            | Self::Enum(siz, _) => *siz,
            Self::Array(arr) => arr.size(),
            Self::Annotation(ann) => ann.size(),
            Self::Null
            | Self::Boolean(_) => 0,
        }
    }
}

#[derive(Debug)]
pub(crate) struct EncodedArrayItem {
    pub(crate) index: Index<EncodedArrayItem>,
    pub(crate) value: EncodedArray,
}

impl DexIndex for Index<EncodedArrayItem> {
    type T = EncodedArrayItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.encoded_array_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("EncodedArrayItem".to_string()))
    }
}

impl DexCollection for EncodedArrayItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl EncodedArrayItem {
    pub(crate) fn size(&self) -> usize {
        self.value.size()
    }
}

#[derive(Debug)]
pub(crate) struct EncodedArray {
    pub(crate) size: Uleb128,
    pub(crate) values: Vec<EncodedValue>,
}

impl PrettyPrint for EncodedArray {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        write!(f, "[")?;
        for (i, value) in self.values.iter().enumerate() {
            value.pp(f, dex)?;
            if i == self.values.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl EncodedArray {
    pub(crate) fn size(&self) -> usize {
        let values_size: usize = self.values.iter().map(EncodedValue::size).sum();
        self.size.size() + values_size
    }
}
