use crate::errors::{ResourcesError, ResourcesResult};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringPoolIndex(usize);

impl StringPoolIndex {
    pub(crate) const fn new(idx: usize) -> Self {
        Self(idx)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct StringPool {
    pub(crate) sorted: bool,
    pub(crate) utf8: bool,
    pub(crate) strings: Vec<Arc<UtfString>>,
    pub(crate) styles: Vec<Style>,
}

impl StringPool {
    pub(crate) const fn new() -> Self {
        Self {
            sorted: false,
            utf8: false,
            strings: Vec::new(),
            styles: Vec::new(),
        }
    }

    pub(crate) fn get(&self, id: StringPoolIndex) -> ResourcesResult<Arc<UtfString>> {
        self.strings
            .get(id.0)
            .ok_or_else(|| ResourcesError::ResNotFound("UtfString".to_string()))
            .map(Arc::clone)
    }

    pub(crate) fn get_or_push(
        &mut self,
        string: String,
    ) -> ResourcesResult<(StringPoolIndex, Arc<UtfString>)> {
        let str_size = string.chars().count();

        if self.utf8 {
            let u8_string = string.into_bytes();

            // search if already exists in string pool
            if let Some((i, utf)) = self.strings.iter().enumerate().find(|(_, utf)| {
                if let UtfString::Utf8 { raw, .. } = utf.as_ref() {
                    raw == &u8_string
                } else {
                    false
                }
            }) {
                return Ok((StringPoolIndex::new(i), utf.clone()));
            }

            let index = StringPoolIndex::new(self.strings.len());
            let utf_string = Arc::new(UtfString::Utf8 {
                self_ref: index,
                raw: u8_string,
                size: str_size,
            });

            // very small chances for the pool to still be sorted,
            // better consider unsorted than checking if it is sorted
            self.sorted = false;
            self.strings.push(utf_string.clone());

            Ok((index, utf_string))
        } else {
            let u16_string: Vec<u16> = string.as_str().encode_utf16().collect();

            // search if already exists in string pool
            if let Some((i, utf)) = self.strings.iter().enumerate().find(|(_, utf)| {
                if let UtfString::Utf16 { raw, .. } = utf.as_ref() {
                    raw == &u16_string
                } else {
                    false
                }
            }) {
                return Ok((StringPoolIndex::new(i), utf.clone()));
            }

            let index = StringPoolIndex::new(self.strings.len());
            let utf_string = Arc::new(UtfString::Utf16 {
                self_ref: index,
                raw: u16_string,
            });

            // very small chances for the pool to still be sorted,
            // better consider unsorted than checking if it is sorted
            self.sorted = false;
            self.strings.push(utf_string.clone());

            Ok((index, utf_string))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UtfString {
    Utf8 {
        self_ref: StringPoolIndex,
        raw: Vec<u8>,
        size: usize,
    },
    Utf16 {
        self_ref: StringPoolIndex,
        raw: Vec<u16>,
    },
}

impl UtfString {
    pub fn string(&self) -> ResourcesResult<String> {
        match self {
            Self::Utf8 { raw, .. } => {
                let s = String::from_utf8(raw.clone())
                    .map_err(|_| ResourcesError::InvalidUtf8("data".to_string()))?;
                // FIXME check removed because of incorrect string length values in resources
                /*
                 EDIT: s.chars().count() ??
                if s.len() == *size {
                    Ok(s)
                } else {
                    Err(ResourcesError::InvalidUtf8("size".to_string()))
                }
                 */
                Ok(s)
            }
            Self::Utf16 { raw, .. } => String::from_utf16(raw)
                .map_err(|_| ResourcesError::InvalidUtf16("data".to_string())),
        }
    }
}

impl fmt::Display for UtfString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string().map_err(|_| fmt::Error)?)
    }
}

#[derive(Debug)]
pub(crate) struct Style {
    pub(crate) spans: Vec<Span>,
}

#[derive(Debug)]
pub(crate) struct Span {
    pub(crate) name: u32,
    pub(crate) first_char: u32,
    pub(crate) last_char: u32,
}
