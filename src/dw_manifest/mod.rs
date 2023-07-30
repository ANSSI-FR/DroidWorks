use crate::prelude::*;
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let filename = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let package = PackageOptions::manifest_only().open(filename)?;
    let manifest = package.manifest().expect("Android manifest");
    println!("{manifest}");

    Ok(())
}
