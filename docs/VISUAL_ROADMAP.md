# Visual Roadmap — Windowed View (C)

The windowed sprite view (`--view gui`, `src/gui.rs` + `src/sprites.rs`) shares rules, menus, and saves with the terminal view (A). Completed work is listed below; the remaining waves are proposals. References: [current gameplay](gfx/C-live.png), [actor contact sheet](gfx/C-actors.png), and [style comparisons](gfx/index.html).

## Current state

Working: cohesive amber-on-charcoal palette, readable procedural village, clear zones (map / field notes / chronicle / HUD). Known weaknesses, in order of severity, from a vision-model critique of a live frame:

1. **Chronicle text clips** at the window edge while ~60% of the sidebar below sits empty.
2. **Roads dominate** — flat saturated tan bands overpower content.
3. **Four dense text HUD lines** — stats/gear/AI/controls are undifferentiated pipe-runs; MANA floats orphaned with no bar.
4. **Resolved by the actor pass:** tiny glyph badges and the player selection box are replaced by silhouettes and foot markers.
5. **Dead space** in the map's top-right; flimsy 1px panel borders next to heavy roads.

### Completed — actor design

- All 17 NPC archetypes have distinct authored silhouettes: hats/aprons/packs for civilians, spear/shield guards, hooded thieves and bandits, four differently proportioned animals, visible skeleton ribs, and individual boss designs.
- The player has a teal cape and wide gold foot marker. Weapon and armour tiers independently change blade length/material, shoulder plates, helmet, and masterwork trim, using the equipped fields rather than inventory contents.
- Boss phases 2/3 add wounds, awakened eyes, and highlights from `Npc.phase`.
- Two-frame steps and left/right facing follow observed movement. A 120ms flash follows actual HP loss, not floating-text coincidence. Clipped, translucent pixel shadows replace the opaque base.
- `ActorAnimations` lives in the GUI loop, clears on map/seed changes or tick rollback, and never modifies `Game`, saves, or simulation RNG. Teleports do not trigger a walk cycle. Animations use the existing active-game elapsed clock, so pause freezes them.
- Shared UI selects sprite-specific legends; terminal legends/layout and all gameplay rules remain unchanged.

### Completed — higher-resolution view and detail pass

- Window size, fullscreen/restore behavior, and message DPI are tuned for large Windows displays, including a 1440×900 logical startup target. UI density now scales independently of world zoom, while world zoom uses tile sizes 16/24/32/40/48/64.
- Terrain and actors are re-authored at 32 logical units. Terrain gains material cues (cobbles, grass clusters, layered mountain faces, masonry, stairs depth, water bands, road/bank transitions that respect exploration). Actors gain segmented limbs, faces, cloth/armour detail, equipment geometry, and stronger boss phase markers.
- The new `sprites::map_view` viewport snapshot gives future hit detection the same clamp/origin math used by the renderer, while `ui::draw_scene` and `ui::draw_windowed_modal` are now public seams for native windows and focused tests.

Verification combines the permanent crate seams and a temporary native probe before its removal: 174 window/modal/size/scale/zoom combinations plus representative world/terrain rendering; serialized `Game` uneffected; strict Clippy clean with `-D warnings`; zero new simulation fields, no save schema or RNG changes, and no dependencies added.

## Resolution and density

The resolution and the drawing detail are separate concerns. The window aims for a 1440×900 logical startup shape; the map zooms between 16 and 64 pixels without pretending to redraw the art.

| Current | Detail |
|---|---|
| Zoom | `sprites::draw_map` consumes a callers-chosen tile size (16/24/32/40/48/64) and projects through the `sprites::map_view` snapshot. |
| Tile size definition | Terrain paints 32-unit tiles using hashed placement; added detail is independent of window size. |
| Actor detail | Actors use a 32-unit grid with segmented joints and per-archetype construction, not stretched badges. |
| Frame performance | Probe renders 40 overworld frames at 32px in ~0.28 ms median (p95 ~0.79 ms), excluding swap/GPU wait and future overlays. |

Macroquad's `screen_width()`/`screen_height()` are logical units scaled by the OS DPI setting; higher window size adds content space, not art detail.

| Knob | Today | Effect of raising |
|---|---|---|
| Cell grid (`gui.rs::render`) | `cols = (w/11).clamp(100,160)`, `rows = (h/18).clamp(40,70)` | More columns/rows at the same window size mean smaller text and more content, not finer sprite art. |
| Tile size (`sprites.rs::draw_map`) | fixed `20.0` logical pixels | Bigger tiles magnify the existing art and show fewer tiles. Terrain uses a 16-unit paint grid; actors now use a 32-unit grid. |
| Window default (`gui.rs::launch`) | 1280×800, `high_dpi: true` | Larger start; already resizable. Logical drawing size depends on the display's DPI scale. |

