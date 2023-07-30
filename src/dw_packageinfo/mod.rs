use crate::prelude::*;
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let filename = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let package = Package::open(filename)?;
    println!("{package}");

    let manifest = package.manifest().expect("Android manifest");
    println!(
        " - package name: {}",
        manifest.package()?.expect("application package name")
    );
    println!(
        " - version code: {}, version name: {}",
        manifest.version_code()?.expect("application version code"),
        manifest.version_name()?.expect("application version name"),
    );
    println!(" - uses-permissions:");
    for permission in &manifest.uses_permissions(package.resources())? {
        println!("   - {}", permission.name().expect("permission name"));
    }
    println!(" - uses-features:");
    for feature in &manifest.uses_features(package.resources())? {
        println!("   - {}", feature.name().expect("feature name"));
    }
    println!(" - activities:");
    for activity in &manifest.activities(package.resources())? {
        println!("   - {}", activity.name().expect("activity name"));
    }
    println!(" - services:");
    for service in &manifest.services(package.resources())? {
        println!("   - {}", service.name().expect("service name"));
    }
    println!(" - receivers:");
    for receiver in &manifest.receivers(package.resources())? {
        println!("   - {}", receiver.name().expect("receiver name"));
    }
    println!(" - providers:");
    for provider in &manifest.providers(package.resources())? {
        println!("   - {}", provider.name().expect("provider name"));
    }

    Ok(())
}
