#![allow(dead_code)]

mod chunk;
mod parsers;
mod strings;
mod tables;
mod utils;
mod writers;
mod xml;
mod xpath;

pub mod errors;
pub mod manifest;
pub mod nsc;
pub mod resources;
pub mod values;

use crate::errors::ResourcesResult;
use crate::parsers::parse_xml;
use crate::strings::StringPool;
use crate::xml::{XmlEvent, XmlResourceMap};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct Xml {
    pub(crate) string_pool: StringPool,
    pub(crate) xml_resource_map: Option<XmlResourceMap>,
    pub(crate) xml_body: Vec<XmlEvent>,
}

impl fmt::Display for Xml {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"no\"?>"
        )?;
        let mut namespaces_stack = Vec::new();
        let mut namespaces_map = BTreeMap::new();
        let mut i_event = 0;
        while i_event < self.xml_body.len() {
            match &self.xml_body[i_event] {
                XmlEvent::StartNamespace(ns) => {
                    namespaces_stack.push(<&xml::XmlNamespace>::clone(&ns));
                    namespaces_map.insert(ns.uri, ns.prefix);
                }
                XmlEvent::EndNamespace(_ns) => {
                    // TODO remove from namespaces (check if it's the top of the stack)
                    // (stack AND map)
                }
                XmlEvent::StartElement(elt, attrs) => {
                    write!(f, "<")?;
                    if let Some(uri) = &elt.ns {
                        let prefix = namespaces_map.get(uri).unwrap();
                        write!(
                            f,
                            "{}:",
                            self.string_pool.get(*prefix).map_err(|_| fmt::Error)?
                        )?;
                    }
                    write!(f, "{}", elt.name(self).map_err(|_| fmt::Error)?)?;

                    for attr in &attrs.attrs {
                        write!(f, " ")?;
                        if let Some(uri) = &attr.ns {
                            let prefix = namespaces_map.get(uri).unwrap();
                            write!(
                                f,
                                "{}:",
                                self.string_pool.get(*prefix).map_err(|_| fmt::Error)?
                            )?;
                        }
                        write!(f, "{}=", attr.name(self).map_err(|_| fmt::Error)?)?;
                        attr.typed_value
                            .pretty_print_from_xml(f, self)
                            .map_err(|_| fmt::Error)?;
                    }

                    if i_event < self.xml_body.len() - 1 {
                        if let XmlEvent::EndElement(_) = self.xml_body[i_event + 1] {
                            // TODO check that EndElement match the current StartElement
                            write!(f, "/>")?;
                            i_event += 1;
                        } else {
                            write!(f, ">")?;
                        }
                    } else {
                        // TODO error! last event cannot be a StartElement
                    }
                }
                XmlEvent::EndElement(elt) => {
                    if let Some(uri) = &elt.ns {
                        let prefix = namespaces_map.get(uri).unwrap();
                        write!(
                            f,
                            "{}:",
                            self.string_pool.get(*prefix).map_err(|_| fmt::Error)?
                        )?;
                    }
                    write!(f, "</{}>", elt.name(self).map_err(|_| fmt::Error)?)?;
                }
                XmlEvent::Cdata(data) => {
                    write!(f, "{}", self.string_pool.get(data.data).unwrap())?;
                }
            }
            i_event += 1;
        }
        Ok(())
    }
}

impl Default for Xml {
    fn default() -> Self {
        Self {
            string_pool: StringPool::new(),
            xml_resource_map: None,
            xml_body: Vec::new(),
        }
    }
}

pub fn open_xml<P: AsRef<Path>>(path: P) -> ResourcesResult<Xml> {
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    parse_xml(&contents)
}
