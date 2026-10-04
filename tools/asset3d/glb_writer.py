"""Small dependency-free GLB writer for pipeline fixtures and procedural studies.

It writes a valid glTF 2.0 binary from the prototype Mesh type. This is not a
replacement for Blender export; it makes the source/render contract executable
without requiring Blender for procedural examples.
"""

from __future__ import annotations

import json
import math
import struct
from array import array
from pathlib import Path
from typing import Any


def _pad4(data: bytes, fill: bytes = b"\x00") -> bytes:
    return data + fill * ((4 - len(data) % 4) % 4)


def _chunk(chunk_type: int, data: bytes) -> bytes:
    return struct.pack("<II", len(data), chunk_type) + _pad4(data, b" " if chunk_type == 0x4E4F534A else b"\x00")


def _normal(a: tuple[float, float, float], b: tuple[float, float, float], c: tuple[float, float, float]) -> tuple[float, float, float]:
    u = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
    v = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
    n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    length = math.sqrt(sum(x * x for x in n)) or 1.0
    return tuple(x / length for x in n)  # type: ignore[return-value]


def write_glb(mesh: Any, path: Path, asset_id: str, materials: dict[str, tuple[int, int, int]], extras: dict[str, Any] | None = None) -> None:
    """Write a flat-shaded GLB with one primitive per material."""
    groups: dict[str, list[tuple[int, ...]]] = {}
    for face, material in zip(mesh.faces, mesh.materials):
        groups.setdefault(material, []).append(face)

    buffer = bytearray()
    buffer_views: list[dict[str, Any]] = []
    accessors: list[dict[str, Any]] = []
    materials_json: list[dict[str, Any]] = []
    material_index: dict[str, int] = {}
    primitives: list[dict[str, Any]] = []

    def add_view(data: bytes, target: int | None = None) -> int:
        while len(buffer) % 4:
            buffer.append(0)
        offset = len(buffer)
        buffer.extend(data)
        view: dict[str, Any] = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
        if target is not None:
            view["target"] = target
        buffer_views.append(view)
        return len(buffer_views) - 1

    def add_accessor(view: int, component_type: int, count: int, type_name: str, minimum: list[float] | None = None, maximum: list[float] | None = None) -> int:
        accessor: dict[str, Any] = {"bufferView": view, "componentType": component_type, "count": count, "type": type_name}
        if minimum is not None:
            accessor["min"] = minimum
        if maximum is not None:
            accessor["max"] = maximum
        accessors.append(accessor)
        return len(accessors) - 1

    for material, faces in groups.items():
        if material not in material_index:
            material_index[material] = len(materials_json)
            rgb = materials.get(material, (180, 180, 180))
            materials_json.append({
                "name": material,
                "pbrMetallicRoughness": {
                    "baseColorFactor": [rgb[0] / 255, rgb[1] / 255, rgb[2] / 255, 1],
                    "metallicFactor": 0.65 if material in {"gold", "gold_light", "steel", "steel_light"} else 0.05,
                    "roughnessFactor": 0.32 if material in {"gold", "gold_light", "steel", "steel_light"} else 0.78,
                },
                "doubleSided": True,
            })
        positions: list[float] = []
        normals: list[float] = []
        uvs: list[float] = []
        joints: list[int] = []
        weights: list[float] = []
        indices: list[int] = []
        for face in faces:
            for i in range(1, len(face) - 1):
                tri = (face[0], face[i], face[i + 1])
                a, b, c = (mesh.vertices[index] for index in tri)
                n = _normal(a, b, c)
                for vertex in (a, b, c):
                    positions.extend(vertex)
                    normals.extend(n)
                    # A deterministic planar UV gives the source a real UV
                    # stream; production art can replace it with authored UVs.
                    uvs.extend((vertex[0] * 0.25 + 0.5, vertex[2] * 0.25 + 0.5))
                    joints.extend((0, 0, 0, 0))
                    weights.extend((1.0, 0.0, 0.0, 0.0))
                    indices.append(len(indices))
        pos_array = array("f", positions)
        norm_array = array("f", normals)
        uv_array = array("f", uvs)
        index_array = array("I", indices)
        joint_array = array("H", joints)
        weight_array = array("f", weights)
        joint_view = add_view(joint_array.tobytes())
        weight_view = add_view(weight_array.tobytes())
        joint_accessor = add_accessor(joint_view, 5123, len(joints) // 4, "VEC4")
        weight_accessor = add_accessor(weight_view, 5126, len(weights) // 4, "VEC4")
        pos_view = add_view(pos_array.tobytes())
        norm_view = add_view(norm_array.tobytes())
        uv_view = add_view(uv_array.tobytes())
        index_view = add_view(index_array.tobytes(), 34963)
        pos_min = [min(positions[i::3]) for i in range(3)]
        pos_max = [max(positions[i::3]) for i in range(3)]
        pos_accessor = add_accessor(pos_view, 5126, len(positions) // 3, "VEC3", pos_min, pos_max)
        norm_accessor = add_accessor(norm_view, 5126, len(normals) // 3, "VEC3")
        uv_accessor = add_accessor(uv_view, 5126, len(uvs) // 2, "VEC2")
        index_accessor = add_accessor(index_view, 5125, len(indices), "SCALAR")
        primitives.append({
            "attributes": {"POSITION": pos_accessor, "NORMAL": norm_accessor, "TEXCOORD_0": uv_accessor, "JOINTS_0": joint_accessor, "WEIGHTS_0": weight_accessor},
            "indices": index_accessor,
            "material": material_index[material],
            "mode": 4,
        })

    # A small node animation makes the source inspectable as a rigged example;
    # production meshes should replace this with real armature skinning.
    time_data = array("f", [0.0, 0.5, 1.0])
    rot_data = array("f", [0, 0, 0, 1, 0, 0.0436, 0, 0.99905, 0, 0, 0, 1])
    time_view = add_view(time_data.tobytes())
    rot_view = add_view(rot_data.tobytes())
    time_accessor = add_accessor(time_view, 5126, 3, "SCALAR", [0.0], [1.0])
    inverse_bind = array("f", [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
    inverse_view = add_view(inverse_bind.tobytes())
    inverse_accessor = add_accessor(inverse_view, 5126, 1, "MAT4")
    rot_accessor = add_accessor(rot_view, 5126, 3, "VEC4")
    meshes = [{"name": f"{asset_id}_Mesh", "primitives": primitives, "skin": 0}]
    nodes = [
        {"name": f"{asset_id}_Rig", "children": [1, 2, 3, 4, 5], "extras": {"asset3d_rig": True}},
        {"name": f"{asset_id}_Mesh", "mesh": 0, "skin": 0},
        {"name": f"{asset_id}_front", "extras": {"asset3d_marker": "front"}},
        {"name": f"{asset_id}_back", "extras": {"asset3d_marker": "back"}},
        {"name": f"{asset_id}_pivot", "translation": [0, 0, 0], "extras": {"asset3d_marker": "pivot"}},
        {"name": f"{asset_id}_Joint", "translation": [0, 0, 0], "extras": {"asset3d_joint": True}},
    ]
    gltf: dict[str, Any] = {
        "asset": {"version": "2.0", "generator": "laya-asset3d procedural fixture", "extras": extras or {}},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials_json,
        "skins": [{"joints": [5], "skeleton": 5, "inverseBindMatrices": inverse_accessor}],
        "animations": [{
            "name": "idle",
            "samplers": [{"input": time_accessor, "output": rot_accessor, "interpolation": "LINEAR"}],
            "channels": [{"sampler": 0, "target": {"node": 5, "path": "rotation"}}],
        }],
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(buffer)}],
    }
    json_bytes = _pad4(json.dumps(gltf, separators=(",", ":")).encode("utf-8"), b" ")
    bin_bytes = _pad4(bytes(buffer), b"\x00")
    total_length = 12 + 8 + len(json_bytes) + 8 + len(bin_bytes)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as handle:
        handle.write(struct.pack("<4sII", b"glTF", 2, total_length))
        handle.write(struct.pack("<II", len(json_bytes), 0x4E4F534A))
        handle.write(json_bytes)
        handle.write(struct.pack("<II", len(bin_bytes), 0x004E4942))
        handle.write(bin_bytes)
