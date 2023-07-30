use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;

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
    let mut input = OwnDex::open(input_fname)?;
    for dex in input.borrow_dexs() {
        repo.register_dex(dex, false)?;
    }

    let mut cg = repo.build_callgraph()?;
    log_info_callgraph_stats(&cg);

    log::info!("looking for unknown refs (classes or fields)...");
    cg.mark_unknown_refs(&repo)?;
    log_info_callgraph_stats(&cg);

    cg.patch_unknown_refs(&repo)?;

    if let Some(output_fname) = args.get_one::<String>("output") {
        input.modify_dexs();
        input.save(output_fname)?;
    }

    Ok(())
}

fn log_info_callgraph_stats(cg: &callgraph::CallGraph) {
    log::info!("callgraph contains {} methods with:", cg.nb_methods());
    log::info!("    - {} system methods", cg.nb_system_methods());
    log::info!("    - {} zombie methods", cg.nb_zombie_methods());
}
