//! Small writers functions to be used in file format writers.

use crate::leb::{Sleb128, Uleb128};
use std::io::{Result, Seek, Write};

/// Writes zeros to align offset on next 4 bytes.
pub fn align4<W: Write + Seek>(output: &mut W) -> Result<usize> {
    let mut sz = 0;
    for _ in 0..(4 - (output.stream_position().unwrap() % 4)) % 4 {
        sz += le_u8(output, 0x00)?;
    }
    Ok(sz)
}

/// Writes a bytes buffer in given output.
pub fn bytes<W: Write>(output: &mut W, bytes: &[u8]) -> Result<usize> {
    output.write(bytes)
}

/// Writes a string slice in given output.
pub fn tag<W: Write>(output: &mut W, tag: &str) -> Result<usize> {
    output.write(tag.as_bytes())
}

/// Writes a i8 in given output.
pub fn le_i8<W: Write>(output: &mut W, v: i8) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a u8 in given output.
pub fn le_u8<W: Write>(output: &mut W, v: u8) -> Result<usize> {
    output.write(&[v])
}

/// Writes a i16 in given output.
pub fn le_i16<W: Write>(output: &mut W, v: i16) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a u16 in given output.
pub fn le_u16<W: Write>(output: &mut W, v: u16) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a i32 in given output.
pub fn le_i32<W: Write>(output: &mut W, v: i32) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a u32 in given output.
pub fn le_u32<W: Write>(output: &mut W, v: u32) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a i64 in given output.
pub fn le_i64<W: Write>(output: &mut W, v: i64) -> Result<usize> {
    output.write(&v.to_le_bytes())
}

/// Writes a f32 in given output.
pub fn le_f32<W: Write>(output: &mut W, f: f32) -> Result<usize> {
    output.write(&f.to_le_bytes())
}

/// Writes a f64 in given output.
pub fn le_f64<W: Write>(output: &mut W, f: f64) -> Result<usize> {
    output.write(&f.to_le_bytes())
}

