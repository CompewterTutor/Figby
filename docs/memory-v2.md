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
