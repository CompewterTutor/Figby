// Smushing rules engine

/// Bitmask-based smushing mode selector.
///
/// Encodes horizontal rules (lower 6 bits), horizontal mode (SM_KERN/SM_SMUSH),
/// vertical rules (bits 8-12), and vertical mode (V_FIT/V_SMUSH).
/// Matches FIGfont `full_layout` field encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmushMode(u32);

impl SmushMode {
    // Horizontal rule bits (lower 6 bits, values 1/2/4/8/16/32)
    pub const EQUAL_CHARS: u32 = 1; // H1
    pub const UNDERSCORE: u32 = 2; // H2
    pub const HIERARCHY: u32 = 4; // H3
    pub const PAIR: u32 = 8; // H4
    pub const BIGX: u32 = 16; // H5
    pub const HARDBLANK: u32 = 32; // H6
                                   // Horizontal mode bits
    pub const KERN: u32 = 64;
    pub const SMUSH: u32 = 128;
    // Vertical rule bits (bits 8-12, values 256/512/1024/2048/4096)
    pub const V_EQUAL: u32 = 256; // V1
    pub const V_UNDERSCORE: u32 = 512; // V2
    pub const V_HIERARCHY: u32 = 1024; // V3
    pub const V_LINE: u32 = 2048; // V4
    pub const V_SUPERSMUSH: u32 = 4096; // V5
                                        // Vertical mode bits
    pub const V_FIT: u32 = 8192;
    pub const V_SMUSH: u32 = 16384;

    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn contains(self, other: u32) -> bool {
        self.0 & other == other
    }

    pub fn horizontal_rules(self) -> u32 {
        self.0 & 63
    }

    pub fn vertical_rules(self) -> u32 {
        (self.0 >> 8) & 31
    }

    pub fn is_smush(self) -> bool {
        self.0 & Self::SMUSH != 0
    }

    pub fn is_vertical_smush(self) -> bool {
        self.0 & Self::V_SMUSH != 0
    }
}

