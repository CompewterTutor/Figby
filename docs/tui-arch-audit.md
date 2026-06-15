# TUI Architecture Audit — v3.1 Refactor Targets

Audited against ratatui best practices. Each finding tagged with severity:
- 🔴 High — causes borrow errors, makes refactor hard, or breaks idiomatic use
- 🟡 Medium — tech debt, causes tight coupling
- 🟢 Low — style / minor inconsistency

---

## 1. Widget / StatefulWidget Implementations

**`impl Widget` count:** 1
- `canvas.rs:237` — `impl Widget for &CanvasWidget` ✅ correct ref form

**`impl StatefulWidget` count:** 0 ❌

Every widget that carries state (MenuBar, FileOpsDialog, ExportDialog, FontEditor,
Palette, Toolbox, StatusBar, UndoPanel) uses ad-hoc `render(&mut self, frame, area)`
methods instead of the `StatefulWidget` protocol. State is owned inside the widget
struct, not separated into a `*State` type.

### Findings

| file:line | severity | problem |
|-----------|----------|---------|
| `menu.rs` whole | 🔴 | MenuBar has no `StatefulWidget` impl; dropdown hit rects stored on struct |
| `font_editor.rs:322` | 🔴 | FontEditor::render takes `&mut self` — mutates during draw |
| `export.rs:230` | 🟡 | ExportDialog::render takes `&mut self` |
| `file_ops.rs:393` | 🟡 | FileOpsDialog::render takes `&mut self` |
| `component.rs:24` | 🔴 | `Component::draw` is abstract with `&mut self` — bakes mutation into trait |
| `toolbox.rs:169` | 🟡 | `render_stateful_widget(list, area, &mut state)` passes ListState directly — no wrapper StatefulWidget |

---

## 2. render_widget Call Audit

Total: **47 calls**
- 3 pass refs (`&T`) ✅
- 44 pass values (consuming) ❌

Consuming render_widget means widgets are constructed in render, mutated inline, then
discarded. This prevents caching, makes hit-testing impossible without side channels,
and blocks `Widget for &T` migration.

### Key consuming call sites

| file | lines | notes |
|------|-------|-------|
| `mod.rs` | 363, 420, 487, 591, 603, 616, 622, 642, 647 | main render — inline widget construction |
| `font_editor.rs` | 369, 378, 386, 394, 473, 532, 625, 706 | all consuming |
| `file_ops.rs` | 402, 408, 512, 516, 522, 603 | consuming Paragraph/Block/List |
| `menu.rs` | 230, 257, 298, 302, 333 | consuming spans/blocks |
| `palette.rs` | 243, 349 | consuming |
| `status.rs` | 68, 152, 155, 198 | consuming |

---

## 3. Rect Stored in Structs (Layout Coupling)

These fields couple render-time geometry to event-time hit-testing. Correct pattern:
compute all rects in a single `layout()` pass, store in a `FrameLayout` struct.

| file:line | field | severity |
|-----------|-------|----------|
| `mod.rs:118` | `toolbox_area: Rect` | 🔴 |
| `mod.rs:119` | `palette_area: Rect` | 🔴 |
| `components/canvas.rs:12` | `canvas_inner_rect: Rect` | 🟡 |
| `menu.rs:48` | `frame_area: Rect` | 🔴 |

---

## 4. Layout Usage

Only 4 `Layout::default()` sites — correct ratatui API used where layouts exist.
Problem is coverage: many areas use hardcoded Rect arithmetic instead.

| file:line | notes |
|-----------|-------|
| `mod.rs:326` | vertical split (menu/tabs/main/status) ✅ |
| `mod.rs:365` | horizontal split (toolbox/canvas/palette) ✅ |
| `mod.rs:374` | vertical tool/brush split ✅ |
| `font_editor.rs:334` | font editor layout ✅ |

---

## 5. TuiApp God Struct

`mod.rs`: **2136 lines**, **39 fields** on TuiApp (22 pub, 17 private).

Fields span unrelated domains:
- Canvas/editor state (undo, clipboard, selection, tool state)
- Dialog state (file_ops_comp, export_comp, undo_panel_comp)
- App meta (mode, theme, render_mode, dirty, fps, git_branch)
- Layout cache (toolbox_area, palette_area)
- Font editor (font_editor_comp)

Target split per task 3.1.2:
- `AppState` — mode, theme, render_mode, dirty, fps, git_branch
- `DialogState` — file_ops_comp, export_comp, undo_panel_comp
- `EditorState` — canvas, selection, clipboard, tool state, undo

---

## 6. Component / Action Protocol

`component.rs:11` — `Component` trait:
- `handle_key_event(&mut self, KeyEvent) -> Option<Action>`
- `handle_mouse_event(&mut self, MouseEvent) -> Option<Action>`
- `update(&mut self, Action) -> Option<Action>`
- `draw(&mut self, frame, area)` — **abstract, &mut self** ❌

`action.rs:8` — `Action` enum: 16 variants. Actions like `FontEditorAction` carry no
payload — dispatch must inspect component state separately to know what happened.

Target per task 3.1.4: typed event enums (`FontEditorEvent`, `CanvasEvent`) returning
`Option<AppEvent>`. Eliminates the weakly-typed Action intermediary.

---

## 7. Render Methods with &mut self

13 render methods mutate self during draw. Correct pattern is `&self` (read-only) or
`StatefulWidget` with external state mutation.

| file:line | method |
|-----------|--------|
| `mod.rs:322` | TuiApp::render |
| `font_editor.rs:322` | FontEditor::render |
| `font_editor.rs:332` | render_overview |
| `font_editor.rs:476` | render_smush_editor |
| `font_editor.rs:535` | render_transform_editor |
| `font_editor.rs:628` | render_header_editor |
| `export.rs:230` | ExportDialog::render |
| `file_ops.rs:393` | FileOpsDialog::render |
| `file_ops.rs:401` | render_open |
| `file_ops.rs:515` | render_save_as |
| `menu.rs:268` | render_dropdown |
| `tools/text.rs:78` | render_rows_from_buffer |
| `tools/text.rs:263` | render_text_to_buffer |

---

## Refactor Priority Order

Based on severity and task dependencies:

1. **3.1.2** — Split TuiApp (39 fields, 2136 lines) into AppState/DialogState/EditorState
2. **3.1.3** — Convert all render methods from `&mut self` to `Widget for &T`; zero StatefulWidget impls is the core gap
3. **3.1.5** — `FrameLayout` struct: move toolbox_area, palette_area, frame_area out of structs
4. **3.1.4** — Replace Action enum with typed event enums
5. **3.1.7** — MenuBar as proper StatefulWidget (depends on 3.1.3 + 3.1.5)
6. **3.1.8** — Keymap dispatch wired from 3.0.4 table

The 44 consuming render_widget calls will be fixed as a side-effect of 3.1.3 (converting
to `Widget for &T` means refs are passed to render_widget).
