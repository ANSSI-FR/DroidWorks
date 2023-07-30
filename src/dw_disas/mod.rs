use crate::dex::PrettyPrinter;
use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;
use regex::Regex;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

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

    let class_pattern = args
        .get_one::<String>("filter-class")
        .map(|r| Regex::new(r))
        .transpose()?;
    let method_pattern = args
        .get_one::<String>("filter-method")
        .map(|r| Regex::new(r))
        .transpose()?;
    let classes: Box<dyn Iterator<Item = &Class>> = if let Some(r) = &class_pattern {
        Box::new(repo.find_classes(r))
    } else {
        Box::new(repo.iter_classes())
    };
    let classes_methods: Box<dyn Iterator<Item = (&Class, &Method)>> = if let Some(r) =
        &method_pattern
    {
        Box::new(classes.flat_map(|class| {
            class
                .find_methods(r, &repo)
                .map(move |method| (class, method))
        }))
    } else {
        Box::new(
            classes.flat_map(|class| class.iter_methods(&repo).map(move |method| (class, method))),
        )
    };

    for (class, method) in
        classes_methods.filter(|(class, method)| !class.is_system() && method.code().is_some())
    {
        println!("[*] {}", method.descriptor());

        if let Some(cfg_dir) = &args.get_one::<String>("output") {
            let cfg = controlflow::Cfg::build(method)?;
            write_cfg_file(cfg_dir, class.name(), method.name(), &cfg)?;
        } else {
            for instr in method
                .code()
                .expect("code")
                .read()
                .unwrap()
                .iter_instructions()
            {
                println!(
                    "    {:04}: {}",
                    instr.addr(),
                    PrettyPrinter(instr.instr(), method.dex())
                );
            }
        }
    }

    Ok(())
}

fn write_cfg_file<P: AsRef<Path>>(
    base_dir: P,
    class_name: &str,
    method_name: &str,
    cfg: &controlflow::Cfg,
) -> DwResult<()> {
    // prepare directory (base_dir/fully_qualified_class_name)
    let mut dir = base_dir.as_ref().to_path_buf();
    dir.push(class_name);
    create_dir_all(&dir)?;

    // write file
    dir.push(method_name);
    dir.set_extension("dot");
    let mut file = File::create(dir)?;
    file.write_all(cfg.to_dot().as_bytes())?;

    Ok(())
}
