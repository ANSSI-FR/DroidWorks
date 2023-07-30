//! Global error handling.
//!
//! Each sub-crate of the project defines its own type error.
//! Their types can be unified, for example in a main function,
//! when winding results at the top-level.
//!
//! ```rust
//! use droidworks::prelude::*;
//! use droidworks::dex;
//!
//! fn main() -> DwResult<()> { // can return a DwError
//!    let _dex = dex::open("apk/pickcontacts.dex")?; // can return a DexError
//!    Ok(())
//! }
//! ```

use dw_analysis::errors::AnalysisError;
use dw_dex::errors::DexError;
use dw_package::errors::PackageError;
use dw_resources::errors::ResourcesError;
use dw_utils::zipalign::ZipError;
use std::io;
use thiserror::Error;

/// An alias for result that can be a [`DwError`].
pub type DwResult<T> = Result<T, DwError>;

/// The main error type for error winding at the top-level.
/// It mainly consists of transparent wrapper over error types that
/// are defined in dependencies.
#[derive(Debug, Error)]
pub enum DwError {
    /// Custom error for reporting bad command line arguments usage.
    #[error("bad arguments: {0}")]
    BadArguments(String),

    /// Error that can be returned from [I/O operations](std::io).
    #[error(transparent)]
    IO(#[from] io::Error),

    /// Error that can be returned from regex compilation.
    #[error(transparent)]
    Regex(#[from] regex::Error),

    /// Error that can be returned from [`dw_analysis`] functions.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),

    /// Error that can be returned from [`dw_package`] functions.
    #[error(transparent)]
    Package(#[from] PackageError),

    /// Error that can be returned from [`dw_dex`] functions.
    #[error(transparent)]
    Dex(#[from] DexError),

    /// Error that can be returned from [`dw_utils`] zip functions.
    #[error(transparent)]
    Zip(#[from] ZipError),

    /// Error that can be returned from [`dw_resources`] functions.
    #[error(transparent)]
    Resources(#[from] ResourcesError),
}
