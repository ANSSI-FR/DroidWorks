//! Main `DroidWorks` binary command line arguments options.
//!
//! This module declares a function to build `clap` command line arguments
//! parser, so that it can be used from other places than the main binary,
//! such as from bash completion file generator.

use clap::{value_parser, Arg, ArgAction, Command};
use clap_complete::Shell;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn arg_debug() -> Arg {
    Arg::new("debug")
        .short('d')
        .long("debug")
        .action(ArgAction::SetTrue)
        .help("Activate debug mode")
}

fn arg_verbose() -> Arg {
    Arg::new("verbose")
        .short('v')
        .long("verbose")
        .action(ArgAction::SetTrue)
        .help("Activate verbose mode")
}

fn arg_ecslog() -> Arg {
    Arg::new("ecslog")
        .short('e')
        .long("ecslog")
        .action(ArgAction::SetTrue)
        .help("Output logs in ECS format")
}

fn arg_input() -> Arg {
    Arg::new("input")
        .short('i')
        .long("input")
        .action(ArgAction::Set)
        .required(true)
        .help("Input file")
}

fn arg_system() -> Arg {
    Arg::new("system")
        .short('s')
        .long("system")
        .action(ArgAction::Set)
        .help("Additional system/core/api input file")
}

fn arg_output(help: &str) -> Arg {
    Arg::new("output")
        .short('o')
        .long("output")
        .action(ArgAction::Set)
        .help(help.to_string())
}

fn arg_filter_class() -> Arg {
    Arg::new("filter-class")
        .long("filter-class")
        .action(ArgAction::Set)
        .help("Class(es) regex filter")
}

fn arg_filter_method() -> Arg {
    Arg::new("filter-method")
        .long("filter-method")
        .action(ArgAction::Set)
        .help("Method(s) regex filter")
}

#[must_use]
pub fn droidworks() -> Command {
    Command::new(NAME)
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .subcommand(aresources())
        .subcommand(callgraph())
        .subcommand(dexdissect())
        .subcommand(disas())
        .subcommand(hierarchy())
        .subcommand(manifest())
        .subcommand(nsc())
        .subcommand(packageinfo())
        .subcommand(permissions())
        .subcommand(stats())
        .subcommand(strip())
        .subcommand(typecheck())
        .subcommand(
            Command::new("gen-completions")
                .about("Generates completions file")
                .arg(
                    Arg::new("shell")
                        .short('s')
                        .long("shell")
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(Shell))
                        .required(true)
                        .help("Shell type for completion generation"),
                ),
        )
}

#[must_use]
pub fn aresources() -> Command {
    Command::new("aresources")
        .bin_name("dw-aresources")
        .version(VERSION)
        .author(AUTHORS)
        .about("Prints apk resources in aapt form")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
}

#[must_use]
pub fn callgraph() -> Command {
    Command::new("callgraph")
        .bin_name("dw-callgraph")
        .version(VERSION)
        .author(AUTHORS)
        .about("Generates dex callgraph")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(arg_system())
        .arg(arg_output("Output dot file"))
        .arg(arg_filter_class())
        .arg(arg_filter_method())
}

#[must_use]
pub fn dexdissect() -> Command {
    Command::new("dexdissect")
        .bin_name("dw-dexdissect")
        .version(VERSION)
        .author(AUTHORS)
        .about("Dumps dex tables")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(
            Arg::new("table")
                .short('t')
                .long("table")
                .action(ArgAction::Set)
                .value_parser(["strings", "types", "protos", "fields", "methods"])
                .required(true),
        )
}

#[must_use]
pub fn disas() -> Command {
    Command::new("disas")
        .bin_name("dw-disas")
        .version(VERSION)
        .author(AUTHORS)
        .about("Disassembles dalvik bytecode")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(arg_output("Dot output directory"))
        .arg(arg_filter_class())
        .arg(arg_filter_method())
}

