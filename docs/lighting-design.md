# Dynamic Lighting System — Design Specification

**Status:** Partially Implemented (FIGfont density path, v7.4) | **Priority:** Very High  
**Version:** 0.2.0 | **Date:** 2026-07-11

---

## 1. Overview & Goals

Bring 3D-like illumination to ASCII-art canvases in the Figby TUI editor.
The dynamic lighting system enables users to define scene lights (point,
directional, ambient) and compute per-cell shading based on surface normals
derived from FIGfont glyph bitmaps or painted canvas content. The result is a
live-updated palette swap and character-intensity remapping that gives ASCII
art a sense of depth, material, and directional illumination.

**Primary goals:**
- Derive surface normals from FIGfont glyph pixel density or canvas height data
- Define and manage scene lights (positional, directional, ambient)
- Compute per-cell Lambertian diffuse + optional specular shading
- Cast hard shadows from opaque ASCII cells via 2D grid raycasting
- Remap palette colors and character brightness based on computed luminance
- Update the canvas render in real-time as lights/normals/palettes change

---

## 2. Normal-Map Generation

Two complementary approaches, chosen based on data source:

### 2.1 FIGfont Path

For FIGfont-rendered text layers, derive per-cell surface normals from glyph
bitmap pixel density:

1. **Heightfield from fill density:** For each canvas cell containing a FIGcharacter,
   compute the fill ratio `density = filled_pixels / total_pixels` where "filled"
   means non-space rows in the FIGcharacter at that cell position. This gives a
   **height** value `h ∈ [0.0, 1.0]` per cell.

2. **Finite-difference gradient:** Apply Sobel (3×3) or central-difference kernel
   across the heightfield to obtain per-cell gradient `(dx, dy)`.

3. **Normal from gradient:**
   ```
   nx = -dx * height_scale
   ny = -dy * height_scale
   nz = 1.0  (constant, or derived from curvature)
   normal = normalize(nx, ny, nz)
   ```
   `height_scale` controls how "bumpy" the surface appears (default: 0.5, range 0.1–2.0).

4. **Edge handling:** Mirror padding for Sobel at borders.

### 2.2 Canvas Path

For user-painted canvas layers, the normal map is derived from user-assigned
height values stored per cell:

- Each `CanvasCell` gains an optional `height: Option<u8>` field (default `None` = flat `(0,0,1)`).
- User can paint height via brush with `snap_to_grid` + `height_paint` mode in a future tool.
- Heightfield → gradient → normal uses the same Sobel pipeline as the FIGfont path.

### 2.3 Data Structure

```rust
/// Per-cell normal vector in tangent space.
/// Stored as 3 × i8 for compactness (−127..=127 per component).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Normal3(pub i8, pub i8, pub i8);

impl Normal3 {
    pub fn from_f32(x: f32, y: f32, z: f32) -> Self {
        Normal3(
            (x.clamp(-1.0, 1.0) * 127.0) as i8,
            (y.clamp(-1.0, 1.0) * 127.0) as i8,
            (z.clamp(-1.0, 1.0) * 127.0) as i8,
        )
    }
    pub fn to_f32(self) -> (f32, f32, f32) {
        (
            self.0 as f32 / 127.0,
            self.1 as f32 / 127.0,
            self.2 as f32 / 127.0,
        )
    }
}

/// Normal map: aligned 1:1 with canvas dimensions.
pub struct NormalMap {
    pub cells: Vec<Vec<Normal3>>,
    pub width: u16,
    pub height: u16,
}
```

Implementation lives in new module `figby-rs/src/tui/lighting.rs`.

---

## 3. Scene Lights

### 3.1 Light Types

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Clone, Debug)]
pub struct Attenuation {
    pub constant: f32,   // 1.0 typical
    pub linear: f32,     // 0.09 typical
    pub quadratic: f32,  // 0.032 typical
}

impl Default for Attenuation {
    fn default() -> Self {
        Self { constant: 1.0, linear: 0.09, quadratic: 0.032 }
    }
}

