//! Analysis errors definition.

use crate::typing::errors::TypeError;
use dw_dex::errors::DexError;
use dw_package::errors::PackageError;
use regex::Error as RegexError;
use std::path::PathBuf;
use thiserror::Error;

pub type AnalysisResult<T> = Result<T, AnalysisError>;

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("unknown file extension: {0}")]
    FileExt(PathBuf),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("package error: {0}")]
    Package(#[from] PackageError),

    #[error("dex error: {0}")]
    Dex(#[from] DexError),

    #[error("regex error: {0}")]
    Regex(#[from] RegexError),

    #[error("class not found: {0}")]
    ClassNotFound(String),

    #[error("instruction not found: {0}")]
    InstructionNotFound(String),

    #[error("the method has no implementation")]
    NoCode,

    #[error("typing error: {0}")]
    Type(#[from] TypeError),
}
