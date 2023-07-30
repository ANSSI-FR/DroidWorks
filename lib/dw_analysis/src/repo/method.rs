use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::MethodUid;
use dw_dex::code::CodeItem;
use dw_dex::methods::{EncodedMethod, MethodFlags, MethodIdItem};
use dw_dex::types::Type;
use dw_dex::Dex;
use std::fmt;
use std::sync::RwLock;

/// The enriched method definition.
#[derive(Debug, Clone)]
pub struct Method<'a> {
    // Unique identifier in the repository
    uid: MethodUid,
    // Dex method data
    content: &'a EncodedMethod,
    // Dex reference to resolve dex data
    dex: &'a Dex,
    // Cache of names and types that identify the method
    descriptor: MethodDescr,
}

impl<'a> Method<'a> {
    pub(crate) fn new(
        uid: MethodUid,
        encoded_method: &'a EncodedMethod,
        dex: &'a Dex,
    ) -> AnalysisResult<Self> {
        let descriptor = MethodDescr::try_from((dex, encoded_method.descriptor(dex)?))?;
        Ok(Self {
            uid,
            content: encoded_method,
            dex,
            descriptor,
        })
    }

    #[inline]
    pub fn uid(&self) -> MethodUid {
        self.uid
    }

    #[must_use]
    pub const fn dex(&self) -> &Dex {
        self.dex
    }

    #[inline]
    pub fn descriptor(&self) -> &MethodDescr {
        &self.descriptor
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.descriptor().name()
    }

    #[inline]
    pub fn definer(&self) -> &MethodDefiner {
        self.descriptor().definer()
    }

    #[inline]
    pub fn return_type(&self) -> &Type {
        self.descriptor().return_type()
    }

    #[inline]
    pub fn parameters_types(&self) -> &Vec<Type> {
        self.descriptor().parameters_types()
    }

    #[must_use]
    pub fn code(&self) -> Option<&RwLock<CodeItem>> {
        self.content.code(self.dex).ok().flatten()
    }

    #[inline]
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_PUBLIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_private(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_PRIVATE)
    }

    #[inline]
    #[must_use]
    pub const fn is_protected(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_PROTECTED)
    }

    #[inline]
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_STATIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_FINAL)
    }

    #[inline]
    #[must_use]
    pub const fn is_synchronized(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_SYNCHRONIZED)
    }

    #[inline]
    #[must_use]
    pub const fn is_bridge(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_BRIDGE)
    }

    #[inline]
    #[must_use]
    pub const fn is_varargs(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_VARARGS)
    }

    #[inline]
    #[must_use]
    pub const fn is_native(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_NATIVE)
    }

    #[inline]
    #[must_use]
    pub const fn is_abstract(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_ABSTRACT)
    }

    #[inline]
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_STRICT)
    }

    #[inline]
    #[must_use]
    pub const fn is_synthetic(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_SYNTHETIC)
    }

    #[inline]
    #[must_use]
    pub const fn is_constructor(&self) -> bool {
        self.content.flags().contains(MethodFlags::ACC_CONSTRUCTOR)
    }

    #[inline]
    #[must_use]
    pub const fn is_declared_synchronized(&self) -> bool {
        self.content
            .flags()
            .contains(MethodFlags::ACC_DECLARED_SYNCHRONIZED)
    }
}

/// A wrapper to cache prototype information of a method and to allow
/// deriving of eq and ord traits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodDescr {
    definer: MethodDefiner,
    name: String,
    return_type: Type,
    parameters_types: Vec<Type>,
}

impl TryFrom<(&Dex, &MethodIdItem)> for MethodDescr {
    type Error = AnalysisError;

    fn try_from((dex, method): (&Dex, &MethodIdItem)) -> Result<Self, Self::Error> {
        Ok(Self {
            definer: MethodDefiner::try_from(&method.definer(dex)?)?,
            name: method.name(dex)?,
            return_type: method.return_type(dex)?,
            parameters_types: method.parameters_types(dex)?,
        })
    }
}

impl fmt::Display for MethodDescr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let parameters = self
            .parameters_types
            .iter()
            .map(|t| format!("{t}"))
            .collect::<String>();
        write!(
            f,
            "{}->{}({}){}",
            self.definer, self.name, parameters, self.return_type
        )
    }
}

impl MethodDescr {
    #[inline]
    pub fn definer(&self) -> &MethodDefiner {
        &self.definer
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn return_type(&self) -> &Type {
        &self.return_type
    }

    #[inline]
    pub fn parameters_types(&self) -> &Vec<Type> {
        &self.parameters_types
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MethodDefiner {
    Class(String),
    Array(usize, String),
}

impl TryFrom<&Type> for MethodDefiner {
    type Error = AnalysisError;

    fn try_from(typ: &Type) -> Result<Self, Self::Error> {
        match typ {
            Type::Class(cl) => Ok(Self::Class(cl.clone())),
            Type::Array(n, t) => Ok(Self::Array(*n, format!("{}", t))),
            _ => Err(AnalysisError::Internal(
                "method definer must be a class type or array type".to_string(),
            )),
        }
    }
}

impl fmt::Display for MethodDefiner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Class(cl) => write!(f, "{}", cl),
            Self::Array(n, t) => {
                for _ in 0..*n {
                    write!(f, "[")?;
                }
                write!(f, "{};", t)
            }
        }
    }
}

impl MethodDefiner {
    pub fn class_name(&self) -> String {
        match self {
            Self::Class(cl) => cl.clone(),
            Self::Array(_, _) => "java/lang/reflect/Array".to_string(),
        }
    }
}
