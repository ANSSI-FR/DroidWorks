use crate::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub enum OwnDex {
    Dex(Dex),
    Package(Package),
}

impl OwnDex {
    pub fn open<P: AsRef<Path>>(path: P) -> DwResult<Self> {
        if let Some(ext) = path.as_ref().extension() {
            match ext.to_str() {
                Some("dex") => {
                    let dex = dw_dex::open(path)?;
                    return Ok(Self::Dex(dex));
                }
                Some("apk" | "aab") => {
                    let package = dw_package::Options::default()
                        .dont_parse_resources()
                        .open(path)?;
                    return Ok(Self::Package(package));
                }
                _ => return Err(DwError::BadArguments("unknown file extension".to_string())),
            }
        }
        Err(DwError::BadArguments("unknown file extension".to_string()))
    }

    pub fn save<P: AsRef<Path>>(self, path: P) -> DwResult<()> {
        match self {
            Self::Dex(dex) => {
                let buf = dw_dex::write(&dex, true).map_err(DwError::from)?;
                let mut file = File::create("foo.txt")?;
                file.write_all(&buf)?;
                Ok(())
            }
            Self::Package(package) => package.save(path, true).map_err(DwError::from),
        }
    }

    #[must_use]
    pub fn borrow_dexs(&self) -> Vec<&Dex> {
        match self {
            Self::Dex(dex) => vec![dex],
            Self::Package(package) => package.iter_dexs().collect(),
        }
    }

    #[must_use]
    pub fn package(&self) -> Option<&Package> {
        match self {
            Self::Dex(_) => None,
            Self::Package(package) => Some(package),
        }
    }

    pub fn modify_dexs(&mut self) {
        if let Self::Package(package) = self {
            package.modify_dexs()
        }
    }
}
