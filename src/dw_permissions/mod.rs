use crate::prelude::*;
use clap::ArgMatches;
use nu_ansi_term::Color;
use std::collections::BTreeSet;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    match &args.subcommand() {
        Some(("list", sub_args)) => list_permissions(sub_args),
        Some(("diff", sub_args)) => diff_permissions(sub_args),
        Some(("modify", sub_args)) => modify_permissions(sub_args),
        Some((subcommand, _)) => Err(DwError::BadArguments(format!(
            "unknown subcommand '{subcommand}'",
        ))),
        None => Err(DwError::BadArguments("missing subcommand".to_string())),
    }
}

/// List and print permissions from application given on command line arguments.
fn list_permissions(sub_args: &ArgMatches) -> DwResult<()> {
    let filename = sub_args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("input file needed".to_string()))?;

    for perm in get_permissions(filename)? {
        println!("  {perm}");
    }

    Ok(())
}

/// Compute and print permissions diff between two applications
/// given on command line arguments.
fn diff_permissions(sub_args: &ArgMatches) -> DwResult<()> {
    let filename1 = sub_args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("input file needed".to_string()))?;
    let filename2 = sub_args
        .get_one::<String>("output")
        .ok_or_else(|| DwError::BadArguments("output file needed".to_string()))?;
    print_diff(filename1, filename2, sub_args.get_flag("diff-only"))
}

/// Print permissions diff between the two applications passed as arguments.
fn print_diff(filename1: &str, filename2: &str, diff_only: bool) -> DwResult<()> {
    let mut permissions1 = get_permissions(filename1)?
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let mut permissions2 = get_permissions(filename2)?
        .into_iter()
        .rev()
        .collect::<Vec<_>>();

    while let Some(perm1) = permissions1.pop() {
        let mut still_there = false;
        while let Some(perm2) = permissions2.pop() {
            if perm2 < perm1 {
                println!("{}", Color::Red.paint(&format!("+ {perm2}")));
                continue;
            }
            if perm2 > perm1 {
                permissions2.push(perm2);
            } else {
                still_there = true;
            }
            break;
        }
        if still_there {
            if !diff_only {
                println!("  {perm1}");
            }
        } else {
            println!("{}", Color::Green.paint(&format!("- {perm1}")));
        }
    }
    while let Some(perm2) = permissions2.pop() {
        println!("{}", Color::Red.paint(&format!("+ {perm2}")));
    }

    Ok(())
}

/// Given an application filename, returns a set of required permissions names.
fn get_permissions(filename: &str) -> DwResult<BTreeSet<String>> {
    let package = Package::open(filename)?;
    let resources = package.resources();
    let manifest = package.manifest().ok_or_else(|| {
        DwError::BadArguments(format!("{filename} is not a valid Android package"))
    })?;
    let permissions = manifest.uses_permissions(resources)?;
    Ok(permissions
        .into_iter()
        .map(|tag| tag.name().expect("permission name"))
        .collect())
}

/// Edit applications permissions in the Android Manifest, according to given command
/// line arguments.
fn modify_permissions(sub_args: &ArgMatches) -> DwResult<()> {
    let in_filename = sub_args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("input file needed".to_string()))?;
    let out_filename = sub_args
        .get_one::<String>("output")
        .ok_or_else(|| DwError::BadArguments("output file needed".to_string()))?;
    let clear_signatures = sub_args.get_flag("clear-signature");

    let mut package = PackageOptions::manifest_only().open(in_filename)?;
    let manifest = package.manifest_mut().ok_or_else(|| {
        DwError::BadArguments(format!("{in_filename} is not a valid Android package"))
    })?;

    let to_drop = sub_args
        .get_many::<String>("permissions")
        .map_or_else(Vec::new, Iterator::collect);
    for perm in to_drop {
        log::debug!("dropping permission '{}'...", perm);
        if manifest.remove_uses_permission(perm)? {
            log::debug!("permission '{}' dropped.", perm);
        } else {
            log::debug!("permission '{}' not found.", perm);
        }
    }

    package.save(out_filename, clear_signatures)?;
    log::info!("file '{}' written", out_filename);

    print_diff(in_filename, out_filename, false)
}
