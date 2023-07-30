use crate::chunk::{ChunkHeader, ChunkType};
use crate::errors::ResourcesResult;
use crate::resources::Resources;
use crate::strings::{StringPool, UtfString};
use crate::values::Value;
use crate::xml::{
    XmlCdata, XmlElement, XmlElementAttrs, XmlEvent, XmlMetadata, XmlNamespace, XmlResourceMap,
};
use crate::Xml;
use dw_utils::writers::{bytes, le_u16, le_u32, le_u8, tag};
use std::io::{Cursor, Result, Write};

#[allow(dead_code)]
pub fn write_resources(_resources: &Resources) -> ResourcesResult<Vec<u8>> {
    todo!("write_resources");
}

pub fn write_xml(xml: &Xml) -> ResourcesResult<Vec<u8>> {
    let buffer: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(buffer);

    let _ = xml_writer(&mut cursor, xml)?;

    Ok(cursor.into_inner())
}

fn xml_writer<W: Write>(output: &mut W, xml: &Xml) -> Result<usize> {
    const HEADER_SIZE: usize = 8;
    let mut xml_cursor = Cursor::new(Vec::new());

    let mut siz = HEADER_SIZE;
    siz += string_pool_writer(&mut xml_cursor, &xml.string_pool)?;
    if let Some(resource_map) = &xml.xml_resource_map {
        siz += xml_resource_map_writer(&mut xml_cursor, resource_map)?;
    }
    for xml_event in &xml.xml_body {
        siz += xml_body_chunk_writer(&mut xml_cursor, xml_event)?;
    }

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::Xml,
            header_size: HEADER_SIZE,
            chunk_size: siz,
        },
    )?;
    let _ = bytes(output, &xml_cursor.into_inner())?;
    Ok(siz)
}

fn chunk_header_writer<W: Write>(output: &mut W, chunk_header: &ChunkHeader) -> Result<usize> {
    let mut siz = 0;
    siz += le_u16(output, chunk_header.typ.into())?;
    siz += le_u16(output, chunk_header.header_size as u16)?;
    siz += le_u32(output, chunk_header.chunk_size as u32)?;
    Ok(siz)
}

fn string_pool_writer<W: Write>(output: &mut W, string_pool: &StringPool) -> Result<usize> {
    const HEADER_SIZE: usize = 0x1c;
    let mut pool_cursor = Cursor::new(Vec::new());

    // header size + strings offsets + styles offsets
    let mut pool_siz = HEADER_SIZE + string_pool.strings.len() * 4 + string_pool.styles.len() * 4;
    let mut string_offsets = Vec::with_capacity(string_pool.strings.len());
    let mut style_offsets = Vec::with_capacity(string_pool.styles.len());

    let strings_start = pool_siz;
    for string in &string_pool.strings {
        string_offsets.push(pool_siz - strings_start);
        match string.as_ref() {
            UtfString::Utf8 { raw, size, .. } => {
                if !string_pool.utf8 {
                    panic!("expected utf8 flag");
                }
                if *size < 0x80 {
                    pool_siz += le_u8(&mut pool_cursor, *size as u8)?;
                } else {
                    pool_siz += le_u8(&mut pool_cursor, ((*size as u16 >> 8) as u8) | 0x80)?;
                    pool_siz += le_u8(&mut pool_cursor, (*size & 0xff) as u8)?;
                }
                if raw.len() < 0x80 {
                    pool_siz += le_u8(&mut pool_cursor, raw.len() as u8)?;
                } else {
                    pool_siz += le_u8(&mut pool_cursor, ((raw.len() as u16 >> 8) as u8) | 0x80)?;
                    pool_siz += le_u8(&mut pool_cursor, (raw.len() & 0xff) as u8)?;
                }
                for char in raw {
                    pool_siz += le_u8(&mut pool_cursor, *char)?;
                }
                pool_siz += tag(&mut pool_cursor, "\x00")?;
            }
            UtfString::Utf16 { raw, .. } => {
                if string_pool.utf8 {
                    panic!("expected utf16 flag");
                }
                if raw.len() < 0x8000 {
                    pool_siz += le_u16(&mut pool_cursor, raw.len() as u16)?;
                } else {
                    pool_siz +=
                        le_u16(&mut pool_cursor, ((raw.len() as u32 >> 16) as u16) | 0x8000)?;
                    pool_siz += le_u16(&mut pool_cursor, (raw.len() & 0xffff) as u16)?;
                }
                for char in raw {
                    pool_siz += le_u16(&mut pool_cursor, *char)?;
                }
                pool_siz += tag(&mut pool_cursor, "\x00\x00")?;
            }
        }
    }
    let padlen = (4 - (pool_siz & 0x3)) % 4;
    for _ in 0..padlen {
        pool_siz += tag(&mut pool_cursor, "\x00")?;
    }

    let styles_start = pool_siz;
    for style in &string_pool.styles {
        style_offsets.push(pool_siz - styles_start);
        for span in &style.spans {
            pool_siz += le_u32(&mut pool_cursor, span.name)?;
            pool_siz += le_u32(&mut pool_cursor, span.first_char)?;
            pool_siz += le_u32(&mut pool_cursor, span.last_char)?;
        }
        pool_siz += bytes(&mut pool_cursor, &[0xff, 0xff, 0xff, 0xff])?;
    }

    if !string_pool.styles.is_empty() {
        pool_siz += bytes(
            &mut pool_cursor,
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        )?;
    }

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::StringPool,
            header_size: 0x1c,
            chunk_size: pool_siz,
        },
    )?;
    let _ = le_u32(output, string_pool.strings.len() as u32)?;
    let _ = le_u32(output, string_pool.styles.len() as u32)?;
    let mut flags = 0;
    if string_pool.sorted {
        flags |= 1;
    }
    if string_pool.utf8 {
        flags |= 1 << 8;
    }
    let _ = le_u32(output, flags)?;
    let _ = le_u32(output, strings_start as u32)?;
    let _ = le_u32(output, styles_start as u32)?;
    for offset in string_offsets {
        let _ = le_u32(output, offset as u32)?;
    }
    for offset in style_offsets {
        let _ = le_u32(output, offset as u32)?;
    }
    let _ = bytes(output, &pool_cursor.into_inner())?;
    Ok(pool_siz)
}

