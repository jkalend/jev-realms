# Bevy Engine Research — D2-Look Migration Evaluation

> Research-only. Verified **2026-09-23**. Facts marked **[STALE]** are from older releases; **[INFERENCE]** are extrapolations, not measured for this project.

## 0. Headlines

| Fact | Value (as of 2026-09-23) | Source |
|---|---|---|
| Current stable | **0.19.1** (2026-08-13); 0.19.0 2026-06-18 | <https://github.com/bevyengine/bevy/releases> |
| Next release | **0.20.0-rc.1** (2026-09-15) — final imminent | <https://github.com/bevyengine/bevy/releases> |
| Cadence | 3–4 months per breaking `0.x`, no fixed dates; weekly RCs in final weeks | <https://bevy.org/learn/contribute/project-information/release-process/> |
| Renderer | wgpu: DX12 (Windows), Vulkan (Windows/Linux), Metal (macOS), GL fallback; WebGPU/WebGL2 on WASM | <https://docs.rs/wgpu/latest/wgpu/struct.Backends.html> |
| Recent history | 0.17 (2025-09-30): Solari raytracing (exp.), hotpatching (exp.), BSN prequel, tilemap chunks; 0.18 (2026-01-13): feature collections `2d`/`3d`/`ui`; 0.19 (2026-06-18/19): **BSN scene notation, parley text rewrite, vignette/lens distortion, contact shadows, resources-as-components, render-graph→systems** | <https://bevy.org/news/bevy-0-17/>, <https://bevy.org/news/bevy-0-19/> |

---

## 1. Engine fundamentals

### Versions & cadence
- ~1 breaking release per 3–4 months; every `0.x.0` is a migration event; per-release migration guides shipped officially. <https://bevy.org/learn/migration-guides/introduction/>
- 0.16→0.17→0.18→0.19 were each hundreds of changes (see §7).

### wgpu / desktop support
| Platform | Native backend | Notes |
|---|---|---|
| Windows 10+ (primary target) | **DX12** (wgpu default) or Vulkan | Switch via `WGPU_BACKEND=dx12|vulkan|metal|gl` env var |
| Linux | Vulkan | |
| macOS | Metal | Vulkan via MoltenVK is non-default |
Sources: <https://docs.rs/wgpu/latest/wgpu/struct.Backends.html>, <https://bevy.org/learn/errors/b0006/>, <https://bevy.org/news/bevy-0-16/> (GPU-driven features table favors Vulkan).

### ECS model (one paragraph)
Bevy's ECS stores components in **archetype-directed tables**; entities are groupings of components with no behavior. **Systems** are plain Rust functions whose parameter types (`Query<&mut T>`, `Res<R>`, `Commands`, filters) declare all data access; the multithreaded schedule executor silently builds a conflict graph from those declarations and runs non-conflicting systems in parallel on a task pool — two systems conflict if both touch one component and either takes it mutably. There is no "update method" on objects; all behavior lives in systems, all data in components, and structure (plugins, schedules, system sets) replaces inheritance. (<https://docs.rs/bevy_ecs/latest/bevy_ecs/>, <https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/struct.MultiThreadedExecutor.html>, <https://github.com/bevyengine/bevy/blob/main/examples/ecs/ecs_guide.rs>)

### Typical app structure
```rust
App::new()
    .add_plugins(DefaultPlugins)          // window, render, input, assets, audio...
    .add_systems(Startup, setup)         // one-time schedule
    .add_systems(Update, (sys_a, sys_b)) // per-frame schedule
    .run();
```
- `App` owns the ECS `World`; **Plugins** are composable crates/modules that register systems/resources/assets; **Schedules** (`Startup`, `Update`, `FixedUpdate`, custom) are ordered batches of systems. <https://bevy.org/learn/quick-start/getting-started/plugins/>
- Custom schedule label for an externally-paced sim is first-class: `Schedule::new(SimSchedule).set_executor(...)`. <https://docs.rs/bevy/latest/bevy/ecs/schedule/>