#[derive(Clone, Debug)]
pub enum Light {
    Ambient {
        intensity: f32,
        color: Rgb,
    },
    Directional {
        direction: (f32, f32, f32),  // unit vector
        intensity: f32,
        color: Rgb,
    },
    Point {
        position: (f32, f32, f32),  // (x, y, z) in canvas cell units
        intensity: f32,
        color: Rgb,
        attenuation: Attenuation,
    },
}
```

### 3.2 Scene

```rust
#[derive(Clone, Debug)]
pub struct Scene {
    pub lights: Vec<Light>,
}
```

- `Scene` holds zero or more lights.
- Ambient lights sum their contribution (max 1.0 per cell).
- Point and directional lights are each evaluated per cell with shadow test.
- The TUI editor (in a future phase) provides UI to add, select, move, and
  configure per-light properties (position via arrow keys, intensity/color
  via palette picker).

---

## 4. Shading Model

Per-cell fragment computation, evaluated once per light:

### 4.1 Lambertian Diffuse

```
let n = normal.to_f32();          // unit normal at cell
for light in &scene.lights {
    match light {
        Light::Ambient { intensity, color } => {
            ambient += color * intensity;
        }
        Light::Directional { direction, intensity, color } => {
            let ndotl = max(dot(n, direction), 0.0);
            let shadow = cast_shadow(cell_pos, direction);
            if !shadow {
                diffuse += color * (ndotl * intensity);
            }
        }
        Light::Point { position, intensity, color, attenuation } => {
            let light_dir = normalize(position - cell_pos_3d);
            let ndotl = max(dot(n, light_dir), 0.0);
            let dist = length(position - cell_pos_3d);
            let atten = 1.0 / (attenuation.constant
                + attenuation.linear * dist
                + attenuation.quadratic * dist * dist);
            let shadow = cast_shadow(cell_pos, light_dir);
            if !shadow {
                diffuse += color * (ndotl * intensity * atten);
            }
        }
    }
}
let luminance = min(ambient + diffuse, 1.0);
```

### 4.2 Specular (Optional, Blinn-Phong)

Enabled per-palette via a `specular: bool` flag on the palette config:

```
let h = normalize(light_dir + view_dir);  // half-vector
let spec = pow(max(dot(n, h), 0.0), shininess);
specular += color * (spec * intensity);
```

`view_dir` defaults to `(0, 0, 1)` (looking straight into the canvas plane).
`shininess` default 32.0, configurable per palette entry.

### 4.3 Per-Cell Output

Shading produces per-cell **luminance** `f32` in `[0, 1]` and a per-cell
**tint** `Rgb` (light color influence). These feed into the palette LUT
(§6) and char-map selection (§7).

---

## 5. Shadow Casting

### 5.1 Raycast on ASCII Grid

For each point/directional light, trace a ray from the fragment position
toward the light source. Step through canvas cells using DDA (Digital
Differential Analyzer) on the 2D grid:

```rust
fn cast_shadow(
    scene: &Scene,
    normal_map: &NormalMap,
    cell_x: u16,
    cell_y: u16,
    light: &Light,
    layers: &[Layer],
) -> bool {
    // Only point and directional lights cast shadows.
    match light {
        Light::Ambient { .. } => return false,
        Light::Directional { direction, .. } => {
            // Trace from (cell_x, cell_y) along -direction
            // Step through canvas cells, check each for content + casts_shadow
        }
        Light::Point { position, .. } => {
            // Trace from (cell_x, cell_y) toward (position.x, position.y)
            // Step through canvas cells
        }
    }
    false
}
```

### 5.2 DDA Grid Traversal

Algorithm adapted from Amanatides & Woo (1987) for 2D grid traversal:

1. Compute ray direction from cell center to light source.
2. Initialize `t_max` for first grid boundary crossing in x and y.
3. Step into next cell — if cell has content (non-space char) AND that
   layer has `casts_shadow = true`, return `true` (blocked).
4. Stop if ray exits canvas bounds or exceeds `max_shadow_distance`.
5. Return `false` if no occluding cell found.

### 5.3 Soft Shadows (PCF, Future)

First implementation uses binary shadow (visible/blocked). A future
enhancement can use Percentage Closer Filtering (PCF) with 4–16 jittered
rays per light for penumbra edges.

### 5.4 Performance

- Shadow raycast runs per cell per point/directional light.
- With a 100×50 canvas and 3 non-ambient lights: ~15,000 raycasts.
- Each raycast traverses ~10–100 cells worst-case.
- Total: ~150k–1.5M cell checks per frame.
- Mitigations:
  - Shadow cache: recompute only when lights or cell content changes.
  - Distance-limited rays (`max_shadow_distance`, default 50 cells).
  - Threaded compute for large canvases (future).

---

## 6. Per-Palette LUT

The palette lookup table (LUT) maps luminance `[0, 1]` to final color + char
for each base palette entry.

### 6.1 LUT Structure

```rust
/// Pre-computed output for a single luminance level.
#[derive(Clone, Debug)]
pub struct LutEntry {
    pub fg_color: (u8, u8, u8),  // 24-bit RGB foreground
    pub bg_color: Option<(u8, u8, u8)>,
    pub ch: char,                // character representing this brightness
}

