use droidworks::prelude::DwResult;
use droidworks::{cli, dw_callgraph};

fn main() -> DwResult<()> {
    let args = cli::callgraph().get_matches();
    dw_callgraph::run(&args)
}
