//! Dalvik typing informations data structures.

use crate::errors::{DexError, DexResult};
use crate::strings::StringIdItem;
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

/// The Dalvik type descriptor to be used for referencing it from other Dex data items.
#[derive(Debug)]
pub struct TypeIdItem {
    pub(crate) index: Index<TypeIdItem>,
    pub(crate) descriptor_idx: Index<StringIdItem>,
}

impl DexIndex for Index<TypeIdItem> {
    type T = TypeIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.type_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("TypeIdItem".to_string()))
    }
}

impl DexCollection for TypeIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl TypeIdItem {
    /// Returns the concrete Dalvik [`Type`] designated by the descriptor.
    pub fn to_type(&self, dex: &Dex) -> DexResult<Type> {
        let descriptor_id = self.descriptor_idx.get(dex)?;
        let descriptor_string = descriptor_id.to_string(dex)?;
        Type::try_from(descriptor_string.as_ref())
    }

    pub(crate) fn size(&self) -> usize {
        4
    }
}

impl PrettyPrint for TypeIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let typ = self.to_type(dex)?;
        write!(f, "{typ}")?;
        Ok(())
    }
}

/// The Dalvik prototype descriptor to be used for referencing it from other Dex data items.
///
/// A Dalvik method prototype consists of parameters and return types, and also a short form
/// of those informations using [`Shorty`] type descriptor.
#[derive(Debug)]
pub struct ProtoIdItem {
    pub(crate) index: Index<ProtoIdItem>,
    pub(crate) shorty_idx: Index<StringIdItem>,
    pub(crate) return_type_idx: Index<TypeIdItem>,
    pub(crate) parameters_off: Option<Index<TypeList>>,
}

impl DexIndex for Index<ProtoIdItem> {
    type T = ProtoIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.proto_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("ProtoIdItem".to_string()))
    }
}

impl DexCollection for ProtoIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl ProtoIdItem {
    /// Returns the [`Shorty`] type description of the prototype.
    pub fn to_shorty(&self, dex: &Dex) -> DexResult<Shorty> {
        let shorty_id = self.shorty_idx.get(dex)?;
        let shorty_string = shorty_id.to_string(dex)?;
        Shorty::try_from(shorty_string.as_ref())
    }

    /// Returns the return type of the prototype.
    pub fn return_type(&self, dex: &Dex) -> DexResult<Type> {
        self.return_type_idx.get(dex)?.to_type(dex)
    }

    /// Returns the parameters types of the prototype.
    pub fn parameters_types(&self, dex: &Dex) -> DexResult<Vec<Type>> {
        if let Some(off) = self.parameters_off {
            off.get(dex)?.to_types(dex)
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) fn size(&self) -> usize {
        12
    }
}

impl PrettyPrint for ProtoIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let return_ = self.return_type(dex)?;
        write!(f, "(")?;
        for t in &self.parameters_types(dex)? {
            write!(f, "{t}")?;
        }
        write!(f, "){return_}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TypeList {
    pub(crate) index: Index<TypeList>,
    pub(crate) list: Vec<TypeItem>,
}

impl DexIndex for Index<TypeList> {
    type T = TypeList;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.type_lists
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("TypeList".to_string()))
    }
}

impl DexCollection for TypeList {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl TypeList {
    pub(crate) fn to_types(&self, dex: &Dex) -> DexResult<Vec<Type>> {
        self.list
            .iter()
            .map(|t| TypeItem::to_type(t, dex))
            .collect()
    }
}

#[derive(Debug)]
pub(crate) struct TypeItem {
    pub(crate) type_idx: Index<TypeIdItem>,
}

impl TypeItem {
    fn to_type(&self, dex: &Dex) -> DexResult<Type> {
        let type_id = self.type_idx.get(dex)?;
        type_id.to_type(dex)
    }
}

/// Dalvik concrete type descriptor type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Type {
    /// `void` type, only valid for return types.
    Void,
    /// `boolean` type.
    Boolean,
    /// `byte` type.
    Byte,
    /// `short` type.
    Short,
    /// `char` type.
    Char,
    /// `int` type.
    Int,
    /// `long` type.
    Long,
    /// `float` type.
    Float,
    /// `double` type.
    Double,
    /// Array of the given type descriptor, usable recursively for arrays of arrays,
    /// though it is invalid to have more than 255 dimensions.
    Array(usize, Box<Self>),
    /// Type of a fully-qualified class
    Class(String),
}

