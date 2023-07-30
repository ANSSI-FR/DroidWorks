use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::Repo;
use crate::typing::errors::{TypeError, TypeResult};
use dw_dex::fields::FieldIdItem;
use dw_dex::types::{Type, TypeIdItem};
use dw_dex::{DexIndex, Index, WithDex};
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractType {
    Top,
    Join64,
    Long,
    Double,
    Meet64,
    JoinZero,
    Join32,
    Integer,
    Float,
    Meet32,
    Object(BTreeSet<String>),
    Array(usize, Box<Self>),
    Null,
    MeetZero,
    Bottom,
}

impl fmt::Display for AbstractType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Top => write!(f, "⊤"),
            Self::Join64 => write!(f, "Join64"),
            Self::Long => write!(f, "long"),
            Self::Double => write!(f, "double"),
            Self::Meet64 => write!(f, "Meet64"),
            Self::JoinZero => write!(f, " JoinZero"),
            Self::Join32 => write!(f, "Join32"),
            Self::Integer => write!(f, "integer"),
            Self::Float => write!(f, "float"),
            Self::Meet32 => write!(f, "Meet32"),
            Self::Object(s) => {
                write!(f, "OBJ[")?;
                for (i, c) in s.iter().enumerate() {
                    if i < s.len() - 1 {
                        write!(f, "{c}, ")?;
                    } else {
                        write!(f, "{c}")?;
                    }
                }
                write!(f, "]")
            }
            Self::Array(n, t) => write!(f, "ARR[{n} * {t}]"),
            Self::Null => write!(f, "null"),
            Self::MeetZero => write!(f, "MeetZero"),
            Self::Bottom => write!(f, "⊥"),
        }
    }
}

impl TryFrom<&Type> for AbstractType {
    type Error = TypeError;

    fn try_from(descr: &Type) -> TypeResult<Self> {
        match descr {
            Type::Void => Err(TypeError::InvalidFieldType),
            Type::Boolean | Type::Byte | Type::Short | Type::Char | Type::Int => Ok(Self::Integer),
            Type::Long => Ok(Self::Long),
            Type::Float => Ok(Self::Float),
            Type::Double => Ok(Self::Double),
            Type::Array(n, t) => Ok(Self::Array(*n, Box::new(Self::try_from(t.as_ref())?))),
            Type::Class(s) => Ok(Self::object_singleton(s.clone())),
        }
    }
}

impl TryFrom<WithDex<'_, Index<TypeIdItem>>> for AbstractType {
    type Error = TypeError;

    fn try_from(dexed: WithDex<Index<TypeIdItem>>) -> TypeResult<Self> {
        let typ = dexed.data.get(dexed.dex)?.to_type(dexed.dex)?;
        Self::try_from(&typ)
    }
}

impl TryFrom<WithDex<'_, Index<FieldIdItem>>> for AbstractType {
    type Error = TypeError;

    fn try_from(dexed: WithDex<Index<FieldIdItem>>) -> TypeResult<Self> {
        let field_descr = dexed.data.get(dexed.dex)?.type_(dexed.dex)?;
        Self::try_from(&field_descr)
    }
}

lazy_static! {
    pub static ref JAVA_LANG_OBJECT: AbstractType =
        AbstractType::object_singleton("java/lang/Object".to_string());
    pub static ref JAVA_LANG_THROWABLE: AbstractType =
        AbstractType::object_singleton("java/lang/Throwable".to_string());
    pub static ref JAVA_LANG_STRING: AbstractType =
        AbstractType::object_singleton("java/lang/String".to_string());
    pub static ref JAVA_LANG_CLASS: AbstractType =
        AbstractType::object_singleton("java/lang/Class".to_string());
    pub static ref JAVA_IO_SERIALIZABLE: AbstractType =
        AbstractType::object_singleton("java/io/Serializable".to_string());
}

impl AbstractType {
    pub(crate) fn object_singleton(class: String) -> Self {
        let mut set = BTreeSet::new();
        set.insert(class);
        Self::Object(set)
    }

