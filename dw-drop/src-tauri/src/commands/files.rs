use crate::errors::Error;
use crate::state::DwState;
use crate::{read_state, write_state};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{command, State};

#[derive(Serialize)]
pub struct FileEntry {
    name: String,
    size: usize,
}

#[command]
pub async fn files(state: State<'_, DwState>) -> Result<Vec<FileEntry>, Error> {
    read_state!(state.package => |package| {
        package
            .iter_filenames_with_size()
            .map(|(path, size)| Ok(FileEntry {
                name: path.to_str().ok_or_else(|| Error::Internal(400)).map(ToString::to_string)?,
                size,
            }))
            .collect()
    })
}

#[command]
pub async fn extract_file(
    asset: String,
    filename: String,
    state: State<'_, DwState>,
) -> Result<(), Error> {
    read_state!(state.package => |package| {
        Ok(package.extract_file(&PathBuf::from(asset), filename)?)
    })
}

#[command]
pub async fn file(asset: String, state: State<'_, DwState>) -> Result<String, Error> {
    read_state!(state.package => |package| {
        Ok(package.base64_file(&PathBuf::from(asset))?)
    })
}

#[command]
pub async fn remove_files(assets: Vec<String>, state: State<'_, DwState>) -> Result<(), Error> {
    write_state!(state.package => |package| {
        for asset in assets {
            let asset = PathBuf::from(asset);
            package.remove_file(&asset)?;
        }
        Ok(())
    })
}

#[command]
pub async fn replace_file_other(
    asset: String,
    content: String,
    state: State<'_, DwState>,
) -> Result<(), Error> {
    write_state!(state.package => |package| {
        let path = PathBuf::from(asset);
        let dec = base64::decode(content).map_err(|_| Error::InvalidBase64)?;
        package.replace_file_other(path, dec)?;
        Ok(())
    })
}
