use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn figby_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_figby"))
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn expected_output(n: u32) -> Vec<u8> {
    let root = repo_root();
    let path = root.join(format!("tests/res{:03}.txt", n));
    std::fs::read(&path).unwrap_or_else(|_| panic!("missing expected output: {:?}", path))
}

fn run_figby(args: &[&str], stdin_data: Option<&[u8]>) -> Vec<u8> {
    let root = repo_root();
    let mut cmd = Command::new(figby_binary());
    cmd.current_dir(&root);
    cmd.env("FIGLET_FONTDIR", root.join("fonts"));
    cmd.stdout(std::process::Stdio::piped());
    cmd.args(args);
    if stdin_data.is_some() {
        cmd.stdin(std::process::Stdio::piped());
    }
    let mut child = cmd.spawn().expect("failed to spawn figby");
    if let Some(data) = stdin_data {
        child.stdin.as_mut().unwrap().write_all(data).unwrap();
    }
    let output = child.wait_with_output().expect("failed to run figby");
    assert!(
        output.status.success(),
        "figby failed: args={:?} stderr={}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn input_text() -> Vec<u8> {
    let root = repo_root();
    std::fs::read(root.join("tests/input.txt")).expect("missing tests/input.txt")
}

fn long_text() -> Vec<u8> {
    let root = repo_root();
    std::fs::read(root.join("tests/longtext.txt")).expect("missing tests/longtext.txt")
}

fn showfigfonts_output() -> Vec<u8> {
    let root = repo_root();
    let fonts_dir = root.join("fonts");

    let mut font_stems: Vec<String> = std::fs::read_dir(&fonts_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "flf")
                .unwrap_or(false)
        })
        .map(|e| e.path().file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    font_stems.sort();

    let mut output = Vec::new();
    for stem in &font_stems {
        output.extend_from_slice(format!("{} :\n", stem).as_bytes());
        let result = run_figby(&["-f", stem, stem], None);
        output.extend_from_slice(&result);
        output.push(b'\n');
        output.push(b'\n');
    }
    output
}

fn list_control_files_output() -> Vec<u8> {
    let root = repo_root();
    let fonts_dir = root.join("fonts");

    let mut flc_names: Vec<String> = std::fs::read_dir(&fonts_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "flc")
                .unwrap_or(false)
        })
        .map(|e| {
            let p = e.path();
            let rel = p.strip_prefix(&root).unwrap();
            rel.to_str().unwrap().to_string()
        })
        .collect();
    flc_names.sort();

    let mut output = Vec::new();
    for name in &flc_names {
        output.extend_from_slice(name.as_bytes());
        output.push(b'\n');
    }
    output
}

#[test]
fn test_01_showfigfonts() {
    let output = showfigfonts_output();
    let expected = expected_output(1);
    if output != expected {
        let diff_start = output
            .iter()
            .zip(expected.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        let diff_end = diff_start.saturating_sub(10);
        eprintln!(
            "First diff at byte {}:\n  got:  {:?}\n  want: {:?}",
            diff_start,
            &output[diff_end..(diff_end + 40).min(output.len())],
            &expected[diff_end..(diff_end + 40).min(expected.len())]
        );
        panic!("test_01_showfigfonts failed");
    }
    assert_eq!(output, expected);
}

#[test]
fn test_02_all_fonts() {
    let root = repo_root();
    let fonts_dir = root.join("fonts");
    let input = input_text();

    let mut font_stems: Vec<String> = std::fs::read_dir(&fonts_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "flf")
                .unwrap_or(false)
        })
        .map(|e| e.path().file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    font_stems.sort();

    let mut all_output = Vec::new();
    for stem in &font_stems {
        let font_path = format!("fonts/{}", stem);
        let result = run_figby(&["-f", &font_path], Some(&input));
        all_output.extend_from_slice(&result);
    }
    assert_eq!(all_output, expected_output(2));
}

#[test]
fn test_03_long_text() {
    let input = long_text();
    let output = run_figby(&[], Some(&input));
    assert_eq!(output, expected_output(3));
}

#[test]
fn test_04_left_to_right() {
    let input = input_text();
    let output = run_figby(&["-L"], Some(&input));
    assert_eq!(output, expected_output(4));
}

#[test]
fn test_05_right_to_left() {
    let input = input_text();
    let output = run_figby(&["-R"], Some(&input));
    assert_eq!(output, expected_output(5));
}

#[test]
fn test_06_flush_left() {
    let input = input_text();
    let output = run_figby(&["-l"], Some(&input));
    assert_eq!(output, expected_output(6));
}

