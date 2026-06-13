use image::{DynamicImage, ImageError};
use std::path::Path;

/// Default ASCII character map (darkest to brightest).
pub const DEFAULT_CHAR_MAP: &str = " .-:=+*#%@";

/// RGB pixel type alias: (red, green, blue) each 0-255.
pub type RgbPixel = (u8, u8, u8);

/// Load an image from file and convert to grayscale luminance matrix.
///
/// Returns `Vec<Vec<u8>>` where `matrix[y][x]` is the luminance (0-255)
/// at pixel `(x, y)`. The outer vector represents rows, inner columns.
pub fn load_luminance_matrix<P: AsRef<Path>>(path: P) -> Result<Vec<Vec<u8>>, ImageError> {
    let img = image::open(path)?;
    Ok(luminance_from_dynamic(&img))
}

/// Convert a `DynamicImage` to a grayscale luminance matrix.
///
/// Uses `image::DynamicImage::to_luma8()` internally (BT.709 coefficients).
pub fn luminance_from_dynamic(img: &DynamicImage) -> Vec<Vec<u8>> {
    let luma = img.to_luma8();
    let (width, height) = luma.dimensions();
    let mut matrix = Vec::with_capacity(height as usize);
    for y in 0..height {
        let mut row = Vec::with_capacity(width as usize);
        for x in 0..width {
            row.push(luma.get_pixel(x, y).0[0]);
        }
        matrix.push(row);
    }
    matrix
}

/// Load an image from file and return RGB pixel matrix preserving original color.
pub fn load_rgb_matrix<P: AsRef<Path>>(path: P) -> Result<Vec<Vec<RgbPixel>>, ImageError> {
    let img = image::open(path)?;
    Ok(rgb_from_dynamic(&img))
}

/// Convert a `DynamicImage` to an RGB pixel matrix preserving original color.
pub fn rgb_from_dynamic(img: &DynamicImage) -> Vec<Vec<RgbPixel>> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut matrix = Vec::with_capacity(height as usize);
    for y in 0..height {
        let mut row = Vec::with_capacity(width as usize);
        for x in 0..width {
            let p = rgba.get_pixel(x, y).0;
            row.push((p[0], p[1], p[2]));
        }
        matrix.push(row);
    }
    matrix
}

/// Bilinear resize a luminance matrix to new dimensions.
fn bilinear_resize(matrix: &[Vec<u8>], new_width: usize, new_height: usize) -> Vec<Vec<u8>> {
    if matrix.is_empty() || matrix[0].is_empty() || new_width == 0 || new_height == 0 {
        return Vec::new();
    }
    let src_h = matrix.len();
    let src_w = matrix[0].len();
    let mut result = Vec::with_capacity(new_height);
    for dy in 0..new_height {
        let mut row = Vec::with_capacity(new_width);
        let sy = (dy as f64 * src_h as f64 / new_height as f64).min((src_h - 1) as f64);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let y_frac = sy - sy.floor();
        for dx in 0..new_width {
            let sx = (dx as f64 * src_w as f64 / new_width as f64).min((src_w - 1) as f64);
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let x_frac = sx - sx.floor();
            let top = matrix[y0][x0] as f64 * (1.0 - x_frac) + matrix[y0][x1] as f64 * x_frac;
            let bottom = matrix[y1][x0] as f64 * (1.0 - x_frac) + matrix[y1][x1] as f64 * x_frac;
            let val = top * (1.0 - y_frac) + bottom * y_frac;
            row.push(val.round() as u8);
        }
        result.push(row);
    }
    result
}

