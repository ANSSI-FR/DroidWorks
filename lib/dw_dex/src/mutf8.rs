use crate::errors::{DexError, DexResult};

// Decodes a non-null-terminated MUTF-8 buffer and returns its String form.
// Reimplementation of https://android.googlesource.com/platform/libcore/+/7047230/dex/src/main/java/com/android/dex/Mutf8.java
// Since surrogate pairs characters are not valid Rust chars,
// it builds a vector of u16 (i.e. an UTF16 encoded buffer).
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_lossless)]
pub(crate) fn decode(inp: &[u8]) -> DexResult<Vec<u16>> {
    let mut i = 0;
    let mut buf: Vec<u16> = Vec::new();

    while i < inp.len() {
        let a = inp[i];
        i += 1;

        if a == 0 && i != inp.len() - 1 {
            return Err(DexError::InvalidMutf8(
                "null-byte in a non-null terminated string".to_string(),
            ));
        }

        if a < 0x80 {
            buf.push(a as u16);
        } else if (a & 0xe0) == 0xc0 {
            if i >= inp.len() {
                return Err(DexError::InvalidMutf8(
                    "not enough data to read 2-points char".to_string(),
                ));
            }
            let b = inp[i];
            i += 1;
            if (b & 0xc0) != 0x80 {
                return Err(DexError::InvalidMutf8("bad second byte".to_string()));
            }
            let mut ch: u16 = ((a as u16) & 0x1f) << 6;
            ch |= (b as u16) & 0x3f;
            buf.push(ch);
        } else if (a & 0xf0) == 0xe0 {
            if i >= inp.len() - 1 {
                return Err(DexError::InvalidMutf8(
                    "not enough data to read 3-points char".to_string(),
                ));
            }
            let b = inp[i];
            let c = inp[i + 1];
            i += 2;
            if ((b & 0xc0) != 0x80) || ((c & 0xc0) != 0x80) {
                return Err(DexError::InvalidMutf8(
                    "bad second or third byte".to_string(),
                ));
            }
            let mut ch: u16 = ((a as u16) & 0x0f) << 12;
            ch |= ((b as u16) & 0x3f) << 6;
            ch |= (c as u16) & 0x3f;
            buf.push(ch);
        } else {
            return Err(DexError::InvalidMutf8("bad byte".to_string()));
        }
    }

    Ok(buf)
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn _encode(s: &str) -> Vec<u8> {
    let mut buf = Vec::new();

    for ch in s.chars() {
        let c = ch as u16;
        if c != 0 && c <= 127 {
            // U+0000 uses two bytes
            buf.push(c as u8);
        } else if c <= 2047 {
            buf.push((0xc0 | (0x1f & (c >> 6))) as u8);
            buf.push((0x80 | (0x3f & c)) as u8);
        } else {
            buf.push((0xe0 | (0x0f & (c >> 12))) as u8);
            buf.push((0x80 | (0x3f & (c >> 6))) as u8);
            buf.push((0x80 | (0x3f & c)) as u8);
        }
    }

    buf
}
