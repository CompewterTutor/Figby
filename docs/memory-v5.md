# Figby — v5 Memory

## Phase 5.7 — Animation Enhancements

### 5.7.1 — Animated GIF import to timeline

Created `figby-rs/src/gif_import.rs` — GIF decode module using the `gif` crate (0.13):
- `import_gif(path)` reads GIF file via `gif::Decoder::new()`, composites frames
  with proper disposal handling (Keep/Background/Previous), extracts frame delays
  (centiseconds) and loop count from `gif::Repeat` enum.
- `GifImportResult` contains `frames: Vec<Vec<Vec<CanvasCell>>>`, `frame_delays`,
  `loop_count`, and `palette_colors` for palette inference.
- `GifImportError` enum with `Io`, `Decode`, `NoFrames`, `TooLarge` variants.
- Memory guard: rejects GIFs with >1M total cells (`MAX_TOTAL_CELLS`).
- Disposal method handling: `DisposalMethod::Background` clears frame region to
  GIF background color, `DisposalMethod::Previous` saves/restores canvas state.

TUI integration:
- `FileOpsMode::ImportGif` variant in `tui/file_ops.rs` with `enter_import_gif()`,
  `handle_key_import_gif()`, `render_import_gif()` — filters directory to `.gif` files.
- `WelcomeAction::ImageImportGif` in `tui/welcome.rs` — 5th action in IMAGE_ACTIONS
  with `G` keybinding ("GIF Import"). `image_action_for()` updated for index 4.
- `MenuAction::FileImportGif` in `tui/menu.rs` — "Import GIF" item in File menu.
- `TuiApp::perform_import_gif()` in `tui/mod.rs` — decodes GIF, resizes canvas,
  copies first frame to active layer, populates timeline frames with thumbnails
  and `layer_state` buffers, sets frame delays on export dialog, switches to
  ImageEditor mode with timeline visible.

6 files created/modified: `gif_import.rs` (new), `lib.rs`, `file_ops.rs`,
`welcome.rs`, `mod.rs`, `menu.rs`. No `.unwrap()` in production. fmt and clippy pass clean.

### 5.7.2 — Phase merge: release/5.7 → main

Merged all Phase 5.7 work (5.7.1) into default branch (master). Phase 5.7 complete:
animated GIF import to timeline with frame compositing, disposal handling, memory
guard, palette inference. Also includes: Marker brush Alt-modifier palette reversal
(fc6de51). 11 files / 843 lines merged. Next phase: 5.8 (Dynamic Lighting System).

## Phase 5.8 — Dynamic Lighting System

### 5.8.1 — Core lighting engine (`lighting.rs`)

Created `figby-rs/src/tui/lighting.rs` with full core lighting engine:
- `Normal3(i8, i8, i8)` — quantized unit normal with `from_f32()`, `to_f32()`, `dot()`
- `NormalMap` — 2D normal grid with bounds-checked get/set/get_mut, fills flat `(0,0,1)`
- `Rgb(u8, u8, u8)` — color triple
- `Attenuation` — point-light falloff (constant/linear/quadratic) with defaults
- `Light` enum — Ambient/Directional/Point variants
- `Scene` — light collection with add/remove/clear methods
- `LutEntry` / `LightingLut` — 256-entry luminance→(color,char) LUT with lerp and default char map
- `compute_normal_map_figfont()` — Sobel 3×3 gradient on heightfield, mirror-padded borders
- `shade_canvas()` — per-cell Lambertian diffuse + shadow testing for all light types
- `cast_shadow()` — Amanatides & Woo DDA 2D grid traversal, distance-limited
- `intensity_to_char()` — luminance → char via linear index into char map

22 unit tests covering all components in isolation. No `.unwrap()` in production. fmt and clippy pass clean.

### 5.8.2 — Canvas and layer integration

Wired lighting engine into canvas render pipeline:
- `CanvasCell` (defined in `lib.rs` `canvas_inner` module) gained `height: Option<u8>` field (default `None`). All construction sites updated with `height: None`.
- `Layer` gained `accepts_lighting: bool` and `casts_shadow: bool` (both default `true`).
- Created `figby-rs/src/tui/components/canvas.rs` with `shade_composited()` — builds shadow/lighting masks from layer flags, computes normal map via `lighting::compute_normal_map_figfont()`, generates luminance via `lighting::shade_canvas()`, maps through `LightingLut`.
- `TuiApp` gained `lighting_scene: Option<Scene>`, `max_shadow_distance: u16` (default 50), `height_scale: f32` (default 0.5), `lighting_lut: LightingLut` fields.
- Shading pass inserted after layer compositing in render function; skipped when `lighting_scene` is `None`. Buffer preserved via save/restore to prevent frame-to-frame compounding.
- Layer panel shows `A`/`S` status indicators for lighting/shadow flags; `L` toggles `accepts_lighting`, `S` toggles `casts_shadow`.

`CanvasCell` re-exported from `tui/canvas.rs` (`pub use crate::CanvasCell`). 17 files touched. No `.unwrap()` in production. fmt and clippy pass clean.
