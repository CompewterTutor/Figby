use gif::{Encoder as GifEncoder, Repeat};
use image::ImageEncoder;
use ratatui::style::Color;

use crate::tui::canvas::CanvasCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Txt,
    Gif,
    Apng,
}

#[derive(Debug)]
pub enum ExportError {
    IoError(String),
    GifError(String),
    PngError(String),
    InvalidCells(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::IoError(s) => write!(f, "IO error: {s}"),
            ExportError::GifError(s) => write!(f, "GIF error: {s}"),
            ExportError::PngError(s) => write!(f, "PNG/APNG error: {s}"),
            ExportError::InvalidCells(s) => write!(f, "Invalid cells: {s}"),
        }
    }
}

impl std::error::Error for ExportError {}

fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => xterm_to_rgb(i),
        Color::Black => (0, 0, 0),
        Color::Red => (255, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (255, 255, 0),
        Color::Blue => (0, 0, 255),
        Color::Magenta => (255, 0, 255),
        Color::Cyan => (0, 255, 255),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (64, 64, 64),
        Color::LightRed => (255, 128, 128),
        Color::LightGreen => (128, 255, 128),
        Color::LightYellow => (255, 255, 128),
        Color::LightBlue => (128, 128, 255),
        Color::LightMagenta => (255, 128, 255),
        Color::LightCyan => (128, 255, 255),
        Color::White => (255, 255, 255),
        _ => (255, 255, 255),
    }
}

