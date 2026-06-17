# TUI Architecture Audit: ratatui Best-Practice Gaps

Audit date: 2026-06-17
Task: 4.3.1
Auditor: Ralph (automated)

Reference docs:
- <https://docs.rs/ratatui/latest/ratatui/widgets/index.html#authoring-custom-widgets>
- ratatui 0.30.1 (`figby-rs/Cargo.toml:20`)

---

## 1. Custom `Component` trait forces `&mut self` during draw

**File:** `figby-rs/src/tui/component.rs:9-23`

```rust
pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()>;
}
```

**Problem:** `draw(&mut self, ...)` violates ratatui's read-only-render principle.
Widgets should be renderable by reference (`Widget for &T` or `WidgetRef`).
Taking `&mut self` prevents rendering from multiple borrows, blocks sharing
state between widgets without `RefCell`, and misaligns with ratatui's
architecture where rendering should never mutate widget data.

**Affected impls (9 total):**
- `CanvasComponent` — `components/canvas.rs:65`
- `PaletteComponent` — `components/palette.rs:69`
- `ToolboxComponent` — `components/toolbox.rs:72`
- `FontEditorComponent` — `components/font_editor.rs:57`
- `StatusBarComponent` — `components/status_bar.rs:64`
- `ExportComponent` — `components/export.rs:48`
- `FileOpsComponent` — `components/file_ops.rs:65`
- `UndoPanelComponent` — `components/undo_panel.rs:41`
- `ImageEditorComponent` — `components/image_editor.rs:39` (no-op)

**Proposal:** Eliminate custom `Component` trait entirely. Replace with
direct `Widget for &T` / `StatefulWidget` implementations on each inner type.
Move event handling into `TuiApp`'s `handle_key_event` / `handle_mouse_event`
dispatch (already done inline in `mod.rs` — see §5).

---

## 2. State mutation inside render pass

### 2a — Canvas buffer sync during render

**File:** `figby-rs/src/tui/mod.rs:733-738`

```rust
if self.mode == AppMode::FontEditor {
    self.editor.sync_canvas_to_font_char();
}
if self.mode == AppMode::ImageEditor {
    self.editor.sync_image_to_canvas();
}
```

Called from `render_canvas_area()` which is called from `render()`.
These mutate `self.editor.canvas_comp.canvas.buffer` mid-render.

**File:** `figby-rs/src/tui/mod.rs:157-207` — `sync_canvas_to_font_char()` and
`sync_image_to_canvas()` both create or replace `CanvasWidget` and write cells
into the buffer.

### 2b — Canvas component stores rendering geometry

**File:** `figby-rs/src/tui/components/canvas.rs:77-80`

```rust
self.last_canvas_size = (buf_w, buf_h);
self.canvas_inner_rect = centered;
```

These fields (`canvas_inner_rect`, `last_canvas_size`) are mutated inside
`CanvasComponent::draw()` then used in subsequent frame's mouse handlers.

**File:** `figby-rs/src/tui/components/canvas.rs:12-13` — field declarations.

### 2c — FontEditor::render takes `&mut self`

**File:** `figby-rs/src/tui/font_editor.rs:457`

```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect) {
```

This is called from `FontEditorComponent::draw()` which is `&mut self`
(through the `Component` trait). Even if the trait were fixed, `FontEditor::render`
itself takes `&mut self`.

**Proposal:** Decompose into explicit "sync" phase before render pass.
Make render read-only (`&self`). Move geometry fields out of component
structs — compute inside the render closure and pass as parameters
to widget calls.

---

## 3. Four rendering patterns coexist

### Pattern A (correct — `Widget for &T`)
- `CanvasWidget` — `tui/canvas.rs:268`
- `Palette` — `tui/palette.rs:260`
- `Toolbox` — `tui/toolbox.rs:153`
- `CanvasSettings` — `tui/status.rs:153`
- `AnimationTimeline` — `tui/timeline.rs` (impl `Widget for &AnimationTimeline`)
- `MenuBar` — `tui/menu.rs` (impl `StatefulWidget for &MenuBar` — also correct)

These follow the ratatui recommended pattern: the widget holds state and
renders via `&self` reference.

### Pattern B — `render(&self, frame, area)` standalone method
- `BrushState::render()` — `tui/brush.rs:159`
- `ExportDialog::render()` — `tui/export.rs:230`
- `FileOpsDialog::render()` — `tui/file_ops.rs:547`
- `UndoPanel::render()` — `tui/undo_panel.rs:49`
- `WelcomeScreen::render()` — `tui/welcome.rs:30`

These could be simple `Widget for &T` impls instead.

### Pattern C — `render(&mut self, frame, area)` standalone method
- `FontEditor::render()` — `tui/font_editor.rs:457`

Mutates state during render (e.g., search filtering, grid state changes).

### Pattern D — `render()` forwarding methods on inner widgets
- `Palette::render()` delegates to `frame.render_widget(self, area)` — `tui/palette.rs:255-257`
- `Toolbox::render()` delegates to `frame.render_widget(self, area)` — `tui/toolbox.rs:148-150`
- `CanvasSettings::render()` delegates to `frame.render_widget(self, area)` — `tui/status.rs:148-150`

