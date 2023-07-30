use crate::chunk::{ChunkHeader, ChunkType};
use crate::errors::{ResourcesError, ResourcesResult};
use crate::resources::{Resources, ResourcesTable};
use crate::strings::{Span, StringPool, StringPoolIndex, Style, UtfString};
use crate::tables::{
    Config, TableEntry, TableLibrary, TableLibraryEntry, TableMap, TableMapEntry, TableOverlayable,
    TableOverlayablePolicy, TablePackagePoolIndex, TableStagedAlias, TableStagedAliasEntry,
    TableType, TableTypeEntry, TableTypeEntryContent, TableTypeEntryPool, TableTypeEntryPoolIndex,
    TableTypePool, TableTypePoolIndex, TableTypeSpec,
};
use crate::tables::{TablePackage, TablePackagePool};
use crate::values::Value;
use crate::xml::{
    XmlAttribute, XmlCdata, XmlElement, XmlElementAttrs, XmlEvent, XmlMetadata, XmlNamespace,
    XmlResourceMap,
};
use crate::Xml;
use nom::bytes::complete::tag;
use nom::combinator::{complete, map};
use nom::error::{ErrorKind, ParseError};
use nom::multi::count;
use nom::number::complete::{le_u16, le_u32, le_u8};
use nom::Err::Error;
use nom::Finish;
use nom::{IResult, Offset};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::sync::Arc;

pub fn parse_xml(input: &[u8]) -> ResourcesResult<Xml> {
    let (_, xml) = complete(xml_parser)(input).finish()?;
    Ok(xml)
}

pub fn parse_resources(input: &[u8]) -> ResourcesResult<Resources> {
    let (_, res) = complete(resources_parser)(input).finish()?;
    Ok(Resources(res))
}

