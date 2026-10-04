# Production 3D asset pipeline

Status: **the improved 26-actor Blender roster is baked into the isometric game**.

The 24 improved `npc-roster-v2` models, hand-refined Guard and skeletal Lich
now feed the Bevy actor atlas. The terminal and older macroquad GUI retain
their separate text/procedural representations; the playable isometric
`realmscape` view reads the atlas.

## Target flow

```text
.blend / authored meshes
        ↓  Blender adapter
source/<Asset>.glb + source sidecar
        ↓  offline orthographic renderer
render/<Asset>/<animation>/<direction>/<frame>.png
        ↓  tools/asset3d/pipeline.py bake
3D sheet PNG + intermediate manifest fragment
        ↓  tools/asset3d/examples/compose_actor_sheet.py
assets/atlas/actors.png + the `actors` sheet in assets/atlas/manifest.json
```

The runtime remains a 2D isometric sprite renderer. The source is 3D so faces,
backs, equipment, silhouette and lighting can be authored consistently before
being baked into the existing atlas contract.

## What is implemented now

| Piece | Location | Status |
|---|---|---|
| Manifest contract | `tools/asset3d/schema/asset_manifest.schema.json` | Implemented |
| Validator / inspector / baker CLI | `tools/asset3d/pipeline.py` | Implemented and tested |
| Contract tests | `tools/asset3d/tests/test_pipeline.py` | Implemented |
| Blender export/inspection adapter | `tools/asset3d/blender_export.py` | Implemented; requires Blender to run |
| Procedural GLB fixture writer | `tools/asset3d/glb_writer.py` | Implemented; Lich example generated |
| Blender 5.1 Lich and Guard | `docs/gfx/proto/visual-v2/{lich,guard}-blender-5.1/` | Rigged GLBs, eight-direction renders, validated bakes; Lich has skeletal face, belt tome and staff |
| Improved 24-model roster | `docs/gfx/proto/visual-v2/npc-roster-v2/` | Separate GLBs and 256×320 render frames validated and baked into 96px game cells |
| Runtime fragment-to-atlas adapter | `tools/asset3d/examples/compose_actor_sheet.py` | **Wired.** Flattens all 26 improved archetype fragments into the `actors` sheet |
| Actor lanes in the runtime | `crates/view/src/actors.rs` `plate_keys` | **Wired.** idle/walk/attack/hit x 8 directions, resolved per actor per frame |
| Environment meshes: terrain | `tools/atlas/src/{bake3d,paint}.rs` | All 16 tile kinds mapped; seven heightfield families, eight Wall/Floor material-state families, painted roads/grass/interactive tiles and shared joins |
| Water and sightlines | `crates/view/src/tiles.rs` | Four position-hashed River/Ford phases; adjacent wall faces culled and walls within two tiles of the player faded |

## The actor sheet (runtime cutover)

`compose_actor_sheet.py` is the adapter. It reads the 24-archetype
`npc-roster-v2/` bakes plus the hand-refined `guard-blender-5.1/` and
`lich-blender-5.1/`, and writes one sheet. Regenerate it after any terrain
tool rewrites the atlas manifest: the actor sheet declares 96px `cell_size`,
while the terrain tiles use 64px.

* **Keys.** `<Arch>.<action>.<dir>.<frame>`, e.g. `Commoner.walk.front.000`.
  The atlas contract is only "key -> rect", so composite keys resolve with no
  change to the lookup. A flat `<Arch>` alias points at the idle front frame so
  any caller that still asks for a plate gets the 3D character.
* **Replaced, not shadowed.** The sheet keeps its `actors` name, so the old 26
  flat plates are overwritten rather than sitting behind the new art.
* **One transform per archetype.** The opaque-pixel bbox is taken over the union
  of all 56 cells. Cropping per frame would rescale and re-anchor every frame,
  so a walk cycle would breathe and its feet would slide.
* **96px cells.** Declared per sheet via `cell_size` in the manifest
  (`SheetDecl.cell_size` in `crates/view/src/atlas.rs`, defaulting to the global
  `tile_size`). An NPC draws at 1.3x a tile, so a 64px cell resampled down lost
  the face and the weapon.

