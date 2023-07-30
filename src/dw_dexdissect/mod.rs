use crate::dex::PrettyPrinter;
use crate::owndex::OwnDex;
use crate::prelude::*;
use clap::ArgMatches;

pub fn run(args: &ArgMatches) -> DwResult<()> {
    init_logger(args);

    let input_fname = args
        .get_one::<String>("input")
        .ok_or_else(|| DwError::BadArguments("--input needed".to_string()))?;
    let input = OwnDex::open(input_fname)?;

    let table = args
        .get_one::<String>("table")
        .ok_or_else(|| DwError::BadArguments("--table needed".to_string()))?;
    for dex in input.borrow_dexs() {
        match &**table {
            "strings" => {
                for (i, string) in dex.iter_string_ids().enumerate() {
                    println!("String[{:6}] = {}", i, PrettyPrinter(string, dex));
                }
            }
            "types" => {
                for (i, type_) in dex.iter_type_ids().enumerate() {
                    println!("Type[{:6}] = {}", i, PrettyPrinter(type_, dex));
                }
            }
            "protos" => {
                for (i, proto) in dex.iter_proto_ids().enumerate() {
                    println!("Proto[{:6}] = {}", i, PrettyPrinter(proto, dex));
                }
            }
            "fields" => {
                for (i, field) in dex.iter_field_ids().enumerate() {
                    println!("Field[{:6}] = {}", i, PrettyPrinter(field, dex));
                }
            }
            "methods" => {
                for (i, method) in dex.iter_method_ids().enumerate() {
                    println!("Method[{:6}] = {}", i, PrettyPrinter(method, dex));
                }
            }
            _ => {
                return Err(DwError::BadArguments(
                    "subcommand 'dex-dissect' need --table arg".to_string(),
                ))
            }
        }
    }

    Ok(())
}