/// Full LUT: one entry per luminance step (0..=255).
pub struct LightingLut {
    pub entries: Vec<LutEntry>,    // length 256
}
```

### 6.2 LUT Generation

```
for brightness in 0..=255 {
    let t = brightness as f32 / 255.0;
    // Interpolate between shadow_color (t=0) and lit_color (t=1)
    let color = lerp(shadow_color, lit_color, t);
    // Select char from intensity-to-char map (§7)
    let ch = intensity_to_char(t, &char_map);
    entries[brightness] = LutEntry { fg_color: color, ch, .. };
}
```

Each palette entry in the base palette maps to two output entries defined
in the palette config:

```yaml
# In palette YAML:
palette:
  - name: "Red"
    fg: [255, 0, 0]
    lit_color: [255, 128, 128]     # bright tint
    shadow_color: [80, 0, 0]       # dark tint
    specular: false
    shininess: 32.0
```

If `lit_color` / `shadow_color` are absent, defaults are:
- `lit_color = fg` (no tint shift on lit side)
- `shadow_color = fg * 0.3` (darken by 70% for shadow side)

### 6.3 Palette Swap

When user selects a different base palette:
1. Read base palette colors from palette YAML.
2. For each entry, compute `lit_color` and `shadow_color` from palette config.
3. Regenerate `LightingLut` for the current scene luminance state.
4. Canvas redraws with new LUT — instant feedback.

---

## 7. Output Pipeline

### 7.1 Render Flow

```
Scene change / light edit / palette change
       │
       ▼
[1] Normal map recompute
    (if heightfield or cell content changed)
       │
       ▼
[2] Per-cell shading pass
    for each cell with accepts_lighting = true:
      compute luminance from normals + lights + shadows
       │
       ▼
[3] LUT lookup
    index = (luminance * 255) as u8
    char = lut[index].ch
    fg = lut[index].fg_color
    bg = lut[index].bg_color
       │
       ▼
[4] Canvas widget render
    draw char with fg/bg into terminal cell
```

### 7.2 Integration Point

The shading pass hooks into the canvas rendering pipeline after the layer
compositing (`draw()`) but before the frame is committed to ratatui's
`Buffer`. The lighting system holds a reference to the post-composite
`CanvasBuffer` and maps each cell's output char + colors through the LUT.

### 7.3 Char Map Reuse

Reuse `DEFAULT_CHAR_MAP` from `image_input.rs` (` .-:=+*#%@`) as the
default brightness-to-character mapping. Each luminance level maps to
a character index via:

```rust
fn intensity_to_char(t: f32, char_map: &[char]) -> char {
    let idx = (t * (char_map.len() - 1) as f32).round() as usize;
    char_map[idx.min(char_map.len() - 1)]
}
```

---

## 8. Per-Object Flags

Layer-level properties that control lighting behavior:

```rust
#[derive(Clone, Debug)]
pub struct Layer {
    // ... existing fields ...

    /// If true, this layer receives shading from scene lights.
    pub accepts_lighting: bool,

    /// If true, this layer blocks light for layers below.
    pub casts_shadow: bool,
}
```

**Defaults:**
- `accepts_lighting: true` — most layers should be shaded.
- `casts_shadow: true` — solid content layers cast shadows.

Layer compositing order determines overhang: a top layer casts shadows on
layers below it. Layers with `accepts_lighting = false` render unshaded
(useful for UI overlays, borders, labels).

These flags are toggled in the Layers panel (existing TUI component).

---

## 9. Data Flow Diagram

```
┌─────────────┐     ┌──────────────┐     ┌──────────────────┐
│  Scene      │────▶│  Shading     │────▶│  LightingLut     │
│  Lights     │     │  Engine      │     │  (Palette LUT)   │
└─────────────┘     │  (lighting   │     └──────────────────┘
         │          │   .rs)       │                │
         ▼          └──────┬───────┘                ▼
┌─────────────┐            │              ┌──────────────────┐
│  NormalMap  │◀───────────┘              │  LutEntry        │
│  (height    │                            │  lookup          │
│   → grad    │                            │  (char + color)  │
│   → normal) │                            └──────────────────┘
└─────────────┘                                     │
         │                                          ▼
         ▼                                 ┌──────────────────┐
┌─────────────┐     ┌──────────────┐       │  Canvas Widget   │
│  Heightfield│────▶│  Per-Cell    │       │  Render          │
│  (FIGfont   │     │  Shading     │       │  (ratatui)       │
│   density / │     │  (diffuse +  │       └──────────────────┘
│   painted)  │     │   shadow)    │
└─────────────┘     └──────────────┘
```

