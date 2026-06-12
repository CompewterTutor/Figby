use clap::Parser;
use figby::control::{self, CharReader};
use figby::font::{self, FIGfont};
use figby::input;
use figby::render::{add_char, lookup_char, render_line, split_line, Justification};
use figby::smush::SmushMode;
use std::io::{self, Read, Write};
use std::process;

const VERSION_INT: i32 = 20205;
const VERSION: &str = "2.2.5";
const DATE: &str = "31 May 2012";
const FONTFILE_MAGIC: &str = "flf2";
const TOILETFILE_MAGIC: &str = "tlf2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmushOverride {
    No = 0,
    Yes = 1,
    Force = 2,
}

enum InputIter {
    Stdin {
        bytes: io::Bytes<io::BufReader<io::Stdin>>,
        buf: Option<u32>,
    },
    Args {
        args: Vec<String>,
        arg_idx: usize,
        char_idx: usize,
        exhausted: bool,
        buf: Option<u32>,
    },
}

impl InputIter {
    fn new(message: Vec<String>, cmdinput: bool) -> Self {
        if cmdinput {
            let exhausted = message.is_empty();
            Self::Args {
                args: message,
                arg_idx: 0,
                char_idx: 0,
                exhausted,
                buf: None,
            }
        } else {
            Self::Stdin {
                bytes: io::BufReader::new(io::stdin()).bytes(),
                buf: None,
            }
        }
    }

    fn next(&mut self) -> Option<u32> {
        match self {
            Self::Stdin { bytes, buf } => {
                if let Some(c) = buf.take() {
                    return Some(c);
                }
                match bytes.next() {
                    Some(Ok(b)) => Some(b as u32),
                    _ => None,
                }
            }
            Self::Args {
                args,
                arg_idx,
                char_idx,
                exhausted,
                buf,
            } => {
                if let Some(c) = buf.take() {
                    return Some(c);
                }
                if *exhausted {
                    return None;
                }
                if *arg_idx >= args.len() {
                    *exhausted = true;
                    return None;
                }
                let arg_bytes = args[*arg_idx].as_bytes();
                if *char_idx < arg_bytes.len() {
                    let c = arg_bytes[*char_idx] as u32;
                    *char_idx += 1;
                    return Some(c);
                }
                let is_empty = *char_idx == 0;
                let is_last = *arg_idx + 1 >= args.len();
                *arg_idx += 1;
                *char_idx = 0;
                if is_last {
                    *exhausted = true;
                    return None;
                }
                Some(if is_empty { b'\n' as u32 } else { b' ' as u32 })
            }
        }
    }

    fn unget(&mut self, c: u32) {
        match self {
            Self::Stdin { buf, .. } | Self::Args { buf, .. } => {
                *buf = Some(c);
            }
        }
    }
}

impl CharReader for InputIter {
    fn next(&mut self) -> Option<u32> {
        InputIter::next(self)
    }
    fn unget(&mut self, c: u32) {
        InputIter::unget(self, c);
    }
}

#[derive(Debug, Clone)]
struct CliConfig {
    smushmode: u32,
    smushoverride: SmushOverride,
    justification: i32,
    right2left: i32,
    paragraphflag: bool,
    deutschflag: bool,
    cmdinput: bool,
    outputwidth: u32,
    fontdirname: String,
    fontname: String,
    multibyte: u32,
    controlfile: Option<String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            smushmode: 0,
            smushoverride: SmushOverride::No,
            justification: -1,
            right2left: -1,
            paragraphflag: false,
            deutschflag: false,
            cmdinput: false,
            outputwidth: 80,
            fontdirname: "fonts".to_string(),
            fontname: "standard".to_string(),
            multibyte: 0,
            controlfile: None,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Parser, Debug)]
