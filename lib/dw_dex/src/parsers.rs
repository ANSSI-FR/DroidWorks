use crate::annotations::*;
use crate::classes::*;
use crate::code::*;
use crate::errors::{DexError, DexResult};
use crate::fields::*;
use crate::hexlify::hexlify;
use crate::instrs::*;
use crate::map::*;
use crate::methods::*;
use crate::registers::*;
use crate::strings::*;
use crate::types::*;
use crate::values::*;
use crate::{Addr, Dex, HeaderItem, Index, Map};
use dw_utils::leb::{Sleb128, Uleb128};
use nom::bits::complete::take as take_bits;
use nom::bits::{bits, bytes};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::character::complete::digit1;
use nom::combinator::{cond, map, value, verify};
use nom::error::{ErrorKind, ParseError};
use nom::multi::count;
use nom::number::complete::*;
use nom::number::Endianness;
use nom::sequence::{pair, preceded, tuple};
use nom::Err::Error;
use nom::Finish;
use nom::{IResult, Offset};
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::sync::RwLock;

// The guideline for dex parsing is to interpret as much things as possible, as long as:
//  - interpretation does not rely on indirection solving (dereferencing DexRef),
//  - size in file is not lost when storing the parsed data (allows for easier dex file
//    regeneration).

const NO_INDEX: u32 = 0xFFFF_FFFF;

/// Dex parsing function, takes input and returns a freshly built [`Dex`] instance.
pub fn parse_dex(input: &[u8]) -> DexResult<Dex> {
    log::trace!("parsing dex...");

    // parsing header
    let (rest, header) = header_item_parser(input).finish()?;
    let mut cursor = input.offset(rest);
    if cursor != 0x70 {
        return Err(DexError::BadSize("header".to_string()));
    }
    log::debug!("Dex version:  {}", header.version);
    log::debug!("Checksum:     {:x}", header.checksum);
    let checksum = adler32::adler32(&input[12..])?;
    if checksum != header.checksum {
        log::warn!("invalid checksum");
        log::warn!("    expected: {:x}", header.checksum);
        log::warn!("    computed: {:x}", checksum);
    }
    log::debug!("Signature:    {}", hexlify(&header.signature));
    let mut hasher = Sha1::new();
    hasher.update(&input[32..]);
    let signature = hasher.finalize();
    if hexlify(&signature) != hexlify(&header.signature) {
        log::warn!("invalid signature");
        log::warn!("    expected: {}", hexlify(&header.signature));
        log::warn!("    computed: {}", hexlify(&signature));
    }
    log::debug!("File size:    {} bytes", header.file_size);
    log::debug!("Endianness:   {:?} endian", header.endianness);
    log::debug!("Map offset:   {}", header.map_off);

    // parsing map_list
    if input.len() < header.map_off {
        return Err(DexError::InvalidOffset("map_list".to_string()));
    }
    let (_, map_list) = map_list_parser(header.map_off, &input[header.map_off..]).finish()?;
    for item in &map_list.list {
        log::debug!(
            "found {}, offset={:#x}, size={:#x}",
            item.typ,
            item.offset,
            item.size
        );
    }

    let mut dex = Dex::new(0);
    dex.header_item = header;
    dex.map_list = map_list;

    // checking for duplicates
    let mut map: BTreeSet<MapItemType> = BTreeSet::new();
    for item in &dex.map_list.list {
        if map.contains(&item.typ) {
            return Err(DexError::Structure("duplicate map entry".to_string()));
        }
        map.insert(item.typ);
    }

    // reverse map_list to pop and check items
    let mut sections: Vec<_> = dex.map_list.list.iter().rev().collect();

    // header already parsed, just checking metadata
    if let Some(section_descr) = sections.pop() {
        if section_descr.typ != MapItemType::HeaderItem {
            return Err(DexError::Structure("HEADER_ITEM expected".to_string()));
        }
        if section_descr.size != 0x1 {
            return Err(DexError::BadSize("header map entry".to_string()));
        }
        if section_descr.offset != 0x0 {
            return Err(DexError::InvalidOffset("header map entry".to_string()));
        }
    } else {
        return Err(DexError::Structure("HEADER_ITEM expected".to_string()));
    }

    // parsing ordered 'core' sections
    if dex.header_item.string_ids_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.string_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::StringIdItem,
                    size: dex.header_item.string_ids_size,
                    offset: dex.header_item.string_ids_off,
                },
                0x4,
                string_id_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("STRING_ID_ITEM expected".to_string()));
        }
    }
    if dex.header_item.type_ids_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.type_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::TypeIdItem,
                    size: dex.header_item.type_ids_size,
                    offset: dex.header_item.type_ids_off,
                },
                0x4,
                type_id_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("TYPE_ID_ITEM expected".to_string()));
        }
    }
    if dex.header_item.proto_ids_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.proto_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::ProtoIdItem,
                    size: dex.header_item.proto_ids_size,
                    offset: dex.header_item.proto_ids_off,
                },
                0xc,
                proto_id_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("PROTO_ID_ITEM expected".to_string()));
        }
    }
    if dex.header_item.field_ids_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.field_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::FieldIdItem,
                    size: dex.header_item.field_ids_size,
                    offset: dex.header_item.field_ids_off,
                },
                0x8,
                field_id_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("FIELD_ID_ITEM expected".to_string()));
        }
    }
    if dex.header_item.method_ids_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.method_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::MethodIdItem,
                    size: dex.header_item.method_ids_size,
                    offset: dex.header_item.method_ids_off,
                },
                0x8,
                method_id_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("METHOD_ID_ITEM expected".to_string()));
        }
    }
    if dex.header_item.class_defs_size != 0 {
        if let Some(section_descr) = sections.pop() {
            dex.class_def_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                &MapItem {
                    typ: MapItemType::ClassDefItem,
                    size: dex.header_item.class_defs_size,
                    offset: dex.header_item.class_defs_off,
                },
                0x20,
                class_def_item_parser,
            )?;
        } else {
            return Err(DexError::Structure("CLASS_DEF_ITEM expected".to_string()));
        }
    }
    match sections.pop() {
        Some(section_descr) if section_descr.typ == MapItemType::CallSiteIdItem => {
            dex.call_site_id_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                section_descr, // no header descr to compare
                0x4,
                call_site_id_item_parser,
            )?;
        }
        Some(section_descr) if section_descr.typ == MapItemType::MethodHandleItem => {
            dex.method_handle_items = parse_core_section(
                input,
                &mut cursor,
                section_descr,
                section_descr, // no header descr to compare
                0x8,
                method_handle_item_parser,
            )?;
        }
        Some(section_descr) => sections.push(section_descr),
        _ => (),
    }

    // 'data' frontier customs control
    if cursor > dex.header_item.data_off {
        return Err(DexError::InvalidOffset("data section".to_string()));
    }
    while cursor != dex.header_item.data_off {
        if input[cursor] != 0x0 {
            return Err(DexError::NonZeroPadding);
        }
        cursor += 1;
    }
    if input.len() != cursor + dex.header_item.data_size {
        return Err(DexError::BadSize("data section".to_string()));
    }

    // parsing unordered 'data' sections
    while let Some(section_descr) = sections.pop() {
        match section_descr.typ {
            MapItemType::MapList => {
                // drop the result, map_list already parsed
                let _already_parsed =
                    parse_data_section(input, &mut cursor, section_descr, false, map_list_parser)?;
            }
            MapItemType::TypeList => {
                dex.type_lists =
                    parse_data_section(input, &mut cursor, section_descr, true, type_list_parser)?;
            }
            MapItemType::AnnotationSetRefList => {
                dex.annotation_set_ref_lists = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    true,
                    annotation_set_ref_list_parser,
                )?;
            }
            MapItemType::AnnotationSetItem => {
                dex.annotation_set_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    true,
                    annotation_set_item_parser,
                )?;
            }
            MapItemType::ClassDataItem => {
                dex.class_data_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    class_data_item_parser,
                )?;
            }
            MapItemType::CodeItem => {
                dex.code_items =
                    parse_data_section(input, &mut cursor, section_descr, true, code_item_parser)?;
            }
            MapItemType::StringDataItem => {
                dex.string_data_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    string_data_item_parser,
                )?;
            }
            MapItemType::DebugInfoItem => {
                dex.debug_info_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    debug_info_item_parser,
                )?;
            }
            MapItemType::AnnotationItem => {
                dex.annotation_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    annotation_item_parser,
                )?;
            }
            MapItemType::EncodedArrayItem => {
                dex.encoded_array_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    encoded_array_item_parser,
                )?;
            }
            MapItemType::AnnotationsDirectoryItem => {
                dex.annotations_directory_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    true,
                    annotations_directory_item_parser,
                )?;
            }
            MapItemType::HiddenapiClassDataItem => {
                dex.hiddenapi_class_data_items = parse_data_section(
                    input,
                    &mut cursor,
                    section_descr,
                    false,
                    hiddenapi_class_data_item_parser,
                )?;
            }
            _ => return Err(DexError::InvalidType),
        }
    }

    // final padding
    while cursor < input.len() {
        if input[cursor] != 0x0 {
            return Err(DexError::NonZeroPadding);
        }
        cursor += 1;
    }

    Ok(dex)
}

