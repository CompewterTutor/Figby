#![doc = "Figby — Rust port of FIGlet (Frank, Ian & Glenn's Letters)\n\nRenders text in large ASCII art characters using FIGfont (.flf)\nand TOIlet (.tlf) font files with kerning, smushing, and multi-byte\ncharacter support."]

pub mod control;
pub mod font;
pub mod font_gen;
pub mod image_input;
pub mod input;
pub mod render;
pub mod smush;
pub mod template;
pub mod tui;