/// Bilinear resize an RGB pixel matrix to new dimensions.
fn bilinear_resize_rgb(
    matrix: &[Vec<RgbPixel>],
    new_width: usize,
    new_height: usize,
) -> Vec<Vec<RgbPixel>> {
    if matrix.is_empty() || matrix[0].is_empty() || new_width == 0 || new_height == 0 {
        return Vec::new();
    }
    let src_h = matrix.len();
    let src_w = matrix[0].len();
    let mut result = Vec::with_capacity(new_height);
    for dy in 0..new_height {
        let mut row = Vec::with_capacity(new_width);
        let sy = (dy as f64 * src_h as f64 / new_height as f64).min((src_h - 1) as f64);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let y_frac = sy - sy.floor();
        for dx in 0..new_width {
            let sx = (dx as f64 * src_w as f64 / new_width as f64).min((src_w - 1) as f64);
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let x_frac = sx - sx.floor();
            let lerp = |a: u8, b: u8, t: f64| -> u8 {
                (a as f64 * (1.0 - t) + b as f64 * t).round() as u8
            };
            let r = lerp(
                lerp(matrix[y0][x0].0, matrix[y0][x1].0, x_frac),
                lerp(matrix[y1][x0].0, matrix[y1][x1].0, x_frac),
                y_frac,
            );
            let g = lerp(
                lerp(matrix[y0][x0].1, matrix[y0][x1].1, x_frac),
                lerp(matrix[y1][x0].1, matrix[y1][x1].1, x_frac),
                y_frac,
            );
            let b = lerp(
                lerp(matrix[y0][x0].2, matrix[y0][x1].2, x_frac),
                lerp(matrix[y1][x0].2, matrix[y1][x1].2, x_frac),
                y_frac,
            );
            row.push((r, g, b));
        }
        result.push(row);
    }
    result
}

/// Map luminance value (0-255) to char from char_map.
///
/// Luminance 0 (darkest) maps to first char, 255 (brightest) maps to last.
fn luminance_to_char(luminance: u8, char_map: &str) -> char {
    if char_map.is_empty() {
        return ' ';
    }
    let len = char_map.len();
    let idx = (luminance as usize * (len - 1)) / 255;
    char_map.as_bytes()[idx] as char
}

/// Convert luminance matrix to ASCII art string.
///
/// Image is bilinearly resized to `target_width` columns with aspect ratio
/// preserved (terminal char aspect ~2:1 accounted for). Each pixel maps to
/// a char from `char_map`.
pub fn luminance_to_ascii(matrix: &[Vec<u8>], target_width: usize, char_map: &str) -> String {
    if matrix.is_empty() || matrix[0].is_empty() || target_width == 0 {
        return String::new();
    }
    let src_h = matrix.len();
    let src_w = matrix[0].len();
    let target_height = ((target_width as f64 * src_h as f64 / src_w as f64) * 0.5)
        .ceil()
        .max(1.0) as usize;
    let resized = bilinear_resize(matrix, target_width, target_height);
    let mut lines = Vec::with_capacity(resized.len());
    for row in &resized {
        let line: String = row
            .iter()
            .map(|&lum| luminance_to_char(lum, char_map))
            .collect();
        lines.push(line);
    }
    lines.join("\n")
}

/// Load image from file and convert to ASCII art string.
///
/// `target_width` defaults to terminal width (or 80 if undetectable).
/// `char_map` defaults to [`DEFAULT_CHAR_MAP`].
pub fn image_to_ascii<P: AsRef<Path>>(
    path: P,
    target_width: Option<usize>,
    char_map: Option<&str>,
) -> Result<String, ImageError> {
    let img = image::open(path)?;
    let matrix = luminance_from_dynamic(&img);
    let cmap = char_map.unwrap_or(DEFAULT_CHAR_MAP);
    let width = target_width.unwrap_or_else(|| {
        termion::terminal_size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80)
    });
    Ok(luminance_to_ascii(&matrix, width, cmap))
}

/// Apply grayscale in-place to an RGB pixel matrix using BT.709 luminance weights.
pub fn apply_grayscale(matrix: &mut [Vec<RgbPixel>]) {
    for row in matrix.iter_mut() {
        for pixel in row.iter_mut() {
            let luma = (0.2126 * pixel.0 as f64 + 0.7152 * pixel.1 as f64 + 0.0722 * pixel.2 as f64)
                .round()
                .min(255.0) as u8;
            *pixel = (luma, luma, luma);
        }
    }
}

