use crate::annotations::*;
use crate::classes::*;
use crate::code::*;
use crate::errors::DexResult;
use crate::fields::*;
use crate::instrs::*;
use crate::map::*;
use crate::methods::*;
use crate::strings::*;
use crate::types::*;
use crate::values::*;
use crate::{Dex, HeaderItem};
use dw_utils::leb::Uleb128;
use dw_utils::writers::*;
use nom::number::Endianness;
use sha1::{Digest, Sha1};
use std::io::{Cursor, Result, Seek, Write};

const NO_INDEX: u32 = 0xFFFF_FFFF;

/// Dex writing function. Borrows a [`Dex`] structure and returns a buffer.
pub fn write_dex(dex: &Dex, recompute_checksums: bool) -> DexResult<Vec<u8>> {
    log::trace!("writing dex...");

    let buffer: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(buffer);

    for map_item in &dex.map_list.list {
        match map_item.typ {
            MapItemType::HeaderItem => {
                log::trace!("writing header_item");
                let wr = header_item_writer(&mut cursor, &dex.header_item)?;
                debug_assert!(dex.header_item.size() == wr);
            }
            MapItemType::StringIdItem => {
                for item in &dex.string_id_items {
                    log::trace!("writing string_id_item");
                    let wr = string_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::TypeIdItem => {
                for item in &dex.type_id_items {
                    log::trace!("writing type_id_item");
                    let wr = type_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::ProtoIdItem => {
                for item in &dex.proto_id_items {
                    log::trace!("writing proto_id_item");
                    let wr = proto_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::FieldIdItem => {
                for item in &dex.field_id_items {
                    log::trace!("writing field_id_item");
                    let wr = field_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::MethodIdItem => {
                for item in &dex.method_id_items {
                    log::trace!("writing method_id_item");
                    let wr = method_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::ClassDefItem => {
                for item in &dex.class_def_items {
                    log::trace!("writing class_def_item");
                    let wr = class_def_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::CallSiteIdItem => {
                for item in &dex.call_site_id_items {
                    log::trace!("writing call_site_id_item");
                    let wr = call_site_id_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::MethodHandleItem => {
                for item in &dex.method_handle_items {
                    log::trace!("writing method_handle_item");
                    let wr = method_handle_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::MapList => {
                let _ = align4(&mut cursor)?;
                log::trace!("writing map_list");
                let wr = map_list_writer(&mut cursor, &dex.map_list)?;
                debug_assert!(dex.map_list.size() == wr);
            }
            MapItemType::TypeList => {
                for item in dex.type_lists.values() {
                    let _ = align4(&mut cursor)?;
                    log::trace!("writing type_list");
                    let _ = type_list_writer(&mut cursor, item)?;
                }
            }
            MapItemType::AnnotationSetRefList => {
                let _ = align4(&mut cursor)?;
                for item in dex.annotation_set_ref_lists.values() {
                    log::trace!("writing annotation_set_ref_list");
                    let wr = annotation_set_ref_list_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                    // declared as 4-byte-aligned in documentation,
                    // but should already be aligned by construction.
                    assert_eq!(cursor.stream_position().unwrap() % 4, 0);
                }
            }
            MapItemType::AnnotationSetItem => {
                let _ = align4(&mut cursor)?;
                for item in dex.annotation_set_items.values() {
                    log::trace!("writing annotation_set_item");
                    let wr = annotation_set_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                    // declared as 4-byte-aligned in documentation,
                    // but should already be aligned by construction.
                    assert_eq!(cursor.stream_position().unwrap() % 4, 0);
                }
            }
            MapItemType::ClassDataItem => {
                for item in dex.class_data_items.values() {
                    log::trace!("writing class_data_item");
                    let wr = class_data_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::CodeItem => {
                for item in dex.code_items.values() {
                    let _ = align4(&mut cursor)?;
                    log::trace!("writing code_item");
                    let _ = code_item_writer(&mut cursor, &item.read().unwrap())?;
                }
            }
            MapItemType::StringDataItem => {
                for item in dex.string_data_items.values() {
                    log::trace!("writing string_data_item");
                    let wr = string_data_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::DebugInfoItem => {
                for item in dex.debug_info_items.values() {
                    log::trace!("writing debug_info_item");
                    let wr = debug_info_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::AnnotationItem => {
                for item in dex.annotation_items.values() {
                    log::trace!("writing annotation_item");
                    let wr = annotation_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::EncodedArrayItem => {
                for item in dex.encoded_array_items.values() {
                    log::trace!("writing encoded_array_item");
                    let wr = encoded_array_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
            MapItemType::AnnotationsDirectoryItem => {
                let _ = align4(&mut cursor)?;
                for item in dex.annotations_directory_items.values() {
                    log::trace!("writing annotations_directory_item");
                    let wr = annotations_director_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                    // declared as 4-byte-aligned in documentation,
                    // but should already be aligned by construction.
                    assert_eq!(cursor.stream_position().unwrap() % 4, 0);
                }
            }
            MapItemType::HiddenapiClassDataItem => {
                for item in dex.hiddenapi_class_data_items.values() {
                    log::trace!("writing hidden_class_data_item");
                    let wr = hiddenapi_class_data_item_writer(&mut cursor, item)?;
                    debug_assert!(item.size() == wr);
                }
            }
        }
    }

    if recompute_checksums {
        let signature = {
            let buffer = cursor.get_ref();
            let mut hasher = Sha1::new();
            hasher.update(&buffer[32..]);
            hasher.finalize()
        };
        cursor.set_position(12);
        for byte in &signature {
            let _ = le_u8(&mut cursor, *byte)?;
        }
        let checksum = {
            let buffer = cursor.get_ref();
            adler32::adler32(&buffer[12..])?
        };
        cursor.set_position(8);
        let _ = le_u32(&mut cursor, checksum)?;
    }

    Ok(cursor.into_inner())
}

fn magic_writer<W: Write>(output: &mut W, v: u32) -> Result<usize> {
    let mut siz = 0;
    siz += tag(output, "dex\n")?;
    siz += tag(output, &format!("{v:03}"))?;
    siz += tag(output, "\x00")?;
    Ok(siz)
}

fn endian_tag_writer<W: Write>(output: &mut W, endianness: Endianness) -> Result<usize> {
    match endianness {
        Endianness::Little => tag(output, "\x78\x56\x34\x12"),
        Endianness::Big => tag(output, "\x12\x34\x56\x78"),
        Endianness::Native => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "endianness must be explicitly set",
        )),
    }
}

fn header_item_writer<W: Write>(output: &mut W, item: &HeaderItem) -> Result<usize> {
    let mut siz = 0;
    siz += magic_writer(output, item.version)?;
    siz += le_u32(output, item.checksum)?;
    for c in &item.signature {
        siz += le_u8(output, *c)?;
    }
    siz += le_u32(output, item.file_size as u32)?;
    siz += le_u32(output, 0x70)?; // header_size
    siz += endian_tag_writer(output, item.endianness)?;
    siz += le_u32(output, item._link_size as u32)?;
    siz += le_u32(output, item._link_off as u32)?;
    siz += le_u32(output, item.map_off as u32)?;
    siz += le_u32(output, item.string_ids_size as u32)?;
    siz += le_u32(output, item.string_ids_off as u32)?;
    siz += le_u32(output, item.type_ids_size as u32)?;
    siz += le_u32(output, item.type_ids_off as u32)?;
    siz += le_u32(output, item.proto_ids_size as u32)?;
    siz += le_u32(output, item.proto_ids_off as u32)?;
    siz += le_u32(output, item.field_ids_size as u32)?;
    siz += le_u32(output, item.field_ids_off as u32)?;
    siz += le_u32(output, item.method_ids_size as u32)?;
    siz += le_u32(output, item.method_ids_off as u32)?;
    siz += le_u32(output, item.class_defs_size as u32)?;
    siz += le_u32(output, item.class_defs_off as u32)?;
    siz += le_u32(output, item.data_size as u32)?;
    siz += le_u32(output, item.data_off as u32)?;
    Ok(siz)
}

fn string_id_item_writer<W: Write>(output: &mut W, item: &StringIdItem) -> Result<usize> {
    le_u32(output, item.string_data_off.as_usize() as u32)
}

fn type_id_item_writer<W: Write>(output: &mut W, item: &TypeIdItem) -> Result<usize> {
    le_u32(output, item.descriptor_idx.as_usize() as u32)
}

fn proto_id_item_writer<W: Write>(output: &mut W, item: &ProtoIdItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, item.shorty_idx.as_usize() as u32)?;
    siz += le_u32(output, item.return_type_idx.as_usize() as u32)?;
    siz += le_u32(
        output,
        item.parameters_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    Ok(siz)
}

fn field_id_item_writer<W: Write>(output: &mut W, item: &FieldIdItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u16(output, item.class_idx.as_usize() as u16)?;
    siz += le_u16(output, item.type_idx.as_usize() as u16)?;
    siz += le_u32(output, item.name_idx.as_usize() as u32)?;
    Ok(siz)
}

fn method_id_item_writer<W: Write>(output: &mut W, item: &MethodIdItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u16(output, item.class_idx.as_usize() as u16)?;
    siz += le_u16(output, item.proto_idx.as_usize() as u16)?;
    siz += le_u32(output, item.name_idx.as_usize() as u32)?;
    Ok(siz)
}

fn class_def_item_writer<W: Write>(output: &mut W, item: &ClassDefItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, item.class_idx.as_usize() as u32)?;
    siz += le_u32(output, item.access_flags.bits())?;
    siz += le_u32(
        output,
        item.superclass_idx
            .as_ref()
            .map_or(NO_INDEX, |id| id.as_usize() as u32),
    )?;
    siz += le_u32(
        output,
        item.interfaces_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    siz += le_u32(
        output,
        item.source_file_idx
            .as_ref()
            .map_or(NO_INDEX, |id| id.as_usize() as u32),
    )?;
    siz += le_u32(
        output,
        item.annotations_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    siz += le_u32(
        output,
        item.class_data_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    siz += le_u32(
        output,
        item.static_values_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    Ok(siz)
}

fn call_site_id_item_writer<W: Write>(output: &mut W, item: &CallSiteIdItem) -> Result<usize> {
    le_u32(output, item.call_site_off.as_usize() as u32)
}

fn method_handle_item_writer<W: Write>(output: &mut W, item: &MethodHandleItem) -> Result<usize> {
    let (method_handle_type, field_or_method_id) = match &item.method_handle {
        MethodHandle::StaticPut(field_id) => (0x00, field_id.as_usize() as u16),
        MethodHandle::StaticGet(field_id) => (0x01, field_id.as_usize() as u16),
        MethodHandle::InstancePut(field_id) => (0x02, field_id.as_usize() as u16),
        MethodHandle::InstanceGet(field_id) => (0x03, field_id.as_usize() as u16),
        MethodHandle::InvokeStatic(method_id) => (0x04, method_id.as_usize() as u16),
        MethodHandle::InvokeInstance(method_id) => (0x05, method_id.as_usize() as u16),
        MethodHandle::InvokeConstructor(method_id) => (0x06, method_id.as_usize() as u16),
        MethodHandle::InvokeDirect(method_id) => (0x07, method_id.as_usize() as u16),
        MethodHandle::InvokeInterface(method_id) => (0x08, method_id.as_usize() as u16),
    };
    let mut siz = 0;
    siz += le_u16(output, method_handle_type)?;
    siz += le_u16(output, 0)?;
    siz += le_u16(output, field_or_method_id)?;
    siz += le_u16(output, 0)?;
    Ok(siz)
}

fn map_list_writer<W: Write>(output: &mut W, list: &MapList) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, list.list.len() as u32)?;
    for item in &list.list {
        siz += map_item_writer(output, item)?;
    }
    Ok(siz)
}

fn map_item_writer<W: Write>(output: &mut W, item: &MapItem) -> Result<usize> {
    let v: u16 = match item.typ {
        MapItemType::HeaderItem => 0x0000,
        MapItemType::StringIdItem => 0x0001,
        MapItemType::TypeIdItem => 0x0002,
        MapItemType::ProtoIdItem => 0x0003,
        MapItemType::FieldIdItem => 0x0004,
        MapItemType::MethodIdItem => 0x0005,
        MapItemType::ClassDefItem => 0x0006,
        MapItemType::CallSiteIdItem => 0x0007,
        MapItemType::MethodHandleItem => 0x0008,
        MapItemType::MapList => 0x1000,
        MapItemType::TypeList => 0x1001,
        MapItemType::AnnotationSetRefList => 0x1002,
        MapItemType::AnnotationSetItem => 0x1003,
        MapItemType::ClassDataItem => 0x2000,
        MapItemType::CodeItem => 0x2001,
        MapItemType::StringDataItem => 0x2002,
        MapItemType::DebugInfoItem => 0x2003,
        MapItemType::AnnotationItem => 0x2004,
        MapItemType::EncodedArrayItem => 0x2005,
        MapItemType::AnnotationsDirectoryItem => 0x2006,
        MapItemType::HiddenapiClassDataItem => 0xF000,
    };
    let mut siz = 0;
    siz += le_u16(output, v)?;
    siz += le_u16(output, 0)?;
    siz += le_u32(output, item.size as u32)?;
    siz += le_u32(output, item.offset as u32)?;
    Ok(siz)
}

fn type_list_writer<W: Write + Seek>(output: &mut W, list: &TypeList) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, list.list.len() as u32)?;
    for item in &list.list {
        siz += type_item_writer(output, item)?;
    }
    Ok(siz)
}

fn type_item_writer<W: Write>(output: &mut W, item: &TypeItem) -> Result<usize> {
    le_u16(output, item.type_idx.as_usize() as u16)
}

fn annotation_set_ref_list_writer<W: Write>(
    output: &mut W,
    list: &AnnotationSetRefList,
) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, list.list.len() as u32)?;
    for item in &list.list {
        siz += annotation_set_ref_item_writer(output, item)?;
    }
    Ok(siz)
}

fn annotation_set_ref_item_writer<W: Write>(
    output: &mut W,
    item: &AnnotationSetRefItem,
) -> Result<usize> {
    le_u32(output, item.annotations_off.as_usize() as u32)
}

fn annotation_set_item_writer<W: Write>(output: &mut W, item: &AnnotationSetItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, item.entries.len() as u32)?;
    for entry in &item.entries {
        siz += annotation_off_item_writer(output, entry)?;
    }
    Ok(siz)
}

fn annotation_off_item_writer<W: Write>(output: &mut W, item: &AnnotationOffItem) -> Result<usize> {
    le_u32(output, item.annotation_off.as_usize() as u32)
}

fn class_data_item_writer<W: Write>(output: &mut W, item: &ClassDataItem) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, item.static_fields_size)?;
    siz += uleb128(output, item.instance_fields_size)?;
    siz += uleb128(output, item.direct_methods_size)?;
    siz += uleb128(output, item.virtual_methods_size)?;

    for field in &item.static_fields {
        siz += encoded_field_writer(output, field)?;
    }
    for field in &item.instance_fields {
        siz += encoded_field_writer(output, field)?;
    }
    for method in &item.direct_methods {
        siz += encoded_method_writer(output, method)?;
    }
    for method in &item.virtual_methods {
        siz += encoded_method_writer(output, method)?;
    }
    Ok(siz)
}

fn encoded_field_writer<W: Write>(output: &mut W, field: &EncodedField) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, field.field_idx_diff)?;
    siz += uleb128(output, field.access_flags_repr)?;
    Ok(siz)
}

fn encoded_method_writer<W: Write>(output: &mut W, method: &EncodedMethod) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, method.method_idx_diff)?;
    siz += uleb128(output, method.access_flags_repr)?;
    siz += uleb128(
        output,
        method
            .code_off
            .as_ref()
            .map_or(Uleb128::new(0, None), |off| off.as_uleb()),
    )?;
    Ok(siz)
}

fn code_item_writer<W: Write>(output: &mut W, item: &CodeItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u16(output, item.registers_size as u16)?;
    siz += le_u16(output, item.ins_size as u16)?;
    siz += le_u16(output, item.outs_size as u16)?;
    siz += le_u16(output, item.tries.len() as u16)?;
    siz += le_u32(
        output,
        item.debug_info_off
            .as_ref()
            .map_or(0, |off| off.as_usize() as u32),
    )?;
    let insns_size = item.insns.iter().map(|i| i.size() as u32).sum();
    siz += le_u32(output, insns_size)?;
    for instr in &item.insns {
        siz += write_instr(output, instr.instr())?;
    }
    if insns_size % 2 == 1 && !item.tries.is_empty() {
        siz += le_u32(output, 0x0000)?;
    }
    for trie in &item.tries {
        siz += try_item_writer(output, trie)?;
    }
    if let Some(list) = &item.handlers {
        siz += encoded_catch_handle_list_writer(output, list)?;
    }
    Ok(siz)
}

fn try_item_writer<W: Write>(output: &mut W, item: &TryItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, item.start_addr as u32)?;
    siz += le_u16(output, item.insn_count as u16)?;
    siz += le_u16(output, item.handler_off as u16)?;
    Ok(siz)
}

fn encoded_catch_handle_list_writer<W: Write>(
    output: &mut W,
    list: &EncodedCatchHandlerList,
) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, list.size)?;
    for handler in list.list.values() {
        siz += encoded_catch_handler_writer(output, handler)?;
    }
    Ok(siz)
}

fn encoded_catch_handler_writer<W: Write>(
    output: &mut W,
    handler: &EncodedCatchHandler,
) -> Result<usize> {
    let mut siz = 0;
    siz += sleb128(output, handler.size)?;
    for pair in &handler.handlers {
        siz += encoded_type_addr_pair_writer(output, pair)?;
    }
    if let Some(addr) = handler.catch_all_addr {
        siz += uleb128(output, addr)?;
    }
    Ok(siz)
}

fn encoded_type_addr_pair_writer<W: Write>(
    output: &mut W,
    pair: &EncodedTypeAddrPair,
) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, pair.type_idx.as_uleb())?;
    siz += uleb128(output, pair.addr)?;
    Ok(siz)
}

fn string_data_item_writer<W: Write>(output: &mut W, item: &StringDataItem) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, item.utf16_size)?;
    for c in &item.data {
        siz += le_u8(output, *c)?;
    }
    siz += le_u8(output, 0x00)?;
    Ok(siz)
}

fn debug_info_item_writer<W: Write>(output: &mut W, item: &DebugInfoItem) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, item.line_start)?;
    siz += uleb128(output, item.parameters_size)?;
    for parameter_name in &item.parameter_names {
        siz += uleb128p1(output, parameter_name.as_ref().map(|v| v.as_usize() as u32))?;
    }
    siz += debug_bytecode_writer(output, &item.bytecode)?;
    Ok(siz)
}

fn debug_bytecode_writer<W: Write>(output: &mut W, bc: &[DbgInstr]) -> Result<usize> {
    let mut siz = 0;
    for instr in bc {
        match instr {
            DbgInstr::EndSequence => {
                siz += le_u8(output, 0x00)?;
            }
            DbgInstr::AdvancePc { addr_diff } => {
                siz += le_u8(output, 0x01)?;
                siz += uleb128(output, *addr_diff)?;
            }
            DbgInstr::AdvanceLine { line_diff } => {
                siz += le_u8(output, 0x02)?;
                siz += sleb128(output, *line_diff)?;
            }
            DbgInstr::StartLocal {
                register_num,
                name_idx,
                type_idx,
            } => {
                siz += le_u8(output, 0x03)?;
                siz += uleb128(output, *register_num)?;
                siz += uleb128p1(output, name_idx.as_ref().map(|v| v.as_usize() as u32))?;
                siz += uleb128p1(output, type_idx.as_ref().map(|v| v.as_usize() as u32))?;
            }
            DbgInstr::StartLocalExtended {
                register_num,
                name_idx,
                type_idx,
                sig_idx,
            } => {
                siz += le_u8(output, 0x04)?;
                siz += uleb128(output, *register_num)?;
                siz += uleb128p1(output, name_idx.as_ref().map(|v| v.as_usize() as u32))?;
                siz += uleb128p1(output, type_idx.as_ref().map(|v| v.as_usize() as u32))?;
                siz += uleb128p1(output, sig_idx.as_ref().map(|v| v.as_usize() as u32))?;
            }
            DbgInstr::EndLocal { register_num } => {
                siz += le_u8(output, 0x05)?;
                siz += uleb128(output, *register_num)?;
            }
            DbgInstr::RestartLocal { register_num } => {
                siz += le_u8(output, 0x06)?;
                siz += uleb128(output, *register_num)?;
            }
            DbgInstr::SetPrologueEnd => {
                siz += le_u8(output, 0x07)?;
            }
            DbgInstr::SetEpilogueBegin => {
                siz += le_u8(output, 0x08)?;
            }
            DbgInstr::SetFile { name_idx } => {
                siz += le_u8(output, 0x09)?;
                siz += uleb128p1(output, name_idx.as_ref().map(|v| v.as_usize() as u32))?;
            }
            DbgInstr::Special(op) => {
                siz += le_u8(output, *op)?;
            }
        }
    }
    Ok(siz)
}

fn annotation_item_writer<W: Write>(output: &mut W, item: &AnnotationItem) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(
        output,
        match &item.visibility {
            Visibility::Build => 0x00,
            Visibility::Runtime => 0x01,
            Visibility::System => 0x02,
        },
    )?;
    siz += encoded_annotation_writer(output, &item.annotation)?;
    Ok(siz)
}

fn encoded_annotation_writer<W: Write>(output: &mut W, item: &EncodedAnnotation) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, item.type_idx.as_uleb())?;
    siz += uleb128(output, item.size)?;
    for elt in &item.elements {
        siz += annotation_element_writer(output, elt)?;
    }
    Ok(siz)
}

fn annotation_element_writer<W: Write>(
    output: &mut W,
    element: &AnnotationElement,
) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, element.name_idx.as_uleb())?;
    siz += encoded_value_writer(output, &element.value)?;
    Ok(siz)
}

