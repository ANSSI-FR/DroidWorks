use droidworks::prelude::DwResult;
use droidworks::{cli, dw_typecheck};

fn main() -> DwResult<()> {
    let args = cli::typecheck().get_matches();
    dw_typecheck::run(&args)
}
