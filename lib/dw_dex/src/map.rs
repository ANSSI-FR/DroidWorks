use crate::errors::DexError;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug)]
pub(crate) struct MapList {
    pub(crate) list: Vec<MapItem>,
}

impl MapList {
    pub(crate) const fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub(crate) fn size(&self) -> usize {
        4 + 12 * self.list.len()
    }
}

#[derive(Debug)]
pub(crate) struct MapItem {
    pub(crate) typ: MapItemType,
    pub(crate) size: usize,
    pub(crate) offset: usize,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum MapItemType {
    HeaderItem,
    StringIdItem,
    TypeIdItem,
    ProtoIdItem,
    FieldIdItem,
    MethodIdItem,
    ClassDefItem,
    CallSiteIdItem,
    MethodHandleItem,
    MapList,
    TypeList,
    AnnotationSetRefList,
    AnnotationSetItem,
    ClassDataItem,
    CodeItem,
    StringDataItem,
    DebugInfoItem,
    AnnotationItem,
    EncodedArrayItem,
    AnnotationsDirectoryItem,
    HiddenapiClassDataItem,
}

impl fmt::Display for MapItemType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HeaderItem => write!(f, "HEADER_ITEM"),
            Self::StringIdItem => write!(f, "STRING_ID_ITEM"),
            Self::TypeIdItem => write!(f, "TYPE_ID_ITEM"),
            Self::ProtoIdItem => write!(f, "PROTO_ID_ITEM"),
            Self::FieldIdItem => write!(f, "FIELD_ID_ITEM"),
            Self::MethodIdItem => write!(f, "METHOD_ID_ITEM"),
            Self::ClassDefItem => write!(f, "CLASS_DEF_ITEM"),
            Self::CallSiteIdItem => write!(f, "CALL_SITE_ID_ITEM"),
            Self::MethodHandleItem => write!(f, "METHOD_HANDLE_ITEM"),
            Self::MapList => write!(f, "MAP_LIST"),
            Self::TypeList => write!(f, "TYPE_LIST"),
            Self::AnnotationSetRefList => write!(f, "ANNOTATION_SET_REF_LIST"),
            Self::AnnotationSetItem => write!(f, "ANNOTATION_SET_ITEM"),
            Self::ClassDataItem => write!(f, "CLASS_DATA_ITEM"),
            Self::CodeItem => write!(f, "CODE_ITEM"),
            Self::StringDataItem => write!(f, "STRING_DATA_ITEM"),
            Self::DebugInfoItem => write!(f, "DEBUG_INFO_ITEM"),
            Self::AnnotationItem => write!(f, "ANNOTATION_ITEM"),
            Self::EncodedArrayItem => write!(f, "ENCODED_ARRAY_ITEM"),
            Self::AnnotationsDirectoryItem => write!(f, "ANNOTATIONS_DIRECTORY_ITEM"),
            Self::HiddenapiClassDataItem => write!(f, "HIDDENAPI_CLASS_DATA_ITEM"),
        }
    }
}

impl TryFrom<u16> for MapItemType {
    type Error = DexError;

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(Self::HeaderItem),
            0x0001 => Ok(Self::StringIdItem),
            0x0002 => Ok(Self::TypeIdItem),
            0x0003 => Ok(Self::ProtoIdItem),
            0x0004 => Ok(Self::FieldIdItem),
            0x0005 => Ok(Self::MethodIdItem),
            0x0006 => Ok(Self::ClassDefItem),
            0x0007 => Ok(Self::CallSiteIdItem),
            0x0008 => Ok(Self::MethodHandleItem),
            0x1000 => Ok(Self::MapList),
            0x1001 => Ok(Self::TypeList),
            0x1002 => Ok(Self::AnnotationSetRefList),
            0x1003 => Ok(Self::AnnotationSetItem),
            0x2000 => Ok(Self::ClassDataItem),
            0x2001 => Ok(Self::CodeItem),
            0x2002 => Ok(Self::StringDataItem),
            0x2003 => Ok(Self::DebugInfoItem),
            0x2004 => Ok(Self::AnnotationItem),
            0x2005 => Ok(Self::EncodedArrayItem),
            0x2006 => Ok(Self::AnnotationsDirectoryItem),
            0xF000 => Ok(Self::HiddenapiClassDataItem),
            _ => Err(DexError::Structure(format!("unknown map type: '{v}'"))),
        }
    }
}
