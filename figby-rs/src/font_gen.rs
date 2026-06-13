use crate::font::{FIGfont, DEUTSCH_CHARS};
use font_kit::error::FontLoadingError;
use font_kit::error::SelectionError;
use font_kit::font::Font;
use font_kit::handle::Handle;
use font_kit::source::SystemSource;
use std::fmt;

#[derive(Debug)]
pub enum FontGenError {
    Selection(SelectionError),
    FontLoading(FontLoadingError),
}

impl fmt::Display for FontGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontGenError::Selection(e) => write!(f, "font selection error: {e}"),
            FontGenError::FontLoading(e) => write!(f, "font loading error: {e}"),
        }
    }
}

impl std::error::Error for FontGenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FontGenError::Selection(e) => Some(e),
            FontGenError::FontLoading(e) => Some(e),
        }
    }
}

impl From<SelectionError> for FontGenError {
    fn from(e: SelectionError) -> Self {
        FontGenError::Selection(e)
    }
}

impl From<FontLoadingError> for FontGenError {
    fn from(e: FontLoadingError) -> Self {
        FontGenError::FontLoading(e)
    }
}

#[derive(Debug, Clone)]
pub struct FontFamilyInfo {
    pub family: String,
    pub styles: Vec<String>,
}

fn describe_style(font: &Font) -> String {
    let props = font.properties();
    format!(
        "Weight: {}, Style: {:?}",
        props.weight.0 as u32, props.style
    )
}

fn family_is_monospace(name: &str, source: &SystemSource) -> bool {
    if name.to_lowercase().contains("mono") {
        return true;
    }
    if let Ok(family_handle) = source.select_family_by_name(name) {
        let handles = family_handle.fonts();
        if let Some(handle) = handles.first() {
            if let Ok(font) = handle.load() {
                return font.is_monospace();
            }
        }
    }
    false
}

fn load_styles(handles: &[Handle]) -> Vec<String> {
    handles
        .iter()
        .filter_map(|handle| {
            let font = handle.load().ok()?;
            Some(describe_style(&font))
        })
        .collect()
}

pub fn list_system_fonts() -> Result<Vec<FontFamilyInfo>, FontGenError> {
    let source = SystemSource::new();
    let family_names = source.all_families()?;
    let mut result = Vec::with_capacity(family_names.len());

    for name in family_names {
        let styles = match source.select_family_by_name(&name) {
            Ok(family_handle) => load_styles(family_handle.fonts()),
            Err(_) => Vec::new(),
        };
        result.push(FontFamilyInfo {
            family: name,
            styles,
        });
    }

    Ok(result)
}

pub fn list_monospace_fonts() -> Result<Vec<FontFamilyInfo>, FontGenError> {
    let source = SystemSource::new();
    let family_names = source.all_families()?;
    let mut result = Vec::new();

    for name in family_names {
        if family_is_monospace(&name, &source) {
            let styles = match source.select_family_by_name(&name) {
                Ok(family_handle) => load_styles(family_handle.fonts()),
                Err(_) => Vec::new(),
            };
            result.push(FontFamilyInfo {
                family: name,
                styles,
            });
        }
    }

    Ok(result)
}

/// Generate FIGfont header line from font metrics.
///
/// Format: `flf2a<hardblank> <height> <baseline> <max_length> <old_layout> <comment_lines> [<print_direction> [<full_layout> [<codetag_count>]]]`
///
/// Generated header uses old_layout=0 (full-size), comment_lines=0,
/// print_direction=-1 (unset), and codetag_count=0.
pub fn generate_figfont_header(font: &FIGfont) -> String {
    format!(
        "flf2a{hb} {h} {b} {ml} 0 0 -1 {fl} 0",
        hb = font.hardblank,
        h = font.charheight,
        b = font.baseline,
        ml = font.maxlength,
        fl = font.full_layout,
    )
}

