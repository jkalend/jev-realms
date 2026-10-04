# Visual target options — evidence board (2026-09-23)

Owner question: *"still very pixely — when does that change, if ever?"* This board puts the four honest candidates in comparable frames over the same sim world (seed 42, Millbrook-area / Burrow), each with its real cost. Pick the target; the schedule follows.

The pixel medium itself is NOT on trial here — sprites vs rendered models is the decision in row C/D. Pixel art with good lighting about it is option B's entire bet; evidence decides whether that bet pays now.

## Candidate frames

### A — Now (what you're calling "very pixely")
[Evidence: e4-foundation.png](gfx/proto/e4-foundation.png)

Square projection, v1 procedural plates at 40px/tile (authored 64px, so they're blur-shrunk), ramp light, minimal post. Verdict on file: **correct sim, expensive effort NOT shown**. Three named faults: square projection (the "1990s" the doc's own bake-off priced), under-rendered plates (two iter weak: mountain facets, bear), no tuned post.

### B — HD-2D over the same stack (the Octopath evidence path)
[Evidence: e4-iso.png](gfx/proto/e4-iso.png) · [e4-iso-burrow.png](gfx/proto/e4-iso-burrow.png) · [earlier projection mock: proj-b-iso.png](gfx/proto/proj-b-iso.png)

Diamond-iso render transform over the SAME square tile data (sim untouched), iso face plates (already baked), actors upright billboards, tuned post (AA, tone curve) + plate-native zoom lanes. Cost estimate: one lane ~days (projection transform + anchor rules); art delta: face re-authoring for the 4 blocky tile types, already done in this atlas generation.

### C — Pre-rendered 3D (the actual D2 recipe: bake models offline -> stills)
[Evidence: r3d-plateview.png](gfx/proto/r3d-plateview.png)

3D blockout rendered offline, stills dropped into the SAME atlas pipeline (contract unchanged: plates are plates). This is literally D2/Titan-Quest's line: models baked with real lights gives sculpted-felt puffy pixels, zero engine swap. Cost: bake tooling for ALL content (terrain faces × 16, 19+ actors × frames × states, gear, boss phases) — this is the art-fidelity ladder we actually climb (D19 v2 with raytraced/simple-3D source plates; no Blender on box -> renderer inside atlas tool, or headless AI-image plates per contract).

### D — Full real-time 3D view over the same sim
[Evidence: r3d-fullview.png](gfx/proto/r3d-fullview.png) · [r3d-square.png](gfx/proto/r3d-square.png)

Sim untouched (pillar-safe), render seam is the view crate in 3D mode: extruded tiles as blocks, actor meshes later. The Grim-Dawn read. Cost: re-scope of every visual element (meshes/materials/animation for players × states) + every future boss's art cost balloons; the E0 spike would re-run for 3D (a week, not a sprint).

## Decision matrix

| | A now | B HD-2D | C pre-rendered 3D | D full 3D |
| --- | --- | --- | --- | --- |
| Eng. lift | 0 | ~days | ~weeks (bake tooling) | ~months (all meshes) |
| Art lift (near) | 0 | 4 faces done | every plate re-shot | every asset rebuilt |
| Art lift (per boss later) | n/a | moderate | moderate | HIGH |
| Style ceiling | low | high | highest tactile | highest |
| Sim risk | none | none | none | repair overhead low |
| Reversibility | n/a | one renderer flag | bake + render both kept | renderer only |
| Maintains D2-class read | no | yes | yes | beyond |

## Recommendation (awaiting lane verdicts)

**Locked so far (2026-09-23, from rendered evidence):** option **D (full real-time 3D view) is out**. The r3d-spike A/B cannot be faked: identical assets, lighting and camera rig produce an editor-like void scene at tactical zoom (`r3d-fullview.png`) but a genuine chiaroscuro composition in the tight bake (`r3d-plateview.png`) — *the premium feel comes from the curated frame and baked light, not from real-time 3D-ness.* Full 3D also nationalizes every boss's future art cost. Pre-rendered 3D plates retain ~all of its value inside the unmodified 2D render contract: a bake is a plate, the pipeline stays D19. (Spike facts: physical-light tuning danger — torch 42k lumens; uniform meshes, no textures → flat albedo reads blockout by necessity.)

The live question is now **B vs C**: HD-2D iso over the current painter stack against pre-rendered 3D plates over the same pipeline. The completion signal for this board is the iso frame landing from the active lane. Everything below stays pending until then.

**Verdict after all four frames landed (2026-09-23):**

- **A (present look)** is out — projection-led "1990s." (frame: `e4-foundation.png`)
- **D (full real-time 3D)** is out — tactical zoom collapses into editor-blockout while the same assets baked tight carry a chiaroscuro. (`r3d-fullview` vs `r3d-plateview`)
- **B (HD-2D iso) WORKS NOW** — (`e4-iso.png`: village as living diorama; `e4-iso-burrow.png`: boss fight as a lit stage on an unknown void, serif nameplates, channel chip) over EVERY plate we already own. It's shipped code with an iso_round_trip test.
- **C (pre-rendered 3D)** is orthogonal: it changes only the PLATE SOURCE. A baked plate slots iso faces the moment it exists, so B's renderer accepts C's waistcoating without a code path change. That's literally D2's own production line (and D19 already names it).

**Verdict after all four frames landed (2026-09-23):**

- **A (present look)** is out — projection-led "1990s." (frame: `e4-foundation.png`)
- **B (HD-2D iso)** — **ADOPTED, shipped default** in `--view bevy` today (flip: crates/view `ViewConfig::default().projection = Proj::Iso`; square stays compiled as the low-DPI fallback path). Evidence: `e4-iso.png`, `e4-iso-burrow.png`.
- **C (pre-rendered 3D plates)** — **THE IMMEDIATE NEXT STEP, ratified by owner (2026-09-24).** The art source ladders from procedural-painted to 3D-baked starting NOW with terrain faces (first flagged: mountain/bear), then bosses, then actor frames. Tooling: headless bake step inside `tools/atlas` (no external assets/Blender — code-rendered source plates are enough to start). Evidence backing the claim: `r3d-plateview.png` beats `r3d-fullview.png` under identical assets, so the bake carries the value while the pipeline (`manifest.json`, iso faces, all render code) stays untouched.
- **D (full real-time 3D view)** — **PARKED, NOT DISCARDED.** Today's evidence says it's weak *right now* at tactical zoom with blockout/cheap meshes (the blockout read `r3d-fullview.png` proves). But two things can reopen it without redoing D17:
  - C's ladder teaches **precisely which content benefits from genuine meshes** — by the time plates cover the expensive 80%, the incremental cost of a limited 3D mode for **set-piece cameras** (vista reveals, boss-intro sweeps, party marching order views) drops sharply;
  - a re:3D spike with textured model-based terrain (not free blockout) re-answers the tactical-zoom question honestly.
  Keep `spikes/r3d` buildable in the tree as the restart point; revisit after E7's content lands. Decision register points: **D17 unchanged (Bevy stack), D38 added (art-source ladder), D39 added (D reservation — show `"3D big screens"` as a candidate post-E7, not a bounty on full-world 3D)**.

**Recommendation (executed):** Coherence discipline — **B rides today, C re-skins the art under it, D stays in the shed waiting for textures.** Current schedule: A → shipped E4 (2026-09-23); **B default flip: NOW**; **C first batch (terrain 3D-bake: Mountain + DeepForest first (the two blobbiest), then Forest/Wall, then bosses, then actor frames): NOW → next milestone**; D revisit point after E7 content lands.
