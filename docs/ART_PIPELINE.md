# Art pipeline (D19) — plate baking for E4 assets

Status: historical D19/D38 production decisions. Current Bevy runtime atlas:
[`ART_3D_PIPELINE.md`](ART_3D_PIPELINE.md). Terrain remains 64px; the 26
improved Blender NPCs use 96px cells. `realm-atlas actors` and
`atlas bake --actors` are procedural reference/rollback tools, not the live
NPC-sheet regeneration path.

## Plate source: C/C+ hybrid only

All sprite content is AI-authored (D19): procedural-atlas code and/or
image-model plates, batch-baked offline into sheets. Plates exist for the
C/C+ hybrid read **only** — arms A (pure grim painterly) and B (Rogue-Legacy
chunky) are bake-off evidence, not sources:

- **Environment (arm A/C lineage):** grim painterly, baked. Code painters
  derive from gfxlab's `artstyle_deblocked`: the `paint_terrain` grim
  transform (~40% desaturate, ~15% deepen) over the game's existing
  `sprites::terrain_color` palette, deterministic hashed grain, organic
  ridge/canopy silhouettes, rubble-step walls, road/rubble edge interest.
- **Actors (arm B lineage):** chunky, ink-outlined, ~+22% brightness lift
  against the environment. Bodies are a **verbatim port** of
  `sprites::actor()` (the 32-unit vocabulary, scaled ×2), so archetype
  reads match the TUI sprites exactly; the C+ finish is gfxlab's
  `draw_actor_styled` chunky pass — near-black silhouette offset ±3px on
  both axes, then the figure with accents lifted ×1.22. Quadrupeds
  (Rat/Matriarch/Wolf/Bear) take the lift on their body tones too. Ground
  shadow and the allegiance mark sit *outside* the silhouette so outlines
  hug the figure, not the feet.

## Pixel doctrine

- **Base tile: 64px.** Terrain, props and chrome use 64×64 cells. The actor
  sheet declares its own `cell_size: 96` for improved Blender NPCs; the
  manifest's `tile_size` remains the terrain scale.
- **No lighting in plates.** Plates are unlit albedo in the game palette.
  The E0 light stack (route-a sim light map as base layer, route-b
  `bevy_light_2d` torch pools, bloom 0.08 / vignette 0.35 / exposure ×2.2)
  applies at runtime — plates must read grim-dark on their own.
- **Determinism.** Every grain dot, tuft and step is coordinate-hashed
  (integer avalanche mix) from the tile's sheet position — no RNG, no GPU.
  Re-bakes are byte-stable for unchanged code.

## atlas/manifest.v1 (the exchange contract)

`assets/atlas/manifest.json` is the single hand-off format:

```json
{"tile_size": 64,
 "sheets": [
   {"name": "terrain-square", "file": "terrain/square.png",
    "mapping": {"<TileDebugName>": [col, row], ...}},
   {"name": "terrain-iso", "file": "terrain/iso.png",
    "faces": ["top", "left", "right"],
    "mapping": {"<TileDebugName>": [col, face_row], ...}},
   {"name": "actors", "file": "actors.png", "mapping": {}, "states": {}},
   {"name": "props", "file": "props.png", "mapping": {}}]}
```

- Keys are `format!("{:?}")` of `laya_realms::model::{Tile, Archetype}` —
  stable across sim changes with **no new sim API**.
- **Missing keys are never errors:** the renderer falls back to palette
  colour quads (the E0 read, which passed its look test on quads alone).
- Iso: for a mapping entry `[col, r]` and `faces: [top, left, right]`, the
  tile's faces occupy rows `r`, `r+1`, `r+2` in faces order (bake ships
  `r = 0` for every tile).
- **Iso face-cell anchor:** cell 64×64, top-face diamond centre `A = (32, 20)`
  (hw 32, hh 16, 2:1). Flat tiles paint the ground diamond; their left/right
  cells are fully transparent. Blocks (`Forest`, `DeepForest`, `Mountain`,
  `Wall`) paint silhouette art on the top cell and shaded skirts on the side
  cells — composite order **left, right, top**. Skirt heights: wall 20px,
  forest/mountain 28px; block shading left ≈0.62, right ≈0.82, top ≈1.18
  (the gfxlab ratios). Tall art may rise above `A` into the cell's upper half.
