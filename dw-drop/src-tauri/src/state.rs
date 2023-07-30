use droidworks::prelude::*;
use std::sync::RwLock;

#[derive(Debug)]
pub struct DwState {
    pub package: RwLock<Option<Package>>,
    pub dexs_strings: RwLock<Option<Vec<String>>>,
    pub dexs_strings_filter: RwLock<String>,
}

impl DwState {
    pub fn new() -> Self {
        Self {
            package: RwLock::new(None),
            dexs_strings: RwLock::new(None),
            dexs_strings_filter: RwLock::new(String::new()),
        }
    }
}

#[macro_export]
macro_rules! read_state {
    ($state_lock:expr => |$field:ident| $block:block) => {
        match &*$state_lock.read().map_err(|_| Error::Internal(100))? {
            None => Err(Error::NoApk),
            Some($field) => $block,
        }
    };
}

#[macro_export]
macro_rules! write_state {
    ($state_lock:expr => |$field:ident| $block:block) => {
        match &mut *$state_lock.write().map_err(|_| Error::Internal(101))? {
            None => Err(Error::NoApk),
            Some($field) => $block,
        }
    };
}