/// Generate complete .flf file content from font metrics and glyph data.
///
/// Produces header + all 95 required ASCII chars (codes 32-126), 7 Deutsch
/// chars, and any codetagged chars. Missing required chars use space-padded
/// rows of `maxlength` width. Each row terminated with `@`.
pub fn generate_figfont(font: &FIGfont) -> String {
    let mut result = String::new();
    result.push_str(&generate_figfont_header(font));
    result.push('\n');

    let height = font.charheight as usize;
    let pad_row = " ".repeat(font.maxlength as usize);

    for code in 32..=126u32 {
        let ch = font.chars.get(&code);
        for row_i in 0..height {
            let row = ch
                .and_then(|c| c.rows().get(row_i))
                .map(|s| s.as_str())
                .unwrap_or(&pad_row);
            result.push_str(row);
            result.push_str("@\n");
        }
    }

    for &code in &DEUTSCH_CHARS {
        let ch = font.chars.get(&code);
        for row_i in 0..height {
            let row = ch
                .and_then(|c| c.rows().get(row_i))
                .map(|s| s.as_str())
                .unwrap_or(&pad_row);
            result.push_str(row);
            result.push_str("@\n");
        }
    }

    let mut codetagged: Vec<u32> = font
        .chars
        .keys()
        .copied()
        .filter(|code| !(32..=126).contains(code) && !DEUTSCH_CHARS.contains(code))
        .collect();
    codetagged.sort_unstable();

    for code in codetagged {
        let ch = &font.chars[&code];
        result.push_str(&code.to_string());
        result.push('\n');
        for row in ch.rows() {
            result.push_str(row);
            result.push_str("@\n");
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{parse_header, parse_tlf_font, FIGcharacter};

    #[test]
    fn test_list_system_fonts_not_empty() {
        let fonts = list_system_fonts().expect("system font enumeration should succeed");
        assert!(
            !fonts.is_empty(),
            "at least one font family should be found"
        );
    }

    #[test]
    fn test_list_monospace_fonts_returns_subset() {
        let fonts = list_monospace_fonts().expect("monospace font enumeration should succeed");
        assert!(
            !fonts.is_empty(),
            "at least one monospace font family should be found"
        );
        for info in &fonts {
            let is_mono =
                info.family.to_lowercase().contains("mono") || is_any_font_monospace(&info.family);
            assert!(is_mono, "family '{}' should be monospace", info.family);
        }
    }

    #[test]
    fn test_font_family_info_has_styles() {
        let fonts = list_system_fonts().expect("system font enumeration should succeed");
        let any_with_styles = fonts.iter().any(|f| !f.styles.is_empty());
        assert!(
            any_with_styles,
            "at least one family should have non-empty styles"
        );
    }

    fn is_any_font_monospace(name: &str) -> bool {
        let source = SystemSource::new();
        if let Ok(family_handle) = source.select_family_by_name(name) {
            if let Some(handle) = family_handle.fonts().first() {
                if let Ok(font) = handle.load() {
                    return font.is_monospace();
                }
            }
        }
        false
    }

    // --- FIGfont header generation tests ---

    #[test]
    fn test_generate_header_roundtrip() {
        let font = FIGfont {
            hardblank: '$',
            charheight: 8,
            baseline: 7,
            maxlength: 15,
            full_layout: 64,
            ..FIGfont::default()
        };
        let header = generate_figfont_header(&font);
        let parsed = parse_header(&header).expect("should parse generated header");

        assert_eq!(parsed.hardblank, '$');
        assert_eq!(parsed.charheight, 8);
        assert_eq!(parsed.baseline, 7);
        assert_eq!(parsed.maxlength, 15);
        assert_eq!(parsed.old_layout, 0);
        assert_eq!(parsed.comment_lines, 0);
        assert_eq!(parsed.print_direction, -1);
        assert_eq!(parsed.full_layout, 64);
        assert_eq!(parsed.codetag_count, 0);
    }

    #[test]
    fn test_generate_header_defaults_full_size() {
        let font = FIGfont::default();
        let header = generate_figfont_header(&font);
        let parsed = parse_header(&header).expect("should parse generated header");

        assert_eq!(parsed.old_layout, 0);
        assert_eq!(parsed.full_layout, 64);
        assert_eq!(parsed.print_direction, -1);
        assert_eq!(parsed.codetag_count, 0);
    }

    #[test]
    fn test_generate_header_smush_layout() {
        let font = FIGfont {
            hardblank: '$',
            charheight: 6,
            baseline: 5,
            maxlength: 20,
            full_layout: 191,
            ..FIGfont::default()
        };
        let header = generate_figfont_header(&font);
        let parsed = parse_header(&header).expect("should parse generated header");
        assert_eq!(parsed.full_layout, 191);
    }

    #[test]
    fn test_generate_header_hardblank_multi_byte() {
        let font = FIGfont {
            hardblank: '\u{7f}',
            charheight: 3,
            baseline: 3,
            maxlength: 8,
            full_layout: 0,
            ..FIGfont::default()
        };
        let header = generate_figfont_header(&font);
        assert!(
            header.starts_with("flf2a\u{7f}"),
            "header should start with flf2a + hardblank DEL, got: {header:?}"
        );
        let parsed =
            parse_header(&header).expect("should parse generated header with DEL hardblank");
        assert_eq!(parsed.hardblank, '\u{7f}');
        assert_eq!(parsed.charheight, 3);
    }

    #[test]
    fn test_generate_figfont_full_roundtrip() {
        let mut font = FIGfont {
            hardblank: '$',
            charheight: 2,
            baseline: 1,
            maxlength: 10,
            full_layout: 64,
            ..FIGfont::default()
        };

        font.chars.insert(
            32,
            FIGcharacter::from(vec!["   ".to_string(), "   ".to_string()]),
        );
        font.chars.insert(
            65,
            FIGcharacter::from(vec!["  A  ".to_string(), " AAA ".to_string()]),
        );

        font.chars.insert(
            200,
            FIGcharacter::from(vec![" char ".to_string(), " 200  ".to_string()]),
        );

        let content = generate_figfont(&font);
        let parsed = parse_tlf_font(&content).expect("should parse generated .flf content");

        assert_eq!(parsed.hardblank, font.hardblank);
        assert_eq!(parsed.charheight, font.charheight);
        assert_eq!(parsed.baseline, font.baseline);
        assert_eq!(parsed.maxlength, font.maxlength);
        assert_eq!(parsed.old_layout, 0);
        assert_eq!(parsed.full_layout, font.full_layout);
        assert_eq!(parsed.print_direction, -1);
        assert_eq!(parsed.codetag_count, 0);

        // 102 required chars + 1 codetagged = 103
        assert!(
            parsed.chars.len() >= 103,
            "should have at least 103 chars, got {}",
            parsed.chars.len()
        );

        let ch65 = parsed
            .chars
            .get(&65)
            .expect("code 65 (A) should exist in output");
        assert_eq!(ch65.rows(), &["  A  ", " AAA "]);

        let ch200 = parsed
            .chars
            .get(&200)
            .expect("code 200 should exist in output");
        assert_eq!(ch200.rows(), &[" char ", " 200  "]);
    }

    #[test]
    fn test_generate_figfont_empty_chars() {
        let font = FIGfont {
            hardblank: '$',
            charheight: 3,
            baseline: 2,
            maxlength: 8,
            full_layout: 64,
            chars: std::collections::HashMap::new(),
            ..FIGfont::default()
        };

        let content = generate_figfont(&font);
        let parsed = parse_tlf_font(&content).expect("should parse generated font with no chars");

        assert_eq!(parsed.charheight, 3);
        assert_eq!(parsed.maxlength, 8);
        // all 102 required chars should exist as placeholders
        assert_eq!(parsed.chars.len(), 102);

        // Verify some placeholder chars have correct row count
        let ch32 = parsed.chars.get(&32).expect("space should exist");
        assert_eq!(ch32.rows().len(), 3);
        let ch126 = parsed.chars.get(&126).expect("tilde should exist");
        assert_eq!(ch126.rows().len(), 3);
    }
}
