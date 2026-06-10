# Changelog

## [Unreleased] — Rust Port

### Added

- Rust project scaffold (`feiglet-rs/`)
- Cargo workspace configuration
- FIGlet font submodule for test fixtures
- CI configuration (fmt + clippy + test)

### Porting Progress

- [ ] Phase 1.1 — Crate scaffold, font parser
- [ ] Phase 1.2 — Render engine (kerning + smushing)
- [ ] Phase 1.3 — CLI interface (all FIGlet flags)
- [ ] Phase 1.4 — Control files + character mapping
- [ ] Phase 1.5 — Multi-byte input (UTF-8, DBCS, Shift-JIS)
- [ ] Phase 1.6 — TLF (TOIlet) font support
- [ ] Phase 1.7 — Full test suite against original C
- [ ] Phase 1.8 — Optimization + polish
