use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let mut repo = Repo::new();
    let input_fname = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let input = OwnDex::open(input_fname)?;
    for dex in input.borrow_dexs() {
        repo.register_dex(dex, false)?;
    }
    repo.close_hierarchy();

    if args.get_flag("stubs") {
        let mut n = 0;
        for class in repo.iter_classes() {
            if class.has_stub_code(&repo)? {
                n += 1;
            }
        }
        println!("{n}");
        return Ok(());
    }

    let names: Vec<&str> = if args.get_flag("missing") {
        repo.iter_missing_classes().collect()
    } else {
        repo.iter_classes()
            .filter_map(|c| if c.is_system() { None } else { Some(c.name()) })
            .collect()
    };

    if args.get_flag("count") {
        println!("{}", names.len());
    } else {
        for name in names {
            println!("{name}");
        }
    }

    Ok(())
}
