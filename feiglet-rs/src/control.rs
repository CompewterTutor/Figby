use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ControlCommand {
    pub thecommand: u8,
    pub rangelo: u32,
    pub rangehi: u32,
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlState {
    pub commands: Vec<ControlCommand>,
    pub multibyte: u32,
    pub gn: [u32; 4],
    pub gndbl: [bool; 4],
    pub gl: u8,
    pub gr: u8,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            multibyte: 0,
            gn: [0x00, 0x01, 0, 0],
            gndbl: [false; 4],
            gl: 0,
            gr: 1,
        }
    }
}

#[derive(Debug)]
pub enum ControlError {
    IoError(io::Error),
    ParseError(String),
}

impl From<io::Error> for ControlError {
    fn from(e: io::Error) -> Self {
        ControlError::IoError(e)
    }
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::IoError(e) => write!(f, "I/O error: {}", e),
            ControlError::ParseError(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for ControlError {}

struct ByteReader<R: Read> {
    reader: BufReader<R>,
    pushback: Vec<u8>,
}

impl<R: Read> ByteReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            pushback: Vec::new(),
        }
    }

    fn next(&mut self) -> io::Result<Option<u8>> {
        if let Some(b) = self.pushback.pop() {
            return Ok(Some(b));
        }
        let mut buf = [0u8; 1];
        match self.reader.read(&mut buf) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buf[0])),
            Err(e) => Err(e),
        }
    }

    fn unget(&mut self, b: u8) {
        self.pushback.push(b);
    }
}

fn skip_ws<R: Read>(reader: &mut ByteReader<R>) -> io::Result<()> {
    loop {
        match reader.next()? {
            Some(b) if b.is_ascii_whitespace() => continue,
            Some(b) => {
                reader.unget(b);
                return Ok(());
            }
            None => return Ok(()),
        }
    }
}

fn skip_to_eol<R: Read>(reader: &mut ByteReader<R>) -> io::Result<()> {
    loop {
        match reader.next()? {
            Some(b'\n') => return Ok(()),
            Some(b'\r') => {
                match reader.next()? {
                    Some(b'\n') => {}
                    Some(b) => reader.unget(b),
                    None => {}
                }
                return Ok(());
            }
            Some(_) => continue,
            None => return Ok(()),
        }
    }
}

fn read_num<R: Read>(reader: &mut ByteReader<R>) -> io::Result<i64> {
    skip_ws(reader)?;
    let mut sign = 1i64;
    match reader.next()? {
        Some(b'-') => sign = -1,
        Some(b) => reader.unget(b),
        None => return Ok(0),
    }

    let mut acc: u32 = 0;
    let hex_digits = b"0123456789ABCDEF";

    match reader.next()? {
        Some(b'0') => match reader.next()? {
            Some(b'x' | b'X') => loop {
                match reader.next()? {
                    Some(b @ b'0'..=b'9') => {
                        acc = acc * 16 + (b - b'0') as u32;
                    }
                    Some(b @ b'a'..=b'f') => {
                        acc = acc * 16 + (b - b'a' + 10) as u32;
                    }
                    Some(b @ b'A'..=b'F') => {
                        acc = acc * 16 + (b - b'A' + 10) as u32;
                    }
                    Some(b) => {
                        reader.unget(b);
                        break;
                    }
                    None => break,
                }
            },
            Some(b) => {
                reader.unget(b);
                loop {
                    match reader.next()? {
                        Some(b @ b'0'..=b'7') => {
                            acc = acc * 8 + (b - b'0') as u32;
                        }
                        Some(b) => {
                            reader.unget(b);
                            break;
                        }
                        None => break,
                    }
                }
            }
            None => {}
        },
        Some(b) => {
            reader.unget(b);
            while let Some(c) = reader.next()? {
                let c_upper = c.to_ascii_uppercase();
                let pos = hex_digits.iter().position(|&d| d == c_upper);
                match pos {
                    Some(d) => acc = acc * 10 + d as u32,
                    None => {
                        reader.unget(c);
                        break;
                    }
                }
            }
        }
        None => {}
    }

    Ok((acc as i64) * sign)
}

