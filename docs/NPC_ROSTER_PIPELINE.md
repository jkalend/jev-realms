# NPC roster production pass

Status: all 26 authored NPCs are baked into the isometric game's `actors` sheet.

Lich and Guard use their hand-refined Blender sources. The other 24 canonical
`Archetype` NPCs now use the separate visual-v2 geometry (including Skeleton).
Player classes and `BroodHole` (the prop/decal) remain separate asset families.

Authoring data:

```text
tools/asset3d/examples/npc_roster_profiles.json
```

Shared generator:

```text
tools/asset3d/examples/create_blender_npc_roster_v1.py
```

Each profile defines:

- form: humanoid, undead, wisp, beast, rat or stag
- palette and scale
- headgear/silhouette family
- `garment`: the silhouette layer - `robe`, `cloak`, `apron`, `brigandine`,
  `vest` or `none`. A 128px cell reads the outline, not the trim, so the
  garment is what separates two humanoids at a glance.
- weapon or role prop
- `action_profile` with named clips and a required `default_action`
- boss signature and phase actions where applicable

Palette hexes are authored as sRGB and converted to linear by `rgba()` before
they reach a Blender base colour; feeding raw sRGB renders the whole roster
washed out and desaturated.

### Face and headgear

`add_face` sizes sockets, irises, brow, nose, cheeks, ears and mouth to survive a
128x160 cell - roughly 33px per model unit, so anything under 0.05 units is
sub-pixel. Full-head gear (hood, helm, crown, mitre) is built by
`add_head_shell` as a shell pushed behind the face plane plus a brim ring and a
rear peak, never a sphere enclosing the face.

### Pose tracks

`apply_pose` owns a pose track per animation (`TRACK_LENGTH`): idle 2, walk 2,
attack 4, hit 2. `render_counts` renders one frame per declared attack beat, and
`clip_frames` cuts each declared clip from the same track, so a rendered frame
and the matching GLB clip always show the same pose.

`render_frames` detaches `rig.animation_data.action` before rendering. An active
action re-evaluates the pose bones on every depsgraph update and silently
overwrites hand-applied poses, which freezes every frame to the default idle.
The saved `.blend` keeps the action library and its `default_action`.

Outputs are per-NPC directories under:

```text
docs/gfx/proto/visual-v2/npc-roster-v1/<Archetype>/
```

The roster is a shared contract, not a claim that every archetype has final
hand-authored art. Each asset is independently reviewable and must pass the
same manifest, GLB, render, action-name and bake checks as the Lich and Guard.

After the sequential render pass, process every asset with:

```sh
python tools/asset3d/examples/process_npc_roster.py \
  --roster-root docs/gfx/proto/visual-v2/npc-roster-v1
```

### Improved 24-model production roster

`--visual-v2` on `create_blender_npc_roster_v1.py` selects geometry in
`npc_roster_visual_v2.py` and `npc_roster_visual_v2_creatures.py`.
The 17 humanoid/undead profiles have contoured heads and open-face hoods,
shoulder-level cloak/robe wraps, shaped weapons and skinned sleeves/leggings.
Skeleton has open ribs, a rear spine, pelvis, recessed eye cavities, jaw and
individual fingers. Wolf, Bear, Rat, GnawThane, PaleStag, Mirelight and
FalseGlow have distinct quadruped or spectral bodies. Renders are authored at
256×320; the production composer scales each archetype's whole animation
sequence together into bottom-anchored 96×96 game cells.

Every one of the 24 models has its own `.blend`, `.glb`, manifest and
eight-direction idle, walk, attack and hit renders under
`docs/gfx/proto/visual-v2/npc-roster-v2/<Name>/`. Compare all profiles at
`docs/gfx/proto/visual-v2/npc-roster-v2-all-contact.png`; the larger five-model
face, hood, and skeleton detail sheet is `npc-roster-v2-contact.png` beside it.
Open `docs/gfx/proto/visual-v2/index.html` for the compact production gallery:
current world captures, one roster contact, and expandable source catalogs.
Guard and Lich remain separate Blender sources. The production composer reads
all 24 `npc-roster-v2/<Name>/baked/` fragments plus the two hand-refined heroes
and writes `assets/atlas/actors.png` and its 96px `cell_size` mapping:

