# Figby

> Rust rewrite of FIGlet — ASCII art banner generator

Figby renders text as large ASCII art characters using FIGfont (.flf) and
TOIlet (.tlf) font files. This is a **modern Rust port** of FIGlet 2.2.5
preserving all features: kerning, smushing, multi-byte support, control files,
and the full CLI interface.

Original C source lives in the repo root for reference; the Rust port lives
in `figby-rs/`.

## Quick Start

```bash
cargo run -p figby -- -f fonts/standard "Hello, world!"
```

## Project Status

Active development — porting FIGlet 2.2.5 to safe, idiomatic Rust.

See [docs/todo.md](docs/todo.md) for current milestone and task tracking.


## Future Enhancements

- Ascii Image Generator library in rust as a port from the wonderful: git@github.com:TheZoraiz/ascii-image-converter.git

- Ability to import a regular old font file and generate a figlet font

- TUI with ratatui to edit figlet fonts

- Overlay figlet fonts on ascii images for banner


## License

New BSD License (same as FIGlet 2.2.5)