fn read_tchar<R: Read>(reader: &mut ByteReader<R>) -> io::Result<i64> {
    match reader.next()? {
        None => Ok(0),
        Some(b @ (b'\n' | b'\r')) => {
            reader.unget(b);
            Ok(0)
        }
        Some(b'\\') => match reader.next()? {
            None => Ok(0),
            Some(b'a') => Ok(7),
            Some(b'b') => Ok(8),
            Some(b'e') => Ok(27),
            Some(b'f') => Ok(12),
            Some(b'n') => Ok(10),
            Some(b'r') => Ok(13),
            Some(b't') => Ok(9),
            Some(b'v') => Ok(11),
            Some(next) if next == b'-' || next == b'x' || next.is_ascii_digit() => {
                reader.unget(next);
                read_num(reader)
            }
            Some(next) => Ok(next as i64),
        },
        Some(b) => Ok(b as i64),
    }
}

fn charset_name<R: Read>(reader: &mut ByteReader<R>) -> io::Result<i64> {
    let result = read_tchar(reader)?;
    if result == '\n' as i64 || result == '\r' as i64 {
        Ok(0)
    } else {
        Ok(result)
    }
}

fn charset_define<R: Read>(
    n: usize,
    reader: &mut ByteReader<R>,
    state: &mut ControlState,
) -> io::Result<()> {
    skip_ws(reader)?;
    match reader.next()? {
        Some(b'9') => {}
        Some(b) => {
            reader.unget(b);
            skip_to_eol(reader)?;
            return Ok(());
        }
        None => return Ok(()),
    }

    let ch = match reader.next()? {
        Some(b) => b,
        None => return Ok(()),
    };

    if ch == b'6' {
        let cn = charset_name(reader)?;
        state.gn[n] = (65536u64 * cn as u64) as u32 + 0x80;
        state.gndbl[n] = false;
        skip_to_eol(reader)?;
        return Ok(());
    }

    if ch != b'4' {
        skip_to_eol(reader)?;
        return Ok(());
    }

    let ch = match reader.next()? {
        Some(b) => b,
        None => return Ok(()),
    };

    if ch == b'x' {
        match reader.next()? {
            Some(b'9') => {}
            Some(b) => {
                reader.unget(b);
                return Ok(());
            }
            None => return Ok(()),
        }
        match reader.next()? {
            Some(b'4') => {}
            Some(b) => {
                reader.unget(b);
                return Ok(());
            }
            None => return Ok(()),
        }
        skip_ws(reader)?;
        let cn = charset_name(reader)?;
        state.gn[n] = (65536u64 * cn as u64) as u32;
        state.gndbl[n] = true;
        skip_to_eol(reader)?;
        return Ok(());
    }

    reader.unget(ch);
    skip_ws(reader)?;
    let cn = charset_name(reader)?;
    state.gn[n] = (65536u64 * cn as u64) as u32;
    state.gndbl[n] = false;
    Ok(())
}

