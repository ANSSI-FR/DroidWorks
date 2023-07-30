#[derive(Debug, Clone, Copy)]
pub struct Uleb128 {
    value: u32,
    size: usize,
}

impl Uleb128 {
    pub fn new(value: u32, size_hint: Option<usize>) -> Self {
        let needed_size = if value == 0 {
            1
        } else {
            let nbits = 32 - value.leading_zeros() as usize;
            1 + ((nbits - 1) / 7)
        };
        let size = if let Some(hint) = size_hint {
            std::cmp::max(needed_size, hint)
        } else {
            needed_size
        };
        Self { value, size }
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn shift(&mut self, offset: usize) {
        let new_value = self.value + offset as u32;
        let needed = ((32 - (new_value.leading_zeros() as usize)) / 7) + 1;
        if needed > self.size() {
            panic!(
                "not enough space to grow uleb128 value from {} ({} x 7-bits) to {} ({} x 7-bits)",
                self.value(),
                self.size(),
                new_value,
                needed
            );
        }
        self.value = new_value;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sleb128 {
    value: i32,
    size: usize,
}

impl Sleb128 {
    pub fn new(value: i32, size_hint: Option<usize>) -> Self {
        let urepr = if value >= 0 {
            value as u32
        } else {
            let mut w = value as u32;
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
        let needed_size = if urepr == 0 {
            1
        } else {
            let nbits = 32 - urepr.leading_zeros() as usize;
            1 + ((nbits - 1) / 7)
        };
        let size = if let Some(hint) = size_hint {
            std::cmp::max(needed_size, hint)
        } else {
            needed_size
        };
        Self { value, size }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn size(&self) -> usize {
        self.size
    }
}