fn xml_parser(input: &[u8]) -> IResult<&[u8], Xml, ResourcesError> {
    let input_size = input.len();

    log::debug!(">> xml_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.typ != ChunkType::Xml
        || chunk_header.header_size != 8
        || chunk_header.chunk_size != input_size
    {
        log::error!(
            "unexpected chunk header {} {} {}",
            chunk_header.typ,
            chunk_header.header_size,
            chunk_header.chunk_size
        );
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, string_pool) = string_pool_parser(input)?;

    // lookup search for optional xml resource map
    let (_, next_chunk_header) = chunk_header_parser(input)?;
    let (input, xml_resource_map) = if next_chunk_header.typ == ChunkType::XmlResourceMap {
        let (input, resource_map) = xml_resource_map_parser(input)?;
        (input, Some(resource_map))
    } else {
        (input, None)
    };

    let mut input_mut = input;
    let mut xml_body = Vec::new();
    while !input_mut.is_empty() {
        let (input, chunk) = xml_body_chunk_parser(input_mut)?;
        input_mut = input;
        xml_body.push(chunk);
    }
    let input = input_mut;

    log::debug!("<< xml_parser");

    Ok((
        input,
        Xml {
            string_pool,
            xml_resource_map,
            xml_body,
        },
    ))
}

fn resources_parser(input: &[u8]) -> IResult<&[u8], ResourcesTable, ResourcesError> {
    let input_size = input.len();

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.typ != ChunkType::Table
        || chunk_header.header_size != 12
        || chunk_header.chunk_size != input_size
    {
        log::error!(
            "unexpected chunk header {} {} {}",
            chunk_header.typ,
            chunk_header.header_size,
            chunk_header.chunk_size
        );
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, package_count) = le_u32(input)?;
    let (input, string_pool) = string_pool_parser(input)?;

    let mut table_package = Vec::with_capacity(package_count as usize);

    let mut input_mut = input;
    for i in 0..package_count {
        let input = input_mut;
        let (input, table) = table_package_parser(i as usize)(input)?;
        table_package.push(Arc::new(table));
        input_mut = input;
    }

    Ok((
        input,
        ResourcesTable {
            string_pool,
            package_pool: TablePackagePool::new(table_package).map_err(Error)?,
        },
    ))
}

fn chunk_header_parser(input: &[u8]) -> IResult<&[u8], ChunkHeader, ResourcesError> {
    log::debug!(">> chunk_header_parser");
    let (input, typ_tag) = le_u16(input)?;
    let typ = ChunkType::try_from(typ_tag).map_err(Error)?;
    let (input, header_size) = le_u16(input)?;
    let (input, chunk_size) = le_u32(input)?;
    log::debug!("chunk::type_tag = {} = {}", typ_tag, typ);
    log::debug!("chunk::header_size = {}", header_size);
    log::debug!("chunk::chunk_size = {}", chunk_size);
    log::debug!("<< chunk_header_parser");

    Ok((
        input,
        ChunkHeader {
            typ,
            header_size: header_size as usize,
            chunk_size: chunk_size as usize,
        },
    ))
}

fn string_pool_parser(input: &[u8]) -> IResult<&[u8], StringPool, ResourcesError> {
    let input0 = input;

    log::debug!(">> string_pool_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.typ != ChunkType::StringPool || chunk_header.header_size != 0x1c {
        log::error!("invalid chunk header size");
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, string_count) = le_u32(input)?;
    let (input, style_count) = le_u32(input)?;
    let (input, flags) = le_u32(input)?;

    log::debug!("string_pool::string_count = {}", string_count);
    log::debug!("string_pool::style_count = {}", style_count);
    log::debug!("string_pool::flags = {:#x}", flags);

    let sorted = (flags & 1) != 0;
    let utf8 = (flags & (1 << 8)) != 0;

    let (input, strings_start) = le_u32(input)?;
    let (input, styles_start) = le_u32(input)?;

    log::debug!("string_pool::strings_start = {:#x}", strings_start);
    log::debug!("string_pool::styles_start = {:#x}", styles_start);

    let (input, string_offsets) = count(le_u32, string_count as usize)(input)?;
    let (input, style_offsets) = count(le_u32, style_count as usize)(input)?;

    let mut strings = Vec::with_capacity(string_count as usize);
    let mut input_mut = input;
    for (idx, offset) in string_offsets.into_iter().enumerate() {
        //log::debug!(">> string::offset = {:#x}", offset);
        let input = input_mut;
        if input0.offset(input) != strings_start as usize + offset as usize {
            log::warn!(
                "invalid strings_start : {:#x} <> {:#x}",
                input0.offset(input),
                strings_start as usize + offset as usize
            );
            input_mut = &input0[strings_start as usize + offset as usize..];
        }
        let input = input_mut;

        //log::debug!("utf8 = {}", utf8);
        if utf8 {
            let (input, str_len) = le_u8(input)?;
            let (input, str_len) = if str_len & 0x80 == 0 {
                (input, u16::from(str_len))
            } else {
                let (input, len_fix) = le_u8(input)?;
                (input, (u16::from(str_len & 0x7f) << 8) + u16::from(len_fix))
            };

            let (input, raw_len) = le_u8(input)?;
            let (input, raw_len) = if raw_len & 0x80 == 0 {
                (input, u16::from(raw_len))
            } else {
                let (input, len_fix) = le_u8(input)?;
                (input, (u16::from(raw_len & 0x7f) << 8) + u16::from(len_fix))
            };

            //log::debug!("str_len = {}", str_len);
            //log::debug!("raw_len = {}", raw_len);

            let (input, data) = count(le_u8, raw_len as usize)(input)?;
            //let (input, _) = tag("\x00")(input)?;
            let (input, eos) = le_u8(input)?;
            if eos != 0 {
                log::warn!("string is not null terminated");
            }

            strings.push(Arc::new(UtfString::Utf8 {
                self_ref: StringPoolIndex::new(idx),
                raw: data,
                size: str_len as usize,
            }));
            input_mut = input;
        } else {
            let (input, len) = le_u16(input)?;
            let (input, len) = if len & 0x8000 == 0 {
                (input, u32::from(len))
            } else {
                let (input, len_fix) = le_u16(input)?;
                (input, (u32::from(len & 0x7fff) << 16) + u32::from(len_fix))
            };

            //log::debug!("len = {}", len);

            let (input, data) = count(le_u16, len as usize)(input)?;
            let (input, _) = tag("\x00\x00")(input)?;

            strings.push(Arc::new(UtfString::Utf16 {
                self_ref: StringPoolIndex::new(idx),
                raw: data,
            }));
            input_mut = input;
        }
        //log::debug!("<< string::offset = {:#x}", offset);
    }
    let input = input_mut;

    let padlen = (4 - (input0.offset(input) % 4)) % 4;
    let (input, _) = count(tag("\x00"), padlen)(input)?;

    log::debug!("padlen = {}", padlen);

    let mut styles = Vec::with_capacity(style_count as usize);
    let mut input_mut = input;
    for offset in style_offsets {
        //log::debug!(">> style_offset = {:#x}", offset);
        let offset_abs = styles_start as usize + offset as usize;
        if input0.offset(input_mut) > offset_abs {
            log::error!(
                "unexpected offset {} > {}",
                input0.offset(input_mut),
                offset_abs
            );
            return Err(Error(ResourcesError::from_error_kind(
                input_mut,
                ErrorKind::Verify,
            )));
        }
        let input = input_mut;

        let mut spans = Vec::new();
        let (input, mut name) = le_u32(input)?;

        input_mut = input;

        while name != 0xFFFF_FFFF {
            let input = input_mut;

            let (input, first_char) = le_u32(input)?;
            let (input, last_char) = le_u32(input)?;

            spans.push(Span {
                name,
                first_char,
                last_char,
            });

            let (input, next_name) = le_u32(input)?;
            name = next_name;

            input_mut = input;
        }
        let input = input_mut;

        styles.push(Style { spans });

        input_mut = input;

        //log::debug!("<< style_offset = {:#x}", offset);
    }
    let input = input_mut;

    if 0 < style_count {
        let (input, _) = tag(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])(input)?;
        input_mut = input;
    }
    let input = input_mut;

    if input0.offset(input) > chunk_header.chunk_size {
        log::error!(
            "unexpected offset {} > {}",
            input0.offset(input),
            chunk_header.chunk_size
        );
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    if input0.offset(input) < chunk_header.chunk_size {
        let rem_len = chunk_header.chunk_size - input0.offset(input);
        log::debug!(
            "remaining unparsed input of {} bytes : {:?}",
            rem_len,
            &input[0..rem_len]
        );
        let (input, _) = count(tag("\x00"), rem_len)(input)?;
        input_mut = input;
    }

    let input = input_mut;

    log::debug!("<< string_pool_parser");

    Ok((
        input,
        StringPool {
            sorted,
            utf8,
            strings,
            styles,
        },
    ))
}

fn xml_resource_map_parser(input: &[u8]) -> IResult<&[u8], XmlResourceMap, ResourcesError> {
    log::debug!(">> xml_resource_map_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.header_size != 0x08 {
        log::error!("unexpected chunk header size {}", chunk_header.header_size);
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, resource_map) = map(
        count(le_u32, (chunk_header.chunk_size - 0x08) / 4),
        |resource_ids| XmlResourceMap { resource_ids },
    )(input)?;

    log::debug!("<< xml_resource_map_parser");

    Ok((input, resource_map))
}

fn xml_body_chunk_parser(input: &[u8]) -> IResult<&[u8], XmlEvent, ResourcesError> {
    log::debug!("!! xml_body_chunk_parser");
    let (_, next_chunk_header) = chunk_header_parser(input)?;
    match next_chunk_header.typ {
        ChunkType::XmlStartNamespace => xml_namespace_parser(input, XmlEvent::StartNamespace),
        ChunkType::XmlEndNamespace => xml_namespace_parser(input, XmlEvent::EndNamespace),
        ChunkType::XmlStartElement => xml_start_element_parser(input),
        ChunkType::XmlEndElement => xml_end_element_parser(input),
        ChunkType::XmlCdata => xml_cdata_parser(input),
        _ => {
            log::error!("unexpected chunk type {:#}", next_chunk_header.typ);
            Err(Error(ResourcesError::from_error_kind(
                input,
                ErrorKind::Switch,
            )))
        }
    }
}

fn xml_metadata_parser(input: &[u8]) -> IResult<&[u8], XmlMetadata, ResourcesError> {
    let (input, line_number) = le_u32(input)?;
    let (input, comment) = le_u32(input)?;

    Ok((
        input,
        XmlMetadata {
            line_number,
            comment,
        },
    ))
}

fn xml_namespace_parser<C>(input: &[u8], constructor: C) -> IResult<&[u8], XmlEvent, ResourcesError>
where
    C: FnOnce(XmlNamespace) -> XmlEvent,
{
    log::debug!(">> xml_namespace_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.header_size != 0x10 || chunk_header.chunk_size != 0x18 {
        log::error!(
            "unexpected chunk header {} {}",
            chunk_header.header_size,
            chunk_header.chunk_size
        );
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, metadata) = xml_metadata_parser(input)?;
    let (input, prefix) = le_u32(input)?;
    let (input, uri) = le_u32(input)?;

    log::debug!("<< xml_namespace_parser");

    Ok((
        input,
        constructor(XmlNamespace {
            metadata,
            prefix: StringPoolIndex::new(prefix as usize),
            uri: StringPoolIndex::new(uri as usize),
        }),
    ))
}

fn xml_start_element_parser<'a>(input: &'a [u8]) -> IResult<&[u8], XmlEvent, ResourcesError> {
    let input0 = input;

    log::debug!(">> xml_start_element_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.header_size != 0x0010 {
        log::error!("unexpected chunk header size {}", chunk_header.header_size);
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, metadata) = xml_metadata_parser(input)?;
    let (input, ns) = le_u32(input)?;
    let (input, name) = le_u32(input)?;

    let (input, _) = tag("\x14\x00")(input)?; // attr_start
    let (input, _) = tag("\x14\x00")(input)?; // attr_size
    let (input, attr_count) = le_u16(input)?;
    let (input, id_index) = le_u16(input)?;
    let (input, class_index) = le_u16(input)?;
    let (input, style_index) = le_u16(input)?;

    let (input, attrs) = count(
        |input: &'a [u8]| {
            let (input, ns) = le_u32(input)?;
            let (input, name) = le_u32(input)?;
            let (input, raw_value) = le_u32(input)?;
            let (input, typed_value) = value_parser(input)?;
            Ok((
                input,
                XmlAttribute {
                    ns: (ns != 0xffff_ffff).then(|| StringPoolIndex::new(ns as usize)),
                    name: StringPoolIndex::new(name as usize),
                    raw_value,
                    typed_value,
                },
            ))
        },
        attr_count as usize,
    )(input)?;

    if input0.offset(input) != chunk_header.chunk_size {
        log::error!(
            "unexpected offset {} {}",
            input0.offset(input),
            chunk_header.chunk_size
        );
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    log::debug!("<< xml_start_element_parser");

    Ok((
        input,
        XmlEvent::StartElement(
            XmlElement {
                metadata,
                ns: (ns != 0xffff_ffff).then(|| StringPoolIndex::new(ns as usize)),
                name: StringPoolIndex::new(name as usize),
            },
            XmlElementAttrs {
                id_index,
                class_index,
                style_index,
                attrs,
            },
        ),
    ))
}

