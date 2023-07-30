use droidworks::prelude::DwResult;
use droidworks::{cli, dw_hierarchy};

fn main() -> DwResult<()> {
    let args = cli::hierarchy().get_matches();
    dw_hierarchy::run(&args)
}