impl From<u32> for SmushMode {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

/// Characters eligible for hierarchy smushing.
/// Classes ordered from lowest to highest:
///   0: `|`
///   1: `/\`
///   2: `[]`
///   3: `{}`
///   4: `()`
///   5: `<>`
fn hierarchy_class(ch: char) -> Option<usize> {
    match ch {
        '|' => Some(0),
        '/' | '\\' => Some(1),
        '[' | ']' => Some(2),
        '{' | '}' => Some(3),
        '(' | ')' => Some(4),
        '<' | '>' => Some(5),
        _ => None,
    }
}

fn is_hierarchy_char(ch: char) -> bool {
    matches!(
        ch,
        '|' | '/' | '\\' | '[' | ']' | '{' | '}' | '(' | ')' | '<' | '>'
    )
}

/// Attempt horizontal smushing of two adjacent characters.
///
/// Mirror of C `smushem()` in figlet.c:1358-1434.
/// Returns `Some(char)` on smush, `None` on no smush.
pub fn smush_horizontal(
    lch: char,
    rch: char,
    mode: SmushMode,
    hardblank: char,
    right2left: bool,
) -> Option<char> {
    if lch == ' ' {
        return Some(rch);
    }
    if rch == ' ' {
        return Some(lch);
    }

    if !mode.is_smush() {
        return None;
    }

    if mode.horizontal_rules() == 0 {
        if lch == hardblank {
            return Some(rch);
        }
        if rch == hardblank {
            return Some(lch);
        }
        if right2left {
            return Some(lch);
        }
        return Some(rch);
    }

    if mode.contains(SmushMode::HARDBLANK) && lch == hardblank && rch == hardblank {
        return Some(lch);
    }

    if lch == hardblank || rch == hardblank {
        return None;
    }

    if mode.contains(SmushMode::EQUAL_CHARS) && lch == rch {
        return Some(lch);
    }

    if mode.contains(SmushMode::UNDERSCORE) {
        if lch == '_' && is_hierarchy_char(rch) {
            return Some(rch);
        }
        if rch == '_' && is_hierarchy_char(lch) {
            return Some(lch);
        }
    }

    if mode.contains(SmushMode::HIERARCHY) {
        if let (Some(lc), Some(rc)) = (hierarchy_class(lch), hierarchy_class(rch)) {
            if lc < rc {
                return Some(rch);
            }
            if rc < lc {
                return Some(lch);
            }
        }
    }

    if mode.contains(SmushMode::PAIR) {
        match (lch, rch) {
            ('[', ']') | (']', '[') | ('{', '}') | ('}', '{') | ('(', ')') | (')', '(') => {
                return Some('|');
            }
            _ => {}
        }
    }

    if mode.contains(SmushMode::BIGX) {
        match (lch, rch) {
            ('/', '\\') => return Some('|'),
            ('\\', '/') => return Some('Y'),
            ('>', '<') => return Some('X'),
            _ => {}
        }
    }

    None
}

/// Attempt vertical smushing of two vertically-stacked characters.
///
/// Per FIGfont spec, hardblank acts as space for vertical operations.
/// V1-V5 match TOIlet vertical smushing rules.
pub fn smush_vertical(top: char, bottom: char, mode: SmushMode, hardblank: char) -> Option<char> {
    if top == ' ' || top == hardblank {
        return Some(bottom);
    }
    if bottom == ' ' || bottom == hardblank {
        return Some(top);
    }

    if !mode.is_vertical_smush() {
        return None;
    }

    if mode.vertical_rules() == 0 {
        return Some(bottom);
    }

    if mode.contains(SmushMode::V_EQUAL) && top == bottom {
        return Some(top);
    }

    if mode.contains(SmushMode::V_UNDERSCORE) {
        if top == '_' && is_hierarchy_char(bottom) {
            return Some(bottom);
        }
        if bottom == '_' && is_hierarchy_char(top) {
            return Some(top);
        }
    }

    if mode.contains(SmushMode::V_HIERARCHY) {
        if let (Some(tc), Some(bc)) = (hierarchy_class(top), hierarchy_class(bottom)) {
            if tc < bc {
                return Some(bottom);
            }
            if bc < tc {
                return Some(top);
            }
        }
    }

    if mode.contains(SmushMode::V_LINE) {
        match (top, bottom) {
            ('-', '_') | ('_', '-') => return Some('='),
            _ => {}
        }
    }

    if mode.contains(SmushMode::V_SUPERSMUSH) && top == '|' && bottom == '|' {
        return Some('|');
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const HB: char = '@';
    const MODE_SMUSH: u32 = SmushMode::SMUSH;

    // --- H1: Equal chars ---
    #[test]
    fn test_h1_equal_smush() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal('A', 'A', mode, HB, false), Some('A'));
        assert_eq!(smush_horizontal('X', 'X', mode, HB, false), Some('X'));
    }

