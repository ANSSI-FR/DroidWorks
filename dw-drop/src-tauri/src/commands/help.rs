use crate::errors::Error;

use tauri::{api::shell, command, Manager, Window};
use std::path::{Path, PathBuf, Component, Prefix};

#[command]
pub async fn open_help(window: Window) -> Result<(), Error> {
    let doc_index_path = window
        .app_handle()
        .path_resolver()
        .resolve_resource("../docs/build/html/index.html")
        .ok_or_else(|| Error::FailToOpenHelp("doc resource not found".to_string()))?;

    let head = doc_index_path.components()
        .next()
        .ok_or_else(|| Error::FailToOpenHelp("empty doc resource path".to_string()))?;
    let disk;
    let head = if let Component::Prefix(p) = head {
        if let Prefix::VerbatimDisk(d) = p.kind() {
            disk = format!("{}:", d as char);
            Path::new(&disk)
                .components()
                .next()
                .ok_or_else(|| Error::FailToOpenHelp("verbatim path prefix error".to_string()))?
        } else {
            head
        }
    } else {
        head
    };
    let doc_index = std::iter::once(head)
        .chain(doc_index_path.components().skip(1))
        .collect::<PathBuf>();
    
    let url = format!("file://{}", doc_index.to_str().unwrap());
    shell::open(&window.shell_scope(), url, None).map_err(|e| Error::FailToOpenHelp(e.to_string()))
}