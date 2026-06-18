# Figby Handoff — 2026-06-18 (session 2)

## Git state

Branch: `master` — **push needed** (`git push origin master`).

```
c002693 docs: add Phase 5.6 palette UX & editor tasks
87ad664 fix: palette swatches now clickable via mouse
16a3d76 fix: welcome 'New Font from File' now shows TTF/OTF picker
9869403 docs: mark 5.1.5 done — release/5.1 merged to master
1121cbf 5.1.4: Collapsed/shared borders between adjacent panels
```

Phase 5.1 complete. Phase 5.2 is next.

## What was done this session

### Phase 5.1 recovery (ralph was frozen)
Ralph hung on task-5.1.4 for 5h. Killed manually, finished 5.1.4 by hand:
- `layout.rs`: `toolbox_list_borders()` / `toolbox_brush_borders()` / `right_panel_borders()` helpers; `spacing(0)` on all layouts
- `toolbox.rs` / `brush.rs`: `borders: Borders` pub field + `set_borders()` setter
- `tools/text.rs`: `render_options()` takes `borders: Borders` param
- `mod.rs`: toolbox renders at `toolbox_list` rect with `set_borders()` wiring
- Merged task-5.1.4 → release/5.1 → master (5.1.5 done)

### Bug fixes
- **`FontNewFromFile`**: added `FileOpsMode::ImportFont` — shows `.ttf/.otf` picker, converts via `font_file_to_figfont(path, 12.0, charsets::DEFAULT)`, loads into font editor
- **Zip browser**: now shows error "No .flf/.tlf fonts found in ZIP" when archive is empty (was showing only `..` silently)
- **Palette clicks**: `Palette::handle_click(col, row, area)` hit-tests FG/BG toggles + colour swatches; wired in `handle_mouse_event` for left-click on right panel

### Phase 5.6 tasks added
New phase after 5.5 in `docs/todo-v5.md`:
- 5.6.1 Hover colour name tooltip
- 5.6.2 5-per-row hue-grouped palette layout
- 5.6.3 Palette editor (save/load/duplicate)
- 5.6.4 Multi-format import (Paletty/ASE/WezTerm/Windows Terminal)
- 5.6.5 Marker brush (Aseprite-style hue stepping)
- 5.6.6 Phase merge

## Files changed

- `figby-rs/src/tui/layout.rs`
- `figby-rs/src/tui/toolbox.rs`
- `figby-rs/src/tui/brush.rs`
- `figby-rs/src/tui/tools/text.rs`
- `figby-rs/src/tui/mod.rs`
- `figby-rs/src/tui/file_ops.rs`
- `figby-rs/src/tui/palette.rs`
- `figby-rs/tests/tui.rs`
- `docs/todo-v5.md`

## Ralph restart

Monitor cron still active (`*/15 * * * * scripts/ralph-monitor.sh`).
Monitor only handles rate-limits — does NOT detect frozen processes.
If ralph freezes again: `pkill -9 -f ralph.sh`, then:

```bash
cd /home/hippo/git_repos/Figby
git checkout master
./scripts/ralph.sh
```

## Known issues

- 44 pre-existing test failures in `tests/run_tests.rs` (figby binary path, unrelated)
- Ralph freeze is recurring risk — check `/tmp/ralph-impl-<task>.log` age to detect
