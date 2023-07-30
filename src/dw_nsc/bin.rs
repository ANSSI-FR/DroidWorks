use droidworks::prelude::DwResult;
use droidworks::{cli, dw_nsc};

fn main() -> DwResult<()> {
    let args = cli::nsc().get_matches();
    dw_nsc::run(&args)
}