fn xterm_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        16..=231 => {
            let n = index - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            let cube = |v: u8| -> u8 {
                match v {
                    0 => 0,
                    1 => 95,
                    2 => 135,
                    3 => 175,
                    4 => 215,
                    5 => 255,
                    _ => 0,
                }
            };
            (cube(r), cube(g), cube(b))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

const BITMAP_FONT_8X16: [u8; 1520] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x36, 0x36, 0x36, 0x7F, 0x36, 0x36, 0x7F, 0x36, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x08, 0x08, 0x3E, 0x6B, 0x6B, 0x3E, 0x0E, 0x1E, 0x36, 0x36, 0x1C, 0x08, 0x08, 0x00, 0x00,
    0x00, 0x00, 0x63, 0x66, 0x6C, 0x18, 0x30, 0x60, 0x66, 0xCE, 0x8E, 0x06, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1C, 0x36, 0x36, 0x1C, 0x3B, 0x6E, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0C, 0x18, 0x18, 0x30, 0x30, 0x30, 0x30, 0x18, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x30, 0x18, 0x18, 0x0C, 0x0C, 0x0C, 0x0C, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x03, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xC0, 0x80, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x66, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x06, 0x06, 0x1C, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0x4C, 0x0C, 0x0C, 0x7E, 0x0C, 0x0C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x60, 0x60, 0x7C, 0x66, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1C, 0x30, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x66, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0C, 0x38, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x06, 0x06, 0x1E, 0x36, 0x36, 0x36, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x60, 0x60, 0x60, 0x7E, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x60, 0x60, 0x60, 0x7E, 0x60, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x60, 0x60, 0x60, 0x6E, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x6C, 0x6C, 0x78, 0x78, 0x6C, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x76, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x76, 0x76, 0x7E, 0x7E, 0x6E, 0x6E, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x6E, 0x3E, 0x06, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x66, 0x60, 0x60, 0x3C, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x6E, 0x7E, 0x7E, 0x76, 0x76, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x3C, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x7E, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x80, 0xC0, 0xC0, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00,
    0x00, 0x00, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x60, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x06, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0x66, 0x7E, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x1C, 0x36, 0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00,
    0x00, 0x00, 0x60, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0C, 0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38, 0x00, 0x00,
    0x00, 0x00, 0x60, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x06, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x60, 0x60, 0x3C, 0x06, 0x06, 0x7C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x30, 0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x36, 0x1C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x76, 0x76, 0x7E, 0x7E, 0x6C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0E, 0x18, 0x18, 0x18, 0x70, 0x18, 0x18, 0x18, 0x18, 0x0E, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x70, 0x18, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x18, 0x18, 0x70, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x63, 0x77, 0x3E, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn rasterize_char(
    ch: char,
    fg: Option<Color>,
    bg: Option<Color>,
    scale: u8,
) -> Vec<Vec<(u8, u8, u8, u8)>> {
    let s = scale.max(1) as usize;
    let cw = 8 * s;
    let char_h = 16 * s;

    let fg_rgb = fg.map(color_to_rgb);
    let bg_rgb = bg.map(color_to_rgb);

    let char_idx = (ch as usize).wrapping_sub(32);

    let mut result = vec![vec![(0u8, 0u8, 0u8, 0u8); cw]; char_h];

    for (y, row) in result.iter_mut().enumerate().take(char_h) {
        for (x, pixel) in row.iter_mut().enumerate().take(cw) {
            let src_y = y / s;
            let src_x = x / s;
            let pixel_on = char_idx < 95
                && ((BITMAP_FONT_8X16[char_idx * 16 + src_y] >> (7 - src_x)) & 1) == 1;

            if pixel_on {
                let (r, g, b) = fg_rgb.unwrap_or((0, 0, 0));
                *pixel = (r, g, b, 255);
            } else if let Some((r, g, b)) = bg_rgb {
                *pixel = (r, g, b, 255);
            } else {
                *pixel = (0, 0, 0, 0);
            }
        }
    }

    result
}

fn render_frame(
    cells: &[Vec<CanvasCell>],
    scale: u8,
    transparent: bool,
) -> Vec<Vec<(u8, u8, u8, u8)>> {
    let sc = scale as usize;
    let char_w = 8 * sc;
    let char_h = 16 * sc;

    if cells.is_empty() || cells[0].is_empty() {
        return Vec::new();
    }

    let w = cells[0].len() * char_w;
    let h = cells.len() * char_h;

    let mut pixels = vec![vec![(0u8, 0u8, 0u8, 0u8); w]; h];

    for (cy, row) in cells.iter().enumerate() {
        for (cx, cell) in row.iter().enumerate() {
            if transparent && cell.ch == ' ' {
                continue;
            }
            let ch = if cell.ch as u32 >= 32 && cell.ch as u32 <= 126 {
                cell.ch
            } else if cell.ch == ' ' || cell.ch.is_ascii() {
                ' '
            } else {
                '?'
            };
            let char_pixels = rasterize_char(ch, cell.fg, cell.bg, scale);
            let base_y = cy * char_h;
            let base_x = cx * char_w;
            for dy in 0..char_h {
                for dx in 0..char_w {
                    if base_y + dy < h && base_x + dx < w {
                        pixels[base_y + dy][base_x + dx] = char_pixels[dy][dx];
                    }
                }
            }
        }
    }

    pixels
}

pub fn export_cells_to_png(
    cells: &[Vec<CanvasCell>],
    font_size: u8,
) -> Result<Vec<u8>, ExportError> {
    let pixels = render_frame(cells, font_size, false);
    if pixels.is_empty() || pixels[0].is_empty() {
        return Err(ExportError::InvalidCells("empty cell grid".to_string()));
    }
    let h = pixels.len() as u32;
    let w = pixels[0].len() as u32;

    let mut buf = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        let raw: Vec<u8> = pixels
            .iter()
            .flat_map(|row| row.iter().flat_map(|(r, g, b, a)| vec![*r, *g, *b, *a]))
            .collect();
        encoder
            .write_image(&raw, w, h, image::ColorType::Rgba8)
            .map_err(|e| ExportError::IoError(e.to_string()))?;
    }
    Ok(buf)
}

pub fn export_cells_to_png_with_alpha(
    cells: &[Vec<CanvasCell>],
    font_size: u8,
    transparent: bool,
) -> Result<Vec<u8>, ExportError> {
    let pixels = render_frame(cells, font_size, transparent);
    if pixels.is_empty() || pixels[0].is_empty() {
        return Err(ExportError::InvalidCells("empty cell grid".to_string()));
    }
    let h = pixels.len() as u32;
    let w = pixels[0].len() as u32;

    let mut buf = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        let raw: Vec<u8> = pixels
            .iter()
            .flat_map(|row| row.iter().flat_map(|(r, g, b, a)| vec![*r, *g, *b, *a]))
            .collect();
        encoder
            .write_image(&raw, w, h, image::ColorType::Rgba8)
            .map_err(|e| ExportError::IoError(e.to_string()))?;
    }
    Ok(buf)
}