These are dead forwarding methods: they exist only because the outer
Component wrapper calls `self.widget.render(frame, area)` instead of
`frame.render_widget(&self.widget, area)`.

**Proposal:** Unify all on `Widget for &T` / `StatefulWidget for &T`.
Remove dead `render()` forwarding methods.

---

## 4. `io::Result<()>` in draw is dead complexity

**File:** `figby-rs/src/tui/component.rs:22`

```rust
fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()>;
```

Every single impl returns `Ok(())`. Ratatui rendering is infallible — the
`Frame` and `Buffer` APIs never return `Result`. The `io::Result` wrapper
provides no value and forces `let _ = component.draw(frame, area);` at
every call site.

**Affected call sites (all return ignored):**
- `mod.rs:612` — `let _ = self.editor.toolbox_comp.draw(frame, tb_full);`
- `mod.rs:632` — `let _ = self.editor.palette_comp.draw(frame, rp);`
- `mod.rs:682` — `let _ = self.status_bar_comp.draw(frame, fl.status);`
- `mod.rs:731` — `let _ = self.editor.font_editor_comp.draw(frame, inner);`

**Proposal:** Remove `Result` from draw signatures. Change return to `()`.

---

## 5. Component wrapper layer adds no value

Nine `components/*.rs` files wrap inner widgets (`PaletteComponent` wraps
`Palette`, etc.) solely to satisfy the `Component` trait. Analysis of each:

| File | Inner type | Lines | Behavior |
|------|-----------|-------|----------|
| `components/palette.rs` | `Palette` | 73 | `draw()` calls `self.palette.render()` |
| `components/toolbox.rs` | `Toolbox` + `BrushState` | 79 | `draw()` calls `self.toolbox.render()` |
| `components/canvas.rs` | `CanvasWidget` | 95 | `draw()` renders canvas + edge block, mutates geometry state |
| `components/font_editor.rs` | `FontEditor` | 62 | `draw()` calls `self.editor.render()` |
| `components/status_bar.rs` | (inline rendering) | 169 | Full render logic inline |
| `components/export.rs` | `ExportDialog` | 60 | `draw()` calls `self.dialog.render()` |
| `components/file_ops.rs` | `FileOpsDialog` | 77 | `draw()` calls `self.dialog.render()` |
| `components/undo_panel.rs` | `UndoPanel` | 48 | `draw()` calls `self.panel.render()` |
| `components/image_editor.rs` | `ImageEditor` | 42 | `draw()` is no-op |

**Key finding:** Event handling for most of these already happens
directly in `TuiApp::handle_key_event()` (`mod.rs:1462-2037`), not through
`Component::handle_key_event()`. The event dispatch was already inlined
into the app. The Component trait wrapper layer remains solely for the
`draw()` method.

Exceptions: `MenuBar` uses `StatefulWidget for &MenuBar` directly
(no Component wrapper). `AnimationTimeline` uses `Widget for &AnimationTimeline`
directly.

**Proposal:** Remove `Component` trait and all 9 `*Component` wrapper structs.
Inline rendering into `TuiApp::render()` using direct `frame.render_widget(&widget, area)`
calls. This eliminates the two-layer architecture.

---

## 6. Dead `StatusBar` code

**File:** `figby-rs/src/tui/status.rs:13-71`

```rust
pub struct StatusBar;

impl StatusBar {
    pub fn render(
        frame: &mut Frame<'_>,
        area: Rect,
        ...
    ) { ... }
}
```

Old `StatusBar` struct with static `render()` method. Unused in the
render path — `TuiApp::render()` uses `StatusBarComponent` (`components/status_bar.rs`)
instead. The old `StatusBar::render()` is never called.

**Proposal:** Remove dead `StatusBar` code (`status.rs:13-71`).

---

## 7. Frame layout stored as state causes stale-geometry coupling

**File:** `figby-rs/src/tui/layout.rs:37-52` — `FrameLayout` struct stored on `TuiApp`.

```rust
pub struct TuiApp {
    ...
    frame_layout: layout::FrameLayout,
    ...
}
```

**File:** `figby-rs/src/tui/mod.rs:550-551` — stored each frame:
```rust
let fl = layout::FrameLayout::compute(frame.area(), self.zen_mode, self.right_drawer);
self.frame_layout = fl;
```

**Used in mouse handlers at:** `mod.rs:1062`
```rust
if let Some(tb) = self.frame_layout.toolbox_list { ... }
```

The mouse handler uses geometry from the *previous* frame's render pass.
If the terminal was resized between frames, the coordinates are stale until
the next render cycle.

**Additional state:** `components/canvas.rs:12-13` — `canvas_inner_rect` and
`last_canvas_size` stored on `CanvasComponent`, written during draw, read
by mouse handlers.

**Proposal:** Compute layout once per frame in `render()`, pass geometry
as parameters to mouse handlers instead of reading from `self.frame_layout`.
Or, compute layout inline in the mouse handler by reading current terminal
size.