### Fixed timestep → external 4 Hz sim
- Built-in: `Time::<Fixed>::from_hz(4.0)` + systems in `FixedUpdate`; default is 64 Hz so must be overridden. Fixed loops run 0..N× per rendered frame; render-side interpolation goes in `RunFixedMainLoop` after `AfterFixedMainLoop`. <https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html>, <https://bevy.org/news/bevy-0-12/>, <https://docs.rs/bevy/latest/bevy/prelude/enum.RunFixedMainLoopSystems.html>
- Fit for Laya Realms: the existing pure-Rust sim can either (a) be called each `FixedUpdate` at 4 Hz from a thin bridge crate, or (b) keep its own scheduler and Bevy just reads snapshots — the schedule design doesn't force the sim into ECS. **[INFERENCE: both patterns are commonplace; no architectural impedance].**

---

## 2. D2-look deliverables

| # | Deliverable | Status / recommended option | Sources |
|---|---|---|---|
| a | **2D dynamic lighting** | **No first-party 2D lighting in Bevy itself** (explicit gap acknowledged). **`bevy_light_2d` 0.10** (targets Bevy 0.19): component-driven `PointLight2d` {intensity, radius, color}, camera-scoped `AmbientLight2d`, `LightOccluder2d` + dynamic shadows, WebGL2+WebGPU. Attenuation = inverse-square-style falloff with a **hard cutoff at `radius`** (docs link lisyarus's attenuation article) — D2-style torches achievable. Alternatives: `bevy_lit` (Firefly), or custom lighting pass via `Material2d` | <https://docs.rs/bevy_light_2d/latest/bevy_light_2d/>, <https://docs.rs/bevy_light_2d/latest/bevy_light_2d/light/struct.PointLight2d.html> |
| b | **Normal-mapped 2D sprites** | **Not built into the Sprite renderer**; `bevy_light_2d` feature list has no normal-map entry. Paths: (1) custom `Material2d` on `Mesh2d` quads sampling an albedo+normal pair in WGSL (Material2d has existed since 0.8); (2) **3D path**: use `StandardMaterial` (glTF normal maps supported natively) on low-3D geometry / pre-lit art — this is what actually unlocks D2-style light-rolled sprites. NB: 0.20 draft migrates `Sprite` toward `Mesh2d`/`SpriteMaterial` — the Material2d path is the future-blessed one | <https://bevy.org/news/bevy-0-8/>, <https://bevy.org/learn/migration-guides/0-19-to-0-20/> (draft), <https://docs.rs/bevy/latest/bevy/gltf/> |
| c | **Post-processing** | **Built-in at 0.19**: `Bloom` (HDR, auto-enables HDR through required components, likes `Tonemapping::TonyMcMapface`); per-camera `Tonemapping`; **`Vignette` and lens distortion added in 0.19** (`bevy::post_process`). Custom fullscreen WGSL passes via official `custom_post_processing` example. Known pitfall: unordered custom pass vs tonemapping on HDR cameras → corrupt output (open issue) | <https://docs.rs/bevy/latest/bevy/post_process/bloom/struct.Bloom.html>, <https://bevy.org/examples/3d-rendering/post-processing/>, <https://bevy.org/news/bevy-0-19/>, <https://github.com/bevyengine/bevy/issues/24839> |
| d | **Particles** | **No first-party particle system**; community standard is **`bevy_hanabi` 0.19** (GPU-simulated: spawning forces, billboards, mesh particles, ribbons/trails, works with 2D and 3D cameras, HDR/bloom-friendly). Caveat: GPU state not ECS-inspectable per frame. CPU alternative: `bevy_enoki` (instanced; WebGL/mobile-friendly). Plain ECs sprites fine for dust/small counts | <https://github.com/djeedai/bevy_hanabi>, <https://docs.rs/bevy_hanabi/latest/bevy_hanabi/>, <https://github.com/bevyengine/bevy/discussions/18680> |
| e | **Isometric 2:1 ground (2D approach)** | Pure 2D: `Camera2d` is already orthographic — fake the iso projection in tile→world transform (`screen_x = (x−y)·s`, `screen_y = (x+y)·s/2`) and **Y-sort via sprite Z index** (sprites are Z-sorted). Ecosystem: `bevy_ecs_tilemap` 0.19 ships `TilemapType::Isometric(IsoCoordSystem::Diamond)` and `::Staggered` with `iso_diamond`/`iso_staggered` examples (staggered uses 64×32 = 2:1); Bevy-native `TilemapChunk` (0.17+) is square-grid only today. 2.5D alternative: `Camera3d` + `OrthographicProjection` pitched at atan(1/√2)×√2 and use real depth instead of Y-sort hacks | <https://docs.rs/bevy_ecs_tilemap/latest/source/src/map.rs>, <https://docs.rs/bevy_ecs_tilemap/latest/source/examples/iso_staggered.rs>, <https://bevy.org/examples/2d-rendering/tilemap-chunk/> |
| f | **True 3D w/ fixed ortho camera (upgrade path)** | Out of box: **glTF 2.0 loader** (scenes, meshes, animations, cameras; KHR clearcoat/transmission/volume/ior/specular/anisotropy/unlit; NOT variants, iridescence, sheen, quantization, meshopt), **`StandardMaterial` PBR** (base color/metallic/roughness/normal/occlusion/emissive), **point/spot/directional lights with shadows** incl. cascaded shadow maps (`CascadeShadowConfigBuilder`, tunable `DirectionalLightShadowMap`), **light textures** (cookies, 0.17), **contact shadows** (0.19), optional raytraced lighting via Bevy Solari (experimental, 0.17+), DLSS (0.17). Camera3d + orthographic projection = "locked Diablo camera" trivially | <https://docs.rs/bevy/latest/bevy/gltf/>, <https://docs.rs/bevy/latest/bevy/pbr/>, <https://docs.rs/bevy/latest/bevy/light/struct.CascadeShadowConfig.html>, <https://bevy.org/news/bevy-0-17/> |

---

## 3. Tile + actor rendering at roguelike scale

Map = 200×160 overworld ≈ 32k tiles, fog-of-war per tile, per-tile tint/light, sim owns truth.

| Option | Mechanics | Fit when sim owns truth | Sources |
|---|---|---|---|
| **Bevy-native `TilemapChunk`** (0.17+) | One draw call per chunk; part of a growing built-in tilemap effort; square grid | **Best first try**: truth stays in sim's arrays; chunk data is just a View inserted per frame on change; per-tile color channels cover FoW tint. No iso support yet | <https://bevy.org/news/bevy-0-17/>, <https://bevy.org/examples/2d-rendering/tilemap-chunk/> |
| **`bevy_ecs_tilemap` 0.19** | One entity **per tile** + chunk meshes sent to GPU; sparse maps, layers, GPU-powered tile animation, iso/hex | Duplicates truth (32k entities the sim already models); every tile change flips ECS change detection. Still the only ready-made **iso** path. Per-tile `TileColor` covers FoW/tint | <https://github.com/StarArawn/bevy_ecs_tilemap>, <https://docs.rs/bevy_ecs_tilemap/latest/bevy_ecs_tilemap/> |
| **Custom chunked renderer** | Dense `Vec<TileId>` from sim → rebuild dirty chunks only → one mesh/instanced buffer per chunk → 1 draw call/chunk visible | **Most tailorable**: per-tile tint/light/FoW ride the vertex/instance buffer; required if tiles must be normal-mapped or lit by custom shaders. Cost: own culling/streaming/animation | <https://github.com/bevyengine/bevy/discussions/2265> |
| Individual `Sprite` entities for actors | Auto-**batched and instanced**; per-instance GPU data cut 144→80 bytes (0.12); same texture/atlas is the batching prerequisite | Actors/units/items (hundreds-to-low-thousands on screen), not ground tiles | <https://bevy.org/news/bevy-0-12/> |

- **Sprite count comfort zone (desktop)**: historical `bevymark` demos drew ~100k sprites on one machine (0.6 era — **[STALE]**, directionally still valid with instancing since 0.12); practical comfort band ~10k–50k moving sprites on a mid-range+ desktop **[INFERENCE — measure with `cargo run --release --example many_sprites`]**; main failure mode is CPU-side per-frame ECS work on N entities and transparent overdraw, not raw sprite count. <https://bevy.org/news/bevy-0-6/>, <https://github.com/bevyengine/bevy/blob/main/examples/README.md>
- **Sprite sheets / atlases**: built-in — `Sprite::from_atlas_image(texture, TextureAtlas { layout: Handle<TextureAtlasLayout>, index })`; official `sprite_sheet.rs` example animates by index-swap. <https://docs.rs/bevy/latest/bevy/image/struct.TextureAtlas.html>, <https://github.com/bevyengine/bevy/blob/main/examples/2d/sprite_sheet.rs>

**Recommendation**: ground = native `TilemapChunk` or custom chunk meshes (sim stays truth); actors/items = instanced `Sprite`s sharing atlases; particles via hanabi; reach for `bevy_ecs_tilemap` only if iso-before-3D needed in 2D.

---

## 4. UI (modal-heavy RPG: inventory/trade/dialog/atlas/skill tree, keyboard-driven)

| Option | Strengths | Weaknesses for this game | Sources |
|---|---|---|---|
| **`bevy_ui`** (built-in) | Retained ECS nodes, flexbox/grid layout, gradients (0.17), input focus (0.16+), **0.19: parley text engine, rich font selection (families/weights/variable axes), responsive font units (`Px/Vw/Vh/Rem`), `EditableText` with IME/clipboard/bidi**, experimental headless widgets + Feathers (0.17→0.19) | Widget set still experimental; keyboard-first modal flows (nested focus stack, list selection) must be hand-built on top of the primitives; node-graph flexibility vs ratatui's grid is a mindset shift | <https://bevy.org/news/bevy-0-19/>, <https://bevy.org/news/bevy-0-17/>, <https://bevy.org/learn/migration-guides/0-18-to-0-19/> |
| **`bevy_egui`** | Immediate-mode: inventory/trade/dialog screens are code-first and trivial to refresh every frame; tables/grids/widgets today; good dev tooling | Aesthetic skews "tool UI"; per-frame rebuild cost; controlled styling discipline needed to look like a game; input can conflict — use `absorb_input` pattern | <https://github.com/vladbat00/bevy_egui>, <https://docs.rs/bevy_egui/latest/bevy_egui/> |
| **Custom (macros on top of bevy_sprite + text)** | Full control over D2 tab look; trivially keyboard-driven | Everything is "from scratch": focus, alignment, hit-testing, scrolling | — |

- **Text quality**: adequate+ since 0.19 — parley shaping replaces cosmic-text; letter-spacing (`LetterSpacing`), per-span colors/backgrounds (0.17: `TextBackgroundColor`, `Text2dShadow`), emoji/system font families. Pre-0.19 text rendering was a common complaint — don't evaluate on older versions. **[STALE-risk: any review older than 2026-06 prejudices text badly.]** <https://bevy.org/news/bevy-0-19/>
- **Verbatim layout port from ratatui**: impossible — nothing gives you a text-cell grid; but low-pain: represent layouts as isometric data (rows × cols) and render with monospace `bevy_ui` grid nodes, or egui monospace labels, so existing layout *data* survives the move. **[INFERENCE]**
- **Recommendation**: `bevy_ui` for the shipping look (post-0.19 text makes it viable), `bevy_egui` for debug/dev tooling only.

---

## 5. Asset pipelines for D2-style art

| Need | Solution | Sources |
|---|---|---|
| Sprite-sheet import | Built-in `TextureAtlas`/`TextureAtlasLayout` (region grid or custom rects); official example | <https://docs.rs/bevy/latest/bevy/image/struct.TextureAtlas.html> |
| Aseprite | `bevy_aseprite` (reads `.aseprite` w/ tags + frame timing) is **frozen at Bevy 0.12 — [STALE/DEAD]**; use **`bevy_mod_aseprite`** or **`bevy_aseprite_ultra`** (both maintained, expose `Sprite`+atlas from tags) | <https://docs.rs/bevy_aseprite/latest/bevy_aseprite/>, <https://docs.rs/bevy_mod_aseprite/latest/bevy_mod_aseprite/>, <https://github.com/Lommix/bevy_aseprite_ultra> |
| Normal maps from existing 2D art | Tools, not Bevy: **Laigter** (free, itch.io), Sprite Illuminator (commercial), or Krita filters; or bake from a Blender mesh. Quality is headless-automatable. Verify feeding them into a custom `Material2d`, or via 3D `StandardMaterial` | <https://azagaya.itch.io/laigter> |
| **Pre-rendered 3D→2D (D2's own pipeline)** | Model in Blender → lock orthographic iso camera → render N-direction × M-frame sprite sheets (bake albedo + normal + occlusion passes) → import as `TextureAtlasLayout`. Bevy consumes the outputs as plain atlases — nothing engine-specific required. The naturalist upgrade: render again later from the same meshes as real 3D for §2(f) | General Blender practice; consumption side: <https://docs.rs/bevy/latest/bevy/image/struct.TextureAtlas.html> |
| AI-assisted texture generation | Feasible note: image models (SD/Flux-style) for base tile/sprite textures + img2img height→normal converters; process is offline, so Bevy-agnostic. Watch licensing/consistency; treat as concept-art accelerator, not final quality | — **[INFERENCE]** |

---

## 6. "Go much further" inventory

| Unlock | What it gives Laya Realms | Caveat | Sources |
|---|---|---|---|
| Gamepad input | `bevy_gilrs` (Gamepad API) ships in DefaultPlugins | No built-in rebinding UI | <https://github.com/bevyengine/bevy/blob/main/crates/bevy_gilrs> |
| Audio | `bevy_audio` built-in (wav/ogg/mp3/flac); **`bevy_kira_audio`** (kira backend: channels, tweens, web support) | Built-in is minimal — plan kira or wait on upstreaming | <https://github.com/NiklasEi/bevy_kira_audio>, <https://docs.rs/bevy_kira_audio/latest/bevy_kira_audio/> |
| WASM / web build | Same code targets `wasm32-unknown-unknown`; play-in-browser demo build | Separate binaries for WebGPU vs WebGL2 (issue open since 2024); bundle size is a real cost; `dynamic_linking` not usable on wasm; secondary target only | <https://bevy-cheatbook.github.io/platforms/wasm.html>, <https://github.com/bevyengine/bevy/issues/11505> |
| Steam distribution | Plain `YourGame.exe + assets/` folder; `cargo build --release`, **disable `dynamic_linking`**, test from clean `dist/` (working-directory asset gotcha is the #1 ship bug); SteamPipe depots/VDF is engine-agnostic | Save/config must go to user-writable dirs, not install folder | <https://bevy.org/learn/quick-start/getting-started/setup/>, <https://partner.steamgames.com/doc/sdk/uploading> |
| Modding | `bevy_mod_scripting` 0.21 (Bevy 0.19): Lua 5.4 + Rhai, hot reload, generated bindings/declarations | Rust-native dynamic plugins were **removed in 0.15 as unsound** (no stable ABI); must design a constrained mod API, not raw `World` access | <https://docs.rs/crate/bevy_mod_scripting/latest>, <https://bevy.org/news/bevy-0-14/>, <https://makspll.github.io/bevy_mod_scripting/> |
| Threaded ECS for 250-NPC sim | Non-conflicting systems auto-parallelize onto the task pool (12 cores usable); `par_iter_mut` for in-system fan-out; no re-architecture needed if sim stays systems-shaped or lives behind a bridge | Systems touching shared mutable state get serialized by the scheduler; sim crate must play nice over FFI-free Rust boundaries (it's pure Rust already — fine) | <https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/struct.MultiThreadedExecutor.html>, <https://docs.rs/bevy_ecs/latest/bevy_ecs/system/struct.Query.html> |

---

## 7. Risks / costs

| Risk | Evidence | Mitigation |
|---|---|---|
| **Breaking-change cadence + migration burden** | Every `0.x` carries a migration guide; 0.19 alone: `Resource`→subtrait of `Component`, render-graph→render-world systems, cosmic-text→parley, scene-crate rename, bloom-linear color change. 0.20 draft already: BSN syntax, `Tonemapping` moves, `bevy_shape`/`bevy_curve` split, **Sprite→Mesh2d/SpriteMaterial migration**, system unification. Expect days per upgrade, every 3–4 months | Pin `bevy = "0.19"`, commit `Cargo.lock`; time-box upgrade sprints; delay until ecosystem crates (mod_scripting, hanabi, light_2d, egui) release compatible versions — they lag by days-to-weeks | <https://bevy.org/learn/migration-guides/0-18-to-0-19/>, <https://bevy.org/learn/migration-guides/0-19-to-0-20/> (draft) |
| **Compile times on 12-core desktop** | Bevy pulls ~600 transitive deps; full clean rebuild multi-minute. Official mitigations: `bevy/dynamic_linking` for dev only; **Windows: `rust-lld.exe`** via `.cargo/config.toml`; `[profile.dev.package."*"] opt-level=2`; `debug=1`; trim features | Use official `config_fast_builds.toml` recipe on day 1; never ship with `dynamic_linking`; iterate on game code in the leaf crate only | <https://github.com/bevyengine/bevy-website/blob/main/content/learn/book/development-practices/fast-compiles.md>, <https://github.com/bevyengine/bevy/blob/main/.cargo%2Fconfig_fast_builds.toml> |
| **Debug iteration speed (vs macroquad)** | macroquad: recompile binary, hot asset. Bevy adds: **Rust hot patching** (0.17+, dioxus `dx`, experimental: binary crates only, no wasm, system-param changes not supported), asset file-watcher hot reload built-in | Hot patching is lab-grade; realistically budget "edit → cargo run --features dev (~10-60 s link improved by dynamic_linking) → playtest" | <https://bevy.org/news/bevy-0-17/> |
| **Binary / compile trim** | `default-features = false` + feature collections (`2d`, `3d`, `ui`… introduced 0.18) + drop audio/pbr/wasmgs | Skip `bevy_pbr`/`3d` features while staying 2D; re-add at upgrade-path time | <https://bevy.org/learn/migration-guides/0-17-to-0-18/>, <https://bevy.org/learn/quick-start/getting-started/setup/> |
| **Learning-curve hotspots (solo dev from macroquad)** | ECS data-orientation + borrow rules in system params (query conflicts are compile-time); asset system is async (`AssetServer` handles, no immediate draw); no shipped scene/editor — UI and levels are code (BSN macro improving); WGSL + render-schedule internals needed for anything custom (and render internals churned hard in 0.19); event/message split (0.17) is another concept | Budget 1–2 serious weeks to vertical-slice the scaffold before committing; cheat-book as reference | <https://bevy-cheatbook.github.io/>, <https://bevy.org/news/bevy-0-17/> |

### Top-3 risks (assessment)
1. **Migration treadmill + ecosystem lag** — every release is breaking; every plugin (hanabi, light_2d, bevy_egui, mod_scripting, ui helpers) must catch up; skipping versions compounds the diff.
2. **No first-party 2D lighting/normal-mapped sprite path** — the centerpiece of the D2 look is third-party (`bevy_light_2d`) plus custom `Material2d` work; the "real" answer is likely the 3D camera path (§2f), which is a bigger commitment than a "2D Bevy port".
3. **Iteration friction vs macroquad** — linking (even with dynamic_linking) + adult-API surface (UI widgets experimental) will slow a solo dev until workflow discipline (leaf-crate separation, asset hot reload, rust-lld) is established.
