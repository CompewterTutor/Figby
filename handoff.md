# Figby Handoff — 2026-06-22 (v6 hardening session)

## Git state

Branch: `release/6.6` (clean, all tests pass, clippy clean)

## Completed tasks this session

| Task | Description |
|------|-------------|
| 6.7.3 | Unicode panic verified fixed by 6.5.1; added `test_text_tool_unicode_no_panic` |
| 6.7.2 | Quit-confirm dialog: `trigger_quit()` / `quit_confirm_dialog` flag / overlay |
| 6.8.5 | Built-in palettes (Grayscale/Primary/Warm/Cool) in `palette_import::builtin_palettes()` |
| 6.8.7 | Keybinds popup scrollable (↑↓/PgUp/PgDn/q); 20+ missing keybinds added |
| 6.9.3 | Layers menu + Animation menu added to menu bar with full action handlers |
| 6.6.1g | `handle_font_editor_key()` extracted from `handle_key_event` |
| 6.6.1h | `handle_image_editor_key()` extracted from `handle_key_event` |
| 6.9.6 | Visual divider: `toolbox_list_borders()` → `Borders::ALL`; `toolbox_h` +1 to preserve 12-tool inner height |
| 6.8.6 | Lighting tool placeholder: `Tool::Lighting` variant + "Press G to open lighting editor" brush panel |
| 6.8.8 | Timeline scrollable (cached_max_vis_frames); layer panel scrollable (`render_with_stack` → `&mut self`, two-pass) |
| 6.8.3 | Canvas resize: Image → Resize Canvas menu action → opens settings dialog pre-filled with current canvas size |
| 6.8.1 | Open image file dialog: `FileOpsMode::OpenImage` + `enter_open_image()` + `perform_open_image()` + welcome screen 'I' key |

## Files changed this session

- `figby-rs/src/tui/mod.rs` — all the above handlers; `perform_open_image()`; `handle_image_editor_key` intercepts 'o' → `enter_open_image()`; `WelcomeAction::ImageOpen` dispatch
- `figby-rs/src/tui/overlays.rs` — quit-confirm dialog render, scrollable keybinds overlay
- `figby-rs/src/tui/keymap.rs` — added `LayerPanel` / `TextTool` scopes; 20+ missing bindings
- `figby-rs/src/tui/menu.rs` — Image menu + `ImageResizeCanvas`; Layers + Animation menus; `Tool::Lighting`
- `figby-rs/src/palette_import.rs` — `builtin_palettes()` + test
- `figby-rs/src/tui/tools/text.rs` — `test_text_tool_unicode_no_panic`
- `figby-rs/src/tui/layout.rs` — `toolbox_list_borders()` → `Borders::ALL`; updated comments
- `figby-rs/src/tui/toolbox.rs` — `Tool::Lighting` variant (display_name "Lg", key 'n', icon "tool_lighting")
- `figby-rs/src/tui/timeline.rs` — `cached_max_vis_frames` field; updated in `StatefulWidget::render`
- `figby-rs/src/tui/layers.rs` — `render_with_stack` → `&mut self`; two-pass scroll with ▲/▼ indicators
- `figby-rs/src/tui/side_panel.rs` — `layer_panel: Option<&mut LayerPanel>` parameter
- `figby-rs/src/tui/file_ops.rs` — `FileOpsMode::OpenImage`; `enter_open_image()`; `handle_key_open_image()`; `render_open_image()`; Widget impl arm for OpenImage
- `figby-rs/src/tui/welcome.rs` — `WelcomeAction::ImageOpen`; 'I' key; `IMAGE_ACTIONS` 6th entry; `image_action_for(5)`
- `docs/todo-v6.md` — marked 6.6.1g/h, 6.7.2, 6.7.3, 6.8.1, 6.8.3, 6.8.5, 6.8.6, 6.8.7, 6.8.8, 6.9.3, 6.9.6 done

## Current test state

1258 passing, 7 ignored. Clippy clean. cargo fmt applied.

## Remaining todo-v6.md (unchecked)

**Phase 6.8 — Missing Core Features:**
- [ ] 6.8.2 New image dialog: canvas size + palette selection (Medium)
- [ ] 6.8.4 Palette editor UI (High)

**Phase 6.9 — UX Polish:**
- [ ] 6.9.1 Layer panel: icon-based 2-row layout (Medium)
- [ ] 6.9.2 Layers: reorder by drag handle (Medium)
- [ ] 6.9.4 Move tool options to right sidebar (Medium)

## Recommended next tasks (in priority order)

1. **6.8.4** Palette editor UI (High priority)
2. **6.8.2** New image dialog: canvas size + palette selection (Medium)
3. **6.9.1** Layer panel: icon-based 2-row layout (Medium)

## Key decisions / non-obvious choices

- `quit_confirm_dialog Y` triggers `start_save()` (async); `quit_after_save` flag checked in `AsyncResult::SaveComplete` handler. On save failure, `quit_after_save` cleared (no quit).
- Built-in palettes load into `palette_editor.swatches` + open palette editor panel (via View menu). Not in new-image dialog (6.8.2 not done yet).
- Font/image editor key dispatch left as thin wrapper on `TuiApp` (not moved to `FontEditor`/`ImageEditor` structs) because both need `EditorState` mutation (`sync_font_char_to_canvas`, `undo.clear`) that isn't accessible from within those structs.
- `keybindings_scroll` resets to 0 on close.
- `AnimFrameAdd` menu handler uses hardcoded `12×6` thumbnail (matches existing code pattern).
- `6.9.6` separator: `toolbox_list_borders()` changed to `Borders::ALL`. `toolbox_h` bumped from `+1` to `+2` (12 tools × 1 row each + 2 border rows = 14 for list, preserves all 12 tools visible). Brush panel unchanged (LEFT|RIGHT|BOTTOM, no TOP).
- `6.8.1` file dialog: `Widget for &FileOpsDialog` impl renders to `&mut Buffer` directly (no Frame). OpenImage arm duplicates the render logic inline using `Widget::render(...)` calls — same pattern as ImportGif arm. `perform_open_image()` calls `image_editor.load_from_path()` + `sync_image_to_canvas()`.
- `handle_image_editor_key` intercepts 'o'/'O' before `image_editor.handle_key()` to redirect to file dialog; the old raw `entering_path` flow in `ImageEditor` is now bypassed for 'o'.
