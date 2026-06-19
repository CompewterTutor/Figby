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
