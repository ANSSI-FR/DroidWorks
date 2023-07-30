//! Code address representation.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Addr(pub usize);

impl Addr {
    #[inline]
    #[must_use]
    pub const fn entry() -> Self {
        Self(0)
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Addr {
    pub const fn from_offset(base_addr: Self, offset: i32) -> Self {
        if offset.is_negative() {
            Self(base_addr.0 - offset.unsigned_abs() as usize)
        } else {
            Self(base_addr.0 + offset.unsigned_abs() as usize)
        }
    }

    pub const fn offset(self, offset: i32) -> Self {
        Self::from_offset(self, offset)
    }
}
