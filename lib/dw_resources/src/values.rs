use crate::errors::{ResourcesError, ResourcesResult};
use crate::resources::Resources;
use crate::strings::{StringPool, StringPoolIndex};
use crate::tables::TableTypeEntryContent;
use crate::Xml;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Null,
    Reference(u32),
    Attribute(u32),
    String(StringPoolIndex),
    Float(f32),
    Dimension(u32),
    Fraction(u32),
    IntDec(u32),
    IntHex(u32),
    IntBoolean(bool),
    IntColorARGB8(u32),
    IntColorRGB8(u32),
    IntColorARGB4(u32),
    IntColorRGB4(u32),
}

impl Value {
    /// When used in XmlAttribute, a value is accompanied by a raw value.
    /// This method implements the mapping between Value and raw value (u32 representation).
    pub(crate) fn raw_value(&self) -> u32 {
        match self {
            Self::Null => todo!("raw value for Null"),
            Self::Reference(_) => 0xffff_ffff,
            Self::Attribute(_) => todo!("raw value for Attribute"),
            Self::String(s_id) => s_id.index() as u32,
            Self::Float(_) => todo!("raw value for Float"),
            Self::Dimension(_) => todo!("raw value for Dimension"),
            Self::Fraction(_) => todo!("raw value for Fraction"),
            Self::IntDec(_) => 0xffff_ffff,
            Self::IntHex(_) => 0xffff_ffff,
            Self::IntBoolean(_) => 0xffff_ffff,
            Self::IntColorARGB8(_) => todo!("raw value for IntColorARGB8"),
            Self::IntColorRGB8(_) => todo!("raw value for IntColorRGB8"),
            Self::IntColorARGB4(_) => todo!("raw value for IntColorARGB4"),
            Self::IntColorRGB4(_) => todo!("raw value for IntColorRGB4"),
        }
    }

    pub fn pretty_print_from_xml(&self, f: &mut fmt::Formatter, xml: &Xml) -> ResourcesResult<()> {
        match self {
            Self::Null => write!(f, "\"null\"")?,
            Self::Reference(r) => write!(f, "\"@{r:#x}\"")?, // TODO
            Self::Attribute(_) => todo!("display Value::Attribute"),
            Self::String(s_id) => write!(f, "{:?}", xml.string_pool.get(*s_id)?.string()?)?,
            Self::Float(v) => write!(f, "\"{v}\"")?,
            Self::Dimension(_) => todo!("display Value::Dimension"),
            Self::Fraction(_) => todo!("display Value::Fraction"),
            Self::IntDec(i) => write!(f, "\"{i}\"")?,
            Self::IntHex(i) => write!(f, "\"{i:#x}\"")?,
            Self::IntBoolean(b) => write!(f, "\"{b}\"")?,
            Self::IntColorARGB8(_) => todo!("display Value::IntColorARGB8"),
            Self::IntColorRGB8(_) => todo!("display Value::IntColorRGB8"),
            Self::IntColorARGB4(_) => todo!("display Value::IntColorARGB4"),
            Self::IntColorRGB4(_) => todo!("display Value::IntColorRGB4"),
        }
        Ok(())
    }

