use crate::errors::ResourcesError;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug)]
pub(crate) struct ChunkHeader {
    pub(crate) typ: ChunkType,
    pub(crate) header_size: usize,
    pub(crate) chunk_size: usize,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum ChunkType {
    Null,
    StringPool,
    Table,
    Xml,
    XmlStartNamespace,
    XmlEndNamespace,
    XmlStartElement,
    XmlEndElement,
    XmlCdata,
    XmlResourceMap,
    TablePackage,
    TableType,
    TableTypeSpec,
    TableLibrary,
    TableOverlayable,
    TableOverlayablePolicy,
    TableStagedAlias,
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Null => write!(f, "RES_NULL_TYPE"),
            Self::StringPool => write!(f, "RES_STRING_POOL_TYPE"),
            Self::Table => write!(f, "RES_TABLE_TYPE"),
            Self::Xml => write!(f, "RES_XML_TYPE"),
            Self::XmlStartNamespace => write!(f, "RES_XML_START_NAMESPACE_TYPE"),
            Self::XmlEndNamespace => write!(f, "RES_XML_END_NAMESPACE_TYPE"),
            Self::XmlStartElement => write!(f, "RES_XML_START_ELEMENT_TYPE"),
            Self::XmlEndElement => write!(f, "RES_XML_END_ELEMENT_TYPE"),
            Self::XmlCdata => write!(f, "RES_XML_CDATA_TYPE"),
            Self::XmlResourceMap => write!(f, "RES_XML_RESOURCE_MAP_TYPE"),
            Self::TablePackage => write!(f, "RES_TABLE_PACKAGE_TYPE"),
            Self::TableType => write!(f, "RES_TABLE_TYPE_TYPE"),
            Self::TableTypeSpec => write!(f, "RES_TABLE_TYPE_SPEC_TYPE"),
            Self::TableLibrary => write!(f, "RES_TABLE_LIBRARY_TYPE"),
            Self::TableOverlayable => write!(f, "RES_TABLE_OVERLAYABLE"),
            Self::TableOverlayablePolicy => write!(f, "RES_TABLE_OVERLAYABLE_POLICY"),
            Self::TableStagedAlias => write!(f, "RES_TABLE_STAGED_ALIAS"),
        }
    }
}

impl TryFrom<u16> for ChunkType {
    type Error = ResourcesError;

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(Self::Null),
            0x0001 => Ok(Self::StringPool),
            0x0002 => Ok(Self::Table),
            0x0003 => Ok(Self::Xml),
            0x0100 => Ok(Self::XmlStartNamespace),
            0x0101 => Ok(Self::XmlEndNamespace),
            0x0102 => Ok(Self::XmlStartElement),
            0x0103 => Ok(Self::XmlEndElement),
            0x0104 => Ok(Self::XmlCdata),
            0x0180 => Ok(Self::XmlResourceMap),
            0x0200 => Ok(Self::TablePackage),
            0x0201 => Ok(Self::TableType),
            0x0202 => Ok(Self::TableTypeSpec),
            0x0203 => Ok(Self::TableLibrary),
            0x0204 => Ok(Self::TableOverlayable),
            0x0205 => Ok(Self::TableOverlayablePolicy),
            0x0206 => Ok(Self::TableStagedAlias),
            _ => Err(ResourcesError::Structure(format!(
                "unknown chunk type: '{v:#x}'"
            ))),
        }
    }
}

impl From<ChunkType> for u16 {
    fn from(chunk_type: ChunkType) -> Self {
        match chunk_type {
            ChunkType::Null => 0x0000,
            ChunkType::StringPool => 0x0001,
            ChunkType::Table => 0x0002,
            ChunkType::Xml => 0x0003,
            ChunkType::XmlStartNamespace => 0x0100,
            ChunkType::XmlEndNamespace => 0x0101,
            ChunkType::XmlStartElement => 0x0102,
            ChunkType::XmlEndElement => 0x0103,
            ChunkType::XmlCdata => 0x0104,
            ChunkType::XmlResourceMap => 0x0180,
            ChunkType::TablePackage => 0x0200,
            ChunkType::TableType => 0x0201,
            ChunkType::TableTypeSpec => 0x0202,
            ChunkType::TableLibrary => 0x0203,
            ChunkType::TableOverlayable => 0x0204,
            ChunkType::TableOverlayablePolicy => 0x0205,
            ChunkType::TableStagedAlias => 0x0206,
        }
    }
}