fn xml_end_element_parser(input: &[u8]) -> IResult<&[u8], XmlEvent, ResourcesError> {
    log::debug!(">> xml_end_element_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.header_size != 0x10 || chunk_header.chunk_size != 0x18 {
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, metadata) = xml_metadata_parser(input)?;
    let (input, ns) = le_u32(input)?;
    let (input, name) = le_u32(input)?;

    log::debug!("<< xml_end_element_parser");

    Ok((
        input,
        XmlEvent::EndElement(XmlElement {
            metadata,
            ns: (ns != 0xffff_ffff).then(|| StringPoolIndex::new(ns as usize)),
            name: StringPoolIndex::new(name as usize),
        }),
    ))
}

fn xml_cdata_parser(input: &[u8]) -> IResult<&[u8], XmlEvent, ResourcesError> {
    log::debug!(">> xml_cdata_parser");

    let (input, chunk_header) = chunk_header_parser(input)?;
    if chunk_header.header_size != 0x10 || chunk_header.chunk_size != 0x1c {
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, metadata) = xml_metadata_parser(input)?;
    let (input, data) = le_u32(input)?;
    log::debug!("xml_cdata::data = {:#x}", data);
    let (input, value) = value_parser(input)?;

    log::debug!("<< xml_cdata_parser");

    Ok((
        input,
        XmlEvent::Cdata(XmlCdata {
            metadata,
            data: StringPoolIndex::new(data as usize),
            value,
        }),
    ))
}