fn encoded_value_writer<W: Write>(output: &mut W, value: &EncodedValue) -> Result<usize> {
    let mut siz = 0;
    match value {
        EncodedValue::Byte(b) => {
            siz += le_u8(output, 0x00)?;
            siz += le_i8(output, *b)?;
        }
        EncodedValue::Short(s, val) => {
            siz += le_u8(output, (1 << 5) + 0x02)?;
            siz += le_i16_on(output, *val, *s)?;
        }
        EncodedValue::Char(s, val) => {
            siz += le_u8(output, (1 << 5) + 0x03)?;
            siz += le_u16_on(output, *val, *s)?;
        }
        EncodedValue::Int(s, val) => {
            siz += le_u8(output, (3 << 5) + 0x04)?;
            siz += le_i32_on(output, *val, *s)?;
        }
        EncodedValue::Long(s, val) => {
            siz += le_u8(output, (7 << 5) + 0x06)?;
            siz += le_i64_on(output, *val, *s)?;
        }
        EncodedValue::Float(s, val) => {
            siz += le_u8(output, (3 << 5) + 0x10)?;
            siz += le_f32_on(output, *val, *s)?;
        }
        EncodedValue::Double(s, val) => {
            siz += le_u8(output, (7 << 5) + 0x11)?;
            siz += le_f64_on(output, *val, *s)?;
        }
        EncodedValue::MethodType(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x15)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::MethodHandle(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x16)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::String(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x17)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::Type(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x18)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::Field(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x19)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::Method(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x1a)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::Enum(s, idx) => {
            siz += le_u8(output, (3 << 5) + 0x1b)?;
            siz += le_u32_on(output, idx.as_usize() as u32, *s)?;
        }
        EncodedValue::Array(vs) => {
            siz += le_u8(output, 0x1c)?;
            siz += encoded_array_writer(output, vs)?;
        }
        EncodedValue::Annotation(a) => {
            siz += le_u8(output, 0x1d)?;
            siz += encoded_annotation_writer(output, a)?;
        }
        EncodedValue::Null => {
            siz += le_u8(output, 0x1e)?;
        }
        EncodedValue::Boolean(b) => {
            if *b {
                siz += le_u8(output, (1 << 5) + 0x1f)?;
            } else {
                siz += le_u8(output, 0x1f)?;
            }
        }
    }
    Ok(siz)
}