fn parse_core_section<'a, T>(
    input: &'a [u8],
    cursor: &mut usize,
    section_descr: &MapItem,
    header_descr: &MapItem,
    elt_size: usize,
    parser: impl Fn(usize, &'a [u8]) -> IResult<&[u8], T, DexError>,
) -> DexResult<Vec<T>> {
    if section_descr.typ != header_descr.typ {
        return Err(DexError::InvalidType);
    }
    if section_descr.size != header_descr.size {
        return Err(DexError::BadSize(format!("{} section", section_descr.typ,)));
    }
    if section_descr.offset != header_descr.offset {
        return Err(DexError::InvalidOffset(format!(
            "{} section",
            section_descr.typ,
        )));
    }
    if section_descr.offset >= input.len() {
        return Err(DexError::InvalidOffset(format!(
            "{} section",
            section_descr.typ,
        )));
    }
    if *cursor > section_descr.offset {
        return Err(DexError::InvalidOffset(format!(
            "{} section",
            section_descr.typ,
        )));
    }
    while *cursor != section_descr.offset {
        if input[*cursor] != 0x0 {
            return Err(DexError::NonZeroPadding);
        }
        *cursor += 1;
    }

    let mut slice = &input[section_descr.offset..];
    let mut items = Vec::new();
    for i in 0..section_descr.size {
        let (rest, item) = parser(i, slice).finish()?;
        items.push(item);
        slice = rest;
    }

    let rest = slice;
    *cursor = input.offset(rest);
    if *cursor != section_descr.offset + section_descr.size * elt_size {
        return Err(DexError::BadSize(format!("{} section", section_descr.typ,)));
    }
    Ok(items)
}

fn parse_data_section<'a, T>(
    input: &'a [u8],
    cursor: &mut usize,
    section_descr: &MapItem,
    align: bool,
    parser: impl Fn(usize, &'a [u8]) -> IResult<&'a [u8], T, DexError>,
) -> DexResult<Map<T>> {
    let mut items = Map::new();
    if *cursor > section_descr.offset {
        return Err(DexError::InvalidOffset(format!(
            "{} section",
            section_descr.typ,
        )));
    }
    while *cursor != section_descr.offset {
        if input[*cursor] != 0x0 {
            return Err(DexError::NonZeroPadding);
        }
        *cursor += 1;
    }
    for _ in 0..section_descr.size {
        if align {
            let alignment = (4 - (*cursor % 4)) % 4;
            for _ in 0..alignment {
                if input[*cursor] != 0x0 {
                    return Err(DexError::NonZeroPadding);
                }
                *cursor += 1;
            }
        }
        if *cursor >= input.len() {
            return Err(DexError::InvalidOffset(format!(
                "{} section",
                section_descr.typ,
            )));
        }
        let (rest, item) = parser(*cursor, &input[*cursor..]).finish()?;
        items.insert(*cursor, item);
        *cursor = input.offset(rest);
    }
    Ok(items)
}

fn magic_parser(input: &[u8]) -> IResult<&[u8], u32, DexError> {
    let (input, _) = tag("dex\n")(input)?;
    let (input, v) = map(verify(digit1, |ds: &[u8]| ds.len() == 3), |vs: &[u8]| {
        u32::from(vs[0] - 0x30) * 100 + u32::from(vs[1] - 0x30) * 10 + u32::from(vs[2] - 0x30)
    })(input)?;
    let (input, _) = tag("\x00")(input)?;
    Ok((input, v))
}

fn endian_tag_parser(input: &[u8]) -> IResult<&[u8], Endianness, DexError> {
    alt((
        map(tag([0x78, 0x56, 0x34, 0x12]), |_| Endianness::Little),
        map(tag([0x12, 0x34, 0x56, 0x78]), |_| Endianness::Big),
    ))(input)
}

fn header_item_parser(input: &[u8]) -> IResult<&[u8], HeaderItem, DexError> {
    let (input, version) = magic_parser(input)?;
    let (input, checksum) = le_u32(input)?;
    let (input, signature) = count(le_u8, 20)(input)?;
    let (input, file_size) = le_u32(input)?;
    let (input, _header_size) = verify(le_u32, |siz| *siz == 0x70)(input)?;
    let (input, endianness) = endian_tag_parser(input)?;
    let (input, link_size) = le_u32(input)?;
    let (input, link_off) = le_u32(input)?;
    let (input, map_off) = le_u32(input)?;
    let (input, string_ids_size) = le_u32(input)?;
    let (input, string_ids_off) = le_u32(input)?;
    let (input, type_ids_size) = le_u32(input)?;
    let (input, type_ids_off) = le_u32(input)?;
    let (input, proto_ids_size) = le_u32(input)?;
    let (input, proto_ids_off) = le_u32(input)?;
    let (input, field_ids_size) = le_u32(input)?;
    let (input, field_ids_off) = le_u32(input)?;
    let (input, method_ids_size) = le_u32(input)?;
    let (input, method_ids_off) = le_u32(input)?;
    let (input, class_defs_size) = le_u32(input)?;
    let (input, class_defs_off) = le_u32(input)?;
    let (input, data_size) = le_u32(input)?;
    let (input, data_off) = le_u32(input)?;

    if link_size != 0 {
        log::warn!("dex has a non-null link size");
    }

    Ok((
        input,
        HeaderItem {
            version,
            checksum,
            signature,
            file_size: file_size as usize,
            endianness,
            _link_size: link_size as usize,
            _link_off: link_off as usize,
            map_off: map_off as usize,
            string_ids_size: string_ids_size as usize,
            string_ids_off: string_ids_off as usize,
            type_ids_size: type_ids_size as usize,
            type_ids_off: type_ids_off as usize,
            proto_ids_size: proto_ids_size as usize,
            proto_ids_off: proto_ids_off as usize,
            field_ids_size: field_ids_size as usize,
            field_ids_off: field_ids_off as usize,
            method_ids_size: method_ids_size as usize,
            method_ids_off: method_ids_off as usize,
            class_defs_size: class_defs_size as usize,
            class_defs_off: class_defs_off as usize,
            data_size: data_size as usize,
            data_off: data_off as usize,
        },
    ))
}

fn string_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], StringIdItem, DexError> {
    map(le_u32, |string_data_off| StringIdItem {
        index: Index::new(idx),
        string_data_off: Index::new(string_data_off as usize),
    })(input)
}

fn type_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], TypeIdItem, DexError> {
    map(le_u32, |descriptor_idx| TypeIdItem {
        index: Index::new(idx),
        descriptor_idx: Index::new(descriptor_idx as usize),
    })(input)
}

fn proto_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], ProtoIdItem, DexError> {
    let (input, shorty_idx) = le_u32(input)?;
    let (input, return_type_idx) = le_u32(input)?;
    let (input, parameters_off) = le_u32(input)?;
    let parameters_off = if parameters_off == 0 {
        None
    } else {
        Some(Index::new(parameters_off as usize))
    };

    Ok((
        input,
        ProtoIdItem {
            index: Index::new(idx),
            shorty_idx: Index::new(shorty_idx as usize),
            return_type_idx: Index::new(return_type_idx as usize),
            parameters_off,
        },
    ))
}

fn field_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], FieldIdItem, DexError> {
    let (input, class_idx) = le_u16(input)?;
    let (input, type_idx) = le_u16(input)?;
    let (input, name_idx) = le_u32(input)?;

    Ok((
        input,
        FieldIdItem {
            index: Index::new(idx),
            class_idx: Index::new(class_idx as usize),
            type_idx: Index::new(type_idx as usize),
            name_idx: Index::new(name_idx as usize),
        },
    ))
}

fn method_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], MethodIdItem, DexError> {
    let (input, class_idx) = le_u16(input)?;
    let (input, proto_idx) = le_u16(input)?;
    let (input, name_idx) = le_u32(input)?;

    Ok((
        input,
        MethodIdItem {
            index: Index::new(idx),
            class_idx: Index::new(class_idx as usize),
            proto_idx: Index::new(proto_idx as usize),
            name_idx: Index::new(name_idx as usize),
        },
    ))
}

fn class_def_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], ClassDefItem, DexError> {
    let (input, class_idx) = le_u32(input)?;
    let (input, access_flags) = le_u32(input)?;
    let (input, superclass_idx) = le_u32(input)?;
    let (input, interfaces_off) = le_u32(input)?;
    let (input, source_file_idx) = le_u32(input)?;
    let (input, annotations_off) = le_u32(input)?;
    let (input, class_data_off) = le_u32(input)?;
    let (input, static_values_off) = le_u32(input)?;

    let access_flags = ClassFlags::from_bits(access_flags)
        .ok_or_else(|| Error(DexError::from_error_kind(input, ErrorKind::TagBits)))?;
    let superclass_idx = if superclass_idx == NO_INDEX {
        None
    } else {
        Some(Index::new(superclass_idx as usize))
    };
    let interfaces_off = if interfaces_off == 0 {
        None
    } else {
        Some(Index::new(interfaces_off as usize))
    };
    let source_file_idx = if source_file_idx == NO_INDEX {
        None
    } else {
        Some(Index::new(source_file_idx as usize))
    };
    let annotations_off = if annotations_off == 0 {
        None
    } else {
        Some(Index::new(annotations_off as usize))
    };
    let class_data_off = if class_data_off == 0 {
        None
    } else {
        Some(Index::new(class_data_off as usize))
    };
    let static_values_off = if static_values_off == 0 {
        None
    } else {
        Some(Index::new(static_values_off as usize))
    };

    Ok((
        input,
        ClassDefItem {
            index: Index::new(idx),
            class_idx: Index::new(class_idx as usize),
            access_flags,
            superclass_idx,
            interfaces_off,
            source_file_idx,
            annotations_off,
            class_data_off,
            static_values_off,
        },
    ))
}

fn call_site_id_item_parser(idx: usize, input: &[u8]) -> IResult<&[u8], CallSiteIdItem, DexError> {
    map(le_u32, |call_site_off| CallSiteIdItem {
        index: Index::new(idx),
        call_site_off: Index::new(call_site_off as usize),
    })(input)
}

