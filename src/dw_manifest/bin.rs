use droidworks::prelude::DwResult;
use droidworks::{cli, dw_manifest};

fn main() -> DwResult<()> {
    let args = cli::manifest().get_matches();
    dw_manifest::run(&args)
}
