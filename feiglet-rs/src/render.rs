// Character rendering, kerning, smushing

use crate::font::{FIGcharacter, FIGfont};

pub fn lookup_char<'a>(
    font: &'a FIGfont,
    code: u32,
    current_width: &mut usize,
) -> &'a FIGcharacter {
    let ch = font.chars.get(&code).unwrap_or_else(|| {
        font.chars
            .get(&0)
            .expect("FIGfont missing required char code 0")
    });
    *current_width = ch.width();
    ch
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_font() -> FIGfont {
        let mut chars = HashMap::new();
        chars.insert(0, FIGcharacter::from(vec!["#".to_string()]));
        chars.insert(65, FIGcharacter::from(vec![" A ".to_string()]));
        chars.insert(66, FIGcharacter::from(vec![" B  ".to_string()]));
        FIGfont {
            chars,
            ..FIGfont::default()
        }
    }

    #[test]
    fn test_lookup_char_known() {
        let font = test_font();
        let mut current_width = 0;
        let ch = lookup_char(&font, 65, &mut current_width);
        assert_eq!(ch.width(), 3);
        assert_eq!(current_width, 3);
        assert_eq!(ch.rows(), &[" A "]);
    }

    #[test]
    fn test_lookup_char_fallback() {
        let font = test_font();
        let mut current_width = 0;
        let ch = lookup_char(&font, 9999, &mut current_width);
        assert_eq!(ch.width(), 1);
        assert_eq!(current_width, 1);
        assert_eq!(ch.rows(), &["#"]);
    }

    #[test]
    fn test_lookup_char_previous_width() {
        let font = test_font();
        let mut current_width = 0;
        lookup_char(&font, 65, &mut current_width);
        assert_eq!(current_width, 3);
        let previous_width = current_width;
        lookup_char(&font, 66, &mut current_width);
        assert_eq!(current_width, 4);
        assert_eq!(previous_width, 3);
    }
}
