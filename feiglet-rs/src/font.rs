// FIGfont/TLF font parser
//
// Core data types for parsed font data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single character glyph in a FIGfont.
///
/// Stores each row of the character as a `String`. The number of rows
/// equals the font's `charheight`. Width is derived from the first row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FIGcharacter {
    rows: Vec<String>,
}

impl FIGcharacter {
    /// Width of the character (length of first row), or 0 if empty.
    pub fn width(&self) -> usize {
        self.rows.first().map_or(0, |r| r.len())
    }

    /// Access the character's rows.
    pub fn rows(&self) -> &[String] {
        &self.rows
    }
}

impl From<Vec<String>> for FIGcharacter {
    fn from(rows: Vec<String>) -> Self {
        FIGcharacter { rows }
    }
}

/// A character node mapping a character code to its glyph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FCharnode {
    pub ord: u32,
    pub character: FIGcharacter,
}

/// A parsed FIGfont or TOIlet font.
///
/// Owns all character glyphs and font metadata extracted from the
/// font file header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FIGfont {
    /// The hardblank character (e.g., `$`)
    pub hardblank: char,
    /// Font height in lines
    pub charheight: u32,
    /// Baseline offset
    pub baseline: u32,
    /// Maximum character width
    pub maxlength: u32,
    /// Legacy layout mode (smush)
    pub old_layout: i32,
    /// Full layout flags (smush2)
    pub full_layout: i32,
    /// Print direction: -1 = unset, 0 = left-to-right, 1 = right-to-left
    pub print_direction: i32,
    /// Number of comment lines after header
    pub comment_lines: u32,
    /// Character storage: code → glyph
    pub chars: HashMap<u32, FIGcharacter>,
    /// Number of expected codetagged characters
    pub codetag_count: u32,
}

impl Default for FIGfont {
    fn default() -> Self {
        FIGfont {
            hardblank: '$',
            charheight: 1,
            baseline: 0,
            maxlength: 1,
            old_layout: 0,
            full_layout: 0,
            print_direction: -1,
            comment_lines: 0,
            chars: HashMap::new(),
            codetag_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_figcharacter_default() {
        let c = FIGcharacter::default();
        assert!(c.rows().is_empty());
        assert_eq!(c.width(), 0);
    }

    #[test]
    fn test_figcharacter_with_rows() {
        let rows = vec!["  __  ".to_string(), " / _ \\ ".to_string()];
        let c = FIGcharacter::from(rows);
        assert_eq!(c.width(), 6);
        assert_eq!(c.rows().len(), 2);
    }

    #[test]
    fn test_ficharnode_new() {
        let c = FIGcharacter::from(vec![" A ".to_string()]);
        let node = FCharnode {
            ord: 65,
            character: c,
        };
        assert_eq!(node.ord, 65);
        assert_eq!(node.character.width(), 3);
    }

    #[test]
    fn test_figfont_default() {
        let font = FIGfont::default();
        assert_eq!(font.hardblank, '$');
        assert_eq!(font.charheight, 1);
        assert_eq!(font.maxlength, 1);
        assert_eq!(font.old_layout, 0);
        assert_eq!(font.full_layout, 0);
        assert_eq!(font.print_direction, -1);
        assert_eq!(font.comment_lines, 0);
        assert!(font.chars.is_empty());
        assert_eq!(font.codetag_count, 0);
    }

    #[test]
    fn test_figfont_with_char() {
        let mut font = FIGfont::default();
        let c = FIGcharacter::from(vec![" X ".to_string()]);
        font.chars.insert(88, c);
        let retrieved = font.chars.get(&88);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().width(), 3);
    }

    #[test]
    fn test_figcharacter_serde_roundtrip() {
        let original = FIGcharacter::from(vec!["  __  ".to_string(), " / _ \\ ".to_string()]);
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: FIGcharacter = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_ficharnode_serde_roundtrip() {
        let original = FCharnode {
            ord: 65,
            character: FIGcharacter::from(vec![" A ".to_string()]),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: FCharnode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_figfont_serde_roundtrip() {
        let original = FIGfont {
            charheight: 6,
            maxlength: 10,
            old_layout: 3,
            full_layout: 7,
            comment_lines: 5,
            chars: HashMap::from([(65, FIGcharacter::from(vec![" A ".to_string()]))]),
            ..FIGfont::default()
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: FIGfont = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }
}
