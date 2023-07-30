use droidworks::prelude::DwResult;
use droidworks::{cli, dw_packageinfo};

fn main() -> DwResult<()> {
    let args = cli::packageinfo().get_matches();
    dw_packageinfo::run(&args)
}
