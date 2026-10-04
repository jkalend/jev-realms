# Laya Realms asset3d pipeline

Standalone authored-3D source and bake tooling. It does not load models into the
Rust runtime and does not overwrite `assets/atlas/manifest.json`.

## Quick start

```sh
python tools/asset3d/pipeline.py validate tools/asset3d/examples/actor_manifest.json --no-files
python -m unittest discover -s tools/asset3d/tests -v
```


The first end-to-end fixture is the Lich example:

```sh
python tools/asset3d/examples/make_lich_example.py
```

It writes a GLB, 56 directional/animation PNGs, a baked sheet and an
intermediate fragment under `docs/gfx/proto/visual-v2/lich-3d-example/`.

The verified Blender 5.1 version of the Lich example uses a skeletal face,
peaked hood, rib hints, angular cape, curved arms and a hand-constrained staff:
New actor/player/boss manifests also carry an `action_profile`: weapon-specific
clip names plus boss signature/phase actions. The generic examples demonstrate
an unarmed Commoner and a gavel-wielding Adjudicator; render lanes alone are
not considered a complete action set.

```bat
"F:\Blender\blender.exe" --background ^
  --python tools/asset3d/examples/create_blender_lich_v6.py -- ^
  --output-root docs/gfx/proto/visual-v2/lich-blender-5.1
"F:\Blender\blender.exe" docs/gfx/proto/visual-v2/lich-blender-5.1/blender/Lich.blend ^
  --background --python tools/asset3d/blender_export.py -- ^
  --manifest docs/gfx/proto/visual-v2/lich-blender-5.1/lich_manifest.json ^
  --output-root docs/gfx/proto/visual-v2/lich-blender-5.1
```
The v6 authoring script writes the directional PNGs, manifest and named action
clips alongside the `.blend`; after the adapter exports `source/*.glb`, validate and bake:

```sh
python tools/asset3d/examples/make_blender_lich_preview.py \
  docs/gfx/proto/visual-v2/lich-blender-5.1 \
  --output docs/gfx/proto/visual-v2/lich-blender-5.1/lich-blender-5.1-preview.png
python tools/asset3d/pipeline.py validate docs/gfx/proto/visual-v2/lich-blender-5.1/lich_manifest.json
python tools/asset3d/pipeline.py bake docs/gfx/proto/visual-v2/lich-blender-5.1/lich_manifest.json \
  --output-dir docs/gfx/proto/visual-v2/lich-blender-5.1/baked \
  --sheet-name lich-blender-5.1-baked
```

See `docs/NPC_ACTION_MATRIX.md` for the per-character weapon/boss roster and
`docs/ART_3D_PIPELINE.md` for the source contract and runtime boundary.
