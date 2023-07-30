//! Types definitions to address Dalvik registers.
//!
//! In Dalvik bytecode, registers (or register pairs) are addressed either on 8 or 16 bits.
//! To ease the bytecode manipulation, we define a [register](Reg) wrapper over an 16 bits integer.
//! Also, this allows to differentiate registers from constant values in the definition of bytecode
//! formats, thus allowing deriving several functions (see [`instruction_derive`] crate for details).
//!
//! Finally, registers groups (lists or ranges) are defined in this module.

use serde::Serialize;
use std::{fmt, io};

/// The register type.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct Reg(u16);

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl From<u8> for Reg {
    fn from(r: u8) -> Self {
        Self(u16::from(r))
    }
}

impl From<u16> for Reg {
    fn from(r: u16) -> Self {
        Self(r)
    }
}

impl TryFrom<Reg> for u8 {
    type Error = io::Error;

    fn try_from(r: Reg) -> Result<Self, Self::Error> {
        if r.0 <= u16::from(Self::MAX) {
            Ok(r.0 as Self)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "register cannot fit into u8",
            ))
        }
    }
}

impl From<Reg> for u16 {
    fn from(r: Reg) -> Self {
        r.0
    }
}

impl Reg {
    /// Returns the wrapped register slot number.
    #[inline]
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Returns the following register.
    ///
    /// This function is used to address register pairs without manipulating slot
    /// numbers directly.
    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// An explicit list of registers, used for methods parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct RegList(Vec<Reg>);

impl fmt::Display for RegList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, " {{")?;
        for i in 0..self.0.len() {
            write!(f, "{}", self.0[i])?;
            if i < self.0.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "}}")
    }
}

impl<T> From<Vec<T>> for RegList
where
    Reg: From<T>,
{
    fn from(args: Vec<T>) -> Self {
        Self(args.into_iter().map(Reg::from).collect())
    }
}

impl<T> TryFrom<RegList> for Vec<T>
where
    T: TryFrom<Reg>,
{
    type Error = <T as TryFrom<Reg>>::Error;

    fn try_from(args: RegList) -> Result<Self, Self::Error> {
        args.0.into_iter().map(Reg::try_into).collect()
    }
}

impl RegList {
    /// Checks if the list contains no register.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a new iterator over the registers list.
    #[must_use]
    pub const fn iter(&self) -> RegListIterator {
        RegListIterator {
            list: self,
            current: 0,
        }
    }
}

/// An [`Iterator`] over registers from a list.
pub struct RegListIterator<'a> {
    list: &'a RegList,
    current: usize,
}

impl<'a> Iterator for RegListIterator<'a> {
    type Item = Reg;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.list.0.len() {
            None
        } else {
            let res = Some(self.list.0[self.current]);
            self.current += 1;
            res
        }
    }
}

/// A range of register addresses, used to pass consecutive register slots as method parameters.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RegRange {
    begin: Reg,
    end: Reg,
}

impl fmt::Display for RegRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{} .. {}}}", self.begin, self.end)
    }
}

impl<T> From<(T, T)> for RegRange
where
    Reg: From<T>,
    T: PartialOrd,
{
    fn from(bounds: (T, T)) -> Self {
        assert!(bounds.0 <= bounds.1, "invalid registers range");
        Self {
            begin: Reg::from(bounds.0),
            end: Reg::from(bounds.1),
        }
    }
}

impl RegRange {
    /// Returns the first register of the range.
    #[inline]
    #[must_use]
    pub const fn begin(&self) -> &Reg {
        &self.begin
    }

    /// Returns the last register of the range.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> &Reg {
        &self.end
    }

    /// Returns a new iterator over the register range.
    #[must_use]
    pub const fn iter(&self) -> RegRangeIterator {
        RegRangeIterator {
            range: self,
            current: *self.begin(),
        }
    }
}

/// An [`Iterator`] over registers from a range.
pub struct RegRangeIterator<'a> {
    range: &'a RegRange,
    current: Reg,
}

impl<'a> Iterator for RegRangeIterator<'a> {
    type Item = Reg;

    fn next(&mut self) -> Option<Reg> {
        if self.current.0 > self.range.end().0 {
            None
        } else {
            let res = self.current;
            self.current = self.current.next();
            Some(res)
        }
    }
}
