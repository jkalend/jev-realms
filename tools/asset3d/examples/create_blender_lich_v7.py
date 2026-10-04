"""Create visual-v3 overhaul of the Blender Lich boss.

Underkeep Level 8 Dungeon Boss:
- Replaces generic smiley face with a sinister, gaunt necromancer skull, deep
  sunken orbital sockets, aggressive V-brow, and piercing cyan soul-fire embers.
- Replaces toy dot crown with the majestic Crown of the Underkeep (solid diadem,
  tall central spire, flared horn spires, glowing soul gem).
- Flared gothic pauldrons and high collar replace the spherical shoulder blob,
  giving an imposing, regal boss silhouette.
- Upgraded Scepter of the Soul Harrower with bone talons cradling a glowing
  necrotic Soul Orb and ferrule.
- Chained Grimoire and suspended Soul Phylactery with glowing soul mist.
- Rich abyssal royal purple robes with antique gold filigree trims.

Run:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_lich_v7.py -- \\
      --output-root docs/gfx/proto/visual-v3/lich-blender-5.1
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bmesh
import bpy  # type: ignore
from mathutils import Matrix, Vector  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parent))
import create_blender_lich_v4 as base  # type: ignore
import create_blender_lich_v5 as v5  # type: ignore

_TAIL = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
_ROOT_INDEX = _TAIL.index("--output-root") + 1 if "--output-root" in _TAIL else None
ROOT = Path(_TAIL[_ROOT_INDEX]).resolve() if _ROOT_INDEX is not None else Path.cwd()
PROBE = "--probe" in _TAIL
CELL = (256, 320)  # High-fidelity v3 resolution
BASE_ROT = base.BASE_ROT
DIRECTIONS = base.DIRECTIONS
ANIMATIONS = base.ANIMATIONS
ACTION_PROFILE = v5.ACTION_PROFILE

ARM_POINTS = {
    "L": ((-0.37, 2.04, 0.0), (-0.53, 1.76, -0.03), (-0.49, 1.43, 0.05), (-0.50, 1.27, 0.08)),
    "R": ((0.37, 2.04, 0.0), (0.62, 1.76, 0.10), (0.72, 1.48, 0.20), (0.77, 1.36, 0.22)),
}


def emissive_material(name: str, color: tuple[float, float, float, float], strength: float = 3.5) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = 0.2
    if "Emission Color" in bsdf.inputs:
        bsdf.inputs["Emission Color"].default_value = color
    if "Emission Strength" in bsdf.inputs:
        bsdf.inputs["Emission Strength"].default_value = strength
    return mat


def make_hood_v3(mat: bpy.types.Material, rim_mat: bpy.types.Material) -> tuple[bpy.types.Object, bpy.types.Object, bpy.types.Object]:
    """Peaked gothic cowl that plunges the upper cranium into shadow."""
    profile = [
        (2.28, 0.31, 0.18),
        (2.42, 0.40, 0.25),
        (2.66, 0.45, 0.28),
        (2.93, 0.40, 0.24),
        (3.18, 0.12, 0.10),
    ]
    steps = 27
    start = math.radians(44)
    end = math.radians(316)
    vertices: list[tuple[float, float, float]] = []
    for y, rx, rz in profile:
        for step in range(steps):
            theta = start + (end - start) * step / (steps - 1)
            vertices.append((rx * math.sin(theta), y, rz * math.cos(theta)))
    faces: list[tuple[int, ...]] = []
    for row in range(len(profile) - 1):
        for column in range(steps - 1):
            a = row * steps + column
            faces.append((a, a + 1, a + steps + 1, a + steps))
    hood = v5.mesh_object("Lich_Hood", vertices, faces, mat, subsurf=1, thickness=0.055)
    edge_l = [(rx * math.sin(start), y, rz * math.cos(start) + 0.01) for y, rx, rz in profile]
    edge_r = [(rx * math.sin(end), y, rz * math.cos(end) + 0.01) for y, rx, rz in profile]
    tube_l = base.add_curve_tube("Lich_HoodEdgeL", edge_l, 0.024, rim_mat)
    tube_r = base.add_curve_tube("Lich_HoodEdgeR", edge_r, 0.024, rim_mat)
    return hood, tube_l, tube_r


def add_skull_v3(materials: dict[str, bpy.types.Material]) -> None:
    """Anatomical, gaunt necromancer skull with deep sockets and burning soul embers."""
    bone = materials["bone"]
    bone_dark = materials["bone_dark"]
    ivory = materials["ivory"]
    black = materials["black"]
    soul = materials["soul_ember"]
    soul_core = materials["soul_core"]

    # Void interior cavity inside hood
    base.add_ico("Lich_ShadowCavity", (0, 2.75, 0.05), (0.27, 0.29, 0.22), black)

    # Cranium (broad weathered forehead, gaunt temples)
    base.add_ico("Lich_SkullCranium", (0, 2.80, 0.085), (0.23, 0.27, 0.185), bone)
    base.add_ico("Lich_SkullMuzzle", (0, 2.61, 0.25), (0.115, 0.062, 0.085), bone)
    base.add_ico("Lich_MouthCavity", (0, 2.49, 0.325), (0.13, 0.075, 0.035), black)

    # Sinister brow and deep orbital sockets
    for side, label in ((-1, "L"), (1, "R")):
        # Aggressive furrowed brow ridge angled downward toward nose
        base.add_curve_tube(
            f"Lich_BrowRidge{label}",
            [(side * 0.025, 2.81, 0.315), (side * 0.11, 2.865, 0.295), (side * 0.21, 2.82, 0.245)],
            0.026, bone,
        )
        # Deep recessed eye socket
        base.add_ico(f"Lich_Socket{label}", (side * 0.102, 2.76, 0.285), (0.088, 0.082, 0.042), black)
        # Piercing soul fire embers
        base.add_ico(f"Lich_EyeGlow{label}", (side * 0.102, 2.76, 0.318), (0.034, 0.030, 0.016), soul)
        base.add_ico(f"Lich_EyeCore{label}", (side * 0.102, 2.76, 0.322), (0.016, 0.014, 0.008), soul_core)

        # Sharp zygomatic arch (cheekbone)
        base.add_curve_tube(
            f"Lich_Cheekbone{label}",
            [(side * 0.22, 2.70, 0.235), (side * 0.175, 2.59, 0.31), (side * 0.095, 2.56, 0.335)],
            0.028, bone,
        )
        # Mandible ramus
        base.add_curve_tube(
            f"Lich_JawRamus{label}",
            [(side * 0.185, 2.62, 0.215), (side * 0.165, 2.48, 0.295), (side * 0.10, 2.435, 0.34)],
            0.032, bone,
        )

    # Chin and nasal aperture
    base.add_curve_tube("Lich_JawChin", [(-0.10, 2.44, 0.335), (0, 2.42, 0.352), (0.10, 2.44, 0.335)], 0.032, bone)
    base.add_curve_tube("Lich_NoseBridge", [(0, 2.74, 0.308), (0, 2.67, 0.362)], 0.017, bone)
    v5.mesh_object(
        "Lich_NasalCavity",
        [(-0.045, 2.665, 0.364), (0.045, 2.665, 0.364), (0, 2.595, 0.375)],
        [(0, 1, 2)], black, subsurf=0, thickness=0.008,
    )

    # Skeletal teeth
    for index, x in enumerate((-0.085, -0.042, 0.0, 0.042, 0.085)):
        base.add_ico(f"Lich_UpperTeeth{index}", (x, 2.542, 0.366), (0.018, 0.030, 0.014), ivory)
        base.add_ico(f"Lich_LowerTeeth{index}", (x, 2.472, 0.363), (0.017, 0.026, 0.014), ivory)

    # Gothic hood peak fold
    base.add_curve_tube(
        "Lich_HoodPeak",
        [(0, 3.16, 0.12), (0, 3.20, 0.24), (0, 3.16, 0.34)],
        0.026, materials["violet_light"],
    )


def add_crown_of_underkeep(materials: dict[str, bpy.types.Material]) -> None:
    """The majestic Crown of the Underkeep: jagged gold diadem with radiant Soul Gem."""
    gold = materials["gold"]
    soul = materials["soul_ember"]
    soul_core = materials["soul_core"]

    # Solid diadem band circling brow
    base.add_torus("Lich_CrownBand", (0, 3.03, 0.05), 0.28, 0.024, gold, (math.pi / 2, 0, 0))

    # Center towering spiked blade
    base.add_ico("Lich_CrownSpireCenterBase", (0, 3.08, 0.29), (0.055, 0.080, 0.035), gold)
    base.add_ico("Lich_CrownSpireCenterMid", (0, 3.22, 0.26), (0.042, 0.095, 0.028), gold)
    base.add_ico("Lich_CrownSpireCenterTip", (0, 3.39, 0.22), (0.026, 0.090, 0.020), gold)

    # Flanking mid spires
    for side, label in ((-1, "L"), (1, "R")):
        base.add_ico(f"Lich_CrownSpireMid{label}", (side * 0.135, 3.12, 0.23), (0.045, 0.115, 0.032), gold)
        base.add_ico(f"Lich_CrownSpireMidTip{label}", (side * 0.165, 3.27, 0.20), (0.028, 0.080, 0.022), gold)

        # Swept horn blades at outer temples
        base.add_curve_tube(
            f"Lich_CrownHorn{label}",
            [(side * 0.24, 3.04, 0.12), (side * 0.30, 3.16, 0.03), (side * 0.34, 3.28, -0.06)],
            0.028, gold,
        )

    # Radiant Soul Gem mounted in the center brow of the crown
    base.add_ico("Lich_CrownGemMount", (0, 3.05, 0.315), (0.055, 0.065, 0.025), gold)
    base.add_ico("Lich_CrownGem", (0, 3.05, 0.328), (0.038, 0.050, 0.022), soul)
    base.add_ico("Lich_CrownGemCore", (0, 3.05, 0.334), (0.018, 0.026, 0.012), soul_core)


def add_shoulders_and_mantle(materials: dict[str, bpy.types.Material]) -> None:
    """Gothic high collar and flared archmage pauldrons replace the round blob."""
    violet_light = materials["violet_light"]
    gold = materials["gold"]
    soul = materials["soul_ember"]

    # Gothic high collar framing the head
    base.add_curve_tube("Lich_CollarL", [(-0.26, 2.15, -0.16), (-0.32, 2.38, -0.18), (-0.22, 2.56, -0.14)], 0.036, violet_light)
    base.add_curve_tube("Lich_CollarR", [(0.26, 2.15, -0.16), (0.32, 2.38, -0.18), (0.22, 2.56, -0.14)], 0.036, violet_light)
    base.add_curve_tube("Lich_CollarBack", [(0, 2.18, -0.25), (0, 2.48, -0.22)], 0.040, violet_light)
    base.add_curve_tube("Lich_CollarRim", [(-0.22, 2.56, -0.14), (0, 2.48, -0.22), (0.22, 2.56, -0.14)], 0.018, gold)

    # Flared layered pauldrons giving wide commanding silhouette
    for side, label in ((-1, "L"), (1, "R")):
        # Main pauldron shell
        base.add_ico(f"Lich_PauldronMain{label}", (side * 0.46, 2.08, 0.02), (0.20, 0.12, 0.18), violet_light)
        # Upward flared shoulder spike wing
        base.add_curve_tube(
            f"Lich_PauldronSpike{label}",
            [(side * 0.36, 2.06, 0.02), (side * 0.56, 2.16, 0.05), (side * 0.68, 2.24, 0.06)],
            0.030, gold,
        )
        # Lower pauldron rim trim
        base.add_curve_tube(
            f"Lich_PauldronTrim{label}",
            [(side * 0.34, 1.96, 0.14), (side * 0.58, 2.02, 0.10), (side * 0.52, 1.94, -0.10)],
            0.022, gold,
        )

    # Ornate Mantle Brooch at chest
    base.add_ico("Lich_Brooch", (0, 2.10, 0.345), (0.075, 0.085, 0.040), gold)
    base.add_ico("Lich_BroochGem", (0, 2.10, 0.370), (0.030, 0.040, 0.015), soul)


def add_ribs_v3(materials: dict[str, bpy.types.Material]) -> None:
    """Anatomical, menacing ribcage in sunken dark torso cavity."""
    bone = materials["bone"]
    ivory = materials["ivory"]
    black = materials["black"]

    # Sunken chest shadow cavity
    base.add_ico("Lich_ChestCavity", (0, 1.78, 0.27), (0.20, 0.24, 0.08), black)

    for side, label in ((-1, "L"), (1, "R")):
        base.add_curve_tube(
            f"Lich_RibShadow{label}",
            [(side * 0.13, 1.55, 0.299), (side * 0.17, 1.82, 0.301), (side * 0.12, 2.02, 0.294)],
            0.052, black,
        )
        for index, y in enumerate((1.61, 1.72, 1.83, 1.94)):
            outer = 0.25 - index * 0.013
            base.add_curve_tube(
                f"Lich_Rib{label}{index}",
                [(side * 0.035, y - 0.015, 0.363), (side * 0.14, y - 0.045, 0.351),
                 (side * outer, y + 0.035, 0.319)],
                0.026, bone,
            )
        base.add_curve_tube(
            f"Lich_Collarbone{label}",
            [(0, 1.98, 0.357), (side * 0.17, 2.045, 0.334), (side * 0.32, 2.025, 0.293)],
            0.032, bone,
        )
    base.add_curve_tube("Lich_Sternum", [(0, 1.58, 0.348), (0, 1.98, 0.359)], 0.028, ivory)


def add_arms_v3(materials: dict[str, bpy.types.Material]) -> None:
    violet = materials["violet"]
    violet_light = materials["violet_light"]
    bone = materials["bone"]
    bone_dark = materials["bone_dark"]
    for label, points in ARM_POINTS.items():
        shoulder, elbow, wrist, hand = points
        base.add_curve_tube(f"Lich_UpperArm{label}", [shoulder, elbow], 0.135, violet)
        base.add_ico(f"Lich_Elbow{label}", elbow, (0.135, 0.13, 0.125), violet_light)
        base.add_curve_tube(f"Lich_Forearm{label}", [elbow, wrist], 0.105, violet)
        base.add_ico(f"Lich_Cuff{label}", wrist, (0.115, 0.075, 0.10), materials["gold"])
        base.add_ico(f"Lich_Hand{label}", hand, (0.10, 0.13, 0.09), bone)
        for index, dx in enumerate((-0.038, 0.0, 0.038)):
            base.add_curve_tube(
                f"Lich_Finger{label}{index}",
                [(hand[0] + dx, hand[1] - 0.10, hand[2] + 0.01), (hand[0] + dx, hand[1] - 0.18, hand[2] + 0.02)],
                0.010,
                bone_dark,
            )


def add_scepter_of_soul_harrower(materials: dict[str, bpy.types.Material]) -> None:
    """Scepter of the Soul Harrower: gnarled stave, talons cradling Soul Orb, ferrule."""
    iron = materials["iron"]
    gold = materials["gold"]
    bone = materials["bone"]
    soul = materials["soul_ember"]
    soul_core = materials["soul_core"]

    # Main staff shaft (dark forged iron)
    base.add_curve_tube(
        "Lich_StaffShaft",
        [(0.80, 0.10, 0.22), (0.78, 1.25, 0.21), (0.80, 2.30, 0.22), (0.78, 3.15, 0.22)],
        0.038, iron,
    )

    # Gold filigree bands
    base.add_torus("Lich_StaffRing0", (0.78, 1.40, 0.21), 0.054, 0.014, gold, (math.pi / 2, 0, 0))
    base.add_torus("Lich_StaffRing1", (0.79, 2.10, 0.22), 0.054, 0.014, gold, (math.pi / 2, 0, 0))
    base.add_torus("Lich_StaffRing2", (0.78, 2.95, 0.22), 0.058, 0.016, gold, (math.pi / 2, 0, 0))

    # Headpiece collar
    base.add_ico("Lich_StaffHeadBase", (0.78, 3.16, 0.22), (0.085, 0.065, 0.085), gold)

    # Skeletal horn talons cradling the Soul Orb
    base.add_curve_tube(
        "Lich_StaffTalonL",
        [(0.70, 3.16, 0.22), (0.63, 3.32, 0.22), (0.71, 3.48, 0.22)],
        0.024, bone,
    )
    base.add_curve_tube(
        "Lich_StaffTalonR",
        [(0.86, 3.16, 0.22), (0.93, 3.32, 0.22), (0.85, 3.48, 0.22)],
        0.024, bone,
    )
    base.add_curve_tube(
        "Lich_StaffTalonBack",
        [(0.78, 3.16, 0.14), (0.78, 3.34, 0.09), (0.78, 3.47, 0.15)],
        0.022, bone,
    )

    # Floating Soul Orb with brilliant inner core
    base.add_ico("Lich_StaffOrb", (0.78, 3.32, 0.22), (0.135, 0.135, 0.135), soul)
    base.add_ico("Lich_StaffOrbCore", (0.78, 3.32, 0.22), (0.075, 0.075, 0.075), soul_core)

    # Orbiting soul shards
    base.add_ico("Lich_StaffShard0", (0.70, 3.36, 0.32), (0.025, 0.040, 0.018), soul)
    base.add_ico("Lich_StaffShard1", (0.86, 3.28, 0.32), (0.022, 0.035, 0.016), soul)

    # Bottom pointed counterweight ferrule
    base.add_curve_tube("Lich_StaffFerrule", [(0.80, 0.12, 0.22), (0.80, -0.08, 0.22)], 0.032, iron)
    base.add_ico("Lich_StaffFerruleTip", (0.80, -0.10, 0.22), (0.025, 0.060, 0.025), gold)


def add_phylactery_and_tome(materials: dict[str, bpy.types.Material]) -> None:
    """Tome of Souls on left hip and suspended Soul Phylactery on right hip."""
    gold = materials["gold"]
    soul = materials["soul_ember"]
    soul_core = materials["soul_core"]

    # Existing detailed tome on left hip
    v5.add_book(materials)

    # Suspended Soul Phylactery on right hip
    base.add_curve_tube(
        "Lich_PhylacteryChain",
        [(0.26, 1.24, 0.18), (0.33, 1.10, 0.22), (0.31, 0.98, 0.24)],
        0.014, gold,
    )
    base.add_ico("Lich_PhylacteryCap", (0.31, 0.96, 0.24), (0.045, 0.028, 0.045), gold)
    base.add_ico("Lich_PhylacteryVial", (0.31, 0.88, 0.24), (0.042, 0.085, 0.042), soul)
    base.add_ico("Lich_PhylacteryCore", (0.31, 0.88, 0.24), (0.020, 0.055, 0.020), soul_core)
    base.add_ico("Lich_PhylacteryBase", (0.31, 0.80, 0.24), (0.040, 0.026, 0.040), gold)


def assign_detail_weights_v3(parts: list[bpy.types.Object]) -> None:
    head_parts = (
        "Lich_Skull", "Lich_Cheekbone", "Lich_Jaw", "Lich_Socket",
        "Lich_EyeGlow", "Lich_EyeCore", "Lich_Brow", "Lich_Nose", "Lich_Nasal",
        "Lich_Mouth", "Lich_UpperTeeth", "Lich_LowerTeeth",
        "Lich_Hood", "Lich_Crown", "Lich_ShadowCavity"
    )
    spine_parts = (
        "Lich_Book", "Lich_Cape", "Lich_Robe", "Lich_Collar",
        "Lich_Pauldron", "Lich_Phylactery", "Lich_Belt", "Lich_Rib",
        "Lich_Collarbone", "Lich_Sternum", "Lich_Brooch", "Lich_ChestCavity"
    )
    for obj in parts:
        name = obj.name
        if name.startswith(spine_parts):
            bone = "spine"
        elif name.startswith(head_parts):
            bone = "head"
        else:
            continue
        indices = list(range(len(obj.data.vertices)))
        for group in obj.vertex_groups:
            group.remove(indices)
        group = obj.vertex_groups.get(bone) or obj.vertex_groups.new(name=bone)
        group.add(indices, 1.0, "REPLACE")


def assign_arm_weights_v3(parts: list[bpy.types.Object]) -> None:
    for obj in parts:
        name = obj.name
        is_staff = name.startswith("Lich_Staff")
        if not is_staff and not (
            name.startswith("Lich_UpperArm") or name.startswith("Lich_Forearm")
            or name.startswith("Lich_Elbow") or name.startswith("Lich_Cuff")
            or name.startswith("Lich_Hand") or name.startswith("Lich_Finger")
        ):
            continue
        indices = list(range(len(obj.data.vertices)))
        for group in obj.vertex_groups:
            group.remove(indices)
        if is_staff:
            group_name = "weapon.R"
        else:
            label = "L" if name.endswith("L") or "ArmL" in name or "ElbowL" in name or "CuffL" in name or "HandL" in name or "FingerL" in name else "R"
            if name.startswith("Lich_UpperArm") or name.startswith("Lich_Elbow"):
                group_name = f"upper_arm.{label}"
            elif name.startswith("Lich_Forearm") or name.startswith("Lich_Cuff"):
                group_name = f"forearm.{label}"
            else:
                group_name = f"hand.{label}"
        group = obj.vertex_groups.get(group_name) or obj.vertex_groups.new(name=group_name)
        group.add(indices, 1.0, "REPLACE")


def add_rig_v3() -> bpy.types.Object:
    parts = [obj for obj in bpy.context.scene.objects if obj.type == "MESH" and obj.get("asset3d_id") == "Lich"]
    data = bpy.data.armatures.new("Lich_RigData")
    rig = bpy.data.objects.new("Lich_Rig", data)
    bpy.context.collection.objects.link(rig)
    rig["asset3d_id"] = "Lich"
    rig["asset3d_rig"] = True
    rig.rotation_euler = BASE_ROT
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = data.edit_bones
    root = bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 1, 0)
    spine = bones.new("spine"); spine.head = (0, 1, 0); spine.tail = (0, 2.18, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.18, 0); neck.tail = (0, 2.53, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.53, 0); head.tail = (0, 3.15, 0); head.parent = neck
    for label, points in ARM_POINTS.items():
        upper = bones.new(f"upper_arm.{label}"); upper.head, upper.tail = points[0], points[1]; upper.parent = spine
        fore = bones.new(f"forearm.{label}"); fore.head, fore.tail = points[1], points[2]; fore.parent = upper; fore.use_connect = True
        hand = bones.new(f"hand.{label}"); hand.head, hand.tail = points[2], points[3]; hand.parent = fore; hand.use_connect = True
        if label == "R":
            weapon = bones.new("weapon.R"); weapon.head = points[2]; weapon.tail = (0.78, 3.35, 0.22); weapon.parent = hand
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    assign_arm_weights_v3(parts)
    assign_detail_weights_v3(parts)
    for obj in parts:
        obj.matrix_parent_inverse = Matrix.Identity(4)
    for marker in (bpy.data.objects.get("Lich_front"), bpy.data.objects.get("Lich_back"), bpy.data.objects.get("Lich_pivot")):
        if marker is not None:
            marker.parent = rig
            marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig


def apply_pose_v3(rig: bpy.types.Object, anim: str, frame: int) -> None:
    v5.reset_pose(rig)
    bob, spine_roll, head_pitch = v5.pose(anim, frame)
    rig.location = (0, bob, 0)
    rig.rotation_euler = v5.BASE_ROT
    rig.pose.bones["spine"].rotation_euler.z = spine_roll
    rig.pose.bones["head"].rotation_euler.z = head_pitch
    if anim == "idle":
        arms = {
            "upper_arm.L": (0.12, 0.06, -0.28), "forearm.L": (0.24, 0.04, 0.34), "hand.L": (0.0, 0.0, 0.08),
            "upper_arm.R": (-0.08, 0.08, 0.26), "forearm.R": (0.20, 0.06, -0.38), "hand.R": (0.0, 0.04, -0.12),
        }
    elif anim == "walk":
        sign = 1 if frame == 0 else -1
        arms = {
            "upper_arm.L": (0.06 * sign, 0.02, 0.30 * sign), "forearm.L": (0.16, 0.0, -0.24 * sign), "hand.L": (0.0, 0.0, 0.06 * sign),
            "upper_arm.R": (-0.05 * sign, 0.03, -0.26 * sign), "forearm.R": (0.18, 0.02, -0.14), "hand.R": (0.0, 0.03, -0.05 * sign),
        }
    elif anim == "attack":
        if frame == 0:
            arms = {
                "upper_arm.L": (0.18, -0.08, -0.52), "forearm.L": (0.34, 0.0, -0.72), "hand.L": (0.0, 0.0, 0.12),
                "upper_arm.R": (-0.16, 0.10, 0.52), "forearm.R": (0.30, 0.06, -0.78), "hand.R": (0.0, 0.06, -0.14),
            }
        elif frame == 1:
            arms = {
                "upper_arm.L": (0.10, -0.12, -0.22), "forearm.L": (0.22, 0.0, -0.34), "hand.L": (0.0, 0.0, 0.06),
                "upper_arm.R": (-0.22, 0.16, -0.34), "forearm.R": (0.12, 0.10, -0.12), "hand.R": (0.0, 0.08, -0.04),
            }
        else:
            arms = {
                "upper_arm.L": (0.06, -0.06, -0.14), "forearm.L": (0.14, 0.0, -0.20), "hand.L": (0.0, 0.0, 0.04),
                "upper_arm.R": (-0.10, 0.08, -0.18), "forearm.R": (0.16, 0.04, -0.30), "hand.R": (0.0, 0.04, -0.06),
            }
    else:
        arms = {
            "upper_arm.L": (0.20, -0.10, 0.54), "forearm.L": (0.40, 0.0, -0.78), "hand.L": (0.0, 0.0, 0.16),
            "upper_arm.R": (-0.18, 0.10, -0.50), "forearm.R": (0.38, 0.06, -0.84), "hand.R": (0.0, 0.06, -0.16),
        }
    for name, angles in arms.items():
        rig.pose.bones[name].rotation_euler = angles
    bpy.context.view_layer.update()


def render_frames_v3(rig: bpy.types.Object, turntable: bpy.types.Object) -> None:
    scene = bpy.context.scene
    directions = DIRECTIONS[:1] if PROBE else DIRECTIONS
    animations = ANIMATIONS
    rig.animation_data.action = None
    for animation, count in animations.items():
        for direction, angle in directions:
            target = ROOT / "render/Lich" / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                turntable.rotation_euler = (0, 0, angle)
                apply_pose_v3(rig, animation, frame)
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def main() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = base.add_scene()
    scene.render.resolution_x, scene.render.resolution_y = CELL

    soul_mat = emissive_material("Lich_SoulEmber", (0.03, 0.92, 0.88, 1.0), 3.5)
    soul_core = emissive_material("Lich_SoulCore", (0.78, 0.98, 1.0, 1.0), 5.5)

    materials = {
        "violet": base.material("Lich_Robe", (0.10, 0.03, 0.16, 1.0), 0.04, 0.85),
        "violet_light": base.material("Lich_Hood", (0.18, 0.06, 0.26, 1.0), 0.03, 0.80),
        "violet_dark": base.material("Lich_Cape", (0.06, 0.02, 0.10, 1.0), 0.02, 0.88),
        "bone": base.material("Lich_Bone", (0.70, 0.66, 0.55, 1.0), 0.03, 0.62),
        "ivory": base.material("Lich_Ivory", (0.86, 0.82, 0.70, 1.0), 0.02, 0.55),
        "bone_dark": base.material("Lich_BoneDark", (0.22, 0.20, 0.17, 1.0), 0.02, 0.72),
        "gold": base.material("Lich_Gold", (0.74, 0.48, 0.12, 1.0), 0.76, 0.32),
        "iron": base.material("Lich_Iron", (0.14, 0.14, 0.16, 1.0), 0.85, 0.38),
        "soul_ember": soul_mat,
        "soul_core": soul_core,
        "teal": soul_mat,
        "black": base.material("Lich_Black", (0.003, 0.004, 0.006, 1.0), 0.0, 0.85),
    }

    body = base.make_profile_body(materials)
    v5.make_cape(materials["violet_dark"])
    make_hood_v3(materials["violet_dark"], materials["gold"])
    add_skull_v3(materials)
    add_crown_of_underkeep(materials)
    add_shoulders_and_mantle(materials)
    add_ribs_v3(materials)
    add_arms_v3(materials)
    add_scepter_of_soul_harrower(materials)
    base.add_torus("Lich_Belt", (0, 1.24, 0), 0.35, 0.032, materials["gold"], (math.pi / 2, 0, 0))
    add_phylactery_and_tome(materials)

    for x in (-0.22, 0, 0.22):
        base.add_curve_tube(f"Lich_RobeFold{x}", [(x, 0.22, 0.46), (x * 0.8, 0.70, 0.40), (x * 0.45, 1.18, 0.30)], 0.011, materials["violet_light"])
    base.add_curve_tube("Lich_CapeSeam", [(0, 2.05, -0.33), (0, 1.35, -0.43), (0, 0.35, -0.53)], 0.013, materials["violet_light"])
    base.add_markers()

    rig = add_rig_v3()
    turntable = v5.add_turntable(rig)

    # Patch v5.apply_pose to use our v3 poses for action library
    v5.apply_pose = apply_pose_v3
    v5.build_action_library(rig)

    blend_path = ROOT / "blender/Lich.blend"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))

    render_frames_v3(rig, turntable)

    manifest = {
        "version": 1,
        "assets": [{
            "id": "Lich", "kind": "boss", "source": "source/Lich.glb", "render_root": "render/Lich",
            "cell_size": {"width": CELL[0], "height": CELL[1]}, "directions": [n for n, _ in DIRECTIONS],
            "animations": ANIMATIONS, "uvs": True, "materials": True, "rig": True, "pivot": "feet_center",
            "action_profile": ACTION_PROFILE,
            "metadata": {"source_kind": "blender_5.1_skeletal_humanoid_v3", "front_axis": "-Y"},
        }],
    }
    (ROOT / "lich_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_LICH_V7_SUCCESS", blend_path)


if __name__ == "__main__":
    main()
