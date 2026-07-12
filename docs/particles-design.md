# Particle System Design

Status: **v7** — per-particle keyframe tracks + on-death secondary emitters
landed in `figby-rs/src/tui/particles.rs` (task 7.5.2). Future extensions
tracked under "Deferred" below.

## 1. Overview

Figby's particle system is a lightweight 2D emitter that lives entirely in
`tui/particles.rs`. A `ParticleSystem` owns a `ParticleConfig` and a
`Vec<Particle>`. Every `update(dt, bounds, layer_mask)` tick advances the
simulation; `render_to_canvas` / `bake_to_buffer` rasterise the live
particles into a `CanvasBuffer`.

The system is intentionally minimal — no GPU, no physics solver, no
inter-particle forces — so it stays cheap enough to tick at 60 FPS inside
the TUI event loop alongside the rest of the editor.

## 2. Data Model

### 2.1 `ParticleConfig` (emitter-wide)

| Field                     | Type                          | Purpose                                              |
|---------------------------|-------------------------------|------------------------------------------------------|
| `emitter_x` / `emitter_y` | `f64`                         | Emitter origin                                       |
| `spawn_rate`              | `f64`                         | Particles per second                                 |
| `lifetime_min` / `max`    | `f64`                         | Per-particle lifetime range (random within)          |
| `velocity_*_min/max`     | `f64`                         | Initial velocity range                                |
| `acceleration_x/y`        | `f64`                         | Global acceleration (gravity / wind)                 |
| `spread_angle`            | `f64`                         | Cone half-angle applied to initial velocity          |
| `emission_shape`          | `EmissionShape`               | Point / Circle / Rect spawn region                   |
| `edge_mode`               | `EdgeMode`                    | Bounce / Wrap / Despawn at canvas bounds (7.5.1)     |
| `collide_with_layer`      | `bool`                        | Reflect off non-blank layer cells (7.5.1)            |
| `keyframes`               | `Vec<ParticleKeyframe>`       | Per-particle keyframe track (7.5.2)                   |
| `on_death_count`          | `usize`                       | Secondary burst size on primary death (7.5.2)        |
| `on_death_config`         | `Option<Box<ParticleConfig>>` | Sub-config for the death burst (7.5.2)               |

### 2.2 `Particle` (per-instance)

| Field                 | Type                          | Notes                                          |
|-----------------------|-------------------------------|------------------------------------------------|
| `x` / `y`             | `f64`                         | Position                                        |
| `vx` / `vy`           | `f64`                         | Velocity                                        |
| `remaining_lifetime`  | `f64`                         | Counts down; ≤ 0 → dead                         |
| `total_lifetime`       | `f64`                         | Set at spawn; denominator for `progress()`      |
| `size` / `color` / `character` / `opacity` | —            | Static fallback when no keyframes               |
| `blend_mode`          | `BlendMode`                   | Compositing mode                                |
| `keyframes`           | `Vec<ParticleKeyframe>`        | Optional per-particle track (cloned from config)|
| `is_secondary`        | `bool`                        | `true` for death-burst particles; prevents recursion |

### 2.3 `ParticleKeyframe`

| Field      | Type                | Notes                                              |
|------------|---------------------|----------------------------------------------------|
| `time`     | `f64`               | Fraction of total lifetime in `[0.0, 1.0]`         |
| `color`    | `Option<(u8,u8,u8)>`| `None` inherits from the other side of the segment  |
| `size`     | `u8`               |                                                    |
| `character`| `char`             |                                                    |
| `opacity`  | `u8`               |                                                    |

Keyframes need not be sorted in the input; `render_values()` sorts a
clone each call. For hot paths that becomes measurable, cache the sorted
order on the particle at spawn.

## 3. Interpolation

`Particle::render_values()` returns `(color, size, character, opacity)`
after applying the keyframe track:

