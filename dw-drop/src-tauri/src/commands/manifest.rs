use crate::errors::Error;
use crate::state::DwState;
use crate::{read_state, write_state};
use droidworks::prelude::{Manifest, ManifestTag};
use droidworks::resources::values::ResolvedValue;
use tauri::{command, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfo {
    package: Option<String>,
    version_code: Option<u32>,
    version_name: Option<String>,
    min_sdk_version: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationFlag {
    name: String,
    value: Option<bool>,
    previous: Option<bool>,
}

#[command]
pub async fn application_infos(state: State<'_, DwState>) -> Result<ApplicationInfo, Error> {
    read_state!(state.package => |package| {
        let manifest = package.manifest().ok_or(Error::InvalidApk)?;
        let resources = package.resources();
        let package = manifest.package()?;
        let version_code = manifest.version_code()?;
        let version_name = manifest.version_name()?;
        let min_sdk_version = match &manifest.uses_sdk(resources)?[..] {
            [] => {
                println!("[WARN] no uses-sdk tag found in manifest");
                None
            }
            [tag] => match tag.attributes["minSdkVersion"] {
                Some(ResolvedValue::Int(i)) => Some(i),
                Some(_) => {
                    println!("[WARN] minSdkVersion is not an int");
                    None
                }
                None => None,
            },
            _ => {
                println!("[WARN] multiple uses-sdk tags found in manifest");
                None
            }
        };
        Ok(ApplicationInfo {
            package,
            version_code,
            version_name,
            min_sdk_version,
        })
    })
}

#[command]
pub async fn application_flags(state: State<'_, DwState>) -> Result<Vec<ApplicationFlag>, Error> {
    read_state!(state.package => |package| {
        let manifest = package.manifest().ok_or(Error::InvalidApk)?;
        let resources = package.resources();
        let allow_backup = manifest.allow_backup(resources)?;
        let allow_clear_user_data = manifest.allow_clear_user_data(resources)?;
        let debuggable = manifest.debuggable(resources)?;
        let uses_cleartext_traffic = manifest.uses_cleartext_traffic(resources)?;
        Ok(vec![
            ApplicationFlag {
                name: "allow_backup".to_string(),
                value: allow_backup,
                previous: allow_backup,
            },
            ApplicationFlag {
                name: "allow_clear_user_data".to_string(),
                value: allow_clear_user_data,
                previous: allow_clear_user_data,
            },
            ApplicationFlag {
                name: "debuggable".to_string(),
                value: debuggable,
                previous: debuggable,
            },
            ApplicationFlag {
                name: "uses_cleartext_traffic".to_string(),
                value: uses_cleartext_traffic,
                previous: uses_cleartext_traffic,
            },
        ])
    })
}

#[command]
pub async fn set_application_flags(
    flags: Vec<ApplicationFlag>,
    state: State<'_, DwState>,
) -> Result<(), Error> {
    write_state!(state.package => |package| {
        let manifest = package.manifest_mut().ok_or(Error::InvalidApk)?;
        for flag in flags.into_iter().filter(|flag| flag.value != flag.previous) {
            match flag.name.as_str() {
                "allow_backup" => manifest.set_allow_backup(flag.value)?,
                "allow_clear_user_data" => manifest.set_allow_clear_user_data(flag.value)?,
                "debuggable" => manifest.set_debuggable(flag.value)?,
                "uses_cleartext_traffic" => manifest.set_uses_cleartext_traffic(flag.value)?,
                _ => Err(Error::InvalidFlag)?,
            }
        }
        Ok(())
    })
}

macro_rules! manifest_collection_commands {
    (
        <= $collection:ident using $getter:ident,
        => $drop_collection:ident using $eraser:ident,
    ) => {
        #[command]
        pub async fn $collection(state: State<'_, DwState>) -> Result<Vec<ManifestTag>, Error> {
            read_state!(state.package => |package| {
                let manifest = package.manifest().ok_or(Error::InvalidApk)?;
                let resources = package.resources();
                let collection_items = Manifest::$getter(manifest, resources)?;
                Ok(collection_items.into_iter().collect())
            })
        }

        #[command]
        pub fn $drop_collection(items: Vec<String>, state: State<DwState>) -> Result<(), Error> {
            write_state!(state.package => |package| {
                let manifest = package.manifest_mut().ok_or(Error::InvalidApk)?;
                for item in items {
                    Manifest::$eraser(manifest, &item)?;
                }
                Ok(())
            })
        }
    }
}

manifest_collection_commands! {
    <= permissions using uses_permissions,
    => drop_permissions using remove_uses_permission,
}

#[command]
pub fn add_permissions(items: Vec<String>, state: State<DwState>) -> Result<(), Error> {
    write_state!(state.package => |package| {
        let manifest = package.manifest_mut().ok_or(Error::InvalidApk)?;
        for item in items {
            Manifest::add_uses_permission(manifest, &item)?;
        }
        Ok(())
    })
}

manifest_collection_commands! {
    <= activities using activities,
    => drop_activities using remove_activity,
}
manifest_collection_commands! {
    <= services using services,
    => drop_services using remove_service,
}
manifest_collection_commands! {
    <= receivers using receivers,
    => drop_receivers using remove_receiver,
}
manifest_collection_commands! {
    <= providers using providers,
    => drop_providers using remove_provider,
}
manifest_collection_commands! {
    <= features using uses_features,
    => drop_features using remove_uses_feature,
}
