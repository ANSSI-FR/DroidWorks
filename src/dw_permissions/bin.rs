//! A tool to list, edit and compare Android applications permissions.
//!
//! Permissions are extracted from the Android Manifest of the given
//! application.

use droidworks::prelude::DwResult;
use droidworks::{cli, dw_permissions};

fn main() -> DwResult<()> {
    let args = cli::permissions().get_matches();
    dw_permissions::run(&args)
}