#[command(
    name = "figby",
    about = "Rust port of FIGlet — ASCII art banner generator"
)]
struct CliArgs {
    #[arg(short = 'A')]
    flag_A: bool,
    #[arg(short = 'D')]
    flag_D: bool,
    #[arg(short = 'E')]
    flag_E: bool,
    #[arg(short = 'X')]
    flag_X: bool,
    #[arg(short = 'L')]
    flag_L: bool,
    #[arg(short = 'R')]
    flag_R: bool,
    #[arg(short = 'x')]
    flag_x: bool,
    #[arg(short = 'l')]
    flag_l: bool,
    #[arg(short = 'c')]
    flag_c: bool,
    #[arg(short = 'r')]
    flag_r: bool,
    #[arg(short = 'p')]
    flag_p: bool,
    #[arg(short = 'n')]
    flag_n: bool,
    #[arg(short = 's')]
    flag_s: bool,
    #[arg(short = 'k')]
    flag_k: bool,
    #[arg(short = 'S')]
    flag_S: bool,
    #[arg(short = 'o')]
    flag_o: bool,
    #[arg(short = 'W')]
    flag_W: bool,
    #[arg(short = 't')]
    flag_t: bool,
    #[arg(short = 'v')]
    flag_v: bool,
    #[arg(short = 'N')]
    flag_N: bool,
    #[arg(short = 'F')]
    flag_F: bool,
    #[arg(short = 'I')]
    infocode: Option<i32>,
    #[arg(short = 'm', allow_hyphen_values = true)]
    smushmode_arg: Option<i32>,
    #[arg(short = 'w')]
    outputwidth_arg: Option<u32>,
    #[arg(short = 'd')]
    fontdir: Option<String>,
    #[arg(short = 'f')]
    fontname_arg: Option<String>,
    #[arg(short = 'C')]
    controlfile: Option<String>,
    #[arg()]
    message: Vec<String>,
}

impl CliConfig {
    fn from_args(args: CliArgs) -> Self {
        let mut config = CliConfig::default();

        if !args.message.is_empty() || args.flag_A {
            config.cmdinput = true;
        }

        if args.flag_D {
            config.deutschflag = true;
        }
        if args.flag_E {
            config.deutschflag = false;
        }

        if args.flag_X {
            config.right2left = -1;
        }
        if args.flag_L {
            config.right2left = 0;
        }
        if args.flag_R {
            config.right2left = 1;
        }

        if args.flag_x {
            config.justification = -1;
        }
        if args.flag_l {
            config.justification = 0;
        }
        if args.flag_c {
            config.justification = 1;
        }
        if args.flag_r {
            config.justification = 2;
        }

        if args.flag_p {
            config.paragraphflag = true;
        }
        if args.flag_n {
            config.paragraphflag = false;
        }

        if args.flag_N {
            config.multibyte = 0;
        }

        if args.flag_W {
            config.smushmode = 0;
            config.smushoverride = SmushOverride::Yes;
        }
        if args.flag_k {
            config.smushmode = 64;
            config.smushoverride = SmushOverride::Yes;
        }
        if args.flag_o {
            config.smushmode = 128;
            config.smushoverride = SmushOverride::Yes;
        }
        if args.flag_S {
            config.smushmode = 128;
            config.smushoverride = SmushOverride::Force;
        }
        if args.flag_s {
            config.smushoverride = SmushOverride::No;
        }

        if let Some(val) = args.smushmode_arg {
            if val < -1 {
                config.smushoverride = SmushOverride::No;
            } else if val == -1 {
                config.smushmode = 0;
                config.smushoverride = SmushOverride::Yes;
            } else if val == 0 {
                config.smushmode = 64;
                config.smushoverride = SmushOverride::Yes;
            } else {
                config.smushmode = (val as u32 & 63) | 128;
                config.smushoverride = SmushOverride::Yes;
            }
        }

        if args.flag_t {
            if let Some(cols) = get_columns() {
                if cols > 0 {
                    config.outputwidth = cols as u32;
                }
            }
        }

        if let Some(val) = args.outputwidth_arg {
            config.outputwidth = val;
        }

        if let Some(val) = args.fontdir {
            config.fontdirname = val;
        }

        if let Some(val) = args.fontname_arg {
            config.fontname = val;
        }

        config.controlfile = args.controlfile;

        config
    }
}

fn printusage(out: &mut impl Write, myname: &str) -> io::Result<()> {
    writeln!(
        out,
        "Usage: {myname} [ -cklnoprstvxDELNRSWX ] [ -d fontdirectory ]"
    )?;
    writeln!(
        out,
        "              [ -f fontfile ] [ -m smushmode ] [ -w outputwidth ]"
    )?;
    writeln!(
        out,
        "              [ -C controlfile ] [ -I infocode ] [ message ]"
    )?;
    Ok(())
}