pub fn le_i16_on<W: Write>(output: &mut W, v: i16, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}
pub fn le_u16_on<W: Write>(output: &mut W, v: u16, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_i32_on<W: Write>(output: &mut W, v: i32, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_u32_on<W: Write>(output: &mut W, v: u32, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_i64_on<W: Write>(output: &mut W, v: i64, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_u64_on<W: Write>(output: &mut W, v: u64, s: usize) -> Result<usize> {
    let vbytes = v.to_le_bytes();
    let sz = output.write(&vbytes[0..s])?;
    for b in vbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_f32_on<W: Write>(output: &mut W, f: f32, s: usize) -> Result<usize> {
    let fbytes = f.to_le_bytes();
    let sz = output.write(&fbytes[0..s])?;
    for b in fbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

pub fn le_f64_on<W: Write>(output: &mut W, f: f64, s: usize) -> Result<usize> {
    let fbytes = f.to_le_bytes();
    let sz = output.write(&fbytes[0..s])?;
    for b in fbytes.iter().skip(s) {
        assert_eq!(*b, 0);
    }
    Ok(sz)
}

/// Writes a u32 in given output, in uleb128 format.
#[allow(clippy::cast_possible_truncation)]
pub fn uleb128<W: Write>(output: &mut W, val: Uleb128) -> Result<usize> {
    let mut v = val.value();
    let mut size = 0;
    loop {
        let lo7 = (v & 0b111_1111) as u8;
        v >>= 7;
        if v == 0 {
            if val.size() - 1 > size {
                size += le_u8(output, lo7 | 0b1000_0000)?;
            } else {
                size += le_u8(output, lo7)?;
            }
            break;
        }
        size += le_u8(output, lo7 | 0b1000_0000)?;
    }
    assert!(val.size() >= size);
    for i in 0..(val.size() - size) {
        if i == val.size() - size - 1 {
            le_u8(output, 0)?;
        } else {
            le_u8(output, 0b1000_0000)?;
        }
    }
    Ok(val.size())
}

/// Writes a u32 in given output, in uleb128p1 format.
pub fn uleb128p1<W: Write>(output: &mut W, v: Option<u32>) -> Result<usize> {
    match v {
        None => output.write(&[0x00]),
        Some(v) => uleb128(output, Uleb128::new(v + 1, None)),
    }
}

/// Write a i32 in given output, in sleb128 format.
#[allow(clippy::cast_sign_loss)]
pub fn sleb128<W: Write>(output: &mut W, v: Sleb128) -> Result<usize> {
    let v = v.value();
    let w = if v >= 0 {
        v as u32
    } else {
        let mut w = v as u32;
        if w & 0xf800_0000 == 0xf800_0000 {
            w &= 0x0fff_ffff;

            for i in &[21, 14, 7] {
                if w & (0b1111_1111 << (i - 1)) == (0b1111_1111 << (i - 1)) {
                    w &= !(0b111_1111 << i);
                } else {
                    break;
                }
            }
        }
        w
    };

    uleb128(output, Uleb128::new(w, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_writer() {
        let mut buf = Vec::new();
        assert!(tag(&mut buf, "hello").is_ok());
        assert_eq!(buf, vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn le_i8_writer() {
        let mut buf = Vec::new();
        assert!(le_i8(&mut buf, -40i8).is_ok());
        assert_eq!(buf, vec![216]);
    }

    #[test]
    fn le_u8_writer() {
        let mut buf = Vec::new();
        assert!(le_u8(&mut buf, 40u8).is_ok());
        assert_eq!(buf, vec![40]);
    }

    #[test]
    fn le_i16_writer() {
        let mut buf = Vec::new();
        assert!(le_i16(&mut buf, -200i16).is_ok());
        assert_eq!(buf, vec![0x38, 0xff]);
    }

    #[test]
    fn le_u16_writer() {
        let mut buf = Vec::new();
        assert!(le_u16(&mut buf, 0x88b8u16).is_ok());
        assert_eq!(buf, vec![0xb8, 0x88]);
    }

    #[test]
    fn le_i32_writer() {
        let mut buf = Vec::new();
        assert!(le_i32(&mut buf, 0x12345678i32).is_ok());
        assert_eq!(buf, vec![0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn le_u32_writer() {
        let mut buf = Vec::new();
        assert!(le_u32(&mut buf, 0x12345678u32).is_ok());
        assert_eq!(buf, vec![0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn le_i64_writer() {
        let mut buf = Vec::new();
        assert!(le_i64(&mut buf, 0x1122334455667788i64).is_ok());
        assert_eq!(buf, vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn uleb128_writer() {
        let mut buf = Vec::new();
        assert!(uleb128(&mut buf, Uleb128::new(0, None)).is_ok());
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        assert!(uleb128(&mut buf, Uleb128::new(1, None)).is_ok());
        assert_eq!(buf, vec![0x01]);

        buf.clear();
        assert!(uleb128(&mut buf, Uleb128::new(127, None)).is_ok());
        assert_eq!(buf, vec![0x7f]);

        buf.clear();
        assert!(uleb128(&mut buf, Uleb128::new(16256, None)).is_ok());
        assert_eq!(buf, vec![0x80, 0x7f]);
    }

    #[test]
    fn uleb128p1_writer() {
        let mut buf = Vec::new();
        assert!(uleb128p1(&mut buf, None).is_ok());
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        assert!(uleb128p1(&mut buf, Some(0)).is_ok());
        assert_eq!(buf, vec![0x01]);

        buf.clear();
        assert!(uleb128p1(&mut buf, Some(126)).is_ok());
        assert_eq!(buf, vec![0x7f]);

        buf.clear();
        assert!(uleb128p1(&mut buf, Some(16255)).is_ok());
        assert_eq!(buf, vec![0x80, 0x7f]);
    }

    #[test]
    fn sleb128_writer() {
        let mut buf = Vec::new();
        assert!(sleb128(&mut buf, Sleb128::new(0, None)).is_ok());
        assert_eq!(buf, vec![0x00]);

        buf.clear();
        assert!(sleb128(&mut buf, Sleb128::new(1, None)).is_ok());
        assert_eq!(buf, vec![0x01]);

        buf.clear();
        assert!(sleb128(&mut buf, Sleb128::new(-1, None)).is_ok());
        assert_eq!(buf, vec![0x7f]);

        buf.clear();
        assert!(sleb128(&mut buf, Sleb128::new(-128, None)).is_ok());
        assert_eq!(buf, vec![0x80, 0x7f]);
    }
}
