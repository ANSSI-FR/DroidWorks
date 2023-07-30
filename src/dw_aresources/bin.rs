use droidworks::prelude::DwResult;
use droidworks::{cli, dw_aresources};

fn main() -> DwResult<()> {
    let args = cli::aresources().get_matches();
    dw_aresources::run(&args)
}