```sh
python tools/asset3d/examples/compose_actor_sheet.py
```

Regenerate and export one model before rebuilding its fragment and game sheet:

```sh
F:/Blender/blender.exe --background \
  --python tools/asset3d/examples/create_blender_npc_roster_v1.py -- \
  --npc Chief --output-root docs/gfx/proto/visual-v2/npc-roster-v2/Chief \
  --visual-v2
F:/Blender/blender.exe docs/gfx/proto/visual-v2/npc-roster-v2/Chief/blender/Chief.blend \
  --background --python tools/asset3d/blender_export.py -- \
  --manifest docs/gfx/proto/visual-v2/npc-roster-v2/Chief/chief_manifest.json \
  --output-root docs/gfx/proto/visual-v2/npc-roster-v2/Chief
python tools/asset3d/pipeline.py validate \
  docs/gfx/proto/visual-v2/npc-roster-v2/Chief/chief_manifest.json
python tools/asset3d/pipeline.py bake \
  docs/gfx/proto/visual-v2/npc-roster-v2/Chief/chief_manifest.json \
  --output-dir docs/gfx/proto/visual-v2/npc-roster-v2/Chief/baked \
  --sheet-name chief-baked
python tools/asset3d/examples/compose_actor_sheet.py
```

Use `make_npc_roster_contact_sheet.py` with `--columns` and `--thumb-width`
only for optional art comparison; it is not the game asset. Never run
`atlas actors` after composition without composing again: that command emits
the old procedural actor sheet instead.

### Character fidelity refinement

`character_surface.py` supplies deterministic, mesh-authored drape folds and
exportable PBR material values shared by NPC, Guard/Lich and player generators.
Wool/linen are matte with restrained sheen; leather and bone have broad
highlights; steel retains distinct forged edges. These are geometry and material
changes, not a post-render filter. Existing profiles, rigs, clip names and pivots
remain the production contract.

Humanoid drapes have subdivided gravity folds, tunics/skirts have restrained
radial pleats, and hands have tapered palms and separate thumbs. Commoner/Vendor
vest closures are narrow sewn openings rather than bright paired chest cords.
Creature lofts use continuous organic normals while antlers, claws and crown
tips retain their edges; muzzle lips follow the skull. Guard has a tapered
breastplate and central ridge; Lich has additional cape/robe fold volume.

Regenerate every canonical NPC with the command above, changing `--npc` and its
matching output directory for each key in `npc_roster_profiles.json`, then run:

```sh
python tools/asset3d/examples/process_npc_roster.py \
  --roster-root docs/gfx/proto/visual-v2/npc-roster-v2 --force
F:/Blender/blender.exe --background \
  --python tools/asset3d/examples/create_blender_guard_v1.py -- \
  --output-root docs/gfx/proto/visual-v2/guard-blender-5.1
F:/Blender/blender.exe --background \
  --python tools/asset3d/examples/create_blender_lich_v6.py -- \
  --output-root docs/gfx/proto/visual-v2/lich-blender-5.1
```

The hero output directories intentionally match `compose_actor_sheet.py`.
Export each saved hero `.blend` with `blender_export.py`, validate its manifest,
and bake its fragment before composing; generation alone does not refresh the
runtime. Blender jobs must run sequentially.

Two fresh rendered review passes are retained at
`docs/gfx/proto/visual-v2/character-quality-pass1.png` and
`character-quality-pass2.png`. Each shows the same thirteen representative NPC,
hero, creature and player models in idle, walk, attack and hit poses. These
are front-pose art probes, not replacements for the complete direction/action
production renders. Pass two kept metal panels rigid rather than treating
them as cloth, rounded the beast torso/neck/skull joins, and restrained player
collars, shoulder ornaments and fauld bulk. The full 24-NPC production contact
sheet remains `npc-roster-v2-all-contact.png`.