/// Apply negative/invert in-place to an RGB pixel matrix.
pub fn apply_negative(matrix: &mut [Vec<RgbPixel>]) {
    for row in matrix.iter_mut() {
        for pixel in row.iter_mut() {
            *pixel = (255 - pixel.0, 255 - pixel.1, 255 - pixel.2);
        }
    }
}

/// Generate 24-bit ANSI foreground color escape code.
pub fn ansi_color_code(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

/// Return the ANSI reset escape code.
pub fn ansi_reset_code() -> &'static str {
    "\x1b[0m"
}

/// Configuration for colored ASCII output.
pub struct ImageColorConfig<'a> {
    pub colored: bool,
    pub grayscale: bool,
    pub negative: bool,
    pub char_map: &'a str,
    pub target_width: Option<usize>,
}

impl<'a> Default for ImageColorConfig<'a> {
    fn default() -> Self {
        Self {
            colored: false,
            grayscale: false,
            negative: false,
            char_map: DEFAULT_CHAR_MAP,
            target_width: None,
        }
    }
}

/// Convert RGB pixel matrix to ASCII art string with color/grayscale/negative options.
///
/// Image is bilinearly resized to `target_width` (defaults to 80 if None) with
/// aspect ratio preserved. When `colored=true`, each char is wrapped in 24-bit
/// ANSI color escape codes preserving the original pixel color. When
/// `grayscale=true`, RGB values are converted to luminance before char mapping.
/// When `negative=true`, pixel values are inverted.
pub fn color_matrix_to_ascii(matrix: &[Vec<RgbPixel>], config: &ImageColorConfig) -> String {
    if matrix.is_empty() || matrix[0].is_empty() {
        return String::new();
    }
    let width = config.target_width.unwrap_or(80);
    if width == 0 {
        return String::new();
    }
    let src_h = matrix.len();
    let src_w = matrix[0].len();
    let target_height = ((width as f64 * src_h as f64 / src_w as f64) * 0.5)
        .ceil()
        .max(1.0) as usize;
    let mut working = bilinear_resize_rgb(matrix, width, target_height);
    if config.negative {
        apply_negative(&mut working);
    }
    if config.grayscale {
        apply_grayscale(&mut working);
    }
    let reset = ansi_reset_code();
    let mut lines = Vec::with_capacity(working.len());
    if config.colored {
        for row in &working {
            let line: String = row
                .iter()
                .map(|&(r, g, b)| {
                    let luma =
                        (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64).round() as u8;
                    let c = luminance_to_char(luma, config.char_map);
                    format!("{}{}{}", ansi_color_code(r, g, b), c, reset)
                })
                .collect();
            lines.push(line);
        }
    } else {
        for row in &working {
            let line: String = row
                .iter()
                .map(|&(r, g, b)| {
                    let luma =
                        (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64).round() as u8;
                    luminance_to_char(luma, config.char_map)
                })
                .collect();
            lines.push(line);
        }
    }
    lines.join("\n")
}