    // --- H2: Underscore smushing ---
    #[test]
    fn test_h2_underscore_left() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::UNDERSCORE);
        assert_eq!(smush_horizontal('_', '/', mode, HB, false), Some('/'));
        assert_eq!(smush_horizontal('_', '|', mode, HB, false), Some('|'));
        assert_eq!(smush_horizontal('_', '<', mode, HB, false), Some('<'));
    }

    #[test]
    fn test_h2_underscore_right() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::UNDERSCORE);
        assert_eq!(smush_horizontal('/', '_', mode, HB, false), Some('/'));
        assert_eq!(smush_horizontal('|', '_', mode, HB, false), Some('|'));
        assert_eq!(smush_horizontal('>', '_', mode, HB, false), Some('>'));
    }

    // --- H3: Hierarchy ---
    #[test]
    fn test_h3_hierarchy_forward() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::HIERARCHY);
        assert_eq!(smush_horizontal('|', '/', mode, HB, false), Some('/'));
        assert_eq!(smush_horizontal('/', '[', mode, HB, false), Some('['));
        assert_eq!(smush_horizontal('[', '{', mode, HB, false), Some('{'));
        assert_eq!(smush_horizontal('{', '(', mode, HB, false), Some('('));
        assert_eq!(smush_horizontal('(', '<', mode, HB, false), Some('<'));
    }

    #[test]
    fn test_h3_hierarchy_reverse() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::HIERARCHY);
        assert_eq!(smush_horizontal('/', '|', mode, HB, false), Some('/'));
        assert_eq!(smush_horizontal('[', '/', mode, HB, false), Some('['));
        assert_eq!(smush_horizontal('{', '[', mode, HB, false), Some('{'));
        assert_eq!(smush_horizontal('(', '{', mode, HB, false), Some('('));
        assert_eq!(smush_horizontal('<', '(', mode, HB, false), Some('<'));
    }

    // --- H4: Pair smushing ---
    #[test]
    fn test_h4_pair_brackets() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::PAIR);
        assert_eq!(smush_horizontal('[', ']', mode, HB, false), Some('|'));
        assert_eq!(smush_horizontal(']', '[', mode, HB, false), Some('|'));
    }

    #[test]
    fn test_h4_pair_braces() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::PAIR);
        assert_eq!(smush_horizontal('{', '}', mode, HB, false), Some('|'));
        assert_eq!(smush_horizontal('}', '{', mode, HB, false), Some('|'));
    }

    #[test]
    fn test_h4_pair_parens() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::PAIR);
        assert_eq!(smush_horizontal('(', ')', mode, HB, false), Some('|'));
        assert_eq!(smush_horizontal(')', '(', mode, HB, false), Some('|'));
    }

    // --- H5: Big X ---
    #[test]
    fn test_h5_bigx_fwd() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::BIGX);
        assert_eq!(smush_horizontal('/', '\\', mode, HB, false), Some('|'));
    }

    #[test]
    fn test_h5_bigx_rev() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::BIGX);
        assert_eq!(smush_horizontal('\\', '/', mode, HB, false), Some('Y'));
    }

    #[test]
    fn test_h5_bigx_greater_less() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::BIGX);
        assert_eq!(smush_horizontal('>', '<', mode, HB, false), Some('X'));
    }

    #[test]
    fn test_h5_bigx_no_reverse() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::BIGX);
        assert_eq!(smush_horizontal('<', '>', mode, HB, false), None);
    }

    // --- H6: Hardblank ---
    #[test]
    fn test_h6_hardblank_pair() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::HARDBLANK);
        assert_eq!(smush_horizontal(HB, HB, mode, HB, false), Some(HB));
    }

    // --- Hardblank guard ---
    #[test]
    fn test_hardblank_visible_none() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::HARDBLANK | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal(HB, 'A', mode, HB, false), None);
        assert_eq!(smush_horizontal('A', HB, mode, HB, false), None);
    }

    // --- Blank smushing ---
    #[test]
    fn test_blank_smush_left() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal(' ', 'A', mode, HB, false), Some('A'));
        assert_eq!(smush_horizontal(' ', HB, mode, HB, false), Some(HB));
    }

    #[test]
    fn test_blank_smush_right() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal('A', ' ', mode, HB, false), Some('A'));
        assert_eq!(smush_horizontal(HB, ' ', mode, HB, false), Some(HB));
    }

    // --- Width guard moved to add_char ---
    #[test]
    fn test_width_guard() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::EQUAL_CHARS);
        // Width guard is no longer in smush_horizontal — width ≥ 2
        // constraint is handled in add_char
        assert_eq!(smush_horizontal('A', 'A', mode, HB, false), Some('A'));
    }

    // --- Kerning mode ---
    #[test]
    fn test_kerning_mode() {
        let mode = SmushMode::new(SmushMode::KERN | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal('A', 'A', mode, HB, false), None);
    }

    // --- Universal overlapping ---
    #[test]
    fn test_universal_overlap() {
        let mode = SmushMode::new(MODE_SMUSH);
        assert_eq!(smush_horizontal('A', 'B', mode, HB, false), Some('B'));
    }

    #[test]
    fn test_universal_hardblank() {
        let mode = SmushMode::new(MODE_SMUSH);
        assert_eq!(smush_horizontal(HB, 'A', mode, HB, false), Some('A'));
        assert_eq!(smush_horizontal('A', HB, mode, HB, false), Some('A'));
    }

    #[test]
    fn test_universal_right2left() {
        let mode = SmushMode::new(MODE_SMUSH);
        assert_eq!(smush_horizontal('A', 'B', mode, HB, true), Some('A'));
    }

    // --- No smush when no rule matches ---
    #[test]
    fn test_no_rule_match_returns_none() {
        let mode = SmushMode::new(MODE_SMUSH | SmushMode::EQUAL_CHARS);
        assert_eq!(smush_horizontal('A', 'B', mode, HB, false), None);
    }

    // --- Vertical V1: Equal chars ---
    #[test]
    fn test_v1_equal() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_EQUAL);
        assert_eq!(smush_vertical('A', 'A', mode, HB), Some('A'));
        assert_eq!(smush_vertical('X', 'X', mode, HB), Some('X'));
    }

    #[test]
    fn test_v1_not_equal() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_EQUAL);
        assert_eq!(smush_vertical('A', 'B', mode, HB), None);
    }

    // --- Vertical V2: Underscore ---
    #[test]
    fn test_v2_underscore() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_UNDERSCORE);
        assert_eq!(smush_vertical('_', '/', mode, HB), Some('/'));
        assert_eq!(smush_vertical('/', '_', mode, HB), Some('/'));
        assert_eq!(smush_vertical('_', '|', mode, HB), Some('|'));
    }

    // --- Vertical V3: Hierarchy ---
    #[test]
    fn test_v3_hierarchy() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_HIERARCHY);
        assert_eq!(smush_vertical('|', '/', mode, HB), Some('/'));
        assert_eq!(smush_vertical('/', '[', mode, HB), Some('['));
        assert_eq!(smush_vertical('[', '|', mode, HB), Some('['));
    }

    // --- Vertical V4: Line ---
    #[test]
    fn test_v4_line() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_LINE);
        assert_eq!(smush_vertical('-', '_', mode, HB), Some('='));
        assert_eq!(smush_vertical('_', '-', mode, HB), Some('='));
    }

    #[test]
    fn test_v4_line_no_match() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_LINE);
        assert_eq!(smush_vertical('-', '-', mode, HB), None);
        assert_eq!(smush_vertical('_', '_', mode, HB), None);
    }

    // --- Vertical V5: Supersmush ---
    #[test]
    fn test_v5_supersmush() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_SUPERSMUSH);
        assert_eq!(smush_vertical('|', '|', mode, HB), Some('|'));
    }

    // --- Vertical blank handling ---
    #[test]
    fn test_vertical_blank() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_EQUAL);
        assert_eq!(smush_vertical(' ', 'A', mode, HB), Some('A'));
        assert_eq!(smush_vertical('A', ' ', mode, HB), Some('A'));
    }

    #[test]
    fn test_vertical_hardblank_as_blank() {
        let mode = SmushMode::new(SmushMode::V_SMUSH | SmushMode::V_EQUAL);
        assert_eq!(smush_vertical(HB, 'A', mode, HB), Some('A'));
        assert_eq!(smush_vertical('A', HB, mode, HB), Some('A'));
    }

    // --- Vertical universal ---
    #[test]
    fn test_vertical_universal() {
        let mode = SmushMode::new(SmushMode::V_SMUSH);
        assert_eq!(smush_vertical('A', 'B', mode, HB), Some('B'));
    }

    // --- Vertical no V_SMUSH mode ---
    #[test]
    fn test_vertical_kerning() {
        let mode = SmushMode::new(SmushMode::V_EQUAL);
        assert_eq!(smush_vertical('A', 'A', mode, HB), None);
    }

    // --- Mode type tests ---
    #[test]
    fn test_smush_mode_constants() {
        let mode = SmushMode::new(0);
        assert!(!mode.is_smush());
        assert!(!mode.is_vertical_smush());
        assert_eq!(mode.horizontal_rules(), 0);
        assert_eq!(mode.vertical_rules(), 0);

        let mode = SmushMode::new(
            SmushMode::SMUSH | SmushMode::EQUAL_CHARS | SmushMode::V_SMUSH | SmushMode::V_EQUAL,
        );
        assert!(mode.is_smush());
        assert!(mode.is_vertical_smush());
        assert!(mode.contains(SmushMode::EQUAL_CHARS));
        assert!(mode.contains(SmushMode::V_EQUAL));
    }
}
