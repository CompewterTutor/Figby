// Property-based fuzz testing for FIGfont parser.
//
// Each test generates random (often malformed) inputs and asserts
// the parser never panics — only returns Ok or Err.

use figby::font::{parse_char_data, parse_codetagged, parse_header, parse_tlf_font, FIGfont};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_parse_header(s in any::<String>()) {
        let _ = parse_header(&s);
    }

    #[test]
    fn fuzz_parse_tlf_font(s in any::<String>()) {
        let _ = parse_tlf_font(&s);
    }

    #[test]
    fn fuzz_parse_char_data(
        lines in prop::collection::vec(any::<String>(), 0..200),
        height in 1..20u32,
    ) {
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        let _ = parse_char_data(&mut font, &lines);
    }

    #[test]
    fn fuzz_parse_codetagged(
        lines in prop::collection::vec(any::<String>(), 0..100),
        height in 1..10u32,
    ) {
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        let _ = parse_codetagged(&mut font, &lines);
    }
}
