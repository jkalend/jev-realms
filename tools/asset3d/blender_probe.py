"""Print Blender version and glTF exporter availability.

Run with:
    blender --background --python tools/asset3d/blender_probe.py
"""

from __future__ import annotations

import bpy  # type: ignore


def main() -> int:
    print(f"BLENDER_VERSION={bpy.app.version_string}")
    try:
        bpy.ops.export_scene.gltf.get_rna_type()
    except Exception as exc:  # Blender raises its own RNA/operator errors
        print(f"GLTF_EXPORTER=unavailable: {exc}")
        return 2
    print("GLTF_EXPORTER=available")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