fn xml_resource_map_writer<W: Write>(
    output: &mut W,
    xml_resource_map: &XmlResourceMap,
) -> Result<usize> {
    const HEADER_SIZE: usize = 0x8;
    let mut resources_cursor = Cursor::new(Vec::new());

    let mut resources_siz = HEADER_SIZE;
    for id in &xml_resource_map.resource_ids {
        resources_siz += le_u32(&mut resources_cursor, *id)?;
    }

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::XmlResourceMap,
            header_size: HEADER_SIZE,
            chunk_size: resources_siz,
        },
    )?;
    let _ = bytes(output, &resources_cursor.into_inner())?;
    Ok(resources_siz)
}

fn xml_body_chunk_writer<W: Write>(output: &mut W, xml_event: &XmlEvent) -> Result<usize> {
    match xml_event {
        XmlEvent::StartNamespace(ns) => {
            xml_namespace_writer(output, ns, ChunkType::XmlStartNamespace)
        }
        XmlEvent::EndNamespace(ns) => xml_namespace_writer(output, ns, ChunkType::XmlEndNamespace),
        XmlEvent::StartElement(elt, attrs) => xml_start_element_writer(output, elt, attrs),
        XmlEvent::EndElement(elt) => xml_end_element_writer(output, elt),
        XmlEvent::Cdata(cdata) => xml_cdata_writer(output, cdata),
    }
}

fn xml_metadata_writer<W: Write>(output: &mut W, metadata: &XmlMetadata) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, metadata.line_number)?;
    siz += le_u32(output, metadata.comment)?;
    Ok(siz)
}

fn xml_namespace_writer<W: Write>(
    output: &mut W,
    namespace: &XmlNamespace,
    tag: ChunkType,
) -> Result<usize> {
    const HEADER_SIZE: usize = 0x10;
    let mut namespace_cursor = Cursor::new(Vec::new());

    let mut namespace_siz = HEADER_SIZE;
    let _ = xml_metadata_writer(&mut namespace_cursor, &namespace.metadata)?;
    namespace_siz += le_u32(&mut namespace_cursor, namespace.prefix.index() as u32)?;
    namespace_siz += le_u32(&mut namespace_cursor, namespace.uri.index() as u32)?;

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: tag,
            header_size: HEADER_SIZE,
            chunk_size: namespace_siz,
        },
    )?;
    let _ = bytes(output, &namespace_cursor.into_inner())?;
    Ok(namespace_siz)
}

fn xml_start_element_writer<W: Write>(
    output: &mut W,
    element: &XmlElement,
    attrs: &XmlElementAttrs,
) -> Result<usize> {
    const HEADER_SIZE: usize = 0x10;
    let mut element_cursor = Cursor::new(Vec::new());

    let mut element_siz = HEADER_SIZE;
    let _ = xml_metadata_writer(&mut element_cursor, &element.metadata)?;
    element_siz += le_u32(
        &mut element_cursor,
        if let Some(ns) = element.ns {
            ns.index() as u32
        } else {
            0xffff_ffff
        },
    )?;
    element_siz += le_u32(&mut element_cursor, element.name.index() as u32)?;

    element_siz += tag(&mut element_cursor, "\x14\x00")?; // attr_start
    element_siz += tag(&mut element_cursor, "\x14\x00")?; // attr_size
    element_siz += le_u16(&mut element_cursor, attrs.attrs.len() as u16)?;
    element_siz += le_u16(&mut element_cursor, attrs.id_index)?;
    element_siz += le_u16(&mut element_cursor, attrs.class_index)?;
    element_siz += le_u16(&mut element_cursor, attrs.style_index)?;

    for attr in &attrs.attrs {
        element_siz += le_u32(
            &mut element_cursor,
            if let Some(ns) = attr.ns {
                ns.index() as u32
            } else {
                0xffff_ffff
            },
        )?;
        element_siz += le_u32(&mut element_cursor, attr.name.index() as u32)?;
        element_siz += le_u32(&mut element_cursor, attr.raw_value)?;
        element_siz += value_writer(&mut element_cursor, attr.typed_value)?;
    }

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::XmlStartElement,
            header_size: HEADER_SIZE,
            chunk_size: element_siz,
        },
    )?;
    let _ = bytes(output, &element_cursor.into_inner())?;
    Ok(element_siz)
}

