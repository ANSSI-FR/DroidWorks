use droidworks::prelude::DwResult;
use droidworks::{cli, dw_dexdissect};

fn main() -> DwResult<()> {
    let args = cli::dexdissect().get_matches();
    dw_dexdissect::run(&args)
}