- **Edge dither (de-block), square + flat-iso plates:** the bake-off's
  dither speckles the *neighbour's* colour along shared edges; a standalone
  tile cannot know its neighbours, so plates bake the flecks
  **semi-transparent** (dark 1–2px band alpha ≈140, sparse lifted flecks
  just inside ≈100). Composited over any neighbour at runtime they read as
  a blend. True dual-tile blended edges would need neighbour-variant plates
  — a future batch if E4 screenshot evidence asks for them.

### Actors sheet (batch 2/3)

`actors.png` (512×320, 8 cols × 5 rows, one idle frame per key):

- The **27 `Archetype` Debug names** in enum declaration order (BroodHole's
  cell is reserved but EMPTY — the maw ships as a prop decal), canonical
  portraits from the `actor()` port — boss silhouettes included (Chief
  horned + broad pauldrons, Matriarch bone-spines + mane, Lich crowned
  robe + sea-lit staff, Adjudicator gold colossus + gavel arm, plus the
  E3 arrivals (Tidemother gown, Cragmother ridge plates) and the E5/E7
  batch-3 plates: GnawThane — dire-rat king with shard crown and red eyes;
  Tollmaster — broad pauldron warlord with coin sash and toll-hook;
  Mirelight + FalseGlow — veiled wisps with hollow glow-faces, ink sockets
  and tapering tails (FalseGlow is the dimmer mimic, ink sockets removed);
  PaleStag — bespoke ghost stag (slim barrel, long legs, long forking
  antlers); Alchemist — stained apron with the three-vial belt;
  OathlessCurate — violet mitre over the page he never stops folding.
  Canonical accents live in `actors.rs::accent`.
- **States convention (manifest.v1 amendment, batch 3):** a sheet may
  carry `states: { "<Group>": { "<variant>": [col, row] } }`. Consumers
  flatten groups to composite keys `"<Group>.<variant>"` — tolerant by
  construction (missing → quad fallback; the actors loader passes any
  group through generically). `states.Player.<ClassDebugName>` was the
  precedent; `states.PaleStag.at_bay` ([4,3], head-lifted alert pose) is
  the proof-case for per-archetype pose/animation states. Enum variants
  stay in `mapping`; states are for pose/gear/animation rows without enum
  keys, keyed by their owning group so later batches (walk/attack frames,
  gear tiers) can add rows without touching the schema.
- Player class hero frames are the `states.Player` precedent (cols 1-6,
  row 4 — the last row). Same base figure (mid gear: weapon 1, armour 1,
  no relic); the hue-swap touches cape + feet mark in the class meter hue
  (amber/ivory/rust/olive/cyan/violet). Gear/phase/walk frames are later
  batches keyed likewise under `states.Player`.

### Props sheet (batch 2/3)

`props.png` (320×64, 5×1): `TorchBrazier`, `Potion`, `RuneStone`,
`RelicPedestal` (D38-baked), `BroodHole` — chunky finish shared with
actors, palette pulled from the environment plates. Keys are plate names;
when a `Prop` enum lands in the sim they become its Debug names, same
rule. **Terrain-decal archetypes** — figures that are ground, not bodies —
ship here: a consumer looking up such a key via `actor_ref` gets the
props-sheet cell (additive fallback, baked into crates/view atlas.rs).



The baker lives in `tools/atlas` (bin `atlas`), macroquad-free pure-RGBA
raster via the `image` crate; it depends on `laya_realms` by path **only**
for the `Tile`/`Archetype` key enums.

```sh
cargo run -p realm-atlas -- terrain   # bake sheets + manifest into assets/atlas/
cargo run -p realm-atlas -- preview   # docs/gfx/proto/atlas-terrain-contact.png
```

`terrain` re-parses the written manifest, re-decodes every named PNG and
checks dims + full key coverage as a self-check — trust the
`ATLAS VERIFY OK` line. Iterate by editing the painters and re-baking;
review the contact sheet (square | iso-block on a neutral backdrop, Debug
names printed by the CLI). PNGs are build artifacts: **never hand-edit
them** — changes that can't come from the painter code don't survive.

## Bake source: procedural 3D bakes (D38), batch 1

Mountain and DeepForest plates (square **and** iso) now come from a
headless 3D bake in `tools/atlas/src/bake3d.rs`, not the painters: analytic
heightfields (Mountain = ridged-fbm conical crag + talus + scree boulder;
DeepForest = seven metaball canopy domes over moss ground) rendered with a
real lighting rig — sun NW key, warm SE fill, horizon AO, marched
self-shadows — splatted orthographic at 3× (196→64 box) for AA. Albedos
are the same grim palette entries as the painterly plates, so baked plates
slot the grim transform discipline unchanged. Iso bakes render the whole
block in one oblique pass (top surface + south walls, strata/mottle on
walls) and split it into the manifest's three face rows per the anchor
convention — compositing order and coordinates are untouched, **manifest
keys and cell positions are identical**, so the renderer never knows the
source changed. `atlas bake [--tiles Mountain,DeepForest]` swaps plates
in place; the painter code paths are intact for rollback
(`atlas terrain` rewrites everything painted). Review artifact:
`docs/gfx/proto/atlas-baked-terrain-contact.png` (painted | baked, square
| iso). Later batches per D38 may extend `bake` to actor/boss bases; the
painter fallback stays.

**Batch v3 (de-block wave, owner review — "EXTREMELY blocky; follow the
D2 route, walls HAVE to make a wall")**: the bake language changed, not
the contract. (1) Cube read killed from two sides: a MACRO blotch field
keyed to one fixed phase per tile family modulates every albedo (like
tiles share large blotches at shared borders — the honest plate-only
stand-in for per-coordinate variants, which remain a future batch), and
every block skirt now ERODES at the foot (noise dropout grows with depth)
so feet feather into rubble instead of ending at a corner line. (2) Wall
re-invented: no more keep cube — a tall thin masonry FACE along the iso
X-diagonal (u+v=72 gives the whole run a FLAT screen top edge), 22px
face, parapet merlon steps on the top edge line ONLY (no full top plane),
pilaster bumps every 8px, coursed joints + arrow-slit marks at mid-face,
robbing dropout at the base, fallen stones scattered in front of it.
(3) Clutter decals per family on every baked block (Mountain/Rock/Wall:
rubble stones; Forest/DeepForest: moss + roots on skirts, tufts on tops).
(4) Actors v3: eye sockets sharpen to near-ink 0.62r dots, weapons gain
rest-cant (guard spear ~45deg, visible in the world cast), and every
baked actor anchors a soft ellipse contact shadow (post-chunky,
alpha-empty pixels only — feet and shadows no longer float). (5) QA
contract upgraded: the judgment sheet is now a WHOLE fake 6x6 iso map
with 8 actors standing on it
(`docs/gfx/proto/atlas-terrain-world-contact.png`), not per-tile rows.
**Answered after v3.2 — sheet-drop, not double height.** The flagged 40px+ face
now ships without double-height assets, because the 64px ceiling was an artefact
of where the face was *painted*, not of the cell: the bake paints the face art
24px lower in its cell (`bake3d::WALL_FACE_DROP`) and the view lifts the face
sprite back by the matching `Proj::ISO_WALL_FACE_LIFT`, so a 40px face fits
exactly as well as a 20px one. The view also raises the cap by the full face
height (`Proj::ISO_WALL_LIFT`), and that combination is what turns the rampart
line into real silhouette. See ART_3D_PIPELINE.md § Walls for the invariant.

The `Wall.tall` 2-cell curtain is retained as offline comparison art; gameplay
uses the one-cell cap and dropped faces. The environment quality cutover uses
eight authored `states.Wall.<family>` / `states.Floor.<family>[2]` materials,
not a tint of shared grey stone. Static keys select the current `sim::Zone`,
including cave, woodland, arena and dock identities as well as the dressed
town/crypt/sanctum/Underkeep families. See ART_3D_PIPELINE.md § Walls for the
two actual-atlas review passes and regeneration commands.

**Batch v4 (states through the states door):** the flagged cap from v3
answered with the manifest's own convention — terrain-iso gains states
rows, additive-only (plain mapping keys ALWAYS stay; consumers that
ignore states see no change). Two features ship: (1) **Wall.tall**
(anchor `[9,3]`, body `[9,4]`): a 2-cell curtain rendering a 40px masonry
face on the iso diagonal — one 128px bake buffer split at the cell
boundary so the parapet cap sits in the upper cell and the foot with the
standard `(32,20)` ground anchor in the lower; consumer composites draw
the anchor cell for the tall block then stack [c, r+1] above it (message
sent to the view lane). (2) **Ridge variants**: Mountain/DeepForest/
Forest/Rock each get `states.<Tile>.v2 = [16..19, 0]` — one extra column
with rows in base face order, same silhouette family under a permuted
bake seed so macro-blotch phases differ block-to-block. Honest cap
(unchanged): TWO variant phases per family means runs longer than 2
tiles still reuse a phase (adjacencies stay seamless, repetition is
bounded); per-coordinate sampling remains the future full answer.
Rollback (`atlas terrain`) now rebuilds the terrain pane WITHOUT
clobbering actor/prop/chrome mappings and clears these states; the
plain `terrain` -> `bake --tiles ...` cycle reapplies everything. Verify
enforces: plain keys always present alongside states, extra states keys
additive-tolerated, expanded grid dims gated on empty-vs-populated
states.

**Batch 2 (D38):** Forest (same metaball canopy recipe as DeepForest with
the lighter leaf palette), Rock (truncated-pyramid outcrop cluster — angles
do the reading, NW facets glint), Wall (keep cuboid: flat platform, hashed
merlons on all four parapet rims, coursed masonry skirts, merlon AO free
from the rig) and the RelicPedestal prop (stepped plinth + felt ring +
relic hemisphere, baked through the same rig then the shared chunky
outline pass). Wall/relief vocabulary lesson baked into v3: iso skirts
must shade from a **uniform per-material wall albedo** — inheriting the
silhouette column's top colour produced vertical "bark" striping on every
block. Facet-read note: Forest vs DeepForest use an identical crease/tip
k-spread; the single parameter that makes Forest's facets read better is
the leaf albedo luminance headroom under the clamp, not any geometry knob.
Door joinery remains procedurally painted (see the state contract below);
the Adjudicator colossus-arm has no manifest slot yet.

## Door states and passage

`cargo run -p realm-atlas -- doors` patches production door cells after the
terrain/material bake. It preserves every other existing terrain cell and
all actor, player, relic, prop and chrome sheets. Iso joinery is a separate
`doors.png` sheet: four transparent 96px plates with mapping keys `closed_x`,
`open_x`, `closed_y`, `open_y`. Square poses occupy columns 8–11 of its terrain
sheet. The plain `Door` mapping remains the closed-door fallback. Running
`terrain` resets square extensions; run `doors` again after rebuilding it.

The standing stone frame carries weathered timber planks/grain, a knot,
forged iron straps/rivets, hinge plates and a latch. Open poses swing the
thick leaf aside and reveal the actual floor through the threshold. Iso draws
the normal `Floor.<zone>` phase as ground, then a standing door child anchored
at plate A=(48,64), rendered at T×1.5 with a 40px source rise matching masonry.
The frame spans one 32px wall edge, not 56px: the latter embedded the jambs
and leaf in adjacent wall cells and made the visible door look undersized.
A full-depth lintel continues the entire wall cap over the opening. The open
leaf rotates into the passage perpendicular to the corresponding wall run.
The leaf/jamb plane sits on the exposed facade edge, not inside the wall
footprint: south (+y/2) for an x-run, east (+x/2) for a y-run. That half-cell
projection displacement aligns the joinery with the wall facade. Deeper
corridor doors can still be partly hidden by nearer walls.
It keeps normal tile-band prop ordering, so walls and actors still occlude it
from the appropriate side. Square poses retain that projection's current
shared floor. Its state keys are `Door.<zone><phase>_<closed|open>_<x|y>`;
the wall-run orientation follows the stronger pair of adjacent wall neighbours
(ties use x).

The simulation keeps the original `Tile::Door` save key and persists open
positions in `Map.open_doors`; older saves lacking that field load closed.
Successful player/NPC movement opens a door before occupying it, including
combat and forced movement, without an extra step or movement toll. Doors
therefore cannot strand an actor or break existing pathfinding. E toggles a
cardinally adjacent door after existing NPC/portal/chest/shrine priorities;
an occupied doorway cannot be closed. The terminal shows open doors as `/`.
The Bevy cell cache includes leaf state/orientation so interaction refreshes
the sprite immediately; closed doors occlude torch light, open ones do not.

`cargo run -p realms-view --bin realmscape -- doors` stages the real generated
cellar threshold at (20,14), initializes a Keepwarden, and captures both states
from the same unobstructed south/front stand (20,16): `e4-door-closed.png` at
frame 50 and `e4-door-open.png` at frame 100 in `docs/gfx/proto/`. It asserts bump-open,
occupied-close refusal, adjacent E close/reopen and return passage before
printing `DOOR SMOKE OK`. The `doors-square` runner exercises the same verbs
and writes `e4-door-square-closed.png` / `e4-door-square-open.png`.
The same runners also capture the perpendicular cellar doorway at (12,6),
then the town's thin-wall doorways at (10,25) and (14,9). The town captures
are `e4-door-wall-<x|y>-<closed|open>.png` (with `square-` before `wall-` for
square projection); these expose both wall joints and the continuous lintel.
The gallery uses these four town views. All openings use authored maps without
altering their wall layout; passage assertions run in both wall orientations.
Focused core regression tests are selected with
`cargo test -p laya-realms door_`.

Geometry correction verification: rebaked the production door atlas and ran
both headed door runners; cellar transitions and town x/y passage assertions
passed. Inspected closed/open wall joints and lintels in both orientations.
Refreshed the town view and all 27 map captures. Core gameplay is unchanged
by this art correction; the earlier full core suite passed 91 tests.

## Bake source: procedural 3D actor bakes (D38 rung 2)

Organic actors go through the mesh-SDF kit in `tools/atlas/src/bake_actor.rs`
(the "mesh SDF for silhouettes" the bake verdict flagged): parametric rigs in
the painters' own 32-unit figure grid — humanoids (tapered capsule torso,
limb capsules, skin/bone head, interchangeable headwear: none/cap/hood/mitre/
helm, robe gown or leg capsules, optional pauldrons) and quadrupeds (barrel
capsule, four leg capsules, neck/head/muzzle/ear/tail blocks) — rendered by
sphere tracing an orthographic front camera at 3× with NW key sun,
warm fill, 5-tap SDF AO and penumbra soft shadows, then the same chunky
±3px silhouette finish as the painted plates. Albedos re-use the painted
family colors (cloth/fold/skin/bone/steel/leather + per-archetype accents)
so in-place key swaps keep the reads; an emissive material flag covers glow
faces, drift motes and reagent vials. `atlas bake --actors` swaps the baked
set in place (players = `states.Player` row, `PaleStag.at_bay` = its states
slot — all cells shared with the painter layout); `atlas actors` stays the
full-painted rollback. Batch 1 = 6 class heroes + E5/E7 set (GnawThane,
PaleStag ×2 states, Tollmaster, Mirelight, FalseGlow, Alchemist,
OathlessCurate). Review artifact: `docs/gfx/proto/atlas-actors-3d-contact.png`
(painted | baked-3d pair per subject). Hard-won conventions: the figure grid
maps +z FORWARD and +y UP from the camera's side (v1 shipped both inverted,
cancelling only on symmetric parts — caught on review); hood masses stay
behind the face-cavity plane or the glow-face disappears; spectral tails
fade by post-bake alpha ramp, not geometry.

