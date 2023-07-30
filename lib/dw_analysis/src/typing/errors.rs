//! Typing errors definitions.

use crate::typing::AbstractType;
use dw_dex::errors::DexError;
use dw_dex::registers::Reg;
use thiserror::Error;

/// An alias for result that can be a [`TypeError`].
pub type TypeResult<T> = Result<T, TypeError>;

/// The typing error type.
#[derive(Debug, Error)]
pub enum TypeError {
    /// Error that can be returned when trying to access invalid Dex structure.
    #[error("Dex error: {0}")]
    Dex(#[from] DexError),

    #[error("Subtyping error: {0} is not a subtype of {1}")]
    NotASubtype(AbstractType, AbstractType),

    #[error("Bad pair types: {} doesn't match {}", type1, type2)]
    BadPairTypes { type1: String, type2: String }, // TODO: replace String for Type ?

    #[error("Out of bounds register: {0}")]
    OutOfBoundsRegister(Reg),

    #[error("Trying to read result after no call was invoked")]
    MissingResult,

    #[error("Trying to read exception after no exception was thrown")]
    MissingException,

    #[error("Declared return and effective return type doesn't match")]
    BadReturnType,

    #[error("Trying to merge incompatible typing states")]
    IncompatibleStates,

    #[error("Invalid array element type")]
    InvalidArrayElementType,

    #[error("Invalid field type")]
    InvalidFieldType,

    #[error("Missing 'this' argument at method invocation")]
    MissingThisArgument,

    #[error("Bad arity")]
    BadArity,

    #[error("Class type expected")]
    ExpectedClass,

    #[error("Array type expected")]
    ExpectedArray,

    #[error("Exception was expected as last result")]
    ExpectedException,

    #[error("Any result (but not an exception) was expected as last result")]
    ExpectedResult,
}
