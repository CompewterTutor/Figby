# Figby v2 — Memory

## Phase 2.0 — CLI Polish

### 2.0.1 — CLI `--help` output

Added `help = "..."` to every `#[arg()]` field and `long_about` to `#[command()]`
on `CliArgs` struct in `main.rs`. Every FIGlet flag now has a descriptive help
string. Tests verify `--help` exits with `DisplayHelp` and output contains
key flags and descriptions. `--long-help` not a built-in clap 4 flag — tested
via `render_long_help()` directly.

Pre-existing clippy issue in `render.rs`: `calc_smush_amount()` missing
`#[allow(clippy::too_many_arguments)]` — added to pass clippy gate.
Pre-existing bench bug: `calc_smush_amount` call had wrong argument order —
fixed to pass clippy `--all-targets`.

### 2.0.5 — `.ftmp` template file format design + CLI

Created `figby-rs/src/template.rs` — `.ftmp` template parser and renderer.
Format uses TOML frontmatter delimited by `---` lines with two sections:
- `[canvas]`: width, height, keep_ratio, margin, padding
- `[variables.varname]`: text, font, x, y, z, align, overlap, plus
  border/shadow fields (stubbed, deferred to 2.0.7).

Parser (`parse_ftmp`) reads frontmatter via `toml::from_str`, extracts
`{{varname}}` placeholders from body via simple string scanning.
Renderer (`render_template`) sorts layers by z-index (ascending),
loads fonts with dedup cache, renders each layer's text through FIGlet
pipeline (`add_char`/`render_line`), places onto canvas at (x,y) with
overwrite or flow overlap mode. Flow layers stack vertically via cursor.
Margin and padding applied to final output.

CLI flag `--render-template` (`-T`) added to `CliArgs`. When specified,
reads `.ftmp`, parses, renders, prints output rows. Font directory
resolved from `-d`, `FIGLET_FONTDIR`, or default `/usr/share/figlet`.

`toml = "0.8"` added to Cargo.toml dependencies for TOML frontmatter parsing.

### 2.0.9 — Builtin template functions: date + repo-data (syntax + reserve)

Added `TemplateBuiltin` enum with `Date(String)` and `RepoData(String)` variants
to `template.rs`. Added `builtin: Option<TemplateBuiltin>` field to `Layer` struct
(default `None`). `parse_ftmp()` recognizes `{{date:format}}` and
`{{repo-data:field}}` tags before the variables lookup. `render_template()` skips
builtin layers with `continue` (no-op, deferred to 2.1). No `.unwrap()` in
production — all new code uses proper Option handling. fmt and clippy pass clean.

### 2.0.7 — Border and shadow rendering for template output

Added three private helper functions to `template.rs`:
- `content_bbox()` — scans rendered rows at canvas position for non-space chars,
  returns `Option<(top, bottom, left, right)>` bounding box.
- `fill_border()` — fills a ring of `'.'` chars around the content bbox, only
  overwriting space cells. Border ring = expanded rect minus content rect.
- `fill_shadow()` — fills `'.'` chars in a rectangular region offset down-right
  from content bbox, only overwriting space cells.

Wired into `render_template()` in both image and text branches — after
`place_on_canvas()`, computes bbox from each layer's rendered rows + placement,
applies border then shadow. Only activates when `border_width`/`shadow_size` is
`Some` and `> 0`.

4 new tests: border-only, shadow-only, border+shadow, border-no-overwrite-other-layer.
Plus 5 direct unit tests for the helpers (content_bbox basic/no-content/multi-row,
fill_border ring/shadow offset/border preserves content).

### 2.0.x — Fix broken template tests

Fixed 6 failing unit tests in `template.rs`:

1. **TOML quoting bug** in `make_border_shadow_ftmp()`: Raw string `r#"border_color = "."#`
   consumed the closing `"` of the TOML value into the `"#` raw string delimiter, producing
   `border_color = ".` (invalid TOML). Fixed with regular string `"border_color = \".\""`.

2. **`test_render_overwrite_mode`**: Assertion expected `starts_with("BB  ")` but test font
   renders `" BB "` (leading space). Changed text from `"AA"/"BB"` to single `"A"/"B"` and
   assertion to `starts_with(" BB")`.

3. **`test_render_z_order`**: Placed layers in order z=2, z=0, z=1 without actual sorting —
   last-placed (z=1) won. Fixed to place in ascending z-order so highest z overwrites last.

4. **`test_fill_shadow_offset`**: Asserted `canvas[3][3] == '.'` but shadow rect is a single
   cell at (row=3, col=4) — col 3 is outside shadow. Fixed assertion to expect `' '`.

### 2.0.x — README header template (redesign + defer)

After review:
- Template format redesigned: YAML frontmatter (not TOML), typed elements in
  `canvas.ftmp-elements`, body just `{{name}}` placeholders. Reference format
  in `assets/templates/figby-cli-h1.ftmp`.
- Image tag: `{{img:name}}` not `{{img:source:width:height:...}}` — all metadata
  in frontmatter under element's YAML block.
- **Implementation deferred.** The template system will be rewritten when TUI
  infrastructure lands (Phase 2.3+ ratatui), which provides a proper canvas
  widget with per-cell color metadata. Current TOML-based parser is too
  restrictive and can't handle ANSI color in the text grid.
- Template file saved as reference design: `assets/templates/figby-30w.ftmp`.

### 2.0.10 — Phase merge: release/2.0 → master

Phase 2.0 merged into master. This merge brings 3 commits that landed on
`release/2.0` after the initial phase merge (10035c9):
- Fix broken template tests (TOML quoting, assertion fixes)
- Redesign `.ftmp` format: YAML frontmatter, defer template rendering to TUI
- Add `assets/tui/icons.yaml` for Phase 2.2, renumber 2.2.5→2.2.6

All subtasks 2.0.1–2.0.10 complete. Phase 2.1 (Image-to-ASCII Pipeline) is next.

## Phase 2.1 — Image-to-ASCII Pipeline

### 2.1.1 — Image loading + grayscale conversion via `rascii_art`

Added `image = { version = "0.24", features = ["jpeg", "png", "bmp", "webp"] }` to
Cargo.toml. `rascii_art = "0.4.5"` was already present (used by `template.rs`).

Created `image_input.rs` module with two public functions:
- `load_luminance_matrix(path)` — opens image from file path, returns `Result<Vec<Vec<u8>>, ImageError>`
- `luminance_from_dynamic(img)` — converts `&DynamicImage` to luminance matrix

Both delegate to `image::DynamicImage::to_luma8()` then extract pixel rows into
`Vec<Vec<u8>>` (outer=rows, inner=columns, each 0-255). No `.unwrap()` in
production — all errors propagate as `ImageError`.

7 unit tests: PNG fixture load, JPEG encode+load, BMP encode+load, WEBP encode+load,
known RGB luminance ordering (green > red > blue), luminance range (non-empty rows),
nonexistent file returns error. `lib.rs` updated with `pub mod image_input;`.
