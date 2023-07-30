use crate::errors::{ResourcesError, ResourcesResult};
use crate::resources::Resources;
use crate::strings::StringPool;
use crate::values::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TablePackagePoolIndex(usize);

impl TablePackagePoolIndex {
    pub(crate) const fn new(idx: usize) -> Self {
        Self(idx)
    }
}

#[derive(Debug)]
pub(crate) struct TablePackagePool {
    packages: Vec<Arc<TablePackage>>,
    index: BTreeMap<u8, usize>,
}

impl TablePackagePool {
    pub(crate) fn new(packages: Vec<Arc<TablePackage>>) -> ResourcesResult<Self> {
        let mut index = BTreeMap::new();

        for package in &packages {
            if index.insert(package.id, package.self_ref.0).is_some() {
                return Err(ResourcesError::ResAlreadyDefined(format!(
                    "package {:#x}",
                    package.id
                )));
            }
        }

        Ok(Self { packages, index })
    }

    pub(crate) const fn packages(&self) -> &Vec<Arc<TablePackage>> {
        &self.packages
    }

    pub(crate) fn resolve(&self, package_id: u8) -> Option<Arc<TablePackage>> {
        self.index
            .get(&package_id)
            .and_then(|idx| self.packages.get(*idx))
            .map(Arc::clone)
    }