fn method_handle_item_parser(
    idx: usize,
    input: &[u8],
) -> IResult<&[u8], MethodHandleItem, DexError> {
    let (input, method_handle_type) = le_u16(input)?;
    let (input, _unused1) = le_u16(input)?;
    let (input, field_or_method_id) = le_u16(input)?;
    let (input, _unused2) = le_u16(input)?;

    let field_id = Index::new(field_or_method_id as usize);
    let method_id = Index::new(field_or_method_id as usize);

    let method_handle = match method_handle_type {
        0x00 => Ok(MethodHandle::StaticPut(field_id)),
        0x01 => Ok(MethodHandle::StaticGet(field_id)),
        0x02 => Ok(MethodHandle::InstancePut(field_id)),
        0x03 => Ok(MethodHandle::InstanceGet(field_id)),
        0x04 => Ok(MethodHandle::InvokeStatic(method_id)),
        0x05 => Ok(MethodHandle::InvokeInstance(method_id)),
        0x06 => Ok(MethodHandle::InvokeConstructor(method_id)),
        0x07 => Ok(MethodHandle::InvokeDirect(method_id)),
        0x08 => Ok(MethodHandle::InvokeInterface(method_id)),
        _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Switch))),
    }?;

    Ok((
        input,
        MethodHandleItem {
            index: Index::new(idx),
            method_handle,
        },
    ))
}

fn map_list_parser(_offset: usize, input: &[u8]) -> IResult<&[u8], MapList, DexError> {
    let (input, nb) = le_u32(input)?;
    map(count(map_item_parser, nb as usize), |list| MapList { list })(input)
}

fn map_item_parser(input: &[u8]) -> IResult<&[u8], MapItem, DexError> {
    let (input, typ_tag) = le_u16(input)?;
    let typ = MapItemType::try_from(typ_tag).map_err(Error)?;
    let (input, _unused) = le_u16(input)?;
    let (input, size) = le_u32(input)?;
    let (input, offset) = le_u32(input)?;

    Ok((
        input,
        MapItem {
            typ,
            size: size as usize,
            offset: offset as usize,
        },
    ))
}

fn type_list_parser(offset: usize, input: &[u8]) -> IResult<&[u8], TypeList, DexError> {
    let (input, nb) = le_u32(input)?;
    let (input, list) = count(type_item_parser, nb as usize)(input)?;

    Ok((
        input,
        TypeList {
            index: Index::new(offset),
            list,
        },
    ))
}

fn type_item_parser(input: &[u8]) -> IResult<&[u8], TypeItem, DexError> {
    map(le_u16, |type_idx| TypeItem {
        type_idx: Index::new(type_idx as usize),
    })(input)
}

fn annotation_set_ref_list_parser(
    offset: usize,
    input: &[u8],
) -> IResult<&[u8], AnnotationSetRefList, DexError> {
    let (input, nb) = le_u32(input)?;
    map(count(annotation_set_ref_item_parser, nb as usize), |list| {
        AnnotationSetRefList {
            index: Index::new(offset),
            list,
        }
    })(input)
}

fn annotation_set_ref_item_parser(input: &[u8]) -> IResult<&[u8], AnnotationSetRefItem, DexError> {
    map(le_u32, |annotations_off| AnnotationSetRefItem {
        annotations_off: Index::new(annotations_off as usize),
    })(input)
}

fn annotation_set_item_parser(
    offset: usize,
    input: &[u8],
) -> IResult<&[u8], AnnotationSetItem, DexError> {
    let (input, nb) = le_u32(input)?;
    map(count(annotation_off_item_parser, nb as usize), |entries| {
        AnnotationSetItem {
            index: Index::new(offset),
            entries,
        }
    })(input)
}

fn annotation_off_item_parser(input: &[u8]) -> IResult<&[u8], AnnotationOffItem, DexError> {
    map(le_u32, |annotation_off| AnnotationOffItem {
        annotation_off: Index::new(annotation_off as usize),
    })(input)
}

fn class_data_item_parser(offset: usize, input: &[u8]) -> IResult<&[u8], ClassDataItem, DexError> {
    let (input, static_fields_size) = uleb128(input)?;
    let (input, instance_fields_size) = uleb128(input)?;
    let (input, direct_methods_size) = uleb128(input)?;
    let (input, virtual_methods_size) = uleb128(input)?;

    let mut idx: usize = 0;
    let mut input = input;

    let mut static_fields = Vec::new();
    for _ in 0..static_fields_size.value() {
        let (inp, (f, i)) = encoded_field_parser(idx)(input)?;
        static_fields.push(f);
        idx = i;
        input = inp;
    }

    idx = 0;
    let mut instance_fields = Vec::new();
    for _ in 0..instance_fields_size.value() {
        let (inp, (f, i)) = encoded_field_parser(idx)(input)?;
        instance_fields.push(f);
        idx = i;
        input = inp;
    }

    idx = 0;
    let mut direct_methods = Vec::new();
    for _ in 0..direct_methods_size.value() {
        let (inp, (m, i)) = encoded_method_parser(idx)(input)?;
        direct_methods.push(m);
        idx = i;
        input = inp;
    }

    idx = 0;
    let mut virtual_methods = Vec::new();
    for _ in 0..virtual_methods_size.value() {
        let (inp, (m, i)) = encoded_method_parser(idx)(input)?;
        virtual_methods.push(m);
        idx = i;
        input = inp;
    }

    Ok((
        input,
        ClassDataItem {
            index: Index::new(offset),
            static_fields_size,
            instance_fields_size,
            direct_methods_size,
            virtual_methods_size,
            static_fields,
            instance_fields,
            direct_methods,
            virtual_methods,
        },
    ))
}

fn encoded_field_parser<'a>(
    base_idx: usize,
) -> impl Fn(&'a [u8]) -> IResult<&[u8], (EncodedField, usize), DexError> {
    move |input: &[u8]| {
        let (input, field_idx_diff) = uleb128(input)?;
        let (input, access_flags_repr) = uleb128(input)?;

        let access_flags = FieldFlags::from_bits(access_flags_repr.value())
            .ok_or_else(|| Error(DexError::from_error_kind(input, ErrorKind::TagBits)))?;
        let idx = base_idx + field_idx_diff.value() as usize;

        Ok((
            input,
            (
                EncodedField {
                    field_idx_diff,
                    field_idx: Index::new(idx),
                    access_flags_repr,
                    access_flags,
                },
                idx,
            ),
        ))
    }
}

fn encoded_method_parser<'a>(
    base_idx: usize,
) -> impl Fn(&'a [u8]) -> IResult<&[u8], (EncodedMethod, usize), DexError> {
    move |input: &[u8]| {
        let (input, method_idx_diff) = uleb128(input)?;
        let (input, access_flags_repr) = uleb128(input)?;
        let (input, code_off) = uleb128(input)?;

        let access_flags = MethodFlags::from_bits(access_flags_repr.value())
            .ok_or_else(|| Error(DexError::from_error_kind(input, ErrorKind::TagBits)))?;
        let code_off = if code_off.value() == 0 {
            None
        } else {
            Some(Index::new_uleb(code_off))
        };
        let idx = base_idx + method_idx_diff.value() as usize;

        Ok((
            input,
            (
                EncodedMethod {
                    method_idx_diff,
                    method_idx: Index::new(idx),
                    access_flags_repr,
                    access_flags,
                    code_off,
                },
                idx,
            ),
        ))
    }
}

fn code_item_parser(offset: usize, input: &[u8]) -> IResult<&[u8], RwLock<CodeItem>, DexError> {
    let (input, registers_size) = le_u16(input)?;
    let (input, ins_size) = le_u16(input)?;
    let (input, outs_size) = le_u16(input)?;
    let (input, tries_size) = le_u16(input)?;
    let (input, debug_info_off) = le_u32(input)?;
    let (input, insns_size) = le_u32(input)?;

    if input.len() < insns_size as usize * 2 {
        return Err(Error(DexError::from_error_kind(input, ErrorKind::Complete)));
    }

    let mut insns = Vec::new();
    let mut addr = 0;
    let mut insns_buffer = &input[..insns_size as usize * 2];
    while !insns_buffer.is_empty() {
        let (rest, instr) = parse_instr(insns_buffer)?;
        let size = instr.size();
        insns.push(LabeledInstr {
            addr: Addr(addr),
            instr,
        });
        addr += size;
        insns_buffer = rest;
    }

    let (input, _) = cond(tries_size != 0 && insns_size % 2 == 1, tag("\x00\x00"))(
        &input[insns_size as usize * 2..],
    )?;

    let (input, tries) = count(try_item_parser, tries_size as usize)(input)?;
    let (input, handlers) = cond(tries_size != 0, encoded_catch_handler_list_parser)(input)?;

    let debug_info_off = if debug_info_off == 0 {
        None
    } else {
        Some(Index::new(debug_info_off as usize))
    };

    Ok((
        input,
        RwLock::new(CodeItem {
            index: Index::new(offset),
            registers_size: registers_size as usize,
            ins_size: ins_size as usize,
            outs_size: outs_size as usize,
            debug_info_off,
            insns,
            tries,
            handlers,
        }),
    ))
}

fn try_item_parser(input: &[u8]) -> IResult<&[u8], TryItem, DexError> {
    let (input, start_addr) = le_u32(input)?;
    let (input, insn_count) = le_u16(input)?;
    let (input, handler_off) = le_u16(input)?;

    Ok((
        input,
        TryItem {
            start_addr: start_addr as usize,
            insn_count: insn_count as usize,
            handler_off: handler_off as usize,
        },
    ))
}

fn encoded_catch_handler_list_parser(
    input: &[u8],
) -> IResult<&[u8], EncodedCatchHandlerList, DexError> {
    let (i, nb) = uleb128(input)?;

    let mut current = i;
    let mut list = Map::new();
    for _ in 0..nb.value() as usize {
        let offset = input.offset(current);
        let (i, elt) = encoded_catch_handler_parser(current)?;
        current = i;
        list.insert(offset, elt);
    }

    Ok((current, EncodedCatchHandlerList { size: nb, list }))
}

fn encoded_catch_handler_parser(input: &[u8]) -> IResult<&[u8], EncodedCatchHandler, DexError> {
    let (input, size) = sleb128(input)?;
    let (input, handlers) = count(
        encoded_type_addr_pair_parser,
        size.value().unsigned_abs() as usize,
    )(input)?;
    let (input, catch_all_addr) = cond(size.value() <= 0, uleb128)(input)?;

    Ok((
        input,
        EncodedCatchHandler {
            size,
            handlers,
            catch_all_addr,
        },
    ))
}

