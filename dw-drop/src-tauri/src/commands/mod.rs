pub mod apksigner;
pub mod dex;
pub mod files;
pub mod help;
pub mod manifest;
pub mod nsc;

use crate::errors::Error;
use crate::read_state;
use crate::state::DwState;
use droidworks::prelude::*;
use tauri::{command, State};

#[command]
pub async fn open_application(filename: String, state: State<'_, DwState>) -> Result<(), Error> {
    *state.package.write().map_err(|_| Error::Internal(600))? =
        Some(PackageOptions::default().open(filename)?);
    Ok(())
}

#[command]
pub async fn save_application(filename: String, state: State<'_, DwState>) -> Result<(), Error> {
    read_state!(state.package => |package| {
        Ok(package.save(filename, true)?)
    })
}
