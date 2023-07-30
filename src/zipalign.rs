//! A rust implementation of the zipalign tool from Android tools suite.
//!
//! This utility performs zip archive alignment and ensures that all uncompressed
//! files in the archive are aligned relative to the start of the file.
//!
//! # Examples
//!
//! To align infile.apk and ssave it as outfile.apk:
//!
//! ```bash
//! zipalign -p -f -v 4 infile.apk outfile.apk
//! ```
//!
//! To confirm the alignment of existing.apk:
//!
//! ```bash
//! zipalign -c -v 4 existing.apk
//! ```

#![forbid(unsafe_code)]

use clap::{value_parser, Arg, ArgAction, Command};
use droidworks::prelude::*;
use droidworks::utils::zipalign;

/// `zipalign` tool entry point.
fn main() -> DwResult<()> {
    let args = Command::new("zipalign")
        .bin_name("zipalign")
        .version("0.1")
        .arg(
            Arg::new("check")
                .short('c')
                .action(ArgAction::SetTrue)
                .help("Check alignment only (does not modify file)"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .action(ArgAction::SetTrue)
                .help("Overwrite existing outfile.zip"),
        )
        .arg(
            Arg::new("page_align_shared_libs")
                .short('p')
                .action(ArgAction::SetTrue)
                .help("Page-align uncompressed .so files"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .action(ArgAction::SetTrue)
                .help("Verbose output"),
        )
        .arg(
            Arg::new("zopfli")
                .short('z')
                .action(ArgAction::SetTrue)
                .help("Recompress using Zopfli"),
        )
        .arg(
            Arg::new("ALIGNMENT")
                .value_name("align")
                .value_parser(value_parser!(u64))
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("INFILE")
                .value_name("infile.zip")
                .index(2)
                .required(true),
        )
        .arg(
            Arg::new("OUTFILE")
                .value_name("outfile.zip")
                .index(3)
                .required(false),
        )
        .get_matches();

    let check = args.get_flag("check");
    let force = args.get_flag("force");
    let page_align_shared_libs = args.get_flag("page_align_shared_libs");
    let verbose = args.get_flag("verbose");
    let zopfli = args.get_flag("zopfli");

    let alignment = *args
        .get_one::<u64>("ALIGNMENT")
        .ok_or_else(|| DwError::BadArguments("alignment not found".to_string()))?;

    let infile = args
        .get_one::<String>("INFILE")
        .ok_or_else(|| DwError::BadArguments("infile not found".to_string()))?;

    if !check {
        let outfile = args
            .get_one::<String>("OUTFILE")
            .ok_or_else(|| DwError::BadArguments("outfile not found".to_string()))?;
        zipalign::process(
            infile,
            outfile,
            alignment,
            force,
            zopfli,
            page_align_shared_libs,
        )?;
    }

    zipalign::verify(infile, alignment, verbose, page_align_shared_libs)?;
    Ok(())
}