pub fn export_cells_to_txt(cells: &[Vec<CanvasCell>]) -> String {
    cells
        .iter()
        .map(|row| row.iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn export_cells_to_gif(
    frame_cells: &[Vec<Vec<CanvasCell>>],
    frame_delays_cs: &[u16],
    font_size: u8,
    loop_count: u16,
) -> Result<Vec<u8>, ExportError> {
    if frame_cells.is_empty() {
        return Err(ExportError::InvalidCells("no frames".to_string()));
    }

    let pixels = render_frame(&frame_cells[0], font_size, false);
    if pixels.is_empty() || pixels[0].is_empty() {
        return Err(ExportError::InvalidCells("empty cell grid".to_string()));
    }
    let h = pixels.len() as u16;
    let w = pixels[0].len() as u16;

    let mut buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut buf, w, h, &[])
            .map_err(|e| ExportError::GifError(e.to_string()))?;
        let repeat = if loop_count == 0 {
            Repeat::Infinite
        } else {
            Repeat::Finite(loop_count)
        };
        encoder
            .set_repeat(repeat)
            .map_err(|e| ExportError::GifError(e.to_string()))?;

        for (i, cells) in frame_cells.iter().enumerate() {
            let frame_pixels = render_frame(cells, font_size, false);
            let raw: Vec<u8> = frame_pixels
                .iter()
                .flat_map(|row| {
                    row.iter().flat_map(|(r, g, b, a)| {
                        if *a == 0 {
                            vec![255u8, 255, 255]
                        } else {
                            vec![*r, *g, *b]
                        }
                    })
                })
                .collect();
            let delay = frame_delays_cs.get(i).copied().unwrap_or(10);
            let mut frame = gif::Frame::from_rgb(w, h, &raw);
            frame.delay = delay;
            encoder
                .write_frame(&frame)
                .map_err(|e| ExportError::GifError(e.to_string()))?;
        }
    }
    Ok(buf)
}

