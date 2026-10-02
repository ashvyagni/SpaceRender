# Legacy prototype (not built)

The original hand-written wgpu/winit renderer and its ECS crates. They are kept for
reference only and are excluded from the Cargo workspace.

Why they were retired, and what was carried forward, is explained in
[docs/ENGINE_ASSESSMENT.md](../docs/ENGINE_ASSESSMENT.md). In short: the f64 math
(`cosmogon_core`), the orbital mechanics (`cosmogon_physics`) and the real Solar System data
(now `crates/cosmogon_sim/data/sol.toml`) were kept; the renderer, which had no UI, no
post-processing in the frame and a units bug that put every planet beyond the far plane, was
replaced by the Bevy application in `crates/cosmogon`.