fn value_parser(input: &[u8]) -> IResult<&[u8], Value, ResourcesError> {
    //let (input, _) = tag("\x08\x00")(input)?; // size
    let (input, _size) = le_u16(input)?;
    let (input, _) = tag("\x00")(input)?;
    let (input, vtyp) = le_u8(input)?;
    let (input, data) = le_u32(input)?;

    log::debug!("value:vtyp = {:#x}", vtyp);
    log::debug!("value:data = {:#x}", data);

    let value = match vtyp {
        0x00 => {
            if data != 0 {
                log::warn!("value null contains data");
            }
            Value::Null
        }
        0x01 => Value::Reference(data),
        0x02 => Value::Attribute(data),
        0x03 => Value::String(StringPoolIndex::new(data as usize)),
        0x04 => Value::Float(f32::from_bits(data)),
        0x05 => Value::Dimension(data),
        0x06 => Value::Fraction(data),
        0x10 => Value::IntDec(data),
        0x11 => Value::IntHex(data),
        0x12 => Value::IntBoolean(data != 0),
        0x1c => Value::IntColorARGB8(data),
        0x1d => Value::IntColorRGB8(data),
        0x1e => Value::IntColorARGB4(data),
        0x1f => Value::IntColorRGB4(data),
        _ => {
            log::debug!("unknown value type: {:#x}", vtyp);
            return Err(Error(ResourcesError::from_error_kind(
                input,
                ErrorKind::Switch,
            )));
        }
    };

    Ok((input, value))
}

fn opt_value<T>(default: T) -> impl FnMut(T) -> Option<T>
where
    T: 'static + Eq,
{
    move |value: T| -> Option<T> {
        if value != default {
            return Some(value);
        }
        None
    }
}