    pub(crate) fn get(&self, id: TablePackagePoolIndex) -> ResourcesResult<Arc<TablePackage>> {
        self.packages
            .get(id.0)
            .ok_or_else(|| ResourcesError::ResNotFound("TablePackage by raw index".to_string()))
            .map(Arc::clone)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TableTypePoolIndex(usize);

impl TableTypePoolIndex {
    pub(crate) const fn new(idx: usize) -> Self {
        Self(idx)
    }
}

#[derive(Debug)]
pub(crate) struct TableTypePool {
    types: Vec<Arc<TableType>>,
    index: BTreeMap<u8, BTreeSet<usize>>,
}

impl TableTypePool {
    pub(crate) fn new(types: Vec<Arc<TableType>>) -> Self {
        let mut index = BTreeMap::new();

        for typ in &types {
            match index.get_mut(&typ.id) {
                None => {
                    let mut indexes = BTreeSet::new();
                    let _opt = indexes.insert(typ.self_ref.0);
                    let _opt = index.insert(typ.id, indexes);
                }
                Some(indexes) => {
                    let _opt = indexes.insert(typ.self_ref.0);
                }
            }
        }

        Self { types, index }
    }

    pub(crate) const fn types(&self) -> &Vec<Arc<TableType>> {
        &self.types
    }

    pub(crate) fn resolve(&self, type_id: u8) -> Option<Vec<Arc<TableType>>> {
        self.index.get(&type_id).map(|set| {
            set.iter()
                .fold(Vec::with_capacity(set.len()), |mut res, idx| {
                    res.push(self.types.get(*idx).unwrap().clone());
                    res
                })
        })
    }

    pub(crate) fn get(&self, id: TableTypePoolIndex) -> ResourcesResult<Arc<TableType>> {
        self.types
            .get(id.0)
            .ok_or_else(|| ResourcesError::ResNotFound("TableType by raw index".to_string()))
            .map(Arc::clone)
    }
}

#[derive(Debug)]
pub(crate) struct TablePackage {
    pub(crate) self_ref: TablePackagePoolIndex,
    pub(crate) id: u8, //is a u32 in serialized version
    pub(crate) name: String,
    pub(crate) last_public_type: u32,
    pub(crate) last_public_key: u32,
    pub(crate) type_strings: Option<StringPool>,
    pub(crate) key_strings: Option<StringPool>,
    pub(crate) string_pools: Vec<StringPool>,
    pub(crate) table_type_specs: Vec<TableTypeSpec>,
    pub(crate) type_pool: TableTypePool,
    pub(crate) table_libraries: Vec<TableLibrary>,
    pub(crate) table_overlayables: Vec<TableOverlayable>,
    pub(crate) table_overlayable_policies: Vec<TableOverlayablePolicy>,
    pub(crate) table_staged_aliases: Vec<TableStagedAlias>,
}

impl TablePackage {
    pub fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
    ) -> ResourcesResult<()> {
        writeln!(f, "package name={} id={:0>2x}", self.name, self.id)?;
        for t in self.type_pool.types() {
            t.pretty_print(f, resources, self)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TableTypeSpec {
    pub(crate) id: u8,
    pub(crate) config_mask: Vec<u32>,
}

#[derive(Debug)]
pub(crate) struct TableType {
    pub(crate) self_ref: TableTypePoolIndex,
    pub(crate) id: u8,
    pub(crate) config: Config,
    pub(crate) entry_pool: TableTypeEntryPool,
}

impl TableType {
    fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
        package: &TablePackage,
    ) -> ResourcesResult<()> {
        let real_count = self.entry_pool.entries().len();

        //let opt_typ_spec = package.table_type_specs.get((self.id - 1) as usize);
        let opt_typ = package
            .type_strings
            .as_ref()
            .and_then(|ts| ts.strings.get((self.id - 1) as usize));

        write!(f, "  type ")?;
        if let Some(typ) = opt_typ {
            write!(f, "{typ}")?;
        } else {
            write!(f, "?????")?;
        }
        writeln!(f, " id={:0>2x} entryCount={}", self.id, real_count)?;

        for (id, e) in self.entry_pool.entries() {
            e.pretty_print(f, resources, package, self, *id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TableTypeEntryPoolIndex(u16);

impl TableTypeEntryPoolIndex {
    pub(crate) const fn new(idx: u16) -> Self {
        Self(idx)
    }
}

#[derive(Debug)]
pub(crate) struct TableTypeEntryPool {
    entries: BTreeMap<u16, Arc<TableTypeEntry>>,
}

impl TableTypeEntryPool {
    pub(crate) fn new(entries: BTreeMap<u16, Arc<TableTypeEntry>>) -> Self {
        Self { entries }
    }

    pub(crate) const fn entries(&self) -> &BTreeMap<u16, Arc<TableTypeEntry>> {
        &self.entries
    }

    pub(crate) fn resolve(&self, entry_id: u16) -> Option<Arc<TableTypeEntry>> {
        self.entries.get(&entry_id).map(Arc::clone)
    }

    fn get(&self, id: TableTypeEntryPoolIndex) -> ResourcesResult<Arc<TableTypeEntry>> {
        self.entries
            .get(&id.0)
            .ok_or_else(|| ResourcesError::ResNotFound("TableTypeEntry".to_string()))
            .map(Arc::clone)
    }
}

#[derive(Debug)]
pub(crate) enum TableTypeEntryContent {
    EntryValue(Value),
    EntryMap(TableMapEntry),
}

impl TableTypeEntryContent {
    fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
        package: &TablePackage,
        ttype: &TableType,
    ) -> ResourcesResult<()> {
        match self {
            Self::EntryValue(value) => {
                value.pretty_print_from_resources(f, resources)?;
                writeln!(f)?;
            }
            Self::EntryMap(map) => {
                map.pretty_print(f, resources, package, ttype)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TableTypeEntry {
    pub(crate) self_ref: TableTypeEntryPoolIndex,
    pub(crate) entry: TableEntry,
    pub(crate) content: TableTypeEntryContent,
}

impl TableTypeEntry {
    fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
        package: &TablePackage,
        ttype: &TableType,
        id: u16,
    ) -> ResourcesResult<()> {
        self.entry.pretty_print(f, package, ttype, id)?;
        write!(f, "      (")?;
        ttype.config.pretty_print(f)?;
        write!(f, ") ")?;
        self.content.pretty_print(f, resources, package, ttype)
    }
}

#[derive(Debug)]
pub(crate) struct TableEntry {
    pub(crate) flags: u16,
    pub(crate) key: u32,
}

impl TableEntry {
    pub const fn is_complex(&self) -> bool {
        (self.flags & 0x1) == 0x1
    }

    pub const fn is_public(&self) -> bool {
        (self.flags & 0x2) == 0x2
    }

    pub const fn is_weak(&self) -> bool {
        (self.flags & 0x4) == 0x4
    }

    fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        package: &TablePackage,
        ttype: &TableType,
        id: u16,
    ) -> ResourcesResult<()> {
        let opt_key = package
            .key_strings
            .as_ref()
            .and_then(|ks| ks.strings.get(self.key as usize));
        let opt_typ = package
            .type_strings
            .as_ref()
            .and_then(|ts| ts.strings.get((ttype.id - 1) as usize));

        write!(
            f,
            "    resource 0x{:0>2x}{:0>2x}{:0>4x} ",
            package.id, ttype.id, id
        )?;

        if let Some(typ) = opt_typ {
            write!(f, "{typ}")?;
        } else {
            write!(f, "?????")?;
        }

        if let Some(key) = opt_key {
            writeln!(f, "/{key}")?;
        } else {
            writeln!(f, "/(name removed)")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TableMapEntry {
    pub(crate) parent: u32,
    pub(crate) table_maps: Vec<TableMap>,
}

impl TableMapEntry {
    pub fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
        package: &TablePackage,
        ttype: &TableType,
    ) -> ResourcesResult<()> {
        let opt_typ = package
            .type_strings
            .as_ref()
            .and_then(|ts| ts.strings.get((ttype.id - 1) as usize));

        match opt_typ {
            None => {
                writeln!(f, "(?????) size={}", self.table_maps.len())?;
                for map in &self.table_maps {
                    write!(f, "        ")?;
                    map.pretty_print(f, resources, true)?;
                }
            }
            Some(typ) => match typ.to_string().as_str() {
                "array" => {
                    writeln!(f, "({}) size={}", typ, self.table_maps.len())?;
                    write!(f, "        [")?;
                    for map in &self.table_maps {
                        write!(f, " ")?;
                        map.pretty_print(f, resources, false)?;
                    }
                    writeln!(f, " ]")?;
                }
                "attr" => {
                    write!(f, "({typ}) ")?;
                    self.table_maps[0].pretty_print(f, resources, true)?;
                    if 1 < self.table_maps.len() {
                        writeln!(f, " size={}", self.table_maps.len())?;
                    }
                    for map in &self.table_maps[1..] {
                        write!(f, "        ")?;
                        map.pretty_print(f, resources, true)?;
                        writeln!(f)?;
                    }
                    if 1 == self.table_maps.len() {
                        writeln!(f)?;
                    }
                }
                typ => {
                    writeln!(f, "({}) size={}", typ, self.table_maps.len())?;
                    for map in &self.table_maps {
                        write!(f, "        ")?;
                        map.pretty_print(f, resources, true)?;
                        writeln!(f)?;
                    }
                }
            },
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TableMap {
    pub(crate) name: u32,
    pub(crate) value: Value,
}

impl TableMap {
    pub fn pretty_print(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
        with_name: bool,
    ) -> ResourcesResult<()> {
        if with_name {
            write!(f, "????? name={:#x} ???? ", self.name)?;
        }
        self.value.pretty_print_from_resources(f, resources)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Config {
    pub(crate) imsi_mcc: Option<u16>,
    pub(crate) imsi_mnc: Option<u16>,
    pub(crate) locale_language: Option<Vec<u8>>,
    pub(crate) locale_country: Option<Vec<u8>>,
    pub(crate) screen_type_orientation: Option<u8>,
    pub(crate) screen_type_touchscreen: Option<u8>,
    pub(crate) screen_type_density: Option<u16>,
    pub(crate) input_keyboard: Option<u8>,
    pub(crate) input_navigation: Option<u8>,
    pub(crate) input_flags: Option<u8>,
    pub(crate) input_pad0: Option<u8>,
    pub(crate) screen_size_width: Option<u16>,
    pub(crate) screen_size_height: Option<u16>,
    pub(crate) version_sdk: Option<u16>,
    pub(crate) version_minor: Option<u16>,

    pub(crate) screen_config_layout: Option<u8>,
    pub(crate) screen_config_ui_mode: Option<u8>,
    pub(crate) screen_config_smallest_width_dp: Option<u16>,

    pub(crate) screen_size_dp_width: Option<u16>,
    pub(crate) screen_size_dp_height: Option<u16>,

    pub(crate) locale_script: Option<Vec<u8>>,
    pub(crate) locale_variant: Option<Vec<u8>>,

    pub(crate) screen_config_2_layout: Option<u8>,
    pub(crate) screen_config_color_mode: Option<u8>,
    pub(crate) screen_config_2_pad2: Option<u16>,
}

impl Config {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    fn locale_language(&self) -> Option<String> {
        self.locale_language.as_ref().map(|l| {
            let mut s = String::with_capacity(2);
            s.push(l[0] as char);
            s.push(l[1] as char);
            s
        })
    }

    fn locale_country(&self) -> Option<String> {
        self.locale_country.as_ref().map(|c| {
            let mut s = String::with_capacity(2);
            s.push(c[0] as char);
            s.push(c[1] as char);
            s
        })
    }

    fn importance_score_of_locale(&self) -> i8 {
        let v0 = self.locale_variant.as_ref().map_or(0, |l| l[0]);
        let s0 = self.locale_script.as_ref().map_or(0, |l| l[0]);

        //TODO: complete here with !localeScriptWasComputed and
        // see getImportanceScoreOfLocale in ResourcesTypes.cpp:
        // return (localeVariant[0] ? 4 : 0)
        // + (localeScript[0] && !localeScriptWasComputed ? 2: 0)
        // + (localeNumberingSystem[0] ? 1: 0);

        (if v0 == 0 { 0 } else { 4 }) + (if s0 == 0 { 0 } else { 2 })
    }

    pub fn is_more_specific_than(&self, other: &Self) -> bool {
        if (self.imsi_mcc.is_some() || other.imsi_mcc.is_some()) && self.imsi_mcc != other.imsi_mcc
        {
            if self.imsi_mcc.is_none() {
                return false;
            }
            if other.imsi_mcc.is_none() {
                return true;
            }
        }

        if (self.imsi_mnc.is_some() || other.imsi_mnc.is_some()) && self.imsi_mnc != other.imsi_mnc
        {
            if self.imsi_mnc.is_none() {
                return false;
            }
            if other.imsi_mnc.is_none() {
                return true;
            }
        }

        if self.locale_language.is_some() || other.locale_language.is_some() {
            let s0 = self.locale_language.as_ref().map_or(0, |l| l[0]);
            let o0 = other.locale_language.as_ref().map_or(0, |l| l[0]);
            if s0 != o0 {
                if s0 == 0 {
                    return false;
                }
                if o0 == 0 {
                    return true;
                }
            }
        }

        if self.locale_country.is_some() || other.locale_country.is_some() {
            let s0 = self.locale_country.as_ref().map_or(0, |l| l[0]);
            let o0 = other.locale_country.as_ref().map_or(0, |l| l[0]);
            if s0 != o0 {
                if s0 == 0 {
                    return false;
                }
                if o0 == 0 {
                    return true;
                }
            }
        }

        let diff = self.importance_score_of_locale() - other.importance_score_of_locale();

        match diff.cmp(&0) {
            Ordering::Less => return false,
            Ordering::Greater => return true,
            Ordering::Equal => (),
        }

        if self.screen_config_layout.is_some() || other.screen_config_layout.is_some() {
            const MASK_LAYOUTDIR: u8 = 0xC0;
            let s = self.screen_config_layout.as_ref().unwrap_or(&0) & MASK_LAYOUTDIR;
            let o = other.screen_config_layout.as_ref().unwrap_or(&0) & MASK_LAYOUTDIR;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if (self.screen_config_smallest_width_dp.is_some()
            || other.screen_config_smallest_width_dp.is_some())
            && self.screen_config_smallest_width_dp != other.screen_config_smallest_width_dp
        {
            if self.screen_config_smallest_width_dp.is_none() {
                return false;
            }
            if other.screen_config_smallest_width_dp.is_none() {
                return true;
            }
        }

        if (self.screen_size_dp_width.is_some() || other.screen_size_dp_width.is_some())
            && self.screen_size_dp_width != other.screen_size_dp_width
        {
            if self.screen_size_dp_width.is_none() {
                return false;
            }
            if other.screen_size_dp_width.is_none() {
                return true;
            }
        }

        if (self.screen_size_dp_height.is_some() || other.screen_size_dp_height.is_some())
            && self.screen_size_dp_height != other.screen_size_dp_height
        {
            if self.screen_size_dp_height.is_none() {
                return false;
            }
            if other.screen_size_dp_height.is_none() {
                return true;
            }
        }

        if self.screen_config_layout.is_some() || other.screen_config_layout.is_some() {
            const MASK_SCREENSIZE: u8 = 0x0F;
            let s = self.screen_config_layout.as_ref().unwrap_or(&0) & MASK_SCREENSIZE;
            let o = other.screen_config_layout.as_ref().unwrap_or(&0) & MASK_SCREENSIZE;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }

            const MASK_SCREENLONG: u8 = 0x30;
            let s = self.screen_config_layout.as_ref().unwrap_or(&0) & MASK_SCREENLONG;
            let o = other.screen_config_layout.as_ref().unwrap_or(&0) & MASK_SCREENLONG;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if self.screen_config_2_layout.is_some() || other.screen_config_2_layout.is_some() {
            const MASK_SCREENROUND: u8 = 0x03;
            let s = self.screen_config_2_layout.as_ref().unwrap_or(&0) & MASK_SCREENROUND;
            let o = other.screen_config_2_layout.as_ref().unwrap_or(&0) & MASK_SCREENROUND;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if self.screen_config_color_mode.is_some() || other.screen_config_color_mode.is_some() {
            const MASK_HDR: u8 = 0x0C;
            let s = self.screen_config_color_mode.as_ref().unwrap_or(&0) & MASK_HDR;
            let o = other.screen_config_color_mode.as_ref().unwrap_or(&0) & MASK_HDR;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }

            const MASK_WIDE_COLOR_GAMUT: u8 = 0x03;
            let s = self.screen_config_color_mode.as_ref().unwrap_or(&0) & MASK_WIDE_COLOR_GAMUT;
            let o = other.screen_config_color_mode.as_ref().unwrap_or(&0) & MASK_WIDE_COLOR_GAMUT;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if self.screen_type_orientation != other.screen_type_orientation {
            if self.screen_type_orientation.is_none() {
                return false;
            }
            if other.screen_type_orientation.is_none() {
                return true;
            }
        }

        if self.screen_config_ui_mode.is_some() || other.screen_config_ui_mode.is_some() {
            const MASK_UI_MODE_TYPE: u8 = 0x0F;
            let s = self.screen_config_ui_mode.as_ref().unwrap_or(&0) & MASK_UI_MODE_TYPE;
            let o = other.screen_config_ui_mode.as_ref().unwrap_or(&0) & MASK_UI_MODE_TYPE;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }

            const MASK_UI_MODE_NIGHT: u8 = 0x30;
            let s = self.screen_config_ui_mode.as_ref().unwrap_or(&0) & MASK_UI_MODE_NIGHT;
            let o = other.screen_config_ui_mode.as_ref().unwrap_or(&0) & MASK_UI_MODE_NIGHT;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if self.screen_type_touchscreen != other.screen_type_touchscreen {
            if self.screen_type_touchscreen.is_none() {
                return false;
            }
            if other.screen_type_touchscreen.is_none() {
                return true;
            }
        }

        if self.input_flags.is_some() || other.input_flags.is_some() {
            const MASK_KEYSHIDDEN: u8 = 0x03;
            let s = self.input_flags.as_ref().unwrap_or(&0) & MASK_KEYSHIDDEN;
            let o = other.input_flags.as_ref().unwrap_or(&0) & MASK_KEYSHIDDEN;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }

            const MASK_NAVHIDDEN: u8 = 0x0C;
            let s = self.input_flags.as_ref().unwrap_or(&0) & MASK_NAVHIDDEN;
            let o = other.input_flags.as_ref().unwrap_or(&0) & MASK_NAVHIDDEN;
            if (s ^ o) != 0 {
                if s == 0 {
                    return false;
                }
                if o == 0 {
                    return true;
                }
            }
        }

        if self.input_keyboard != other.input_keyboard {
            if self.input_keyboard.is_none() {
                return false;
            }
            if other.input_keyboard.is_none() {
                return true;
            }
        }

        if self.input_navigation != other.input_navigation {
            if self.input_navigation.is_none() {
                return false;
            }
            if other.input_navigation.is_none() {
                return true;
            }
        }

        if self.screen_size_width != other.screen_size_width {
            if self.screen_size_width.is_none() {
                return false;
            }
            if other.screen_size_width.is_none() {
                return true;
            }
        }

        if self.screen_size_height != other.screen_size_height {
            if self.screen_size_height.is_none() {
                return false;
            }
            if other.screen_size_height.is_none() {
                return true;
            }
        }

        if self.version_sdk != other.version_sdk {
            if self.version_sdk.is_none() {
                return false;
            }
            if other.version_sdk.is_none() {
                return true;
            }
        }

        if self.version_minor != other.version_minor {
            if self.version_minor.is_none() {
                return false;
            }
            if other.version_minor.is_none() {
                return true;
            }
        }

        false
    }

    pub fn pretty_print(&self, f: &mut fmt::Formatter) -> ResourcesResult<()> {
        if let Some(mcc) = self.imsi_mcc {
            write!(f, " imsi_mcc={mcc}")?;
        }
        if let Some(mnc) = self.imsi_mnc {
            write!(f, " imsi_mnc={mnc}")?;
        }
        if let Some(language) = self.locale_language() {
            write!(f, " language={language}")?;
        }
        if let Some(country) = self.locale_country() {
            write!(f, " country={country}")?;
        }
        if let Some(orientation) = self.screen_type_orientation {
            write!(f, " screen_type_orientation={orientation}",)?;
        }
        if let Some(touchscreen) = self.screen_type_touchscreen {
            write!(f, " screen_type_touchscreen={touchscreen}",)?;
        }
        if let Some(density) = self.screen_type_density {
            write!(f, " screen_type_density={density}",)?;
        }
        if let Some(keyboard) = self.input_keyboard {
            write!(f, " input_keyboard={keyboard}")?;
        }
        if let Some(navigation) = self.input_navigation {
            write!(f, " input_navigation={navigation}")?;
        }
        if let Some(flags) = self.input_flags {
            write!(f, " input_flags={flags}")?;
        }
        if let Some(pad0) = self.input_pad0 {
            write!(f, " input_pad0={pad0}")?;
        }
        if let Some(width) = self.screen_size_width {
            write!(f, " screen_size_width={width}")?;
        }
        if let Some(height) = self.screen_size_height {
            write!(f, " screen_size_height={height}",)?;
        }
        if let Some(sdk) = self.version_sdk {
            write!(f, " version_sdk={sdk}")?;
        }
        if let Some(minor) = self.version_minor {
            write!(f, " version_minor={minor}")?;
        }
        if let Some(layout) = self.screen_config_layout {
            write!(f, " screen_config_layout={layout}",)?;
        }
        if let Some(mode) = self.screen_config_ui_mode {
            write!(f, " screen_config_ui_mode={mode}",)?;
        }
        if let Some(dp) = self.screen_config_smallest_width_dp {
            write!(f, " screen_config_smallest_width_dp={dp}",)?;
        }
        if let Some(dp_width) = self.screen_size_dp_width {
            write!(f, " screen_size_dp_width={dp_width}",)?;
        }
        if let Some(dp_height) = self.screen_size_dp_height {
            write!(f, " screen_size_dp_height={dp_height}",)?;
        }
        if let Some(script) = &self.locale_script {
            write!(f, " locale_script={script:#?}",)?;
        }
        if let Some(variant) = &self.locale_variant {
            write!(f, " locale_variant={variant:#?}",)?;
        }
        if let Some(layout) = self.screen_config_2_layout {
            write!(f, " screen_config_2_layout={layout}",)?;
        }
        if let Some(color_mode) = self.screen_config_color_mode {
            write!(f, " screen_config_color_mode={color_mode}",)?;
        }
        if let Some(pad2) = self.screen_config_2_pad2 {
            write!(f, " screen_config_2_pad2={pad2}",)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TableLibrary {
    pub(crate) libraries: Vec<TableLibraryEntry>,
}

#[derive(Debug)]
pub(crate) struct TableLibraryEntry {
    pub(crate) id: u32,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct TableOverlayable {
    pub(crate) name: String,
    pub(crate) actor: String,
}

#[derive(Debug)]
pub(crate) struct TableOverlayablePolicy {
    pub(crate) flags: u32,
    pub(crate) entries: Vec<u32>,
}

impl TableOverlayablePolicy {
    pub const fn is_public(&self) -> bool {
        (self.flags & 0x0000_0001) == 0x0000_0001
    }

    pub const fn is_system_partition(&self) -> bool {
        (self.flags & 0x0000_0002) == 0x0000_0002
    }

    pub const fn is_vendor_partition(&self) -> bool {
        (self.flags & 0x0000_0004) == 0x0000_0004
    }

    pub const fn is_product_partition(&self) -> bool {
        (self.flags & 0x0000_0008) == 0x0000_0008
    }

    pub const fn is_signature(&self) -> bool {
        (self.flags & 0x0000_0010) == 0x0000_0010
    }

    pub const fn is_odm_partition(&self) -> bool {
        (self.flags & 0x0000_0020) == 0x0000_0020
    }

    pub const fn is_oem_partition(&self) -> bool {
        (self.flags & 0x0000_0040) == 0x0000_0040
    }

    pub const fn is_actor_signature(&self) -> bool {
        (self.flags & 0x0000_0080) == 0x0000_0080
    }

    pub const fn is_config_signature(&self) -> bool {
        (self.flags & 0x0000_0100) == 0x0000_0100
    }
}

#[derive(Debug)]
pub(crate) struct TableStagedAlias {
    pub(crate) entries: Vec<TableStagedAliasEntry>,
}

#[derive(Debug)]
pub(crate) struct TableStagedAliasEntry {
    pub(crate) stage_id: u32,
    pub(crate) finalized_id: u32,
}