fn printinfo(
    out: &mut impl Write,
    infocode: i32,
    config: &CliConfig,
    myname: &str,
) -> io::Result<()> {
    match infocode {
        0 => {
            writeln!(
                out,
                "FIGlet Copyright (C) 1991-2012 Glenn Chappell, Ian Chai, John Cowan,"
            )?;
            writeln!(out, "Christiaan Keet and Claudio Matsuoka")?;
            writeln!(
                out,
                "Internet: <info@figlet.org> Version: {}, date: {}",
                VERSION, DATE
            )?;
            writeln!(out)?;
            writeln!(
                out,
                "FIGlet, along with the various FIGlet fonts and documentation, may be"
            )?;
            writeln!(out, "freely copied and distributed.")?;
            writeln!(out)?;
            writeln!(
                out,
                "If you use FIGlet, please send an e-mail message to <info@figlet.org>."
            )?;
            writeln!(out)?;
            writeln!(
                out,
                "The latest version of FIGlet is available from the web site,"
            )?;
            writeln!(out, "\thttp://www.figlet.org/")?;
            writeln!(out)?;
            printusage(out, myname)?;
        }
        1 => {
            writeln!(out, "{}", VERSION_INT)?;
        }
        2 => {
            writeln!(out, "{}", config.fontdirname)?;
        }
        3 => {
            writeln!(out, "{}", config.fontname)?;
        }
        4 => {
            writeln!(out, "{}", config.outputwidth)?;
        }
        5 => {
            write!(out, "{}", FONTFILE_MAGIC)?;
            write!(out, " {}", TOILETFILE_MAGIC)?;
            writeln!(out)?;
        }
        _ => {}
    }
    Ok(())
}

fn get_columns() -> Option<u16> {
    termion::terminal_size().ok().map(|(cols, _)| cols)
}

#[allow(clippy::ptr_arg, clippy::too_many_arguments)]
fn flush_output_line(
    output_rows: &mut Vec<String>,
    font: &FIGfont,
    justification: Justification,
    output_width: usize,
    char_buffer: &mut Vec<u32>,
    outlinelen: &mut usize,
    prev_width: &mut usize,
    out: &mut impl Write,
) {
    let rendered = render_line(output_rows, font.hardblank, justification, output_width);
    for row in &rendered {
        let _ = writeln!(out, "{}", row);
    }
    for row in output_rows.iter_mut() {
        row.clear();
    }
    char_buffer.clear();
    *outlinelen = 0;
    *prev_width = 0;
}

