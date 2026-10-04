# Engine Alternatives — Premium 2D/2.5D Rust Roguelike

> Research-only. Verified **2026-09-23** (facts re-fetched this date from crates.io API + GitHub API after the original research pass). Companion to [bevy-engine.md](bevy-engine.md) (Bevy deep dive) and [visual-goals.md](visual-goals.md) (target look: HD-2D pixel + lights).

Scope constraint from the project: the simulation (model/engine/social/persist/laya_client, incl. tokio) **must keep compiling as a pure-Rust library**; terminal TUI stays the default view; single dev; Windows-first; target = lit 2D sprites + post stack (Target C), not full 3D production.

---

## 1. Bevy (baseline)

One-line: covered in depth by the sibling file — **0.19.1 stable** (0.20-rc.1 cut 2026-09-15), ECS+wgpu, built-in Bloom/Tonemapping/Vignette, no first-party 2D lighting (ecosystem + custom shader), breaking cadence 3–4 months. See [bevy-engine.md](bevy-engine.md).
Sources: <https://github.com/bevyengine/bevy/releases>, <https://crates.io/crates/bevy> (0.20.0-rc.1, 2026-09-15)

## 2. Fyrox

| Row | Finding |
|---|---|
| Latest + cadence | **1.0.1** (crates.io 2026-03-28); v1.0.0 announced 2026-03-29 after ~7 years — first stability commitment | <https://crates.io/crates/fyrox>, <https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/> |
| Health | 9.5k★, pushed **2026-09-22** (day before verification), 60 open issues — active, but ~1 core maintainer (bus-factor risk like most engine projects) | <https://api.github.com/repos/FyroxEngine/Fyrox> |
| Renderer | Custom engine renderer (OpenGL-derived core; 3D PBR heritage) **not wgpu** — 2D via `scene::dim2` module (sprite/rectangle/particle nodes, `dim2` index) | <https://docs.rs/fyrox-impl/latest/fyrox_impl/scene/dim2/index.html>, <https://raw.githubusercontent.com/FyroxEngine/Fyrox/master/README.md> |
| 2D dynamic lighting | No first-party 2D-light pipeline identified; realistic route = orthographic camera + 3D lights over 2D sprites (works, but it's the 3D path wearing 2D clothes) | (negative result of targeted search, 2026-09-23) |
| Normal-mapped 2D sprites | 3D materials yes; per-sprite 2D normal maps not a shipped feature of `dim2` | same search |
| Post-processing | Yes — 3D-grade pipeline (bloom, tonemapping, etc. per renderer docs) | <https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/> |
| Particles | Built-in particle system nodes (3D heritage; usable in `dim2`) | <https://docs.rs/fyrox/1.0.1/fyrox/scene/index.html> |
| Isometric | Orthographic camera trivial; no 2D iso-tilemap tooling — own transform, same as Bevy custom | — |
| UI | **Own retained UI crate (fyrox-ui)** + scene editor **FyroxEd** (scene graph, assets, hot keys) — the only Rust engine here with a real editor | <https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/> |
| WASM | Not a focus; desktop-first engine | README |
| License | MIT | crates.io |
| Sim-as-library fit | Good — Rust scripting plugin model; sim crate stays a dependency. Note the engine *drives* the loop (scene update model), so bridge = per-frame snapshot like Bevy | <https://fyrox-book.github.io/scripting/rust/rust.html> |
| Iteration | Rust-script **hot-reload** documented; editor asset caching; compile cost ≈ Bevy-class | <https://fyrox-book.github.io/scripting/rust/rust.html> |
| Community | Small-to-medium (9.5k★ vs Bevy 40k★+); forum+Discord exist | GitHub |
| Fit | Credible #2-in-Rust: editor + stability era, but the lit-2D story is *weaker* than Bevy's ecosystem — exactly opposite of our target where lighting is the whole plan | — |

## 3. Godot 4 + Rust (godot-rust/gdext)

| Row | Finding |
|---|---|
| Latest + cadence | **Godot 4.7.2-stable** (2026-08-18); **gdext 0.5.5** (2026-08-09); both active (repo pushes 2026-09-22/23) | <https://github.com/godotengine/godot/releases>, <https://github.com/godot-rust/gdext/releases> |
| Health | 117k★ engine / 5.2k★ gdext — healthiest project on this list | <https://api.github.com/repos/godotengine/godot> |
| Renderer | Vulkan Clustered/Forward+/Mobile; **first-party 2D lighting**: PointLight2D/DirectionalLight2D, LightOccluder2D shadows, CanvasItem **normal maps**, 2D HDR (4.4 rework) | <https://docs.godotengine.org/en/stable/tutorials/2d/2d_lights_and_shadows.html>, <https://godotengine.org/releases/4.4/> |
| Lit/normal-mapped sprites | **Native** — the only candidate where this is zero shader work | Godot docs above |
| Post-processing | Per-canvas-item shaders + HDR 2D pipeline (4.4+, tonemap/glow available in 2D) | <https://godotengine.org/releases/4.4/> |
| Particles | GPUParticles2D — mature, editor-authored | docs |
| Isometric | First-class 2D transforms; tilemaps incl. isometric layer shape | docs |
| UI | **Best-in-class** for modal RPG screens (Control nodes + theme system; the editor *is* a UI tool) | docs |
| WASM | Engine yes; **gdext WASM export is the weak leg** (godot-rust book documents caveats/toolchain friction) | <https://godot-rust.github.io/book/toolchain/export-web.html> |
| License | MIT (engine + gdext) | repos |
| Sim-as-library fit | **The FFI tax is the decision**: sim compiles into a GDExtension library; Godot drives frames, Rust sim answers via exported calls; ALL engine types cross an ABI boundary and hot-reload of the Rust lib works from Godot 4.2+. Feasible and documented — but every gameplay call becomes boundary-shaped, and the terminal TUI would link the same sim crate natively (two front-ends, one foreign — fine, but the *main* front-end becomes C++/GDScript-flavoured) | <https://godot-rust.github.io/book/> |
| Iteration | Godot-side: instant (scene/art/scriptless tweaks). Rust-side: recompile + reload cycle; GDExtension hot-reload exists (4.2+) | <https://github.com/godot-rust/gdext> |
| Community | Largest on the list by far | GitHub |
| Fit | The strongest *capabilities-per-dollar* candidate — rejected on shape: the editor's value is authored scenes/UI, and a seeded procedural roguelike has almost none of the former; we'd pay the permanent FFI + dual-language tax mostly to buy the first-party 2D lighting | — |

## 4. wgpu + winit (hand-rolled)

| Row | Finding |
|---|---|
| Latest | **wgpu 30.0.1** (2026-08-22), **winit 0.31-beta.3** (2026-09-04); both hyperactive (18k★/pushes daily) | <https://crates.io/crates/wgpu>, <https://api.github.com/repos/gfx-rs/wgpu> |
| What it is | The exact stack Bevy/vello/ggez are built on: GPU abstraction + windowing, **nothing else** — no scenes, sprites, assets, text, audio, UI, particles | <https://github.com/gfx-rs/wgpu> |
| 2D lighting / normals / post / particles | Whatever you write in WGSL. Full control = full cost; realistically weeks before first light ray | — |
| UI | egui integrates well on raw wgpu/winit (community pattern), or write your own | — |
| WASM | Yes (wgpu = the WASM-capable backend) | repo README |
| License | MIT/Apache-2.0 | repos |
| Sim-as-library fit | Perfect (it's just crates) | — |
| Iteration | Fastest compile loop on the list; slowest feature loop on the list | — |
| Community | Large (as infrastructure, not as game engine) | GitHub |
| Fit | Only the right answer if the goal becomes "own the renderer forever"; for shipping a game it's the long way to what Bevy hands us on day one | — |

## 5. ggez & macroquad (Rust 2D microframeworks)

| Row | ggez | macroquad |
|---|---|---|
| Latest + cadence | **0.10.0** (crates.io 2026-06-03; repo pushed 2026-08-24; GitHub release tags lag at 0.9.3/2023) | **0.4.16** (2026-07-30; pushed 2026-08-18) |
| Health | 4.7k★, 72 open issues, small-team pace | 4.6k★, **338 open issues**, single-maintainer shape |
| Renderer | wgpu, 2D-only (Canvas/quads; LÖVE-inspired) | OpenGL/WebGL via miniquad, immediate-mode 2D |
| Lighting / normals / post | None built-in; custom WGSL shaders + render-to-texture canvases — *our light map is writable here, we'd write the whole post stack too* | None built-in; custom material shaders exist (quad_gl) — the ceiling the project already hit: CPU-shaped art, no lighting pipeline, alpha-blend-only fades |
| UI | None (own UI or egui integration) | rudimentary `ui` module; project already bypasses it |
| WASM | Experimental | Yes (first-class) |
| License | MIT | MIT |
| Sim-as-library fit | Perfect | Perfect (today's arrangement) |
| Community | Small | Small-medium |
| Fit | Same ceiling class as macroquad with marginally better shader API — migrating sideways buys nothing but a port | Current engine; "stay" baseline = v0.1 fallback (path A) |

Sources: <https://crates.io/crates/ggez>, <https://crates.io/crates/macroquad>, <https://api.github.com/repos/ggez/ggez>, <https://api.github.com/repos/not-fl3/macroquad>, <https://raw.githubusercontent.com/ggez/ggez/master/CHANGELOG.md>, <https://raw.githubusercontent.com/not-fl3/macroquad/master/LICENSE-MIT>

## 6. raylib (+ raylib-rs bindings)

| Row | Finding |
|---|---|
| Latest + cadence | Core **raylib 6.0** (2026-04-23; 34.8k★, very active); Rust crate **`raylib` 6.0.0** (2026-06-10); bindings repo moved to the `raylib-rs` org | <https://github.com/raysan5/raylib/releases>, <https://crates.io/crates/raylib>, <https://api.github.com/repos/raylib-rs/raylib-rs> |
| Renderer | C99 immediate-mode 2D/3D over OpenGL — macroquad's spiritual parent, bigger and older |
| Lighting / normals / post | Nothing engine-side for 2D; DIY shaders (known community `rlights` 2D-lighting pattern); 3D side has full materials | <https://github.com/raysan5/raylib> |
| UI | raygui (immediate) — thin for modal RPG menus |
| WASM | Yes (via emscripten, bindings follow) |
| License | zlib (core) / MIT-ish bindings |
| Sim-as-library fit | Good technically — but adds a **C toolchain + FFI surface** to a currently pure-Rust build tree for *no capability we don't already have* |
| Fit | Strictly dominated for us: same ceiling class as macroquad plus a C dependency |

---

## 7. Comparison matrix

| Criterion | Bevy | Fyrox | Godot+gdext | wgpu+winit | ggez | macroquad (current) | raylib-rs |
|---|---|---|---|---|---|---|---|
| Latest (2026-09) | 0.19.1 (0.20-rc out) | 1.0.1 | 4.7.2 + gdext 0.5.5 | wgpu 30.0.1 | 0.10.0 | 0.4.16 | 6.0.0 |
| Cadence risk | High (breaking 3–4 mo) | Low–Med (1.0 era) | Med (4.x stable; gdext ABI pins) | High (raw churn) | Low | Low | Low |
| First-party 2D lighting | ✗ (ecosystem+custom) | ✗ (3D workaround) | **✓** | ✗ DIY | ✗ DIY | ✗ DIY | ✗ DIY |
| Normal-mapped 2D sprites | custom Material2d | ✗ | **✓ native** | DIY WGSL | DIY WGSL | ✗ | DIY GLSL |
| Post stack | **built-in 0.19** | built-in (3D-grade) | built-in (2D HDR) | DIY | DIY | ✗ | DIY |
| Editor | none official | **FyroxEd** | **best-in-class** | none | none | none | none |
| UI for modal RPG menus | egui/bevy_ui | fyrox-ui | **Control+theme** | egui DIY | DIY | own (shipped) | raygui |
| Sim = pure-Rust crate | **✓ zero FFI** | ✓ | FFI (GDExtension ABI) | ✓ | ✓ | ✓ | C FFI dep |
| Audio/input/gamepad | full | full | full | DIY | 2D basics | 2D basics | full |
| WASM | ✓ | weak | engine ✓ / gdext caveats | ✓ | exp. | ✓ | ✓ |
| Ecosystem size | large | small-medium | largest | infra-large | small | small | medium |
| Iteration (solo dev) | slow-ish build, code-driven | hot-reload + editor | editor fast / Rust FFI slow | fast build, slow features | fast everything | fastest | medium |

## 8. Scored recommendation for Laya Realms

Scored against: Target C look (lit pixel sprites + post) · pure-Rust sim preserved · single dev · TUI stays default · procedural (not scene-authored) content.

| Rank | Engine | Score | One-liner |
|---|---|---|---|
| **1** | **Bevy** | 8/10 | Only option that's Rust-native *and* ecosystem-deep exactly where the plan's risk sits (lighting, post, particles); cadence tax is manageable by pinning |
| **2** | Godot 4 + gdext | 6.5/10 | Best lighting/editor in the set; pays for it with a permanent FFI + dual-language tax on a project whose content is generated, not scene-authored — and the TUI would forever be the second-class front-end |
| **3** | Fyrox | 5.5/10 | Rust-native + real editor + fresh 1.0 stability; loses precisely on the point that matters: weaker lit-2D story than Bevy's ecosystem, far smaller community |
| 4 | wgpu + winit | 4/10 | Fine craftsmanship, wrong job: rebuild Bevy's decade alone |
| 5 | ggez | 3/10 | Sideways move from macroquad |
| 6 | raylib-rs | 2.5/10 | C FFI tax for capabilities we already have |
| 7 | macroquad (stay) | 2/10 | v0.1 fallback (path A) — ceiling is the problem being solved |

**Top pick keeps: Bevy** — the alternatives research changes the *confidence*, not the *answer*: no Rust-native competitor out-ranks it on the lit-2D target, and the only engine that beats its 2D capability (Godot) imposes an architecture (FFI-driven, editor-shaped) that fights a generated, sim-first, Rust-everywhere project.

**Top pick's top-3 risks** (into the decision register):
1. **Breaking cadence (3–4 months) + ecosystem plugin lag** — mitigate: pin one version per wave, migrate between waves only, keep third-party surface minimal (light_2d, hanabi, ui picked at E4/E6 latest).
2. **The core visual mechanism is custom anyway** — the projected light map from sim state is a custom render-to-texture pass regardless of engine; Bevy provides no shortcut for it (nothing does — it's *our* data). Its WGSL cost is the same in every candidate, while Bevy pays the *rest* (bloom, vignette, input, assets) for free.
3. **Solo-dev iteration friction** — multi-minute clean builds / ~600 deps: mitigations documented (dynamic_linking dev-only, rust-lld, leaf-crate layout, asset hot reload) but the day-one feel is worse than macroquad; accept as the price of the 0.19 post stack.

*Sources audited live 2026-09-23: crates.io API (fyrox, ggez, macroquad, wgpu, winit, raylib, bevy), GitHub API (Fyrox, godot, raylib, gdext, ggez, macroquad, wgpu repos + latest releases), fyrox.rs 1.0 announcement, Godot 2D lights-and-shadows tutorial, Godot 4.4/4.7 release pages, godot-rust book (export-web caveats), docs.rs (fyrox `dim2`, sprite), ggez CHANGELOG, raylib 6.0 release notes, bevy release process page.*