pub fn export_cells_to_apng(
    frame_cells: &[Vec<Vec<CanvasCell>>],
    frame_delays_cs: &[u16],
    font_size: u8,
    loop_count: u16,
) -> Result<Vec<u8>, ExportError> {
    if frame_cells.is_empty() {
        return Err(ExportError::InvalidCells("no frames".to_string()));
    }

    let pixels = render_frame(&frame_cells[0], font_size, false);
    if pixels.is_empty() || pixels[0].is_empty() {
        return Err(ExportError::InvalidCells("empty cell grid".to_string()));
    }
    let h = pixels.len() as u32;
    let w = pixels[0].len() as u32;

    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .set_animated(frame_cells.len() as u32, loop_count.into())
            .map_err(|e| ExportError::PngError(e.to_string()))?;

        let mut writer = encoder
            .write_header()
            .map_err(|e| ExportError::PngError(e.to_string()))?;

        for (i, cells) in frame_cells.iter().enumerate() {
            let frame_pixels = render_frame(cells, font_size, false);
            let raw: Vec<u8> = frame_pixels
                .iter()
                .flat_map(|row| row.iter().flat_map(|(r, g, b, a)| vec![*r, *g, *b, *a]))
                .collect();
            let delay = frame_delays_cs.get(i).copied().unwrap_or(10);
            writer
                .set_frame_delay(delay, 100)
                .map_err(|e| ExportError::PngError(e.to_string()))?;
            writer
                .write_image_data(&raw)
                .map_err(|e| ExportError::PngError(e.to_string()))?;
        }

        writer
            .finish()
            .map_err(|e| ExportError::PngError(e.to_string()))?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::canvas::CanvasCell;

    fn make_buffer(
        rows: usize,
        cols: usize,
        ch: char,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> Vec<Vec<CanvasCell>> {
        (0..rows)
            .map(|_| (0..cols).map(|_| CanvasCell { ch, fg, bg }).collect())
            .collect()
    }

    #[test]
    fn test_output_txt_simple() {
        let cells = vec![
            vec![
                CanvasCell {
                    ch: 'A',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'B',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'C',
                    fg: None,
                    bg: None,
                },
            ],
            vec![
                CanvasCell {
                    ch: 'D',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'E',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'F',
                    fg: None,
                    bg: None,
                },
            ],
        ];
        assert_eq!(export_cells_to_txt(&cells), "ABC\nDEF");
    }

    #[test]
    fn test_output_txt_color_stripped() {
        let cells = vec![vec![
            CanvasCell {
                ch: 'X',
                fg: Some(Color::Red),
                bg: None,
            },
            CanvasCell {
                ch: 'Y',
                fg: Some(Color::Green),
                bg: None,
            },
        ]];
        let txt = export_cells_to_txt(&cells);
        assert_eq!(txt, "XY");
        assert!(!txt.contains('\x1b'));
    }

    #[test]
    fn test_output_png_rasterized_size() {
        let cells = make_buffer(5, 10, 'A', None, None);
        let png_bytes = export_cells_to_png(&cells, 2).expect("PNG export should succeed");
        let img = image::load_from_memory(&png_bytes).expect("should decode PNG");
        assert_eq!(img.width(), 10 * 8 * 2);
        assert_eq!(img.height(), 5 * 16 * 2);
    }

    #[test]
    fn test_output_png_roundtrip() {
        let cells = vec![
            vec![
                CanvasCell {
                    ch: 'A',
                    fg: Some(Color::Red),
                    bg: None,
                },
                CanvasCell {
                    ch: 'B',
                    fg: None,
                    bg: Some(Color::Blue),
                },
            ],
            vec![
                CanvasCell {
                    ch: ' ',
                    fg: None,
                    bg: None,
                },
                CanvasCell {
                    ch: 'X',
                    fg: Some(Color::Green),
                    bg: Some(Color::White),
                },
            ],
        ];
        let png_bytes = export_cells_to_png(&cells, 1).expect("PNG export should succeed");
        let img = image::load_from_memory(&png_bytes).expect("should decode PNG");
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 32);
        let rgb = img.to_rgb8();
        let pixel = rgb.get_pixel(3, 2);
        assert_eq!(pixel[0], 255);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);
    }

    #[test]
    fn test_output_gif_single_frame() {
        let cells = make_buffer(2, 2, 'A', Some(Color::Red), None);
        let gif_bytes =
            export_cells_to_gif(&[cells], &[10], 1, 0).expect("GIF export should succeed");
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut reader = decoder
            .read_info(&gif_bytes[..])
            .expect("should decode GIF");
        let info = reader.next_frame_info().unwrap().unwrap();
        assert_eq!(info.width, 16);
        assert_eq!(info.height, 32);
    }

    #[test]
    fn test_output_gif_multi_frame() {
        let cells_a = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let cells_b = make_buffer(1, 1, 'B', Some(Color::Blue), None);
        let gif_bytes = export_cells_to_gif(&[cells_a, cells_b], &[10, 20], 1, 0)
            .expect("GIF export should succeed");
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut reader = decoder
            .read_info(&gif_bytes[..])
            .expect("should decode GIF");
        assert_eq!(reader.next_frame_info().unwrap().unwrap().delay, 10);
        assert_eq!(reader.next_frame_info().unwrap().unwrap().delay, 20);
    }

    #[test]
    fn test_export_format_equality() {
        assert_eq!(ExportFormat::Png, ExportFormat::Png);
        assert_ne!(ExportFormat::Png, ExportFormat::Gif);
        assert_ne!(ExportFormat::Txt, ExportFormat::Gif);
    }

    #[test]
    fn test_export_cells_to_txt_non_ascii_fallback() {
        let cells = vec![vec![CanvasCell {
            ch: '\u{2603}',
            fg: None,
            bg: None,
        }]];
        let txt = export_cells_to_txt(&cells);
        assert_eq!(txt, "\u{2603}");
    }

    #[test]
    fn test_xterm_to_rgb_basic() {
        assert_eq!(xterm_to_rgb(0), (0, 0, 0));
        assert_eq!(xterm_to_rgb(9), (255, 0, 0));
        assert_eq!(xterm_to_rgb(15), (255, 255, 255));
        assert_eq!(xterm_to_rgb(16), (0, 0, 0));
        assert_eq!(xterm_to_rgb(46), (0, 255, 0));
        assert_eq!(xterm_to_rgb(231), (255, 255, 255));
        assert_eq!(xterm_to_rgb(232), (8, 8, 8));
        assert_eq!(xterm_to_rgb(255), (238, 238, 238));
    }

    #[test]
    fn test_png_with_alpha_opaque_matches_png() {
        let cells = make_buffer(2, 3, 'X', Some(Color::Red), None);
        let opaque = export_cells_to_png(&cells, 1).expect("PNG export");
        let alpha = export_cells_to_png_with_alpha(&cells, 1, false).expect("PNG with alpha");
        let img_opaque = image::load_from_memory(&opaque).expect("decode opaque");
        let img_alpha = image::load_from_memory(&alpha).expect("decode alpha");
        assert_eq!(img_opaque.width(), img_alpha.width());
        assert_eq!(img_opaque.height(), img_alpha.height());
    }

    #[test]
    fn test_output_gif_finite_loop() {
        let cells = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let gif_bytes = export_cells_to_gif(std::slice::from_ref(&cells), &[10], 1, 3)
            .expect("GIF export should succeed");
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut reader = decoder
            .read_info(&gif_bytes[..])
            .expect("should decode GIF");
        // Verify we can read at least one frame
        assert!(reader.next_frame_info().is_ok());
    }

    #[test]
    fn test_output_gif_infinite_loop() {
        let cells = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let gif_bytes = export_cells_to_gif(std::slice::from_ref(&cells), &[10], 1, 0)
            .expect("GIF export should succeed");
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut reader = decoder
            .read_info(&gif_bytes[..])
            .expect("should decode GIF");
        assert!(reader.next_frame_info().is_ok());
    }

    #[test]
    fn test_png_with_alpha_transparent_skip_space() {
        let cells = vec![
            vec![CanvasCell {
                ch: 'A',
                fg: Some(Color::Red),
                bg: None,
            }],
            vec![CanvasCell {
                ch: ' ',
                fg: None,
                bg: None,
            }],
        ];
        let bytes = export_cells_to_png_with_alpha(&cells, 1, true).expect("PNG with alpha");
        let img = image::load_from_memory(&bytes).expect("decode PNG");
        let rgba = img.to_rgba8();
        let space_row_y = 16u32;
        let any_x = 0u32;
        let pixel = rgba.get_pixel(any_x, space_row_y);
        assert_eq!(
            pixel[3], 0,
            "space cell in transparent mode should have alpha=0"
        );
    }

    #[test]
    fn test_output_apng_single_frame() {
        let cells = make_buffer(2, 2, 'A', Some(Color::Red), None);
        let apng_bytes =
            export_cells_to_apng(&[cells], &[10], 1, 0).expect("APNG export should succeed");
        let img = image::load_from_memory(&apng_bytes).expect("should decode PNG");
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_output_apng_multi_frame_timing() {
        use std::io::{BufReader, Cursor};
        let cells_a = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let cells_b = make_buffer(1, 1, 'B', Some(Color::Blue), None);
        let apng_bytes = export_cells_to_apng(&[cells_a, cells_b], &[10, 20], 1, 0)
            .expect("APNG export should succeed");
        let cursor = Cursor::new(&apng_bytes[..]);
        let decoder = png::Decoder::new(BufReader::new(cursor));
        let mut reader = decoder.read_info().expect("should decode APNG header");
        let buf_size = reader.output_buffer_size().unwrap_or(1024);
        let mut buf = vec![0u8; buf_size];
        // Read first frame
        reader.next_frame(&mut buf).expect("should read frame 1");
        // Get second frame info then read it
        let fc = reader
            .next_frame_info()
            .expect("should have frame 2 control");
        assert_eq!(fc.delay_num, 20);
        assert_eq!(fc.delay_den, 100);
        reader.next_frame(&mut buf).expect("should read frame 2");
    }

    #[test]
    fn test_output_apng_infinite_loop() {
        use std::io::{BufReader, Cursor};
        let cells = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let apng_bytes =
            export_cells_to_apng(&[cells], &[10], 1, 0).expect("APNG export should succeed");
        let cursor = Cursor::new(&apng_bytes[..]);
        let decoder = png::Decoder::new(BufReader::new(cursor));
        let reader = decoder.read_info().expect("should decode APNG header");
        let ac = reader
            .info()
            .animation_control
            .as_ref()
            .expect("APNG should have animation control");
        assert_eq!(ac.num_plays, 0);
    }

    #[test]
    fn test_output_apng_finite_loop() {
        use std::io::{BufReader, Cursor};
        let cells = make_buffer(1, 1, 'A', Some(Color::Red), None);
        let apng_bytes =
            export_cells_to_apng(&[cells], &[10], 1, 3).expect("APNG export should succeed");
        let cursor = Cursor::new(&apng_bytes[..]);
        let decoder = png::Decoder::new(BufReader::new(cursor));
        let reader = decoder.read_info().expect("should decode APNG header");
        let ac = reader
            .info()
            .animation_control
            .as_ref()
            .expect("APNG should have animation control");
        assert_eq!(ac.num_plays, 3);
    }

    #[test]
    fn test_output_apng_empty_frames_error() {
        let result = export_cells_to_apng(&[], &[], 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_format_apng() {
        assert_eq!(ExportFormat::Apng, ExportFormat::Apng);
        assert_ne!(ExportFormat::Apng, ExportFormat::Png);
        assert_ne!(ExportFormat::Apng, ExportFormat::Gif);
        assert_ne!(ExportFormat::Apng, ExportFormat::Txt);
    }
}
