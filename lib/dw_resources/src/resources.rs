//! The Android resources data structures and accessors.

use crate::errors::ResourcesResult;
use crate::parsers::parse_resources;
use crate::strings::StringPool;
use crate::tables::{Config, TablePackage, TablePackagePool, TableTypeEntry};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct ResourcesTable {
    pub(crate) string_pool: StringPool,
    pub(crate) package_pool: TablePackagePool,
}

#[derive(Debug)]
pub struct Resources(pub(crate) ResourcesTable);

pub fn parse(input: &[u8]) -> ResourcesResult<Resources> {
    parse_resources(input)
}

pub fn write(_resources: &Resources) -> ResourcesResult<Vec<u8>> {
    todo!("write_resource(&resources.res)")
}

impl Resources {
    #[must_use]
    pub fn available_configs(&self) -> BTreeSet<Config> {
        self.0
            .package_pool
            .packages()
            .iter()
            .fold(BTreeSet::new(), |res, package| {
                package.type_pool.types().iter().fold(res, |mut res, typ| {
                    let _ = res.insert(typ.config.clone());
                    res
                })
            })
    }

    pub(crate) fn package_lookup(&self, reference: u32) -> Option<Arc<TablePackage>> {
        let package_id = (reference >> 24) as u8;
        self.0.package_pool.resolve(package_id)
    }

    pub(crate) fn lookup(&self, reference: u32) -> Option<BTreeMap<Config, Arc<TableTypeEntry>>> {
        let type_id = ((reference >> 16) & 0xff) as u8;
        let entry_id = (reference & 0xffff) as u16;
        self.package_lookup(reference)
            .and_then(|package| package.type_pool.resolve(type_id))
            .map(|types| {
                types.into_iter().fold(BTreeMap::new(), |mut res, typ| {
                    if let Some(entry) = typ.entry_pool.resolve(entry_id) {
                        if res.insert(typ.config.clone(), entry).is_some() {
                            log::warn!(
                                "several entries for resource {:#x} with same configuration",
                                reference
                            );
                        }
                    }
                    res
                })
            })
    }
}

impl fmt::Display for Resources {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for package in self.0.package_pool.packages() {
            package.pretty_print(f, self).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}