---

## 8. `EditorState` mixes permanent state with transient drag state

**File:** `figby-rs/src/tui/mod.rs:114-119`

```rust
selection_drag_origin: Option<(i16, i16)>,
selection_polygon_points: Vec<(i16, i16)>,
selection_lasso_points: Vec<(i16, i16)>,
prev_mouse_buf: Option<(i16, i16)>,
line_start: Option<(i16, i16)>,
saved_buffer: Option<canvas::CanvasBuffer>,
```

These fields are transient operation state — set during mouse-down, read
during drag/move, cleared on mouse-up. They are not "editor state" in the
persistent sense, but they live alongside permanent state like
`canvas_comp`, `palette_comp`, `undo`, `selection`, `clipboard`.

**Problem:** Makes `EditorState` harder to reason about. Every method on
`EditorState` potentially touches transient fields. The `selection_polygon_points`
is checked in multiple unrelated places (`mod.rs:506`, `mod.rs:754`, `mod.rs:1870-1890`).

**Proposal:** Extract transient drag state into a separate struct
(e.g., `DragState` or `InteractionState`) stored directly on `TuiApp`.
Or, refactor into local variables scoped to the mouse event handler.

---

## 9. `WidgetRef` available but unused

ratatui 0.30.1 supports `WidgetRef` (stable since 0.28). `WidgetRef` is
an alternative to `Widget for &T` that uses an explicit `Ref` wrapper.
Current code uses only `Widget for &T`.

This is not a gap per se — `Widget for &T` is equally correct. However,
`WidgetRef` provides a cleaner migration path for types that currently
implement custom `render()` methods (Pattern B above), because the
`WidgetRef` trait can be implemented on the value type directly rather
than requiring a reference wrapper.

**Proposal:** Not required, but worth noting as an alternative migration
strategy. Using `Widget for &T` everywhere (fixing Pattern B and C) is
equally correct and simpler for the current codebase.

---

## 10. `ratatui::init()` is acceptable but bare

**File:** `figby-rs/src/tui/mod.rs:467`

```rust
let mut terminal = ratatui::init();
```

Uses the default `ratatui::init()` which calls `try_init()` with a default
panic hook that restores the terminal on panic. Fine for simple apps, but
a TUI editor may benefit from:
- Custom panic hook that writes a log file
- `AlternateScreenBackend` negotiation (e.g., fallback if alt screen unsupported)
- Custom `Terminal` creation with `Viewport` control

**Proposal:** Low priority. Consider custom terminal initialization
for production robustness.

---

## 11. `AnimationTimeline` uses correct pattern (exemplary)

**File:** `figby-rs/src/tui/timeline.rs`

`AnimationTimeline` implements `Widget for &AnimationTimeline` and
`StatefulWidget for &AnimationTimeline` — exactly the recommended pattern.
State is separated into `TimelineState` (analogous to `ListState`).
This is the model that all other widgets should follow.

Similarly, `MenuBar` + `MenuBarState` (`tui/menu.rs`) correctly uses
`StatefulWidget for &MenuBar` with separate state, and is rendered via
`frame.render_stateful_widget()`.

---

## Summary: Concrete Refactors for 4.3.2

Priority ordered:

| # | Refactor | Impact | Files |
|---|----------|--------|-------|
| P0 | Remove `Component` trait. Inline `draw()` into `TuiApp::render()`. Replace with direct `Widget for &T` / `frame.render_widget()`. | **High** — eliminates two-layer wrapper pattern, fixes `&mut self` issue | `component.rs` (delete), all 9 `components/*.rs` (delete or inline), `mod.rs` (update render), `brush.rs`, `export.rs`, `file_ops.rs`, `undo_panel.rs`, `font_editor.rs`, `welcome.rs` (add `Widget for &T` impls) |
| P1 | Remove `io::Result<()>` from draw/return types | **Low effort**, mechanical change | `component.rs`, `mod.rs` call sites |
| P2 | Decompose sync phase out of render pass | **Medium** — requires separating state update from rendering | `mod.rs:733-738`, `mod.rs:157-207` |
| P3 | Extract transient drag state from `EditorState` | **Medium** — refactors mouse handler | `mod.rs:114-119`, mouse handler methods |
| P4 | Remove dead `StatusBar` code | **Low effort**, mechanical | `status.rs:13-71` |
| P5 | Remove dead `render()` forwarding methods | **Low effort**, mechanical | `palette.rs:255-257`, `toolbox.rs:148-150`, `status.rs:148-150` |
| P6 | Move geometry out of stored state into render params | **Medium** — affects layout + mouse handler coupling | `layout.rs`, `mod.rs` mouse handlers, `components/canvas.rs` |
| P7 | Add `Widget for &BrushState`, `Widget for &ExportDialog`, etc. for Pattern B types | **Medium** — 5 types need `Widget` impls | `brush.rs`, `export.rs`, `file_ops.rs`, `undo_panel.rs`, `welcome.rs` |