fn xml_end_element_writer<W: Write>(output: &mut W, element: &XmlElement) -> Result<usize> {
    const HEADER_SIZE: usize = 0x10;
    let mut element_cursor = Cursor::new(Vec::new());

    let mut element_siz = HEADER_SIZE;
    let _ = xml_metadata_writer(&mut element_cursor, &element.metadata)?;
    element_siz += le_u32(
        &mut element_cursor,
        if let Some(ns) = element.ns {
            ns.index() as u32
        } else {
            0xffff_ffff
        },
    )?;
    element_siz += le_u32(&mut element_cursor, element.name.index() as u32)?;

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::XmlEndElement,
            header_size: HEADER_SIZE,
            chunk_size: element_siz,
        },
    )?;
    let _ = bytes(output, &element_cursor.into_inner())?;
    Ok(element_siz)
}

fn xml_cdata_writer<W: Write>(output: &mut W, cdata: &XmlCdata) -> Result<usize> {
    const HEADER_SIZE: usize = 0x10;
    let mut cdata_cursor = Cursor::new(Vec::new());

    let mut cdata_siz = HEADER_SIZE;
    let _ = xml_metadata_writer(&mut cdata_cursor, &cdata.metadata)?;
    cdata_siz += le_u32(&mut cdata_cursor, cdata.data.index() as u32)?;
    cdata_siz += value_writer(&mut cdata_cursor, cdata.value)?;

    let _ = chunk_header_writer(
        output,
        &ChunkHeader {
            typ: ChunkType::XmlCdata,
            header_size: HEADER_SIZE,
            chunk_size: cdata_siz,
        },
    )?;
    let _ = bytes(output, &cdata_cursor.into_inner())?;
    Ok(cdata_siz)
}

fn value_writer<W: Write>(output: &mut W, value: Value) -> Result<usize> {
    let mut value_siz: usize = 0;
    value_siz += tag(output, "\x08\x00")?; // size
    value_siz += tag(output, "\x00")?;
    match value {
        Value::Null => {
            value_siz += le_u8(output, 0x00)?;
            value_siz += le_u32(output, 0)?;
        }
        Value::Reference(data) => {
            value_siz += le_u8(output, 0x01)?;
            value_siz += le_u32(output, data)?;
        }
        Value::Attribute(data) => {
            value_siz += le_u8(output, 0x02)?;
            value_siz += le_u32(output, data)?;
        }
        Value::String(s) => {
            value_siz += le_u8(output, 0x03)?;
            value_siz += le_u32(output, s.index() as u32)?;
        }
        Value::Float(f) => {
            value_siz += le_u8(output, 0x04)?;
            value_siz += le_u32(output, f.to_bits())?;
        }
        Value::Dimension(data) => {
            value_siz += le_u8(output, 0x05)?;
            value_siz += le_u32(output, data)?;
        }
        Value::Fraction(data) => {
            value_siz += le_u8(output, 0x06)?;
            value_siz += le_u32(output, data)?;
        }
        Value::IntDec(i) => {
            value_siz += le_u8(output, 0x10)?;
            value_siz += le_u32(output, i)?;
        }
        Value::IntHex(i) => {
            value_siz += le_u8(output, 0x11)?;
            value_siz += le_u32(output, i)?;
        }
        Value::IntBoolean(b) => {
            value_siz += le_u8(output, 0x12)?;
            value_siz += le_u32(output, u32::from(b))?;
        }
        Value::IntColorARGB8(data) => {
            value_siz += le_u8(output, 0x1c)?;
            value_siz += le_u32(output, data)?;
        }
        Value::IntColorRGB8(data) => {
            value_siz += le_u8(output, 0x1d)?;
            value_siz += le_u32(output, data)?;
        }
        Value::IntColorARGB4(data) => {
            value_siz += le_u8(output, 0x1e)?;
            value_siz += le_u32(output, data)?;
        }
        Value::IntColorRGB4(data) => {
            value_siz += le_u8(output, 0x1f)?;
            value_siz += le_u32(output, data)?;
        }
    }
    Ok(value_siz)
}