### Chrome sheet (D30 lane)

`chrome.png` (512×192, 8 cols × 3 rows, 17 keys) — the UI plate family,
baked by `atlas chrome` through the same depth-map + lighting rig (bevel
relief per pixel, then NW key / fill / cavity AO; **torch-lit, not flat**):
`OrbFrameRed`/`OrbFrameBlue` (brass ridge, stone well, hue gem north, four
rivets — consumers draw the fill INSIDE the well, over the inset), the
9-patch pane kit (`PanelCorner{NW,NE,SW,SE}` brass elbows with corner
boss, `PanelEdge{N,S,E,W}` brass bands, `PanelFill` mottled stone — elbows
join flush with bands, composite freely), `BossNameplate{L,M,R}` (arched
gothic brow with a red chip at the center, finials outboard),
`ButtonNormal`/`ButtonSelected` (brass frame + stone face; selected adds
the SEA-channel band + pip gem), `SigilPip` (octagonal housing, inset
rune-lit core), `XPSliver` (recessed rail with tick channels).
`atlas chrome` is its own rollback (idempotent re-bake with no painter
path); consumers fall back to flat-drawing without it.

The in-game modal panes use a single flat well with a thin brass rim instead
of assembling the 64px corner/edge plates: those large plates left a visible
corner-sized seam and a horizontal rail across headings. Atlas button strips,
item glyphs, orbs, and other chrome consumers still use their original plates.

