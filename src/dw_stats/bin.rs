use droidworks::prelude::DwResult;
use droidworks::{cli, dw_stats};

fn main() -> DwResult<()> {
    let args = cli::stats().get_matches();
    dw_stats::run(&args)
}
