use crate::errors::{ResourcesError, ResourcesResult};
use crate::strings::{StringPoolIndex, UtfString};
use crate::values::Value;
use crate::Xml;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct XmlResourceMap {
    pub(crate) resource_ids: Vec<u32>,
}

#[derive(Debug)]
pub(crate) enum XmlEvent {
    StartNamespace(XmlNamespace),
    EndNamespace(XmlNamespace),
    StartElement(XmlElement, XmlElementAttrs),
    EndElement(XmlElement),
    Cdata(XmlCdata),
}

impl XmlEvent {
    pub(crate) fn start_element(&self) -> ResourcesResult<(&XmlElement, &XmlElementAttrs)> {
        match self {
            Self::StartElement(elt, attrs) => Ok((elt, attrs)),
            _ => Err(ResourcesError::XmlQuery(
                "'start element' event was expected".to_string(),
            )),
        }
    }
}

#[derive(Debug)]
pub(crate) struct XmlNamespace {
    pub(crate) metadata: XmlMetadata,
    pub(crate) prefix: StringPoolIndex,
    pub(crate) uri: StringPoolIndex,
}

#[derive(Clone, Debug)]
pub(crate) struct XmlMetadata {
    pub(crate) line_number: u32,
    pub(crate) comment: u32,
}

#[derive(Clone, Debug)]
pub struct XmlElement {
    pub(crate) metadata: XmlMetadata,
    pub(crate) ns: Option<StringPoolIndex>,
    pub(crate) name: StringPoolIndex,
}

impl XmlElement {
    pub fn name(&self, xml: &Xml) -> ResourcesResult<Arc<UtfString>> {
        xml.string_pool.get(self.name)
    }
}

#[derive(Clone, Debug)]
pub struct XmlElementAttrs {
    pub(crate) id_index: u16,
    pub(crate) class_index: u16,
    pub(crate) style_index: u16,
    pub(crate) attrs: Vec<XmlAttribute>,
}

#[derive(Clone, Debug)]
pub(crate) struct XmlAttribute {
    pub(crate) ns: Option<StringPoolIndex>,
    pub(crate) name: StringPoolIndex,
    pub(crate) raw_value: u32,
    pub(crate) typed_value: Value,
}

impl XmlAttribute {
    pub fn name(&self, xml: &Xml) -> ResourcesResult<Arc<UtfString>> {
        xml.string_pool.get(self.name)
    }
}

#[derive(Debug)]
pub(crate) struct XmlCdata {
    pub(crate) metadata: XmlMetadata,
    pub(crate) data: StringPoolIndex,
    pub(crate) value: Value,
}