Creation uses the shipped actor atlas for class and frame portraits and
`assets/ui/boon-icons.png` for three 128px boon emblems, indexed in model
order (Tithe pouch, Drilled marches, Take nothing). Regenerate the latter
with `python tools/gen_boon_icons.py` (Pillow required). Conversation
portraits also use the shipped actor atlas, while response labels and their
order come from `social::talk_options`; both overlays retain core's Enter
action for selections.

**Bloom discipline (from the nameplate-gold bleed):** the light pass caps
luminance at 1.30 and spec chips at 2px, so no pixel trips the post
stack's bloom band — verified on the demo contact before shipping.
Demo assembly proof (pane + orbs + nameplate + button row + pips +
sliver, plates only): `docs/gfx/proto/atlas-chrome-contact.png`.

**Rung-2 v2 (owner-flagged language rebuild, five targets):** heads gained
real volume + face cues (brow ridge slab, nose bump, twin sockets, mouth
line, role hair/hood masses) — no more bare capsules; bodies untricked
(waist taper with a leather belt + buckle at y 21, shoulder capsules now
enter pauldrons from underneath, smaller shoulder spheres); albedo ladder
re-keyed for family identities (cloth ×0.66 accent, fold ×0.40, exposure
lift) so civilians/guards/bosses stop sharing mud; every roster member got
its painted identity details back as geometry (Traveller bedroll + straps,
Vendor coin apron + brow band, Bandit kerchief, Smuggler parcel, Chief
2X-scale horn arcs, Matriarch tall brood-spines, Skeleton jaw-block +
sockets); render went 4× supersample (256→64) killing curve staircases.
Per-family verdict against painted (full table in the batch-2 report):
civilians TIE-or-better (v1 LOSS fixed), guards WIN (helmeted + spear),
bosses WIN (horns/crowns/colossus read), quadrupeds WIN. Nothing degraded
to the painted roster — `PAINTED_ONLY` stays at exactly two (the wisps).

