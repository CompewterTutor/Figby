use clap::Parser;
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmushOverride {
    No = 0,
    Yes = 1,
    Force = 2,
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
        }
    }
}

#[allow(non_snake_case)]
#[derive(Parser, Debug)]
#[command(
    name = "feiglet",
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

        if let Some(val) = args.outputwidth_arg {
            config.outputwidth = val;
        }

        if let Some(val) = args.fontdir {
            config.fontdirname = val;
        }

        if let Some(val) = args.fontname_arg {
            config.fontname = val;
        }

        config
    }
}

fn run(_config: CliConfig) {}

fn main() {
    let args = CliArgs::parse();

    if args.flag_F {
        eprintln!("Error: -F option is not implemented in this version");
        process::exit(1);
    }

    let config = CliConfig::from_args(args);
    run(config);
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let args = CliArgs::try_parse_from(["feiglet"]).unwrap();
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
        let args = CliArgs::try_parse_from(["feiglet", "-A"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.cmdinput);
    }

    #[test]
    fn test_flag_D_deutsch() {
        let args = CliArgs::try_parse_from(["feiglet", "-D"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.deutschflag);
    }

    #[test]
    fn test_flag_E_deutsch() {
        let args = CliArgs::try_parse_from(["feiglet", "-E"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(!config.deutschflag);
    }

    #[test]
    fn test_flags_X_L_R_right2left() {
        let args_x = CliArgs::try_parse_from(["feiglet", "-X"]).unwrap();
        assert_eq!(CliConfig::from_args(args_x).right2left, -1);

        let args_l = CliArgs::try_parse_from(["feiglet", "-L"]).unwrap();
        assert_eq!(CliConfig::from_args(args_l).right2left, 0);

        let args_r = CliArgs::try_parse_from(["feiglet", "-R"]).unwrap();
        assert_eq!(CliConfig::from_args(args_r).right2left, 1);
    }

    #[test]
    fn test_flags_x_l_c_r_justification() {
        let args_x = CliArgs::try_parse_from(["feiglet", "-x"]).unwrap();
        assert_eq!(CliConfig::from_args(args_x).justification, -1);

        let args_l = CliArgs::try_parse_from(["feiglet", "-l"]).unwrap();
        assert_eq!(CliConfig::from_args(args_l).justification, 0);

        let args_c = CliArgs::try_parse_from(["feiglet", "-c"]).unwrap();
        assert_eq!(CliConfig::from_args(args_c).justification, 1);

        let args_r = CliArgs::try_parse_from(["feiglet", "-r"]).unwrap();
        assert_eq!(CliConfig::from_args(args_r).justification, 2);
    }

    #[test]
    fn test_flags_p_n_paragraph() {
        let args_p = CliArgs::try_parse_from(["feiglet", "-p"]).unwrap();
        assert!(CliConfig::from_args(args_p).paragraphflag);

        let args_n = CliArgs::try_parse_from(["feiglet", "-n"]).unwrap();
        assert!(!CliConfig::from_args(args_n).paragraphflag);
    }

    #[test]
    fn test_flags_s_k_S_o_W_smush() {
        let args_s = CliArgs::try_parse_from(["feiglet", "-s"]).unwrap();
        let config_s = CliConfig::from_args(args_s);
        assert_eq!(config_s.smushoverride, SmushOverride::No);

        let args_k = CliArgs::try_parse_from(["feiglet", "-k"]).unwrap();
        let config_k = CliConfig::from_args(args_k);
        assert_eq!(config_k.smushmode, 64);
        assert_eq!(config_k.smushoverride, SmushOverride::Yes);

        let args_S = CliArgs::try_parse_from(["feiglet", "-S"]).unwrap();
        let config_S = CliConfig::from_args(args_S);
        assert_eq!(config_S.smushmode, 128);
        assert_eq!(config_S.smushoverride, SmushOverride::Force);

        let args_o = CliArgs::try_parse_from(["feiglet", "-o"]).unwrap();
        let config_o = CliConfig::from_args(args_o);
        assert_eq!(config_o.smushmode, 128);
        assert_eq!(config_o.smushoverride, SmushOverride::Yes);

        let args_W = CliArgs::try_parse_from(["feiglet", "-W"]).unwrap();
        let config_W = CliConfig::from_args(args_W);
        assert_eq!(config_W.smushmode, 0);
        assert_eq!(config_W.smushoverride, SmushOverride::Yes);
    }

    #[test]
    fn test_flag_N_multibyte() {
        let args = CliArgs::try_parse_from(["feiglet", "-N"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.multibyte, 0);
    }

    #[test]
    fn test_flag_t_terminal() {
        let args = CliArgs::try_parse_from(["feiglet", "-t"]).unwrap();
        assert!(args.flag_t);
    }

    #[test]
    fn test_flag_v_version() {
        let args = CliArgs::try_parse_from(["feiglet", "-v"]).unwrap();
        assert!(args.flag_v);
    }

    #[test]
    fn test_flag_I_infocode() {
        let args = CliArgs::try_parse_from(["feiglet", "-I", "3"]).unwrap();
        assert_eq!(args.infocode, Some(3));
    }

    #[test]
    fn test_flag_m_smushmode() {
        let args_0 = CliArgs::try_parse_from(["feiglet", "-m", "0"]).unwrap();
        let config_0 = CliConfig::from_args(args_0);
        assert_eq!(config_0.smushmode, 64);
        assert_eq!(config_0.smushoverride, SmushOverride::Yes);

        let args_neg1 = CliArgs::try_parse_from(["feiglet", "-m", "-1"]).unwrap();
        let config_neg1 = CliConfig::from_args(args_neg1);
        assert_eq!(config_neg1.smushmode, 0);
        assert_eq!(config_neg1.smushoverride, SmushOverride::Yes);

        let args_neg2 = CliArgs::try_parse_from(["feiglet", "-m", "-2"]).unwrap();
        let config_neg2 = CliConfig::from_args(args_neg2);
        assert_eq!(config_neg2.smushoverride, SmushOverride::No);

        let args_5 = CliArgs::try_parse_from(["feiglet", "-m", "5"]).unwrap();
        let config_5 = CliConfig::from_args(args_5);
        assert_eq!(config_5.smushmode, (5 & 63) | 128);
        assert_eq!(config_5.smushoverride, SmushOverride::Yes);
    }

    #[test]
    fn test_flag_w_width() {
        let args = CliArgs::try_parse_from(["feiglet", "-w", "120"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.outputwidth, 120);
    }

    #[test]
    fn test_flag_d_fontdir() {
        let args = CliArgs::try_parse_from(["feiglet", "-d", "/my/fonts"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.fontdirname, "/my/fonts");
    }

    #[test]
    fn test_flag_f_fontname() {
        let args = CliArgs::try_parse_from(["feiglet", "-f", "big"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.fontname, "big");
    }

    #[test]
    fn test_flag_C_controlfile() {
        let args = CliArgs::try_parse_from(["feiglet", "-C", "my.flc"]).unwrap();
        assert_eq!(args.controlfile, Some("my.flc".to_string()));
    }

    #[test]
    fn test_flag_F_error() {
        let args = CliArgs::try_parse_from(["feiglet", "-F"]).unwrap();
        assert!(args.flag_F);
    }

    #[test]
    fn test_positional_args_cmdinput() {
        let args = CliArgs::try_parse_from(["feiglet", "hello"]).unwrap();
        let config = CliConfig::from_args(args);
        assert!(config.cmdinput);
    }

    #[test]
    fn test_flag_last_wins() {
        let args = CliArgs::try_parse_from(["feiglet", "-k", "-s"]).unwrap();
        let config = CliConfig::from_args(args);
        assert_eq!(config.smushoverride, SmushOverride::No);
    }
}
