use image::{DynamicImage, ImageError};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::bmp::BmpEncoder;
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::webp::WebPEncoder;
    use image::ColorType;
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
}
