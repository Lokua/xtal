# auto_un Performance Notes (Short Handoff)

Date: 2026-02-10
Target: regain stable 60 FPS for recording headroom.

## Baseline Context

- User-reported baseline before this pass: ~50 FPS.
- Regressive branch dropped to ~43 FPS (or worse).
- Large artifacts from aggressive culling are unacceptable.

## Tried And Rejected

1. Satellite full precompute in `prepare_scene_state`
- Precomputed satellite centers/strands for every fragment up front.
- Result: major regression (~10 FPS worse).
- Why: expensive trig ran even on background pixels.

2. Lazy satellite precompute helper (`prepare_satellite_instances`)
- Deferred full satellite prep until SDF path.
- Still failed to recover baseline in user testing (~43 FPS).
- Removed with rollback to avoid carrying uncertain complexity.

3. Broad cache rewrite bundle (harmonic/radius private caches + precompute path)
- Bundled with the regressive changes above.
- Net result was negative in real scene usage.
- Do not reintroduce as a bundle without per-step profiling.

## Kept (Safe) Wins

1. Ray-march scene bounding-sphere gate
- Added `ray_sphere_interval` in `ray_march`.
- If ray misses overall scene bound, return `MAX_DIST` immediately.
- March starts at sphere entry and exits near sphere exit.
- Goal: skip nearly all SDF work for background pixels.

2. Cached satellite cluster bound
- `g_sat_cluster_bound` is computed once in `prepare_scene_state`.
- `scene_sdf` broad-phase reuses it instead of recomputing each call.

## Guardrails

- Prefer transparent wins first (background rejection, bound reuse).
- Avoid geometry approximations that can introduce holes/arcs/artifacts.
- Verify with user’s heavy preset (large radius + recording intent), not
  synthetic settings.
