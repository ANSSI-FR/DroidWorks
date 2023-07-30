use droidworks::prelude::*;

// Each error defines its arguments, the lesser arguments it has,
// the easier the translation can be done in the i18n frontend.
#[derive(serde::Serialize)]
pub enum Error {
    Package(String),
    Resources(String),
    IO(String),
    Internal(u32),
    InvalidApk,
    NoApk,
    ZipAlignFailed,
    ApkSignerFailed,
    InvalidBase64,
    InvalidFlag,
    InvalidArguments,
    FailToOpenHelp(String),
}

impl From<PackageError> for Error {
    fn from(e: PackageError) -> Self {
        Self::Package(e.to_string())
    }
}

impl From<ResourcesError> for Error {
    fn from(e: ResourcesError) -> Self {
        Self::Resources(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e.to_string())
    }
}