fn encoded_type_addr_pair_parser(input: &[u8]) -> IResult<&[u8], EncodedTypeAddrPair, DexError> {
    let (input, type_idx) = uleb128(input)?;
    let (input, addr) = uleb128(input)?;

    Ok((
        input,
        EncodedTypeAddrPair {
            type_idx: Index::new_uleb(type_idx),
            addr,
        },
    ))
}

fn string_data_item_parser(
    offset: usize,
    input: &[u8],
) -> IResult<&[u8], StringDataItem, DexError> {
    let (input, utf16_size) = uleb128(input)?;
    let (input, data) = take_until("\x00")(input)?;
    let (input, _null) = tag("\x00")(input)?;

    Ok((
        input,
        StringDataItem {
            index: Index::new(offset),
            utf16_size,
            data: data.to_vec(),
        },
    ))
}

fn debug_info_item_parser(offset: usize, input: &[u8]) -> IResult<&[u8], DebugInfoItem, DexError> {
    let (input, line_start) = uleb128(input)?;
    let (input, parameters_size) = uleb128(input)?;
    let (input, parameter_names) = count(
        map(uleb128p1, |n_opt| n_opt.map(|n| Index::new(n as usize))),
        parameters_size.value() as usize,
    )(input)?;
    let (input, bytecode) = debug_bytecode_parser(input)?;

    Ok((
        input,
        DebugInfoItem {
            index: Index::new(offset),
            line_start,
            parameters_size,
            parameter_names,
            bytecode,
        },
    ))
}

fn debug_bytecode_parser(input: &[u8]) -> IResult<&[u8], Vec<DbgInstr>, DexError> {
    let mut inp = input;
    let mut bc = Vec::new();
    loop {
        let opcode = inp[0];
        inp = &inp[1..];
        match opcode {
            0x00 => {
                bc.push(DbgInstr::EndSequence);
                break;
            }
            0x01 => {
                let (i, addr_diff) = uleb128(inp)?;
                bc.push(DbgInstr::AdvancePc { addr_diff });
                inp = i;
            }
            0x02 => {
                let (i, line_diff) = sleb128(inp)?;
                bc.push(DbgInstr::AdvanceLine { line_diff });
                inp = i;
            }
            0x03 => {
                let (i, register_num) = uleb128(inp)?;
                let (i, name_idx) = uleb128p1(i)?;
                let (i, type_idx) = uleb128p1(i)?;
                bc.push(DbgInstr::StartLocal {
                    register_num,
                    name_idx: name_idx.map(|v| Index::new(v as usize)),
                    type_idx: type_idx.map(|v| Index::new(v as usize)),
                });
                inp = i;
            }
            0x04 => {
                let (i, register_num) = uleb128(inp)?;
                let (i, name_idx) = uleb128p1(i)?;
                let (i, type_idx) = uleb128p1(i)?;
                let (i, sig_idx) = uleb128p1(i)?;
                bc.push(DbgInstr::StartLocalExtended {
                    register_num,
                    name_idx: name_idx.map(|v| Index::new(v as usize)),
                    type_idx: type_idx.map(|v| Index::new(v as usize)),
                    sig_idx: sig_idx.map(|v| Index::new(v as usize)),
                });
                inp = i;
            }
            0x05 => {
                let (i, register_num) = uleb128(inp)?;
                bc.push(DbgInstr::EndLocal { register_num });
                inp = i;
            }
            0x06 => {
                let (i, register_num) = uleb128(inp)?;
                bc.push(DbgInstr::RestartLocal { register_num });
                inp = i;
            }
            0x07 => bc.push(DbgInstr::SetPrologueEnd),
            0x08 => bc.push(DbgInstr::SetEpilogueBegin),
            0x09 => {
                let (i, name_idx) = uleb128p1(inp)?;
                bc.push(DbgInstr::SetFile {
                    name_idx: name_idx.map(|v| Index::new(v as usize)),
                });
                inp = i;
            }
            _ => bc.push(DbgInstr::Special(opcode)),
        }
    }
    Ok((inp, bc))
}

fn annotation_item_parser(offset: usize, input: &[u8]) -> IResult<&[u8], AnnotationItem, DexError> {
    let (input, visibility) = le_u8(input)?;
    let (input, annotation) = encoded_annotation_parser(input)?;

    let visibility = match visibility {
        0x00 => Ok(Visibility::Build),
        0x01 => Ok(Visibility::Runtime),
        0x02 => Ok(Visibility::System),
        _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Switch))),
    }?;

    Ok((
        input,
        AnnotationItem {
            index: Index::new(offset),
            visibility,
            annotation,
        },
    ))
}

fn encoded_annotation_parser(input: &[u8]) -> IResult<&[u8], EncodedAnnotation, DexError> {
    let (input, type_idx) = uleb128(input)?;
    let (input, size) = uleb128(input)?;
    let (input, elements) = count(annotation_element_parser, size.value() as usize)(input)?;

    Ok((
        input,
        EncodedAnnotation {
            type_idx: Index::new_uleb(type_idx),
            size,
            elements,
        },
    ))
}

fn annotation_element_parser(input: &[u8]) -> IResult<&[u8], AnnotationElement, DexError> {
    let (input, name_idx) = uleb128(input)?;
    let (input, value) = encoded_value_parser(input)?;

    Ok((
        input,
        AnnotationElement {
            name_idx: Index::new_uleb(name_idx),
            value,
        },
    ))
}

fn encoded_value_parser(input: &[u8]) -> IResult<&[u8], EncodedValue, DexError> {
    let (input, tag) = le_u8(input)?;
    let value_arg = (tag & 0b1110_0000) >> 5;
    let value_typ = tag & 0b11111;
    let value_siz = value_arg as usize + 1;

    match value_typ {
        0x00 => {
            if value_arg != 0 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_i8, EncodedValue::Byte)(input)
        }
        0x02 => {
            if value_arg > 1 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_i16_on(value_siz), |value| {
                EncodedValue::Short(value_siz, value)
            })(input)
        }
        0x03 => {
            if value_arg > 1 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u16_on(value_siz), |value| {
                EncodedValue::Char(value_siz, value)
            })(input)
        }
        0x04 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_i32_on(value_siz), |value| {
                EncodedValue::Int(value_siz, value)
            })(input)
        }
        0x06 => {
            if value_arg > 7 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_i64_on(value_siz), |value| {
                EncodedValue::Long(value_siz, value)
            })(input)
        }
        0x10 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_f32_on(value_siz), |value| {
                EncodedValue::Float(value_siz, value)
            })(input)
        }
        0x11 => {
            if value_arg > 7 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_f64_on(value_siz), |value| {
                EncodedValue::Double(value_siz, value)
            })(input)
        }
        0x15 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::MethodType(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x16 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::MethodHandle(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x17 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::String(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x18 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::Type(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x19 => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::Field(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x1a => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::Method(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x1b => {
            if value_arg > 3 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(le_u32_on(value_siz), |idx| {
                EncodedValue::Enum(value_siz, Index::new(idx as usize))
            })(input)
        }
        0x1c => {
            if value_arg != 0 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(encoded_array_parser, EncodedValue::Array)(input)
        }
        0x1d => {
            if value_arg != 0 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            map(encoded_annotation_parser, EncodedValue::Annotation)(input)
        }
        0x1e => {
            if value_arg != 0 {
                return Err(Error(DexError::from_error_kind(input, ErrorKind::Tag)));
            }
            Ok((input, EncodedValue::Null))
        }
        0x1f => match value_arg {
            0 => Ok((input, EncodedValue::Boolean(false))),
            1 => Ok((input, EncodedValue::Boolean(true))),
            _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Switch))),
        },
        _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Switch))),
    }
}

fn encoded_array_item_parser(
    offset: usize,
    input: &[u8],
) -> IResult<&[u8], EncodedArrayItem, DexError> {
    map(encoded_array_parser, |value| EncodedArrayItem {
        index: Index::new(offset),
        value,
    })(input)
}

fn encoded_array_parser(input: &[u8]) -> IResult<&[u8], EncodedArray, DexError> {
    let (input, nb) = uleb128(input)?;
    map(
        count(encoded_value_parser, nb.value() as usize),
        move |values| EncodedArray { size: nb, values },
    )(input)
}

fn annotations_directory_item_parser(
    offset: usize,
    input: &[u8],
) -> IResult<&[u8], AnnotationsDirectoryItem, DexError> {
    let (input, class_annotations_off) = le_u32(input)?;
    let (input, fields_size) = le_u32(input)?;
    let (input, annotated_methods_size) = le_u32(input)?;
    let (input, annotated_parameters_size) = le_u32(input)?;
    let (input, field_annotations) = count(field_annotation_parser, fields_size as usize)(input)?;
    let (input, method_annotations) =
        count(method_annotation_parser, annotated_methods_size as usize)(input)?;
    let (input, parameter_annotations) = count(
        parameter_annotation_parser,
        annotated_parameters_size as usize,
    )(input)?;

    Ok((
        input,
        AnnotationsDirectoryItem {
            index: Index::new(offset),
            class_annotations_off: Index::new(class_annotations_off as usize),
            field_annotations,
            method_annotations,
            parameter_annotations,
        },
    ))
}

fn field_annotation_parser(input: &[u8]) -> IResult<&[u8], FieldAnnotation, DexError> {
    let (input, field_idx) = le_u32(input)?;
    let (input, annotations_off) = le_u32(input)?;

    Ok((
        input,
        FieldAnnotation {
            field_idx: Index::new(field_idx as usize),
            annotations_off: Index::new(annotations_off as usize),
        },
    ))
}

fn method_annotation_parser(input: &[u8]) -> IResult<&[u8], MethodAnnotation, DexError> {
    let (input, method_idx) = le_u32(input)?;
    let (input, annotations_off) = le_u32(input)?;

    Ok((
        input,
        MethodAnnotation {
            method_idx: Index::new(method_idx as usize),
            annotations_off: Index::new(annotations_off as usize),
        },
    ))
}