fn encoded_array_item_writer<W: Write>(output: &mut W, item: &EncodedArrayItem) -> Result<usize> {
    encoded_array_writer(output, &item.value)
}

fn encoded_array_writer<W: Write>(output: &mut W, array: &EncodedArray) -> Result<usize> {
    let mut siz = 0;
    siz += uleb128(output, array.size)?;
    for value in &array.values {
        siz += encoded_value_writer(output, value)?;
    }
    Ok(siz)
}

fn annotations_director_item_writer<W: Write>(
    output: &mut W,
    item: &AnnotationsDirectoryItem,
) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, item.class_annotations_off.as_usize() as u32)?;
    siz += le_u32(output, item.field_annotations.len() as u32)?;
    siz += le_u32(output, item.method_annotations.len() as u32)?;
    siz += le_u32(output, item.parameter_annotations.len() as u32)?;
    for annot in &item.field_annotations {
        siz += field_annotation_writer(output, annot)?;
    }
    for annot in &item.method_annotations {
        siz += method_annotation_writer(output, annot)?;
    }
    for annot in &item.parameter_annotations {
        siz += parameter_annotation_writer(output, annot)?;
    }
    Ok(siz)
}

fn field_annotation_writer<W: Write>(
    output: &mut W,
    annotation: &FieldAnnotation,
) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, annotation.field_idx.as_usize() as u32)?;
    siz += le_u32(output, annotation.annotations_off.as_usize() as u32)?;
    Ok(siz)
}