    pub(crate) fn is_subseteq(&self, other: &Self, repo: &Repo) -> AnalysisResult<()> {
        if self.subseteq(other, repo)? {
            Ok(())
        } else {
            Err(AnalysisError::Type(TypeError::NotASubtype(
                self.clone(),
                other.clone(),
            )))
        }
    }

    pub(crate) fn subseteq(&self, other: &Self, repo: &Repo) -> AnalysisResult<bool> {
        match (self, other) {
            (_, Self::Top)
            | (Self::Bottom, _)
            | (Self::Join64, Self::Join64)
            | (Self::Double, Self::Join64)
            | (Self::Long, Self::Join64)
            | (Self::Meet64, Self::Join64)
            | (Self::Join32, Self::Join32)
            | (Self::Float, Self::Join32)
            | (Self::Integer, Self::Join32)
            | (Self::Meet32, Self::Join32)
            | (Self::MeetZero, Self::Join32)
            | (Self::JoinZero, Self::JoinZero)
            | (Self::Integer, Self::JoinZero)
            | (Self::Object(_), Self::JoinZero)
            | (Self::Meet32, Self::JoinZero)
            | (Self::Array(_, _), Self::JoinZero)
            | (Self::MeetZero, Self::JoinZero)
            | (Self::Null, Self::JoinZero)
            | (Self::Double, Self::Double)
            | (Self::Meet64, Self::Double)
            | (Self::Long, Self::Long)
            | (Self::Meet64, Self::Long)
            | (Self::Float, Self::Float)
            | (Self::Meet32, Self::Float)
            | (Self::MeetZero, Self::Float)
            | (Self::Integer, Self::Integer)
            | (Self::Meet32, Self::Integer)
            | (Self::MeetZero, Self::Integer)
            | (Self::Meet64, Self::Meet64)
            | (Self::Meet32, Self::Meet32)
            | (Self::MeetZero, Self::Meet32)
            | (Self::MeetZero, Self::MeetZero) => Ok(true),

            (Self::Object(os1), Self::Object(os2)) => {
                assert!(!os1.is_empty());
                assert!(!os2.is_empty());
                for o2 in os2 {
                    let mut exists = false;
                    for o1 in os1 {
                        if repo.is_typeable_as(o1, o2)? {
                            exists = true;
                            break;
                        }
                    }
                    if !exists {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (Self::Array(_, _), t @ Self::Object(_)) => Ok(t == &*JAVA_LANG_OBJECT
                || t == &Self::JoinZero
                || t == &Self::Top
                || t == &*JAVA_IO_SERIALIZABLE),
            (Self::Null, Self::Object(_)) | (Self::MeetZero, Self::Object(_)) => Ok(true),

            (Self::Array(m, s), Self::Array(n, t)) => {
                if m == n {
                    s.subseteq(t, repo)
                } else {
                    Ok((n < m)
                        && (t.as_ref() == &*JAVA_LANG_OBJECT
                            || t.as_ref() == &Self::JoinZero
                            || t.as_ref() == &Self::Top))
                }
            }
            (Self::Null, Self::Array(_, _)) | (Self::MeetZero, Self::Array(_, _)) => Ok(true),

            (Self::Null, Self::Null) | (Self::MeetZero, Self::Null) => Ok(true),

            _ => Ok(false),
        }
    }

    pub(crate) fn join(self, other: Self, repo: &Repo) -> AnalysisResult<Self> {
        if self.subseteq(&other, repo)? {
            return Ok(other);
        }
        if other.subseteq(&self, repo)? {
            return Ok(self);
        }
        match (self, other) {
            (Self::Double, Self::Long) => Ok(Self::Join64),

            (Self::Float, Self::Integer) | (Self::Integer, Self::Float) => Ok(Self::Join32),

            (Self::Integer, Self::Object(_))
            | (Self::Object(_), Self::Integer)
            | (Self::Meet32, Self::Object(_))
            | (Self::Object(_), Self::Meet32)
            | (Self::Integer, Self::Array(_, _))
            | (Self::Array(_, _), Self::Integer)
            | (Self::Meet32, Self::Array(_, _))
            | (Self::Array(_, _), Self::Meet32)
            | (Self::Integer, Self::Null)
            | (Self::Null, Self::Integer)
            | (Self::Meet32, Self::Null)
            | (Self::Null, Self::Meet32) => Ok(Self::JoinZero),

            (Self::Object(os1), Self::Object(os2)) => {
                assert!(!os1.is_empty());
                assert!(!os2.is_empty());
                let mut res = BTreeSet::new();
                for o1 in os1 {
                    for o2 in &os2 {
                        for lub in repo.least_common_types(&o1, o2)? {
                            res.insert(lub);
                        }
                    }
                }
                assert!(!res.is_empty());
                Ok(Self::Object(res))
            }

            (Self::Array(_, _), Self::Object(_)) | (Self::Object(_), Self::Array(_, _)) => {
                Ok(JAVA_LANG_OBJECT.clone())
            }

            (Self::Array(n1, t1), Self::Array(n2, t2)) => {
                if n1 == n2 {
                    Ok(Self::Array(n1, Box::new(t1.join(*t2, repo)?)))
                } else {
                    Ok(JAVA_LANG_OBJECT.clone())
                }
            }

            _ => Ok(Self::Top),
        }
    }

    pub(crate) fn meet(self, other: Self, repo: &Repo) -> AnalysisResult<Self> {
        if self.subseteq(&other, repo)? {
            return Ok(self);
        }
        if other.subseteq(&self, repo)? {
            return Ok(other);
        }
        match (self, other) {
            (Self::Double, Self::Long) => Ok(Self::Meet64),

            (Self::Float, Self::Integer)
            | (Self::Integer, Self::Float)
            | (Self::Float, Self::JoinZero)
            | (Self::JoinZero, Self::Float) => Ok(Self::Meet32),

            (Self::JoinZero, Self::Join32) | (Self::Join32, Self::JoinZero) => Ok(Self::Integer),

            (Self::Float, Self::Object(_))
            | (Self::Object(_), Self::Float)
            | (Self::Integer, Self::Object(_))
            | (Self::Object(_), Self::Integer)
            | (Self::Meet32, Self::Object(_))
            | (Self::Object(_), Self::Meet32)
            | (Self::Integer, Self::Array(_, _))
            | (Self::Array(_, _), Self::Integer)
            | (Self::Float, Self::Array(_, _))
            | (Self::Array(_, _), Self::Float)
            | (Self::Meet32, Self::Array(_, _))
            | (Self::Array(_, _), Self::Meet32)
            | (Self::Integer, Self::Null)
            | (Self::Null, Self::Integer)
            | (Self::Float, Self::Null)
            | (Self::Null, Self::Float)
            | (Self::Meet32, Self::Null)
            | (Self::Null, Self::Meet32) => Ok(Self::MeetZero),

            (Self::Object(os1), Self::Object(os2)) => {
                assert!(!os1.is_empty());
                assert!(!os2.is_empty());
                let mut res = os1.clone();
                res.append(&mut os2.clone());
                let mut ignore = BTreeSet::new();
                for o1 in &os1 {
                    let so1 = o1.as_str();
                    for o2 in &os2 {
                        let so2 = o2.as_str();
                        if repo.is_typeable_as(so1, so2)? {
                            res.remove(o2);
                            ignore.insert(o2);
                            break;
                        }
                    }
                }
                for o2 in &os2 {
                    if !ignore.contains(o2) {
                        let so2 = o2.as_str();
                        for o1 in &os1 {
                            let so1 = o1.as_str();
                            if repo.is_typeable_as(so2, so1)? {
                                res.remove(o1);
                                break;
                            }
                        }
                    }
                }
                Ok(Self::Object(res))
            }

            (Self::Array(_, _), Self::Object(_)) | (Self::Object(_), Self::Array(_, _)) => {
                Ok(Self::Null)
            }

            (Self::Array(n1, t1), Self::Array(n2, t2)) => {
                if n1 == n2 {
                    Ok(Self::Array(n1, Box::new(t1.meet(*t2, repo)?)))
                } else {
                    Ok(Self::Null)
                }
            }

            _ => Ok(Self::Bottom),
        }
    }
}
