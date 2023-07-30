use crate::analysis;
use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let mut nb_success = 0;
    let mut nb_fails = 0;
    let mut last_res = Ok(());

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

    let sys_sdk_version = sys.as_ref().and_then(|sys| {
        sys.package().and_then(|package| {
            package
                .manifest()
                .and_then(|manifest| manifest.version_code().transpose())
        })
    });

    let input_fname = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let input = OwnDex::open(input_fname)?;
    for dex in input.borrow_dexs() {
        repo.register_dex(dex, false)?;
    }
    repo.close_hierarchy();

    let input_compile_sdk_version = input.package().and_then(|package| {
        package
            .manifest()
            .and_then(|manifest| manifest.compile_sdk_version().transpose())
    });

    match (sys_sdk_version, input_compile_sdk_version) {
        (Some(Ok(sysv)), Some(Ok(inputv))) => {
            if sysv != inputv {
                log::warn!("API level is {sysv} while input compileSdkVersion is {inputv}");
            }
        }
        (None, Some(Ok(inputv))) => log::warn!("missing system API level {inputv}"),
        (_, None) | (_, Some(_)) => log::warn!("unknown input API level"),
    }

    let typecheck = if *args.get_one::<bool>("backward").unwrap_or(&false) {
        log::info!("backward typecheck");
        analysis::backward_typecheck
    } else {
        log::info!("forward typecheck");
        analysis::forward_typecheck
    };

    for (class, method) in repo
        .iter_classes_methods()
        .filter(|(class, method)| !class.is_system() && method.code().is_some())
    {
        log::info!("typecheck {}", method.descriptor());

        //let cfg = controlflow::Cfg::build(method, class, method.dex())?;
        match typecheck(method, class, &repo) {
            Ok(res) => {
                log::debug!(
                    "{:#?}",
                    res.entries.get(&Addr::entry()).expect("code entry block")
                );
                nb_success += 1
            }
            Err(err) => {
                log::error!("{}", err);
                /*
                    info!("    ------------------------------");
                    for block in cfg.iter_ordered_blocks() {
                        for instr in block.instructions() {
                            info!(
                                "    {:04}: {}",
                                instr.addr(),
                                PrettyPrinter(instr.instr(), method.dex())
                            );
                        }
                        info!("    ------------------------------");
                }
                    */
                nb_fails += 1;
                last_res = Err(err.into());
            }
        }
    }

    log::info!("");
    log::info!(
        "typechecked methods: {} / {}",
        nb_success,
        nb_success + nb_fails
    );

    last_res
}