**Batch 2 (rung 2, roster-wide):** baked set now covers all six class
heroes plus 24 archetypes — Commoner/Vendor/Thief/Traveller/Bandit/
Smuggler (shared civilian rig + headwear/pack/sack toggles), Guard
(helm + kite shield + spear), Skeleton (rib rungs + bare skull sockets),
Chief (horned helm + gold torc + great club), Lich (pronged crown +
sea-orb staff), Adjudicator (all-gold colossus + visor + gavel), Oracle
(ring-top staff + scroll bundle), Companion (cape + buckler + sword),
Tidemother (gown + kelp drapes + barnacle coronet), and quadrupeds
Rat/Wolf/Bear/Matriarch (brood-spines). **Spectral exemption (Main's call
after batch-1 review): Mirelight and FalseGlow stay emissive-painted** —
the rig's glow-body read as squat robots until it grows a bloom pass (may
wait for the engine's own bloom). `cmd_bake_actors` re-stamps
`PAINTED_ONLY` cells from painter code every run, so the revert survives
any `actors` / `bake --actors` ordering. Losing pitches recorded in the
review: wisps only (spectral identity); batch 2 needed one arm-fix
(Bandit's blade, v1 read as a generic hood) — everything else won or
tied against the painted plates.

## Future: image-model plates

When a plate outgrows procedural art, an image model renders a 64×64 (or
`N·64`) plate per Debug key — same grim palette, same anchor conventions.
The swap is **plate-source only**: a slice step maps generated art onto the
same cells and the manifest contract is unchanged. Style enforcement lives
at bake time (palette clamp + outline/dither pass in the same tool), not in
the renderer, so the exchange format, the renderer fallback rules and E4's
loading code never move when the plate source changes.

## Production 3D source path

The authored Blender/GLB → UV/material/rig → directional-render →
atlas-fragment pipeline is documented in
[`ART_3D_PIPELINE.md`](ART_3D_PIPELINE.md). Its 26-NPC sheet now replaces
the procedural NPC plates in `assets/atlas/actors.png` and the live Bevy
manifest. The earlier D19/D38 bakes remain reference/rollback art; the
procedural player/prop families retain their separate sources.