Macroquad's `screen_width()`/`screen_height()` divide framebuffer dimensions by the DPI scale. High-DPI rendering uses the display's pixel density; it is not an extra art-detail or supersampling pass.

## Rules every change must respect

1. **Presentation only.** Keep terminal content/layout unchanged. Shared UI may select a view-specific legend (as the actor pass does); native geometry remains in `gui.rs`/`sprites.rs`.
2. **Shared queries, not copies.** Prompts and hints (interaction target, initiative order, quest filter, save-slot presence) must call the same helpers the terminal and `input.rs` use, or the window will teach controls the game ignores.
3. **Animation never touches the sim RNG.** Use presentation clocks or read existing game time. Actor motion reads `elapsed_ms`; scenery can use coordinate hashes or `get_time()`.
4. **No model-schema changes.** `Game` is save-serialized; new fields need `#[serde(default)]` or avoidance (e.g. derive damage-number color from text content instead of a new `FloatingText` field).
5. **No new dependencies, no shaders required.** All effects below are plain alpha-blended draws (macroquad 0.4.16 blends always — `quad_gl.rs:401`); `draw_triangle/poly/ellipse/arc`, `draw_rectangle_ex` (rotation), `Image→Texture2D` CPU authoring all verified against the crate source.

## Level of detail (art fidelity)

"Detail" is three independent knobs with opposite cost models:

| Knob | What | Cost |
| --- | --- | --- |
| Logical paint grid (`sprites.rs::tile_sprite` / `actor`) | Terrain: 16 units; actors: 20 units | Each painted rectangle adds per-frame CPU/geometry work. A larger coordinate grid alone does not multiply work; added shapes do. No quad-count benchmark has been run. |
| Atlas resolution (planned) | Cache authored tiles as textures | A hypothetical 16 types × 8 variants × 32² × 4 bytes is 512 KiB before padding/mipmaps. One quad per tile decouples geometry count from detail, but texture memory, sampling, and cache generation still cost work. |
| Sprite design | Distinct shapes, equipment and phase cues | Implemented with clipped primitives; textures are not required to improve readability. |

**Verdict:** worth it, in this order:

1. **Actor redesign — complete.** See the actor pass above. It uses mirrored geometry, not a texture atlas.
2. **Terrain at 32 units — next candidate after atlas work.** Cobblestones, grass tufts, canopy shading, wall top-faces and water depth bands. Preview at actual tile size: a 32-unit design downscaled into a 20-pixel tile can lose detail. Caching by itself neither adds detail nor guarantees crispness.
3. **Beyond 32 units — defer until the smaller art is evaluated.** More procedural noise is not necessarily more readable. Authored assets remain an option, not a technical prerequisite.

## Wave 1 — readability and presentation

| Improvement | What | Touchpoints |
|---|---|---|
| Resolution/density experiment | ✓ Implemented: window size target, fullscreen restore, independent UI scale and map zoom, small-window auto-fit; preserves modals by reducing scale | `gui.rs::launch`, `gui.rs::render`, `sprites.rs::draw_map` |
| Native floating combat text | Bypass `Canvas.text`'s ink backing box; full-res `draw_text_ex`, slight rotation, fade by ttl, color by content (red damage / green heal / gold XP) | `sprites.rs` effects loop |
| Day/night ambient tint + torch glow | Translucent blue over map at night, warm at dusk (sim already tracks `hour()`/`night()`/`torch_until`); radial glow when torch lit | after `draw_map` call in `gui.rs` |
| Actor life — partly complete | Movement frames, facing, HP-loss flash and translucent pixel shadows shipped. Idle bob remains optional | `sprites.rs::actor`, `ActorAnimations` |
| Screen shake | Decay-based jitter on damage — origin offsets only; **not** `set_camera` (macroquad's camera stack doesn't restore viewports, `camera.rs:303`) | `gui.rs` run loop → `draw_map` param |
| Nameplates + boss bar | World-space names above hostiles/named NPCs (matching sprite colors); boss HP bar with phase pips at viewport top | `sprites.rs` NPC loop |
| Chronicle wrap fix | Remaining: wrap log lines into the reserved sidebar instead of clipping | `sprites.rs`/`gui.rs` sidebar region |

