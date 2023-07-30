use crate::errors::Error;
use crate::state::DwState;
use crate::{read_state, write_state};
use droidworks::resources;
use std::path::PathBuf;
use tauri::{command, State};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Nsc {
    typ: String,
    name: Option<String>,
}

#[command]
pub async fn available_nscs(state: State<'_, DwState>) -> Result<Vec<Nsc>, Error> {
    write_state!(state.package => |package| {
        let manifest = package.manifest().ok_or(Error::InvalidApk)?;
        let resources = package.resources().ok_or(Error::InvalidApk)?;
        let nsc = manifest.network_security_config(Some(resources))?;
        if let Some(nsc) = &nsc {
            let nsc_path = PathBuf::from(nsc);
            package.set_nsc_path(nsc_path)?;
        }
        let is_some = nsc.is_some();
        let mut nscs = vec![Nsc {
            typ: "original".to_string(),
            name: nsc,
        }];
        for nsc_name in resources::nsc::AVAILABLE_CUSTOM_NSCS {
            nscs.push(Nsc {
                typ: "custom".to_string(),
                name: Some(nsc_name.to_string()),
            });
        }
        if is_some {
            nscs.push(Nsc {
                typ: "none".to_string(),
                name: None,
            });
        }
        Ok(nscs)
    })
}

#[command]
pub async fn get_nsc(nsc: Nsc, state: State<'_, DwState>) -> Result<Option<String>, Error> {
    read_state!(state.package => |package| {
        match nsc.typ.as_str() {
            "original" => Ok(package
                .network_security_config()
                .map(|nsc| format!("{}", nsc))),
            "custom" => {
                let name = nsc.name.ok_or(Error::InvalidArguments)?;
                let nsc = resources::nsc::network_security_config(&name)?;
                Ok(Some(format!("{}", nsc)))
            }
            "none" => Ok(None),
            _ => Err(Error::InvalidFlag),
        }
    })
}

#[command]
pub async fn commit_nsc(nsc: Nsc, state: State<'_, DwState>) -> Result<(), Error> {
    write_state!(state.package => |package| {
        match nsc.typ.as_str() {
            "original" => Ok(()),
            "custom" => {
                // prepare custom nsc
                let name = nsc.name.ok_or(Error::InvalidArguments)?;
                let nsc_parsed = resources::nsc::network_security_config(&name)?;
                let nsc_xml = resources::nsc::write(&nsc_parsed)?;

                // check if there is a nsc to replace
                let manifest = package.manifest().ok_or(Error::InvalidApk)?;
                let resources = package.resources().ok_or(Error::InvalidApk)?;
                let nsc = manifest.network_security_config(Some(resources))?;
                if nsc.is_some() {
                    // replace previous nsc
                    package.replace_file_nsc(nsc_xml)?;
                } else {
                    // insert new nsc in apk
                    let manifest_mut = package.manifest_mut().ok_or(Error::InvalidApk)?;
                    manifest_mut
                        .insert_network_security_config("res/network_security_config.xml")?;
                    package
                        .insert_file(PathBuf::from("res/network_security_config.xml"), nsc_xml)?;
                }

                Ok(())
            }
            "none" => {
                let manifest = package.manifest().ok_or(Error::InvalidApk)?;
                let resources = package.resources().ok_or(Error::InvalidApk)?;
                let nsc = manifest
                    .network_security_config(Some(resources))?
                    .ok_or(Error::InvalidApk)?;

                let manifest_mut = package.manifest_mut().ok_or(Error::InvalidApk)?;
                manifest_mut.remove_network_security_config()?;
                package.remove_file(&PathBuf::from(nsc))?;
                Ok(())
            }
            _ => Err(Error::InvalidFlag),
        }
    })
}
