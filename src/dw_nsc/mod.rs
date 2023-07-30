use crate::prelude::*;
use clap::ArgMatches;
use std::path::PathBuf;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let filename = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let mut package = PackageOptions::default().dont_parse_dex().open(filename)?;
    let manifest = package.manifest().expect("Android manifest");
    let resources = package.resources();
    let nsc_path = match manifest.network_security_config(resources)? {
        None => {
            log::warn!("cannot find network security config xml file");
            return Ok(());
        }
        Some(path) => {
            log::info!("network security config is in '{path}'");
            path
        }
    };
    package.set_nsc_path(PathBuf::from(nsc_path))?;
    let nsc = package
        .network_security_config()
        .expect("network security config");
    println!("{nsc}");

    Ok(())
}