fn run(config: CliConfig, message: Vec<String>) {
    let font = match font::load_font(&config.fontname, &config.fontdirname) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let mut control_state = match config.controlfile {
        Some(ref path) => {
            let mut state = control::ControlState::default();
            if let Err(e) = control::read_control(path, &mut state) {
                eprintln!("Error reading control file: {e}");
                process::exit(1);
            }
            state
        }
        None => control::ControlState::default(),
    };

    let smush_mode = match config.smushoverride {
        SmushOverride::No => SmushMode::new(font.full_layout as u32),
        SmushOverride::Yes => SmushMode::new(config.smushmode),
        SmushOverride::Force => SmushMode::new(font.full_layout as u32 | config.smushmode),
    };

    let rtl = if config.right2left == -1 {
        font.print_direction == 1
    } else {
        config.right2left == 1
    };
    let justification = if config.justification == -1 {
        if rtl {
            Justification::Right
        } else {
            Justification::Left
        }
    } else {
        Justification::from_i32(config.justification)
    };

    let outlinelen_limit = config.outputwidth.saturating_sub(1) as usize;
    let output_width = config.outputwidth as usize;
    let height = font.charheight as usize;

    let mut output_rows = vec![String::new(); height];
    let mut char_buffer: Vec<u32> = Vec::new();
    let mut outlinelen: usize = 0;
    let mut prev_width: usize = 0;
    let mut wordbreakmode: i32 = 0;
    let mut last_was_eol_flag: bool = false;

    let mut input = InputIter::new(message, config.cmdinput);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut hz_state = input::HZState::default();

    let next_char = |input: &mut InputIter,
                     state: &mut control::ControlState,
                     hz: &mut input::HZState|
     -> Option<u32> {
        match config.multibyte {
            0 => state.iso2022(input),
            1 | 4 => input::read_dbcs_char(input),
            2 => input::read_utf8_char(input),
            3 => input::read_hz_char(input, hz),
            _ => input.next(),
        }
    };

    loop {
        let c = match next_char(&mut input, &mut control_state, &mut hz_state) {
            Some(c) => c,
            None => {
                if outlinelen != 0 {
                    flush_output_line(
                        &mut output_rows,
                        &font,
                        justification,
                        output_width,
                        &mut char_buffer,
                        &mut outlinelen,
                        &mut prev_width,
                        &mut out,
                    );
                }
                break;
            }
        };

        let mut c = c;

        // Paragraph mode
        if c == b'\n' as u32 && config.paragraphflag && !last_was_eol_flag {
            match next_char(&mut input, &mut control_state, &mut hz_state) {
                None => {
                    // Trailing newline at EOF becomes a space (matches C figlet behavior)
                    c = b' ' as u32;
                }
                Some(c2) => {
                    let is_ws = c2 <= 127 && (c2 as u8 as char).is_ascii_whitespace();
                    if !is_ws {
                        c = b' ' as u32;
                    }
                    input.unget(c2);
                }
            }
        }

        // Update last_was_eol_flag
        last_was_eol_flag = c <= 127
            && (c as u8 as char).is_ascii_whitespace()
            && c != b'\t' as u32
            && c != b' ' as u32;

        c = input::deutsch_reroute(c, config.deutschflag);

        c = control::remap_char(&control_state, c);

        // Space normalization
        if c <= 127 && (c as u8 as char).is_ascii_whitespace() {
            c = if c == b'\t' as u32 || c == b' ' as u32 {
                b' ' as u32
            } else {
                b'\n' as u32
            };
        }

        // Skip control chars 1-31 (except \n) and 127 (DEL)
        if (c > 0 && c < b' ' as u32 && c != b'\n' as u32) || c == 127 {
            continue;
        }

        // Inner loop (handles addchar retry after split/print, like C do-while)
        // Every branch either breaks (char handled) or falls through to retry
        loop {
            if wordbreakmode == -1 {
                if c == b' ' as u32 {
                    break;
                } else if c == b'\n' as u32 {
                    wordbreakmode = 0;
                    break;
                }
                wordbreakmode = 0;
            }

            if c == b'\n' as u32 {
                flush_output_line(
                    &mut output_rows,
                    &font,
                    justification,
                    output_width,
                    &mut char_buffer,
                    &mut outlinelen,
                    &mut prev_width,
                    &mut out,
                );
                wordbreakmode = 0;
                break;
            }

            if add_char(
                &font,
                c,
                &mut output_rows,
                &mut outlinelen,
                &mut prev_width,
                smush_mode,
                rtl,
                outlinelen_limit,
            ) {
                char_buffer.push(c);
                if c != b' ' as u32 {
                    wordbreakmode = if wordbreakmode >= 2 { 3 } else { 1 };
                } else {
                    wordbreakmode = if wordbreakmode > 0 { 2 } else { 0 };
                }
                break;
            }

            if outlinelen == 0 {
                // Raw-char path: char wider than empty line
                let mut dummy = 0;
                let ch = lookup_char(&font, c, &mut dummy);
                let rows = ch.rows();
                let raw_rows: Vec<String> = if rtl && output_width > 1 {
                    rows.iter()
                        .map(|row| {
                            let len = row.chars().count();
                            let start = len.saturating_sub(outlinelen_limit);
                            row.chars().skip(start).collect()
                        })
                        .collect()
                } else {
                    rows.to_vec()
                };
                let rendered = render_line(&raw_rows, font.hardblank, justification, output_width);
                for row in &rendered {
                    let _ = writeln!(out, "{}", row);
                }
                wordbreakmode = -1;
                break;
            }

            // addchar failed — need to flush current line and retry
            if c == b' ' as u32 {
                if wordbreakmode == 2 {
                    if let Some((part1_rows, part2_start)) = split_line(
                        &font,
                        &char_buffer,
                        &mut output_rows,
                        &mut outlinelen,
                        &mut prev_width,
                        smush_mode,
                        rtl,
                        outlinelen_limit,
                    ) {
                        let rendered =
                            render_line(&part1_rows, font.hardblank, justification, output_width);
                        for row in &rendered {
                            let _ = writeln!(out, "{}", row);
                        }
                        char_buffer.drain(..part2_start);
                    } else {
                        flush_output_line(
                            &mut output_rows,
                            &font,
                            justification,
                            output_width,
                            &mut char_buffer,
                            &mut outlinelen,
                            &mut prev_width,
                            &mut out,
                        );
                    }
                } else {
                    flush_output_line(
                        &mut output_rows,
                        &font,
                        justification,
                        output_width,
                        &mut char_buffer,
                        &mut outlinelen,
                        &mut prev_width,
                        &mut out,
                    );
                }
                wordbreakmode = -1;
                break;
            }

            // Non-space char that didn't fit — retry after flush/split
            if wordbreakmode >= 2 {
                if let Some((part1_rows, part2_start)) = split_line(
                    &font,
                    &char_buffer,
                    &mut output_rows,
                    &mut outlinelen,
                    &mut prev_width,
                    smush_mode,
                    rtl,
                    outlinelen_limit,
                ) {
                    let rendered =
                        render_line(&part1_rows, font.hardblank, justification, output_width);
                    for row in &rendered {
                        let _ = writeln!(out, "{}", row);
                    }
                    char_buffer.drain(..part2_start);
                } else {
                    flush_output_line(
                        &mut output_rows,
                        &font,
                        justification,
                        output_width,
                        &mut char_buffer,
                        &mut outlinelen,
                        &mut prev_width,
                        &mut out,
                    );
                }
            } else {
                flush_output_line(
                    &mut output_rows,
                    &font,
                    justification,
                    output_width,
                    &mut char_buffer,
                    &mut outlinelen,
                    &mut prev_width,
                    &mut out,
                );
            }
            wordbreakmode = if wordbreakmode == 3 { 1 } else { 0 };
            // loop continues (retry addchar with fresh output line)
        }
    }
}