`plate_keys` in the view derives the key per frame: facing from the tile step
(sampled *before* the glide update rewrites `current`), action from
move/hit/attack/idle, and a time-based frame ping-pong. The fallback chain is
`exact -> same lane frame 000 -> flat <Arch>`, so a lane with fewer frames than
asked for still resolves and an actor never drops to a colour quad.

## Environment status and in-game viewing

The `environment-v5-seamless-standard.png` study is a layout/material reference,
not a runtime texture. The actual game uses terrain baked into
`assets/atlas/terrain/{square,iso}.png`: all 16 tile kinds retain their map keys;
Mountain, DeepForest, Forest, Rock, River, Ford and Ruins receive lit heightfields
in both projections. Wall and Floor use flat material surfaces with painted
roughness, joints and wear; the iso sheet adds eight authored material families
through the existing `states` contract. Road, Grass and interactive plates remain
on the same 64px grid. Ground relief returns to the shared plane at the edges;
material-phase noise also tapers away before the shared boundary.
Wall faces are culled and nearby walls fade so creatures and usable cells stay
visible. Taller two-cell wall art is retained in the manifest for comparison
but is not drawn by the gameplay renderer.

```sh
target/debug/realmscape.exe city-live   # Millbrook: walkable streets and Guard
target/debug/realmscape.exe lich-live   # Underkeep: Skeletons and Lich boss
target/debug/realmscape.exe water-live  # Overworld: playable river and woods
target/debug/realmscape.exe ui          # all 14 modals -> docs/gfx/proto/ui-cases/
target/debug/realmscape.exe levels      # one capture per map -> docs/gfx/proto/levels/
```

The playable Bevy HUD uses live player HP, mana, stamina, XP, sigils and class
meter; its lower rail stays at the screen edge. The older macroquad GUI does
not consume this atlas.

## Environment meshes

`bake3d` is a heightfield renderer: displaced surfaces share the same baked
sun/fill/rim light. Floor is a flat worn material surface rather than separate
raised cubes. Crags stay inside their shared ground diamond; forest crowns use
overhanging prop sprites. Roads and grass remain flat for readable navigation.

Heights are budgeted against the cell, and the budget is not obvious: a feature
at the diamond's centre `(u,v)=(32,32)` can rise ~20px before its silhouette
leaves the cell, while a feature touching the cell's corners has only ~4px. An
overshooting feature is clipped flat at the cell top and its dark back-facing
interior shows through as a crater, so relief is centred, and `bake_iso`
additionally tapers all non-wall relief back to the ground plane near the
footprint edge.

### Walls

A wall is a **raised cap plus two exposed faces**, not a curtain on every cell.
The cap is a normal ground diamond that the *view* lifts by the face height
(`Proj::ISO_WALL_LIFT`), so a wall stands on its own cell and never bites into
the tile to its south. The earlier "ground clips into the wall" failure came
from the opposite model, hanging faces downward into the neighbouring cell.

Faces are 40 sheet px tall (`bake3d::WALL_FACE_H`), which does not fit under the
cap in one cell by the naive route: the south face's outer vertex would land at
`AY - 40` and be clipped, leaving a notch at every cell boundary along a run. So
the bake paints the face art `WALL_FACE_DROP` = 24px *lower* in its cell than it
belongs, and the view lifts the face sprite back by the matching
`Proj::ISO_WALL_FACE_LIFT`. That raises the usable ceiling to `AY + 24` = 44px.
Face height, drop and the view's two lifts must move together or a transparent
band opens between cap and face.

Faces now have staggered stone blocks, narrow mortar, small bevel highlights,
surface roughness and damp/dirty feet. Dressed ashlar uses three deep courses,
sanctum and Underkeep use two massive courses, while crypt and cave/woodland
stones have smaller or irregular joints. A separate heavy coping pattern covers
the cap; it is not a copy of the room's pavers or earth. The dock family uses
wood beams, iron straps and bolt highlights instead of masonry.

