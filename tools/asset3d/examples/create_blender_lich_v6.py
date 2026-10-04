"""Refine v5 Lich arms with articulated upper-arm/forearm/hand geometry.

v5 fixed the skeletal identity and the direction/turntable problem, but each arm
was still one thick swept tube with a single effective hinge. v6 splits the arm
into upper arm, elbow, forearm, cuff and hand, then applies independent
shoulder/elbow/wrist rotations for every action lane.

Run:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_lich_v6.py -- \
      --output-root docs/gfx/proto/visual-v2/lich-blender-5.1-v6
"""

from __future__ import annotations

import sys
from pathlib import Path

import bpy  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parent))
import create_blender_lich_v5 as v5  # type: ignore


ARM_POINTS = {
    "L": ((-0.37, 2.04, 0.0), (-0.53, 1.76, -0.03), (-0.49, 1.43, 0.05), (-0.50, 1.27, 0.08)),
    "R": ((0.37, 2.04, 0.0), (0.62, 1.76, 0.10), (0.72, 1.48, 0.20), (0.77, 1.36, 0.22)),
}


def add_arms(materials: dict[str, bpy.types.Material]) -> None:
    violet = materials["violet"]
    violet_light = materials["violet_light"]
    bone = materials["bone"]
    bone_dark = materials["bone_dark"]
    for label, points in ARM_POINTS.items():
        shoulder, elbow, wrist, hand = points
        # Separate pieces give the joint a real bulge and let the forearm bend
        # instead of stretching one continuous sausage.
        v5.base.add_curve_tube(f"Lich_UpperArm{label}", [shoulder, elbow], 0.135, violet)
        v5.base.add_ico(f"Lich_Elbow{label}", elbow, (0.135, 0.13, 0.125), violet_light)
        v5.base.add_curve_tube(f"Lich_Forearm{label}", [elbow, wrist], 0.105, violet)
        v5.base.add_ico(f"Lich_Cuff{label}", wrist, (0.115, 0.075, 0.10), materials["gold"])
        v5.base.add_ico(f"Lich_Hand{label}", hand, (0.10, 0.13, 0.09), bone)
        for index, dx in enumerate((-0.038, 0.0, 0.038)):
            v5.base.add_curve_tube(
                f"Lich_Finger{label}{index}",
                [(hand[0] + dx, hand[1] - 0.10, hand[2] + 0.01), (hand[0] + dx, hand[1] - 0.18, hand[2] + 0.02)],
                0.010,
                bone_dark,
            )


def assign_arm_weights(parts: list[bpy.types.Object]) -> None:
    for obj in parts:
        name = obj.name
        is_staff = name in {"Lich_Staff", "Lich_StaffOrb"}
        if not is_staff and not (name.startswith("Lich_UpperArm") or name.startswith("Lich_Forearm") or name.startswith("Lich_Elbow") or name.startswith("Lich_Cuff") or name.startswith("Lich_Hand") or name.startswith("Lich_Finger")):
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
            elif name.startswith("Lich_Forearm"):
                group_name = f"forearm.{label}"
            else:
                group_name = f"hand.{label}"
        group = obj.vertex_groups.get(group_name) or obj.vertex_groups.new(name=group_name)
        group.add(indices, 1.0, "REPLACE")


def apply_pose(rig: bpy.types.Object, anim: str, frame: int) -> None:
    v5.reset_pose(rig)
    bob, spine_roll, head_pitch = v5.pose(anim, frame)
    rig.location = (0, bob, 0)
    rig.rotation_euler = v5.BASE_ROT
    rig.pose.bones["spine"].rotation_euler.z = spine_roll
    rig.pose.bones["head"].rotation_euler.z = head_pitch
    # Each tuple is (shoulder pitch, shoulder depth, shoulder roll). The forearm
    # always keeps a bend; the hand follows the staff instead of staying rigid.
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


v5.add_arms = add_arms
v5.assign_arm_weights = assign_arm_weights
v5.apply_pose = apply_pose
v5.main()
