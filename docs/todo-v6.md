# Figby v6 — Pre-Release Hardening & Polish

Milestone goal: Clear every release blocker from the 2026-06-18 codebase audit
before UI/UX polish, public docs, and crates.io publish.

Source: `docs/codebase-audit-2026-06-18.md` (read it for full rationale; finding
IDs B0/B1/.../A1/S1 below map 1:1 to that doc). Severity: 🔴 blocker, 🟠 arch,
🟡 smell.

**Fix order is intentional — do phases in sequence.** 6.0 (security) and 6.1
(green tests) gate everything. Confirm `cargo test` GREEN + `cargo clippy
--all-targets` clean after every task.

---

## Phase 6.0 — Critical Security (🔴 do first)

- [x] `6.0.1` Remove `$(cmd)` command substitution from template resolver (B0/RCE)
  - **Goal:** `resolve_text_value` runs `sh -c <cmd>` for any `$(...)` in a
    template text value — rendering a shared `.ftmp` executes its embedded shell.
    Remove the `$(...)` branch entirely (recommended), OR gate behind a
    default-OFF `--unsafe-template-exec` flag that is NEVER honored for non-local
    templates. Treat `.ftmp` as untrusted data by default.
  - **Touches:** `figby-rs/src/template.rs:160-187` (the `strip_prefix('(')`
    branch in `resolve_text_value`); remove/rewrite the unit tests at
    `template.rs:1034-1058` that assert `$(echo …)` runs.
  - **Success:** New security test asserts `$(...)` does NOT execute by default
    (literal passthrough or error). `${VAR}` env expansion may stay but document
    the leak risk; consider gating it too.
  - **Difficulty:** Medium

- [x] `6.0.2` Sandbox `{{img:PATH}}` template image paths (B0 adjacent)
  - **Goal:** `render_template` reads an arbitrary local path via
    `rascii_art::render_to(&img.source)` — a shared template can read/exfil-render
    any file the user can read. Restrict image source to the template's own
    directory (reject absolute paths and `..` traversal).
  - **Touches:** `figby-rs/src/template.rs:571-590`.
  - **Success:** Template referencing `/etc/passwd` or `../../x` is rejected;
    same-dir relative image still renders. Add a test.
  - **Difficulty:** Medium

- [x] `6.0.3` Cap template canvas dimensions (B7/DoS)
  - **Goal:** `render_template` allocates `vec![vec![' '; width]; height]` from
    unvalidated `u32` frontmatter; a crafted `width=4000000000` → OOM. Clamp
    `width`/`height` (e.g. `width*height <= 1_000_000` cells) and `margin`/
    `padding` to sane maxima at the top of `render_template`.
  - **Touches:** `figby-rs/src/template.rs:517-526` (alloc), `:673-686`
    (margin/padding `repeat`).
  - **Success:** Oversize-dimension template returns an error instead of OOM;
    add a test asserting the cap.
  - **Difficulty:** Low

---

## Phase 6.1 — Green the Test Suite (🔴 blocker B3)

> All 10 failures are STALE TESTS, not app bugs (confirmed in audit). No
> production logic change expected. See audit B3 "Refined diagnosis."

- [x] `6.1.1` Fix welcome-gate stale tests (mode/tool cluster)
  - **Goal:** 4 tests press keys without dismissing the welcome screen, so keys
    route to welcome dispatch (gate at `mod.rs:2427`, `WelcomeScreen.show`
    defaults true). Add `app.welcome_screen.show = false;` after `TuiApp::new()`
    in each (pattern already used by the passing quit sub-test at `tui.rs:72`).
  - **Touches:** `figby-rs/tests/tui.rs` — `test_tui_mode_switching` (:60),
    `test_tool_selection_roundtrip` (:131),
    `test_image_editor_mode_switch_and_toggle` (:2139),
    `test_palette_fg_keyboard_shortcut` (:2457).
  - **Success:** All 4 pass; no production code touched.
  - **Difficulty:** Low

- [x] `6.1.2` Fix layers-model stale tests (poke active layer, not composite)
  - **Goal:** Tests write/read `editor.canvas.buffer` (composite output), but the
    app now sources from `layer_stack` + `recomposite_canvas`. Rewrite to write
    to / read the active LAYER buffer.
  - **Touches:** `figby-rs/tests/tui.rs` — `test_fill_tool_keyboard` (:881),
    `test_selection_perimeter_delete` (:2552), `test_tui_smoke_all_panels_render`
    (:47), `test_palette_render_contains_labels` (:617),
    `test_settings_toggle_visibility` (:749).
  - **Success:** All 5 pass; `Selection`/`flood_fill` production logic unchanged
    (already proven correct by passing unit tests).
  - **Difficulty:** Medium

