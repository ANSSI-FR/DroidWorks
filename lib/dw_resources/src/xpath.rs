use crate::errors::{ResourcesError, ResourcesResult};
use crate::strings::StringPoolIndex;
use crate::values::Value;
use crate::xml::{XmlAttribute, XmlElement, XmlElementAttrs, XmlEvent, XmlMetadata};
use crate::Xml;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(crate) struct Context<'a> {
    xml: &'a Xml,
    selection: Vec<(XPathResult, BTreeMap<StringPoolIndex, StringPoolIndex>)>,
}

pub(crate) struct ContextMut<'a> {
    xml: &'a mut Xml,
    selection: Vec<(XPathResult, BTreeMap<StringPoolIndex, StringPoolIndex>)>,
}

impl<'a> Context<'a> {
    pub(crate) const fn new(xml: &'a Xml) -> Self {
        Context {
            xml,
            selection: Vec::new(),
        }
    }

    pub(crate) fn select(self, selector: Select) -> ResourcesResult<Self> {
        let Context { xml, selection } = self;
        let mut new_selection = Vec::new();

        match selector {
            Select::Root(regsel) => {
                let starts = if selection.is_empty() {
                    Ok(vec![(0, BTreeMap::new())])
                } else {
                    selection
                        .into_iter()
                        .map(|(selected, namespaces)| Ok((selected.node()? + 1, namespaces)))
                        .collect::<ResourcesResult<Vec<(usize, BTreeMap<_, _>)>>>()
                }?;
                for (start, mut namespaces) in starts {
                    let mut level = 1;
                    let mut i = start;
                    while level > 0 && i < xml.xml_body.len() {
                        match &xml.xml_body[i] {
                            XmlEvent::StartNamespace(ns) => {
                                namespaces.insert(ns.uri, ns.prefix);
                            }
                            XmlEvent::EndNamespace(ns) => {
                                let _prev = namespaces.remove(&ns.uri).unwrap();
                            }
                            XmlEvent::StartElement(elt, _) => {
                                if level == 1
                                    && regsel.is_match(&xml.string_pool.get(elt.name)?.string()?)
                                {
                                    new_selection.push((XPathResult::Node(i), namespaces.clone()));
                                }
                                level += 1;
                            }
                            XmlEvent::EndElement(_) => level -= 1,
                            _ => (),
                        }
                        i += 1;
                    }
                }
            }

            Select::Attr(sel) => {
                for (selected, namespaces) in selection {
                    let event_id = selected.node()?;
                    let (_, attrs) = xml.xml_body.get(event_id).unwrap().start_element()?;
                    for (attr_id, attr) in attrs.attrs.iter().enumerate() {
                        if xml.string_pool.get(attr.name)?.string()? == sel {
                            new_selection
                                .push((XPathResult::Attr(event_id, attr_id), namespaces.clone()));
                        }
                    }
                }
            }
        }
        Ok(Self {
            xml,
            selection: new_selection,
        })
    }

