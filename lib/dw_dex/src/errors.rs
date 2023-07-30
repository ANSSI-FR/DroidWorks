//! Dex errors definitions.

use crate::Addr;
use std::{fmt, io};
use thiserror::Error;

/// An alias for result that can be a [`DexError`].
pub type DexResult<T> = Result<T, DexError>;

/// The Dex error type.
#[derive(Debug, Error)]
pub enum DexError {
    /// Error that can be returned when doing [std::io](I/O) operations.
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    /// Error that can be returned when formatting dex parts.
    #[error("Formatting error: {0}")]
    Fmt(#[from] fmt::Error),

    /// Error that can be returned at parsing.
    #[error("parsing error")]
    Parsing(Vec<u8>, nom::error::ErrorKind),

    /// Custom internal error type.
    #[error("internal error: {0}")]
    Internal(String),

    /// Invalid MUTF-8 string.
    #[error("invalid MUTF-8: {0}")]
    InvalidMutf8(String),

    #[error("dex structure is invalid: {0}")]
    Structure(String),

    #[error("dex {0} has bad size")]
    BadSize(String),

    #[error("dex {0} has invalid offset")]
    InvalidOffset(String),

    #[error("unexpected padding value in dex")]
    NonZeroPadding,

    #[error("resource not found in dex tables: {0}")]
    ResNotFound(String),

    #[error("could not convert {} into {}", from, to)]
    Conversion { from: String, to: String },

    #[error("invalid type")]
    InvalidType,

    #[error("Compilation error ({0}): {1}")]
    Compile(String, String),

    #[error("Instruction not found (address: {0})")]
    InstructionNotFound(Addr),

    #[error("Bad instruction(s) size")]
    BadInstructionSize,
}

impl nom::error::ParseError<&[u8]> for DexError {
    fn from_error_kind(input: &[u8], kind: nom::error::ErrorKind) -> Self {
        Self::Parsing(input.to_vec(), kind)
    }

    fn append(_: &[u8], _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl nom::error::ParseError<(&[u8], usize)> for DexError {
    fn from_error_kind(input: (&[u8], usize), kind: nom::error::ErrorKind) -> Self {
        Self::Parsing(input.0.to_vec(), kind)
    }

    fn append(_: (&[u8], usize), _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl nom::ErrorConvert<Self> for DexError {
    fn convert(self) -> Self {
        self
    }
}