fn table_package_parser<'a>(
    pool_idx: usize,
) -> impl Fn(&'a [u8]) -> IResult<&[u8], TablePackage, ResourcesError> {
    move |input: &[u8]| {
        let input0 = input;

        log::debug!(">> table_package_parser");

        let (input, chunk_header) = chunk_header_parser(input)?;
        if chunk_header.typ != ChunkType::TablePackage {
            log::error!("unexpected chunk header {}", chunk_header.typ);
            return Err(Error(ResourcesError::from_error_kind(
                input,
                ErrorKind::Verify,
            )));
        }

        let (input, id) = le_u32(input)?;
        let id = u8::try_from(id).map_err(|_| {
            Error(ResourcesError::UnexpectedValue {
                name: "package id".to_string(),
                typ: "not an u8".to_string(),
            })
        })?;
        let (input, name_raw) = count(le_u16, 128)(input)?;
        let first_zero = name_raw.partition_point(|c| *c != 0);
        let name = String::from_utf16(&name_raw[0..first_zero])
            .map_err(|_| Error(ResourcesError::InvalidUtf16("package name".to_string())))?;
        let (input, type_strings_offset) = le_u32(input)?;
        let (input, last_public_type) = le_u32(input)?;
        let (input, key_strings_offset) = le_u32(input)?;
        let (input, last_public_key) = le_u32(input)?;

        log::debug!("package::id = {:#x}", id);
        log::debug!("package::name.len = {}", name.len());
        log::debug!("package::type_strings_offset = {:#x}", type_strings_offset);
        log::debug!("package::last_public_type = {}", last_public_type);
        log::debug!("package::key_strings_offset = {:#x}", key_strings_offset);
        log::debug!("package::last_public_key = {}", last_public_key);

        let padlen = type_strings_offset as usize - input0.offset(input);
        let (input, _) = count(tag("\x00"), padlen)(input)?;

        let mut type_strings = None;
        let mut key_strings = None;
        let mut string_pools = Vec::new();
        let mut table_type_specs = Vec::new();
        let mut table_types = Vec::new();
        let mut table_libraries = Vec::new();
        let mut table_overlayables = Vec::new();
        let mut table_overlayable_policies = Vec::new();
        let mut table_staged_aliases = Vec::new();

        let mut input_mut = input;
        while input0.offset(input_mut) < chunk_header.chunk_size {
            let (_, next_chunk_header) = chunk_header_parser(input_mut)?;
            match next_chunk_header.typ {
                ChunkType::StringPool => {
                    let (input, pool) = string_pool_parser(input_mut)?;
                    if input0.offset(input_mut) == type_strings_offset as usize {
                        type_strings = Some(pool);
                    } else if input0.offset(input_mut) == key_strings_offset as usize {
                        key_strings = Some(pool);
                    } else {
                        string_pools.push(pool);
                    }
                    input_mut = input;
                }
                ChunkType::TableTypeSpec => {
                    let (input, table) = table_type_spec_parser(input_mut)?;
                    table_type_specs.push(table);
                    input_mut = input;
                }
                ChunkType::TableType => {
                    let (input, table) = table_type_parser(table_types.len())(input_mut)?;
                    table_types.push(Arc::new(table));
                    input_mut = input;
                }
                ChunkType::TableLibrary => {
                    let (input, table) = table_library_parser(input_mut)?;
                    table_libraries.push(table);
                    input_mut = input;
                }
                ChunkType::TableOverlayable => {
                    let (input, table) = table_overlayable_parser(input_mut)?;
                    table_overlayables.push(table);
                    input_mut = input;
                }
                ChunkType::TableOverlayablePolicy => {
                    let (input, table) = table_overlayable_policy_parser(input_mut)?;
                    table_overlayable_policies.push(table);
                    input_mut = input;
                }
                ChunkType::TableStagedAlias => {
                    let (input, table) = table_staged_alias_parser(input_mut)?;
                    table_staged_aliases.push(table);
                    input_mut = input;
                }
                _ => {
                    log::debug!("unexpected chunk type = {}", next_chunk_header.typ);
                    return Err(Error(ResourcesError::from_error_kind(
                        input,
                        ErrorKind::Switch,
                    )));
                }
            }
        }
        let input = input_mut;

        log::debug!("<< table_package_parser");

        let package = TablePackage {
            self_ref: TablePackagePoolIndex::new(pool_idx),
            id,
            name,
            last_public_type,
            last_public_key,
            type_strings,
            key_strings,
            string_pools,
            table_type_specs,
            type_pool: TableTypePool::new(table_types),
            table_libraries,
            table_overlayables,
            table_overlayable_policies,
            table_staged_aliases,
        };

        Ok((input, package))
    }
}

