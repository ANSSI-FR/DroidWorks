use clap::ArgMatches;
use clap_complete::{generate, Shell};
use droidworks::prelude::*;
use droidworks::{
    cli, dw_aresources, dw_callgraph, dw_dexdissect, dw_disas, dw_hierarchy, dw_manifest, dw_nsc,
    dw_packageinfo, dw_permissions, dw_stats, dw_strip, dw_typecheck,
};
use std::io;

fn main() -> DwResult<()> {
    let args = cli::droidworks().get_matches();

    match &args.subcommand() {
        Some(("aresources", cmd_args)) => dw_aresources::run(cmd_args),
        Some(("callgraph", cmd_args)) => dw_callgraph::run(cmd_args),
        Some(("dexdissect", cmd_args)) => dw_dexdissect::run(cmd_args),
        Some(("disas", cmd_args)) => dw_disas::run(cmd_args),
        Some(("hierarchy", cmd_args)) => dw_hierarchy::run(cmd_args),
        Some(("manifest", cmd_args)) => dw_manifest::run(cmd_args),
        Some(("nsc", cmd_args)) => dw_nsc::run(cmd_args),
        Some(("packageinfo", cmd_args)) => dw_packageinfo::run(cmd_args),
        Some(("permissions", cmd_args)) => dw_permissions::run(cmd_args),
        Some(("stats", cmd_args)) => dw_stats::run(cmd_args),
        Some(("strip", cmd_args)) => dw_strip::run(cmd_args),
        Some(("typecheck", cmd_args)) => dw_typecheck::run(cmd_args),
        Some(("gen-completions", sub_args)) => subcommand_gen_completions(sub_args),
        Some((subcommand, _)) => Err(DwError::BadArguments(format!(
            "unknown subcommand '{subcommand}'"
        ))),
        None => Err(DwError::BadArguments("missing subcommand".to_string())),
    }
}

fn subcommand_gen_completions(sub_args: &ArgMatches) -> DwResult<()> {
    let generator = *sub_args
        .get_one::<Shell>("shell")
        .ok_or_else(|| DwError::BadArguments("--shell needed".to_string()))?;
    let mut cmd = cli::droidworks();
    let cmd_name = cmd.get_name().to_string();
    generate(generator, &mut cmd, cmd_name, &mut io::stdout());
    Ok(())
}
