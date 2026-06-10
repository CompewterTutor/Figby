// Character rendering, kerning, smushing

use crate::font::{FIGcharacter, FIGfont};
use crate::smush::{smush_horizontal, SmushMode};

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

fn last_non_space(s: &str, fallback_pos: usize, fallback_char: char) -> (usize, char) {
    for (i, c) in s.char_indices().rev() {
        if c != ' ' {
            return (i, c);
        }
    }
    (fallback_pos, fallback_char)
}

fn first_non_space(s: &str, fallback_pos: usize, fallback_char: char) -> (usize, char) {
    for (i, c) in s.char_indices() {
        if c != ' ' {
            return (i, c);
        }
    }
    (fallback_pos, fallback_char)
}

/// Maximum overlap between current character and output line.
///
/// For each row, find last non-space in output and first non-space
/// in current character. Minimum across all rows determines the
/// smush amount. Handles left-to-right and right-to-left.
///
/// Mirror of C `smushamt()` in figlet.c:1446-1485.
pub fn calc_smush_amount(
    output_rows: &[String],
    curr_rows: &[String],
    outlinelen: usize,
    currcharwidth: usize,
    mode: SmushMode,
    hardblank: char,
    right2left: bool,
) -> usize {
    if !mode.contains(SmushMode::KERN | SmushMode::SMUSH) {
        return 0;
    }

    let mut maxsmush = currcharwidth;

    for (output_row, curr_row) in output_rows.iter().zip(curr_rows.iter()) {
        let (linebd, ch1, charbd, ch2) = if right2left {
            let out_len = output_row.chars().count();
            if maxsmush > out_len {
                maxsmush = out_len;
            }

            let (charbd, c1) = last_non_space(curr_row, 0, ' ');
            let (linebd, c2) = first_non_space(output_row, outlinelen, '\0');

            (linebd, c1, charbd, c2)
        } else {
            let (linebd, c1) = last_non_space(output_row, 0, ' ');
            let (charbd, c2) = first_non_space(curr_row, currcharwidth, '\0');

            (linebd, c1, charbd, c2)
        };

        let amt = if right2left {
            (linebd + currcharwidth).saturating_sub(1 + charbd)
        } else {
            (charbd + outlinelen).saturating_sub(1 + linebd)
        };

        let amt = if ch1 == ' '
            || ch1 == '\0'
            || (ch2 != '\0'
                && smush_horizontal(
                    ch1,
                    ch2,
                    mode,
                    hardblank,
                    outlinelen,
                    currcharwidth,
                    right2left,
                )
                .is_some())
        {
            amt + 1
        } else {
            amt
        };

        if amt < maxsmush {
            maxsmush = amt;
        }
    }

    maxsmush
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

    // --- calc_smush_amount tests ---

    const HB: char = '$';

    #[test]
    fn test_smush_no_mode() {
        let mode = SmushMode::new(0);
        let output = vec!["A".to_string()];
        let curr = vec!["B".to_string()];
        let result = calc_smush_amount(&output, &curr, 1, 1, mode, HB, false);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_smush_kerning_only() {
        let mode = SmushMode::new(SmushMode::KERN);
        let output = vec!["A".to_string()];
        let curr = vec!["A".to_string()];
        let result = calc_smush_amount(&output, &curr, 1, 1, mode, HB, false);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_smush_ltr_basic() {
        let mode = SmushMode::new(SmushMode::SMUSH);
        let output = vec!["A".to_string()];
        let curr = vec![" B ".to_string()];
        let result = calc_smush_amount(&output, &curr, 1, 3, mode, HB, false);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_smush_rtl_basic() {
        let mode = SmushMode::new(SmushMode::SMUSH);
        let output = vec![" A".to_string()];
        let curr = vec!["B ".to_string()];
        let result = calc_smush_amount(&output, &curr, 2, 2, mode, HB, true);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_smush_takes_row_min() {
        let mode = SmushMode::new(SmushMode::SMUSH);
        let output = vec!["AAA".to_string(), "A A".to_string()];
        let curr = vec!["  B".to_string(), "B  ".to_string()];
        let result = calc_smush_amount(&output, &curr, 3, 3, mode, HB, false);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_smush_boundary_smush() {
        let mode = SmushMode::new(SmushMode::SMUSH | SmushMode::EQUAL_CHARS);
        let output = vec!["A".to_string()];
        let curr = vec![" A".to_string()];
        let result = calc_smush_amount(&output, &curr, 1, 2, mode, HB, false);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_smush_boundary_no_smush() {
        let mode = SmushMode::new(SmushMode::SMUSH | SmushMode::EQUAL_CHARS);
        let output = vec!["A".to_string()];
        let curr = vec![" B".to_string()];
        let result = calc_smush_amount(&output, &curr, 1, 2, mode, HB, false);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_smush_output_all_spaces() {
        let mode = SmushMode::new(SmushMode::SMUSH);
        let output = vec!["  ".to_string()];
        let curr = vec!["A ".to_string()];
        let result = calc_smush_amount(&output, &curr, 2, 2, mode, HB, false);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_smush_curr_all_spaces() {
        let mode = SmushMode::new(SmushMode::SMUSH);
        let output = vec!["A ".to_string()];
        let curr = vec!["  ".to_string()];
        let result = calc_smush_amount(&output, &curr, 2, 2, mode, HB, false);
        assert_eq!(result, 2);
    }
}
