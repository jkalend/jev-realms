#!/usr/bin/env python3
"""Author the six player classes in Blender 5.1, in both body builds.

The Lich and Guard are the reference assets; this covers the player, which the
roster generator deliberately excluded. Two things make the player a different
asset family from the NPC roster:

  * both builds - `Player.<Class>.<Build>` - so the creation screen's frame
    choice is visible in the model, not just in a name;
  * equipment sockets. The rig carries a `hand.R` and a `chest` bone, and
    `build_gear` parents an authored item model to one of them, so an equipped
    item is part of the same GLB rather than a separate overlay.

Run one asset:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_player_v1.py -- \
      --class Keepwarden --build Male --output-root docs/gfx/proto/ui-v1/player/Keepwarden.Male
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import create_blender_lich_v4 as base
from character_surface import folded_panel, cloth_radius

PROFILE_PATH = Path(__file__).with_name("player_profiles.json")
PROFILES = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
TAIL = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
ROOT = Path(TAIL[TAIL.index("--output-root") + 1]).resolve()
CLASS_NAME = TAIL[TAIL.index("--class") + 1]
BUILD = TAIL[TAIL.index("--build") + 1]
PROBE = "--probe" in TAIL
ATLAS_ONLY = "--atlas" in TAIL
GEAR = [a for a in TAIL if a.startswith("--gear=")]
# --gear drives which layers are authored, so the atlas can composite them at
# bake time and the view can swap a weapon without re-rendering the body:
#   -1        bare body (no weapon, no plate, no relic)
#   W<n>      body + weapon at tier n, no plate
#   A<n>      body + plate at tier n, no weapon
#   <n>       both at tier n (the review render)
GEAR_TIER = 1
GEAR_LAYER = "both"
if GEAR:
    raw = GEAR[0].split("=")[1]
    if raw == "-1":
        GEAR_TIER, GEAR_LAYER = -1, "none"
    elif raw[0] in "WA":
        GEAR_LAYER, GEAR_TIER = "weapon" if raw[0] == "W" else "armour", int(raw[1:])
    else:
        GEAR_TIER = int(raw)
RELIC = next((a.split("=")[1] for a in TAIL if a.startswith("--relic=")), None)

BASE_ROT = (math.pi / 2, 0, 0)
CELL = (128, 160)
DIRECTIONS = base.DIRECTIONS

# Two builds, one rig. These are shoulder, waist, chest and limb ratios; the
# face and the class kit are identical so the read stays "same order, other frame".
BUILDS = {
    "Male": {
        "scale": 1.04,
        "shoulder": 0.38,
        "waist": 0.28,
        "chest": 0.39,
        "hip": 0.21,
        "limb": 0.125,
        "forearm": 0.100,
        "arm_spread": 0.02,
        "head": 0.24,
        "torso_depth": 0.25,
    },
    "Female": {
        "scale": 0.98,
        "shoulder": 0.32,
        "waist": 0.23,
        "chest": 0.34,
        "hip": 0.22,
        "limb": 0.110,
        "forearm": 0.088,
        "arm_spread": 0.018,
        "head": 0.23,
        "torso_depth": 0.23,
    },
}

ANIMATIONS = {"idle": 2, "walk": 2, "attack": 3, "hit": 2}
TRACK_LENGTH = {"idle": 2, "walk": 2, "attack": 4, "hit": 2, "phase": 2}


def rgba(value: str) -> tuple[float, float, float, float]:
    """Hex -> linear RGBA. Blender base colours are linear; raw sRGB renders washed."""
    value = value.lstrip("#")
    channels = [int(value[i : i + 2], 16) / 255 for i in (0, 2, 4)]
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels) + (1.0,)


def make_materials(cfg: dict) -> dict[str, bpy.types.Material]:
    p = cfg["palette"]
    cloth = rgba(p["cloth"])
    return {
        "skin": base.material("Player_Skin", rgba(p["skin"]), 0.0, 0.56),
        "cloth": base.material("Player_Cloth", cloth, 0.04, 0.54),
        "cloth_shade": base.material("Player_ClothShade", tuple(c * 0.68 for c in cloth[:3]) + (1.0,), 0.02, 0.68),
        "accent": base.material("Player_Accent", rgba(p["accent"]), 0.5, 0.34),
        "metal": base.material("Player_Metal", rgba(p["metal"]), 0.62, 0.38),
        "leather": base.material("Player_WornLeather", rgba("#705037"), 0.02, 0.88),
        "iron": base.material("Player_DullIron", rgba("#627077"), 0.42, 0.68),
        "steel": base.material("Player_BrightSteel", rgba("#BCCAD2"), 0.72, 0.27),
        "gold": base.material("Player_RelicGold", rgba("#E5B954"), 0.66, 0.31),
        "bone": base.material("Player_RelicBone", rgba("#DAD0AC"), 0.03, 0.66),
        "dark": base.material("Player_Dark", rgba("#0A0C10"), 0.0, 0.76),
        "glow": base.material("Player_Glow", rgba("#3FC0B0"), 0.1, 0.28),
    }


def tag(obj: bpy.types.Object, name: str, asset: str, mat=None) -> bpy.types.Object:
    obj.name = name
    obj["asset3d_id"] = asset
    if mat is not None and hasattr(obj.data, "materials") and mat.name not in {s.name for s in obj.data.materials}:
        obj.data.materials.append(mat)
    return obj


def _bake_scale(obj: bpy.types.Object, scale) -> None:
    """Bake the primitive's scale into the mesh.

    The glTF exporter rejects an unapplied object scale, and a scaled object also
    distorts an armature modifier's rigid weights, so the size belongs in the
    vertices rather than the transform.
    """
    from mathutils import Matrix

    obj.data.transform(Matrix.Diagonal((scale[0], scale[1], scale[2], 1.0)))
    obj.scale = (1.0, 1.0, 1.0)


def add_uv_sphere(name, loc, scale, mat, asset) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=20, ring_count=12, location=loc)
    obj = bpy.context.object
    _bake_scale(obj, scale)
    bpy.ops.object.shade_smooth()
    return tag(obj, name, asset, mat)


def add_ico(name, loc, scale, mat, asset) -> bpy.types.Object:
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=1.0, location=loc)
    obj = bpy.context.object
    _bake_scale(obj, scale)
    bpy.ops.object.shade_smooth()
    return tag(obj, name, asset, mat)


def add_tube(name, points, radius, mat, asset) -> bpy.types.Object:
    curve = bpy.data.curves.new(name, "CURVE")
    curve.dimensions = "3D"
    curve.bevel_depth = radius
    curve.bevel_resolution = 2
    spline = curve.splines.new("POLY")
    spline.points.add(len(points) - 1)
    for point, co in zip(spline.points, points):
        point.co = (*co, 1.0)
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    tag(obj, name, asset, mat)
    # A curve object is not a MESH, and the rig only parents meshes. Left as a
    # curve the limb sits in the rest pose for every frame while its joint
    # spheres animate around it - the classic detached-limb rig bug.
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.convert(target="MESH")
    return bpy.context.object


def add_torus(name, loc, major, minor, mat, asset, rotation=(0, 0, 0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major, minor_radius=minor, location=loc, rotation=rotation, major_segments=24, minor_segments=8
    )
    obj = bpy.context.object
    bpy.ops.object.shade_smooth()
    return tag(obj, name, asset, mat)


def lathe(name, profile, mat, asset, segments=24) -> bpy.types.Object:
    """Surface of revolution about local +Y; profile entries are (y, rx, rz)."""
    vertices, faces = [], []
    textile = any(part in name for part in ("Torso", "Skirt", "WorkVest"))
    if textile:
        segments = max(32, segments)
    for y, rx, rz in profile:
        for j in range(segments):
            a = math.tau * j / segments
            fold = cloth_radius(a, y, profile[0][0], profile[-1][0]) if textile else 1
            vertices.append((rx * math.cos(a) * fold, y, rz * math.sin(a) * fold))
    for row in range(len(profile) - 1):
        for j in range(segments):
            a = row * segments + j
            b = row * segments + (j + 1) % segments
            faces.append((a, b, (row + 1) * segments + (j + 1) % segments, (row + 1) * segments + j))
    bottom, top = len(vertices), len(vertices) + 1
    vertices += [(0, profile[0][0], 0), (0, profile[-1][0], 0)]
    for j in range(segments):
        faces.append((bottom, (j + 1) % segments, j))
        start = (len(profile) - 1) * segments
        faces.append((top, start + j, start + (j + 1) % segments))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    for poly in obj.data.polygons:
        poly.use_smooth = True
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)


def add_elliptical_band(name: str, y_center: float, rx: float, rz: float,
                        height: float, thickness: float, mat, asset: str,
                        segments: int = 24) -> bpy.types.Object:
    """Elliptical band / ring conforming to anatomy with distinct width and depth."""
    hh = height * 0.5
    profile = [
        (y_center - hh, rx, rz),
        (y_center - hh * 0.5, rx + thickness, rz + thickness),
        (y_center + hh * 0.5, rx + thickness, rz + thickness),
        (y_center + hh, rx, rz),
    ]
    return lathe(name, profile, mat, asset, segments=segments)

def between(a, b, amount):
    return tuple(x + (y - x) * amount for x, y in zip(a, b))


def shaped_limb(name, joints, widths, depths, mat, asset, smooth=True) -> bpy.types.Object:
    """Continuous tapered limb with elliptical cross-sections matching the v3 humanoid anatomy."""
    sides = 12
    vertices = []
    for (x, y, z), width, depth in zip(joints, widths, depths):
        for i in range(sides):
            angle = math.tau * i / sides
            vertices.append((x + width * math.cos(angle), y, z + depth * math.sin(angle)))
    faces = []
    for row in range(len(joints) - 1):
        for i in range(sides):
            a, b = row * sides + i, row * sides + (i + 1) % sides
            faces.append((a, b, b + sides, a + sides))
    faces.extend((tuple(reversed(tuple(range(sides)))), tuple((len(joints) - 1) * sides + i for i in range(sides))))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    for poly in mesh.polygons:
        poly.use_smooth = smooth
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)


def head_surface(name: str, rings, mat, asset: str, s: float, opening: float = 0, thickness: float = 0) -> bpy.types.Object:
    """Stack elliptical contours for refined cranial/facial profile or headgear shell."""
    sides = 16
    angles = [opening + (math.tau - 2 * opening) * i / sides for i in range(sides + 1)] if opening else [math.tau * i / sides for i in range(sides)]
    vertices = []
    for y, rx, center, rz in rings:
        for angle in angles:
            x = rx * math.sin(angle)
            z = center + rz * math.cos(angle)
            vertices.append((x * s, y * s, z * s))
    count = len(angles)
    columns = count - 1 if opening else count
    faces = [(row * count + col, row * count + (col + 1) % count,
              (row + 1) * count + (col + 1) % count, (row + 1) * count + col)
             for row in range(len(rings) - 1) for col in range(columns)]
    if not opening:
        faces += [tuple(reversed(range(count))),
                  tuple((len(rings) - 1) * count + i for i in range(count))]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    if thickness > 0:
        mod = obj.modifiers.new("ShellThickness", "SOLIDIFY")
        mod.thickness = thickness * s
        base.apply_modifier(obj, mod)
    for poly in mesh.polygons:
        poly.use_smooth = True
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)


def tapered_limb(name, joints, radii, mat, asset) -> bpy.types.Object:
    """Continuous joint-to-joint volume; avoids the bead-and-pipe silhouette."""
    segments = 10
    vertices = [
        (x + width * math.cos(math.tau * i / segments), y,
         z + depth * math.sin(math.tau * i / segments))
        for (x, y, z), (width, depth) in zip(joints, radii)
        for i in range(segments)
    ]
    faces = [
        (row * segments + i, row * segments + (i + 1) % segments,
         (row + 1) * segments + (i + 1) % segments, (row + 1) * segments + i)
        for row in range(len(joints) - 1) for i in range(segments)
    ]
    faces.extend((tuple(reversed(range(segments))),
                  tuple((len(joints) - 1) * segments + i for i in range(segments))))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    for poly in mesh.polygons:
        poly.use_smooth = True
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)


def cloth_panel(name, rows, mat, asset, thickness=0.035) -> bpy.types.Object:
    """Folded cloth or planar metal plate with silhouette and clean bevels."""
    bsdf = mat.node_tree.nodes.get("Principled BSDF") if hasattr(mat, "node_tree") and mat.node_tree else None
    metal = bsdf.inputs["Metallic"].default_value >= .4 if bsdf else False
    if metal:
        vertices = [point for row in rows for point in row]
        faces = [(i * 3 + j, i * 3 + j + 1, (i + 1) * 3 + j + 1, (i + 1) * 3 + j)
                 for i in range(len(rows) - 1) for j in range(2)]
    else:
        vertices, faces = folded_panel(rows)
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    modifier = obj.modifiers.new("Cloth thickness", "SOLIDIFY")
    modifier.thickness = thickness
    base.apply_modifier(obj, modifier)
    for polygon in obj.data.polygons:
        polygon.use_smooth = not metal
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)



def tune_scene() -> None:
    """Roster tone: 12 player sheets in a contact grid need a darker key range than
    the Lich/Guard hero scene, and EEVEE Next ray tracing supplies contact occlusion."""
    scene = bpy.context.scene
    for light in scene.objects:
        if light.type == "LIGHT":
            light.data.energy *= 0.52
    scene.world.color = (0.004, 0.005, 0.010)
    eevee = scene.eevee
    if hasattr(eevee, "use_raytracing"):
        eevee.use_raytracing = True
    if hasattr(eevee, "use_shadows"):
        eevee.use_shadows = True
    scene.view_settings.exposure = 0.15



def add_rod(name: str, p0, p1, radius: float, mat, asset: str, sides: int = 8) -> bpy.types.Object:
    """Explicit cylinder between two points.

    Gear uses this instead of a bevelled curve: a curve's bevel radius does not
    survive the rig's rigid re-weighting reliably, and a thin blade comes out as
    a bead. Built from vertices, the radius is exactly what was asked for.
    """
    from mathutils import Vector

    a, b = Vector(p0), Vector(p1)
    axis = b - a
    if axis.length < 1e-6:
        raise ValueError(f"{name}: degenerate rod")
    up = Vector((0, 0, 1)) if abs(axis.normalized().z) < 0.9 else Vector((1, 0, 0))
    n1 = axis.cross(up).normalized()
    n2 = axis.cross(n1).normalized()
    vertices, faces = [], []
    for centre in (a, b):
        for k in range(sides):
            ang = math.tau * k / sides
            vertices.append(tuple(centre + n1 * (radius * math.cos(ang)) + n2 * (radius * math.sin(ang))))
    for k in range(sides):
        nxt = (k + 1) % sides
        faces.append((k, nxt, sides + nxt, sides + k))
    faces.append(tuple(range(sides - 1, -1, -1)))
    faces.append(tuple(range(sides, sides * 2)))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    for poly in obj.data.polygons:
        poly.use_smooth = True
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)


def add_box(name: str, lo, hi, mat, asset: str) -> bpy.types.Object:
    x0, y0, z0 = lo
    x1, y1, z1 = hi
    vertices = [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
                (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]
    faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    bevel = obj.modifiers.new("Forged edge", "BEVEL")
    bevel.width = min(.012, min(x1 - x0, y1 - y0, z1 - z0) * .16)
    bevel.segments = 2
    base.apply_modifier(obj, bevel)
    base.ensure_uv(obj)
    return tag(obj, name, asset, mat)



def side_label(side: int) -> str:
    """`-1` / `1` as `L` / `R`.

    A part named with the raw int (`Pauldron-1`) does not end in "L", so
    `bone_group` routes it to the RIGHT limb and the left-hand ornament rides the
    right arm. Every side-named part must use this.
    """
    return "L" if side < 0 else "R"


# --------------------------------------------------------------------------- body


def add_face(asset: str, mats: dict, s: float, y: float, z: float) -> None:
    """Paired almond eyes, refined brows and a sleek tapered nose bridge."""
    for side in (-1, 1):
        label = side_label(side)
        add_ico(f"{asset}_Eye{label}", (side * .088 * s, 2.60 * s, .315 * s),
                (.028 * s, .018 * s, .010 * s), mats["accent"], asset)
        add_ico(f"{asset}_Cheek{label}", (side * .145 * s, 2.50 * s, .285 * s),
                (.038 * s, .035 * s, .014 * s), mats["skin"], asset)
        add_ico(f"{asset}_Ear{label}", (side * .235 * s, 2.49 * s, .06 * s),
                (.032 * s, .065 * s, .045 * s), mats["skin"], asset)
        add_tube(f"{asset}_Brow{label}",
                 [(side * .030 * s, 2.665 * s, .320 * s),
                  (side * .095 * s, 2.675 * s, .310 * s),
                  (side * .155 * s, 2.650 * s, .275 * s)],
                 .013 * s, mats["cloth_shade"], asset)
    # Sleek, naturally tapered nose bridge
    shaped_limb(f"{asset}_Nose",
                [(0, 2.65 * s, .305 * s),
                 (0, 2.58 * s, .335 * s),
                 (0, 2.52 * s, .370 * s),
                 (0, 2.49 * s, .350 * s)],
                (.016 * s, .022 * s, .026 * s, .020 * s),
                (.016 * s, .023 * s, .024 * s, .016 * s),
                mats["skin"], asset)
    add_ico(f"{asset}_FaceChin", (0, 2.345 * s, .245 * s),
            (.085 * s, .042 * s, .045 * s), mats["skin"], asset)
    add_tube(f"{asset}_Mouth",
             [(-.055 * s, 2.415 * s, .285 * s),
              (0, 2.405 * s, .300 * s),
              (.055 * s, 2.415 * s, .285 * s)],
             .009 * s, mats["dark"], asset)


def add_head(asset: str, cfg: dict, mats: dict, b: dict, s: float) -> None:
    """Head + the class headgear. Rear shell ensures the facial read remains clear."""
    head = cfg["head"]
    r = b["head"]
    head_surface(f"{asset}_Face", [
        (2.28, .075 * r / .24, .12, .10 * r / .24),
        (2.35, .150 * r / .24, .10, .17 * r / .24),
        (2.48, .210 * r / .24, .08, .23 * r / .24),
        (2.61, .230 * r / .24, .06, .26 * r / .24),
        (2.74, .210 * r / .24, .05, .25 * r / .24),
        (2.83, .130 * r / .24, .04, .17 * r / .24),
        (2.87, .015 * r / .24, .035, .025),
    ], mats["skin"], asset, s)
    add_face(asset, mats, s, 2.52 * s, 0.20 * s)
    if head == "helm":
        head_surface(f"{asset}_HelmShell", [
            (2.55, .265, .015, .28),
            (2.72, .260, .020, .28),
            (2.88, .190, .015, .20),
            (2.95, .015, .015, .025),
        ], mats["metal"], asset, s)
        add_torus(f"{asset}_HelmBrow", (0, 2.68 * s, .07 * s), .32 * s, .024 * s, mats["metal"], asset, (math.pi / 2, 0, 0))
        shaped_limb(f"{asset}_NoseGuard",
                    [(0, 2.70 * s, .36 * s), (0, 2.54 * s, .39 * s), (0, 2.44 * s, .37 * s)],
                    (.020 * s, .024 * s, .018 * s), (.020 * s, .024 * s, .018 * s),
                    mats["accent"], asset)
        for side in (-1, 1):
            lbl = side_label(side)
            add_ico(f"{asset}_CheekGuard{lbl}", (side * .19 * s, 2.44 * s, .22 * s),
                    (.035 * s, .095 * s, .060 * s), mats["metal"], asset)
        add_tube(f"{asset}_HelmCrest", [(0, 2.72 * s, -.20 * s), (0, 2.96 * s, 0), (0, 2.82 * s, .24 * s)],
                 .025 * s, mats["accent"], asset)
    elif head == "hood":
        head_surface(f"{asset}_HoodShell", [
            (2.23, .29, -.025, .29),
            (2.47, .29, .015, .37),
            (2.66, .28, .025, .41),
            (2.78, .22, .055, .41),
            (2.90, .014, -.035, .06),
        ], mats["cloth"], asset, s, opening=1.05, thickness=.032)
        add_tube(f"{asset}_HoodPeak", [(0, 2.78 * s, -.16 * s), (0, 2.50 * s, -.22 * s), (0, 2.20 * s, -.25 * s)],
                 .035 * s, mats["cloth_shade"], asset)
    elif head == "cap":
        head_surface(f"{asset}_Cap", [
            (2.63, .248, .017, .27),
            (2.76, .245, .022, .275),
            (2.88, .160, .016, .195),
            (2.92, .015, .015, .025),
        ], mats["cloth_shade"], asset, s)
        add_torus(f"{asset}_CapRoll", (0, 2.74 * s, .08 * s), .255 * s, .036 * s, mats["cloth"], asset, (math.pi / 2, 0, 0))
    elif head == "mitre":
        for side in (-1, 1):
            lbl = side_label(side)
            box_verts = [
                (side * .04 * s, 2.76 * s, -.16 * s), (side * .22 * s, 2.76 * s, -.12 * s),
                (side * .20 * s, 2.76 * s, .14 * s), (side * .04 * s, 2.76 * s, .18 * s),
                (side * .14 * s, 3.22 * s, 0)
            ]
            mesh = bpy.data.meshes.new(f"{asset}_MitrePeak{lbl}")
            mesh.from_pydata(box_verts, [], [(0, 1, 4), (1, 2, 4), (2, 3, 4), (3, 0, 4)])
            mesh.update()
            obj = bpy.data.objects.new(f"{asset}_MitrePeak{lbl}", mesh)
            bpy.context.collection.objects.link(obj)
            mod = obj.modifiers.new("Solid", "SOLIDIFY"); mod.thickness = .025 * s; base.apply_modifier(obj, mod)
            tag(obj, f"{asset}_MitrePeak{lbl}", asset, mats["cloth"])
        add_torus(f"{asset}_MitreBand", (0, 2.74 * s, 0), .24 * s, .028 * s, mats["accent"], asset, (math.pi / 2, 0, 0))
        add_ico(f"{asset}_MitreGem", (0, 2.82 * s, .14 * s), (.045 * s, .045 * s, .030 * s), mats["glow"], asset)
    elif head == "horned":
        head_surface(f"{asset}_HoodShell", [
            (2.28, .29, -.02, .29),
            (2.50, .29, .02, .38),
            (2.70, .28, .03, .42),
            (2.82, .22, .05, .42),
            (2.92, .014, -.03, .06),
        ], mats["cloth_shade"], asset, s, opening=1.05, thickness=.032)
        for side in (-1, 1):
            lbl = side_label(side)
            shaped_limb(f"{asset}_Horn{lbl}", [
                (side * .20 * s, 2.75 * s, -.06 * s),
                (side * .34 * s, 2.96 * s, -.10 * s),
                (side * .42 * s, 3.16 * s, -.04 * s),
                (side * .36 * s, 3.30 * s, .06 * s)
            ], (.065 * s, .052 * s, .035 * s, .008 * s),
               (.065 * s, .050 * s, .032 * s, .008 * s),
               mats["bone"], asset, smooth=True)


def add_garment(asset: str, cfg: dict, mats: dict, b: dict, s: float) -> None:
    """Class garment tailored over the anatomical torso."""
    garment = cfg["garment"]
    sh, wa, dz = b["shoulder"], b["waist"], b["torso_depth"]
    if garment == "robe":
        lathe(f"{asset}_Skirt", [(0.08 * s, 0.49 * s, 0.42 * s), (0.30 * s, 0.46 * s, 0.39 * s),
                                (0.72 * s, 0.39 * s, 0.34 * s), (1.13 * s, 0.35 * s, 0.29 * s)], mats["cloth"], asset)
        cloth_panel(f"{asset}_RobeFront",
                    [[(-.22 * s, 1.15 * s, .28 * s), (0, 1.15 * s, .32 * s), (.22 * s, 1.15 * s, .28 * s)],
                     [(-.27 * s, .70 * s, .32 * s), (0, .68 * s, .40 * s), (.27 * s, .70 * s, .32 * s)],
                     [(-.34 * s, .12 * s, .36 * s), (0, .10 * s, .46 * s), (.34 * s, .12 * s, .36 * s)]],
                    mats["cloth_shade"], asset)
        add_tube(f"{asset}_Sash", [(-.31 * s, 1.18 * s, .10 * s), (0, 1.16 * s, .34 * s),
                                    (.31 * s, 1.18 * s, .10 * s)], .045 * s, mats["accent"], asset)
    elif garment == "cloak":
        cloth_panel(f"{asset}_Cloak",
                    [[(-sh * s, 2.12 * s, -.16 * s), (0, 2.14 * s, -.34 * s), (sh * s, 2.12 * s, -.16 * s)],
                     [(-.43 * s, 1.77 * s, -.19 * s), (0, 1.75 * s, -.47 * s), (.43 * s, 1.77 * s, -.19 * s)],
                     [(-.48 * s, 1.14 * s, -.21 * s), (0, 1.10 * s, -.51 * s), (.48 * s, 1.14 * s, -.21 * s)],
                     [(-.55 * s, .55 * s, -.19 * s), (0, .46 * s, -.57 * s), (.55 * s, .55 * s, -.19 * s)]],
                    mats["cloth_shade"], asset)
        add_tube(f"{asset}_CloakHem", [(-.55 * s, .55 * s, -.19 * s),
                                       (0, .46 * s, -.57 * s), (.55 * s, .55 * s, -.19 * s)],
                 .028 * s, mats["accent"], asset)
        add_ico(f"{asset}_Clasp", (0, 2.13 * s, .24 * s), (.075 * s, .075 * s, .05 * s), mats["accent"], asset)
    elif garment == "brigandine":
        # Contoured anatomical breastplates, laminar pauldrons, collar & rivets (no horizontal barrel rings!)
        for side in (-1, 1):
            lbl = side_label(side)
            rows = [
                [(side * .03 * s, 2.04 * s, .28 * s), (side * .17 * s, 2.04 * s, .28 * s), (side * .34 * s, 2.04 * s, .16 * s)],
                [(side * .04 * s, 1.70 * s, .30 * s), (side * .18 * s, 1.70 * s, .32 * s), (side * .34 * s, 1.70 * s, .21 * s)],
                [(side * .05 * s, 1.35 * s, .29 * s), (side * .18 * s, 1.35 * s, .31 * s), (side * .30 * s, 1.35 * s, .22 * s)],
            ]
            cloth_panel(f"{asset}_Breastplate{lbl}", rows, mats["metal"], asset)
            for height in (1.45, 1.85):
                add_ico(f"{asset}_Rivet{lbl}{int(height * 100)}", (side * .26 * s, height * s, .26 * s),
                        (.022 * s, .024 * s, .014 * s), mats["accent"], asset)
            rows_p = [
                [(side * .30 * s, 2.14 * s, -.10 * s), (side * .44 * s, 2.15 * s, 0), (side * .52 * s, 2.10 * s, .07 * s)],
                [(side * .36 * s, 1.98 * s, -.11 * s), (side * .49 * s, 2.00 * s, .01 * s), (side * .55 * s, 1.94 * s, .09 * s)],
                [(side * .37 * s, 1.82 * s, -.08 * s), (side * .49 * s, 1.83 * s, .02 * s), (side * .53 * s, 1.79 * s, .09 * s)],
            ]
            cloth_panel(f"{asset}_Pauldron{lbl}", rows_p, mats["metal"], asset)
        add_elliptical_band(f"{asset}_Collar", 2.16 * s, 0.21 * s, 0.17 * s, 0.06 * s, 0.024 * s, mats["metal"], asset)
    elif garment == "vest":
        for side in (-1, 1):
            lbl = side_label(side)
            rows_v = [
                [(side * .08 * s, 2.06 * s, .28 * s), (side * .23 * s, 2.08 * s, .26 * s), (side * .37 * s, 2.00 * s, .17 * s)],
                [(side * .10 * s, 1.55 * s, .32 * s), (side * .24 * s, 1.55 * s, .34 * s), (side * .40 * s, 1.51 * s, .18 * s)],
                [(side * .14 * s, 1.07 * s, .28 * s), (side * .28 * s, 1.03 * s, .29 * s), (side * .37 * s, 1.02 * s, .16 * s)],
            ]
            cloth_panel(f"{asset}_Vest{lbl}", rows_v, mats["cloth_shade"], asset)
            add_uv_sphere(f"{asset}_ShoulderPad{lbl}", (side * (sh + .03) * s, 2.05 * s, 0),
                          (.145 * s, .065 * s, .16 * s), mats["cloth"], asset)
        add_ico(f"{asset}_VestClasp", (0, 1.72 * s, (dz + .09) * s),
                (.08 * s, .07 * s, .035 * s), mats["accent"], asset)


def add_limbs(asset: str, b: dict, mats: dict, s: float) -> tuple[dict, dict]:
    sh = b["shoulder"]
    arms = {
        "L": ((-sh * s, 2.02 * s, 0),
              (-(sh + 0.16) * s, 1.72 * s, 0.02 * s),
              (-(sh + 0.10) * s, 1.38 * s, 0.08 * s),
              (-(sh + 0.10) * s, 1.22 * s, 0.10 * s)),
        "R": ((sh * s, 2.02 * s, 0),
              ((sh + 0.16) * s, 1.72 * s, 0.02 * s),
              ((sh + 0.10) * s, 1.40 * s, 0.08 * s),
              ((sh + 0.12) * s, 1.24 * s, 0.10 * s)),
    }
    for label, (shoulder, elbow, wrist, hand) in arms.items():
        joints = (
            between(shoulder, elbow, -0.16),
            between(shoulder, elbow, 0.43),
            between(shoulder, elbow, 0.79),
            elbow,
            between(elbow, wrist, 0.23),
            between(elbow, wrist, 0.65),
            between(wrist, hand, 0.80),
        )
        w = b["limb"]
        widths = (w * 1.15, w * 1.08, w * 0.90, w * 0.82, w * 0.80, w * 0.88, w * 0.65)
        shaped_limb(f"{asset}_ArmSleeve{label}", joints,
                    tuple(wid * s for wid in widths),
                    tuple(wid * 0.85 * s for wid in widths),
                    mats["cloth"], asset, smooth=True)
        # Contoured anatomical hand
        shaped_limb(f"{asset}_Hand{label}",
                    ((hand[0], hand[1] + .08 * s, hand[2]),
                     (hand[0], hand[1], hand[2] + .015 * s),
                     (hand[0], hand[1] - .085 * s, hand[2] + .027 * s)),
                    (.058 * s, .072 * s, .058 * s),
                    (.038 * s, .046 * s, .035 * s), mats["skin"], asset)
        sign = -1 if label == "L" else 1
        add_ico(f"{asset}_HandThumb{label}",
                (hand[0] - sign * .068 * s, hand[1] + .015 * s, hand[2] + .038 * s),
                (.030 * s, .058 * s, .032 * s), mats["skin"], asset)

    legs = {
        "L": ((-b["hip"] * s, 1.05 * s, 0),
              (-0.24 * s, 0.58 * s, 0.02 * s),
              (-0.25 * s, 0.10 * s, 0.05 * s)),
        "R": ((b["hip"] * s, 1.05 * s, 0),
              (0.24 * s, 0.58 * s, 0.02 * s),
              (0.25 * s, 0.10 * s, 0.05 * s)),
    }
    for label, (hip, knee, ankle) in legs.items():
        joints = (
            between(hip, knee, -0.08),
            between(hip, knee, 0.44),
            between(hip, knee, 0.83),
            knee,
            between(knee, ankle, 0.22),
            between(knee, ankle, 0.65),
            between(knee, ankle, 1.04),
        )
        widths = (0.170, 0.165, 0.132, 0.118, 0.104, 0.108, 0.088)
        shaped_limb(f"{asset}_Legging{label}", joints,
                    tuple(wid * s for wid in widths),
                    tuple(wid * 0.82 * s for wid in widths),
                    mats["cloth_shade"], asset, smooth=True)
        # Tailored leather boot with ankle cuff, heel and toe rise
        x = ankle[0]
        shaped_limb(f"{asset}_Boot{label}",
                    ((x, 0.19 * s, 0.03 * s),
                     (x, 0.08 * s, 0.09 * s),
                     (x, 0.035 * s, 0.11 * s)),
                    (0.105 * s, 0.145 * s, 0.145 * s),
                    (0.095 * s, 0.190 * s, 0.190 * s),
                    mats["leather"], asset)
    return arms, legs


def add_body(asset: str, cfg: dict, mats: dict, build_name: str) -> tuple[dict, dict]:
    b = BUILDS[build_name]
    s = b["scale"]
    lathe(f"{asset}_Torso", [
        (0.88 * s, (b["hip"] * 1.35) * s, (b["torso_depth"] * 0.90) * s),
        (1.15 * s, (b["waist"] * 1.15) * s, (b["torso_depth"] * 0.95) * s),
        (1.40 * s, (b["waist"] * 1.00) * s, (b["torso_depth"] * 0.88) * s),
        (1.75 * s, (b["chest"] * 0.95) * s, (b["torso_depth"] * 0.98) * s),
        (2.02 * s, (b["chest"] * 1.00) * s, (b["torso_depth"] * 1.00) * s),
        (2.18 * s, (b["shoulder"] * 0.70) * s, (b["torso_depth"] * 0.72) * s),
        (2.28 * s, 0.14 * s, 0.13 * s),
    ], mats["cloth"], asset, segments=16)
    add_tube(f"{asset}_TunicSeam", [(-.14 * s, 2.00 * s, b["torso_depth"] * 0.99 * s),
                                    (0, 1.88 * s, b["torso_depth"] * 1.03 * s),
                                    (.14 * s, 2.00 * s, b["torso_depth"] * 0.99 * s)],
             .018 * s, mats["cloth_shade"], asset)
    add_elliptical_band(f"{asset}_Belt", 1.15 * s, b["waist"] * 1.18 * s, b["torso_depth"] * 0.98 * s, 0.08 * s, 0.022 * s, mats["leather"], asset)
    add_ico(f"{asset}_BeltBuckle", (0, 1.15 * s, (b["torso_depth"] * 0.98 + 0.022) * s), (.045 * s, .035 * s, .018 * s), mats["accent"], asset)
    add_garment(asset, cfg, mats, b, s)
    add_head(asset, cfg, mats, b, s)
    return add_limbs(asset, b, mats, s)


# --------------------------------------------------------------------------- gear
#
# Every gear builder authors in SOCKET SPACE: origin at the socket, +Y up the
# limb. `attach` then places it into rig space and rigid-weights it to one bone,
# so the same builder yields both an equipped GLB and a standalone item model.


def _tier_metal(mats: dict, tier: int) -> tuple:
    """Each rung has a distinct material, not subpixel rivet-count changes."""
    return (
        (mats["leather"], mats["iron"]),
        (mats["iron"], mats["leather"]),
        (mats["steel"], mats["iron"]),
        (mats["gold"], mats["steel"]),
    )[tier]


def sword(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Straight service sword. Tier drives length, guard and inlay."""
    body, trim = _tier_metal(mats, tier)
    length = (.56 + .14 * tier) * s
    half = (.028 + .008 * tier) * s
    add_rod(f"{asset}_G{socket}Grip", (0, -0.10 * s, 0), (0, 0.03 * s, 0), 0.026 * s, mats["dark"], asset)
    add_ico(f"{asset}_G{socket}Pommel", (0, -0.115 * s, 0), (0.042 * s, 0.032 * s, 0.042 * s), trim, asset)
    guard = (0.085 + 0.018 * tier) * s
    add_rod(f"{asset}_G{socket}Guard", (-guard, 0.035 * s, 0), (guard, 0.035 * s, 0), 0.017 * s, trim, asset)
    if tier >= 3:
        for side in (-1, 1):
            add_ico(f"{asset}_G{socket}Wing{side_label(side)}", (side * (guard + 0.018 * s), 0.035 * s, 0), (0.05 * s, 0.026 * s, 0.042 * s), trim, asset)
    add_box(f"{asset}_G{socket}Blade", (-half, .05 * s, -half * .55),
            (half, .05 * s + length, half * .55), body, asset)
    if tier >= 2:
        add_box(f"{asset}_G{socket}Fuller", (-half * .22, .09 * s, half * .56),
                (half * .22, length - .02 * s, half * .62), mats["dark"], asset)
        add_ico(f"{asset}_G{socket}Stone", (0, .035 * s, half * .9),
                (.036 * s, .036 * s, .02 * s), trim, asset)
    if tier >= 3:
        add_ico(f"{asset}_G{socket}Tip", (0, (.05 * s + length), 0),
                (.045 * s, .11 * s, .018 * s), mats["steel"], asset)
    return []


