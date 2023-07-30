use droidworks::prelude::DwResult;
use droidworks::{cli, dw_disas};

fn main() -> DwResult<()> {
    let args = cli::disas().get_matches();
    dw_disas::run(&args)
}
