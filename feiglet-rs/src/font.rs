// FIGfont/TLF font parser
//
// Core data types for parsed font data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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

/// Errors that can occur during font parsing.
#[derive(Debug, PartialEq)]
pub enum FontError {
    /// The magic number doesn't match a known font format.
    InvalidMagic,
    /// A general parsing error occurred.
    ParseError(String),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::InvalidMagic => write!(f, "invalid font magic number"),
            FontError::ParseError(msg) => write!(f, "font parse error: {}", msg),
        }
    }
}

impl std::error::Error for FontError {}

/// The 7 Deutsch characters supported by FIGlet: Ä, Ö, Ü, ä, ö, ü, ß.
pub(crate) const DEUTSCH_CHARS: [u32; 7] = [196, 214, 220, 228, 246, 252, 223];

/// Strip trailing endmark characters from a font file line.
///
/// Follows `figlet.c:1155-1165` algorithm:
/// 1. Strip trailing whitespace
/// 2. Last remaining char is the endmark
/// 3. Remove all consecutive endmark chars from the right
///
/// Trailing whitespace before endmarks is preserved (width correctness).
/// Returns empty string if the line is all endmarks/whitespace.
fn strip_endmarks(line: &str) -> String {
    let trimmed = line.trim_end_matches(|c: char| c.is_ascii_whitespace());
    let endmark = match trimmed.chars().last() {
        Some(c) => c,
        None => return String::new(),
    };
    let endmark_len = endmark.len_utf8();
    let mut end_pos = trimmed.len();
    while end_pos >= endmark_len {
        let slice = &trimmed[end_pos - endmark_len..end_pos];
        if slice.starts_with(endmark) {
            end_pos -= endmark_len;
        } else {
            break;
        }
    }
    trimmed[..end_pos].to_string()
}

/// Parse the 95 required ASCII FIGcharacters (codes 32–126) and 7 Deutsch chars.
///
/// Returns the unconsumed slice of lines (for subsequent codetag parsing).
/// Each character reads `font.charheight` rows, stripping endmarks via
/// `strip_endmarks()`. Stores parsed glyphs in `font.chars` keyed by char code.
pub fn parse_char_data<'a>(
    font: &mut FIGfont,
    lines: &'a [String],
) -> Result<&'a [String], FontError> {
    let height = font.charheight as usize;
    let mut cursor = 0;

    // ASCII chars 32–126 (95 characters)
    for code in 32..=126 {
        if cursor + height > lines.len() {
            let parsed = cursor / height;
            return Err(FontError::ParseError(format!(
                "unexpected end of font data: parsed {} ASCII chars, need {} more rows for code {}",
                parsed, height, code
            )));
        }
        let rows: Vec<String> = lines[cursor..cursor + height]
            .iter()
            .map(|l| strip_endmarks(l))
            .collect();
        font.chars.insert(code, FIGcharacter::from(rows));
        cursor += height;
    }

    // Deutsch chars
    for &code in &DEUTSCH_CHARS {
        if cursor + height > lines.len() {
            let parsed = cursor / height - 95; // subtract ASCII chars
            return Err(FontError::ParseError(format!(
                "unexpected end of font data: parsed {} Deutsch chars, need {} more rows for code {}",
                parsed, height, code
            )));
        }
        let rows: Vec<String> = lines[cursor..cursor + height]
            .iter()
            .map(|l| strip_endmarks(l))
            .collect();
        font.chars.insert(code, FIGcharacter::from(rows));
        cursor += height;
    }

    Ok(&lines[cursor..])
}

