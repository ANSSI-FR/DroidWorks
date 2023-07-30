use dw_dex::registers::Reg;
use thiserror::Error;

pub type FlowResult<T> = Result<T, FlowError>;

#[derive(Debug, Error)]
pub enum FlowError {
    /*
    /// Error that can be returned when trying to access invalid Dex structure.
    #[error("Dex error: {0}")]
    Dex(#[from] DexError),

    #[error("Subtyping error: {0} is not a subtype of {1}")]
    NotASubtype(AbstractType, AbstractType),
    */
    #[error("Bad pair types: {} doesn't match {}", type1, type2)]
    BadPairTypes { type1: String, type2: String }, // TODO: replace String for Type ?

    #[error("Out of bounds register: {0}")]
    OutOfBoundsRegister(Reg),

    #[error("Trying to read result after no call was invoked")]
    MissingResult,

    #[error("Trying to read exception after no exception was thrown")]
    MissingException,

    #[error("Trying to merge incompatible flow states")]
    IncompatibleStates,
    /*
    #[error("Invalid array element type")]
    InvalidArrayElementType,

    #[error("Invalid field type")]
    InvalidFieldType,
     */
    #[error("Missing 'this' argument at method invocation")]
    MissingThisArgument,

    #[error("Bad arity")]
    BadArity,

    /*
    #[error("Class type expected")]
    ExpectedClass,

    #[error("Array type expected")]
    ExpectedArray,

    #[error("Exception was expected as last result")]
    ExpectedException,

    #[error("Any result (but not an exception) was expected as last result")]
    ExpectedResult,
    */
    #[error("Trying to access an unknown AMG vertex")]
    UnknownAmgVertex,

    #[error("Trying to access an unknown class")]
    UnknownClass,

    #[error(
        "Trying to access a field of something not an instance, a parameter or a static value"
    )]
    InvalidFieldAccess,

    #[error("Trying error")]
    TypingError,
}