1. Compute `progress = (1.0 - remaining_lifetime / total_lifetime)` clamped to `[0,1]`.
2. If no keyframes or `total_lifetime ≈ 0`, return the particle's static fields.
3. Clamp to the first/last keyframe when `progress` is outside the track range.
4. Find the adjacent pair `(a, b)` with `a.time ≤ progress < b.time`.
5. `t = (progress - a.time) / (b.time - a.time)` clamped to `[0,1]`.
6. Lerp `color` (channel-wise `round().clamp(0,255)`), `size`, `opacity`.
7. `character` picks the nearer endpoint: `a.character` if `t < 0.5`,
   else `b.character`.

The character pick is a step rather than a blend because ASCII glyphs
don't interpolate. A future "morph" mode could crossfade by rendering
both characters with split opacity.

## 4. Lifecycle Hooks

### 4.1 On-Death Burst

When a primary particle's `remaining_lifetime` drops to ≤ 0 and the
config carries `on_death_count > 0` plus an `on_death_config`, the system
spawns `on_death_count` secondaries at the parent's `(x, y)` using the
sub-config. The sub-config is a `Box<ParticleConfig>` so the recursive
reference stays fixed-size.

Secondaries are flagged `is_secondary = true` so their own death cannot
trigger another burst — the recursion is bounded at depth 1 by design.
Lifting this to N-level recursion would require a depth counter and a
cap to prevent runaway particle counts.

The burst fires once per primary, after the `update` step, before the
`retain` that culls dead particles. Secondaries begin with a fresh
`total_lifetime` from the sub-config, so their keyframe track (if any)
runs independently of the parent's progress.

### 4.2 Spawn-Time Hooks (future)

Today spawn is uniform: every particle draws from the same config. A
future `on_spawn` callback could mutate the particle before it enters
the active set — for example, inheriting the parent's velocity for a
"trail" effect, or sampling colour from a palette gradient.

## 5. Rendering

`render_to_canvas` and `bake_to_buffer` are the two rasterisation
paths. Both call `p.render_values()` per particle to pick the
interpolated colour/char, then write to the cell at `(x.round(), y.round())`
if it falls inside the buffer. Negative or out-of-range positions are
silently skipped — no panics.

Opacity and `blend_mode` are currently stored but not applied during
rasterisation; the cell is overwritten directly. Wiring `blend_mode`
into the canvas write is a deferred item (see §7).

## 6. Test Coverage

Tests live in `particles.rs::tests`. Coverage after 7.5.2:

- Spawn / motion / acceleration / expire / lifecycle
- Spawn-rate accumulator fractional carry
- Pause / resume / clear / zero / negative `dt` no-op
- Emission shapes (Point / Circle / Rect)
- Spread angle cone
- Render-to-canvas + bounds clipping
- Bake-to-buffer independence + frame sequence
- Edge collision: Bounce (R/L), Wrap, Despawn, no-op without bounds
- Layer-cell collision reflect + disabled no-op
- **Keyframe colour interpolation at 25% / 50% / 75%** (7.5.2)
- **Keyframe character step at low-t / high-t** (7.5.2)
- **Keyframe empty-track fallback to static fields** (7.5.2)
- **Keyframe progress clamps to endpoints** (7.5.2)
- **Render-to-canvas uses interpolated colour** (7.5.2)
- **On-death burst spawns N secondaries at parent position** (7.5.2)
- **On-death burst is non-recursive** (7.5.2)
- **On-death disabled when count = 0** (7.5.2)
- **Secondary inherits sub-config keyframes** (7.5.2)

## 7. Deferred / Future Work

- **Apply `blend_mode` + `opacity` during rasterisation** — currently
  stored only; canvas write is a direct overwrite.
- **N-level recursive bursts** — replace the `is_secondary` boolean
  with a `depth: u8` and a configurable `max_recursion_depth`.
- **Vector-field emitters** — wind / gravity wells beyond global
  `acceleration_x/y`; tracked in `docs/todo-v7.md` "Deferred to post-v7".
- **Spawn-time hooks** — mutate particle at spawn (parent-velocity
  inheritance, palette gradient sampling).
- **Sorted-keyframe caching** — `render_values` clones + sorts on every
  call; cache the sorted `Vec` at spawn if profiling shows it.
- **Character morph mode** — crossfade two glyphs across a segment
  instead of the step pick.
- **Per-particle trail / trajectory indicator** — manual note #9 asked
  for a visible vector-of-travel; not yet shipped.
