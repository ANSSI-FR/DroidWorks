use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;
use std::fs::File;
use std::io::Write;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let mut repo = Repo::new();
    let sys = args
        .get_one::<String>("system")
        .map(OwnDex::open)
        .transpose()?;
    if let Some(sys) = &sys {
        for dex in sys.borrow_dexs() {
            repo.register_dex(dex, true)?;
        }
    }
    let input_fname = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let input = OwnDex::open(input_fname)?;
    for dex in input.borrow_dexs() {
        repo.register_dex(dex, false)?;
    }
    repo.close_hierarchy();

    if args.get_flag("filter-class") || args.get_flag("filter-method") {
        todo!("handle filters for subcommand 'hierarchy'");
    }
    if let Some(dot_filename) = args.get_one::<String>("output") {
        let mut file = File::create(dot_filename)?;
        file.write_all(repo.hierarchy().to_dot().as_bytes())?;
        log::info!("dot output written in {:?}", dot_filename);
    }
    Ok(())
}
