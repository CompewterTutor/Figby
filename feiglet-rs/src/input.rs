use crate::control::CharReader;

pub fn read_utf8_char(input: &mut impl CharReader) -> Option<u32> {
    let b0 = input.next()?;

    if b0 < 0x80 {
        return Some(b0);
    }

    let length = if b0 & 0xE0 == 0xC0 {
        2
    } else if b0 & 0xF0 == 0xE0 {
        3
    } else if b0 & 0xF8 == 0xF0 {
        4
    } else if b0 & 0xFC == 0xF8 {
        5
    } else if b0 & 0xFE == 0xFC {
        6
    } else {
        return Some(0x0080);
    };

    let mut buf = [0u8; 6];
    buf[0] = b0 as u8;

    for slot in buf.iter_mut().take(length).skip(1) {
        match input.next() {
            Some(b) if b & 0xC0 == 0x80 => {
                *slot = b as u8;
            }
            _ => {
                return Some(0x0080);
            }
        }
    }

    match std::str::from_utf8(&buf[..length]) {
        Ok(s) => s.chars().next().map(|c| c as u32),
        Err(_) => Some(0x0080),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockReader {
        data: Vec<u8>,
        pos: usize,
        buf: Option<u32>,
    }

    impl MockReader {
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
                pos: 0,
                buf: None,
            }
        }
    }

    impl CharReader for MockReader {
        fn next(&mut self) -> Option<u32> {
            if let Some(c) = self.buf.take() {
                return Some(c);
            }
            if self.pos < self.data.len() {
                let c = self.data[self.pos] as u32;
                self.pos += 1;
                Some(c)
            } else {
                None
            }
        }

        fn unget(&mut self, c: u32) {
            self.buf = Some(c);
        }
    }

    #[test]
    fn test_utf8_ascii() {
        let mut input = MockReader::new(b"abc");
        assert_eq!(read_utf8_char(&mut input), Some('a' as u32));
        assert_eq!(read_utf8_char(&mut input), Some('b' as u32));
        assert_eq!(read_utf8_char(&mut input), Some('c' as u32));
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_2byte() {
        // U+00A9 © = 0xC2 0xA9
        let mut input = MockReader::new(&[0xC2, 0xA9]);
        assert_eq!(read_utf8_char(&mut input), Some(0x00A9));
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_3byte() {
        // U+4E2D 中 = 0xE4 0xB8 0xAD
        let mut input = MockReader::new(&[0xE4, 0xB8, 0xAD]);
        assert_eq!(read_utf8_char(&mut input), Some(0x4E2D));
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_4byte() {
        // U+1F600 😀 = 0xF0 0x9F 0x98 0x80
        let mut input = MockReader::new(&[0xF0, 0x9F, 0x98, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x1F600));
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_overlong_c0() {
        // Overlong 2-byte encoding of NUL: 0xC0 0x80
        let mut input = MockReader::new(&[0xC0, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_overlong_c1() {
        // Overlong 2-byte encoding of 0x7F: 0xC1 0xBF
        let mut input = MockReader::new(&[0xC1, 0xBF]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_surrogate() {
        // Surrogate U+D800 encoded as 0xED 0xA0 0x80
        let mut input = MockReader::new(&[0xED, 0xA0, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_invalid_lead_byte() {
        // 0xFF
        let mut input = MockReader::new(&[0xFF]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
        // 0xFE
        let mut input = MockReader::new(&[0xFE]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_f5_plus() {
        // 0xF5 0x80 0x80 0x80 would decode to 0x140000 > U+10FFFF
        let mut input = MockReader::new(&[0xF5, 0x80, 0x80, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
        // 0xF6
        let mut input = MockReader::new(&[0xF6, 0x80, 0x80, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
        // 0xF7
        let mut input = MockReader::new(&[0xF7, 0x80, 0x80, 0x80]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_truncated() {
        // 2-byte sequence missing continuation byte
        let mut input = MockReader::new(&[0xC2]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
        // 3-byte sequence with only 1 continuation byte
        let mut input = MockReader::new(&[0xE4, 0xB8]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
        // Empty after partial
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_bad_continuation() {
        // Valid leading byte but invalid continuation (0x00 instead of 0x80..0xBF)
        let mut input = MockReader::new(&[0xC2, 0x00]);
        assert_eq!(read_utf8_char(&mut input), Some(0x0080));
    }

    #[test]
    fn test_utf8_eof_on_first_byte() {
        let mut input = MockReader::new(b"");
        assert_eq!(read_utf8_char(&mut input), None);
    }

    #[test]
    fn test_utf8_multiple_chars() {
        let mut input = MockReader::new("A©中😀".as_bytes());
        assert_eq!(read_utf8_char(&mut input), Some('A' as u32));
        assert_eq!(read_utf8_char(&mut input), Some(0x00A9));
        assert_eq!(read_utf8_char(&mut input), Some(0x4E2D));
        assert_eq!(read_utf8_char(&mut input), Some(0x1F600));
        assert_eq!(read_utf8_char(&mut input), None);
    }
}
