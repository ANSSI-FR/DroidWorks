use std::collections::BTreeMap;

use crate::prelude::*;
use crate::{analysis, owndex};
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let mut repo = Repo::new();
    let sys = args
        .get_one::<String>("system")
        .map(owndex::OwnDex::open)
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
    let input = owndex::OwnDex::open(input_fname)?;
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

    let callgraph = analysis::callgraph::CallGraph::build(&repo, true)?;
    let mut typecheck = BTreeMap::new();
    let mut signatures = BTreeMap::new();

    let mut has_changed = true;
    let mut nb_success = 0;
    let mut nb_fails = 0;
    let mut last_res = Ok(());

    loop {
        if !has_changed {
            break;
        }

        has_changed = false;
        nb_success = 0;
        nb_fails = 0;
        last_res = Ok(());

        for method in callgraph.traverse_from_callees_to_callers() {
            let Some(class) = repo.get_class_by_name(&method.class_name()) else { continue };
            /*
                if class.is_system() {
                continue;
            }
                 */

            if method.is_zombie() {
                continue;
            }

            let Some(method) = method.definition() else { continue };
            if method.code().is_none() {
                continue;
            }

            if let std::collections::btree_map::Entry::Vacant(e) = typecheck.entry(method.uid()) {
                log::info!("forward typecheck {}", method.descriptor());
                let forward = analysis::forward_typecheck(method, class, &repo)?;
                e.insert(forward);
            }

            log::info!("information flow {}", method.descriptor());
            let context = analysis::information_flow::StateContext::try_from((
                method,
                &repo,
                &typecheck[&method.uid()],
                &signatures,
            ))?;

            match analysis::information_flow(method, class, &context) {
                Ok(mut signature) => {
                    nb_success += 1;

                    signature.prune();

                    log::info!(
                        "information flow signature\n{}",
                        signature
                            .pretty_print(&repo)
                            .map_err(analysis::errors::AnalysisError::from)?
                    );

                    match signatures.get_mut(&method.uid()) {
                        None => {
                            has_changed = true;
                            signatures.insert(method.uid(), signature);
                        }
                        Some(prev_sig) => {
                            if !has_changed {
                                has_changed = prev_sig != &mut signature;
                            }
                            *prev_sig = signature;
                        }
                    }
                }
                Err(err) => {
                    log::error!("{err}");
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
    }

    log::info!("");
    log::info!(
        "information flowed methods: {} / {}",
        nb_success,
        nb_success + nb_fails
    );

    last_res
}
