use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;
use regex::Regex;
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

    let filter_class = args.get_one::<String>("filter-class");
    let filter_method = args.get_one::<String>("filter-method");
    let cg = if filter_class.is_none() && filter_method.is_none() {
        repo.build_callgraph()?
    } else {
        let class_pattern = filter_class.map(|r| Regex::new(r)).transpose()?;
        let method_pattern = filter_method.map(|r| Regex::new(r)).transpose()?;
        log::debug!(
            "filtering callgraph on class pattern {:?}, method pattern {:?}",
            class_pattern,
            method_pattern
        );
        repo.build_callgraph()?.filter(|meth| {
            (class_pattern.is_none()
                || class_pattern.as_ref().unwrap().is_match(&meth.class_name()))
                && (method_pattern.is_none()
                    || method_pattern.as_ref().unwrap().is_match(meth.name()))
        })
    };

    log::info!("callgraph contains {} methods with:", cg.nb_methods());
    log::info!("    - {} system methods", cg.nb_system_methods());
    log::info!("    - {} zombie methods", cg.nb_zombie_methods());

    if let Some(dot_filename) = &args.get_one::<String>("output") {
        let mut file = File::create(dot_filename)?;
        file.write_all(cg.to_dot().as_bytes())?;
        log::info!("dot output written in {:?}", dot_filename);
    }

    Ok(())
}