fn parameter_annotation_parser(input: &[u8]) -> IResult<&[u8], ParameterAnnotation, DexError> {
    let (input, method_idx) = le_u32(input)?;
    let (input, annotations_off) = le_u32(input)?;

    Ok((
        input,
        ParameterAnnotation {
            method_idx: Index::new(method_idx as usize),
            annotations_off: Index::new(annotations_off as usize),
        },
    ))
}

fn hiddenapi_class_data_item_parser(
    _offset: usize,
    input: &[u8],
) -> IResult<&[u8], HiddenapiClassDataItem, DexError> {
    let start = input;
    let (input, size) = le_u32(input)?;

    let mut offsets = Vec::new();
    let mut min_flag_offset = std::u32::MAX;
    let mut input = input;
    loop {
        let off = input.offset(start);
        if off == min_flag_offset as usize {
            break;
        }
        let (inp, offset) = le_u32(input)?;
        offsets.push(Index::new(offset as usize));
        input = inp;
        if offset != 0 && offset < min_flag_offset {
            min_flag_offset = offset;
        }
    }

    let mut flags = Map::new();
    loop {
        let off = input.offset(start);
        if off == size as usize {
            break;
        }
        let (inp, uleb_repr) = uleb128(input)?;
        let flag = HiddenapiFlag::try_from(uleb_repr.value()).map_err(Error)?;
        flags.insert(off, HiddenapiClassFlag { uleb_repr, flag });
        input = inp;
    }

    Ok((input, HiddenapiClassDataItem { offsets, flags }))
}