#[must_use]
pub fn hierarchy() -> Command {
    Command::new("hierarchy")
        .bin_name("dw-hierarchy")
        .version(VERSION)
        .author(AUTHORS)
        .about("Generates classes hierarchy graph")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(arg_system())
        .arg(arg_output("Output dot file"))
        .arg(arg_filter_class())
        .arg(arg_filter_method())
}

#[must_use]
pub fn manifest() -> Command {
    Command::new("manifest")
        .bin_name("dw-manifest")
        .version(VERSION)
        .author(AUTHORS)
        .about("Prints apk manifest in classic xml form")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
}

#[must_use]
pub fn nsc() -> Command {
    Command::new("nsc")
        .bin_name("dw-nsc")
        .version(VERSION)
        .author(AUTHORS)
        .about("Prints network security config in classix xml form, if there is")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
}

#[must_use]
pub fn packageinfo() -> Command {
    Command::new("packageinfo")
        .bin_name("dw-packageinfo")
        .version(VERSION)
        .author(AUTHORS)
        .about("Prints various information about the given file")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
}

#[must_use]
pub fn permissions() -> Command {
    Command::new("permissions")
        .bin_name("dw-permissions")
        .version(VERSION)
        .author(AUTHORS)
        .about("Allow manipulation of the permissions list of an app")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .subcommand(
            Command::new("list")
                .about("Lists application permissions")
                .arg(arg_input())
        )
        .subcommand(
            Command::new("diff")
                .about("Prints applications permissions diff")
                .arg(arg_input())
                .arg(arg_output("Output file"))
                .arg(
                    Arg::new("diff-only")
                        .long("diff-only")
                        .action(ArgAction::SetTrue)
                        .help("Shows only differences"),
                ),
        )
        .subcommand(
            Command::new("modify")
                .about("Modifies required application permissions")
                .after_help("Example:\n $ dw-perms modify myapp.apk mycleanapp.apk -r android.permission.ACCESS_FINE_LOCATION")
                .arg(
                    Arg::new("clear-signature")
                        .short('c')
                        .long("clear-signature")
                        .action(ArgAction::SetTrue)
                        .help("Remove existing signature(s)"),
                )
                .arg(arg_input())
                .arg(arg_output("Output file"))
                .arg(
                    Arg::new("permissions")
                        .short('r')
                        .long("drop")
                        .action(ArgAction::Append)
                        .help("Permissions to drop"),
                ),
        )
}

#[must_use]
pub fn stats() -> Command {
    Command::new("stats")
        .bin_name("dw-stats")
        .version(VERSION)
        .author(AUTHORS)
        .about("Prints stats about defined and undefined bytecode")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .action(ArgAction::SetTrue)
                .help("Print only classes count"),
        )
        .arg(
            Arg::new("missing")
                .short('m')
                .long("missing")
                .action(ArgAction::SetTrue)
                .help("Print missing class names"),
        )
        .arg(
            Arg::new("stubs")
                .short('s')
                .long("stubs")
                .action(ArgAction::SetTrue)
                .conflicts_with("missing")
                .help("Consider only stubs classes"),
        )
}

#[must_use]
pub fn strip() -> Command {
    Command::new("strip")
        .bin_name("dw-strip")
        .version(VERSION)
        .author(AUTHORS)
        .about("Strip unknown methods from app or dex")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(arg_output("Output app or dex file"))
        .arg(arg_system())
}

#[must_use]
pub fn typecheck() -> Command {
    Command::new("typecheck")
        .bin_name("dw-typecheck")
        .version(VERSION)
        .author(AUTHORS)
        .about("Runs a typechecking pass onto dalvik bytecode")
        .arg(arg_debug())
        .arg(arg_verbose())
        .arg(arg_ecslog())
        .arg(arg_input())
        .arg(arg_system())
        .arg(
            Arg::new("backward")
                .long("backward")
                .action(ArgAction::SetTrue)
                .help("Run a backward analysis (instead of forward analysis)"),
        )
}