/// Parse the header line of a FIGfont (.flf) file.
///
/// Expected format:
/// `flf2a<hardblank> <height> <baseline> <max_length> <old_layout> <comment_lines>`
/// `[<print_direction> [<full_layout> [<codetag_count>]]]`
///
/// Missing optional fields are defaulted following FIGlet 2.2.5 conventions.
pub fn parse_header(line: &str) -> Result<FIGfont, FontError> {
    if !line.starts_with("flf2a") {
        return Err(FontError::InvalidMagic);
    }

    let rest = &line[5..];
    if rest.is_empty() {
        return Err(FontError::InvalidMagic);
    }

    let hardblank = rest.chars().next().ok_or(FontError::InvalidMagic)?;
    let rest = rest[hardblank.len_utf8()..].trim_start();

    let tokens: Vec<&str> = rest.split_whitespace().collect();

    if tokens.len() < 5 {
        return Err(FontError::ParseError(format!(
            "expected at least 5 numeric fields after hardblank, got {}",
            tokens.len()
        )));
    }

    let height = tokens[0].parse::<i32>().map_err(|e| {
        FontError::ParseError(format!("invalid height value '{}': {}", tokens[0], e))
    })?;
    let baseline = tokens[1].parse::<i32>().map_err(|e| {
        FontError::ParseError(format!("invalid baseline value '{}': {}", tokens[1], e))
    })?;
    let max_length = tokens[2].parse::<i32>().map_err(|e| {
        FontError::ParseError(format!("invalid max_length value '{}': {}", tokens[2], e))
    })?;
    let old_layout = tokens[3].parse::<i32>().map_err(|e| {
        FontError::ParseError(format!("invalid old_layout value '{}': {}", tokens[3], e))
    })?;
    let comment_lines = tokens[4].parse::<i32>().map_err(|e| {
        FontError::ParseError(format!(
            "invalid comment_lines value '{}': {}",
            tokens[4], e
        ))
    })?;

    let print_direction = if tokens.len() > 5 {
        tokens[5].parse::<i32>().map_err(|e| {
            FontError::ParseError(format!(
                "invalid print_direction value '{}': {}",
                tokens[5], e
            ))
        })?
    } else {
        -1
    };

    let full_layout = if tokens.len() > 6 {
        tokens[6].parse::<i32>().map_err(|e| {
            FontError::ParseError(format!("invalid full_layout value '{}': {}", tokens[6], e))
        })?
    } else if old_layout == 0 {
        64
    } else if old_layout < 0 {
        0
    } else {
        (old_layout & 31) | 128
    };

    let codetag_count = if tokens.len() > 7 {
        tokens[7].parse::<i32>().map_err(|e| {
            FontError::ParseError(format!(
                "invalid codetag_count value '{}': {}",
                tokens[7], e
            ))
        })?
    } else {
        0
    };

    Ok(FIGfont {
        hardblank,
        charheight: height as u32,
        baseline: baseline as u32,
        maxlength: max_length as u32,
        old_layout,
        full_layout,
        print_direction,
        comment_lines: comment_lines as u32,
        chars: HashMap::new(),
        codetag_count: codetag_count as u32,
    })
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

    #[test]
    fn test_parse_header_full() {
        let result = parse_header("flf2a$ 6 5 20 15 3 0 143 229");
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.hardblank, '$');
        assert_eq!(font.charheight, 6);
        assert_eq!(font.baseline, 5);
        assert_eq!(font.maxlength, 20);
        assert_eq!(font.old_layout, 15);
        assert_eq!(font.comment_lines, 3);
        assert_eq!(font.print_direction, 0);
        assert_eq!(font.full_layout, 143);
        assert_eq!(font.codetag_count, 229);
        assert!(font.chars.is_empty());
    }

    #[test]
    fn test_parse_header_minimal() {
        let result = parse_header("flf2a$ 6 5 20 15 3");
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.hardblank, '$');
        assert_eq!(font.charheight, 6);
        assert_eq!(font.baseline, 5);
        assert_eq!(font.maxlength, 20);
        assert_eq!(font.old_layout, 15);
        assert_eq!(font.comment_lines, 3);
        assert_eq!(font.print_direction, -1);
        assert_eq!(font.full_layout, (15 & 31) | 128);
        assert_eq!(font.codetag_count, 0);
    }

    #[test]
    fn test_parse_header_old_layout_zero() {
        let result = parse_header("flf2a$ 6 5 20 0 3");
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.old_layout, 0);
        assert_eq!(font.full_layout, 64);
    }

    #[test]
    fn test_parse_header_old_layout_negative() {
        let result = parse_header("flf2a$ 6 5 20 -1 3");
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.old_layout, -1);
        assert_eq!(font.full_layout, 0);
    }

    #[test]
    fn test_parse_header_with_print_direction() {
        let result = parse_header("flf2a$ 8 3 15 5 2 1");
        assert!(result.is_ok());
        let font = result.unwrap();
        assert_eq!(font.charheight, 8);
        assert_eq!(font.baseline, 3);
        assert_eq!(font.maxlength, 15);
        assert_eq!(font.old_layout, 5);
        assert_eq!(font.comment_lines, 2);
        assert_eq!(font.print_direction, 1);
        assert_eq!(font.full_layout, (5 & 31) | 128);
    }

    #[test]
    fn test_parse_header_invalid_magic() {
        let result = parse_header("flf2b$ 6 5 20 15 3");
        assert_eq!(result, Err(FontError::InvalidMagic));
    }

    #[test]
    fn test_parse_header_wrong_prefix() {
        let result = parse_header("xyzzy$ 6 5 20 15 3");
        assert_eq!(result, Err(FontError::InvalidMagic));
    }

    #[test]
    fn test_parse_header_empty() {
        let result = parse_header("");
        assert_eq!(result, Err(FontError::InvalidMagic));
    }

    #[test]
    fn test_parse_header_truncated_magic() {
        let result = parse_header("flf");
        assert_eq!(result, Err(FontError::InvalidMagic));
    }

    #[test]
    fn test_parse_header_not_enough_fields() {
        let result = parse_header("flf2a$ 6 5 20");
        assert!(result.is_err());
        assert!(matches!(result, Err(FontError::ParseError(_))));
    }

    #[test]
    fn test_parse_header_non_numeric_field() {
        let result = parse_header("flf2a$ 6 x 20 15 3");
        assert!(result.is_err());
        assert!(matches!(result, Err(FontError::ParseError(_))));
    }

    // --- strip_endmarks tests ---

    #[test]
    fn test_strip_endmarks_typical() {
        assert_eq!(strip_endmarks("  __  @"), "  __  ");
        assert_eq!(strip_endmarks("$@@"), "$");
        assert_eq!(strip_endmarks(" @"), " ");
    }

    #[test]
    fn test_strip_endmarks_trailing_newline() {
        // With trailing newline: "  __  @\n"
        // trim_end_matches(whitespace) removes \n → "  __  @"
        // Then same as above
        assert_eq!(strip_endmarks("  __  @\n"), "  __  ");
        assert_eq!(strip_endmarks("$@@\n"), "$");
    }

    #[test]
    fn test_strip_endmarks_no_endmark() {
        // Already clean — no endmarks present
        let line = "hello";
        assert_eq!(strip_endmarks(line), "hello");
    }

    #[test]
    fn test_strip_endmarks_empty() {
        assert_eq!(strip_endmarks(""), "");
        // "\n" → trim whitespace → "" → empty
        assert_eq!(strip_endmarks("\n"), "");
        // "\r\n" → trim whitespace → "" → empty
        assert_eq!(strip_endmarks("\r\n"), "");
    }

    #[test]
    fn test_strip_endmarks_whitespace_only() {
        assert_eq!(strip_endmarks("   \n"), "");
        assert_eq!(strip_endmarks("\t\n"), "");
    }

    #[test]
    fn test_strip_endmarks_multi_char_endmark() {
        // Endmark is '@', lines with multiple endmarks
        assert_eq!(strip_endmarks("AAA@@@@"), "AAA");
        assert_eq!(strip_endmarks("  X  @@@"), "  X  ");
    }

    #[test]
    fn test_strip_endmarks_trailing_spaces_preserved() {
        // Trailing spaces before endmarks are preserved per C spec
        assert_eq!(strip_endmarks("X  @"), "X  ");
        assert_eq!(strip_endmarks("  X  @"), "  X  ");
    }

    // --- parse_char_data tests ---

    fn build_102_char_fixture(height: u32) -> Vec<String> {
        let mut lines = Vec::new();
        for code in 32..=126u32 {
            let c = char::from_u32(code).expect("valid ASCII");
            for _ in 0..height {
                lines.push(format!("{}  @", c));
            }
        }
        for &code in &DEUTSCH_CHARS {
            let c = char::from_u32(code).expect("valid Deutsch char");
            for _ in 0..height {
                lines.push(format!("{}  @", c));
            }
        }
        lines
    }

    #[test]
    fn test_parse_char_data_102_chars() {
        let height = 2;
        let fixture = build_102_char_fixture(height);
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        let remaining = parse_char_data(&mut font, &fixture).expect("parse should succeed");
        assert!(remaining.is_empty(), "all lines should be consumed");

        // Must have 95 + 7 = 102 chars
        assert_eq!(font.chars.len(), 102, "should have exactly 102 chars");

        // All ASCII keys 32..=126 present
        for code in 32..=126u32 {
            assert!(
                font.chars.contains_key(&code),
                "missing ASCII char code {code}"
            );
        }

        // All Deutsch keys present
        for &code in &DEUTSCH_CHARS {
            assert!(
                font.chars.contains_key(&code),
                "missing Deutsch char code {code}"
            );
        }
    }

    #[test]
    fn test_parse_char_data_endmarks_stripped() {
        let height = 1;
        let fixture = build_102_char_fixture(height);
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        parse_char_data(&mut font, &fixture).expect("parse should succeed");

        for (code, ch) in &font.chars {
            for (i, row) in ch.rows().iter().enumerate() {
                assert!(
                    !row.ends_with('@'),
                    "char code {code} row {i} still ends with '@': '{row}'"
                );
            }
        }
    }

    #[test]
    fn test_parse_char_data_widths_consistent() {
        let height = 3;
        let fixture = build_102_char_fixture(height);
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        parse_char_data(&mut font, &fixture).expect("parse should succeed");

        for (code, ch) in &font.chars {
            let rows = ch.rows();
            assert!(!rows.is_empty(), "char code {code} has no rows");
            let expected_width = rows[0].len();
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(
                    row.len(),
                    expected_width,
                    "char code {code} row {i} width mismatch: expected {expected_width}, got {}",
                    row.len()
                );
            }
        }
    }

    #[test]
    fn test_parse_char_data_too_few_lines() {
        let height = 2;
        // Only provide 10 lines — not enough for even 1 char (needs height=2)
        let fixture: Vec<String> = (0..10).map(|i| format!("line{i}@")).collect();
        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        let result = parse_char_data(&mut font, &fixture);
        assert!(result.is_err(), "should fail with too few lines");
        match &result {
            Err(FontError::ParseError(msg)) => {
                assert!(
                    msg.contains("unexpected end of font data"),
                    "error should mention end of font data, got: {msg}"
                );
            }
            other => panic!("expected ParseError, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_char_data_returns_unconsumed() {
        // Provide extra lines beyond the 102 chars
        let height = 1;
        let mut fixture = build_102_char_fixture(height);
        fixture.push("codetag 0".to_string());
        fixture.push("row1@".to_string());

        let mut font = FIGfont {
            charheight: height,
            ..FIGfont::default()
        };
        let remaining = parse_char_data(&mut font, &fixture).expect("parse should succeed");

        assert_eq!(remaining.len(), 2, "should return 2 unconsumed lines");
        assert_eq!(remaining[0], "codetag 0");
        assert_eq!(remaining[1], "row1@");
    }
}
