use crate::errors::{ResourcesError, ResourcesResult};
use crate::parsers::parse_xml;
use crate::writers::write_xml;
use crate::Xml;
use std::fmt;

#[derive(Debug)]
pub struct NetworkSecurityConfig {
    xml: Xml,
}

pub fn parse(input: &[u8]) -> ResourcesResult<NetworkSecurityConfig> {
    let xml = parse_xml(input)?;
    Ok(NetworkSecurityConfig { xml })
}

pub fn write(nsc: &NetworkSecurityConfig) -> ResourcesResult<Vec<u8>> {
    write_xml(&nsc.xml)
}

impl fmt::Display for NetworkSecurityConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.xml)
    }
}

pub fn system_store_without_clear_traffic() -> ResourcesResult<NetworkSecurityConfig> {
    let input = include_bytes!("../data/nsc_system_store_without_clear_traffic.axml");
    parse(input)
}

pub fn system_and_user_stores_without_clear_traffic() -> ResourcesResult<NetworkSecurityConfig> {
    let input = include_bytes!("../data/nsc_system_and_user_stores_without_clear_traffic.axml");
    parse(input)
}

pub fn system_store_with_clear_traffic() -> ResourcesResult<NetworkSecurityConfig> {
    let input = include_bytes!("../data/nsc_system_store_with_clear_traffic.axml");
    parse(input)
}

pub fn system_and_user_stores_with_clear_traffic() -> ResourcesResult<NetworkSecurityConfig> {
    let input = include_bytes!("../data/nsc_system_and_user_stores_with_clear_traffic.axml");
    parse(input)
}

pub const AVAILABLE_CUSTOM_NSCS: [&str; 4] = [
    "system_store_without_clear_traffic",
    "system_and_user_stores_without_clear_traffic",
    "system_store_with_clear_traffic",
    "system_and_user_stores_with_clear_traffic",
];

pub fn network_security_config(name: &str) -> ResourcesResult<NetworkSecurityConfig> {
    match name {
        "system_store_without_clear_traffic" => system_store_without_clear_traffic(),
        "system_and_user_stores_without_clear_traffic" => {
            system_and_user_stores_without_clear_traffic()
        }
        "system_store_with_clear_traffic" => system_store_with_clear_traffic(),
        "system_and_user_stores_with_clear_traffic" => system_and_user_stores_with_clear_traffic(),
        _ => Err(ResourcesError::ResNotFound(name.to_string())),
    }
}