`tiles.rs` selects `Wall.<family>` and `Floor.<family>[2]` states with static keys;
there is no runtime zone tint and no per-frame key formatting. `Zone::of` maps
authored dungeon identities explicitly: Burrow/Crag Den are cave, Crimson Hollow
is sanctum, Underkeep is pale flagstone, Fen Barrow/cellar are crypt, Toll Camp
and cities are town, Mire/Hart/overworld are woodland, trial yards/final arena are
arena, and the Tidemother's Landing is dock. A dungeon index is not a theme;
modulo-three classification would incorrectly make the outdoor boss arenas
share unrelated dungeon interiors. Zone is included in the tile cache so a map
change at the same position refreshes both cap and exposed faces.

The expanded iso sheet is 3456×320 (54 columns): columns 30–53 hold wall, floor
and alternate-floor plates per family. The base square/iso Wall and Floor keys
use town material. Every family preserves identical cap/face alpha coverage,
the 40px face height, 24px face drop and both runtime lifts.

Two offline reviews of **the generated runtime atlas**, not concept art, are
saved in `docs/gfx/proto/visual-v2/environment-quality/`:

* `pass-1-{zones,plates}.png`: staggered masonry and eight ground patterns.
* `pass-2-{zones,plates}.png`: corrected soil-like coping, over-cloudy cave
  ground, weak sanctum/arena motifs and insufficient cave/dock wall distinction.
* `pass-2-woodland-water.png`: actual tree/grass/road/water cells; crown lobes
  remain inside their image and water no longer has alternating bank stripes.
* `runtime-water-correction-{run,plates}.png`: short curved current crests added
  after the first live water capture showed the calmer surface was too plain.
  The actual updated flow plates and a position-hashed river run show broken
  glints rather than alternating tile-wide strips.

The zone scenes composite atlas plates at the same relative anchor/lift values
as the view. They are offline asset proof, **not live gameplay captures**.
Regenerate the production terrain and tree props with:

```sh
cargo run -p realm-atlas -- terrain
cargo run -p realm-atlas -- bake --tiles Mountain,DeepForest,Forest,Rock,Wall,River,Ford,Ruins,Floor
cargo run -p realm-atlas -- props
cargo run -p realm-atlas -- doors
```

