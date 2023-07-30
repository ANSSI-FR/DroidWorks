use crate::errors::Error;
use droidworks::utils::zipalign;
use std::path::PathBuf;
use std::process::Command;
use tauri::{command, AppHandle};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SigningData {
    application_path: String,
    signer: Signer,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Signer {
    Keystore {
        ks_path: String,
        ks_pass: String,
    },
    KeyAndCert {
        key_path: String,
        key_pass: String,
        cert_path: String,
    },
}

#[command]
pub async fn apksigner(signing: SigningData, app_handle: AppHandle) -> Result<String, Error> {
    let filename = PathBuf::from(&signing.application_path);
    let new_filename = filename.with_extension("signed.apk");
    let new_filename_str = String::from(new_filename.to_str().ok_or_else(|| Error::Internal(200))?);
    let fname_ret = new_filename_str.clone();

    if zipalign::process(
        &signing.application_path,
        &new_filename_str,
        4,
        true,  // force
        false, // zopfli
        true,  // page_align_shared_libs
    )
    .is_err()
    {
        return Err(Error::ZipAlignFailed);
    }

    let mut apksigner_path = app_handle
        .path_resolver()
        .resource_dir()
        .ok_or(Error::InvalidApk)?;
    apksigner_path.push("external");
    apksigner_path.push("apksigner.jar");

    let result: Result<(), Error> = tauri::async_runtime::spawn(async move {
        let mut command = Command::new("java");
        command
            .args([
                "-jar",
                apksigner_path.to_str().ok_or_else(|| Error::Internal(201))?,
            ])
            .arg("sign");
        match signing.signer {
            Signer::Keystore { ks_path, ks_pass } => command
                .args(["--ks", &ks_path])
                .args(["--ks-pass", &format!("pass:{}", &ks_pass)]),
            Signer::KeyAndCert {
                key_path,
                key_pass,
                cert_path,
            } => {
                let mut cmd = command.args(["--key", &key_path]);
                if !key_pass.is_empty() {
                    cmd = cmd.args(["--key-pass", &format!("pass:{}", &key_pass)]);
                }
                cmd.args(["--cert", &cert_path])
            }
        };
        let mut child = command
            .arg(&new_filename_str)
            .spawn()
            .map_err(|_| Error::ApkSignerFailed)?;
        let status = child
            .wait()
            .map_err(|_| Error::ApkSignerFailed)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::ApkSignerFailed)
        }
    })
    .await
    .map_err(|_| Error::Internal(202))?;

    result.map(|()| fname_ret)
}