- [x] `6.1.3` Fix shadow round-vs-truncate lib test
  - **Goal:** `palette_editor::test_load_current_from_palette` asserts `#4D0000`
    but `default_shadow_hex` truncates (`255*0.3=76.4→76=#4C0000`). Pick one:
    fix the test to `#4C0000`, OR change impl to `.round()`. Recommend `.round()`
    (matches user expectation) + update test.
  - **Touches:** `figby-rs/src/palette_import.rs:38`,
    `figby-rs/src/tui/palette_editor.rs:889`.
  - **Success:** lib test green; `cargo test` fully GREEN (0 fail).
  - **Difficulty:** Low

---

## Phase 6.2 — CI & Merge Gate (🔴 blocker — stops RED ever merging again)

- [x] `6.2.1` Add hard `cargo test` gate to ralph merge phase
  - **Goal:** ROOT CAUSE of B3 — `phase_review_and_merge` auto-merges on an LLM
    "approved" string with no real test run. Add a literal
    `cargo test --manifest-path figby-rs/Cargo.toml || { abort merge; }` (plus
    clippy/fmt) BEFORE the LLM review and before each task merge.
  - **Touches:** `scripts/ralph.sh:540-606`.
  - **Success:** A deliberately-failing test blocks the merge step.
  - **Difficulty:** Low

- [x] `6.2.2` GitHub Actions CI (fmt + clippy -D warnings + test)
  - **Goal:** New workflow runs `cargo fmt --check`, `cargo clippy --all-targets
    -- -D warnings`, `cargo test` on push/PR. Must be green to merge. Delete
    legacy `.travis.yml`.
  - **Touches:** new `.github/workflows/ci.yml`; remove `.travis.yml`.
  - **Success:** CI green on a clean branch; red on an intentional break.
  - **Difficulty:** Low

---

## Phase 6.3 — Parser Hardening (🔴 META — security backbone)

> Copy `palette_import.rs`'s pattern (checked_add, per-block bounds, count caps).
> It is the model — bring the others up to that bar. Extend `tests/fuzz.rs`
> (currently fonts only) with a target per parser.

- [x] `6.3.1` Validate FIGfont header numerics (B1)
  - **Goal:** `height/baseline/maxlength` parsed as `i32` then `as u32` — negative
    height → huge `charheight`, `height==0` accepted. Validate `height` in
    `1..=255`, reject negative `baseline`/`maxlength`, clamp `maxlength`.
  - **Touches:** `figby-rs/src/font.rs:261-326`.
  - **Success:** Crafted `.flf` with negative/zero/huge height rejected; fuzz
    target added.
  - **Difficulty:** Medium

- [x] `6.3.2` Cap zip decompression size (B2/zip-bomb)
  - **Goal:** `extract_first_zip_entry` / `read_zip_entry` call `read_to_end()`
    with no limit. Cap via `entry.size()` check or `take(MAX)`. (Path-traversal
    already defended.)
  - **Touches:** `figby-rs/src/font.rs:464-486`, `:526`.
  - **Success:** Small zip-bomb font rejected before exhausting memory; test.
  - **Difficulty:** Low

- [x] `6.3.3` Fix GIF memory-guard timing (B4/DoS)
  - **Goal:** `MAX_TOTAL_CELLS` is checked AFTER the read loop already cloned every
    frame. Check `width*height <= CAP` before the loop (dims known at `:69-70`);
    track `frame_count` in-loop and bail the moment `w*h*count` exceeds cap. Add a
    defensive length check on `frame.buffer[idx]` (`:199,224`).
  - **Touches:** `figby-rs/src/gif_import.rs:69-95,199,224`.
  - **Success:** Oversize GIF bails during decode; gif fuzz/oversize test added
    (module currently has 0 tests — S4).
  - **Difficulty:** Medium

- [x] `6.3.4` Range-check control-file group indices (B5/panic)
  - **Goal:** `state.gl = d - b'0'` / `gr` with no range check → byte `l 9` or
    `< '0'` → index ≥4 into `[u32;4]` or underflow → panic. Validate `d` is
    `b'0'..=b'3'` before assigning (ignore/clamp otherwise).
  - **Touches:** `figby-rs/src/control.rs:544,551` (and the `gn[..]` indexing at
    `:204-215`).
  - **Success:** Crafted `.flc` no longer panics; fuzz target added.
  - **Difficulty:** Low

