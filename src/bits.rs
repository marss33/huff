// escribe y lee bits de a uno, empezando por el bit mas alto de cada byte

pub struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    filled: u8,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            bytes: Vec::new(),
            current: 0,
            filled: 0,
        }
    }

    pub fn push(&mut self, bit: bool) {
        self.current = (self.current << 1) | bit as u8;
        self.filled += 1;
        if self.filled == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.filled = 0;
        }
    }

    // el ultimo byte se completa con ceros
    pub fn finish(mut self) -> Vec<u8> {
        if self.filled > 0 {
            self.bytes.push(self.current << (8 - self.filled));
        }
        self.bytes
    }
}

pub struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> BitReader<'a> {
        BitReader { bytes, pos: 0 }
    }
}

impl Iterator for BitReader<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        let byte = *self.bytes.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1 == 1;
        self.pos += 1;
        Some(bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let bits = [
            true, false, true, true, false, false, true, false, true, true,
        ];
        let mut w = BitWriter::new();
        for b in bits {
            w.push(b);
        }
        let bytes = w.finish();
        assert_eq!(bytes, [0b1011_0010, 0b1100_0000]);

        let leidos: Vec<bool> = BitReader::new(&bytes).take(bits.len()).collect();
        assert_eq!(leidos, bits);
    }

    #[test]
    fn sin_bits() {
        assert!(BitWriter::new().finish().is_empty());
        assert_eq!(BitReader::new(&[]).next(), None);
    }
}