fn table_type_spec_parser(input: &[u8]) -> IResult<&[u8], TableTypeSpec, ResourcesError> {
    let input0 = input;
    let (input, chunk_header) = chunk_header_parser(input)?;

    log::debug!(">> table_type_spec_parser");

    if chunk_header.header_size != 0x10 {
        log::error!("invalid chunk header size: {}", chunk_header.header_size);
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    let (input, id) = le_u8(input)?;
    let (input, _) = tag("\x00")(input)?;
    let (input, _) = tag("\x00\x00")(input)?;
    let (input, entry_count) = le_u32(input)?;
    let (input, config_mask) = count(le_u32, entry_count as usize)(input)?;

    log::debug!("type_spec::id = {:#x}", id);
    log::debug!("type_spec::entry_count = {}", entry_count);

    if input0.offset(input) != chunk_header.chunk_size {
        log::error!("invalid offset: {:#x}", input0.offset(input));
        return Err(Error(ResourcesError::from_error_kind(
            input,
            ErrorKind::Verify,
        )));
    }

    log::debug!("<< table_type_spec_parser");

    Ok((input, TableTypeSpec { id, config_mask }))
}

fn table_type_parser<'a>(
    pool_idx: usize,
) -> impl Fn(&'a [u8]) -> IResult<&[u8], TableType, ResourcesError> {
    move |input: &[u8]| {
        let input0 = input;

        log::debug!(">> table_type_parser");

        let (input, chunk_header) = chunk_header_parser(input)?;

        log::debug!("header_size = {}", chunk_header.header_size);
        log::debug!("chunk_size = {}", chunk_header.chunk_size);

        let (input, id) = le_u8(input)?;
        let (input, _) = tag("\x00")(input)?;
        let (input, _) = tag("\x00\x00")(input)?;
        let (input, entry_count) = le_u32(input)?;
        let (input, entries_start) = le_u32(input)?;

        log::debug!("type::id = {:#x}", id);
        log::debug!("type::entry_count = {}", entry_count);
        log::debug!("type::entries_start = {:#x}", entries_start);

        let input1 = input;

        let (input, config_size) = le_u32(input)?;
        let (input, imsi_mcc) = map(le_u16, opt_value(0))(input)?;
        let (input, imsi_mnc) = map(le_u16, opt_value(0))(input)?;
        let (input, locale_language) = count(le_u8, 2)(input)?;
        let locale_language = if locale_language.iter().any(|b| b != &0) {
            Some(locale_language)
        } else {
            None
        };
        let (input, locale_country) = count(le_u8, 2)(input)?;
        let locale_country = if locale_country.iter().any(|b| b != &0) {
            Some(locale_country)
        } else {
            None
        };
        let (input, screen_type_orientation) = map(le_u8, opt_value(0))(input)?;
        let (input, screen_type_touchscreen) = map(le_u8, opt_value(0))(input)?;
        let (input, screen_type_density) = map(le_u16, opt_value(0))(input)?;
        let (input, input_keyboard) = map(le_u8, opt_value(0))(input)?;
        let (input, input_navigation) = map(le_u8, opt_value(0))(input)?;
        let (input, input_flags) = map(le_u8, opt_value(0))(input)?;
        let (input, input_pad0) = map(le_u8, opt_value(0))(input)?;
        let (input, screen_size_width) = map(le_u16, opt_value(0))(input)?;
        let (input, screen_size_height) = map(le_u16, opt_value(0))(input)?;
        let (input, version_sdk) = map(le_u16, opt_value(0))(input)?;
        let (input, version_minor) = map(le_u16, opt_value(0))(input)?;

        log::debug!("type::config_size = {}", config_size);
        log::debug!("type::imsi_mcc = {:?}", imsi_mcc);
        log::debug!("type::imsi_mnc = {:?}", imsi_mnc);
        log::debug!("type::locale_language = {:?}", locale_language);
        log::debug!("type::locale_country = {:?}", locale_country);
        log::debug!(
            "type::screen_type_orientation = {:?}",
            screen_type_orientation
        );
        log::debug!(
            "type::screen_type_touchscreen = {:?}",
            screen_type_touchscreen
        );
        log::debug!("type::screen_type_density = {:?}", screen_type_density);
        log::debug!("type::input_keyboard = {:?}", input_keyboard);
        log::debug!("type::input_navigation = {:?}", input_navigation);
        log::debug!("type::input_flags = {:?}", input_flags);
        log::debug!("type::input_pad0 = {:?}", input_pad0);
        log::debug!("type::screen_size_width = {:?}", screen_size_width);
        log::debug!("type::screen_size_height = {:?}", screen_size_height);
        log::debug!("type::version_sdk = {:?}", version_sdk);
        log::debug!("type::version_minor = {:?}", version_minor);

        let mut config = Config {
            imsi_mcc,
            imsi_mnc,
            locale_language,
            locale_country,
            screen_type_orientation,
            screen_type_touchscreen,
            screen_type_density,
            input_keyboard,
            input_navigation,
            input_flags,
            input_pad0,
            screen_size_width,
            screen_size_height,
            version_sdk,
            version_minor,
            screen_config_layout: None,
            screen_config_ui_mode: None,
            screen_config_smallest_width_dp: None,
            screen_size_dp_width: None,
            screen_size_dp_height: None,
            locale_script: None,
            locale_variant: None,
            screen_config_2_layout: None,
            screen_config_color_mode: None,
            screen_config_2_pad2: None,
        };

        let mut input_mut = input;

        if 32 <= config_size {
            let (input, screen_config_layout) = map(le_u8, opt_value(0))(input)?;
            let (input, screen_config_ui_mode) = map(le_u8, opt_value(0))(input)?;
            let (input, screen_config_smallest_width_dp) = map(le_u16, opt_value(0))(input)?;

            log::debug!("type::screen_config_layout = {:?}", screen_config_layout);
            log::debug!("type::screen_config_ui_mode = {:?}", screen_config_ui_mode);
            log::debug!(
                "type::screen_config_smallest_width_dp = {:?}",
                screen_config_smallest_width_dp
            );

            config.screen_config_layout = screen_config_layout;
            config.screen_config_ui_mode = screen_config_ui_mode;
            config.screen_config_smallest_width_dp = screen_config_smallest_width_dp;

            input_mut = input;

            if 36 <= config_size {
                let (input, screen_size_dp_width) = map(le_u16, opt_value(0))(input)?;
                let (input, screen_size_dp_height) = map(le_u16, opt_value(0))(input)?;

                log::debug!("type::screen_size_dp_width = {:?}", screen_size_dp_width);
                log::debug!("type::screen_size_dp_height = {:?}", screen_size_dp_height);

                config.screen_size_dp_width = screen_size_dp_width;
                config.screen_size_dp_height = screen_size_dp_height;

                input_mut = input;

                if 48 <= config_size {
                    let (input, locale_script) = count(le_u8, 4)(input)?;
                    let locale_script = if locale_script.iter().any(|b| b != &0) {
                        Some(locale_script)
                    } else {
                        None
                    };
                    let (input, locale_variant) = count(le_u8, 8)(input)?;
                    let locale_variant = if locale_variant.iter().any(|b| b != &0) {
                        Some(locale_variant)
                    } else {
                        None
                    };

                    log::debug!("type::locale_script = {:?}", locale_script);
                    log::debug!("type::locale_variant = {:?}", locale_variant);

                    config.locale_script = locale_script;
                    config.locale_variant = locale_variant;

                    input_mut = input;

                    if 52 <= config_size {
                        let (input, screen_config_2_layout) = map(le_u8, opt_value(0))(input)?;
                        let (input, screen_config_color_mode) = map(le_u8, opt_value(0))(input)?;
                        let (input, screen_config_2_pad2) = map(le_u16, opt_value(0))(input)?;

                        log::debug!(
                            "type::screen_config_2_layout = {:?}",
                            screen_config_2_layout
                        );
                        log::debug!(
                            "type::screen_config_color_mode = {:?}",
                            screen_config_color_mode
                        );
                        log::debug!("type::screen_config_2_pad2 = {:?}", screen_config_2_pad2);

                        config.screen_config_2_layout = screen_config_2_layout;
                        config.screen_config_color_mode = screen_config_color_mode;
                        config.screen_config_2_pad2 = screen_config_2_pad2;

                        input_mut = input;
                    }
                }
            }
        };
        let input = input_mut;

        let padlen = config_size as usize - input1.offset(input);
        let (input, _) = count(tag("\x00"), padlen)(input)?;

        let (input, entry_offsets) = count(le_u32, entry_count as usize)(input)?;

        let mut entries = BTreeMap::new();
        let mut entry_idx: u16 = 0;
        let mut input_mut = input;
        for offset in entry_offsets {
            log::debug!(">> type::entry_offset = {:#x}", offset);
            if offset == 0xFFFF_FFFF {
                entry_idx += 1;
                continue;
            }
            let input = input_mut;
            if input0.offset(input) != entries_start as usize + offset as usize {
                log::warn!(
                    "invalid offset: {:#x} != {:#x} (= {:#x} + {:#x}) => using given offset",
                    input0.offset(input),
                    entries_start as usize + offset as usize,
                    entries_start,
                    offset
                );
                input_mut = &input0[entries_start as usize + offset as usize..];
                /*
                    return Err(Error(ResourcesError::from_error_kind(
                    input,
                    ErrorKind::Verify,
                )));
                     */
            }
            let input = input_mut;

            let (input, entry) = table_entry_parser(input)?;

            log::debug!("is_complex = {}", entry.is_complex());

            let content = if entry.is_complex() {
                let (input, parent) = le_u32(input)?;
                let (input, map_count) = le_u32(input)?;

                log::debug!("entry::parent = {:#x}", parent);
                log::debug!("entry::map_count = {}", map_count);

                let (input, table_maps) = count(table_map_parser, map_count as usize)(input)?;

                let map = TableMapEntry { parent, table_maps };
                input_mut = input;
                TableTypeEntryContent::EntryMap(map)
            } else {
                let (input, value) = value_parser(input)?;
                log::debug!("entry = {:?}", value);
                input_mut = input;
                TableTypeEntryContent::EntryValue(value)
            };

            let _opt = entries.insert(
                entry_idx,
                Arc::new(TableTypeEntry {
                    self_ref: TableTypeEntryPoolIndex::new(entry_idx),
                    entry,
                    content,
                }),
            );

            entry_idx += 1;

            log::debug!("<< type::entry_offset = {:#x}", offset);
        }
        let input = input_mut;

        if input0.offset(input) != chunk_header.chunk_size {
            log::warn!(
                "invalid end offset: {:#x} != {:#x}",
                input0.offset(input),
                chunk_header.chunk_size
            );
            input_mut = &input0[chunk_header.chunk_size..];
            /*
                return Err(Error(ResourcesError::from_error_kind(
                input,
                ErrorKind::Verify,
            )));
                 */
        }

        let input = input_mut;

        log::debug!("<< table_type_parser");

        Ok((
            input,
            TableType {
                self_ref: TableTypePoolIndex::new(pool_idx),
                id,
                config,
                entry_pool: TableTypeEntryPool::new(entries),
            },
        ))
    }
}