impl Type {
    /// Returns a java-like representation of the type.
    /// This method is useful for pretty-printing bytecode data. Its result differs
    /// from the `Display` implementation, which produces strings in the Dalvik
    /// format.
    #[must_use]
    pub fn to_java_string(&self) -> String {
        match self {
            Self::Void => "void".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::Byte => "byte".to_string(),
            Self::Short => "short".to_string(),
            Self::Char => "char".to_string(),
            Self::Int => "int".to_string(),
            Self::Long => "long".to_string(),
            Self::Float => "float".to_string(),
            Self::Double => "double".to_string(),
            Self::Array(n, sub) => {
                let mut s = sub.to_java_string();
                for _ in 0..*n {
                    s.push_str("[]");
                }
                s
            }
            Self::Class(name) => name.replace('/', "."),
        }
    }

    pub fn as_class_name(&self) -> DexResult<&str> {
        if let Self::Class(name) = self {
            Ok(name)
        } else {
            Err(DexError::InvalidType)
        }
    }

    pub fn to_definer_name(&self) -> DexResult<String> {
        match self {
            Self::Class(name) => Ok(name.to_string()),
            Self::Array(_, _) => Ok(format!("{self}")),
            _ => Err(DexError::InvalidType),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Void => write!(f, "V"),
            Self::Boolean => write!(f, "Z"),
            Self::Byte => write!(f, "B"),
            Self::Short => write!(f, "S"),
            Self::Char => write!(f, "C"),
            Self::Int => write!(f, "I"),
            Self::Long => write!(f, "J"),
            Self::Float => write!(f, "F"),
            Self::Double => write!(f, "D"),
            Self::Array(n, inner) => {
                for _ in 0..*n {
                    write!(f, "[")?;
                }
                write!(f, "{inner}")
            }
            Self::Class(classname) => write!(f, "L{classname};"),
        }
    }
}

impl TryFrom<&str> for Type {
    type Error = DexError;

    fn try_from(s: &str) -> DexResult<Self> {
        if s.is_empty() {
            return Err(DexError::Conversion {
                from: format!("&str ({s:?})"),
                to: "Type".to_string(),
            });
        }

        if s == "V" {
            return Ok(Self::Void);
        }

        let mut i: usize = 0;
        while i < s.len() && &s[i..=i] == "[" {
            i += 1;
        }
        if i >= s.len() || i >= 255 {
            return Err(DexError::Conversion {
                from: format!("&str ({s:?})"),
                to: "Type".to_string(),
            });
        }

        let t = match &s[i..] {
            "Z" => Ok(Self::Boolean),
            "B" => Ok(Self::Byte),
            "S" => Ok(Self::Short),
            "C" => Ok(Self::Char),
            "I" => Ok(Self::Int),
            "J" => Ok(Self::Long),
            "F" => Ok(Self::Float),
            "D" => Ok(Self::Double),
            sub => {
                let l = sub.len();
                if l < 2 {
                    return Err(DexError::Conversion {
                        from: format!("&str ({s:?})"),
                        to: "Type".to_string(),
                    });
                }
                if &sub[0..1] == "L" && &sub[l - 1..l] == ";" {
                    Ok(Self::Class(sub[1..l - 1].to_string()))
                } else {
                    Err(DexError::Conversion {
                        from: format!("&str: ({s:?})"),
                        to: "Type".to_string(),
                    })
                }
            }
        }?;
        if i == 0 {
            Ok(t)
        } else {
            Ok(Self::Array(i, Box::new(t)))
        }
    }
}

/// Short form representation of a Dalvik [`Type`].
///
/// Same definition as [`Type`] except that there is no distinction between various
/// references (class or array) types.
#[derive(Debug)]
pub enum Shorty {
    /// `void` shorty type, only valid for return types
    Void,
    /// `boolean` shorty type
    Boolean,
    /// `byte` shorty type
    Byte,
    /// `short` shorty type
    Short,
    /// `char` shorty type
    Char,
    /// `int` shorty type
    Int,
    /// `long` shorty type
    Long,
    /// `float` shorty type
    Float,
    /// `double` shorty type
    Double,
    /// Reference shorty type, can designate any class of array.
    Reference,
}

impl TryFrom<&str> for Shorty {
    type Error = DexError;

    fn try_from(s: &str) -> DexResult<Self> {
        match s {
            "V" => Ok(Self::Void),
            "Z" => Ok(Self::Boolean),
            "B" => Ok(Self::Byte),
            "S" => Ok(Self::Short),
            "C" => Ok(Self::Char),
            "I" => Ok(Self::Int),
            "J" => Ok(Self::Long),
            "F" => Ok(Self::Float),
            "D" => Ok(Self::Double),
            _ => Err(DexError::Conversion {
                from: format!("&str: ({s:?})"),
                to: "Shorty".to_string(),
            }),
        }
    }
}
