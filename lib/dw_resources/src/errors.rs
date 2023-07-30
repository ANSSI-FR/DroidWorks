use std::{fmt, io};
use thiserror::Error;

pub type ResourcesResult<T> = Result<T, ResourcesError>;

#[derive(Debug, Error)]
pub enum ResourcesError {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    #[error("Format error: {0}")]
    Fmt(#[from] fmt::Error),

    #[error("parsing error")]
    Parsing(Vec<u8>, nom::error::ErrorKind),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("invalid UTF-8")]
    InvalidUtf8(String),

    #[error("invalid UTF-16")]
    InvalidUtf16(String),

    #[error("resources structure is invalid: {0}")]
    Structure(String),

    #[error("cannot resolve a resource without resources: {0}")]
    CannotResolveWithoutResources(String),

    #[error("resource not found in resources tables: {0}")]
    ResNotFound(String),

    #[error("resource already defined in resources tables: {0}")]
    ResAlreadyDefined(String),

    #[error("unexpected value: {name} is {typ}")]
    UnexpectedValue { name: String, typ: String },

    #[error("too complex resource: {0}")]
    TooComplexResource(String),

    #[error("xml query failed: {0}")]
    XmlQuery(String),

    #[error("value type error: {0}")]
    ValueType(String),
}

impl nom::error::ParseError<&[u8]> for ResourcesError {
    fn from_error_kind(input: &[u8], kind: nom::error::ErrorKind) -> Self {
        Self::Parsing(input.to_vec(), kind)
    }

    fn append(_: &[u8], _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