fn parse_instr(input: &[u8]) -> IResult<&[u8], Instr, DexError> {
    let (input, mnemonic) = le_u8(input)?;
    match mnemonic {
        0x00 => parse_pseudo_instr(input),
        0x01 => map(parse_12x, |(a, b)| Instr::Move(Reg::from(a), Reg::from(b)))(input),
        0x02 => map(parse_22x, |(a, b)| {
            Instr::MoveFrom16(Reg::from(a), Reg::from(b))
        })(input),
        0x03 => map(parse_32x, |(a, b)| {
            Instr::Move16(Reg::from(a), Reg::from(b))
        })(input),
        0x04 => map(parse_12x, |(a, b)| {
            Instr::MoveWide(Reg::from(a), Reg::from(b))
        })(input),
        0x05 => map(parse_22x, |(a, b)| {
            Instr::MoveWideFrom16(Reg::from(a), Reg::from(b))
        })(input),
        0x06 => map(parse_32x, |(a, b)| {
            Instr::MoveWide16(Reg::from(a), Reg::from(b))
        })(input),
        0x07 => map(parse_12x, |(a, b)| {
            Instr::MoveObject(Reg::from(a), Reg::from(b))
        })(input),
        0x08 => map(parse_22x, |(a, b)| {
            Instr::MoveObjectFrom16(Reg::from(a), Reg::from(b))
        })(input),
        0x09 => map(parse_32x, |(a, b)| {
            Instr::MoveObject16(Reg::from(a), Reg::from(b))
        })(input),
        0x0a => map(parse_11x, |a| Instr::MoveResult(Reg::from(a)))(input),
        0x0b => map(parse_11x, |a| Instr::MoveResultWide(Reg::from(a)))(input),
        0x0c => map(parse_11x, |a| Instr::MoveResultObject(Reg::from(a)))(input),
        0x0d => map(parse_11x, |a| Instr::MoveException(Reg::from(a)))(input),
        0x0e => map(parse_10x, |()| Instr::ReturnVoid)(input),
        0x0f => map(parse_11x, |a| Instr::Return(Reg::from(a)))(input),
        0x10 => map(parse_11x, |a| Instr::ReturnWide(Reg::from(a)))(input),
        0x11 => map(parse_11x, |a| Instr::ReturnObject(Reg::from(a)))(input),
        0x12 => map(parse_11n, |(a, b)| Instr::Const4(Reg::from(a), b))(input),
        0x13 => map(parse_21s, |(a, b)| Instr::Const16(Reg::from(a), b))(input),
        0x14 => map(parse_31i, |(a, b)| Instr::Const(Reg::from(a), b))(input),
        0x15 => map(parse_21h, |(a, b)| Instr::ConstHigh16(Reg::from(a), b))(input),
        0x16 => map(parse_21s, |(a, b)| Instr::ConstWide16(Reg::from(a), b))(input),
        0x17 => map(parse_31i, |(a, b)| Instr::ConstWide32(Reg::from(a), b))(input),
        0x18 => map(parse_51l, |(a, b)| Instr::ConstWide(Reg::from(a), b))(input),
        0x19 => map(parse_21h, |(a, b)| Instr::ConstWideHigh16(Reg::from(a), b))(input),
        0x1a => map(parse_21c, |(a, b)| {
            Instr::ConstString(Reg::from(a), Index::new(b))
        })(input),
        0x1b => map(parse_31c, |(a, b)| {
            Instr::ConstStringJumbo(Reg::from(a), Index::new(b))
        })(input),
        0x1c => map(parse_21c, |(a, b)| {
            Instr::ConstClass(Reg::from(a), Index::new(b))
        })(input),
        0x1d => map(parse_11x, |a| Instr::MonitorEnter(Reg::from(a)))(input),
        0x1e => map(parse_11x, |a| Instr::MonitorExit(Reg::from(a)))(input),
        0x1f => map(parse_21c, |(a, b)| {
            Instr::CheckCast(Reg::from(a), Index::new(b))
        })(input),
        0x20 => map(parse_22c, |(a, b, c)| {
            Instr::InstanceOf(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x21 => map(parse_12x, |(a, b)| {
            Instr::ArrayLength(Reg::from(a), Reg::from(b))
        })(input),
        0x22 => map(parse_21c, |(a, b)| {
            Instr::NewInstance(Reg::from(a), Index::new(b))
        })(input),
        0x23 => map(parse_22c, |(a, b, c)| {
            Instr::NewArray(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x24 => map(parse_35c, |(args, b)| {
            Instr::FilledNewArray(RegList::from(args), Index::new(b))
        })(input),
        0x25 => map(parse_3rc, |(a, b, c)| {
            Instr::FilledNewArrayRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x26 => map(parse_31t, |(a, b)| Instr::FillArrayData(Reg::from(a), b))(input),
        0x27 => map(parse_11x, |a| Instr::Throw(Reg::from(a)))(input),
        0x28 => map(parse_10t, Instr::Goto)(input),
        0x29 => map(parse_20t, Instr::Goto16)(input),
        0x2a => map(parse_30t, Instr::Goto32)(input),
        0x2b => map(parse_31t, |(a, b)| Instr::PackedSwitch(Reg::from(a), b))(input),
        0x2c => map(parse_31t, |(a, b)| Instr::SparseSwitch(Reg::from(a), b))(input),
        0x2d => map(parse_23x, |(a, b, c)| {
            Instr::CmplFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x2e => map(parse_23x, |(a, b, c)| {
            Instr::CmpgFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x2f => map(parse_23x, |(a, b, c)| {
            Instr::CmplDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x30 => map(parse_23x, |(a, b, c)| {
            Instr::CmpgDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x31 => map(parse_23x, |(a, b, c)| {
            Instr::CmpLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x32 => map(parse_22t, |(a, b, c)| {
            Instr::IfEq(Reg::from(a), Reg::from(b), c)
        })(input),
        0x33 => map(parse_22t, |(a, b, c)| {
            Instr::IfNe(Reg::from(a), Reg::from(b), c)
        })(input),
        0x34 => map(parse_22t, |(a, b, c)| {
            Instr::IfLt(Reg::from(a), Reg::from(b), c)
        })(input),
        0x35 => map(parse_22t, |(a, b, c)| {
            Instr::IfGe(Reg::from(a), Reg::from(b), c)
        })(input),
        0x36 => map(parse_22t, |(a, b, c)| {
            Instr::IfGt(Reg::from(a), Reg::from(b), c)
        })(input),
        0x37 => map(parse_22t, |(a, b, c)| {
            Instr::IfLe(Reg::from(a), Reg::from(b), c)
        })(input),
        0x38 => map(parse_21t, |(a, b)| Instr::IfEqz(Reg::from(a), b))(input),
        0x39 => map(parse_21t, |(a, b)| Instr::IfNez(Reg::from(a), b))(input),
        0x3a => map(parse_21t, |(a, b)| Instr::IfLtz(Reg::from(a), b))(input),
        0x3b => map(parse_21t, |(a, b)| Instr::IfGez(Reg::from(a), b))(input),
        0x3c => map(parse_21t, |(a, b)| Instr::IfGtz(Reg::from(a), b))(input),
        0x3d => map(parse_21t, |(a, b)| Instr::IfLez(Reg::from(a), b))(input),
        0x44 => map(parse_23x, |(a, b, c)| {
            Instr::Aget(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x45 => map(parse_23x, |(a, b, c)| {
            Instr::AgetWide(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x46 => map(parse_23x, |(a, b, c)| {
            Instr::AgetObject(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x47 => map(parse_23x, |(a, b, c)| {
            Instr::AgetBoolean(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x48 => map(parse_23x, |(a, b, c)| {
            Instr::AgetByte(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x49 => map(parse_23x, |(a, b, c)| {
            Instr::AgetChar(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4a => map(parse_23x, |(a, b, c)| {
            Instr::AgetShort(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4b => map(parse_23x, |(a, b, c)| {
            Instr::Aput(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4c => map(parse_23x, |(a, b, c)| {
            Instr::AputWide(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4d => map(parse_23x, |(a, b, c)| {
            Instr::AputObject(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4e => map(parse_23x, |(a, b, c)| {
            Instr::AputBoolean(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x4f => map(parse_23x, |(a, b, c)| {
            Instr::AputByte(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x50 => map(parse_23x, |(a, b, c)| {
            Instr::AputChar(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x51 => map(parse_23x, |(a, b, c)| {
            Instr::AputShort(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x52 => map(parse_22c, |(a, b, c)| {
            Instr::Iget(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x53 => map(parse_22c, |(a, b, c)| {
            Instr::IgetWide(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x54 => map(parse_22c, |(a, b, c)| {
            Instr::IgetObject(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x55 => map(parse_22c, |(a, b, c)| {
            Instr::IgetBoolean(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x56 => map(parse_22c, |(a, b, c)| {
            Instr::IgetByte(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x57 => map(parse_22c, |(a, b, c)| {
            Instr::IgetChar(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x58 => map(parse_22c, |(a, b, c)| {
            Instr::IgetShort(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x59 => map(parse_22c, |(a, b, c)| {
            Instr::Iput(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5a => map(parse_22c, |(a, b, c)| {
            Instr::IputWide(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5b => map(parse_22c, |(a, b, c)| {
            Instr::IputObject(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5c => map(parse_22c, |(a, b, c)| {
            Instr::IputBoolean(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5d => map(parse_22c, |(a, b, c)| {
            Instr::IputByte(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5e => map(parse_22c, |(a, b, c)| {
            Instr::IputChar(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x5f => map(parse_22c, |(a, b, c)| {
            Instr::IputShort(Reg::from(a), Reg::from(b), Index::new(c))
        })(input),
        0x60 => map(parse_21c, |(a, b)| Instr::Sget(Reg::from(a), Index::new(b)))(input),
        0x61 => map(parse_21c, |(a, b)| {
            Instr::SgetWide(Reg::from(a), Index::new(b))
        })(input),
        0x62 => map(parse_21c, |(a, b)| {
            Instr::SgetObject(Reg::from(a), Index::new(b))
        })(input),
        0x63 => map(parse_21c, |(a, b)| {
            Instr::SgetBoolean(Reg::from(a), Index::new(b))
        })(input),
        0x64 => map(parse_21c, |(a, b)| {
            Instr::SgetByte(Reg::from(a), Index::new(b))
        })(input),
        0x65 => map(parse_21c, |(a, b)| {
            Instr::SgetChar(Reg::from(a), Index::new(b))
        })(input),
        0x66 => map(parse_21c, |(a, b)| {
            Instr::SgetShort(Reg::from(a), Index::new(b))
        })(input),
        0x67 => map(parse_21c, |(a, b)| Instr::Sput(Reg::from(a), Index::new(b)))(input),
        0x68 => map(parse_21c, |(a, b)| {
            Instr::SputWide(Reg::from(a), Index::new(b))
        })(input),
        0x69 => map(parse_21c, |(a, b)| {
            Instr::SputObject(Reg::from(a), Index::new(b))
        })(input),
        0x6a => map(parse_21c, |(a, b)| {
            Instr::SputBoolean(Reg::from(a), Index::new(b))
        })(input),
        0x6b => map(parse_21c, |(a, b)| {
            Instr::SputByte(Reg::from(a), Index::new(b))
        })(input),
        0x6c => map(parse_21c, |(a, b)| {
            Instr::SputChar(Reg::from(a), Index::new(b))
        })(input),
        0x6d => map(parse_21c, |(a, b)| {
            Instr::SputShort(Reg::from(a), Index::new(b))
        })(input),
        0x6e => map(parse_35c, |(args, b)| {
            Instr::InvokeVirtual(RegList::from(args), Index::new(b))
        })(input),
        0x6f => map(parse_35c, |(args, b)| {
            Instr::InvokeSuper(RegList::from(args), Index::new(b))
        })(input),
        0x70 => map(parse_35c, |(args, b)| {
            Instr::InvokeDirect(RegList::from(args), Index::new(b))
        })(input),
        0x71 => map(parse_35c, |(args, b)| {
            Instr::InvokeStatic(RegList::from(args), Index::new(b))
        })(input),
        0x72 => map(parse_35c, |(args, b)| {
            Instr::InvokeInterface(RegList::from(args), Index::new(b))
        })(input),
        0x74 => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeVirtualRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x75 => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeSuperRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x76 => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeDirectRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x77 => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeStaticRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x78 => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeInterfaceRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0x7b => map(parse_12x, |(a, b)| {
            Instr::NegInt(Reg::from(a), Reg::from(b))
        })(input),
        0x7c => map(parse_12x, |(a, b)| {
            Instr::NotInt(Reg::from(a), Reg::from(b))
        })(input),
        0x7d => map(parse_12x, |(a, b)| {
            Instr::NegLong(Reg::from(a), Reg::from(b))
        })(input),
        0x7e => map(parse_12x, |(a, b)| {
            Instr::NotLong(Reg::from(a), Reg::from(b))
        })(input),
        0x7f => map(parse_12x, |(a, b)| {
            Instr::NegFloat(Reg::from(a), Reg::from(b))
        })(input),
        0x80 => map(parse_12x, |(a, b)| {
            Instr::NegDouble(Reg::from(a), Reg::from(b))
        })(input),
        0x81 => map(parse_12x, |(a, b)| {
            Instr::IntToLong(Reg::from(a), Reg::from(b))
        })(input),
        0x82 => map(parse_12x, |(a, b)| {
            Instr::IntToFloat(Reg::from(a), Reg::from(b))
        })(input),
        0x83 => map(parse_12x, |(a, b)| {
            Instr::IntToDouble(Reg::from(a), Reg::from(b))
        })(input),
        0x84 => map(parse_12x, |(a, b)| {
            Instr::LongToInt(Reg::from(a), Reg::from(b))
        })(input),
        0x85 => map(parse_12x, |(a, b)| {
            Instr::LongToFloat(Reg::from(a), Reg::from(b))
        })(input),
        0x86 => map(parse_12x, |(a, b)| {
            Instr::LongToDouble(Reg::from(a), Reg::from(b))
        })(input),
        0x87 => map(parse_12x, |(a, b)| {
            Instr::FloatToInt(Reg::from(a), Reg::from(b))
        })(input),
        0x88 => map(parse_12x, |(a, b)| {
            Instr::FloatToLong(Reg::from(a), Reg::from(b))
        })(input),
        0x89 => map(parse_12x, |(a, b)| {
            Instr::FloatToDouble(Reg::from(a), Reg::from(b))
        })(input),
        0x8a => map(parse_12x, |(a, b)| {
            Instr::DoubleToInt(Reg::from(a), Reg::from(b))
        })(input),
        0x8b => map(parse_12x, |(a, b)| {
            Instr::DoubleToLong(Reg::from(a), Reg::from(b))
        })(input),
        0x8c => map(parse_12x, |(a, b)| {
            Instr::DoubleToFloat(Reg::from(a), Reg::from(b))
        })(input),
        0x8d => map(parse_12x, |(a, b)| {
            Instr::IntToByte(Reg::from(a), Reg::from(b))
        })(input),
        0x8e => map(parse_12x, |(a, b)| {
            Instr::IntToChar(Reg::from(a), Reg::from(b))
        })(input),
        0x8f => map(parse_12x, |(a, b)| {
            Instr::IntToShort(Reg::from(a), Reg::from(b))
        })(input),
        0x90 => map(parse_23x, |(a, b, c)| {
            Instr::AddInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x91 => map(parse_23x, |(a, b, c)| {
            Instr::SubInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x92 => map(parse_23x, |(a, b, c)| {
            Instr::MulInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x93 => map(parse_23x, |(a, b, c)| {
            Instr::DivInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x94 => map(parse_23x, |(a, b, c)| {
            Instr::RemInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x95 => map(parse_23x, |(a, b, c)| {
            Instr::AndInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x96 => map(parse_23x, |(a, b, c)| {
            Instr::OrInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x97 => map(parse_23x, |(a, b, c)| {
            Instr::XorInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x98 => map(parse_23x, |(a, b, c)| {
            Instr::ShlInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x99 => map(parse_23x, |(a, b, c)| {
            Instr::ShrInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9a => map(parse_23x, |(a, b, c)| {
            Instr::UshrInt(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9b => map(parse_23x, |(a, b, c)| {
            Instr::AddLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9c => map(parse_23x, |(a, b, c)| {
            Instr::SubLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9d => map(parse_23x, |(a, b, c)| {
            Instr::MulLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9e => map(parse_23x, |(a, b, c)| {
            Instr::DivLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0x9f => map(parse_23x, |(a, b, c)| {
            Instr::RemLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa0 => map(parse_23x, |(a, b, c)| {
            Instr::AndLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa1 => map(parse_23x, |(a, b, c)| {
            Instr::OrLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa2 => map(parse_23x, |(a, b, c)| {
            Instr::XorLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa3 => map(parse_23x, |(a, b, c)| {
            Instr::ShlLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa4 => map(parse_23x, |(a, b, c)| {
            Instr::ShrLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa5 => map(parse_23x, |(a, b, c)| {
            Instr::UshrLong(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa6 => map(parse_23x, |(a, b, c)| {
            Instr::AddFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa7 => map(parse_23x, |(a, b, c)| {
            Instr::SubFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa8 => map(parse_23x, |(a, b, c)| {
            Instr::MulFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xa9 => map(parse_23x, |(a, b, c)| {
            Instr::DivFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xaa => map(parse_23x, |(a, b, c)| {
            Instr::RemFloat(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xab => map(parse_23x, |(a, b, c)| {
            Instr::AddDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xac => map(parse_23x, |(a, b, c)| {
            Instr::SubDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xad => map(parse_23x, |(a, b, c)| {
            Instr::MulDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xae => map(parse_23x, |(a, b, c)| {
            Instr::DivDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xaf => map(parse_23x, |(a, b, c)| {
            Instr::RemDouble(Reg::from(a), Reg::from(b), Reg::from(c))
        })(input),
        0xb0 => map(parse_12x, |(a, b)| {
            Instr::AddInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb1 => map(parse_12x, |(a, b)| {
            Instr::SubInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb2 => map(parse_12x, |(a, b)| {
            Instr::MulInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb3 => map(parse_12x, |(a, b)| {
            Instr::DivInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb4 => map(parse_12x, |(a, b)| {
            Instr::RemInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb5 => map(parse_12x, |(a, b)| {
            Instr::AndInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb6 => map(parse_12x, |(a, b)| {
            Instr::OrInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb7 => map(parse_12x, |(a, b)| {
            Instr::XorInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb8 => map(parse_12x, |(a, b)| {
            Instr::ShlInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xb9 => map(parse_12x, |(a, b)| {
            Instr::ShrInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xba => map(parse_12x, |(a, b)| {
            Instr::UshrInt2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xbb => map(parse_12x, |(a, b)| {
            Instr::AddLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xbc => map(parse_12x, |(a, b)| {
            Instr::SubLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xbd => map(parse_12x, |(a, b)| {
            Instr::MulLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xbe => map(parse_12x, |(a, b)| {
            Instr::DivLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xbf => map(parse_12x, |(a, b)| {
            Instr::RemLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc0 => map(parse_12x, |(a, b)| {
            Instr::AndLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc1 => map(parse_12x, |(a, b)| {
            Instr::OrLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc2 => map(parse_12x, |(a, b)| {
            Instr::XorLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc3 => map(parse_12x, |(a, b)| {
            Instr::ShlLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc4 => map(parse_12x, |(a, b)| {
            Instr::ShrLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc5 => map(parse_12x, |(a, b)| {
            Instr::UshrLong2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc6 => map(parse_12x, |(a, b)| {
            Instr::AddFloat2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc7 => map(parse_12x, |(a, b)| {
            Instr::SubFloat2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc8 => map(parse_12x, |(a, b)| {
            Instr::MulFloat2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xc9 => map(parse_12x, |(a, b)| {
            Instr::DivFloat2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xca => map(parse_12x, |(a, b)| {
            Instr::RemFloat2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xcb => map(parse_12x, |(a, b)| {
            Instr::AddDouble2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xcc => map(parse_12x, |(a, b)| {
            Instr::SubDouble2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xcd => map(parse_12x, |(a, b)| {
            Instr::MulDouble2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xce => map(parse_12x, |(a, b)| {
            Instr::DivDouble2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xcf => map(parse_12x, |(a, b)| {
            Instr::RemDouble2addr(Reg::from(a), Reg::from(b))
        })(input),
        0xd0 => map(parse_22s, |(a, b, c)| {
            Instr::AddIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd1 => map(parse_22s, |(a, b, c)| {
            Instr::RsubInt(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd2 => map(parse_22s, |(a, b, c)| {
            Instr::MulIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd3 => map(parse_22s, |(a, b, c)| {
            Instr::DivIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd4 => map(parse_22s, |(a, b, c)| {
            Instr::RemIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd5 => map(parse_22s, |(a, b, c)| {
            Instr::AndIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd6 => map(parse_22s, |(a, b, c)| {
            Instr::OrIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd7 => map(parse_22s, |(a, b, c)| {
            Instr::XorIntLit16(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd8 => map(parse_22b, |(a, b, c)| {
            Instr::AddIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xd9 => map(parse_22b, |(a, b, c)| {
            Instr::RsubIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xda => map(parse_22b, |(a, b, c)| {
            Instr::MulIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xdb => map(parse_22b, |(a, b, c)| {
            Instr::DivIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xdc => map(parse_22b, |(a, b, c)| {
            Instr::RemIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xdd => map(parse_22b, |(a, b, c)| {
            Instr::AndIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xde => map(parse_22b, |(a, b, c)| {
            Instr::OrIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xdf => map(parse_22b, |(a, b, c)| {
            Instr::XorIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xe0 => map(parse_22b, |(a, b, c)| {
            Instr::ShlIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xe1 => map(parse_22b, |(a, b, c)| {
            Instr::ShrIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xe2 => map(parse_22b, |(a, b, c)| {
            Instr::UshrIntLit8(Reg::from(a), Reg::from(b), c)
        })(input),
        0xfa => map(parse_45cc, |(args, b, c)| {
            Instr::InvokePolymorphic(RegList::from(args), Index::new(b), Index::new(c))
        })(input),
        0xfb => map(parse_4rcc, |(a, b, c, d)| {
            Instr::InvokePolymorphicRange(RegRange::from((a, b)), Index::new(c), Index::new(d))
        })(input),
        0xfc => map(parse_35c, |(args, b)| {
            Instr::InvokeCustom(RegList::from(args), Index::new(b))
        })(input),
        0xfd => map(parse_3rc, |(a, b, c)| {
            Instr::InvokeCustomRange(RegRange::from((a, b)), Index::new(c))
        })(input),
        0xfe => map(parse_21c, |(a, b)| {
            Instr::ConstMethodHandle(Reg::from(a), Index::new(b))
        })(input),
        0xff => map(parse_21c, |(a, b)| {
            Instr::ConstMethodType(Reg::from(a), Index::new(b))
        })(input),
        _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Tag))),
    }
}

fn parse_pseudo_instr(input: &[u8]) -> IResult<&[u8], Instr, DexError> {
    let (input, ident) = le_u8(input)?;
    match ident {
        0x00 => Ok((input, Instr::Nop)),
        0x01 => {
            let (input, size) = le_u16(input)?;
            let (input, first_key) = le_i32(input)?;
            let (input, targets) = count(le_i32, size as usize)(input)?;
            Ok((input, Instr::PackedSwitchPayload(first_key, targets)))
        }
        0x02 => {
            let (input, size) = le_u16(input)?;
            let (input, keys) = count(le_i32, size as usize)(input)?;
            let (input, targets) = count(le_i32, size as usize)(input)?;
            Ok((input, Instr::SparseSwitchPayload(keys, targets)))
        }
        0x03 => {
            let (i, element_width) = le_u16(input)?;
            let (i, size) = le_u32(i)?;
            let (i, data) = count(count(le_u8, element_width as usize), size as usize)(i)?;
            let expect_size = ((size as usize * element_width as usize + 1) / 2 + 3) * 2;
            let parsed_size = input.offset(i);
            let padd_offset = expect_size - parsed_size;
            Ok((&i[padd_offset..], Instr::FillArrayDataPayload(data)))
        }
        _ => Err(Error(DexError::from_error_kind(input, ErrorKind::Switch))),
    }
}

fn parse_10x(input: &[u8]) -> IResult<&[u8], (), DexError> {
    value((), tag("\x00"))(input)
}

fn parse_12x<'a>(input: &'a [u8]) -> IResult<&'a [u8], (u8, u8), DexError> {
    bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (u8, u8), DexError> {
            let (input, b) = take_bits(4_usize)(input)?;
            let (input, a) = take_bits(4_usize)(input)?;
            Ok((input, (a, b)))
        },
    )(input)
}

fn parse_11n<'a>(input: &'a [u8]) -> IResult<&'a [u8], (u8, i8), DexError> {
    bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (u8, i8), DexError> {
            let (input, b): ((&[u8], usize), u8) = take_bits(4_usize)(input)?;
            let (input, a) = take_bits(4_usize)(input)?;
            Ok((input, (a, b as i8)))
        },
    )(input)
}

fn parse_11x(input: &[u8]) -> IResult<&[u8], u8, DexError> {
    le_u8(input)
}

fn parse_10t(input: &[u8]) -> IResult<&[u8], i8, DexError> {
    le_i8(input)
}

fn parse_20t(input: &[u8]) -> IResult<&[u8], i16, DexError> {
    preceded(tag("\x00"), le_i16)(input)
}

fn parse_22x(input: &[u8]) -> IResult<&[u8], (u8, u16), DexError> {
    pair(le_u8, le_u16)(input)
}

fn parse_21t(input: &[u8]) -> IResult<&[u8], (u8, i16), DexError> {
    pair(le_u8, le_i16)(input)
}

fn parse_21s(input: &[u8]) -> IResult<&[u8], (u8, i16), DexError> {
    pair(le_u8, le_i16)(input)
}

fn parse_21h(input: &[u8]) -> IResult<&[u8], (u8, i16), DexError> {
    pair(le_u8, le_i16)(input)
}

fn parse_21c(input: &[u8]) -> IResult<&[u8], (u8, usize), DexError> {
    pair(le_u8, map(le_u16, |b| b as usize))(input)
}

fn parse_23x(input: &[u8]) -> IResult<&[u8], (u8, u8, u8), DexError> {
    tuple((le_u8, le_u8, le_u8))(input)
}

fn parse_22b(input: &[u8]) -> IResult<&[u8], (u8, u8, i8), DexError> {
    tuple((le_u8, le_u8, le_i8))(input)
}

fn parse_22t<'a>(input: &'a [u8]) -> IResult<&'a [u8], (u8, u8, i16), DexError> {
    let (input, (b, a)) = bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (u8, u8), DexError> {
            pair(take_bits(4_usize), take_bits(4_usize))(input)
        },
    )(input)?;
    let (input, c) = le_i16(input)?;
    Ok((input, (a, b, c)))
}

fn parse_22s<'a>(input: &'a [u8]) -> IResult<&'a [u8], (u8, u8, i16), DexError> {
    let (input, (b, a)) = bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (u8, u8), DexError> {
            pair(take_bits(4_usize), take_bits(4_usize))(input)
        },
    )(input)?;
    let (input, c) = le_i16(input)?;
    Ok((input, (a, b, c)))
}

fn parse_22c<'a>(input: &'a [u8]) -> IResult<&'a [u8], (u8, u8, usize), DexError> {
    let (input, (b, a)) = bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (u8, u8), DexError> {
            pair(take_bits(4_usize), take_bits(4_usize))(input)
        },
    )(input)?;
    let (input, c) = le_u16(input)?;
    Ok((input, (a, b, c as usize)))
}

fn parse_30t(input: &[u8]) -> IResult<&[u8], i32, DexError> {
    preceded(tag("\x00"), le_i32)(input)
}

fn parse_32x(input: &[u8]) -> IResult<&[u8], (u16, u16), DexError> {
    preceded(tag("\x00"), pair(le_u16, le_u16))(input)
}

fn parse_31i(input: &[u8]) -> IResult<&[u8], (u8, i32), DexError> {
    pair(le_u8, le_i32)(input)
}

fn parse_31t(input: &[u8]) -> IResult<&[u8], (u8, i32), DexError> {
    pair(le_u8, le_i32)(input)
}

fn parse_31c(input: &[u8]) -> IResult<&[u8], (u8, usize), DexError> {
    pair(le_u8, map(le_u32, |b| b as usize))(input)
}

// allow single char names so that they match the specifications
#[allow(clippy::many_single_char_names)]
#[allow(clippy::type_complexity)]
fn parse_35c<'a>(input: &'a [u8]) -> IResult<&'a [u8], (Vec<u8>, usize), DexError> {
    bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (Vec<u8>, usize), DexError> {
            let (input, a) = verify(take_bits(4_usize), |a| *a <= 5)(input)?;
            let (input, g) = take_bits(4_usize)(input)?;
            let (input, b) =
                bytes(|input: &'a [u8]| -> IResult<&'a [u8], u16, DexError> { le_u16(input) })(
                    input,
                )?;
            let (input, d) = take_bits(4_usize)(input)?;
            let (input, c) = take_bits(4_usize)(input)?;
            let (input, f) = take_bits(4_usize)(input)?;
            let (input, e) = take_bits(4_usize)(input)?;
            Ok((
                input,
                match a {
                    0 => (vec![], b as usize),
                    1 => (vec![c], b as usize),
                    2 => (vec![c, d], b as usize),
                    3 => (vec![c, d, e], b as usize),
                    4 => (vec![c, d, e, f], b as usize),
                    5 => (vec![c, d, e, f, g], b as usize),
                    _ => unreachable!(), // checked with verify
                },
            ))
        },
    )(input)
}

fn parse_3rc(input: &[u8]) -> IResult<&[u8], (u16, u16, usize), DexError> {
    // checking 'a' to prevent overflow:
    let (input, a) = verify(map(le_u8, u16::from), |a| *a != 0)(input)?;
    let (input, (b, c)) = pair(le_u16, le_u16)(input)?;
    Ok((input, (c, c.saturating_add(a) - 1, b as usize)))
}

// allow single char names so that they match the specifications
#[allow(clippy::many_single_char_names)]
#[allow(clippy::type_complexity)]
fn parse_45cc<'a>(input: &'a [u8]) -> IResult<&'a [u8], (Vec<u8>, usize, usize), DexError> {
    bits(
        |input: (&'a [u8], usize)| -> IResult<(&'a [u8], usize), (Vec<u8>, usize, usize), DexError> {
            let (input, a) = verify(take_bits(4_usize), |a| *a <= 5)(input)?;
            let (input, g) = take_bits(4_usize)(input)?;
            let (input, b) =
                bytes(|input: &'a [u8]| -> IResult<&'a [u8], u16, DexError> { le_u16(input) })(input)?;
            let (input, d) = take_bits(4_usize)(input)?;
            let (input, c) = take_bits(4_usize)(input)?;
            let (input, f) = take_bits(4_usize)(input)?;
            let (input, e) = take_bits(4_usize)(input)?;
            let (input, h) =
                bytes(|input: &'a [u8]| -> IResult<&'a [u8], u16, DexError> { le_u16(input) })(input)?;
            Ok((
                input,
                match a {
                    0 => (vec![], b as usize, h as usize),
                    1 => (vec![c], b as usize, h as usize),
                    2 => (vec![c, d], b as usize, h as usize),
                    3 => (vec![c, d, e], b as usize, h as usize),
                    4 => (vec![c, d, e, f], b as usize, h as usize),
                    5 => (vec![c, d, e, f, g], b as usize, h as usize),
                    _ => unreachable!(), // checked with verify
                },
            ))
        },
    )(input)
}

fn parse_4rcc(input: &[u8]) -> IResult<&[u8], (u16, u16, usize, usize), DexError> {
    // checking 'a' to prevent overflow:
    let (input, a) = verify(map(le_u8, u16::from), |a| *a != 0)(input)?;
    let (input, (b, c, h)) = tuple((le_u16, le_u16, le_u16))(input)?;
    Ok((input, (c, c.saturating_add(a) - 1, b as usize, h as usize)))
}

fn parse_51l(input: &[u8]) -> IResult<&[u8], (u8, i64), DexError> {
    pair(le_u8, le_i64)(input)
}

fn uleb128(input: &[u8]) -> IResult<&[u8], Uleb128, DexError> {
    let start = input;
    let (input, bs) = verify(take_till(|b| b & 0x80 == 0), |bs: &[u8]| bs.len() < 5)(input)?;
    let (input, b) = map(le_u8, u32::from)(input)?;

    let res = bs
        .iter()
        .rev()
        .fold(b, |acc, v| (acc << 7) + u32::from(v & 0x7f));
    Ok((input, Uleb128::new(res, Some(start.offset(input)))))
}

fn uleb128p1(input: &[u8]) -> IResult<&[u8], Option<u32>, DexError> {
    map(uleb128, |x| {
        if x.value() == 0 {
            None
        } else {
            Some(x.value() - 1)
        }
    })(input)
}

#[allow(clippy::cast_possible_wrap)]
fn sleb128(input: &[u8]) -> IResult<&[u8], Sleb128, DexError> {
    let start = input;
    let (input, bs) = verify(take_till(|b| b & 0x80 == 0), |bs: &[u8]| bs.len() < 5)(input)?;
    let (input, b) = map(le_u8, u32::from)(input)?;

    let is_neg = b & 0x40 != 0;
    let mut res = bs
        .iter()
        .rev()
        .fold(b, |acc, v| (acc << 7) + u32::from(v & 0x7f));
    if is_neg {
        let mut i = 0;
        loop {
            let mask = 1 << (31 - i);
            if mask & res != 0 {
                break;
            }
            res |= mask;
            i += 1;
        }
    }
    Ok((input, Sleb128::new(res as i32, Some(start.offset(input)))))
}

fn le_u16_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], u16, DexError> {
    move |input: &[u8]| {
        if siz == 0 || siz > 2 {
            return Err(Error(DexError::from_error_kind(
                input,
                ErrorKind::LengthValue,
            )));
        }
        map(count(le_u8, siz), |bs| {
            bs.iter().rev().fold(0, |acc, b| (acc << 8) + u16::from(*b))
        })(input)
    }
}

#[allow(clippy::cast_possible_wrap)]
fn le_i16_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], i16, DexError> {
    move |input: &[u8]| map(le_u16_on(siz), |value| value as i16)(input)
}

fn le_u32_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], u32, DexError> {
    move |input: &[u8]| {
        if siz == 0 || siz > 4 {
            return Err(Error(DexError::from_error_kind(
                input,
                ErrorKind::LengthValue,
            )));
        }
        map(count(le_u8, siz), |bs| {
            bs.iter().rev().fold(0, |acc, b| (acc << 8) + u32::from(*b))
        })(input)
    }
}

#[allow(clippy::cast_possible_wrap)]
fn le_i32_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], i32, DexError> {
    move |input: &[u8]| map(le_u32_on(siz), |value| value as i32)(input)
}

fn le_u64_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], u64, DexError> {
    move |input: &[u8]| {
        if siz == 0 || siz > 8 {
            return Err(Error(DexError::from_error_kind(
                input,
                ErrorKind::LengthValue,
            )));
        }
        map(count(le_u8, siz), |bs| {
            bs.iter().rev().fold(0, |acc, b| (acc << 8) + u64::from(*b))
        })(input)
    }
}

#[allow(clippy::cast_possible_wrap)]
fn le_i64_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], i64, DexError> {
    move |input: &[u8]| map(le_u64_on(siz), |value| value as i64)(input)
}

#[allow(clippy::cast_precision_loss)]
fn le_f32_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], f32, DexError> {
    move |input: &[u8]| map(le_u32_on(siz), |value| value as f32)(input)
}

#[allow(clippy::cast_precision_loss)]
fn le_f64_on(siz: usize) -> impl Fn(&[u8]) -> IResult<&[u8], f64, DexError> {
    move |input: &[u8]| map(le_u64_on(siz), |value| value as f64)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uleb128_parser() {
        assert_eq!(0, uleb128(&[0x00]).unwrap().1.value());
        assert_eq!(1, uleb128(&[0x01]).unwrap().1.value());
        assert_eq!(127, uleb128(&[0x7f]).unwrap().1.value());
        assert_eq!(16256, uleb128(&[0x80, 0x7f]).unwrap().1.value());
    }

    #[test]
    fn uleb128p1_parser() {
        assert_eq!(None, uleb128p1(&[0x00]).unwrap().1);
        assert_eq!(Some(0), uleb128p1(&[0x01]).unwrap().1);
        assert_eq!(Some(126), uleb128p1(&[0x7f]).unwrap().1);
        assert_eq!(Some(16255), uleb128p1(&[0x80, 0x7f]).unwrap().1);
    }

    #[test]
    fn sleb128_parser() {
        assert_eq!(0, sleb128(&[0x00]).unwrap().1.value());
        assert_eq!(1, sleb128(&[0x01]).unwrap().1.value());
        assert_eq!(-1, sleb128(&[0x7f]).unwrap().1.value());
        assert_eq!(-128, sleb128(&[0x80, 0x7f]).unwrap().1.value());
    }

    #[test]
    fn le_u32_on_parser() {
        use nom::number::complete::le_u32;
        let input = vec![0x12, 0x34, 0x56, 0x00];
        let r1 = le_u32::<_, DexError>(&input[..4]).unwrap().1;
        let r2 = le_u32_on(3)(&input[..3]).unwrap().1;
        assert_eq!(r1, r2);
    }
}
