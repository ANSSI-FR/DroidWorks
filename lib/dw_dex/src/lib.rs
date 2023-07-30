//! Android Dex data structures definitions.

mod addr;
mod annotations;
mod hexlify;
mod map;
mod mutf8;
mod parsers;
mod strings;
mod values;
mod writers;

pub mod classes;
pub mod code;
pub mod errors;
pub mod fields;
pub mod instrs;
pub mod methods;
pub mod registers;
pub mod types;

pub use crate::addr::Addr;
pub use crate::parsers::parse_dex as parse;
pub use crate::writers::write_dex as write;

use crate::annotations::*;
use crate::classes::*;
use crate::code::*;
use crate::errors::DexResult;
use crate::fields::*;
use crate::map::MapList;
use crate::methods::*;
use crate::strings::*;
use crate::types::*;
use crate::values::*;
use dw_utils::leb::Uleb128;
use nom::number::Endianness;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::ops;
use std::path::Path;
use std::sync::RwLock;

#[derive(Debug)]
pub(crate) struct HeaderItem {
    pub(crate) version: u32,
    pub(crate) checksum: u32,
    pub(crate) signature: Vec<u8>,
    pub(crate) file_size: usize,
    pub(crate) endianness: Endianness,
    pub(crate) _link_size: usize,
    pub(crate) _link_off: usize,
    pub(crate) map_off: usize,
    pub(crate) string_ids_size: usize,
    pub(crate) string_ids_off: usize,
    pub(crate) type_ids_size: usize,
    pub(crate) type_ids_off: usize,
    pub(crate) proto_ids_size: usize,
    pub(crate) proto_ids_off: usize,
    pub(crate) field_ids_size: usize,
    pub(crate) field_ids_off: usize,
    pub(crate) method_ids_size: usize,
    pub(crate) method_ids_off: usize,
    pub(crate) class_defs_size: usize,
    pub(crate) class_defs_off: usize,
    pub(crate) data_size: usize,
    pub(crate) data_off: usize,
}

impl HeaderItem {
    fn new(version: u32) -> Self {
        Self {
            version,
            checksum: 0,
            signature: [0u8; 20].to_vec(),
            file_size: 0,
            endianness: Endianness::Little,
            _link_size: 0,
            _link_off: 0,
            map_off: 0,
            string_ids_size: 0,
            string_ids_off: 0,
            type_ids_size: 0,
            type_ids_off: 0,
            proto_ids_size: 0,
            proto_ids_off: 0,
            field_ids_size: 0,
            field_ids_off: 0,
            method_ids_size: 0,
            method_ids_off: 0,
            class_defs_size: 0,
            class_defs_off: 0,
            data_size: 0,
            data_off: 0,
        }
    }

    const fn size(&self) -> usize {
        112
    }
}

pub(crate) type Map<T> = BTreeMap<usize, T>;

#[derive(Debug, Clone, Copy)]
enum IndexValue {
    Usize(usize),
    Uleb(Uleb128),
}

#[derive(Debug)]
pub struct Index<T: ?Sized> {
    value: IndexValue,
    marker: PhantomData<T>,
}

impl<T> Clone for Index<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            marker: self.marker,
        }
    }
}

impl<T> Copy for Index<T> {}

impl<T> Index<T> {
    pub(crate) const fn new(idx: usize) -> Self {
        Self {
            value: IndexValue::Usize(idx),
            marker: PhantomData,
        }
    }

    pub(crate) fn new_uleb(idx: Uleb128) -> Self {
        Self {
            value: IndexValue::Uleb(idx),
            marker: PhantomData,
        }
    }

    pub(crate) fn as_usize(&self) -> usize {
        match self.value {
            IndexValue::Usize(v) => v,
            IndexValue::Uleb(u) => u.value() as usize,
        }
    }

    pub(crate) fn as_uleb(&self) -> Uleb128 {
        match self.value {
            IndexValue::Usize(v) => Uleb128::new(v as u32, None),
            IndexValue::Uleb(u) => u,
        }
    }
}

pub trait DexIndex: Sized {
    type T;

    fn get(self, dex: &Dex) -> DexResult<&Self::T>;
}

pub trait DexCollection {
    type Idx;

    fn index(&self) -> Self::Idx;
}

pub trait PrettyPrint {
    fn pp(&self, f: &mut fmt::Formatter, dex: &Dex) -> DexResult<()>;
}

pub struct PrettyPrinter<'a, T>(pub &'a T, pub &'a Dex);

impl<'a, T: PrettyPrint> fmt::Display for PrettyPrinter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.pp(f, self.1).map_err(|_| fmt::Error)
    }
}