The door patch adds closed and swung-open joinery over the current zone floor
phases. Reapply it after terrain regeneration; see
[door states and passage](ART_PIPELINE.md#door-states-and-passage).

Final integration was exercised in the headed Bevy runner with `iso`, `water`,
`lich`, `iso-burrow`, `levels` and `ui`. The primary world captures, all 27 map
captures and all 14 modal captures now use the refined production characters
and zone terrain. Atlas smoke checked 2,088 mapped/state keys against image
bounds, eight wall/floor families, 96px actor cells, 204 player gear/body keys
and 18 relic overlays. The seven existing asset3d contract tests passed.

### Sprite layer order

Terrain, wall faces, trees, props and actors share **one band per tile**
(`Proj::tile_band`) with a sub-layer each: `Z_WALL_FACE` < `Z_TREE` < `Z_PROP` <
`Z_ACTOR` < `Z_EFFECT`, every one of them smaller than the band's 0.01 step.

They used to be three disjoint strata (terrain ~0.0x, props 1.0, actors 2.0),
which meant terrain could never draw over an actor - the reason NPCs cut
straight through walls. Keeping every layer inside the per-tile step is what lets
a wall occlude an actor standing behind it while that actor still draws over the
tile it stands on. New world sprites join this scheme as
`Proj::tile_band(pos) +` a `Proj::Z_*` constant; only the torch flame and light
(`lights.rs`) and the `Text2d` damage numbers (`effects.rs`) sit above it, and
deliberately so.

River and Ford are shallow dark teal surfaces with quiet directional ripples;
Ford adds submerged gravel. Four baked flow variants per water tile
(`River.v2..v5`, `Ford.v2..v5`) resolve via position hash. The phase changes
the current without replacing the entire channel with a white diamond.
Seven seed-scattered curved crest segments and subtle adjacent troughs give the
water a readable surface at game scale. They fade before the footprint edge;
neither the material nor the geometry creates a bank band on each water cell.
The water-only corrective bake is `cargo run -p realm-atlas -- bake --tiles River,Ford`.

### Ruins and Floor

`Ruins` is a room that came apart - surviving wall stubs at different heights, a
fallen lintel bridging two of them, rubble drifts, moss in the sheltered ground.
`Floor` stays exactly flat. Its material family supplies staggered pavers,
wet cobbles, diagonal sanctum inlay, cracked pale flags, packed cave earth,
woodland leaf mould/roots, inset arena slabs or dock planks. Two deterministic
interior-weathering phases share the same edge and joint layout.
Both base surfaces bake into square and iso sheets; material states are iso.
`Road` and `Grass` also stay flat, with compacted gravel and softer sparse
tufts respectively, so walkable routes remain readable.

### Woodland

`Forest` and `DeepForest` carry their volume in an **overhanging sprite**
(`tiles.rs::TreeSpr`), not in the tile plate: a 64px iso cell's ground diamond
already covers its own middle, so a crown painted inside the cell is hidden by
the plate it sits on. The plates paint undergrowth only; the trees are six
props-sheet silhouettes (`Tree1..3`, `DeepTree1..3`) picked by position hash,
drawn at `T * 1.55` and bottom-anchored so the trunk foot lands on the cell
anchor. One cell in five stays bare (`tree_bare`) so the crowns do not seal the
stand into a solid wall, and each tree takes its own height and brightness step
off the same hash - at one fixed size and tone a wood reads as a stamped hedge.

All of it hangs off one rule that is easy to get wrong: the second argument of
`Proj::Iso::world` is **z**, not a y offset. `CELL_ANCHOR_DY` is a *y*
correction for tile plates (`face_center`) and belongs nowhere else; passing it
as z parked every tree six units under the lowest terrain band, so the map drew
over it and only the crowns poked out past the map's edge.

## Source requirements

Every authored asset must provide:

- A `.glb` source file.
- UVs on every mesh.
- Named material slots with authored textures/material values.
- An armature or explicit rig marker.
- Applied object scale.
- `front`, `back`, and `pivot` empty markers.
- `feet_center` or `ground_center` pivot convention.
- A sidecar JSON containing the source hash and object list.
- For actors, players and bosses, an `action_profile` naming weapon-specific
  clips; bosses additionally declare signature and phase actions.

The Blender adapter checks these requirements before export. The manifest
requires them again at the pipeline seam, so a hand-authored or AI-generated
source cannot silently bypass the contract.

## Direction and animation contract

The example manifest uses eight directions in this order:

```text
front, front_right, right, back_right,
back, back_left, left, front_left
```

Sixteen-direction assets are supported by the schema; the exact labels are
carried in the manifest so a later adapter can use degree-indexed names without
guessing.

Actors, players and bosses require these animation lanes:

```text
idle, walk, attack, hit
```

Frame counts are explicit in the manifest. The first production slice should
keep these small; fidelity comes from clean meshes, materials and lighting
before frame-count explosion.

### Per-character action profiles

`animations` is the render-lane contract (`idle`, `walk`, `attack`, `hit`);
it is not enough to identify the actual clips. Every new actor, player and boss
manifest must also carry an `action_profile`:

```json
{
  "id": "boss_lich_staff",
  "weapon": "staff",
  "default_action": "lich_staff_idle_guard",
  "clips": {
    "idle": ["lich_staff_idle_guard"],
    "walk": ["lich_staff_walk_a", "lich_staff_walk_b"],
    "attack": ["lich_staff_cast_windup", "lich_staff_cast_release", "lich_staff_cast_recover"],
    "hit": ["lich_staff_hit_recoil"]
  },
  "boss": {
    "signature": "summon_and_curse",
    "phase_actions": ["lich_phase_summon", "lich_phase_curse", "lich_phase_reinforce"]
  }
}
```

The profile is selected by **archetype + weapon + role**, not by body type
alone. The minimum production matrix is:

| Profile family | Required identity clips |
|---|---|
| Unarmed | neutral idle, walk, empty-hand attack, hit |
| Sword / axe | weapon-specific windup, slash, recover, block |
| Spear / scythe | reach-specific thrust/sweep, guard, recover |
| Bow / crossbow | aim, loose, reload, flinch |
| Staff / caster orb | cast windup, release, recover, channel |
| Claw / natural weapon | swipe combo, pounce, recoil |
| Shield / support | guard, bash, rally, protect reaction |
| Boss | all family clips plus `signature` and phase actions |

Locomotion may be shared only when proportions and silhouette do not change
the action. Attack, hit, death and phase clips must be unique when the weapon,
defense role or boss mechanic changes the read. Bosses must declare a
signature action and phase actions; a generic four-row idle attack sheet is not
an acceptable boss asset.

`default_action` is the clip the Blender renderer binds for a resting model
before capturing the first frame. An unbound skeleton or T-pose is never a
valid sprite-sheet frame. The manifest profile is also the source of truth
for other viewers/importers that do not preserve Blender's active action.

The Lich uses the `boss_lich_staff` profile above. Other authored NPCs carry
their weapon-specific action profiles. The validator checks the profile when
present; omission is a legacy-fixture exception, not the production standard.
The full archetype/weapon roster is maintained in `docs/NPC_ACTION_MATRIX.md`.

Render files use this layout:

```text
render/Commoner/idle/front/000.png
render/Commoner/walk/front_left/001.png
```

Each render must be:

- RGBA PNG with non-zero alpha.
- Exactly the asset's declared `cell_size`.
- Orthographic, fixed camera and fixed light rig.
- Pivot-aligned at the feet/ground center.
- Named by the direction and frame contract above.

## Commands

Validate structure only (useful before source assets exist):

```sh
python tools/asset3d/pipeline.py validate tools/asset3d/examples/actor_manifest.json --no-files
```

Inspect a manifest:

```sh
python tools/asset3d/pipeline.py inspect tools/asset3d/examples/actor_manifest.json
```

Validate real source and render files:

```sh
python tools/asset3d/pipeline.py validate path/to/asset_manifest.json
```

Bake a validated set into an intermediate sheet and fragment:

```sh
python tools/asset3d/pipeline.py bake path/to/asset_manifest.json \
  --output-dir build/asset3d-bake \
  --sheet-name actors-3d-baked
```

The baker writes:

```text
build/asset3d-bake/actors-3d-baked.png
build/asset3d-bake/actors-3d-baked.fragment.json
```

The fragment includes per-cell source hashes, animation/direction/frame keys,
base mappings and state mappings. `compose_actor_sheet.py` reads these
fragments and writes their cells, animation keys and sheet size into the live
manifest. Do not run the procedural `realm-atlas actors` over that output.

## Blender export

Run inside Blender against a scene containing tagged asset objects:

```sh
blender path/to/scene.blend --background \
  --python tools/asset3d/blender_export.py -- \
  --manifest path/to/asset_manifest.json \
  --output-root build/asset3d
```

Blender 5.1 is the intended authoring target. Its glTF operator supports the
`GLB`, animation, skin, morph-target, material and selection export options
used by the adapter.

If Blender is not on `PATH`, use its full executable path:

```bat
set BLENDER_EXE=C:\path\to\Blender 5.1\blender.exe
"%BLENDER_EXE%" --background --python tools/asset3d/blender_probe.py
"%BLENDER_EXE%" path\to\scene.blend --background ^
  --python tools/asset3d/blender_export.py -- ^
  --manifest path\to\asset_manifest.json ^
  --output-root build\asset3d
```

The adapter requires object tags or an `<AssetId>__` naming prefix and checks
UVs, materials, rig markers, front/back markers, pivot marker and scale
before invoking the GLB exporter.

## Validation and test

Run the contract tests with:

```sh
python -m unittest discover -s tools/asset3d/tests -v
```

The production acceptance bar for a hero, boss or NPC is:

1. GLB opens independently and retains front/back geometry.
2. UVs, materials and rig markers validate.
3. Eight-direction idle/walk/attack/hit renders share one light rig and pivot.
4. Bake produces a deterministic sheet and fragment.
5. The live atlas resolves the model in a running game.

## Lich end-to-end example

The first executable example is generated by:

```sh
python tools/asset3d/examples/make_lich_example.py
```

Outputs live under `docs/gfx/proto/visual-v2/lich-3d-example/`:

- `source/Lich.glb` — procedural Lich mesh, planar UV stream, material slots,
  front/back/pivot markers and a minimal one-joint skin fixture.
- `lich_manifest.json` — eight directions and `idle/walk/attack/hit` frame
  contract.
- `render/Lich/**` — 56 validated RGBA directional renders.
- `baked/lich-3d-baked.png` — deterministic intermediate sheet.
- `baked/lich-3d-baked.fragment.json` — hashed cells and state mappings.
- `lich-3d-example.png` — human review contact sheet.

This early source → validate → render → bake fixture does not feed the game.
Its GLB has only a one-joint skin; the Blender-authored, multi-bone Lich below
supersedes it without changing the manifest or render layout contract.

### Blender 5.1 actual export

Blender 5.1 was located at `F:\Blender\blender.exe` and verified headlessly:

```text
BLENDER_VERSION=5.1.0
GLTF_EXPORTER=available
```

The current Blender scene/export is under
`docs/gfx/proto/visual-v2/lich-blender-5.1/`:

- `blender/Lich.blend` — editable Blender source scene.
- `tools/asset3d/examples/create_blender_lich_v6.py` — reproducible
  articulated rig built on the v5 robe, hood, skull and equipment geometry.
- `source/Lich.glb` — rigged Blender-exported source with named staff and
  boss-phase action clips.
- `source/Lich.json` — Blender export sidecar and source hash.
- `render/Lich/**` — 56 Blender-rendered directional/animation PNGs.
- `baked/lich-blender-5.1-baked.{png,fragment.json}` — game-sheet input.

The in-game Lich has an exposed skull with recessed sockets, nose cavity,
jaw and teeth, open cowl, paired ribs, a gold-bound tome strapped to the belt
and a staff constrained to the right hand. Separate arm/forearm/hand bones
drive its attack and walk poses; the saved GLB carries the named action library
while the game uses the corresponding directional baked frames.

The stiff-arm v5 pass is preserved at `lich-blender-5.1-v5-stiff-arms-rejected/`;
the round v4 pass is at `lich-blender-5.1-v4-round-rejected/`; the imported
cone-ish pass is at `lich-blender-5.1-imported-cone-rejected/`; the older
primitive-only proof is at `lich-blender-5.1-cone-rejected/`. None is the
current example.

The export, action-name validation and deterministic bake completed successfully.

### Guard end-to-end example

The first non-boss weapon-specific asset is under
`docs/gfx/proto/visual-v2/guard-blender-5.1/`:

- `blender/Guard.blend` — editable Guard source scene.
- `tools/asset3d/examples/create_blender_guard_v1.py` — Guard authoring source.
- `source/Guard.glb` — rigged source with named spear/shield clips.
- `guard_manifest.json` — `npc_guard_spear` action profile and render contract.
- `render/Guard/**` and `baked/guard-blender-5.1-baked.{png,fragment.json}`
  — 56 directional frames and the game-sheet fragment.

The Guard has an open-faced crested helmet, fitted cuirass/backplate and belt,
kite shield and hand-aligned spear. Its manually rendered action frames differ
across idle/walk/attack/hit; the saved Blender scene retains the named action
library. Both heroes are live in `assets/atlas/actors.png`.

### Remaining NPC roster

The other 24 canonical NPCs are authored through the shared pipeline in
`docs/NPC_ROSTER_PIPELINE.md`. Their visual-v2 meshes now feed the live atlas;
Skeleton in the Underkeep uses the exposed-bone model, not the older
striped-tunic source. Player classes and `BroodHole` retain separate sheets.

## Runtime boundary

The game renders validated Blender meshes as offline directional sprites,
not live GLB files; Blender is never a runtime dependency. `realm-atlas actors`
still emits procedural reference art and will replace the production
sheet if run by itself. Re-run `compose_actor_sheet.py` after actor/terrain
bake commands that rewrite the atlas manifest, preserving actor `cell_size: 96`.