#[test]
fn test_07_flush_right() {
    let input = input_text();
    let output = run_figby(&["-r"], Some(&input));
    assert_eq!(output, expected_output(7));
}

#[test]
fn test_08_center() {
    let input = input_text();
    let output = run_figby(&["-c"], Some(&input));
    assert_eq!(output, expected_output(8));
}

#[test]
fn test_09_kerning() {
    let input = input_text();
    let output = run_figby(&["-k"], Some(&input));
    assert_eq!(output, expected_output(9));
}

#[test]
fn test_10_full_width() {
    let input = input_text();
    let output = run_figby(&["-W"], Some(&input));
    assert_eq!(output, expected_output(10));
}

#[test]
fn test_11_overlap() {
    let input = input_text();
    let output = run_figby(&["-o"], Some(&input));
    assert_eq!(output, expected_output(11));
}

#[test]
fn test_12_tlf_font() {
    let input = input_text();
    let output = run_figby(&["-f", "tests/emboss"], Some(&input));
    assert_eq!(output, expected_output(12));
}

#[test]
fn test_13_kerning_flush_left_rtl() {
    let input = input_text();
    let output = run_figby(&["-klR"], Some(&input));
    assert_eq!(output, expected_output(13));
}

#[test]
fn test_14_kerning_center_rtl_slant() {
    let input = input_text();
    let output = run_figby(&["-kcR", "-f", "slant"], Some(&input));
    assert_eq!(output, expected_output(14));
}

#[test]
fn test_15_full_width_flush_right_rtl() {
    let input = input_text();
    let output = run_figby(&["-WrR"], Some(&input));
    assert_eq!(output, expected_output(15));
}

#[test]
fn test_16_overlap_flush_right_big() {
    let input = input_text();
    let output = run_figby(&["-or", "-f", "big"], Some(&input));
    assert_eq!(output, expected_output(16));
}

#[test]
fn test_17_tlf_kerning_flush_right() {
    let input = input_text();
    let output = run_figby(&["-kr", "-f", "tests/emboss"], Some(&input));
    assert_eq!(output, expected_output(17));
}

#[test]
fn test_18_tlf_overlap_center() {
    let input = input_text();
    let output = run_figby(&["-oc", "-f", "tests/emboss"], Some(&input));
    assert_eq!(output, expected_output(18));
}

#[test]
fn test_19_tlf_full_width_flush_left_rtl() {
    let input = input_text();
    let output = run_figby(&["-WRl", "-f", "tests/emboss"], Some(&input));
    assert_eq!(output, expected_output(19));
}

#[test]
fn test_20_specify_font_directory() {
    let root = repo_root();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let src = root.join("fonts/script.flf");
    let dst = dir.path().join("foo.flf");
    std::fs::copy(&src, &dst).expect("failed to copy font");

    let input = input_text();
    let fontdir_str = dir.path().to_str().unwrap();
    let output = run_figby(&["-d", fontdir_str, "-f", "foo"], Some(&input));
    assert_eq!(output, expected_output(20));
}

#[test]
fn test_21_paragraph_mode() {
    let input = input_text();
    let output = run_figby(&["-p", "-w250"], Some(&input));
    assert_eq!(output, expected_output(21));
}

#[test]
fn test_22_short_line() {
    let input = input_text();
    let output = run_figby(&["-w5"], Some(&input));
    assert_eq!(output, expected_output(22));
}

#[test]
fn test_23_kerning_paragraph_center_small() {
    let input = input_text();
    let output = run_figby(&["-kpc", "-f", "small"], Some(&input));
    assert_eq!(output, expected_output(23));
}

#[test]
fn test_24_list_control_files() {
    let output = list_control_files_output();
    assert_eq!(output, expected_output(24));
}

#[test]
fn test_25_uskata_control() {
    let output = run_figby(&["-f", "banner", "-C", "fonts/uskata.flc"], Some(b"ABCDE"));
    assert_eq!(output, expected_output(25));
}

#[test]
fn test_26_jis0201_control() {
    let output = run_figby(
        &["-f", "banner", "-C", "fonts/jis0201.flc"],
        Some(b"\xb1\xb2\xb3\xb4\xb5"),
    );
    assert_eq!(output, expected_output(26));
}

#[test]
fn test_27_rtl_smushing_jave() {
    let input = input_text();
    let output = run_figby(&["-f", "tests/flowerpower", "-R"], Some(&input));
    assert_eq!(output, expected_output(27));
}