/// The top-level Dex data structure.
#[derive(Debug)]
pub struct Dex {
    // meta data
    pub(crate) header_item: HeaderItem,
    pub(crate) map_list: MapList,

    // indexed collections
    pub(crate) string_id_items: Vec<StringIdItem>,
    pub(crate) type_id_items: Vec<TypeIdItem>,
    pub(crate) proto_id_items: Vec<ProtoIdItem>,
    pub(crate) field_id_items: Vec<FieldIdItem>,
    pub(crate) method_id_items: Vec<MethodIdItem>,
    pub(crate) class_def_items: Vec<ClassDefItem>,
    pub(crate) call_site_id_items: Vec<CallSiteIdItem>,
    pub(crate) method_handle_items: Vec<MethodHandleItem>,

    // offset-indexed collections
    pub(crate) type_lists: Map<TypeList>,
    pub(crate) annotation_set_ref_lists: Map<AnnotationSetRefList>,
    pub(crate) annotation_set_items: Map<AnnotationSetItem>,
    pub(crate) class_data_items: Map<ClassDataItem>,
    pub(crate) code_items: Map<RwLock<CodeItem>>,
    pub(crate) string_data_items: Map<StringDataItem>,
    pub(crate) debug_info_items: Map<DebugInfoItem>,
    pub(crate) annotation_items: Map<AnnotationItem>,
    pub(crate) encoded_array_items: Map<EncodedArrayItem>,
    pub(crate) annotations_directory_items: Map<AnnotationsDirectoryItem>,
    pub(crate) hiddenapi_class_data_items: Map<HiddenapiClassDataItem>,
}

impl<Idx: DexIndex> ops::Index<Idx> for Dex {
    type Output = Idx::T;

    fn index(&self, idx: Idx) -> &Self::Output {
        idx.get(self).unwrap()
    }
}

impl Dex {
    fn new(version: u32) -> Self {
        Self {
            header_item: HeaderItem::new(version),
            map_list: MapList::new(),
            string_id_items: Vec::new(),
            type_id_items: Vec::new(),
            proto_id_items: Vec::new(),
            field_id_items: Vec::new(),
            method_id_items: Vec::new(),
            class_def_items: Vec::new(),
            call_site_id_items: Vec::new(),
            method_handle_items: Vec::new(),
            type_lists: Map::new(),
            annotation_set_ref_lists: Map::new(),
            annotation_set_items: Map::new(),
            class_data_items: Map::new(),
            code_items: Map::new(),
            string_data_items: Map::new(),
            debug_info_items: Map::new(),
            annotation_items: Map::new(),
            encoded_array_items: Map::new(),
            annotations_directory_items: Map::new(),
            hiddenapi_class_data_items: Map::new(),
        }
    }

    #[inline]
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.header_item.version
    }

    #[inline]
    pub fn iter_string_ids(&self) -> impl Iterator<Item = &StringIdItem> {
        self.string_id_items.iter()
    }

    #[inline]
    pub fn iter_type_ids(&self) -> impl Iterator<Item = &TypeIdItem> {
        self.type_id_items.iter()
    }

    #[inline]
    pub fn iter_proto_ids(&self) -> impl Iterator<Item = &ProtoIdItem> {
        self.proto_id_items.iter()
    }

    #[inline]
    pub fn iter_field_ids(&self) -> impl Iterator<Item = &FieldIdItem> {
        self.field_id_items.iter()
    }

    #[inline]
    pub fn iter_method_ids(&self) -> impl Iterator<Item = &MethodIdItem> {
        self.method_id_items.iter()
    }

    #[inline]
    pub fn iter_class_defs(&self) -> impl Iterator<Item = &ClassDefItem> {
        self.class_def_items.iter()
    }

    #[inline]
    pub fn iter_call_site_ids(&self) -> impl Iterator<Item = &CallSiteIdItem> {
        self.call_site_id_items.iter()
    }

    #[inline]
    pub fn iter_method_handles(&self) -> impl Iterator<Item = &MethodHandleItem> {
        self.method_handle_items.iter()
    }
}

/// Open and parses the given dex file path.
pub fn open<P: AsRef<Path>>(path: P) -> DexResult<Dex> {
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    parse(&contents)
}

pub struct WithDex<'a, T> {
    pub dex: &'a Dex,
    pub data: T,
}

impl<'a, T> WithDex<'a, T> {
    pub const fn new(dex: &'a Dex, data: T) -> Self {
        WithDex { dex, data }
    }
}
