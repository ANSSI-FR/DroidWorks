//! Package errors definitions.

use dw_dex::errors::DexError;
use dw_resources::errors::ResourcesError;
use std::io;
use thiserror::Error;
use zip::result::ZipError;

/// An alias for result that can be a [`PackageError`].
pub type PackageResult<T> = Result<T, PackageError>;

/// The package error type.
#[derive(Debug, Error)]
pub enum PackageError {
    /// Error that can be returned when doing [std::io](I/O) operations.
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    /// Error that can be returned when opening or saving a zip file.
    #[error("zip error: {0}")]
    Zip(#[from] ZipError),

    #[error("file has been modified: {0}")]
    FileHasBeenModified(String),

    #[error(transparent)]
    Dex(#[from] DexError),

    #[error(transparent)]
    Resources(#[from] ResourcesError),
}
