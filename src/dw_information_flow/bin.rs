use droidworks::prelude::DwResult;
use droidworks::{cli, dw_information_flow};

fn main() -> DwResult<()> {
    let args = cli::information_flow().get_matches();
    dw_information_flow::run(&args)
}