fn table_entry_parser(input: &[u8]) -> IResult<&[u8], TableEntry, ResourcesError> {
    let (input, _size) = le_u16(input)?;
    let (input, flags) = le_u16(input)?;
    let (input, key) = le_u32(input)?;

    log::debug!("entry::flags = {:#x}", flags);
    log::debug!("entry::key = {:#x}", key);

    Ok((input, TableEntry { flags, key }))
}

fn table_map_parser(input: &[u8]) -> IResult<&[u8], TableMap, ResourcesError> {
    log::debug!(">> table_map_parser");

    let (input, name) = le_u32(input)?;
    log::debug!("map::name = {:#x}", name);
    let (input, value) = value_parser(input)?;

    log::debug!("<< table_map_parser");

    Ok((input, TableMap { name, value }))
}

fn table_library_parser(input: &[u8]) -> IResult<&[u8], TableLibrary, ResourcesError> {
    log::debug!(">> table_library_parser");

    let (input, _chunk_header) = chunk_header_parser(input)?;

    let (input, count) = le_u32(input)?;
    log::debug!("library::count = {}", count);

    let mut libraries = Vec::with_capacity(count as usize);

    let mut input_mut = input;
    for _ in 0..count {
        let input = input_mut;

        let (input, library) = table_library_entry_parser(input)?;
        libraries.push(library);

        input_mut = input;
    }
    let input = input_mut;

    log::debug!("<< table_library_parser");

    Ok((input, TableLibrary { libraries }))
}

