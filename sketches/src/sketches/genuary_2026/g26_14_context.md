# Genuary 2026 Day 14 - Dreams Context

## Project Overview

Creating a shader for Genuary 2026 prompt "everything fits perfectly" with
Jamuary theme "A Tender Kind of Dreams" (Dreamy, 81 BPM, A Phrygian, Bone White
color).

## Current Implementation

Working files:

- `g26_14_dreams.wgsl` - Main shader
- `g26_14_dreams.yaml` - Control definitions
- `g26_14_dreams.rs` - Rust setup (8 banks)

## Features Implemented

### 1. Voronoi Pattern with Rounded Boxes

- Uses rounded box distance metric instead of circular
- Tiles fit together perfectly
- FBM noise mixed with Voronoi for organic variation
- Posterization creates topographic contour levels
- Edge detection for depth

### 2. Scrolling

- `scroll_down` (d1): Checkbox - direction control
- `scroll_speed` (d2): 0.0-1.0 - speed multiplier

### 3. Tile Extrusion (Pulsing)

- `extrude_amount` (d3): 0.0-0.5 - how much tiles grow/shrink
- `extrude_frequency` (d4): 0.1-10.0 - pulse speed
- Each tile pulses independently based on cell hash

### 4. Cell-Based Gaps

- `gap_amount` (e1): 0.0-1.0 - percentage of cells to remove
- Entire Voronoi cells disappear, showing white/light background
- Random selection based on cell hash

### 5. Density Waves (CURRENT ISSUE)

- `wave_enabled` (e2): Checkbox - toggle waves
- `wave_invert` (f1): Checkbox - invert density pattern
- `wave_speed` (e3): 0.1-2.0 - movement speed
- `wave_scale` (e4): 0.0-0.5 - density multiplier

**Current Implementation:**

```wgsl
var local_scale = scale;
if wave_enabled {
    let movement = vec2f(
        sin(time * wave_speed * 0.2) * 3.0,
        time * wave_speed * 0.5
    );
    let noise_pos = scrolled_pos * 0.8 + movement;

    let n1 = fbm(noise_pos, 3);
    let n2 = fbm(noise_pos * 1.3 + vec2f(5.2, 1.3), 3);

    let combined = n1 * n2;
    var sharp = pow(combined, 4.0);

    if wave_invert {
        sharp = 1.0 - sharp;
    }

    local_scale = mix(scale, scale * (1.0 + wave_scale), sharp);
}
```

## CURRENT PROBLEM

The density waves create a "magnifying glass" effect - continuous smooth
transitions between densities that distort the pattern rather than creating
discrete regions of different density tiles.

**User's Goal:** Send subtle pulses of higher-density Voronoi waves through the
canvas. Like the screenshots shown - areas with scale=8 mixed with areas with
scale=16, creating irregular "snake-like" waves of denser tiles moving through.

**What Doesn't Work:**

- Uniform horizontal wave bands
- Continuous smooth transitions (creates magnifying glass distortion)
- Large regions (too coarse)
- Direct scale mixing (creates interference patterns)

**Key Insight from User:** The pattern needs distinct regions where the tiles
themselves are at different scales, not a gradual distortion. Need to avoid
"corny" effects like whirlpools, magnifying glass looks, or anything that feels
mechanical.

## User Coding Conventions

- NO inline comments (always above)
- NO column alignment with extra tabs/spaces
- "Main up top" organization pattern
- Lines should not exceed 80 characters
- Always use Edit tool (never Write) for existing files

## Color Scheme

- Bone white: `vec3f(0.96, 0.96, 0.92)`
- Background (in tiles): `vec3f(0.05, 0.05, 0.08)`
- Gap background: `vec3f(0.96, 0.96, 1.0)`

## Parameters Available

- a.x, a.y: width, height
- a.z: time (also available as `params.a.z` directly)
- b1-b4: scale, octaves, brightness, contrast
- c1-c4: contour_levels, contour_smoothness, depth_strength, roundness
- d1-d4: scroll_down, scroll_speed, extrude_amount, extrude_frequency
- e1-e4: gap_amount, wave_enabled, wave_speed, wave_scale
- f1: wave_invert

## Things That Were Tried and Failed

### Warp Points (Removed)

- Localized points that warped the pattern
- Tried: inward pinch, rotation, color influence, cell shifts
- All created "corny whirlpool effects"

### Gap Fills (Removed)

- Tried: flowing lines, halos with nested rings, gradients
- Nothing looked good, all removed

### Subdivision Approach (Removed)

- Attempted to subdivide individual Voronoi cells
- Made pattern look bad with sharp discontinuities

### Various Wave Approaches (All Failed)

- Uniform sine waves (too mechanical)
- Threshold-based binary switching (still too abrupt)
- Simple FBM modulation (too smooth, magnifying glass)
- Current dual-FBM with pow(4.0) (still magnifying glass)

## Next Steps

Need to find a way to create irregular, organic patches of higher-density tiles
that:

1. Don't create smooth transitions (avoid magnifying glass)
2. Move through the canvas in organic "snake-like" patterns
3. Feel dreamy and fit the "everything fits perfectly" theme
4. Are subtle enough to work at small wave_scale values (0.1-0.5)

Possible directions to explore:

- Cell-level scale selection (similar to gap approach)
- Multiple discrete scale layers composited
- Different algorithmic approach entirely