/// Load image from file and convert to ASCII with optional color config.
///
/// Convenience wrapper around [`load_rgb_matrix`] + [`color_matrix_to_ascii`].
pub fn image_to_colored_ascii<P: AsRef<Path>>(
    path: P,
    config: &ImageColorConfig,
) -> Result<String, ImageError> {
    let matrix = load_rgb_matrix(path)?;
    Ok(color_matrix_to_ascii(&matrix, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::bmp::BmpEncoder;
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::PngEncoder;
    use image::codecs::webp::WebPEncoder;
    use image::ColorType;
    use image::ImageEncoder;
    use image::RgbImage;

    const TEST_PNG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/img/figby.png");

    fn make_checkerboard_pixels(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_white = (x + y) % 2 == 0;
                if is_white {
                    pixels.extend_from_slice(&[255, 255, 255]);
                } else {
                    pixels.extend_from_slice(&[0, 0, 0]);
                }
            }
        }
        pixels
    }

    fn encode_temp_image(
        dir: &tempfile::TempDir,
        name: &str,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        match name.rsplit('.').next_back().unwrap() {
            "jpg" | "jpeg" => {
                let mut encoder = JpegEncoder::new(file);
                encoder
                    .encode(pixels, width, height, ColorType::Rgb8)
                    .unwrap();
            }
            "png" => {
                PngEncoder::new(file)
                    .write_image(pixels, width, height, ColorType::Rgb8)
                    .unwrap();
            }
            "bmp" => {
                let mut encoder = BmpEncoder::new(&mut file);
                encoder
                    .encode(pixels, width, height, ColorType::Rgb8)
                    .unwrap();
            }
            "webp" => {
                let encoder = WebPEncoder::new_lossless(file);
                encoder
                    .encode(pixels, width, height, ColorType::Rgb8)
                    .unwrap();
            }
            ext => panic!("unknown format: {ext}"),
        }
        path
    }

    fn test_format_load(width: u32, height: u32, filename: &str) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let pixels = make_checkerboard_pixels(width, height);
        let path = encode_temp_image(&dir, filename, &pixels, width, height);
        let matrix = load_luminance_matrix(&path)
            .unwrap_or_else(|e| panic!("failed to load {filename}: {e}"));
        assert_eq!(matrix.len(), height as usize, "{filename}: wrong height");
        assert_eq!(matrix[0].len(), width as usize, "{filename}: wrong width");
    }

    #[test]
    fn test_load_png() {
        let matrix = load_luminance_matrix(TEST_PNG).expect("failed to load PNG fixture");
        assert!(!matrix.is_empty(), "empty matrix");
        assert!(!matrix[0].is_empty(), "empty row");
    }

    #[test]
    fn test_load_jpeg() {
        test_format_load(4, 4, "test.jpg");
    }

    #[test]
    fn test_load_bmp() {
        test_format_load(2, 2, "test.bmp");
    }

    #[test]
    fn test_load_webp() {
        test_format_load(2, 2, "test.webp");
    }

    #[test]
    fn test_luminance_values_known() {
        let mut img = RgbImage::new(3, 1);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        img.put_pixel(2, 0, image::Rgb([0, 0, 255]));
        let matrix = luminance_from_dynamic(&DynamicImage::ImageRgb8(img));
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].len(), 3);
        assert!(
            matrix[0][1] > matrix[0][0],
            "green ({}) should be brighter than red ({})",
            matrix[0][1],
            matrix[0][0]
        );
        assert!(
            matrix[0][0] > matrix[0][2],
            "red ({}) should be brighter than blue ({})",
            matrix[0][0],
            matrix[0][2]
        );
    }

    #[test]
    fn test_luminance_range() {
        let matrix = load_luminance_matrix(TEST_PNG).expect("failed to load PNG fixture");
        assert!(matrix.iter().all(|row| !row.is_empty()));
    }

    #[test]
    fn test_load_nonexistent() {
        let result = load_luminance_matrix("/nonexistent/path/image.png");
        assert!(result.is_err(), "expected error for nonexistent path");
    }

    // -- luminance_to_char tests --

    #[test]
    fn test_luminance_to_char_black() {
        assert_eq!(luminance_to_char(0, DEFAULT_CHAR_MAP), ' ');
    }

    #[test]
    fn test_luminance_to_char_white() {
        assert_eq!(luminance_to_char(255, DEFAULT_CHAR_MAP), '@');
    }

    #[test]
    fn test_luminance_to_char_mid() {
        let mid_idx = DEFAULT_CHAR_MAP.len() / 2;
        let expected = DEFAULT_CHAR_MAP.as_bytes()[mid_idx] as char;
        assert_eq!(luminance_to_char(128, DEFAULT_CHAR_MAP), expected);
    }

    #[test]
    fn test_luminance_to_char_custom_map() {
        let map = "#@";
        assert_eq!(luminance_to_char(0, map), '#');
        assert_eq!(luminance_to_char(255, map), '@');
        assert_eq!(luminance_to_char(128, map), '#');
    }

    #[test]
    fn test_luminance_to_char_empty_map() {
        assert_eq!(luminance_to_char(100, ""), ' ');
    }

    #[test]
    fn test_luminance_to_char_single_char_map() {
        assert_eq!(luminance_to_char(0, "X"), 'X');
        assert_eq!(luminance_to_char(255, "X"), 'X');
    }

    // -- bilinear_resize tests --

    #[test]
    fn test_bilinear_resize_identity() {
        let matrix = vec![vec![1u8, 2], vec![3, 4]];
        let resized = bilinear_resize(&matrix, 2, 2);
        assert_eq!(resized, matrix);
    }

    #[test]
    fn test_bilinear_resize_upscale() {
        let matrix = vec![vec![0u8, 255]];
        let resized = bilinear_resize(&matrix, 4, 1);
        assert_eq!(resized.len(), 1);
        assert_eq!(resized[0].len(), 4);
        assert_eq!(resized[0][0], 0);
        assert_eq!(resized[0][3], 255);
    }

    #[test]
    fn test_bilinear_resize_downscale() {
        let matrix = vec![vec![0u8, 128, 255]];
        let resized = bilinear_resize(&matrix, 1, 1);
        assert_eq!(resized.len(), 1);
        assert_eq!(resized[0].len(), 1);
        assert!(resized[0][0] >= 120 && resized[0][0] <= 140);
    }

    #[test]
    fn test_bilinear_resize_empty() {
        assert!(bilinear_resize(&[], 10, 10).is_empty());
        assert!(bilinear_resize(&[vec![]], 10, 10).is_empty());
        assert!(bilinear_resize(&[vec![0u8]], 0, 1).is_empty());
    }

    #[test]
    fn test_bilinear_resize_single_pixel() {
        let matrix = vec![vec![123u8]];
        let resized = bilinear_resize(&matrix, 3, 3);
        assert_eq!(resized.len(), 3);
        assert!(resized.iter().all(|row| row.iter().all(|&v| v == 123)));
    }

    // -- luminance_to_ascii tests --

    #[test]
    fn test_luminance_to_ascii_all_white() {
        let matrix = vec![vec![255u8; 3], vec![255u8; 3]];
        let result = luminance_to_ascii(&matrix, 3, DEFAULT_CHAR_MAP);
        assert!(!result.is_empty(), "output should not be empty");
        for line in result.lines() {
            assert_eq!(line.len(), 3);
            assert!(line.chars().all(|c| c == '@'));
        }
    }

    #[test]
    fn test_luminance_to_ascii_all_black() {
        let matrix = vec![vec![0u8; 3], vec![0u8; 3]];
        let result = luminance_to_ascii(&matrix, 3, DEFAULT_CHAR_MAP);
        assert!(!result.is_empty());
        for line in result.lines() {
            assert_eq!(line.len(), 3);
            assert!(line.chars().all(|c| c == ' '));
        }
    }

    #[test]
    fn test_luminance_to_ascii_custom_map() {
        let matrix = vec![vec![0u8, 255]];
        let result = luminance_to_ascii(&matrix, 2, "#@");
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "#@");
    }

    #[test]
    fn test_luminance_to_ascii_empty_matrix() {
        assert_eq!(luminance_to_ascii(&[], 10, DEFAULT_CHAR_MAP), "");
        assert_eq!(luminance_to_ascii(&[vec![]], 10, DEFAULT_CHAR_MAP), "");
    }

    #[test]
    fn test_luminance_to_ascii_zero_width() {
        let matrix = vec![vec![0u8]];
        assert_eq!(luminance_to_ascii(&matrix, 0, DEFAULT_CHAR_MAP), "");
    }

    // -- image_to_ascii tests --

    #[test]
    fn test_image_to_ascii_png() {
        let result =
            image_to_ascii(TEST_PNG, Some(40), None).expect("failed to convert PNG to ASCII");
        assert!(!result.is_empty(), "ASCII output should not be empty");
        let lines: Vec<&str> = result.lines().collect();
        assert!(!lines.is_empty(), "should have at least one row");
        for line in &lines {
            assert!(!line.is_empty(), "each row should be non-empty");
            for c in line.chars() {
                assert!(
                    DEFAULT_CHAR_MAP.contains(c),
                    "char '{c}' not in default map"
                );
            }
        }
    }

    #[test]
    fn test_image_to_ascii_custom_map() {
        let result =
            image_to_ascii(TEST_PNG, Some(40), Some("#@")).expect("failed to convert PNG to ASCII");
        assert!(!result.is_empty());
        for c in result.chars() {
            if c != '\n' {
                assert!(c == '#' || c == '@', "unexpected char '{c}' in output");
            }
        }
    }

    #[test]
    fn test_image_to_ascii_width() {
        let narrow =
            image_to_ascii(TEST_PNG, Some(20), None).expect("failed to convert at width 20");
        let wide = image_to_ascii(TEST_PNG, Some(80), None).expect("failed to convert at width 80");
        let narrow_lines: Vec<&str> = narrow.lines().collect();
        let wide_lines: Vec<&str> = wide.lines().collect();
        assert!(
            narrow_lines[0].len() < wide_lines[0].len(),
            "narrow output should be narrower than wide output"
        );
    }

    #[test]
    fn test_image_to_ascii_nonexistent() {
        let result = image_to_ascii("/nonexistent/path/image.png", Some(40), None);
        assert!(result.is_err(), "expected error for nonexistent path");
    }

    #[test]
    fn test_image_to_ascii_temp_image() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let pixels = vec![255u8; 2 * 2 * 3];
        let path = encode_temp_image(&dir, "test.png", &pixels, 2, 2);
        let result = image_to_ascii(&path, Some(4), None).expect("failed to convert temp image");
        assert!(!result.is_empty());
    }

    // -- RGB matrix tests --

    #[test]
    fn test_rgb_matrix_load() {
        let matrix = load_rgb_matrix(TEST_PNG).expect("failed to load PNG fixture");
        assert!(!matrix.is_empty(), "empty matrix");
        assert!(!matrix[0].is_empty(), "empty row");
    }

    #[test]
    fn test_rgb_pixel_preserved() {
        let mut img = image::RgbImage::new(3, 1);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        img.put_pixel(2, 0, image::Rgb([0, 0, 255]));
        let matrix = rgb_from_dynamic(&image::DynamicImage::ImageRgb8(img));
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].len(), 3);
        assert_eq!(matrix[0][0], (255, 0, 0));
        assert_eq!(matrix[0][1], (0, 255, 0));
        assert_eq!(matrix[0][2], (0, 0, 255));
    }

    #[test]
    fn test_apply_grayscale_inplace() {
        let mut matrix = vec![
            vec![(100, 150, 200), (10, 20, 30)],
            vec![(0, 0, 0), (255, 255, 255)],
        ];
        apply_grayscale(&mut matrix);
        for row in &matrix {
            for &(r, g, b) in row {
                assert_eq!(r, g, "R should equal G after grayscale");
                assert_eq!(g, b, "G should equal B after grayscale");
            }
        }
        assert_eq!(matrix[0][0].0, 147, "luminance of (100,150,200)");
        assert_eq!(matrix[0][1].0, 19, "luminance of (10,20,30)");
        assert_eq!(matrix[1][0].0, 0, "black pixel stays 0");
        assert_eq!(matrix[1][1].0, 255, "white pixel stays 255");
    }

    #[test]
    fn test_apply_negative_inplace() {
        let mut matrix = vec![
            vec![(100, 150, 200), (0, 128, 255)],
            vec![(255, 255, 255), (0, 0, 0)],
        ];
        apply_negative(&mut matrix);
        assert_eq!(matrix[0][0], (155, 105, 55));
        assert_eq!(matrix[0][1], (255, 127, 0));
        assert_eq!(matrix[1][0], (0, 0, 0));
        assert_eq!(matrix[1][1], (255, 255, 255));
    }

    #[test]
    fn test_ansi_color_code_format() {
        assert_eq!(ansi_color_code(255, 0, 0), "\x1b[38;2;255;0;0m");
        assert_eq!(ansi_color_code(0, 255, 0), "\x1b[38;2;0;255;0m");
        assert_eq!(ansi_color_code(0, 0, 255), "\x1b[38;2;0;0;255m");
        assert_eq!(ansi_color_code(123, 45, 67), "\x1b[38;2;123;45;67m");
    }

    #[test]
    fn test_ansi_reset_code() {
        assert_eq!(ansi_reset_code(), "\x1b[0m");
    }

    #[test]
    fn test_colored_ascii_output() {
        let matrix = vec![
            vec![(255, 0, 0), (255, 0, 0)],
            vec![(255, 0, 0), (255, 0, 0)],
        ];
        let config = ImageColorConfig {
            colored: true,
            target_width: Some(2),
            ..Default::default()
        };
        let result = color_matrix_to_ascii(&matrix, &config);
        assert!(!result.is_empty(), "output should not be empty");
        for line in result.lines() {
            assert!(!line.is_empty());
            assert!(
                line.contains("\x1b[38;2;255;0;0m"),
                "line should contain red ANSI code"
            );
            assert!(line.contains("\x1b[0m"), "line should contain reset code");
            for c in line.chars() {
                if c != '\n' {
                    assert!(
                        DEFAULT_CHAR_MAP.contains(c)
                            || c == '\x1b'
                            || c == '['
                            || c == 'm'
                            || c == ';'
                            || c.is_ascii_digit(),
                        "unexpected char '{c}' in colored output"
                    );
                }
            }
        }
    }

    #[test]
    fn test_colored_ascii_grayscale_flag() {
        let matrix = vec![vec![(255, 0, 0), (0, 255, 0)]];
        let config = ImageColorConfig {
            grayscale: true,
            colored: false,
            target_width: Some(2),
            ..Default::default()
        };
        let result = color_matrix_to_ascii(&matrix, &config);
        assert!(!result.is_empty());
        assert!(
            !result.contains("\x1b"),
            "output should not contain ANSI codes when colored=false"
        );
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].is_empty());
    }

    #[test]
    fn test_colored_ascii_negative_flag() {
        let matrix = vec![vec![(100, 150, 200), (0, 0, 0)]];
        let config = ImageColorConfig {
            negative: true,
            colored: true,
            target_width: Some(2),
            ..Default::default()
        };
        let result = color_matrix_to_ascii(&matrix, &config);
        assert!(!result.is_empty());
        assert!(
            result.contains("\x1b[38;2;155;105;55m"),
            "should contain inverted color (155,105,55)"
        );
        assert!(
            result.contains("\x1b[38;2;255;255;255m"),
            "should contain inverted black as white"
        );
    }

    #[test]
    fn test_color_bilinear_resize() {
        let matrix = vec![
            vec![(255, 0, 0), (0, 255, 0)],
            vec![(0, 0, 255), (255, 255, 255)],
        ];
        let resized = bilinear_resize_rgb(&matrix, 4, 4);
        assert_eq!(resized.len(), 4);
        assert_eq!(resized[0].len(), 4);
        assert_eq!(resized[0][0], (255, 0, 0), "top-left should be original");
        assert_eq!(
            resized[3][3],
            (255, 255, 255),
            "bottom-right should be original"
        );
        let mid = resized[2][2];
        assert!(
            mid.0 > 0 && mid.1 > 0 && mid.2 > 0,
            "interior pixel should have all channels positive"
        );
    }

    #[test]
    fn test_rgb_load_nonexistent() {
        let result = load_rgb_matrix::<&str>("/nonexistent/path/image.png");
        assert!(result.is_err(), "expected error for nonexistent path");
    }
}
