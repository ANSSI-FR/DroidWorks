use crate::errors::{ResourcesError, ResourcesResult};
use crate::resources::Resources;
use crate::values::ResolvedValue;
use crate::xml::XmlAttribute;
use crate::Xml;

pub enum ResOption<T> {
    None,
    TooComplex,
    Some(T),
}

pub(crate) fn extract_single_bool_attribute(
    attrs: &[&XmlAttribute],
    xml: &Xml,
    resources: Option<&Resources>,
) -> ResourcesResult<Option<bool>> {
    if attrs.is_empty() {
        return Ok(None);
    }
    if let [attribute] = attrs[..] {
        match attribute.typed_value.resolve(&xml.string_pool, resources) {
            Ok(ResolvedValue::Bool(b)) => Ok(Some(b)),
            Ok(_) => Err(ResourcesError::ValueType(
                "boolean or reference expected".to_string(),
            )),
            Err(ResourcesError::CannotResolveWithoutResources(_)) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        Err(ResourcesError::XmlQuery(
            "not a single attribute".to_string(),
        ))
    }
}

pub(crate) fn extract_single_string_attribute(
    attrs: &[&XmlAttribute],
    xml: &Xml,
    resources: Option<&Resources>,
) -> ResourcesResult<Option<String>> {
    if attrs.is_empty() {
        return Ok(None);
    }
    if let [attribute] = attrs[..] {
        match attribute.typed_value.resolve(&xml.string_pool, resources) {
            Ok(ResolvedValue::String(s)) => Ok(Some(s)),
            Ok(_) => Err(ResourcesError::ValueType(
                "string or reference expected".to_string(),
            )),
            Err(ResourcesError::CannotResolveWithoutResources(_)) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        Err(ResourcesError::XmlQuery(
            "not a single attribute".to_string(),
        ))
    }
}
