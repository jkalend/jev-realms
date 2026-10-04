# Player character and equipment

Status: player and equipment plates ship in the runtime atlas.

The player is the one asset family that has to exist twice over, and the one
family whose look is defined by what the player is carrying. Both requirements
shape this pipeline.

## Two builds

Character creation is three beats - order, body frame, boon
(`crates/core/src/input.rs`). The frame is `model::Build` (`Male` / `Female`),
stored on `player.build`, and it is **presentation only**: no stat reads it. It
selects the model and the sprite key `Player.<Class>.<Build>`.

```sh
cargo run -p realms-view --bin realmscape
```

## Authoring data

```text
tools/asset3d/examples/player_profiles.json   # 6 orders: kit, head, garment, palette, clips
tools/asset3d/examples/create_blender_player_v1.py
tools/asset3d/examples/render_player_roster.py
```

Each profile declares a class weapon family (`sword`, `gravedigger`,
`gutterblade`, `oathrod`, `sigilfocus`, `fenward`), a head, a garment and a
palette. The order's identity is the kit plus the palette; the tier dresses it.

## Equipment is part of the model and visible at 64px

`player.weapon`, `player.armour` and `player.relic` are the tracked gear. The
Blender rig has four equipment placements:

| placement | holds | attached to |
|---|---|---|
| `weapon.R` | class weapon, tiers 0-3 | right hand |
| `forearm.L` | Keepwarden shield, tiers 0-3 | left forearm |
| `chest` | leather, iron, steel or gilded plate | torso |
| `relic` | one of nine named boss insignia | chest, in front of plate |

Parts are built in local equipment space then moved onto sockets. Their
`<asset>_GW...`, `_GS...`, `_GC...` and `_GR...` prefixes select the
deform bone. Bare bodies, weapons, armour and relics share a common raw
render frame; the runtime baker isolates changed pixels against the bare render
before fitting all layers using that body's bounding box. Fitting each weapon
independently would recenter long blades and break hand alignment.

## Rules this generator enforces

- **Sides are named `L` / `R`, never `-1` / `1`.** A part named `Pauldron-1`
  does not end in `L`, so the router sends the left ornament to the *right* arm
  and it rides across the body. `side_label` is the only way to build a
  side-named part.
- **Bone rotation happens in armature space.** `bone_rotate` rotates about a
  chosen armature axis through the bone's own head. A bone's local Euler axes
  depend on its roll, so `rotation_euler.z = 0.16` on an upright neck swings the
  head sideways instead of nodding it.
- **Object scale is baked into the mesh.** The glTF exporter rejects an unapplied
  object scale, and a scaled object distorts an armature modifier's rigid
  weights.
- **Curve primitives are converted to mesh before rigging.** The rig parents
  meshes only; a curve limb sits in the rest pose for every frame while its
  joint spheres animate around it.
- **Gear is built from explicit rods and boxes.** A bevelled curve's bevel radius
  does not survive rigid re-weighting reliably, and a thin blade comes out as a
  bead.

## Clip / frame contract

Poses are cut from per-animation tracks (`idle` 2, `walk` 2, `attack` 4, `hit` 2),
one frame per declared attack beat. The render pass detaches the rig action
before rendering, because an active action re-evaluates the pose bones on every
depsgraph update and silently overwrites the hand-applied pose.
`build_action_library` refuses to export a clip whose keys are identical, and the
idle and walk tracks drive the bones each body form actually has.

## Regenerating shipped assets

Run these **sequentially** (not in parallel with other Blender jobs). `--force`
refreshes cached Blender renders after an authoring change; omit it to resume
an interrupted bake. `--atlas` in the layer renderer asks Blender for only the
front idle frame used by the 64px atlas; the saved rig still contains the
authored animation action library.

```sh
python tools/asset3d/examples/render_player_layers.py \
  --root docs/gfx/proto/ui-v1/layers --only all --force
python tools/asset3d/examples/render_relic_overlays.py \
  --layers docs/gfx/proto/ui-v1/layers \
  --output docs/gfx/proto/ui-v1/relics --force
python tools/asset3d/examples/bake_player_sheet.py \
  --layers docs/gfx/proto/ui-v1/layers \
  --relics docs/gfx/proto/ui-v1/relics
python tools/asset3d/examples/make_gear_tier_sheet.py \
  --class Keepwarden --build Male \
  --output docs/gfx/proto/ui-v1/gear-ladder.png
python tools/asset3d/examples/make_gear_on_player.py \
  --class Keepwarden --build Male \
  --output docs/gfx/proto/ui-v1/mockups/gear-on-player.png
python tools/asset3d/examples/make_player_roster_sheet.py \
  --atlas assets/atlas --output docs/gfx/proto/ui-v1/player-roster.png
```