## Wave 2 — the native HUD (M effort, one seam change)

| Improvement | What | Parity note |
|---|---|---|
| **Seam: `draw_scene` returns band rects** | ✓ Public: `ui::draw_scene` and `ui::draw_windowed_modal` return the same Rects the renderers use; terminal ignores the extra fields | One-time contract change; no visual change to A |
| Graphical HUD | Rounded-end gauges with heart/bolt/rune icons, low-HP pulse, `actor()` portrait, AI badge | Same numbers and red-threshold logic as terminal; presentation only |
| Context keybind ribbon | Pill chips adapting to state: near NPC → "E talk", shop → "H haggle", combat → "F/X/C", low HP → "I potion" | Selection derives from live game state, never a parallel list |
| Interaction prompt | "E: talk to Mara" above the player | Requires extracting `interact_target()` beside `social.rs::interact()` so prompt and action can't diverge |
| Combat turn-order strip | Actor chips, current actor highlighted, wind-up countdown | Extract `initiative()` from `ui.rs` combat panel so both views sort identically |
| Minimap corner | ~180px corner chart | Reuse `draw_atlas` verbatim (same exploration gating); M-key modal stays |
| Quest tracker card | Pinned top-left: title, progress bar, one-line description | Use the identical active-quest filter as FIELD NOTES (`ui.rs:570`) |

## Wave 3 — bigger swings (M–L effort)

| Improvement | What | Note |
|---|---|---|
| Camera smoothing | Lerp toward player-centered target; `screen()` already operates in f32 so fractional offsets work | View-only state in `gui.rs` |
| Pre-rendered tile atlas | Later only if CPU cost demands it; authored detail was added procedurally and remains a separate step | Keep animated water and interactive tile state correct; preview the cache against the procedural renderer |
| Particles | Hit sparks, spell motes, outdoor rain — translucent 2–3px rects, <100 quads | Outdoor check by map kind |
| Terrain blending | Dithered 1px edge strips where neighbors differ | Fixes hard tile edges; calms road dominance |
| Rounded translucent modal panels | Skip panel-border cells in `draw_buffer`, substitute one rounded dark rect + subtle border; inner text stays | Presentation only |
| Vignette + title screen + transitions | Pre-rendered radial vignette image; parallax terrain title with ember particles; 150ms modal eases; "MILLBROOK — 08:00" region banners | Transitions never gate input |

## Capability notes (verified in macroquad 0.4.16 source)

- `draw_triangle`, `draw_poly`, `draw_circle`, `draw_ellipse`, `draw_arc`, `draw_hexagon`, `draw_line` — `shapes.rs`.
- `draw_rectangle_ex` with `offset`/`rotation` — `shapes.rs:162`.
- `Image::gen_image_color` / `set_pixel` → `Texture2D::from_image` / `from_rgba8` — CPU texture authoring, no render target needed.
- `draw_texture_ex` flips/rotation/pivot; its color argument tints textures (light/fog becomes a tint).
- Alpha blending is always on for plain draws (`quad_gl.rs:401`) — all fades/tints/rain work without shaders.
- `push/pop_camera_state` does **not** restore viewports (`camera.rs:303-310`) — keep the `Canvas` clip / origin-offset approach; never `set_camera` inside `draw_map`.

---

## Chapter 2 — The Bevy migration (E0/E4 era; supersedes macroquad waves for the flagship path)

**Engine decision (D17, §7.1 of D2_EVOLUTION):** Bevy 2.5D staged with path-A macroquad fallback. **Visual target (D20, §7.3):** C — HD-2D pixel + lights + Darkest Dungeon thematics. The macroquad waves 1–3 above remain valid for view C in maintenance, but the E4 Bevy front-end supersedes them on the flagship path. Contract note: the "no new dependencies / no shaders" rules in this document are **lifted for the Bevy front-end only** — the terminal and macroquad views keep them.

§7.4 deliverables landed as: route-a light map + route-b torch `PointLight2d` pools + day/night ambient (E0 spike); ortho camera with smooth follow + wheel zoom; post stack Bloom 0.08 / Vignette 0.35; DD transfers (orbs HUD, class meter chip, gold-serif nameplates); font upgrade (bundled Cinzel, hot-slot). Sprite upgrades travel separately through the D19/`tools/atlas` pipeline authority (ART_PIPELINE.md): C/C+ hybrid plates, iso default (D37), plate sources climbing to pre-rendered 3D bakes terrain-first (D38), full real-time 3D parked-alive (D39). HUD chrome (D30) follows the same bake lane when it lands.