pub fn read_control<P: AsRef<Path>>(path: P, state: &mut ControlState) -> Result<(), ControlError> {
    let file = File::open(path.as_ref())?;
    let mut reader = ByteReader::new(BufReader::new(file));

    loop {
        let command = match reader.next()? {
            None => break,
            Some(b) => b,
        };

        match command {
            b't' => {
                skip_ws(&mut reader)?;
                let firstch = read_tchar(&mut reader)?;
                let dashcheck = match reader.next()? {
                    Some(b) => b,
                    None => break,
                };
                let lastch = if dashcheck == b'-' {
                    read_tchar(&mut reader)?
                } else {
                    reader.unget(dashcheck);
                    firstch
                };
                skip_ws(&mut reader)?;
                let target = read_tchar(&mut reader)?;
                let offset = target - firstch;
                skip_to_eol(&mut reader)?;
                state.commands.push(ControlCommand {
                    thecommand: 1,
                    rangelo: firstch as u32,
                    rangehi: lastch as u32,
                    offset,
                });
            }
            b'0'..=b'9' | b'-' => {
                reader.unget(command);
                let firstch = read_num(&mut reader)?;
                skip_ws(&mut reader)?;
                let lastch = read_num(&mut reader)?;
                let offset = lastch - firstch;
                skip_to_eol(&mut reader)?;
                state.commands.push(ControlCommand {
                    thecommand: 1,
                    rangelo: firstch as u32,
                    rangehi: firstch as u32,
                    offset,
                });
            }
            b'f' => {
                skip_to_eol(&mut reader)?;
                state.commands.push(ControlCommand {
                    thecommand: 0,
                    rangelo: 0,
                    rangehi: 0,
                    offset: 0,
                });
            }
            b'b' => {
                state.multibyte = 1;
            }
            b'u' => {
                state.multibyte = 2;
            }
            b'h' => {
                state.multibyte = 3;
            }
            b'j' => {
                state.multibyte = 4;
            }
            b'g' => {
                state.multibyte = 0;
                skip_ws(&mut reader)?;
                let sub = match reader.next()? {
                    Some(b) => b,
                    None => continue,
                };
                match sub {
                    b'0' => charset_define(0, &mut reader, state)?,
                    b'1' => charset_define(1, &mut reader, state)?,
                    b'2' => charset_define(2, &mut reader, state)?,
                    b'3' => charset_define(3, &mut reader, state)?,
                    b'l' | b'L' => {
                        skip_ws(&mut reader)?;
                        if let Some(d) = reader.next()? {
                            state.gl = d - b'0';
                        }
                        skip_to_eol(&mut reader)?;
                    }
                    b'r' | b'R' => {
                        skip_ws(&mut reader)?;
                        if let Some(d) = reader.next()? {
                            state.gr = d - b'0';
                        }
                        skip_to_eol(&mut reader)?;
                    }
                    _ => {
                        skip_to_eol(&mut reader)?;
                    }
                }
            }
            b'\r' | b'\n' => {}
            _ => {
                skip_to_eol(&mut reader)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    const FONTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fonts/");

    struct TempFile {
        _dir: tempfile::TempDir,
        path: std::path::PathBuf,
    }

    impl AsRef<Path> for TempFile {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }

    fn write_temp_file(content: &str) -> TempFile {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("test.flc");
        let mut file = std::fs::File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        TempFile {
            _dir: dir,
            path: file_path,
        }
    }

    fn test_state() -> ControlState {
        ControlState::default()
    }

    #[test]
    fn test_state_defaults() {
        let state = ControlState::default();
        assert!(state.commands.is_empty());
        assert_eq!(state.multibyte, 0);
        assert_eq!(state.gn, [0x00, 0x01, 0, 0]);
        assert_eq!(state.gndbl, [false; 4]);
        assert_eq!(state.gl, 0);
        assert_eq!(state.gr, 1);
    }

    #[test]
    fn test_read_empty_file() {
        let file = write_temp_file("");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_magic_header_freeze() {
        // C's readcontrol treats 'f' in "flc2a" as a freeze command
        let file = write_temp_file("flc2a\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        assert_eq!(state.commands[0].thecommand, 0);
    }

    #[test]
    fn test_read_magic_header_freeze_no_newline() {
        let file = write_temp_file("flc2a");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        assert_eq!(state.commands[0].thecommand, 0);
    }

    #[test]
    fn test_read_translate_single() {
        let file = write_temp_file("t a b\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 1);
        assert_eq!(cmd.rangelo, b'a' as u32);
        assert_eq!(cmd.rangehi, b'a' as u32);
        assert_eq!(cmd.offset, 1);
    }

    #[test]
    fn test_read_translate_range() {
        let file = write_temp_file("t a-z A-Z\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 1);
        assert_eq!(cmd.rangelo, b'a' as u32);
        assert_eq!(cmd.rangehi, b'z' as u32);
        assert_eq!(cmd.offset, -32i64);
    }

    #[test]
    fn test_read_translate_negative_offset() {
        let file = write_temp_file("t = \\-6\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 1);
        assert_eq!(cmd.rangelo, b'=' as u32);
        assert_eq!(cmd.rangehi, b'=' as u32);
        assert_eq!(cmd.offset, -6 - b'=' as i64);
    }

    #[test]
    fn test_read_freeze() {
        let file = write_temp_file("f\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 0);
    }

    #[test]
    fn test_read_backslash_line_skipped() {
        let file = write_temp_file("\\0x037A \\0x0399\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        // starts with \, not 0-9 or -, so skipped by default case
        assert_eq!(state.commands.len(), 0);
    }

    #[test]
    fn test_read_mapping_table_decimal() {
        let file = write_temp_file("65 90\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 1);
        assert_eq!(cmd.rangelo, 65);
        assert_eq!(cmd.rangehi, 65);
        assert_eq!(cmd.offset, 25);
    }

    #[test]
    fn test_read_mapping_table_negative() {
        let file = write_temp_file("65 \\-1\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        // starts with '6', so unget, readnum gets 65, skipws, readnum reads \ -> ?
        // Actually, after skipws we read \, but readnum doesn't handle backslash
        // readnum's first char after skip_ws is \, which is not -, not 0, so base=10, unget
        // Then readnum reads \, which is not in hex_digits, so unget and return 0
        // So firstch=65, lastch=0, offset=-65
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.rangelo, 65);
        assert_eq!(cmd.rangehi, 65);
    }

    #[test]
    fn test_read_comment() {
        let file = write_temp_file("# comment line\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_blank_line() {
        let file = write_temp_file("\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_blank_line_crlf() {
        let file = write_temp_file("\r\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_multibyte_commands() {
        let mut state;

        state = test_state();
        read_control(write_temp_file("b\n"), &mut state).unwrap();
        assert_eq!(state.multibyte, 1);

        state = test_state();
        read_control(write_temp_file("u\n"), &mut state).unwrap();
        assert_eq!(state.multibyte, 2);

        state = test_state();
        read_control(write_temp_file("h\n"), &mut state).unwrap();
        assert_eq!(state.multibyte, 3);

        state = test_state();
        read_control(write_temp_file("j\n"), &mut state).unwrap();
        assert_eq!(state.multibyte, 4);
    }

    #[test]
    fn test_read_utf8_control_file() {
        let path = [FONTS_DIR, "utf8.flc"].concat();
        let mut state = test_state();
        read_control(&path, &mut state).unwrap();
        assert_eq!(state.multibyte, 2);
        // 'f' from "flc2a" adds 1 freeze command
        assert_eq!(state.commands.len(), 1);
    }

    #[test]
    fn test_read_hz_control_file() {
        let path = [FONTS_DIR, "hz.flc"].concat();
        let mut state = test_state();
        read_control(&path, &mut state).unwrap();
        assert_eq!(state.multibyte, 3);
        assert_eq!(state.commands.len(), 1);
    }

    #[test]
    fn test_read_charset_g0_94() {
        let file = write_temp_file("g094 J\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.gn[0], 65536 * b'J' as u32);
        assert!(!state.gndbl[0]);
    }

    #[test]
    fn test_read_charset_g1_96() {
        // C readcontrol has NO skipws before charsetname in the 96 path
        // (matching C bug: readTchar reads the space = 32 instead of 'J')
        let file = write_temp_file("g196 J\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.gn[1], 65536 * b' ' as u32 + 0x80);
        assert!(!state.gndbl[1]);
    }

    #[test]
    fn test_read_charset_g2_94x_double() {
        let file = write_temp_file("g294x94 J\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.gn[2], 65536 * b'J' as u32);
        assert!(state.gndbl[2]);
    }

    #[test]
    fn test_read_gl_gr_selection() {
        let file = write_temp_file("g l 2\ng r 0\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.gl, 2);
        assert_eq!(state.gr, 0);
    }

    #[test]
    fn test_read_unknown_command_skipped() {
        let file = write_temp_file("x\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_escape_sequences() {
        let pairs = [
            (b'a', 7u8),
            (b'b', 8),
            (b'e', 27),
            (b'f', 12),
            (b'n', 10),
            (b'r', 13),
            (b't', 9),
            (b'v', 11),
        ];
        for &(esc, expected) in &pairs {
            let content = format!("t \\{} a\n", esc as char);
            let file = write_temp_file(&content);
            let mut state = test_state();
            read_control(&file, &mut state).unwrap();
            assert_eq!(state.commands.len(), 1, "escape \\{}", esc as char);
            let cmd = &state.commands[0];
            assert_eq!(cmd.rangelo, expected as u32, "escape \\{}", esc as char);
            assert_eq!(cmd.rangehi, expected as u32, "escape \\{}", esc as char);
        }
    }

    #[test]
    fn test_read_octal_escape() {
        // C readnum treats \377 as decimal 377, not octal 255
        let file = write_temp_file("t \\377 a\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.rangelo, 377);
    }

    #[test]
    fn test_read_hex_escape() {
        let file = write_temp_file("t \\0x1B a\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.rangelo, 27);
    }

    #[test]
    fn test_read_error_nonexistent_file() {
        let mut state = test_state();
        let result = read_control("/nonexistent/path.flc", &mut state);
        assert!(result.is_err());
        match result.unwrap_err() {
            ControlError::IoError(_) => {}
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_read_real_world_fixtures() {
        let fixtures = vec![
            [FONTS_DIR, "moscow.flc"].concat(),
            [FONTS_DIR, "koi8r.flc"].concat(),
            [FONTS_DIR, "646-cn.flc"].concat(),
        ];
        for fixture in &fixtures {
            let mut state = test_state();
            read_control(fixture, &mut state).unwrap();
            assert!(
                !state.commands.is_empty(),
                "{} should have at least one command",
                fixture
            );
        }
    }

    #[test]
    fn test_read_upper_flc_snapshot() {
        let mut state = test_state();
        read_control([FONTS_DIR, "upper.flc"].concat(), &mut state).unwrap();
        // upper.flc has 71 't' command lines + 1 freeze from 'f' in "flc2a"
        assert_eq!(state.commands.len(), 72);
    }

    #[test]
    fn test_read_8859_5_flc() {
        let mut state = test_state();
        read_control([FONTS_DIR, "8859-5.flc"].concat(), &mut state).unwrap();
        assert!(!state.commands.is_empty());
        let cmd = &state.commands[0];
        assert_eq!(cmd.thecommand, 1);
        // Lines in 8859-5.flc start with hex like "0x00\t0x0000"
        // These are mapping table entries: firstch reads 0 (then x00 = 0)
        // Actually the lines have tabs, but the first char is '0' (from 0x00)
        // Our parser handles this
    }

    #[test]
    fn test_read_jis0201_flc() {
        let mut state = test_state();
        read_control([FONTS_DIR, "jis0201.flc"].concat(), &mut state).unwrap();
        assert!(!state.commands.is_empty());
        assert_eq!(state.gl, 0);
        assert_eq!(state.gr, 1);
    }

    #[test]
    fn test_read_frango_flc() {
        let mut state = test_state();
        read_control([FONTS_DIR, "frango.flc"].concat(), &mut state).unwrap();
        assert!(!state.commands.is_empty());
    }

    #[test]
    fn test_freeze_then_translate() {
        let file = write_temp_file("f\nt a b\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 2);
        assert_eq!(state.commands[0].thecommand, 0);
        assert_eq!(state.commands[1].thecommand, 1);
    }

    #[test]
    fn test_multiple_translate_commands() {
        let file = write_temp_file("t a b\nt c d\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 2);
        assert_eq!(state.commands[0].offset, 1);
        assert_eq!(state.commands[1].offset, 1);
    }

    #[test]
    fn test_mapping_table_with_same_value() {
        let file = write_temp_file("65 65\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        assert_eq!(state.commands[0].offset, 0);
        assert_eq!(state.commands[0].rangelo, 65);
    }

    #[test]
    fn test_comment_ignored() {
        let file = write_temp_file("# This is a comment\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_only_whitespace_lines() {
        let file = write_temp_file("  \n\t\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert!(state.commands.is_empty());
    }

    #[test]
    fn test_read_jis0201_flc_g_commands() {
        let mut state = test_state();
        read_control([FONTS_DIR, "jis0201.flc"].concat(), &mut state).unwrap();
        assert_eq!(state.gn[0], 65536 * b'J' as u32);
        assert_eq!(state.gn[1], 65536 * b'I' as u32);
        assert!(!state.gndbl[0]);
        assert!(!state.gndbl[1]);
    }

    #[test]
    fn test_read_mapping_table_hex_entry() {
        let file = write_temp_file("0x4A0020 0x20\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        let cmd = &state.commands[0];
        assert_eq!(cmd.rangelo, 0x4A0020);
        assert_eq!(cmd.rangehi, 0x4A0020);
        assert_eq!(cmd.offset, 0x20i64 - 0x4A0020i64);
    }

    #[test]
    fn test_translate_with_carriage_return() {
        let file = write_temp_file("t a b\r\n");
        let mut state = test_state();
        read_control(&file, &mut state).unwrap();
        assert_eq!(state.commands.len(), 1);
        assert_eq!(state.commands[0].rangelo, b'a' as u32);
        assert_eq!(state.commands[0].offset, 1);
    }
}