**State invalidation triggers:**
- Light added/moved/removed → re-shade all cells (step 2).
- Normal map changed (font re-render, height paint) → recompute normals → re-shade.
- Palette changed → regenerate LUT → re-map all cells (step 3).
- Layer flag toggle (`accepts_lighting`/`casts_shadow`) → re-shade affected cells.

---

## 10. Integration Points

Implementation will touch or create these files:

| File | Role |
|------|------|
| `figby-rs/src/tui/lighting.rs` | **New** — core lighting engine (Scene, Light, NormalMap, LightingLut, shading, shadow casting) |
| `figby-rs/src/tui/layers.rs` | Add `accepts_lighting`, `casts_shadow` flags to `Layer` |
| `figby-rs/src/tui/palette.rs` | LUT generation from palette; `lit_color`/`shadow_color` fields on palette entries |
| `figby-rs/src/tui/canvas.rs` | `CanvasCell` gains optional `height: Option<u8>`; canvas exposes post-composite buffer for shading pass |
| `figby-rs/src/tui/components/canvas.rs` | Shading pass hook after layer compositing, before frame commit |
| `figby-rs/src/tui/mod.rs` | Key dispatch for light editing mode; `Scene` field on `TuiApp` |
| `figby-rs/src/tui/mod.rs` | Light management UI (add/select/move/edit lights) |

---

## 11. Deferred / Not Designed

These features are explicitly out of scope for the initial design:

- Normal-map painting UI (user painting height values directly on canvas)
- Light gizmo rendering in the canvas (visual handles for light positions)
- Per-cell emissive / PBR materials (metalness, roughness, ambient occlusion)
- Multi-bounce indirect lighting or radiosity
- Real-time light preview during move/edit handles
- GIF/animation export with lighting changes across frames
- Light animation (moving lights, pulsing intensity over time)
- Multi-layer shadow interaction (shadows only from layers below, not siblings)

---

## 12. Open Questions

| Question | Options | Recommendation |
|----------|---------|----------------|
| Coordinate system for 3D light positions | Z-up vs Y-up; units = canvas cells vs world units | Z-up, 1 unit = 1 canvas cell width. Canvas at z=0. |
| Normal map resolution | 1 per cell (cell-aligned) vs sub-cell for FIGfont glyphs | Start 1 per cell. Sub-cell requires per-glyph Sobel on glyph bitmap. |
| Shadow ray stepping strategy | DDA on grid vs fixed-step vs hierarchical | DDA (Amanatides & Woo) — efficient, widely used. |
| Performance budget | Real-time 30fps vs deferred render update | Deferred: recompute shading only on change. Budget: <16ms for full recompute on typical 100×50 canvas. |
| View direction for specular | Fixed `(0,0,1)` vs camera position | Fixed `(0,0,1)` — camera is always looking straight at the canvas. |
| Height scale for normals | Single global value vs per-layer | Single global `height_scale` initially; per-layer if users need varied bumpiness. |

---

## 13. Future Test Strategy

When implementation begins, the following test categories will apply:

- **Unit tests:**
  - Normal3 construction, quantization round-trip, clamp bounds
  - Heightfield → gradient → normal pipeline (known height patterns)
  - Light contribution computation (single light, multiple lights, edge cases)
  - Shadow raycast (occluded, non-occluded, edge-of-bounds, within max distance)
  - LUT generation (linear interpolation, default lit/shadow colors)
  - Intensity-to-char mapping (endpoints, midpoint, out-of-range)

- **Integration tests:**
  - Full pipeline: scene + normal map + LUT → shaded cell output
  - Palette swap triggers LUT regeneration
  - Layer flag toggle affects shading inclusion
  - TUI key dispatch for light editing (add/remove/adjust light)

- **Performance benchmarks:**
  - Full scene recompute (100×50 canvas, 5 lights)
  - Shadow raycast cost per cell per light
  - LUT regeneration time per palette entry count

---

## Design History

| Date | Change |
|------|--------|
| 2026-06-17 | Initial design (v0.1.0) |