- [x] `6.3.5` Limit image decode dimensions (B6/DoS)
  - **Goal:** `image::open(path)?` applies no `Limits`. Use
    `image::io::Reader::open()?.limits(Limits::default())` / set max
    width·height for both image-to-ASCII and TUI image import.
  - **Touches:** `figby-rs/src/image_input.rs:25,48`.
  - **Success:** Huge/decompression-bomb image rejected; test.
  - **Difficulty:** Low

---

## Phase 6.4 — Stale Docs (🟠 A2 — fix before release)

- [x] `6.4.1` Rewrite CLAUDE.md to match current source layout
  - **Goal:** Says "Current milestone v3"; references deleted
    `tui/components/{file_ops,font_editor,canvas}` wrappers (only `canvas.rs`,
    `status_bar.rs` remain); lists `font.rs`/`render.rs` under `tui/` (they're
    crate root). Update milestone to v6, fix the source-layout tree.
  - **Touches:** `figby-rs/CLAUDE.md` (or repo-root `CLAUDE.md`).
  - **Success:** Every path in CLAUDE.md exists; milestone current.
  - **Difficulty:** Low

- [x] `6.4.2` Fix AGENTS.md file-structure tree
  - **Goal:** Lists `src/util.rs` (does not exist) + outdated tree.
  - **Touches:** `AGENTS.md`.
  - **Success:** Tree matches actual `figby-rs/src/`.
  - **Difficulty:** Low

---

## Phase 6.5 — Correctness / Robustness (🟡 nice-to-fix this milestone)

- [x] `6.5.1` Replace `render.rs:14` `.expect()` with blank-glyph fallback (S1)
  - **Goal:** `lookup_char` `.expect()`s on missing char 0 — only production
    expect in the crate. A hand-edited font (font editor) could violate the
    char-0 invariant → panic. Return a blank glyph instead.
  - **Touches:** `figby-rs/src/render.rs:11-15`.
  - **Success:** Font missing char 0 renders blank, no panic; test.
  - **Difficulty:** Low

- [x] `6.5.2` Compile-time validate embedded ICONS_YAML (A3)
  - **Goal:** `TuiApp::new` does `serde_yaml::from_str(ICONS_YAML)
    .unwrap_or_default()` — malformed embedded YAML silently drops all icons.
    Add a build/`const` test that parses ICONS_YAML and fails compilation/CI on
    error.
  - **Touches:** `figby-rs/src/tui/mod.rs:405` + a new test.
  - **Success:** Breaking the YAML fails a test, not silently empties icons.
  - **Difficulty:** Low

- [x] `6.5.3` Clamp `font_gen` point_size + add file-path tests (S5)
  - **Goal:** `point_size: f32` unbounded → `charheight`/canvas allocs scale with
    it. Clamp to e.g. `4.0..=200.0`. `font_file_to_figfont` (the .ttf/.otf path
    variant) has 0 tests — add a bundled-font smoke test + a malformed-bytes test.
  - **Touches:** `figby-rs/src/font_gen.rs:566-577` (+ `render_font_glyphs`).
  - **Success:** Out-of-range point_size clamped; both new tests pass.
  - **Difficulty:** Low

---

## Phase 6.6 — Architecture (🟠 A1 — LARGE, may slip past v6)

- [ ] `6.6.1` Split `tui/mod.rs` god object (4076 LOC) — incremental
  - **Goal:** `handle_key_event` (1054 LOC), `handle_mouse_event` (510),
    `render` (332); `TuiApp` ~45 fields. Per audit, prefer the **Component
    Architecture** path: extract one mode/component at a time, each with
    `render` + `handle_event`; model dialogs as an input-layer stack (topmost
    consumes keys first). Group `TuiApp` fields into sub-structs (Animation,
    Lighting, Interaction) to shrink the borrow surface. Do NOT attempt in one PR.
  - **Touches:** `figby-rs/src/tui/mod.rs` (+ new module files).
  - **Success:** mod.rs shrinks meaningfully; each extracted component has its own
    handler + tests; behavior unchanged (test suite still green).
  - **Difficulty:** High — split into sub-tasks before starting.

---

## Deferred to post-v6 (tracked, not blocking)

Color-depth fallback (C1), reduced-motion `--no-anim` (C2), panic-hook terminal
restore + autosave (C3), perf opts (O1/O2/O3), new exports (SVG/asciinema/sixel),
template starter library, 3rd-party crate extraction
(`ratatui-paint-canvas` etc.), release tooling (cargo-dist/release-plz/VHS),
onboarding (`?`-help, which-key, tutorial), DESIGN.md + zoid-ui-kit token
alignment, **Figby→ rename/de-brand** (copyrighted name). See audit doc
🔵 Suggestions and Branding sections.
