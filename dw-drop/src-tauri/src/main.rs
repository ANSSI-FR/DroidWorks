#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod errors;
mod state;

use crate::state::DwState;

fn main() {
    tauri::Builder::default()
        .manage(DwState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_application,
            commands::save_application,
            commands::files::files,
            commands::files::file,
            commands::files::extract_file,
            commands::files::remove_files,
            commands::files::replace_file_other,
            commands::dex::extract_dex_strings,
            commands::dex::dex_strings,
            commands::manifest::application_infos,
            commands::manifest::application_flags,
            commands::manifest::set_application_flags,
            commands::manifest::permissions,
            commands::manifest::drop_permissions,
            commands::manifest::add_permissions,
            commands::manifest::activities,
            commands::manifest::drop_activities,
            commands::manifest::services,
            commands::manifest::drop_services,
            commands::manifest::receivers,
            commands::manifest::drop_receivers,
            commands::manifest::providers,
            commands::manifest::drop_providers,
            commands::manifest::features,
            commands::manifest::drop_features,
            commands::nsc::available_nscs,
            commands::nsc::get_nsc,
            commands::nsc::commit_nsc,
            commands::apksigner::apksigner,
            commands::help::open_help,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
