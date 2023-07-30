use crate::errors::{DexError, DexResult};
use crate::mutf8;
use crate::{Dex, DexCollection, DexIndex, Index, PrettyPrint};
use dw_utils::leb::Uleb128;
use std::fmt;

#[derive(Debug)]
pub struct StringIdItem {
    pub(crate) index: Index<StringIdItem>,
    pub(crate) string_data_off: Index<StringDataItem>,
}

impl DexIndex for Index<StringIdItem> {
    type T = StringIdItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.string_id_items
            .get(self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("StringIdItem".to_string()))
    }
}

impl DexCollection for StringIdItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl StringIdItem {
    pub fn to_string(&self, dex: &Dex) -> DexResult<String> {
        let string_data = self.string_data_off.get(dex)?;
        string_data.to_string()
    }

    pub(crate) fn size(&self) -> usize {
        4
    }
}

impl PrettyPrint for StringIdItem {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()> {
        let string = self.to_string(dex)?;
        write!(f, "{string}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct StringDataItem {
    pub(crate) index: Index<StringDataItem>,
    pub(crate) utf16_size: Uleb128,
    pub(crate) data: Vec<u8>,
}

impl DexIndex for Index<StringDataItem> {
    type T = StringDataItem;

    fn get(self, dex: &Dex) -> DexResult<&Self::T> {
        dex.string_data_items
            .get(&self.as_usize())
            .ok_or_else(|| DexError::ResNotFound("StringDataItem".to_string()))
    }
}

impl DexCollection for StringDataItem {
    type Idx = Index<Self>;

    fn index(&self) -> Self::Idx {
        self.index
    }
}

impl StringDataItem {
    fn to_string(&self) -> DexResult<String> {
        let v = mutf8::decode(&self.data)?;
        if v.len() == self.utf16_size.value() as usize {
            let s = match String::from_utf16(&v) {
                Ok(s) => s,
                Err(err) => {
                    log::debug!("{}", err);
                    log::warn!(
                        "Isolated or out-of-order utf16 surrogate code unit, \"
                        using Rust lossy conversion."
                    );
                    String::from_utf16_lossy(&v)
                }
            };
            Ok(s)
        } else {
            log::error!("raw string:    {:?}", self.data);
            log::error!("raw string length: {}", self.data.len());
            log::error!("expected length:   {}", self.utf16_size.value());
            log::error!("utf16 length:      {}", v.len());
            Err(DexError::BadSize("mutf-8".to_string()))
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.utf16_size.size() + self.data.len() + 1
    }
}