def gravedigger(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Long two-handed shaft, mattock head and spade. Gravebound's kit."""
    body, trim = _tier_metal(mats, tier)
    reach = (.30 + .10 * tier) * s
    add_rod(f"{asset}_G{socket}Shaft", (0, -.30 * s, 0), (0, reach, 0), .034 * s, mats["leather"], asset)
    add_ico(f"{asset}_G{socket}Ferrule", (0, reach, 0), (.052 * s, .07 * s, .05 * s), trim, asset)
    add_rod(f"{asset}_G{socket}Head", (-.19 * s, reach, 0), ((.19 + .065 * tier) * s, reach, 0),
            (.027 + .005 * tier) * s, body, asset)
    add_box(f"{asset}_G{socket}Bit", ((.14 + .06 * tier) * s, (reach / s - .07) * s, -.035 * s),
            ((.24 + .08 * tier) * s, (reach / s + .09) * s, .035 * s), body, asset)
    add_box(f"{asset}_G{socket}Spade", (-.07 * s, .14 * s, -.02 * s),
            (.07 * s, (.28 + .04 * tier) * s, .02 * s), trim, asset)
    for i in range(2 + tier):
        add_torus(f"{asset}_G{socket}Band{i}", (0, (-.10 + .16 * i) * s, 0),
                  .034 * s, .010 * s, trim, asset, (math.pi / 2, 0, 0))
    return []


def gutterblade(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Curved saltmarsh sabre with a knuckle bow. Redwake's kit."""
    body, trim = _tier_metal(mats, tier)
    add_rod(f"{asset}_G{socket}Grip", (0, -0.10 * s, 0), (0, 0.04 * s, 0), 0.024 * s, mats["dark"], asset)
    add_ico(f"{asset}_G{socket}Pommel", (0, -0.115 * s, 0), (0.04 * s, 0.03 * s, 0.04 * s), trim, asset)
    add_rod(f"{asset}_G{socket}Knuckle", (0.045 * s, 0.04 * s, 0), (0.065 * s, 0.12 * s, 0), 0.012 * s, trim, asset)
    for k in range(4 + tier):
        y0 = (.05 + .12 * k) * s
        y1 = (.05 + .12 * (k + 1)) * s
        add_rod(f"{asset}_G{socket}Blade{k}", (.047 * k * s, y0, 0),
                (.047 * (k + 1) * s, y1, 0), (.029 + .004 * tier - .002 * k) * s, body, asset)
    if tier >= 2:
        add_rod(f"{asset}_G{socket}Edge", (.05 * s, .18 * s, .03 * s),
                ((.25 + .045 * tier) * s, (.54 + .12 * tier) * s, .03 * s),
                .012 * s, trim, asset)
    return []


def oathrod(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Road warden's rod: shaft, rune ring, oath-cord. Waysworn's kit."""
    body, trim = _tier_metal(mats, tier)
    reach = (.47 + .10 * tier) * s
    add_rod(f"{asset}_G{socket}Shaft", (0, -.32 * s, 0), (0, reach, 0),
            (.028 + .004 * tier) * s, mats["leather"], asset)
    add_torus(f"{asset}_G{socket}Knot", (0, reach, 0), (.07 + .02 * tier) * s,
              .02 * s, trim, asset, (math.pi / 2, 0, 0))
    add_ico(f"{asset}_G{socket}Head", (0, reach + .10 * s, 0),
            ((.06 + .015 * tier) * s, .09 * s, .065 * s), body, asset)
    add_torus(f"{asset}_G{socket}Ring", (0, reach + .10 * s, 0),
              (.12 + .033 * tier) * s, .022 * s, trim, asset)
    for k in range(1 + tier):
        ang = k * 2.1
        add_ico(f"{asset}_G{socket}Rune{k}",
                ((.12 + .03 * tier) * s * math.cos(ang), reach + .10 * s,
                 (.12 + .03 * tier) * s * math.sin(ang)), (.032 * s,) * 3, mats["glow"], asset)
    add_rod(f"{asset}_G{socket}Cord", (.08 * s, reach, 0), (.14 * s, .02 * s, .03 * s),
            .012 * s, mats["accent"], asset)
    return []


def sigilfocus(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Seal-script focus: staff, floating stone, two rings. SigilSworn's kit."""
    body, trim = _tier_metal(mats, tier)
    reach = (.44 + .08 * tier) * s
    add_rod(f"{asset}_G{socket}Shaft", (0, -.34 * s, 0), (0, reach, 0),
            (.028 + .003 * tier) * s, mats["leather"], asset)
    add_ico(f"{asset}_G{socket}Focus", (0, reach + .13 * s, 0),
            ((.08 + .02 * tier) * s, (.10 + .025 * tier) * s, (.08 + .02 * tier) * s),
            mats["glow"], asset)
    for k in range(1 + tier):
        add_torus(f"{asset}_G{socket}Ring{k}", (0, reach + .13 * s, 0),
                  (.13 + .024 * k + .012 * tier) * s, .018 * s, trim, asset,
                  (.6 * k, .4 * k, 0))
    add_ico(f"{asset}_G{socket}Claw", (0, reach + .27 * s, 0),
            (.055 * s, .07 * s, .05 * s), body, asset)
    return []


def fenward(asset: str, tier: int, mats: dict, s: float = 1.0, socket: str = "W") -> list:
    """Hollow-folk glaive: a fen-leaf blade on a short haft. Fensworn's kit."""
    body, trim = _tier_metal(mats, tier)
    reach = (.72 + .13 * tier) * s
    add_rod(f"{asset}_G{socket}Haft", (0, -.24 * s, 0), (0, .24 * s, 0),
            .035 * s, mats["leather"], asset)
    add_ico(f"{asset}_G{socket}Socket", (0, .24 * s, 0),
            (.055 * s, .065 * s, .055 * s), trim, asset)
    add_rod(f"{asset}_G{socket}Blade", (0, .26 * s, 0), (.01 * s, reach, .02 * s),
            (.038 + .007 * tier) * s, body, asset)
    add_ico(f"{asset}_G{socket}Leaf", (.055 * s, (reach / s - .17) * s, .02 * s),
            ((.09 + .025 * tier) * s, .22 * s, .036 * s), body, asset)
    for k in range(1 + tier):
        add_ico(f"{asset}_G{socket}Vein{k}", (.07 * s, (.34 + .10 * k) * s, .06 * s),
                (.032 * s, .05 * s, .02 * s), trim, asset)
    return []


WEAPONS = {
    "sword": sword, "gravedigger": gravedigger, "gutterblade": gutterblade,
    "oathrod": oathrod, "sigilfocus": sigilfocus, "fenward": fenward,
}

def shield(asset: str, tier: int, mats: dict, s: float) -> None:
    """Keepwarden's off-hand shield, scaled alongside the sword tier."""
    body, trim = _tier_metal(mats, tier)
    add_ico(f"{asset}_GSShield", (0, -.04 * s, 0),
            ((.18 + .030 * tier) * s, (.25 + .040 * tier) * s, .065 * s),
            body, asset)
    add_ico(f"{asset}_GSBoss", (0, 0, .07 * s),
            ((.060 + .015 * tier) * s, (.080 + .012 * tier) * s, .030 * s),
            trim, asset)
    if tier >= 2:
        add_rod(f"{asset}_GSSpine", (0, -.18 * s, .07 * s),
                (0, .18 * s, .07 * s), .028 * s, trim, asset)


def cuirass(asset: str, tier: int, mats: dict, b: dict, s: float, socket: str = "C") -> list:
    """Leather vest -> iron brigandine -> steel harness -> crowned gold plate."""
    body, trim = _tier_metal(mats, tier)
    sh = b["shoulder"]
    width = (.01, .035, .050, .065)[tier]
    profile = [
        (-.08 * s, (b["waist"] + .06 + width) * s, (.26 + width * .5) * s),
        (.14 * s, (b["waist"] + .04 + width) * s, (.27 + width * .5) * s),
        (.38 * s, (sh - .04 + width) * s, (.29 + width * .5) * s),
        (.68 * s, (sh + .01 + width) * s, (.30 + width * .5) * s),
        (.84 * s, (sh - .03 + width) * s, (.28 + width * .5) * s),
        (.96 * s, .23 * s, .20 * s),
    ]
    lathe(f"{asset}_G{socket}Plate", profile, body, asset)
    add_elliptical_band(f"{asset}_G{socket}CollarBand", .98 * s, (.23 + width * .30) * s, (.20 + width * .25) * s,
                        .06 * s, (.018 + .004 * tier) * s, trim, asset)
    for side in (-1, 1):
        label = side_label(side)
        add_uv_sphere(f"{asset}_G{socket}Pauldron{label}",
                      (side * (sh + .05 + width * .3) * s, .84 * s, 0),
                      ((.10 + .026 * tier) * s, (.042 + .012 * tier) * s,
                       (.11 + .022 * tier) * s), trim if tier == 0 else body, asset)
        if tier >= 2:
            add_rod(f"{asset}_G{socket}Lame{label}",
                    (side * (sh + .03) * s, .70 * s, .12 * s),
                    (side * (sh + .11) * s, .59 * s, .11 * s),
                    .024 * s, trim, asset)
        if tier >= 3:
            add_ico(f"{asset}_G{socket}Fin{label}",
                    (side * (sh + .18 + width * .4) * s, .96 * s, -.02 * s),
                    (.055 * s, .12 * s, .065 * s), body, asset)
    if tier >= 1:
        add_elliptical_band(f"{asset}_G{socket}Belt", .06 * s, (b["waist"] + .05 + width) * s,
                            (.26 + width * .5) * s, .06 * s, .018 * s, trim, asset)
    if tier >= 1:
        for row in range(2):
            yy = -.08 - row * .10
            lathe(f"{asset}_G{socket}Fauld{row}",
                  [(yy * s, (b["waist"] + .07 + width + row * .016) * s, (.27 + width * .5) * s),
                   ((yy - .07) * s, (b["waist"] + .09 + width + row * .016) * s, (.29 + width * .5) * s)],
                  body, asset)
        for side in (-1, 1):
            add_rod(f"{asset}_G{socket}Rib{side_label(side)}",
                    (side * .19 * s, .18 * s, (.29 + width * .5) * s),
                    (side * .24 * s, .68 * s, (.28 + width * .5) * s),
                    .016 * s, trim, asset)
    if tier >= 3:
        add_ico(f"{asset}_G{socket}Heart", (0, .62 * s, (.31 + width * .5) * s),
                (.08 * s, .11 * s, .032 * s), mats["glow"], asset)
        add_rod(f"{asset}_G{socket}Sunray", (-.15 * s, .38 * s, (.31 + width * .5) * s),
                (.15 * s, .80 * s, (.29 + width * .5) * s), .013 * s, trim, asset)
    return []


def build_relic(name: str, asset: str, mats: dict, s: float, socket: str) -> None:
    """Named chest emblems, raised in front of even the broadest plate.

    Their primary shapes span 8-14 final sprite pixels; narrow chains and
    rivets cannot communicate an equipped boss drop at game zoom.
    """
    n = f"{asset}_G{socket}"
    add_rod(f"{n}ChainL", (-.19 * s, .21 * s, -.05 * s),
            (-.07 * s, -.05 * s, 0), .018 * s, mats["gold"], asset)
    add_rod(f"{n}ChainR", (.19 * s, .21 * s, -.05 * s),
            (.07 * s, -.05 * s, 0), .018 * s, mats["gold"], asset)
    if name == "Rallybreaker":
        add_ico(f"{n}Warhorn", (0, -.10 * s, 0), (.15 * s, .16 * s, .09 * s), mats["gold"], asset)
        add_rod(f"{n}HornLeft", (-.12 * s, -.12 * s, 0), (-.25 * s, .11 * s, 0),
                .045 * s, mats["bone"], asset)
        add_rod(f"{n}HornRight", (.12 * s, -.12 * s, 0), (.25 * s, .11 * s, 0),
                .045 * s, mats["bone"], asset)
    elif name == "Fangmantle":
        for side in (-1, 1):
            add_ico(f"{n}Fang{side_label(side)}", (side * .11 * s, -.12 * s, 0),
                    (.085 * s, .22 * s, .08 * s), mats["bone"], asset)
        add_ico(f"{n}Blood", (0, -.04 * s, .06 * s), (.09 * s, .12 * s, .04 * s),
                mats["accent"], asset)
    elif name == "Graveglass":
        add_torus(f"{n}SilverRim", (0, -.11 * s, 0), .20 * s, .043 * s,
                  mats["steel"], asset)
        add_uv_sphere(f"{n}Glass", (0, -.11 * s, .02 * s),
                      (.155 * s, .155 * s, .07 * s), mats["glow"], asset)
    elif name == "Saltcrown":
        add_rod(f"{n}CrownBase", (-.23 * s, -.15 * s, 0), (.23 * s, -.15 * s, 0),
                .05 * s, mats["steel"], asset)
        for side in (-1, 0, 1):
            add_ico(f"{n}SaltSpire{side + 1}", (side * .17 * s, .025 * s, 0),
                    (.06 * s, (.20 if side == 0 else .15) * s, .06 * s), mats["bone"], asset)
    elif name == "Stoneheart":
        add_ico(f"{n}Granite", (0, -.10 * s, 0), (.21 * s, .22 * s, .12 * s),
                mats["iron"], asset)
        add_rod(f"{n}Fault", (-.09 * s, -.21 * s, .115 * s),
                (.10 * s, .07 * s, .115 * s), .032 * s, mats["gold"], asset)
    elif name == "GnawboneCrown":
        add_rod(f"{n}Jaw", (-.22 * s, -.18 * s, 0), (.22 * s, -.18 * s, 0),
                .05 * s, mats["bone"], asset)
        for side in (-1, 0, 1):
            add_ico(f"{n}Tooth{side + 1}", (side * .15 * s, .01 * s, 0),
                    (.065 * s, (.21 if side == 0 else .14) * s, .075 * s),
                    mats["bone"], asset)
    elif name == "TollcoinCharm":
        add_torus(f"{n}CoinEdge", (0, -.11 * s, 0), .20 * s, .045 * s,
                  mats["gold"], asset)
        add_uv_sphere(f"{n}CoinFace", (0, -.11 * s, .02 * s),
                      (.155 * s, .155 * s, .05 * s), mats["gold"], asset)
        add_rod(f"{n}Tally", (-.05 * s, -.23 * s, .072 * s),
                (.07 * s, .01 * s, .072 * s), .035 * s, mats["dark"], asset)
    elif name == "WisplightLantern":
        add_box(f"{n}Lantern", (-.17 * s, -.31 * s, -.05 * s),
                (.17 * s, .04 * s, .09 * s), mats["iron"], asset)
        add_uv_sphere(f"{n}Wisp", (0, -.13 * s, .11 * s),
                      (.13 * s, .17 * s, .06 * s), mats["glow"], asset)
        add_torus(f"{n}Bail", (0, .09 * s, 0), .13 * s, .035 * s,
                  mats["gold"], asset)
    elif name == "Hartshorn":
        add_ico(f"{n}StagSkull", (0, -.16 * s, 0), (.13 * s, .15 * s, .08 * s),
                mats["bone"], asset)
        for side in (-1, 1):
            label = side_label(side)
            add_tube(f"{n}Antler{label}",
                     [(side * .08 * s, -.06 * s, 0),
                      (side * .22 * s, .18 * s, 0),
                      (side * .32 * s, .28 * s, 0)], .047 * s, mats["bone"], asset)
    else:
        raise ValueError(f"unknown boss relic: {name}")




# --------------------------------------------------------------------------- rig

# Object-name prefix -> deform bone. Gear is rigid-weighted, so an unmapped part
# silently rides the spine instead of following the hand.
HEAD_PARTS = ("Face", "Eye", "Cheek", "Ear", "Brow", "Nose", "Mouth", "Hood", "Helm",
              "Cap", "Mantle", "Horn", "Mitre", "Chin", "NoseGuard", "CheekGuard", "HelmCrest")
ARM_PARTS = ("UpperArm", "Elbow", "Pauldron", "ShoulderPad")
HAND_PARTS = ("Forearm", "Hand")
LEG_PARTS = ("Thigh", "Shin", "Boot", "Knee")




SOCKET_BONE = {"W": "weapon.R", "S": "forearm.L", "C": "chest", "R": "relic"}


def bone_group(asset: str, name: str) -> str:
    """Route a part to its deform bone. Gear names carry a socket token
    (`<asset>_GW...` = weapon, `_GC...` = chest, `_GR...` = relic) so a part can
    never be mistaken for a body part of the same name."""
    gear = name.startswith(f"{asset}_G")
    if gear:
        return SOCKET_BONE[name[len(asset) + 2]]

    def starts(*prefixes: str) -> bool:
        return any(name.startswith(f"{asset}_{prefix}") for prefix in prefixes)

    side = "L" if name.endswith("L") else "R"
    if starts("Boot"):
        return f"shin.{side}"
    if starts("Hand"):
        return f"hand.{side}"
    if starts("ArmSleeve"):
        return f"upper_arm.{side}"
    if starts("Legging"):
        return f"thigh.{side}"
    if starts(*ARM_PARTS):
        return f"upper_arm.{side}"
    if starts(*HAND_PARTS):
        return f"forearm.{side}"
    if starts(*LEG_PARTS):
        return f"thigh.{side}"
    if starts(*HEAD_PARTS):
        return "head"
    return "spine"


def socket_origins(b: dict, arms: dict) -> dict:
    """Where each equipment socket sits in rig space, in the authored body frame.

    The hand socket is the right hand itself, so a blade lands in the palm for
    either build instead of at a hard-coded height.
    """
    s = b["scale"]
    hand = arms["R"][3]
    offhand = arms["L"][2]
    return {
        "weapon.R": (hand[0], hand[1], hand[2]),
        "forearm.L": (offhand[0] - .09 * s, offhand[1], offhand[2] + .20 * s),
        "chest": (0.0, 1.25 * s, 0.0),
        # A pendant on the chest, not a charm at the hip: the plate and cloak both
        # occlude the hip, and a relic the player cannot see is not equipped art.
        "relic": (0.0, 1.85 * s, 0.58 * s),
    }


def add_markers(asset: str, b: dict, s: float) -> None:
    for name, kind, loc in ((f"{asset}_front", "front", (0, 2.4, 1.4)), (f"{asset}_back", "back", (0, 2.4, -1.4)), (f"{asset}_pivot", "pivot", (0, 0, 0))):
        obj = bpy.data.objects.new(name, None)
        bpy.context.collection.objects.link(obj)
        obj.location = loc
        obj.empty_display_type = "PLAIN_AXES"
        obj.empty_display_size = 0.2
        obj["asset3d_id"] = asset
        obj["asset3d_marker"] = kind


def add_rig(asset: str, arms: dict, legs: dict, b: dict) -> bpy.types.Object:
    parts = [o for o in bpy.context.scene.objects if o.type == "MESH" and o.get("asset3d_id") == asset]
    data = bpy.data.armatures.new(f"{asset}_RigData")
    rig = bpy.data.objects.new(f"{asset}_Rig", data)
    bpy.context.collection.objects.link(rig)
    rig["asset3d_id"] = asset
    rig["asset3d_rig"] = True
    rig.rotation_euler = BASE_ROT
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = data.edit_bones
    s = b["scale"]
    root = bones.new("root"); root.head, root.tail = (0, 0, 0), (0, 0.8 * s, 0)
    spine = bones.new("spine"); spine.head, spine.tail = (0, 0.8 * s, 0), (0, 2.0 * s, 0); spine.parent = root
    neck = bones.new("neck"); neck.head, neck.tail = (0, 2.0 * s, 0), (0, 2.35 * s, 0); neck.parent = spine
    head = bones.new("head"); head.head, head.tail = (0, 2.35 * s, 0), (0, 2.9 * s, 0); head.parent = neck
    # Chest and relic sockets follow the spine so worn plates and emblems animate.
    chest = bones.new("chest"); chest.head, chest.tail = (0, 1.55 * s, 0), (0, 2.2 * s, 0); chest.parent = spine
    relic = bones.new("relic"); relic.head, relic.tail = (0.0, 1.88 * s, 0.20 * s), (0.0, 2.20 * s, 0.20 * s); relic.parent = spine
    for label, pts in arms.items():
        upper = bones.new(f"upper_arm.{label}"); upper.head, upper.tail = pts[0], pts[1]; upper.parent = spine
        fore = bones.new(f"forearm.{label}"); fore.head, fore.tail = pts[1], pts[2]; fore.parent = upper; fore.use_connect = True
        hand = bones.new(f"hand.{label}"); hand.head, hand.tail = pts[2], pts[3]; hand.parent = fore; hand.use_connect = True
    for label, pts in legs.items():
        thigh = bones.new(f"thigh.{label}"); thigh.head, thigh.tail = pts[0], pts[1]; thigh.parent = root
        shin = bones.new(f"shin.{label}"); shin.head, shin.tail = pts[1], pts[2]; shin.parent = thigh; shin.use_connect = True
    hand = arms["R"][3]
    weapon = bones.new("weapon.R"); weapon.head, weapon.tail = hand, (hand[0], hand[1] + 0.66 * s, hand[2]); weapon.parent = bones["hand.R"]
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    for obj in parts:
        obj.matrix_parent_inverse = Matrix.Identity(4)
    for obj in parts:
        for group in obj.vertex_groups:
            group.remove(list(range(len(obj.data.vertices))))
        name = obj.name
        if name.startswith(f"{asset}_ArmSleeve") or name.startswith(f"{asset}_Legging"):
            label = name[-1]
            arm = name.startswith(f"{asset}_ArmSleeve")
            joint = (arms if arm else legs)[label][1][1]
            upper_name, lower_name = ("upper_arm", "forearm") if arm else ("thigh", "shin")
            upper = obj.vertex_groups.get(f"{upper_name}.{label}") or obj.vertex_groups.new(name=f"{upper_name}.{label}")
            lower = obj.vertex_groups.get(f"{lower_name}.{label}") or obj.vertex_groups.new(name=f"{lower_name}.{label}")
            blend = 0.09 * b["scale"]
            for vertex in obj.data.vertices:
                upper_weight = max(0.0, min(1.0, (vertex.co.y - joint + blend) / (2 * blend)))
                if upper_weight > 0:
                    upper.add([vertex.index], upper_weight, "REPLACE")
                if upper_weight < 1:
                    lower.add([vertex.index], 1 - upper_weight, "REPLACE")
        else:
            bname = bone_group(asset, name)
            group = obj.vertex_groups.get(bname) or obj.vertex_groups.new(name=bname)
            group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
    for marker in (bpy.data.objects.get(f"{asset}_front"), bpy.data.objects.get(f"{asset}_back"), bpy.data.objects.get(f"{asset}_pivot")):
        if marker is not None:
            marker.parent = rig
            marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig
    for marker in (bpy.data.objects.get(f"{asset}_front"), bpy.data.objects.get(f"{asset}_back"), bpy.data.objects.get(f"{asset}_pivot")):
        if marker is not None:
            marker.parent = rig
            marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig


# --------------------------------------------------------------------------- pose


def add_turntable(rig: bpy.types.Object) -> bpy.types.Object:
    turntable = bpy.data.objects.new("Player_Turntable", None)
    bpy.context.collection.objects.link(turntable)
    turntable.empty_display_type = "PLAIN_AXES"
    rig.parent = turntable
    rig.matrix_parent_inverse = Matrix.Identity(4)
    return turntable


def reset_pose(rig: bpy.types.Object) -> None:
    for bone in rig.pose.bones:
        bone.location = (0, 0, 0)
        bone.rotation_mode = "XYZ"
        bone.rotation_euler = (0, 0, 0)
        bone.scale = (1, 1, 1)


def bone_rotate(bone, axis, angle: float) -> None:
    """Rotate a pose bone about an ARMATURE-SPACE axis through its own head.

    A bone's local Euler axes depend on its roll, so `rotation_euler.z = 0.16`
    on an upright neck bone swings the head sideways instead of nodding it. Working
    in armature space makes the intent explicit: twist is about the up axis (+Y),
    a nod or tilt is about the forward axis (+Z).
    """
    from mathutils import Matrix, Vector

    rest = bone.bone.matrix_local
    pivot = rest.to_translation()
    rotation = (
        Matrix.Translation(pivot)
        @ Matrix.Rotation(angle, 4, Vector(axis).normalized())
        @ Matrix.Translation(-pivot)
    )
    bone.matrix_basis = rest.inverted() @ rotation @ rest


def apply_pose(rig: bpy.types.Object, cfg: dict, anim: str, frame: int) -> None:
    """Pose the rig for one frame of an animation track."""
    reset_pose(rig)
    s = BUILDS[BUILD]["scale"]
    bob = forward = spine_roll = neck_roll = head_roll = 0.0
    weapon = (0.0, 0.0, 0.0)
    if anim == "idle":
        breath = 1.0 if frame == 0 else -1.0
        bob, spine_roll, neck_roll = 0.04 * breath, 0.05 * breath, 0.07 * breath
        arms = {"upper_arm.L": (0.05, 0.0, -0.10 + 0.05 * breath), "forearm.L": (0.12, 0.0, 0.12),
                "upper_arm.R": (0.0, 0.0, 0.10 + 0.05 * breath), "forearm.R": (0.12, 0.0, -0.14)}
        legs = {"thigh.L": 0.0, "shin.L": 0.0, "thigh.R": 0.0, "shin.R": 0.0}
    elif anim == "walk":
        sign = 1 if frame == 0 else -1
        bob, spine_roll = 0.05 * sign, 0.03 * sign
        arms = {"upper_arm.L": (0.0, 0.0, 0.30 * sign), "forearm.L": (0.18, 0.0, -0.18 * sign),
                "upper_arm.R": (0.0, 0.0, -0.26 * sign), "forearm.R": (0.18, 0.0, -0.12)}
        legs = {"thigh.L": 0.34 * sign, "shin.L": -0.20 * sign, "thigh.R": -0.34 * sign, "shin.R": 0.20 * sign}
    elif anim == "attack":
        if frame == 0:
            spine_roll, neck_roll, forward = 0.22, 0.16, -0.10 * s
            arms = {"upper_arm.L": (-0.55, 0.0, -0.62), "forearm.L": (0.42, 0.0, 0.52),
                    "upper_arm.R": (-0.55, 0.0, 0.66), "forearm.R": (0.42, 0.0, -0.58)}
            weapon = (0.0, 0.0, 0.85)
        elif frame == 1:
            spine_roll, neck_roll, forward = -0.26, -0.20, 0.20 * s
            arms = {"upper_arm.L": (0.34, 0.0, -0.16), "forearm.L": (0.10, 0.0, 0.14),
                    "upper_arm.R": (0.34, 0.0, 0.20), "forearm.R": (0.10, 0.0, -0.18)}
            weapon = (0.0, 0.0, -0.95)
        elif frame == 2:
            spine_roll, neck_roll, forward = 0.20, 0.14, 0.05 * s
            arms = {"upper_arm.L": (-0.40, 0.0, -0.46), "forearm.L": (0.30, 0.0, 0.40),
                    "upper_arm.R": (-0.40, 0.0, 0.50), "forearm.R": (0.30, 0.0, -0.44)}
            weapon = (0.0, 0.0, 0.45)
        else:
            spine_roll, neck_roll, forward = 0.06, 0.04, 0.0
            arms = {"upper_arm.L": (0.10, 0.0, -0.14), "forearm.L": (0.14, 0.0, 0.16),
                    "upper_arm.R": (0.10, 0.0, 0.16), "forearm.R": (0.14, 0.0, -0.18)}
            weapon = (0.0, 0.0, -0.10)
        legs = {"thigh.L": 0.0, "shin.L": 0.0, "thigh.R": 0.0, "shin.R": 0.0}
    else:
        settle = 1.0 if frame == 0 else -1.0
        spine_roll, neck_roll, head_roll, forward = 0.24 * settle, 0.12, -0.20 * settle, -0.12 * s
        arms = {"upper_arm.L": (0.0, 0.0, 0.52 * settle), "forearm.L": (0.30, 0.0, 0.36),
                "upper_arm.R": (0.0, 0.0, -0.48 * settle), "forearm.R": (0.30, 0.0, -0.36)}
        weapon = (0.0, 0.0, 0.30 * settle)
        legs = {"thigh.L": 0.16 * settle, "shin.L": -0.12, "thigh.R": -0.16 * settle, "shin.R": 0.12}
    rig.location = (0, bob * 0.35 + forward * 0.15, forward)
    rig.rotation_euler = BASE_ROT
    for bone_name, angle, axis in (("spine", spine_roll, (0, 1, 0)), ("neck", neck_roll, (0, 0, 1)), ("head", head_roll, (0, 0, 1))):
        if bone_name in rig.pose.bones and angle:
            bone_rotate(rig.pose.bones[bone_name], axis, angle)
    for name, angles in arms.items():
        if name in rig.pose.bones:
            rig.pose.bones[name].rotation_euler = angles
    for name, angle in legs.items():
        if name in rig.pose.bones:
            rig.pose.bones[name].rotation_euler.z = angle
    if "weapon.R" in rig.pose.bones:
        rig.pose.bones["weapon.R"].rotation_euler = weapon
    bpy.context.view_layer.update()


def render_counts(cfg: dict) -> dict[str, int]:
    counts = {name: TRACK_LENGTH[name] for name in ("idle", "walk", "hit")}
    counts["attack"] = max(1, min(len(cfg["clips"]["attack"]), TRACK_LENGTH["attack"]))
    return counts


def clip_frames(action: str, index: int, count: int) -> list[int]:
    if action == "attack" and count > 1:
        return [index, (index + 1) % count]
    length = TRACK_LENGTH[action]
    return [index % length, (index + 1) % length]


def build_action_library(rig: bpy.types.Object, cfg: dict) -> None:
    profile = action_profile(cfg)
    rig.animation_data_create()
    clips = [(name, action, index, len(names)) for action, names in profile["clips"].items() for index, name in enumerate(names)]
    default_action = None
    for name, anim, index, count in clips:
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        if name == profile["default_action"]:
            default_action = action
        poses = []
        for key_frame, source in enumerate(clip_frames(anim, index, count)):
            apply_pose(rig, cfg, anim, source)
            poses.append(tuple(tuple(round(v, 4) for v in bone.rotation_euler) for bone in rig.pose.bones))
            for bone in rig.pose.bones:
                bone.keyframe_insert(data_path="rotation_euler", frame=key_frame, group=bone.name)
            rig.keyframe_insert(data_path="location", frame=key_frame, group="Root")
            rig.keyframe_insert(data_path="rotation_euler", frame=key_frame, group="Root")
        if len(poses) > 1 and poses[0] == poses[-1]:
            raise SystemExit(f"clip {name} ({anim} beat {index} of {count}) is a frozen pose")
    rig.animation_data.action = default_action
    bpy.context.scene.frame_set(0)


def action_profile(cfg: dict) -> dict:
    return {
        "id": f"player_{CLASS_NAME.lower()}_{cfg['weapon']}",
        "weapon": cfg["weapon"],
        "default_action": cfg["default_action"],
        "clips": cfg["clips"],
        "build": BUILD,
    }


def render_frames(rig: bpy.types.Object, turntable: bpy.types.Object, cfg: dict, asset: str) -> None:
    scene = bpy.context.scene
    directions = DIRECTIONS[:1] if PROBE else DIRECTIONS
    # An active action re-evaluates the pose bones on every depsgraph update and
    # would overwrite the hand-applied pose, so render from a detached rig.
    rig.animation_data.action = None
    for animation, count in ({"idle": 1} if ATLAS_ONLY else render_counts(cfg)).items():
        for direction, angle in directions:
            target = ROOT / "render" / asset / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                turntable.rotation_euler = (0, 0, angle)
                apply_pose(rig, cfg, animation, frame)
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def build_gear(asset: str, cfg: dict, mats: dict, b: dict, sockets: dict) -> None:
    """Equipped kit, authored in SOCKET SPACE: origin at the socket, +Y up the limb.

    The socket letter is baked into each part name (`_GW`, `_GC`, `_GR`) so
    `bone_group` can route it, and `place_in_rig_space` moves the whole set onto
    the socket once the body exists.
    """
    s = b["scale"]
    if GEAR_TIER >= 0 and GEAR_LAYER in ("weapon", "both"):
        WEAPONS[cfg["weapon"]](asset, GEAR_TIER, mats, s, "W")
        if cfg["shield"]:
            shield(asset, GEAR_TIER, mats, s)
    if GEAR_TIER >= 0 and GEAR_LAYER in ("armour", "both"):
        cuirass(asset, GEAR_TIER, mats, b, s, "C")
    if RELIC:
        build_relic(RELIC, asset, mats, s, "R")


def place_in_rig_space(asset: str, sockets: dict) -> None:
    """Translate every gear part from socket space into rig space. The parts were
    authored around the origin, so adding the socket position is the whole move."""
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH" or obj.get("asset3d_id") != asset:
            continue
        if not obj.name.startswith(f"{asset}_G"):
            continue
        offset = sockets[SOCKET_BONE[obj.name[len(asset) + 2]]]
        obj.location = Vector(obj.location) + Vector(offset)


def main() -> None:
    if CLASS_NAME not in PROFILES:
        raise SystemExit(f"unknown player class: {CLASS_NAME}")
    if BUILD not in BUILDS:
        raise SystemExit(f"unknown build: {BUILD}")
    cfg = PROFILES[CLASS_NAME]
    # The asset id is the GLB/manifest identifier and must match
    # `^[A-Za-z][A-Za-z0-9_]*$`; the dotted form is only the sprite key.
    asset = f"{CLASS_NAME}{BUILD}"
    bpy.ops.wm.read_factory_settings(use_empty=True)
    base.add_scene()
    tune_scene()
    mats = make_materials(cfg)
    arms, legs = add_body(asset, cfg, mats, BUILD)
    sockets = socket_origins(BUILDS[BUILD], arms)
    build_gear(asset, cfg, mats, BUILDS[BUILD], sockets)
    place_in_rig_space(asset, sockets)
    add_markers(asset, BUILDS[BUILD], BUILDS[BUILD]["scale"])
    rig = add_rig(asset, arms, legs, BUILDS[BUILD])
    turntable = add_turntable(rig)
    build_action_library(rig, cfg)
    blend_path = ROOT / "blender" / f"{asset}.blend"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    render_frames(rig, turntable, cfg, asset)
    manifest = {
        "version": 1,
        "assets": [{
            "id": asset,
            "kind": "player",
            "class": CLASS_NAME,
            "build": BUILD,
            "sprite_key": f"Player.{CLASS_NAME}.{BUILD}",
            "calling": cfg["calling"],
            "source": f"source/{asset}.glb",
            "render_root": f"render/{asset}",
            "cell_size": {"width": CELL[0], "height": CELL[1]},
            "directions": [n for n, _ in DIRECTIONS],
            "animations": render_counts(cfg),
            "action_profile": action_profile(cfg),
            "equipped": {"weapon": cfg["weapon"], "tier": GEAR_TIER, "layer": GEAR_LAYER, "relic": RELIC},
            "uvs": True,
            "materials": True,
            "rig": True,
            "pivot": "feet_center",
            "metadata": {"source_kind": "blender_5.1_player_v1", "front_axis": "-Y",
                         "sockets": ["weapon.R", "chest", "relic"]},
        }],
    }
    (ROOT / f"{asset.lower()}_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_PLAYER_V1", asset, blend_path)


if __name__ == "__main__":
    main()