    pub fn pretty_print_from_resources(
        &self,
        f: &mut fmt::Formatter,
        resources: &Resources,
    ) -> ResourcesResult<()> {
        match self {
            Self::Null => write!(f, "@null")?,
            Self::Reference(r) => write!(f, "@0x{r:0>8x}")?,
            Self::Attribute(_) => write!(f, "?attr?")?,
            Self::String(s_id) => {
                write!(f, "{:?}", resources.0.string_pool.get(*s_id)?.to_string())?;
            }
            Self::Float(_) => write!(f, "?float?")?,
            Self::Dimension(_) => write!(f, "?dimension?")?,
            Self::Fraction(_) => write!(f, "?fraction?")?,
            Self::IntDec(i) => write!(f, "{i}")?,
            Self::IntHex(i) => write!(f, "{i:#x}")?,
            Self::IntBoolean(b) => write!(f, "{b}")?,
            Self::IntColorARGB8(c)
            | Self::IntColorRGB8(c)
            | Self::IntColorARGB4(c)
            | Self::IntColorRGB4(c) => write!(f, "#{c:0>8x}")?,
        }
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        string_pool: &StringPool,
        oresources: Option<&Resources>,
    ) -> ResourcesResult<ResolvedValue> {
        match self {
            Self::Null => Ok(ResolvedValue::Null),
            Self::Reference(r) => {
                if oresources.is_none() {
                    return Err(ResourcesError::ResNotFound(format!(
                        "no resources provided to resolve ]{r:#x}",
                    )));
                }

                let resources = oresources.unwrap();

                let ocmap = resources.lookup(*r);
                if ocmap.is_none() {
                    return Err(ResourcesError::CannotResolveWithoutResources(format!(
                        "resource @{r:#x}",
                    )));
                }
                let cmap = ocmap.unwrap();
                if cmap.is_empty() {
                    return Err(ResourcesError::ResNotFound(format!("resource @{r:#x}")));
                }

                let mut prev = None;
                for (_, entry) in cmap {
                    let cur = match entry.content {
                        TableTypeEntryContent::EntryValue(v) => {
                            let string_pool = &resources.0.string_pool;
                            v.resolve(string_pool, oresources)?
                        }
                        TableTypeEntryContent::EntryMap(_) => {
                            return Err(ResourcesError::TooComplexResource(format!(
                                "resource @{r:#x} is an EntryMap",
                            )));
                        }
                    };

                    match prev {
                        None => prev = Some(cur),
                        Some(ref prev) => {
                            if *prev != cur {
                                return Err(ResourcesError::TooComplexResource(format!("resource @{r:#x} has several configurations with different values")));
                            }
                        }
                    }
                }
                match prev {
                    None => Err(ResourcesError::ResNotFound(format!("resource @{r:#x}"))),
                    Some(rv) => Ok(rv),
                }
            }
            Self::Attribute(a) => Ok(ResolvedValue::Attribute(*a)),
            Self::String(s) => Ok(ResolvedValue::String(string_pool.get(*s)?.string()?)),
            Self::Float(f) => Ok(ResolvedValue::Float(*f)),
            Self::Dimension(d) => Ok(ResolvedValue::Dimension(*d)),
            Self::Fraction(f) => Ok(ResolvedValue::Fraction(*f)),
            Self::IntDec(i) | Self::IntHex(i) => Ok(ResolvedValue::Int(*i)),
            Self::IntBoolean(b) => Ok(ResolvedValue::Bool(*b)),
            Self::IntColorARGB8(c) => {
                let b = c.to_le_bytes();
                Ok(ResolvedValue::Color(Color::Argb(b[0], b[1], b[2], b[3])))
            }
            Self::IntColorRGB8(c) => {
                let b = c.to_le_bytes();
                Ok(ResolvedValue::Color(Color::Rgb(b[1], b[2], b[3])))
            }
            Self::IntColorARGB4(c) => {
                let b = c.to_le_bytes();
                let b2 = b[2];
                let b3 = b[3];
                Ok(ResolvedValue::Color(Color::Argb(
                    (b2 & 0xf0) >> 4,
                    b2 & 0xf,
                    (b3 & 0xf0) >> 4,
                    b3 & 0xf,
                )))
            }
            Self::IntColorRGB4(c) => {
                let b = c.to_le_bytes();
                let b2 = b[2];
                let b3 = b[3];
                Ok(ResolvedValue::Color(Color::Rgb(
                    b2 & 0xf,
                    (b3 & 0xf0) >> 4,
                    b3 & 0xf,
                )))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Color {
    Argb(u8, u8, u8, u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResolvedValue {
    Null,
    Attribute(u32),
    String(String),
    Float(f32),
    Dimension(u32),
    Fraction(u32),
    Int(u32),
    Bool(bool),
    Color(Color),
}

impl ResolvedValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}