fn table_library_entry_parser(input: &[u8]) -> IResult<&[u8], TableLibraryEntry, ResourcesError> {
    log::debug!(">> table_library_entry_parser");

    let (input, id) = le_u32(input)?;
    let (input, name_raw) = count(le_u16, 128)(input)?;
    let first_zero = name_raw.partition_point(|c| *c != 0);
    let name = String::from_utf16(&name_raw[0..first_zero])
        .map_err(|_| Error(ResourcesError::InvalidUtf16("library name".to_string())))?;

    log::debug!("library::id = {:#x}", id);
    log::debug!("library::name = {:?}", name);

    log::debug!("<< table_library_entry_parser");

    Ok((input, TableLibraryEntry { id, name }))
}

fn table_overlayable_parser(input: &[u8]) -> IResult<&[u8], TableOverlayable, ResourcesError> {
    log::debug!(">> table_overlayable_parser");

    let (input, _chunk_header) = chunk_header_parser(input)?;

    let (input, name_raw) = count(le_u16, 256)(input)?;
    let first_zero = name_raw.partition_point(|c| *c != 0);
    let name = String::from_utf16(&name_raw[0..first_zero])
        .map_err(|_| Error(ResourcesError::InvalidUtf16("overlayable name".to_string())))?;

    log::debug!("overlayable::name = {:?}", name);

    let (input, actor_raw) = count(le_u16, 256)(input)?;
    let first_zero = actor_raw.partition_point(|c| *c != 0);
    let actor = String::from_utf16(&actor_raw[0..first_zero]).map_err(|_| {
        Error(ResourcesError::InvalidUtf16(
            "overlayable actor".to_string(),
        ))
    })?;

    log::debug!("overlayable::actor = {:?}", actor);

    log::debug!("<< table_overlayable_parser");

    Ok((input, TableOverlayable { name, actor }))
}

fn table_overlayable_policy_parser(
    input: &[u8],
) -> IResult<&[u8], TableOverlayablePolicy, ResourcesError> {
    log::debug!(">> table_overlayable_policy_parser");

    let (input, _chunk_header) = chunk_header_parser(input)?;

    let (input, flags) = le_u32(input)?;
    let (input, entry_count) = le_u32(input)?;

    log::debug!("overlayable_policy::flags = {:#x}", flags);
    log::debug!("overlayable_policy::entry_count = {:#x}", entry_count);

    let (input, entries) = count(le_u32, entry_count as usize)(input)?;

    log::debug!("<< table_overlayable_policy_parser");

    Ok((input, TableOverlayablePolicy { flags, entries }))
}

fn table_staged_alias_parser(input: &[u8]) -> IResult<&[u8], TableStagedAlias, ResourcesError> {
    log::debug!(">> table_staged_alias_parser");

    let (input, _chunk_header) = chunk_header_parser(input)?;

    let (input, entry_count) = le_u32(input)?;

    log::debug!("staged_alias::entry_count = {:#x}", entry_count);

    let (input, entries) = count(table_staged_alias_entry_parser, entry_count as usize)(input)?;

    log::debug!("<< table_staged_alias_parser");

    Ok((input, TableStagedAlias { entries }))
}

fn table_staged_alias_entry_parser(
    input: &[u8],
) -> IResult<&[u8], TableStagedAliasEntry, ResourcesError> {
    log::debug!(">> table_staged_alias_entry_parser");

    let (input, stage_id) = le_u32(input)?;
    let (input, finalized_id) = le_u32(input)?;

    log::debug!("staged_alias_entry::stage_id = {:#x}", stage_id);
    log::debug!("staged_alias_entry::finalized_id = {:#x}", finalized_id);

    log::debug!("<< table_staged_alias_entry_parser");

    Ok((
        input,
        TableStagedAliasEntry {
            stage_id,
            finalized_id,
        },
    ))
}