    pub(crate) fn filter(self, predicate: Predicate) -> ResourcesResult<Self> {
        let Context { xml, selection } = self;
        let mut new_selection = Vec::new();

        match predicate {
            Predicate::Attr(attr_name, attr_str) => {
                for (selected, namespaces) in selection {
                    let event_id = selected.node()?;
                    let (_, attrs) = xml.xml_body.get(event_id).unwrap().start_element()?;
                    let mut pred_satisfied = false;
                    for attr in &attrs.attrs {
                        if xml.string_pool.get(attr.name)?.string()? == attr_name {
                            match &attr.typed_value {
                                Value::String(str_ref) => {
                                    if xml.string_pool.get(*str_ref)?.string()? == attr_str {
                                        pred_satisfied = true;
                                    }
                                }
                                _ => {
                                    return Err(ResourcesError::XmlQuery(
                                        "not a string".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                    if pred_satisfied {
                        new_selection.push((selected, namespaces));
                    }
                }
            }
        }
        Ok(Self {
            xml,
            selection: new_selection,
        })
    }

    pub(crate) fn nodes(self) -> ResourcesResult<Vec<(&'a XmlElement, &'a XmlElementAttrs)>> {
        let Context { xml, selection } = self;
        let mut nodes = Vec::new();
        for (selected, _namespaces) in selection {
            let event_id = selected.node()?;
            let (elt, attrs) = xml.xml_body.get(event_id).unwrap().start_element()?;
            nodes.push((elt, attrs));
        }
        Ok(nodes)
    }

    pub(crate) fn attributes(self) -> ResourcesResult<Vec<&'a XmlAttribute>> {
        let Context { xml, selection } = self;
        let mut attributes = Vec::new();
        for (selected, _namespaces) in selection {
            let (event_id, attr_id) = selected.attr()?;
            let (_elt, attrs) = xml.xml_body.get(event_id).unwrap().start_element()?;
            attributes.push(attrs.attrs.get(attr_id).unwrap());
        }
        Ok(attributes)
    }
}

impl<'a> ContextMut<'a> {
    pub(crate) fn new(xml: &'a mut Xml) -> Self {
        ContextMut {
            xml,
            selection: Vec::new(),
        }
    }

    pub(crate) fn select(self, selector: Select) -> ResourcesResult<Self> {
        let context = Context {
            xml: self.xml,
            selection: self.selection,
        };
        let new_context = context.select(selector)?;
        let new_selection = new_context.selection;
        Ok(Self {
            xml: self.xml,
            selection: new_selection,
        })
    }

    pub(crate) fn filter(self, predicate: Predicate) -> ResourcesResult<Self> {
        let context = Context {
            xml: self.xml,
            selection: self.selection,
        };
        let new_context = context.filter(predicate)?;
        let new_selection = new_context.selection;
        Ok(Self {
            xml: self.xml,
            selection: new_selection,
        })
    }

    pub(crate) fn has_empty_selection(&self) -> bool {
        self.selection.is_empty()
    }

    pub(crate) fn add_self_contained_nodes(
        self,
        name: String,
        mut attrs: Vec<(Option<String>, String, Value)>,
    ) -> ResourcesResult<ContextMut<'a>> {
        let (name, _) = self.xml.string_pool.get_or_push(name)?;

        let attrs = attrs
            .drain(..)
            .map(|(prefix, name, typed_value)| {
                self.xml.string_pool.get_or_push(name).map(|(name, _)| {
                    let raw_value = typed_value.raw_value();
                    let attribute = XmlAttribute {
                        ns: None, // will be updated in add_self_contained_nodes_raw
                        name,
                        raw_value,
                        typed_value,
                    };
                    (prefix, attribute)
                })
            })
            .collect::<ResourcesResult<Vec<(Option<String>, XmlAttribute)>>>()?;

        let element = XmlElement {
            metadata: XmlMetadata {
                line_number: 0,
                comment: 0,
            },
            ns: None,
            name,
        };

        self.add_self_contained_nodes_raw(&element, attrs)
    }

    fn add_self_contained_nodes_raw(
        mut self,
        element: &XmlElement,
        attributes: Vec<(Option<String>, XmlAttribute)>,
    ) -> ResourcesResult<ContextMut<'a>> {
        let starts = self.selection
            .drain(..)
            .map(|(selected, namespaces)| selected.node().map(|node| (node, namespaces)))
            .collect::<ResourcesResult<BTreeMap<usize, BTreeMap<StringPoolIndex, StringPoolIndex>>>>()?;

        self.selection = Vec::new();

        for (start, namespaces) in starts.into_iter().rev() {
            let start = start + 1;
            let end_element = XmlEvent::EndElement(element.clone());

            let attrs = attributes
                .iter()
                .map(|(prefix, attribute)| {
                    let mut attribute = attribute.clone();
                    if let Some(prefix) = prefix {
                        for (uri_index, prefix_index) in &namespaces {
                            if self
                                .xml
                                .string_pool
                                .get(*prefix_index)?
                                .string()?
                                .eq(prefix)
                            {
                                attribute.ns = Some(*uri_index);
                                break;
                            }
                        }
                    }
                    Ok(attribute)
                })
                .collect::<ResourcesResult<Vec<XmlAttribute>>>()?;

            let attributes = XmlElementAttrs {
                id_index: 0,
                class_index: 0,
                style_index: 0,
                attrs,
            };

            let start_element = XmlEvent::StartElement(element.clone(), attributes);

            self.xml.xml_body.insert(start, end_element);
            self.xml.xml_body.insert(start, start_element);

            self.selection.push((XPathResult::Node(start), namespaces));
        }

        Ok(self)
    }

    pub(crate) fn remove_nodes(self) -> ResourcesResult<bool> {
        let ContextMut { xml, selection } = self;
        let starts = selection
            .iter()
            .map(|(selected, _namespaces)| selected.node())
            .collect::<ResourcesResult<BTreeSet<usize>>>()?;
        let mut removed = false;
        for start in starts.into_iter().rev() {
            let mut level = 1;
            let mut end = start + 1;
            while level > 0 && end < xml.xml_body.len() {
                match &xml.xml_body[end] {
                    XmlEvent::StartElement(_, _) => level += 1,
                    XmlEvent::EndElement(_) => level -= 1,
                    _ => (),
                }
                end += 1;
            }
            let _drain = xml.xml_body.drain(start..end);
            removed = true;
        }
        Ok(removed)
    }

    pub(crate) fn remove_attributes(self) -> ResourcesResult<bool> {
        let ContextMut { xml, selection } = self;
        let nodes_attrs = selection
            .iter()
            .map(|(selected, _namespaces)| selected.attr())
            .collect::<ResourcesResult<BTreeSet<(usize, usize)>>>()?;
        let mut removed = false;
        for (node, attr) in nodes_attrs {
            match xml.xml_body.get_mut(node).unwrap() {
                XmlEvent::StartElement(_, attrs) => {
                    attrs.attrs.remove(attr);
                    removed = true;
                }
                _ => {
                    return Err(ResourcesError::XmlQuery(
                        "'start element' event was expected".to_string(),
                    ))
                }
            }
        }
        Ok(removed)
    }

    pub(crate) fn insert_attribute(self, name: String, value: Value) -> ResourcesResult<()> {
        let ContextMut { xml, selection } = self;
        for (selected, namespaces) in selection {
            let event_id = selected.node()?;
            if let Some(XmlEvent::StartElement(_elt, attrs)) = xml.xml_body.get_mut(event_id) {
                let (string_pool_index, _) = xml.string_pool.get_or_push(name.clone())?;
                // TODO 1/ for now, we push 'android' namespace. must be patched when supporting
                // other attributes in the manifest
                let mut ns = None;
                for (uri, prefix) in namespaces {
                    if xml.string_pool.get(prefix)?.string()?.as_str() == "android" {
                        ns = Some(uri);
                        break;
                    }
                }
                let ns = ns.unwrap();
                attrs.attrs.push(XmlAttribute {
                    ns: Some(ns),
                    name: string_pool_index,
                    raw_value: value.raw_value(),
                    typed_value: value,
                });
            } else {
                return Err(ResourcesError::XmlQuery(
                    "'start element' event was expected".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn edit_attribute(self, new_value: Value) -> ResourcesResult<()> {
        let ContextMut { xml, selection } = self;
        for (selected, _namespaces) in selection {
            let (event_id, attr_id) = selected.attr()?;
            if let Some(XmlEvent::StartElement(_elt, attrs)) = xml.xml_body.get_mut(event_id) {
                match new_value {
                    Value::IntBoolean(b) => {
                        // for intbool, no need to edit raw_value field,
                        // which should stay equal to 0xffffffff
                        attrs.attrs[attr_id].typed_value = Value::IntBoolean(b);
                    }
                    _ => panic!("unsupported edit value type"),
                }
            } else {
                return Err(ResourcesError::XmlQuery(
                    "'start element' event was expected".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Select<'a> {
    Root(&'a Regex), // '/node'
    Attr(&'a str),   // '@attr'
}

#[derive(Clone, Copy)]
pub(crate) enum Predicate<'a> {
    Attr(&'a str, &'a str), // '[@attr=str]'
}

#[derive(Clone)]
pub(crate) enum XPathResult {
    Node(usize),
    Attr(usize, usize),
}

impl XPathResult {
    fn node(&self) -> ResourcesResult<usize> {
        match self {
            Self::Node(event_id) => Ok(*event_id),
            _ => Err(ResourcesError::XmlQuery(
                "node xpath result was expected".to_string(),
            )),
        }
    }

    fn attr(&self) -> ResourcesResult<(usize, usize)> {
        match self {
            Self::Attr(event_id, attr_id) => Ok((*event_id, *attr_id)),
            _ => Err(ResourcesError::XmlQuery(
                "attr xpath result was expected".to_string(),
            )),
        }
    }
}
