use droidworks::prelude::DwResult;
use droidworks::{cli, dw_strip};

fn main() -> DwResult<()> {
    let args = cli::strip().get_matches();
    dw_strip::run(&args)
}
