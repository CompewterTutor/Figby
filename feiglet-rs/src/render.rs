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

/// Append a character to the output line, applying kerning/smushing.
///
/// Mirrors C `addchar()` in figlet.c:1497-1537.
/// `output_rows` must be pre-initialized with `font.charheight` empty strings
/// before the first call. Returns `true` if the character was added,
/// `false` if the output line length would exceed `outlinelen_limit`.
#[allow(clippy::too_many_arguments)]
pub fn add_char(
    font: &FIGfont,
    code: u32,
    output_rows: &mut Vec<String>,
    outlinelen: &mut usize,
    prev_width: &mut usize,
    mode: SmushMode,
    right2left: bool,
    outlinelen_limit: usize,
) -> bool {
    let old_prev_width = *prev_width;
    let ch = lookup_char(font, code, prev_width);
    let curr_width = *prev_width;
    let curr_rows = ch.rows();

    let smush = calc_smush_amount(
        output_rows,
        curr_rows,
        *outlinelen,
        curr_width,
        mode,
        font.hardblank,
        right2left,
    );

    if *outlinelen + curr_width - smush > outlinelen_limit {
        *prev_width = old_prev_width;
        return false;
    }

    for (row_idx, curr_row) in curr_rows.iter().enumerate() {
        let out_chars: Vec<char> = if row_idx < output_rows.len() {
            output_rows[row_idx].chars().collect()
        } else {
            Vec::new()
        };
        let curr_chars: Vec<char> = curr_row.chars().collect();

        let result: String = if right2left {
            let mut temp = curr_chars.clone();
            let overlap = smush.min(curr_chars.len()).min(out_chars.len());
            for (k, rch) in out_chars.iter().enumerate().take(overlap) {
                let col = curr_width.saturating_sub(smush).saturating_add(k);
                if col < temp.len() {
                    let lch = temp[col];
                    if let Some(smushed) = smush_horizontal(
                        lch,
                        *rch,
                        mode,
                        font.hardblank,
                        old_prev_width,
                        curr_width,
                        right2left,
                    ) {
                        temp[col] = smushed;
                    }
                }
            }
            if smush < out_chars.len() {
                temp.extend(&out_chars[smush..]);
            }
            temp.into_iter().collect()
        } else {
            let mut out = out_chars;
            let overlap = smush.min(curr_chars.len());
            for (k, rch) in curr_chars.iter().enumerate().take(overlap) {
                let col = ((*outlinelen as isize)
                    .saturating_sub(smush as isize)
                    .saturating_add(k as isize))
                .max(0) as usize;
                if col < out.len() {
                    let lch = out[col];
                    if let Some(smushed) = smush_horizontal(
                        lch,
                        *rch,
                        mode,
                        font.hardblank,
                        old_prev_width,
                        curr_width,
                        right2left,
                    ) {
                        out[col] = smushed;
                    }
                }
            }
            if smush < curr_chars.len() {
                out.extend(&curr_chars[smush..]);
            }
            out.into_iter().collect()
        };

        if row_idx < output_rows.len() {
            output_rows[row_idx] = result;
        } else {
            output_rows.push(result);
        }
    }

    *outlinelen = output_rows[0].chars().count();
    true
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

    // --- add_char tests ---

    fn add_char_font() -> FIGfont {
        let mut chars = HashMap::new();
        chars.insert(0, FIGcharacter::from(vec!["#".to_string()]));
        chars.insert(72, FIGcharacter::from(vec![" H ".to_string()])); // H
        chars.insert(73, FIGcharacter::from(vec![" I ".to_string()])); // I
        chars.insert(105, FIGcharacter::from(vec![" i ".to_string()])); // i
        chars.insert(65, FIGcharacter::from(vec!["AA".to_string()])); // A
        chars.insert(66, FIGcharacter::from(vec!["BB".to_string()])); // B
        chars.insert(33, FIGcharacter::from(vec!["!!".to_string()])); // !
        FIGfont {
            chars,
            ..FIGfont::default()
        }
    }

    fn setup_add_char() -> (Vec<String>, usize, usize) {
        (vec!["".to_string()], 0, 0)
    }

    #[test]
    fn test_add_char_first_char_ltr() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::KERN);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        let ok = add_char(
            &font,
            72,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200,
        );
        assert!(ok);
        assert_eq!(outlinelen, 2);
        assert_eq!(output_rows[0], "H ");
        assert_eq!(prev_width, 3);
    }

    #[test]
    fn test_add_char_two_chars_kerning() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::KERN);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        assert!(add_char(
            &font,
            72,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert!(add_char(
            &font,
            105,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert_eq!(outlinelen, 3);
        assert_eq!(output_rows[0], "Hi ");
    }

    #[test]
    fn test_add_char_two_chars_smush() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::SMUSH | SmushMode::EQUAL_CHARS);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        assert!(add_char(
            &font,
            65,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert!(add_char(
            &font,
            65,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert_eq!(output_rows[0], "AAA");
    }

    #[test]
    fn test_add_char_rtl_smush() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::SMUSH | SmushMode::EQUAL_CHARS);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        assert!(add_char(
            &font,
            65,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            true,
            200
        ));
        assert!(add_char(
            &font,
            65,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            true,
            200
        ));
        assert_eq!(output_rows[0], "AAA");
    }

    #[test]
    fn test_add_char_limit_bail() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::KERN);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        assert!(add_char(
            &font,
            72,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            3
        ));
        assert_eq!(outlinelen, 2);
        assert_eq!(prev_width, 3);

        let ok = add_char(
            &font,
            73,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            3,
        );
        assert!(!ok);
    }

    #[test]
    fn test_add_char_restores_prev_width_on_fail() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::KERN);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        assert!(add_char(
            &font,
            72,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert_eq!(prev_width, 3);

        let prev_width_before = prev_width;
        let ok = add_char(
            &font,
            73,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            2,
        );
        assert!(!ok);
        assert_eq!(prev_width, prev_width_before);
    }

    #[test]
    fn test_add_char_single_word_c_output() {
        let font = add_char_font();
        let mode = SmushMode::new(SmushMode::KERN);
        let (mut output_rows, mut outlinelen, mut prev_width) = setup_add_char();

        // Build "Hi!"
        assert!(add_char(
            &font,
            72,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert!(add_char(
            &font,
            105,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert!(add_char(
            &font,
            33,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert_eq!(output_rows[0], "Hi!!");
    }

    #[test]
    fn test_add_char_multi_row() {
        let mut chars = HashMap::new();
        chars.insert(0, FIGcharacter::from(vec!["#".to_string(); 2]));
        chars.insert(
            65,
            FIGcharacter::from(vec![" A ".to_string(), " A ".to_string()]),
        );
        chars.insert(
            66,
            FIGcharacter::from(vec![" B ".to_string(), " B ".to_string()]),
        );
        let font = FIGfont {
            chars,
            charheight: 2,
            ..FIGfont::default()
        };
        let mode = SmushMode::new(SmushMode::SMUSH | SmushMode::EQUAL_CHARS);
        let mut output_rows = vec!["".to_string(), "".to_string()];
        let mut outlinelen = 0;
        let mut prev_width = 0;

        assert!(add_char(
            &font,
            65,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));
        assert!(add_char(
            &font,
            66,
            &mut output_rows,
            &mut outlinelen,
            &mut prev_width,
            mode,
            false,
            200
        ));

        assert_eq!(output_rows[0], "A B ");
        assert_eq!(output_rows[1], "A B ");
    }
}