fn method_annotation_writer<W: Write>(
    output: &mut W,
    annotation: &MethodAnnotation,
) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, annotation.method_idx.as_usize() as u32)?;
    siz += le_u32(output, annotation.annotations_off.as_usize() as u32)?;
    Ok(siz)
}

fn parameter_annotation_writer<W: Write>(
    output: &mut W,
    annotation: &ParameterAnnotation,
) -> Result<usize> {
    let mut siz = 0;
    siz += le_u32(output, annotation.method_idx.as_usize() as u32)?;
    siz += le_u32(output, annotation.annotations_off.as_usize() as u32)?;
    Ok(siz)
}

fn hiddenapi_class_data_item_writer<W: Write>(
    output: &mut W,
    item: &HiddenapiClassDataItem,
) -> Result<usize> {
    let mut siz = 0;
    for offset in &item.offsets {
        siz += le_u32(output, offset.as_usize() as u32)?;
    }
    for flag in item.flags.values() {
        siz += uleb128(output, flag.uleb_repr)?;
    }
    Ok(siz)
}

fn write_instr<W: Write>(output: &mut W, instr: &Instr) -> Result<usize> {
    let mut siz = 0;
    match instr {
        Instr::Nop => siz += le_u16(output, 0x0000)?,
        Instr::PackedSwitchPayload(a, b) => {
            siz += le_u16(output, 0x0100)?;
            siz += le_u16(output, b.len() as u16)?;
            siz += le_i32(output, *a)?;
            for v in b {
                siz += le_i32(output, *v)?;
            }
        }
        Instr::SparseSwitchPayload(a, b) => {
            siz += le_u16(output, 0x0200)?;
            siz += le_u16(output, a.len() as u16)?;
            for v in a {
                siz += le_i32(output, *v)?;
            }
            for v in b {
                siz += le_i32(output, *v)?;
            }
        }
        Instr::FillArrayDataPayload(data) => {
            siz += le_u16(output, 0x0300)?;
            siz += le_u16(output, data[0].len() as u16)?;
            siz += le_u32(output, data.len() as u32)?;
            for elt in data {
                for c in elt {
                    siz += le_u8(output, *c)?;
                }
            }
        }
        Instr::Move(a, b) => {
            siz += le_u8(output, 0x01)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MoveFrom16(a, b) => {
            siz += le_u8(output, 0x02)?;
            siz += write_22x(output, u8::try_from(*a)?, u16::from(*b))?;
        }
        Instr::Move16(a, b) => {
            siz += le_u8(output, 0x03)?;
            siz += write_32x(output, u16::from(*a), u16::from(*b))?;
        }
        Instr::MoveWide(a, b) => {
            siz += le_u8(output, 0x04)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MoveWideFrom16(a, b) => {
            siz += le_u8(output, 0x05)?;
            siz += write_22x(output, u8::try_from(*a)?, u16::from(*b))?;
        }
        Instr::MoveWide16(a, b) => {
            siz += le_u8(output, 0x06)?;
            siz += write_32x(output, u16::from(*a), u16::from(*b))?;
        }
        Instr::MoveObject(a, b) => {
            siz += le_u8(output, 0x07)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MoveObjectFrom16(a, b) => {
            siz += le_u8(output, 0x08)?;
            siz += write_22x(output, u8::try_from(*a)?, u16::from(*b))?;
        }
        Instr::MoveObject16(a, b) => {
            siz += le_u8(output, 0x09)?;
            siz += write_32x(output, u16::from(*a), u16::from(*b))?;
        }
        Instr::MoveResult(a) => {
            siz += le_u8(output, 0x0a)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::MoveResultWide(a) => {
            siz += le_u8(output, 0x0b)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::MoveResultObject(a) => {
            siz += le_u8(output, 0x0c)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::MoveException(a) => {
            siz += le_u8(output, 0x0d)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::ReturnVoid => {
            siz += le_u8(output, 0x0e)?;
            siz += write_10x(output)?;
        }
        Instr::Return(a) => {
            siz += le_u8(output, 0x0f)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::ReturnWide(a) => {
            siz += le_u8(output, 0x10)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::ReturnObject(a) => {
            siz += le_u8(output, 0x11)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::Const4(a, b) => {
            siz += le_u8(output, 0x12)?;
            siz += write_11n(output, u8::try_from(*a)?, *b)?;
        }
        Instr::Const16(a, b) => {
            siz += le_u8(output, 0x13)?;
            siz += write_21s(output, u8::try_from(*a)?, *b)?;
        }
        Instr::Const(a, b) => {
            siz += le_u8(output, 0x14)?;
            siz += write_31i(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstHigh16(a, b) => {
            siz += le_u8(output, 0x15)?;
            siz += write_21h(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstWide16(a, b) => {
            siz += le_u8(output, 0x16)?;
            siz += write_21s(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstWide32(a, b) => {
            siz += le_u8(output, 0x17)?;
            siz += write_31i(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstWide(a, b) => {
            siz += le_u8(output, 0x18)?;
            siz += write_51l(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstWideHigh16(a, b) => {
            siz += le_u8(output, 0x19)?;
            siz += write_21h(output, u8::try_from(*a)?, *b)?;
        }
        Instr::ConstString(a, b) => {
            siz += le_u8(output, 0x1a)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::ConstStringJumbo(a, b) => {
            siz += le_u8(output, 0x1b)?;
            siz += write_31c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::ConstClass(a, b) => {
            siz += le_u8(output, 0x1c)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::MonitorEnter(a) => {
            siz += le_u8(output, 0x1d)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::MonitorExit(a) => {
            siz += le_u8(output, 0x1e)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::CheckCast(a, b) => {
            siz += le_u8(output, 0x1f)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::InstanceOf(a, b, c) => {
            siz += le_u8(output, 0x20)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::ArrayLength(a, b) => {
            siz += le_u8(output, 0x21)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NewInstance(a, b) => {
            siz += le_u8(output, 0x22)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::NewArray(a, b, c) => {
            siz += le_u8(output, 0x23)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::FilledNewArray(a, b) => {
            siz += le_u8(output, 0x24)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::FilledNewArrayRange(rr, c) => {
            siz += le_u8(output, 0x25)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::FillArrayData(a, b) => {
            siz += le_u8(output, 0x26)?;
            siz += write_31t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::Throw(a) => {
            siz += le_u8(output, 0x27)?;
            siz += write_11x(output, u8::try_from(*a)?)?;
        }
        Instr::Goto(a) => {
            siz += le_u8(output, 0x28)?;
            siz += write_10t(output, *a)?;
        }
        Instr::Goto16(a) => {
            siz += le_u8(output, 0x29)?;
            siz += write_20t(output, *a)?;
        }
        Instr::Goto32(a) => {
            siz += le_u8(output, 0x2a)?;
            siz += write_30t(output, *a)?;
        }
        Instr::PackedSwitch(a, b) => {
            siz += le_u8(output, 0x2b)?;
            siz += write_31t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::SparseSwitch(a, b) => {
            siz += le_u8(output, 0x2c)?;
            siz += write_31t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::CmplFloat(a, b, c) => {
            siz += le_u8(output, 0x2d)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::CmpgFloat(a, b, c) => {
            siz += le_u8(output, 0x2e)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::CmplDouble(a, b, c) => {
            siz += le_u8(output, 0x2f)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::CmpgDouble(a, b, c) => {
            siz += le_u8(output, 0x30)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::CmpLong(a, b, c) => {
            siz += le_u8(output, 0x31)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::IfEq(a, b, c) => {
            siz += le_u8(output, 0x32)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfNe(a, b, c) => {
            siz += le_u8(output, 0x33)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfLt(a, b, c) => {
            siz += le_u8(output, 0x34)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfGe(a, b, c) => {
            siz += le_u8(output, 0x35)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfGt(a, b, c) => {
            siz += le_u8(output, 0x36)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfLe(a, b, c) => {
            siz += le_u8(output, 0x37)?;
            siz += write_22t(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::IfEqz(a, b) => {
            siz += le_u8(output, 0x38)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::IfNez(a, b) => {
            siz += le_u8(output, 0x39)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::IfLtz(a, b) => {
            siz += le_u8(output, 0x3a)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::IfGez(a, b) => {
            siz += le_u8(output, 0x3b)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::IfGtz(a, b) => {
            siz += le_u8(output, 0x3c)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::IfLez(a, b) => {
            siz += le_u8(output, 0x3d)?;
            siz += write_21t(output, u8::try_from(*a)?, *b)?;
        }
        Instr::Aget(a, b, c) => {
            siz += le_u8(output, 0x44)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetWide(a, b, c) => {
            siz += le_u8(output, 0x45)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetObject(a, b, c) => {
            siz += le_u8(output, 0x46)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetBoolean(a, b, c) => {
            siz += le_u8(output, 0x47)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetByte(a, b, c) => {
            siz += le_u8(output, 0x48)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetChar(a, b, c) => {
            siz += le_u8(output, 0x49)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AgetShort(a, b, c) => {
            siz += le_u8(output, 0x4a)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::Aput(a, b, c) => {
            siz += le_u8(output, 0x4b)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputWide(a, b, c) => {
            siz += le_u8(output, 0x4c)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputObject(a, b, c) => {
            siz += le_u8(output, 0x4d)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputBoolean(a, b, c) => {
            siz += le_u8(output, 0x4e)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputByte(a, b, c) => {
            siz += le_u8(output, 0x4f)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputChar(a, b, c) => {
            siz += le_u8(output, 0x50)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AputShort(a, b, c) => {
            siz += le_u8(output, 0x51)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::Iget(a, b, c) => {
            siz += le_u8(output, 0x52)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetWide(a, b, c) => {
            siz += le_u8(output, 0x53)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetObject(a, b, c) => {
            siz += le_u8(output, 0x54)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetBoolean(a, b, c) => {
            siz += le_u8(output, 0x55)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetByte(a, b, c) => {
            siz += le_u8(output, 0x56)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetChar(a, b, c) => {
            siz += le_u8(output, 0x57)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IgetShort(a, b, c) => {
            siz += le_u8(output, 0x58)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::Iput(a, b, c) => {
            siz += le_u8(output, 0x59)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputWide(a, b, c) => {
            siz += le_u8(output, 0x5a)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputObject(a, b, c) => {
            siz += le_u8(output, 0x5b)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputBoolean(a, b, c) => {
            siz += le_u8(output, 0x5c)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputByte(a, b, c) => {
            siz += le_u8(output, 0x5d)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputChar(a, b, c) => {
            siz += le_u8(output, 0x5e)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::IputShort(a, b, c) => {
            siz += le_u8(output, 0x5f)?;
            siz += write_22c(output, u8::try_from(*a)?, u8::try_from(*b)?, c.as_usize())?;
        }
        Instr::Sget(a, b) => {
            siz += le_u8(output, 0x60)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetWide(a, b) => {
            siz += le_u8(output, 0x61)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetObject(a, b) => {
            siz += le_u8(output, 0x62)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetBoolean(a, b) => {
            siz += le_u8(output, 0x63)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetByte(a, b) => {
            siz += le_u8(output, 0x64)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetChar(a, b) => {
            siz += le_u8(output, 0x65)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SgetShort(a, b) => {
            siz += le_u8(output, 0x66)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::Sput(a, b) => {
            siz += le_u8(output, 0x67)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputWide(a, b) => {
            siz += le_u8(output, 0x68)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputObject(a, b) => {
            siz += le_u8(output, 0x69)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputBoolean(a, b) => {
            siz += le_u8(output, 0x6a)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputByte(a, b) => {
            siz += le_u8(output, 0x6b)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputChar(a, b) => {
            siz += le_u8(output, 0x6c)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::SputShort(a, b) => {
            siz += le_u8(output, 0x6d)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::InvokeVirtual(a, b) => {
            siz += le_u8(output, 0x6e)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeSuper(a, b) => {
            siz += le_u8(output, 0x6f)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeDirect(a, b) => {
            siz += le_u8(output, 0x70)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeStatic(a, b) => {
            siz += le_u8(output, 0x71)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeInterface(a, b) => {
            siz += le_u8(output, 0x72)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeVirtualRange(rr, c) => {
            siz += le_u8(output, 0x74)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::InvokeSuperRange(rr, c) => {
            siz += le_u8(output, 0x75)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::InvokeDirectRange(rr, c) => {
            siz += le_u8(output, 0x76)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::InvokeStaticRange(rr, c) => {
            siz += le_u8(output, 0x77)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::InvokeInterfaceRange(rr, c) => {
            siz += le_u8(output, 0x78)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::NegInt(a, b) => {
            siz += le_u8(output, 0x7b)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NotInt(a, b) => {
            siz += le_u8(output, 0x7c)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NegLong(a, b) => {
            siz += le_u8(output, 0x7d)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NotLong(a, b) => {
            siz += le_u8(output, 0x7e)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NegFloat(a, b) => {
            siz += le_u8(output, 0x7f)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::NegDouble(a, b) => {
            siz += le_u8(output, 0x80)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToLong(a, b) => {
            siz += le_u8(output, 0x81)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToFloat(a, b) => {
            siz += le_u8(output, 0x82)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToDouble(a, b) => {
            siz += le_u8(output, 0x83)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::LongToInt(a, b) => {
            siz += le_u8(output, 0x84)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::LongToFloat(a, b) => {
            siz += le_u8(output, 0x85)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::LongToDouble(a, b) => {
            siz += le_u8(output, 0x86)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::FloatToInt(a, b) => {
            siz += le_u8(output, 0x87)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::FloatToLong(a, b) => {
            siz += le_u8(output, 0x88)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::FloatToDouble(a, b) => {
            siz += le_u8(output, 0x89)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DoubleToInt(a, b) => {
            siz += le_u8(output, 0x8a)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DoubleToLong(a, b) => {
            siz += le_u8(output, 0x8b)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DoubleToFloat(a, b) => {
            siz += le_u8(output, 0x8c)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToByte(a, b) => {
            siz += le_u8(output, 0x8d)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToChar(a, b) => {
            siz += le_u8(output, 0x8e)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::IntToShort(a, b) => {
            siz += le_u8(output, 0x8f)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AddInt(a, b, c) => {
            siz += le_u8(output, 0x90)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::SubInt(a, b, c) => {
            siz += le_u8(output, 0x91)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::MulInt(a, b, c) => {
            siz += le_u8(output, 0x92)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::DivInt(a, b, c) => {
            siz += le_u8(output, 0x93)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::RemInt(a, b, c) => {
            siz += le_u8(output, 0x94)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AndInt(a, b, c) => {
            siz += le_u8(output, 0x95)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::OrInt(a, b, c) => {
            siz += le_u8(output, 0x96)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::XorInt(a, b, c) => {
            siz += le_u8(output, 0x97)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::ShlInt(a, b, c) => {
            siz += le_u8(output, 0x98)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::ShrInt(a, b, c) => {
            siz += le_u8(output, 0x99)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::UshrInt(a, b, c) => {
            siz += le_u8(output, 0x9a)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AddLong(a, b, c) => {
            siz += le_u8(output, 0x9b)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::SubLong(a, b, c) => {
            siz += le_u8(output, 0x9c)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::MulLong(a, b, c) => {
            siz += le_u8(output, 0x9d)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::DivLong(a, b, c) => {
            siz += le_u8(output, 0x9e)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::RemLong(a, b, c) => {
            siz += le_u8(output, 0x9f)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AndLong(a, b, c) => {
            siz += le_u8(output, 0xa0)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::OrLong(a, b, c) => {
            siz += le_u8(output, 0xa1)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::XorLong(a, b, c) => {
            siz += le_u8(output, 0xa2)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::ShlLong(a, b, c) => {
            siz += le_u8(output, 0xa3)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::ShrLong(a, b, c) => {
            siz += le_u8(output, 0xa4)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::UshrLong(a, b, c) => {
            siz += le_u8(output, 0xa5)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AddFloat(a, b, c) => {
            siz += le_u8(output, 0xa6)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::SubFloat(a, b, c) => {
            siz += le_u8(output, 0xa7)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::MulFloat(a, b, c) => {
            siz += le_u8(output, 0xa8)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::DivFloat(a, b, c) => {
            siz += le_u8(output, 0xa9)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::RemFloat(a, b, c) => {
            siz += le_u8(output, 0xaa)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AddDouble(a, b, c) => {
            siz += le_u8(output, 0xab)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::SubDouble(a, b, c) => {
            siz += le_u8(output, 0xac)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::MulDouble(a, b, c) => {
            siz += le_u8(output, 0xad)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::DivDouble(a, b, c) => {
            siz += le_u8(output, 0xae)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::RemDouble(a, b, c) => {
            siz += le_u8(output, 0xaf)?;
            siz += write_23x(
                output,
                u8::try_from(*a)?,
                u8::try_from(*b)?,
                u8::try_from(*c)?,
            )?;
        }
        Instr::AddInt2addr(a, b) => {
            siz += le_u8(output, 0xb0)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::SubInt2addr(a, b) => {
            siz += le_u8(output, 0xb1)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MulInt2addr(a, b) => {
            siz += le_u8(output, 0xb2)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DivInt2addr(a, b) => {
            siz += le_u8(output, 0xb3)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::RemInt2addr(a, b) => {
            siz += le_u8(output, 0xb4)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AndInt2addr(a, b) => {
            siz += le_u8(output, 0xb5)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::OrInt2addr(a, b) => {
            siz += le_u8(output, 0xb6)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::XorInt2addr(a, b) => {
            siz += le_u8(output, 0xb7)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::ShlInt2addr(a, b) => {
            siz += le_u8(output, 0xb8)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::ShrInt2addr(a, b) => {
            siz += le_u8(output, 0xb9)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::UshrInt2addr(a, b) => {
            siz += le_u8(output, 0xba)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AddLong2addr(a, b) => {
            siz += le_u8(output, 0xbb)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::SubLong2addr(a, b) => {
            siz += le_u8(output, 0xbc)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MulLong2addr(a, b) => {
            siz += le_u8(output, 0xbd)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DivLong2addr(a, b) => {
            siz += le_u8(output, 0xbe)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::RemLong2addr(a, b) => {
            siz += le_u8(output, 0xbf)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AndLong2addr(a, b) => {
            siz += le_u8(output, 0xc0)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::OrLong2addr(a, b) => {
            siz += le_u8(output, 0xc1)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::XorLong2addr(a, b) => {
            siz += le_u8(output, 0xc2)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::ShlLong2addr(a, b) => {
            siz += le_u8(output, 0xc3)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::ShrLong2addr(a, b) => {
            siz += le_u8(output, 0xc4)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::UshrLong2addr(a, b) => {
            siz += le_u8(output, 0xc5)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AddFloat2addr(a, b) => {
            siz += le_u8(output, 0xc6)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::SubFloat2addr(a, b) => {
            siz += le_u8(output, 0xc7)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MulFloat2addr(a, b) => {
            siz += le_u8(output, 0xc8)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DivFloat2addr(a, b) => {
            siz += le_u8(output, 0xc9)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::RemFloat2addr(a, b) => {
            siz += le_u8(output, 0xca)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AddDouble2addr(a, b) => {
            siz += le_u8(output, 0xcb)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::SubDouble2addr(a, b) => {
            siz += le_u8(output, 0xcc)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::MulDouble2addr(a, b) => {
            siz += le_u8(output, 0xcd)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::DivDouble2addr(a, b) => {
            siz += le_u8(output, 0xce)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::RemDouble2addr(a, b) => {
            siz += le_u8(output, 0xcf)?;
            siz += write_12x(output, u8::try_from(*a)?, u8::try_from(*b)?)?;
        }
        Instr::AddIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd0)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::RsubInt(a, b, c) => {
            siz += le_u8(output, 0xd1)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::MulIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd2)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::DivIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd3)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::RemIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd4)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::AndIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd5)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::OrIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd6)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::XorIntLit16(a, b, c) => {
            siz += le_u8(output, 0xd7)?;
            siz += write_22s(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::AddIntLit8(a, b, c) => {
            siz += le_u8(output, 0xd8)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::RsubIntLit8(a, b, c) => {
            siz += le_u8(output, 0xd9)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::MulIntLit8(a, b, c) => {
            siz += le_u8(output, 0xda)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::DivIntLit8(a, b, c) => {
            siz += le_u8(output, 0xdb)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::RemIntLit8(a, b, c) => {
            siz += le_u8(output, 0xdc)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::AndIntLit8(a, b, c) => {
            siz += le_u8(output, 0xdd)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::OrIntLit8(a, b, c) => {
            siz += le_u8(output, 0xde)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::XorIntLit8(a, b, c) => {
            siz += le_u8(output, 0xdf)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::ShlIntLit8(a, b, c) => {
            siz += le_u8(output, 0xe0)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::ShrIntLit8(a, b, c) => {
            siz += le_u8(output, 0xe1)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::UshrIntLit8(a, b, c) => {
            siz += le_u8(output, 0xe2)?;
            siz += write_22b(output, u8::try_from(*a)?, u8::try_from(*b)?, *c)?;
        }
        Instr::InvokePolymorphic(a, b, c) => {
            siz += le_u8(output, 0xfa)?;
            siz += write_45cc(
                output,
                &<Vec<u8>>::try_from(a.clone())?,
                b.as_usize(),
                c.as_usize(),
            )?;
        }
        Instr::InvokePolymorphicRange(rr, c, d) => {
            siz += le_u8(output, 0xfb)?;
            siz += write_4rcc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
                d.as_usize(),
            )?;
        }
        Instr::InvokeCustom(a, b) => {
            siz += le_u8(output, 0xfc)?;
            siz += write_35c(output, &<Vec<u8>>::try_from(a.clone())?, b.as_usize())?;
        }
        Instr::InvokeCustomRange(rr, c) => {
            siz += le_u8(output, 0xfd)?;
            siz += write_3rc(
                output,
                u16::from(*rr.begin()),
                u16::from(*rr.end()),
                c.as_usize(),
            )?;
        }
        Instr::ConstMethodHandle(a, b) => {
            siz += le_u8(output, 0xfe)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
        Instr::ConstMethodType(a, b) => {
            siz += le_u8(output, 0xff)?;
            siz += write_21c(output, u8::try_from(*a)?, b.as_usize())?;
        }
    }
    Ok(siz)
}

fn write_10x<W: Write>(output: &mut W) -> Result<usize> {
    le_u8(output, 0x00)
}

fn write_12x<W: Write>(output: &mut W, a: u8, b: u8) -> Result<usize> {
    assert!(a <= 0xf);
    assert!(b <= 0xf);
    le_u8(output, (b << 4) + a)
}

fn write_11n<W: Write>(output: &mut W, a: u8, b: i8) -> Result<usize> {
    assert!(a <= 0xf);
    assert!(b <= 0xf);
    le_u8(output, ((b as u8) << 4) + a)
}

fn write_11x<W: Write>(output: &mut W, a: u8) -> Result<usize> {
    le_u8(output, a)
}

fn write_10t<W: Write>(output: &mut W, a: i8) -> Result<usize> {
    le_i8(output, a)
}

fn write_20t<W: Write>(output: &mut W, a: i16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, 0x00)?;
    siz += le_i16(output, a)?;
    Ok(siz)
}

fn write_22x<W: Write>(output: &mut W, a: u8, b: u16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_u16(output, b)?;
    Ok(siz)
}

fn write_21t<W: Write>(output: &mut W, a: u8, b: i16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i16(output, b)?;
    Ok(siz)
}

fn write_21s<W: Write>(output: &mut W, a: u8, b: i16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i16(output, b)?;
    Ok(siz)
}

fn write_21h<W: Write>(output: &mut W, a: u8, b: i16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i16(output, b)?;
    Ok(siz)
}

fn write_21c<W: Write>(output: &mut W, a: u8, b: usize) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_u16(output, b as u16)?;
    Ok(siz)
}

fn write_23x<W: Write>(output: &mut W, a: u8, b: u8, c: u8) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_u8(output, b)?;
    siz += le_u8(output, c)?;
    Ok(siz)
}

fn write_22b<W: Write>(output: &mut W, a: u8, b: u8, c: i8) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_u8(output, b)?;
    siz += le_i8(output, c)?;
    Ok(siz)
}

fn write_22t<W: Write>(output: &mut W, a: u8, b: u8, c: i16) -> Result<usize> {
    assert!(a <= 0xf);
    assert!(b <= 0xf);
    let mut siz = 0;
    siz += le_u8(output, (b << 4) + a)?;
    siz += le_i16(output, c)?;
    Ok(siz)
}

fn write_22s<W: Write>(output: &mut W, a: u8, b: u8, c: i16) -> Result<usize> {
    assert!(a <= 0xf);
    assert!(b <= 0xf);
    let mut siz = 0;
    siz += le_u8(output, (b << 4) + a)?;
    siz += le_i16(output, c)?;
    Ok(siz)
}

fn write_22c<W: Write>(output: &mut W, a: u8, b: u8, c: usize) -> Result<usize> {
    assert!(a <= 0xf);
    assert!(b <= 0xf);
    let mut siz = 0;
    siz += le_u8(output, (b << 4) + a)?;
    siz += le_u16(output, c as u16)?;
    Ok(siz)
}

fn write_30t<W: Write>(output: &mut W, a: i32) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, 0x00)?;
    siz += le_i32(output, a)?;
    Ok(siz)
}

fn write_32x<W: Write>(output: &mut W, a: u16, b: u16) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, 0x00)?;
    siz += le_u16(output, a)?;
    siz += le_u16(output, b)?;
    Ok(siz)
}

fn write_31i<W: Write>(output: &mut W, a: u8, b: i32) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i32(output, b)?;
    Ok(siz)
}

fn write_31t<W: Write>(output: &mut W, a: u8, b: i32) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i32(output, b)?;
    Ok(siz)
}

fn write_31c<W: Write>(output: &mut W, a: u8, b: usize) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_u32(output, b as u32)?;
    Ok(siz)
}

// allow single char names so that they match the specifications
#[allow(clippy::many_single_char_names)]
fn write_35c<W: Write>(output: &mut W, vs: &[u8], b: usize) -> Result<usize> {
    let a = vs.len() as u8;
    let c = *vs.first().unwrap_or(&0);
    let d = *vs.get(1).unwrap_or(&0);
    let e = *vs.get(2).unwrap_or(&0);
    let f = *vs.get(3).unwrap_or(&0);
    let g = *vs.get(4).unwrap_or(&0);
    assert!(a <= 0xf);
    assert!(c <= 0xf);
    assert!(d <= 0xf);
    assert!(e <= 0xf);
    assert!(f <= 0xf);
    assert!(g <= 0xf);
    let mut siz = 0;
    siz += le_u8(output, (a << 4) + g)?;
    siz += le_u16(output, b as u16)?;
    siz += le_u8(output, (d << 4) + c)?;
    siz += le_u8(output, (f << 4) + e)?;
    Ok(siz)
}

fn write_3rc<W: Write>(output: &mut W, c: u16, n: u16, b: usize) -> Result<usize> {
    let a = n + 1 - c;
    let mut siz = 0;
    siz += le_u8(output, a as u8)?;
    siz += le_u16(output, b as u16)?;
    siz += le_u16(output, c)?;
    Ok(siz)
}

// allow single char names so that they match the specifications
#[allow(clippy::many_single_char_names)]
fn write_45cc<W: Write>(output: &mut W, vs: &[u8], b: usize, h: usize) -> Result<usize> {
    let a = vs.len() as u8;
    let c = *vs.first().unwrap_or(&0);
    let d = *vs.get(1).unwrap_or(&0);
    let e = *vs.get(2).unwrap_or(&0);
    let f = *vs.get(3).unwrap_or(&0);
    let g = *vs.get(4).unwrap_or(&0);
    assert!(a <= 0xf);
    assert!(c <= 0xf);
    assert!(d <= 0xf);
    assert!(e <= 0xf);
    assert!(f <= 0xf);
    assert!(g <= 0xf);
    let mut siz = 0;
    siz += le_u8(output, (a << 4) + g)?;
    siz += le_u16(output, b as u16)?;
    siz += le_u8(output, (d << 4) + c)?;
    siz += le_u8(output, (f << 4) + e)?;
    siz += le_u16(output, h as u16)?;
    Ok(siz)
}

// allow single char names so that they match the specifications
#[allow(clippy::many_single_char_names)]
fn write_4rcc<W: Write>(output: &mut W, c: u16, n: u16, b: usize, h: usize) -> Result<usize> {
    let a = n + 1 - c;
    let mut siz = 0;
    siz += le_u8(output, a as u8)?;
    siz += le_u16(output, b as u16)?;
    siz += le_u16(output, c)?;
    siz += le_u16(output, h as u16)?;
    Ok(siz)
}

fn write_51l<W: Write>(output: &mut W, a: u8, b: i64) -> Result<usize> {
    let mut siz = 0;
    siz += le_u8(output, a)?;
    siz += le_i64(output, b)?;
    Ok(siz)
}