fn main() {
    let args = CliArgs::parse();
    let infocode = args.infocode;

    if args.flag_F {
        eprintln!("Error: -F option is not implemented in this version");
        process::exit(1);
    }

    let message = args.message.clone();
    let config = CliConfig::from_args(args);

    if let Some(infocode) = infocode {
        let myname = match std::env::args().next() {
            Some(s) => {
                let s = s.rsplit('/').next().unwrap_or(&s);
                s.to_string()
            }
            None => "figby".to_string(),
        };
        let mut stdout = io::stdout().lock();
        if let Err(e) = printinfo(&mut stdout, infocode, &config, &myname) {
            eprintln!("Error writing info: {e}");
            process::exit(1);
        }
        process::exit(0);
    }

    run(config, message);
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use figby::font::DEUTSCH_CHARS;

    #[test]
    fn test_default_values() {
        let args = CliArgs::try_parse_from(["figby"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.smushmode, 0);
        assert_eq!(config.smushoverride, SmushOverride::No);
        assert_eq!(config.justification, -1);
        assert_eq!(config.right2left, -1);
        assert!(!config.paragraphflag);
        assert!(!config.deutschflag);
        assert!(!config.cmdinput);
        assert_eq!(config.outputwidth, 80);
        assert_eq!(config.fontdirname, "fonts");
        assert_eq!(config.fontname, "standard");
        assert_eq!(config.multibyte, 0);
    }

    #[test]
    fn test_flag_A_cmdinput() {
        let args = CliArgs::try_parse_from(["figby", "-A"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.cmdinput);
    }

    #[test]
    fn test_flag_D_deutsch() {
        let args = CliArgs::try_parse_from(["figby", "-D"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.deutschflag);
    }

    #[test]
    fn test_flag_E_deutsch() {
        let args = CliArgs::try_parse_from(["figby", "-E"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(!config.deutschflag);
    }

    #[test]
    fn test_flags_X_L_R_right2left() {
        let args_x = CliArgs::try_parse_from(["figby", "-X"]).unwrap();
        assert_eq!(CliConfig::from_args(args_x).right2left, -1);

        let args_l = CliArgs::try_parse_from(["figby", "-L"]).unwrap();
        assert_eq!(CliConfig::from_args(args_l).right2left, 0);

        let args_r = CliArgs::try_parse_from(["figby", "-R"]).unwrap();
        assert_eq!(CliConfig::from_args(args_r).right2left, 1);
    }

    #[test]
    fn test_flags_x_l_c_r_justification() {
        let args_x = CliArgs::try_parse_from(["figby", "-x"]).unwrap();
        assert_eq!(CliConfig::from_args(args_x).justification, -1);

        let args_l = CliArgs::try_parse_from(["figby", "-l"]).unwrap();
        assert_eq!(CliConfig::from_args(args_l).justification, 0);

        let args_c = CliArgs::try_parse_from(["figby", "-c"]).unwrap();
        assert_eq!(CliConfig::from_args(args_c).justification, 1);

        let args_r = CliArgs::try_parse_from(["figby", "-r"]).unwrap();
        assert_eq!(CliConfig::from_args(args_r).justification, 2);
    }

    #[test]
    fn test_flags_p_n_paragraph() {
        let args_p = CliArgs::try_parse_from(["figby", "-p"]).unwrap();
        assert!(CliConfig::from_args(args_p).paragraphflag);

        let args_n = CliArgs::try_parse_from(["figby", "-n"]).unwrap();
        assert!(!CliConfig::from_args(args_n).paragraphflag);
    }

    #[test]
    fn test_flags_s_k_S_o_W_smush() {
        let args_s = CliArgs::try_parse_from(["figby", "-s"]).unwrap();
        let config_s = CliConfig::from_args(args_s);
        assert_eq!(config_s.smushoverride, SmushOverride::No);

        let args_k = CliArgs::try_parse_from(["figby", "-k"]).unwrap();
        let config_k = CliConfig::from_args(args_k);
        assert_eq!(config_k.smushmode, 64);
        assert_eq!(config_k.smushoverride, SmushOverride::Yes);

        let args_S = CliArgs::try_parse_from(["figby", "-S"]).unwrap();
        let config_S = CliConfig::from_args(args_S);
        assert_eq!(config_S.smushmode, 128);
        assert_eq!(config_S.smushoverride, SmushOverride::Force);

        let args_o = CliArgs::try_parse_from(["figby", "-o"]).unwrap();
        let config_o = CliConfig::from_args(args_o);
        assert_eq!(config_o.smushmode, 128);
        assert_eq!(config_o.smushoverride, SmushOverride::Yes);

        let args_W = CliArgs::try_parse_from(["figby", "-W"]).unwrap();
        let config_W = CliConfig::from_args(args_W);
        assert_eq!(config_W.smushmode, 0);
        assert_eq!(config_W.smushoverride, SmushOverride::Yes);
    }

    #[test]
    fn test_flag_N_multibyte() {
        let args = CliArgs::try_parse_from(["figby", "-N"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.multibyte, 0);
    }

    #[test]
    fn test_flag_t_terminal() {
        let args = CliArgs::try_parse_from(["figby", "-t"]).unwrap();
        assert!(args.flag_t);
    }

    #[test]
    fn test_flag_t_updates_width() {
        let args = CliArgs::try_parse_from(["figby", "-t"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.outputwidth >= 80);
    }

    #[test]
    fn test_get_columns_never_panics() {
        let cols = get_columns();
        let _ = cols; // never panics, always returns Some or None
        assert!(cols.is_none() || cols.unwrap() >= 20);
    }

    #[test]
    fn test_flag_t_w_override() {
        let args = CliArgs::try_parse_from(["figby", "-t", "-w", "120"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.outputwidth, 120);
    }

    #[test]
    fn test_flag_v_version() {
        let args = CliArgs::try_parse_from(["figby", "-v"]).unwrap();
        assert!(args.flag_v);
    }

    #[test]
    fn test_flag_I_infocode() {
        let args = CliArgs::try_parse_from(["figby", "-I", "3"]).unwrap();
        assert_eq!(args.infocode, Some(3));
    }

    #[test]
    fn test_flag_m_smushmode() {
        let args_0 = CliArgs::try_parse_from(["figby", "-m", "0"]).unwrap();
        let config_0 = CliConfig::from_args(args_0);
        assert_eq!(config_0.smushmode, 64);
        assert_eq!(config_0.smushoverride, SmushOverride::Yes);

        let args_neg1 = CliArgs::try_parse_from(["figby", "-m", "-1"]).unwrap();
        let config_neg1 = CliConfig::from_args(args_neg1);
        assert_eq!(config_neg1.smushmode, 0);
        assert_eq!(config_neg1.smushoverride, SmushOverride::Yes);

        let args_neg2 = CliArgs::try_parse_from(["figby", "-m", "-2"]).unwrap();
        let config_neg2 = CliConfig::from_args(args_neg2);
        assert_eq!(config_neg2.smushoverride, SmushOverride::No);

        let args_5 = CliArgs::try_parse_from(["figby", "-m", "5"]).unwrap();
        let config_5 = CliConfig::from_args(args_5);
        assert_eq!(config_5.smushmode, (5 & 63) | 128);
        assert_eq!(config_5.smushoverride, SmushOverride::Yes);
    }

    #[test]
    fn test_flag_w_width() {
        let args = CliArgs::try_parse_from(["figby", "-w", "120"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.outputwidth, 120);
    }

    #[test]
    fn test_flag_d_fontdir() {
        let args = CliArgs::try_parse_from(["figby", "-d", "/my/fonts"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.fontdirname, "/my/fonts");
    }

    #[test]
    fn test_flag_f_fontname() {
        let args = CliArgs::try_parse_from(["figby", "-f", "big"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.fontname, "big");
    }

    #[test]
    fn test_flag_C_controlfile() {
        let args = CliArgs::try_parse_from(["figby", "-C", "my.flc"]).unwrap();
        assert_eq!(args.controlfile, Some("my.flc".to_string()));
    }

    #[test]
    fn test_flag_F_error() {
        let args = CliArgs::try_parse_from(["figby", "-F"]).unwrap();
        assert!(args.flag_F);
    }

    #[test]
    fn test_positional_args_cmdinput() {
        let args = CliArgs::try_parse_from(["figby", "hello"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.cmdinput);
    }

    #[test]
    fn test_flag_last_wins() {
        let args = CliArgs::try_parse_from(["figby", "-k", "-s"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.smushoverride, SmushOverride::No);
    }

    #[test]
    fn test_infocode_0_copyright() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 0, &config, "figby").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("FIGlet Copyright (C)"));
        assert!(output.contains("figby"));
    }

    #[test]
    fn test_infocode_1_version() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 1, &config, "figby").unwrap();
        assert_eq!(buf, b"20205\n");
    }

    #[test]
    fn test_infocode_2_fontdir() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 2, &config, "figby").unwrap();
        assert_eq!(buf, b"fonts\n");
    }

    #[test]
    fn test_infocode_3_font() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 3, &config, "figby").unwrap();
        assert_eq!(buf, b"standard\n");
    }

    #[test]
    fn test_infocode_4_outputwidth() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 4, &config, "figby").unwrap();
        assert_eq!(buf, b"80\n");
    }

    #[test]
    fn test_infocode_5_formats() {
        let config = CliConfig::default();
        let mut buf = Vec::new();
        printinfo(&mut buf, 5, &config, "figby").unwrap();
        assert_eq!(buf, b"flf2 tlf2\n");
    }

    // --- InputIter tests ---

    #[test]
    fn test_input_iter_stdin_empty() {
        let mut iter = InputIter::new(vec![], false);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_input_iter_args_empty() {
        let mut iter = InputIter::new(vec![], true);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_input_iter_single_word() {
        let mut iter = InputIter::new(vec!["hello".to_string()], true);
        assert_eq!(iter.next(), Some(b'h' as u32));
        assert_eq!(iter.next(), Some(b'e' as u32));
        assert_eq!(iter.next(), Some(b'l' as u32));
        assert_eq!(iter.next(), Some(b'l' as u32));
        assert_eq!(iter.next(), Some(b'o' as u32));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_input_iter_two_words() {
        let mut iter = InputIter::new(vec!["hello".to_string(), "world".to_string()], true);
        let chars: Vec<u32> = std::iter::from_fn(|| iter.next()).collect();
        assert_eq!(
            chars,
            vec![
                b'h' as u32,
                b'e' as u32,
                b'l' as u32,
                b'l' as u32,
                b'o' as u32,
                b' ' as u32,
                b'w' as u32,
                b'o' as u32,
                b'r' as u32,
                b'l' as u32,
                b'd' as u32,
            ]
        );
    }

    #[test]
    fn test_input_iter_empty_word_middle() {
        let mut iter = InputIter::new(
            vec!["hello".to_string(), "".to_string(), "world".to_string()],
            true,
        );
        let chars: Vec<u32> = std::iter::from_fn(|| iter.next()).collect();
        assert_eq!(
            chars,
            vec![
                b'h' as u32,
                b'e' as u32,
                b'l' as u32,
                b'l' as u32,
                b'o' as u32,
                b' ' as u32,
                b'\n' as u32,
                b'w' as u32,
                b'o' as u32,
                b'r' as u32,
                b'l' as u32,
                b'd' as u32,
            ]
        );
    }

    #[test]
    fn test_input_iter_empty_word_at_end() {
        let mut iter = InputIter::new(vec!["hello".to_string(), "".to_string()], true);
        let chars: Vec<u32> = std::iter::from_fn(|| iter.next()).collect();
        assert_eq!(
            chars,
            vec![
                b'h' as u32,
                b'e' as u32,
                b'l' as u32,
                b'l' as u32,
                b'o' as u32,
                b' ' as u32,
            ]
        );
    }

    #[test]
    fn test_input_iter_unget() {
        let mut iter = InputIter::new(vec!["ab".to_string()], true);
        assert_eq!(iter.next(), Some(b'a' as u32));
        iter.unget(b'x' as u32);
        assert_eq!(iter.next(), Some(b'x' as u32));
        assert_eq!(iter.next(), Some(b'b' as u32));
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_end_to_end_hello() {
        let tmpdir = std::env::temp_dir().join("figby-test-1.3.4");
        let _ = std::fs::create_dir_all(&tmpdir);
        let fontpath = tmpdir.join("testfont.flf");
        // Minimal FIGfont (height=1, baseline=0, max=1, old=0, comment=0)
        let mut font = String::from("flf2a$ 1 0 1 0 0\n");
        for code in 32..=126u32 {
            let c = char::from_u32(code).unwrap();
            font.push(c);
            font.push_str("  @\n");
        }
        for &code in &DEUTSCH_CHARS {
            let c = char::from_u32(code).unwrap();
            font.push(c);
            font.push_str("  @\n");
        }
        std::fs::write(&fontpath, &font).unwrap();
        let config = CliConfig {
            cmdinput: true,
            outputwidth: 80,
            fontdirname: tmpdir.to_str().unwrap().to_string(),
            fontname: "testfont".to_string(),
            ..Default::default()
        };
        // Exercise full pipeline: input → font → render → output
        run(config, vec!["Hello".to_string()]);
        // Should not panic. stdout captured by test framework.
        let _ = std::fs::remove_file(&fontpath);
        let _ = std::fs::remove_dir(&tmpdir);
    }

    #[test]
    fn test_input_iter_all_empty() {
        let mut iter = InputIter::new(vec!["".to_string()], true);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_input_iter_three_empty() {
        let mut iter = InputIter::new(vec!["".to_string(), "".to_string(), "".to_string()], true);
        let chars: Vec<u32> = std::iter::from_fn(|| iter.next()).collect();
        assert_eq!(chars, vec![b'\n' as u32, b'\n' as u32]);
    }

    #[test]
    fn test_input_iter_multiple_empty_then_word() {
        let mut iter = InputIter::new(vec!["".to_string(), "".to_string(), "a".to_string()], true);
        let chars: Vec<u32> = std::iter::from_fn(|| iter.next()).collect();
        assert_eq!(chars, vec![b'\n' as u32, b'\n' as u32, b'a' as u32]);
    }
}