`assets/atlas/players.png` and `assets/atlas/player_relics.png` are registered
by `assets/atlas/manifest.json`. `atlas::Atlas::actor_ref` can resolve the
player sheet by the same key used for other actors.

## Gear swapping (done)

Equipped gear is part of the runtime sprite, not a bake-time decoration. The
plate key carries it: `Player.<Class>.<Build>.W<weapon>A<armour>`.

Layers are rendered independently and composited, so a gear change costs no
re-render:

| layer | renders | what it is |
|---|---|---|
| `body` | 12 | bare character, no weapon, no plate |
| `W<n>` | 48 | body + weapon at tier n |
| `A<n>` | 48 | body + plate at tier n |

The baker composes a 17 x 12 grid of 64px cells in `players.png`, 204 keys:
12 genuinely bare fallbacks in the seventeenth column and 192 weapon/armour
combinations from 108 source layers. Tiers 0-3 are leather, iron, bright steel
and gilded plate. The class weapon silhouette and the Keepwarden shield grow
with the tier.
The 64px previews above read these exact shipped atlas cells, not a separate
high-resolution Blender render or hand-painted gear mock.

`player_plate_keys` returns candidates best-first (exact gear, then the same
weapon on the starting plate, then the same plate on the starting weapon, then
both starting, then the bare body) so an unbaked tier degrades to a character
instead of a quad. `Actor` carries `tex_key` as well as `tex`, because a bare
`tex == 1` check would skip the repaint and leave the old weapon on screen.

## Relics (done)

A relic is a chest pendant, so folding it into the gear matrix would mean 12 x 16
x 9 plates. Instead each relic bakes as a transparent 64x64 overlay on its own
`player_relics` sheet, keyed `Player.Relic.<Relic>.<Build>` - 18 plates.

The overlay is the *difference* between the player wearing the relic and the
same player bare, mapped through the body layer's own fit transform, so the
pendant already sits where the chest socket is. Two traps, both hit:

- a plain `ImageChops.difference` is empty here - both renders are fully opaque
  over the body, so the delta's alpha is zero. The colour delta has to be used
  as a *mask*.
- the reference must be the BARE body. A reference that already wore a plate
  captures the whole cuirass instead of the pendant.

The view spawns the pendant as a second quad on the player's tile at its own
actor id, and despawns it when the relic is removed.

The eighteen relic cells use the Keepwarden bare body for the two build scales;
their high-contrast insignia sit ahead of the thickest tier-three cuirass.
Road and Grass use flat plates; terrain generation is covered in
`docs/ART_3D_PIPELINE.md`.

## Character fidelity refinement

The player generator shares `character_surface.py` with the NPC and hero
sources. Tunics/skirts use broad radial pleats and cape/robe panels carry
anchored gravity folds. Faces have a controlled jaw-to-cranium contour, hands
have tapered palms and thumbs, and weapon boxes have real bevelled edges.
Material values distinguish matte fabric, worn leather, bone and forged metal
without changing class palettes or the layer-difference compositing contract.

Tier armour now follows shoulder/chest/waist contours with shallow depth and
separate overlapping lower fauld lames. Higher tiers no longer increase the
whole torso's width/depth into a rectangular slab. Decorative ribs, rays and
gems remain small enough to leave the fitted cuirass visible. Equipment prefixes,
sockets, actions and the bare-body fitting transform are unchanged.

Refresh the twelve fully animated review/export models as well as all 108
runtime gear layers and 18 relic overlays:

```sh
python tools/asset3d/examples/render_player_roster.py --force
python tools/asset3d/examples/render_player_roster.py --process
python tools/asset3d/examples/render_player_layers.py \
  --root docs/gfx/proto/ui-v1/layers --only all --force
python tools/asset3d/examples/render_relic_overlays.py \
  --layers docs/gfx/proto/ui-v1/layers \
  --output docs/gfx/proto/ui-v1/relics --force
python tools/asset3d/examples/bake_player_sheet.py \
  --layers docs/gfx/proto/ui-v1/layers \
  --relics docs/gfx/proto/ui-v1/relics
```

For an isolated review bake, pass `--manifest <review-dir>/manifest.json` to
`bake_player_sheet.py` using a copy of the current atlas manifest. It writes the
two player sheets next to that manifest, leaving the production manifest alone.
Final integration must run the production bake after terrain/actor generation.

The front-pose model reviews are included in
`docs/gfx/proto/visual-v2/character-quality-pass1.png` and
`character-quality-pass2.png`. The second pass specifically reduced oversized
collar rings, pauldrons, gold fins and belt/fauld bulk after the first fresh
render still read as heavy trim around a slab. The isolated final 64px-cell
comparison is `character-quality-player-roster.png`; its sources are the same
108 production layers, not a separately painted illustration.

