"""Opt-in, non-runtime humanoid art study for the Blender NPC roster.

Use with create_blender_npc_roster_v1.py --visual-v2 and a *separate* output root.
The old authoring pass and all approved 2D/baked assets remain untouched.
"""

from __future__ import annotations

import math
from character_surface import folded_panel


def shaped_limb(api, name, joints, widths, depths, material, npc, smooth=True):
    """One continuous tapered volume, not a cylinder with an elbow/knee ball."""
    vertices = []
    sides = 10
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
    return api.mesh_object(name, vertices, faces, material, npc, subsurf=int(smooth))


def cloth_panel(api, name, rows, mat, npc, thickness=0.025):
    """Tailored drape with broad gravity folds and an anchored sewn border."""
    metal = mat.node_tree.nodes.get("Principled BSDF").inputs["Metallic"].default_value >= .4
    if metal:
        vertices = [point for row in rows for point in row]
        faces = [(i * 3 + j, i * 3 + j + 1, (i + 1) * 3 + j + 1, (i + 1) * 3 + j)
                 for i in range(len(rows) - 1) for j in range(2)]
    else:
        vertices, faces = folded_panel(rows)
    obj = api.mesh_object(name, vertices, faces, mat, npc, thickness=thickness,
                          bevel=min(thickness * .35, .009))
    for polygon in obj.data.polygons:
        polygon.use_smooth = not metal
    return obj


def between(a, b, amount):
    return tuple(x + (y - x) * amount for x, y in zip(a, b))


def garment(npc, cfg, mats, s, api):
    kind = cfg["garment"]
    def rows(data):
        return [[(x * s, y * s, z * s) for x, y, z in row] for row in data]
    if kind == "apron":
        # A wraparound wool vest with a short, soft work apron tied over it.
        # The previous bib ran neck-to-knee and read as a rigid chest plate.
        api.lathe(f"{npc}_WorkVest", [(1.08 * s, .36 * s, .28 * s),
                  (1.43 * s, .36 * s, .28 * s), (1.81 * s, .43 * s, .30 * s),
                  (2.08 * s, .37 * s, .26 * s)], mats["cloth_shade"], npc, segments=14)
        api.mesh_object(f"{npc}_WorkShirtInsert",
                        [(-.12 * s, 2.09 * s, .30 * s),
                         (.12 * s, 2.09 * s, .30 * s),
                         (0, 1.82 * s, .34 * s)],
                        [(0, 1, 2)], mats["cloth"], npc, thickness=.02 * s)
        api.add_curve(f"{npc}_WorkVestClosure",
                      [(0, 2.08 * s, .315 * s), (0, 1.81 * s, .35 * s),
                       (0, 1.40 * s, .325 * s)],
                      .012 * s, mats["leather"], npc)
        for row, height in enumerate((1.50, 1.64, 1.77)):
            api.add_curve(f"{npc}_WorkVestLace{row}",
                          [(-.10 * s, height * s, .335 * s),
                           (.10 * s, (height + .055) * s, .335 * s)],
                          .012 * s, mats["leather"], npc)
        cloth_panel(api, f"{npc}_Apron", rows([
            ((-.28, 1.16, .31), (0, 1.16, .38), (.28, 1.16, .31)),
            ((-.30, .94, .34), (0, .93, .40), (.30, .94, .34)),
            ((-.34, .57, .35), (0, .52, .45), (.34, .57, .35)),
        ]), mats["linen"], npc)
        for side in (-1, 1):
            api.add_curve(f"{npc}_ApronStrap{api.side_label(side)}",
                          [(side * .19 * s, 1.98 * s, .30 * s),
                           (side * .36 * s, 1.79 * s, .23 * s),
                           (side * .37 * s, 1.31 * s, .16 * s),
                           (side * .32 * s, 1.13 * s, -.22 * s),
                           (side * .10 * s, 1.14 * s, -.33 * s)],
                          .025 * s, mats["leather"], npc)
            api.add_curve(f"{npc}_ApronFold{api.side_label(side)}",
                          [(side * .13 * s, .94 * s, .405 * s),
                           (side * .18 * s, .55 * s, .44 * s)],
                          .012 * s, mats["cloth_shade"], npc)
        api.add_curve(f"{npc}_ApronSeam", [(-.32 * s, .56 * s, .36 * s),
                      (0, .53 * s, .45 * s), (.32 * s, .56 * s, .36 * s)],
                      .018 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_Collar", [(-.28 * s, 2.09 * s, .11 * s),
                      (0, 2.13 * s, .25 * s), (.28 * s, 2.09 * s, .11 * s)],
                      .036 * s, mats["linen"], npc)
        api.add_ico(f"{npc}_Pouch", (.37 * s, 1.02 * s, .15 * s),
                    (.12 * s, .13 * s, .07 * s), mats["leather"], npc)
    elif kind == "cloak":
        cloth_panel(api, f"{npc}_Cloak", rows([
            ((-.32, 2.19, -.13), (0, 2.21, -.36), (.32, 2.19, -.13)),
            ((-.43, 1.80, -.19), (0, 1.78, -.49), (.43, 1.80, -.19)),
            ((-.46, 1.21, -.22), (0, 1.17, -.53), (.46, 1.21, -.22)),
            ((-.52, .63, -.22), (0, .52, -.58), (.52, .63, -.22)),
        ]), mats["cloth_shade"], npc, .045 * s)
        # A narrow shoulder wrap belongs at the clavicles; long paired strips
        # hanging over the chest read as rigid frontal gear from every yaw.
        for side in (-1, 1):
            label = api.side_label(side)
            cloth_panel(api, f"{npc}_Lapels{label}", rows([
                ((side * .19, 2.17, .18), (side * .29, 2.15, .20), (side * .38, 2.09, .09)),
                ((side * .20, 2.00, .26), (side * .32, 2.00, .22), (side * .40, 1.96, .10)),
                ((side * .28, 1.87, .25), (side * .37, 1.85, .19), (side * .43, 1.83, .08)),
            ]), mats["cloth_shade"], npc, .018 * s)
        api.add_curve(f"{npc}_CloakClasp", [(-.09 * s, 2.10 * s, .28 * s),
                      (0, 2.10 * s, .30 * s), (.09 * s, 2.10 * s, .28 * s)], .014 * s, mats["accent"], npc)
    elif kind == "robe":
        api.lathe(f"{npc}_Skirt", [(.07 * s, .50 * s, .42 * s), (.26 * s, .48 * s, .40 * s),
                  (.65 * s, .42 * s, .35 * s), (1.08 * s, .36 * s, .29 * s),
                  (1.17 * s, .33 * s, .27 * s)], mats["cloth"], npc, segments=12)
        cloth_panel(api, f"{npc}_RobeFront", rows([
            ((-.20, 1.17, .28), (0, 1.17, .32), (.20, 1.17, .28)),
            ((-.27, .72, .33), (0, .72, .40), (.27, .72, .33)),
            ((-.34, .12, .39), (0, .10, .49), (.34, .12, .39)),
        ]), mats["cloth_shade"], npc)
        for side in (-1, 1):
            api.add_curve(f"{npc}_RobeEdge{api.side_label(side)}",
                          [(side * .19 * s, 1.18 * s, .30 * s), (side * .26 * s, .72 * s, .34 * s),
                           (side * .34 * s, .12 * s, .40 * s)], .018 * s, mats["accent"], npc)
        api.add_curve(f"{npc}_Sash", [(-.33 * s, 1.17 * s, .14 * s),
                      (0, 1.14 * s, .34 * s), (.33 * s, 1.17 * s, .14 * s)], .05 * s, mats["accent"], npc)
        # Plain overlapping shoulder cloth, without highlighted hanging yoke
        # edges that frame the entire front like a harness.
        for side in (-1, 1):
            cloth_panel(api, f"{npc}_RobeYoke{api.side_label(side)}", rows([
                ((side * .16, 2.15, .19), (side * .27, 2.14, .19), (side * .38, 2.09, .09)),
                ((side * .19, 1.99, .27), (side * .31, 1.99, .22), (side * .42, 1.95, .11)),
                ((side * .28, 1.82, .25), (side * .37, 1.81, .18), (side * .43, 1.79, .09)),
            ]), mats["cloth_shade"], npc, .018 * s)
        cloth_panel(api, f"{npc}_RobeBack", rows([
            ((-.28, 2.14, -.17), (0, 2.15, -.34), (.28, 2.14, -.17)),
            ((-.36, 1.73, -.22), (0, 1.70, -.40), (.36, 1.73, -.22)),
            ((-.37, 1.23, -.23), (0, 1.19, -.41), (.37, 1.23, -.23)),
            ((-.37, .69, -.30), (0, .59, -.48), (.37, .69, -.30)),
        ]), mats["cloth_shade"], npc)
        api.add_curve(f"{npc}_RobeBackSeam", [(0, 2.12 * s, -.36 * s),
                      (0, 1.71 * s, -.43 * s), (0, 1.19 * s, -.44 * s),
                      (0, .61 * s, -.51 * s)], .015 * s, mats["accent"], npc)
    elif kind == "brigandine":
        # Fitted steel cuirass covers front, flanks and back; breastplates
        # sit over it as local reinforcements rather than the only armor.
        api.lathe(f"{npc}_Cuirass", [(1.02 * s, .36 * s, .27 * s),
                  (1.22 * s, .39 * s, .31 * s), (1.59 * s, .38 * s, .31 * s),
                  (1.93 * s, .44 * s, .32 * s), (2.15 * s, .34 * s, .24 * s)],
                  mats["armor_dark"], npc, segments=16)
        for side in (-1, 1):
            label = api.side_label(side)
            cloth_panel(api, f"{npc}_Breastplate{label}", rows([
                ((side * .03, 2.04, .32), (side * .19, 2.04, .32), (side * .39, 2.04, .18)),
                ((side * .04, 1.70, .34), (side * .20, 1.70, .36), (side * .38, 1.70, .24)),
                ((side * .06, 1.35, .33), (side * .20, 1.35, .35), (side * .34, 1.35, .25)),
            ]), mats["steel"], npc, .03 * s)
            api.add_curve(f"{npc}_PlateEdge{label}", [(side * .04 * s, 2.06 * s, .34 * s),
                          (side * .06 * s, 1.71 * s, .37 * s), (side * .07 * s, 1.34 * s, .35 * s)],
                          .018 * s, mats["accent"], npc)
            for height in (1.42, 1.83):
                api.add_ico(f"{npc}_Rivet{label}{int(height * 100)}", (side * .30 * s, height * s, .30 * s),
                            (.025 * s, .028 * s, .016 * s), mats["accent"], npc)
            # Overlapping shoulder plate; the upper arm itself remains slender.
            cloth_panel(api, f"{npc}_Pauldron{label}", rows([
                ((side * .34, 2.15, -.12), (side * .49, 2.17, 0), (side * .58, 2.11, .08)),
                ((side * .41, 1.99, -.13), (side * .55, 2.01, .01), (side * .62, 1.94, .10)),
                ((side * .42, 1.82, -.10), (side * .55, 1.83, .02), (side * .59, 1.79, .10)),
            ]), mats["steel"], npc, .04 * s)
        for y in (.98, 1.17):
            api.add_curve(f"{npc}_Tasset{int(y * 100)}", [(-.36 * s, y * s, .13 * s),
                          (0, (y - .02) * s, .35 * s), (.36 * s, y * s, .13 * s)],
                          .035 * s, mats["cloth"], npc)
        for side, angles in ((-1, (math.pi, 1.27 * math.pi, 1.49 * math.pi)),
                             (1, (1.51 * math.pi, 1.75 * math.pi, 2 * math.pi))):
            # Rear plates follow the cuirass contour; the strip gap reads as
            # a leather spine seam in back and reveals steel on the flank.
            verts = [(rx * s * math.cos(a), y * s, rz * s * math.sin(a))
                     for y, rx, rz in ((1.25, .41, .34), (1.70, .42, .35), (2.07, .45, .34))
                     for a in angles]
            faces = [(row * 3 + col, (row + 1) * 3 + col,
                      (row + 1) * 3 + col + 1, row * 3 + col + 1)
                     for row in range(2) for col in range(2)]
            api.mesh_object(f"{npc}_Backplate{api.side_label(side)}",
                            verts, faces, mats["steel"], npc, thickness=.025 * s, bevel=.012 * s)
            api.add_curve(f"{npc}_BackStrap{api.side_label(side)}",
                          [(side * .30 * s, 2.08 * s, -.25 * s),
                           (side * .23 * s, 1.69 * s, -.32 * s),
                           (side * .13 * s, 1.25 * s, -.34 * s)],
                          .025 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_ArmorSpine", [(0, 2.08 * s, -.35 * s),
                      (0, 1.64 * s, -.36 * s), (0, 1.24 * s, -.34 * s)],
                      .024 * s, mats["accent"], npc)
    elif kind == "vest":
        for side in (-1, 1):
            cloth_panel(api, f"{npc}_Vest{api.side_label(side)}", rows([
                ((side * .08, 2.06, .28), (side * .23, 2.08, .26), (side * .37, 2.00, .17)),
                ((side * .10, 1.55, .32), (side * .24, 1.55, .34), (side * .40, 1.51, .18)),
                ((side * .14, 1.07, .28), (side * .28, 1.03, .29), (side * .37, 1.02, .16)),
            ]), mats["cloth_shade"], npc)


def head_surface(api, name, rings, mat, npc, s, opening=0, thickness=0,
                 orbits=False):
    """Stack elliptical contours; optionally open the face or inset skull orbits."""
    sides = 16
    angles = [opening + (math.tau - 2 * opening) * i / sides
              for i in range(sides + 1)] if opening else [
                  math.tau * i / sides for i in range(sides)]
    vertices = []
    for y, rx, center, rz in rings:
        for angle in angles:
            x = rx * math.sin(angle)
            z = center + rz * math.cos(angle)
            if orbits and abs(y - 2.60) < .01 and abs(x) > .045 and z > center:
                z -= .11 * max(0, 1 - abs(abs(x) - .115) / .12)
            vertices.append((x * s, y * s, z * s))
    count = len(angles)
    columns = count - 1 if opening else count
    faces = [(row * count + col, row * count + (col + 1) % count,
              (row + 1) * count + (col + 1) % count, (row + 1) * count + col)
             for row in range(len(rings) - 1) for col in range(columns)]
    if not opening:
        faces += [tuple(reversed(range(count))),
                  tuple((len(rings) - 1) * count + i for i in range(count))]
    api.mesh_object(f"{npc}_{name}", vertices, faces, mat, npc,
                    thickness=thickness * s)


def add_study_head(npc, cfg, mats, s, api):
    """Portrait-scaled anatomy and headgear with an actual side/back silhouette."""
    kind = cfg["head"]
    if kind == "skull":
        add_skull(npc, mats, s, api)
        return
    head_surface(api, "Face", [
        (2.27, .075, .13, .11), (2.34, .16, .11, .19),
        (2.47, .22, .09, .255), (2.61, .245, .065, .28),
        (2.74, .22, .055, .27), (2.83, .135, .04, .18),
        (2.87, .015, .035, .025),
    ], mats["skin"], npc, s)
    for side in (-1, 1):
        label = api.side_label(side)
        # Small almond eyes, cheek pads and ears follow the facial contour,
        # rather than sitting forward as spherical goggles.
        api.add_ico(f"{npc}_Eye{label}", (side * .093 * s, 2.59 * s, .337 * s),
                    (.037 * s, .020 * s, .010 * s), mats["dark"], npc)
        api.add_ico(f"{npc}_Cheek{label}", (side * .158 * s, 2.48 * s, .311 * s),
                    (.056 * s, .048 * s, .020 * s), mats["skin"], npc)
        api.add_ico(f"{npc}_Ear{label}", (side * .245 * s, 2.48 * s, .07 * s),
                    (.043 * s, .078 * s, .053 * s), mats["skin"], npc)
        api.add_curve(f"{npc}_Brow{label}",
                      [(side * .035 * s, 2.666 * s, .339 * s),
                       (side * .105 * s, 2.679 * s, .327 * s),
                       (side * .171 * s, 2.652 * s, .290 * s)],
                      .014 * s, mats["skin"], npc)
    shaped_limb(api, f"{npc}_Nose",
                [(0, 2.65 * s, .32 * s), (0, 2.58 * s, .36 * s),
                 (0, 2.515 * s, .435 * s), (0, 2.485 * s, .395 * s)],
                (.025 * s, .038 * s, .05 * s, .038 * s),
                (.026 * s, .04 * s, .039 * s, .025 * s),
                mats["skin"], npc)
    api.add_ico(f"{npc}_FaceChin", (0, 2.335 * s, .264 * s),
                (.108 * s, .052 * s, .054 * s), mats["skin"], npc)
    api.add_curve(f"{npc}_Mouth", [(-.067 * s, 2.405 * s, .307 * s),
                  (0, 2.393 * s, .325 * s), (.067 * s, 2.405 * s, .307 * s)],
                  .009 * s, mats["dark"], npc)

    if kind in {"hood", "helm", "horned"}:
        armor = kind != "hood"
        shell_mat = mats["steel"] if armor else mats["cloth"]
        bottom = 2.32 if armor else 2.23
        head_surface(api, "HelmetShell" if armor else "HoodShell", [
            (bottom, .31 if armor else .29, -.025, .32 if armor else .29),
            (2.47, .31 if armor else .29, .015, .40 if armor else .35),
            (2.66, .30 if armor else .285, .025, .45 if armor else .39),
            (2.78, .245 if armor else .225, .055, .48 if armor else .40),
            (2.90, .014, -.035, .06),
        ], shell_mat, npc, s, opening=.89 if armor else 1.12, thickness=.033)
        # Top bridge rests on the forehead, leaving the face open in profile.
        api.mesh_object(f"{npc}_{'Helmet' if armor else 'Hood'}Brow",
                        [(-.17 * s, 2.78 * s, .33 * s),
                         (0, 2.805 * s, .39 * s),
                         (.17 * s, 2.78 * s, .33 * s),
                         (-.012 * s, 2.90 * s, .005 * s),
                         (.012 * s, 2.90 * s, .005 * s)],
                        [(0, 1, 3), (1, 4, 3), (1, 2, 4)],
                        shell_mat, npc, thickness=.025 * s)
        if kind == "hood":
            for side in (-1, 1):
                api.add_curve(f"{npc}_HoodFold{api.side_label(side)}",
                              [(side * .255 * s, 2.29 * s, .16 * s),
                               (side * .27 * s, 2.48 * s, .20 * s),
                               (side * .25 * s, 2.64 * s, .20 * s)],
                              .010 * s, mats["cloth_shade"], npc)
            api.add_curve(f"{npc}_HoodBackSeam", [(0, 2.86 * s, -.10 * s),
                          (0, 2.54 * s, -.33 * s), (0, 2.27 * s, -.31 * s)],
                          .011 * s, mats["cloth_shade"], npc)
        else:
            for side in (-1, 1):
                api.add_ico(f"{npc}_HelmetCheek{api.side_label(side)}",
                            (side * .257 * s, 2.44 * s, .19 * s),
                            (.049 * s, .12 * s, .085 * s), mats["steel"], npc)
            if kind == "horned":
                for side in (-1, 1):
                    shaped_limb(api, f"{npc}_Horn{api.side_label(side)}",
                                [(side * .23 * s, 2.73 * s, -.10 * s),
                                 (side * .35 * s, 2.93 * s, -.14 * s),
                                 (side * .41 * s, 3.13 * s, -.20 * s)],
                                (.075 * s, .058 * s, .006 * s),
                                (.074 * s, .05 * s, .006 * s),
                                mats["bone"], npc, smooth=False)
    elif kind == "cap":
        head_surface(api, "Cap", [(2.65, .248, .017, .27),
                     (2.76, .245, .022, .275), (2.86, .15, .016, .185),
                     (2.90, .015, .015, .025)], mats["cloth_shade"], npc, s)
        cloth_panel(api, f"{npc}_CapVisor", [
            [(-.21 * s, 2.745 * s, .16 * s), (0, 2.755 * s, .295 * s),
             (.21 * s, 2.745 * s, .16 * s)],
            [(-.17 * s, 2.72 * s, .295 * s), (0, 2.73 * s, .39 * s),
             (.17 * s, 2.72 * s, .295 * s)]],
            mats["dark"], npc, .028 * s)
    elif kind == "crown":
        points = 16
        vertices = []
        for row in range(2):
            for i in range(points):
                angle = math.tau * i / points
                vertices.append((.248 * math.sin(angle) * s,
                                 (2.74 if row == 0 else 2.88 + .09 * (i % 4 == 0)) * s,
                                 (.025 + .277 * math.cos(angle)) * s))
        faces = [(i, (i + 1) % points, points + (i + 1) % points, points + i)
                 for i in range(points)]
        api.mesh_object(f"{npc}_Crown", vertices, faces, mats["accent"], npc,
                        thickness=.035 * s)
    elif kind == "mitre":
        head_surface(api, "Mitre", [(2.67, .25, .005, .28),
                     (2.80, .27, 0, .29), (3.06, .16, -.025, .20),
                     (3.20, .01, -.04, .035)], mats["cloth_shade"], npc, s)
        api.add_curve(f"{npc}_MitreSeam", [(0, 2.79 * s, -.29 * s),
                      (0, 3.07 * s, -.225 * s), (0, 3.19 * s, -.075 * s)],
                      .013 * s, mats["accent"], npc)
    elif kind == "spines":
        head_surface(api, "Hair", [(2.38, .24, -.07, .24),
                     (2.69, .268, -.015, .30), (2.82, .19, 0, .22),
                     (2.88, .01, 0, .02)], mats["dark"], npc, s,
                     opening=.95, thickness=.022)
        for side in (-1, 1):
            for index in range(3):
                shaped_limb(api, f"{npc}_SpineQuill{api.side_label(side)}{index}",
                            [(side * (.11 + index * .065) * s, 2.73 * s, (-.16 + index * .025) * s),
                             (side * (.14 + index * .085) * s, 2.91 * s, (-.19 + index * .01) * s),
                             (side * (.19 + index * .11) * s, (3.12 - index * .07) * s, -.24 * s)],
                            (.045 * s, .033 * s, .004 * s),
                            (.046 * s, .025 * s, .004 * s), mats["bone"], npc, smooth=False)
    elif kind == "stone":
        head_surface(api, "StoneCrest", [(2.66, .24, -.025, .265),
                     (2.80, .255, -.01, .28), (2.95, .18, -.01, .18),
                     (3.00, .015, -.01, .02)], mats["cloth_shade"], npc, s)
        for side in (-1, 1):
            api.add_ico(f"{npc}_StoneCheek{api.side_label(side)}",
                        (side * .20 * s, 2.48 * s, .205 * s),
                        (.052 * s, .088 * s, .071 * s), mats["cloth_shade"], npc)


def add_skull(npc, mats, s, api):
    """Bone face with inset open orbits, projecting zygomatics and separate jaw."""
    head_surface(api, "Skull", [(2.37, .135, .085, .13),
                 (2.46, .215, .065, .22), (2.60, .27, .05, .31),
                 (2.76, .26, .035, .31), (2.85, .18, .025, .22),
                 (2.91, .015, .02, .025)], mats["bone"], npc, s, orbits=True)
    for side in (-1, 1):
        label = api.side_label(side)
        center = side * .12
        outer, inner = [], []
        for i in range(12):
            angle = math.tau * i / 12
            x = center + .098 * math.cos(angle)
            y = 2.602 + .094 * math.sin(angle)
            outer.append((x * s, y * s, (.405 - .16 * abs(x)) * s))
            x = center + .072 * math.cos(angle)
            y = 2.602 + .067 * math.sin(angle)
            inner.append((x * s, y * s, (.325 - .09 * abs(x)) * s))
        api.mesh_object(f"{npc}_EyeCavity{label}", outer + inner,
                        [(i, (i + 1) % 12, (i + 1) % 12 + 12, i + 12)
                         for i in range(12)] + [tuple(range(12, 24))],
                        mats["dark"], npc)
        api.add_curve(f"{npc}_SkullBrow{label}",
                      [(side * .035 * s, 2.705 * s, .391 * s),
                       (side * .126 * s, 2.713 * s, .395 * s),
                       (side * .219 * s, 2.657 * s, .336 * s)],
                      .034 * s, mats["bone"], npc)
        api.add_curve(f"{npc}_SkullCheek{label}",
                      [(side * .22 * s, 2.53 * s, .33 * s),
                       (side * .16 * s, 2.472 * s, .379 * s),
                       (side * .057 * s, 2.49 * s, .38 * s)],
                      .039 * s, mats["bone"], npc)
        api.add_curve(f"{npc}_SkullJawRam{label}",
                      [(side * .225 * s, 2.48 * s, .045 * s),
                       (side * .21 * s, 2.335 * s, .122 * s),
                       (side * .105 * s, 2.29 * s, .30 * s)],
                      .031 * s, mats["bone"], npc)
    api.mesh_object(f"{npc}_NoseVoid",
                    [(-.054 * s, 2.53 * s, .398 * s),
                     (.054 * s, 2.53 * s, .398 * s),
                     (0, 2.425 * s, .417 * s)],
                    [(0, 1, 2)], mats["dark"], npc, thickness=.008 * s)
    api.add_curve(f"{npc}_SkullJaw", [(-.105 * s, 2.292 * s, .30 * s),
                  (0, 2.267 * s, .325 * s), (.105 * s, 2.292 * s, .30 * s)],
                  .042 * s, mats["bone"], npc)
    for i in range(6):
        api.add_ico(f"{npc}_SkullTooth{i}",
                    ((i - 2.5) * .036 * s, 2.338 * s, .345 * s),
                    (.016 * s, .029 * s, .013 * s), mats["bone"], npc)


def identity_details(npc, mats, s, api):
    """A few silhouette-scale personal objects, not extra frontal armor."""
    if npc in {"Traveller", "Smuggler"}:
        # Cross-body strap follows front and back and disappears into cloak.
        api.add_curve(f"{npc}_SatchelStrap",
                      [(-.30 * s, 2.08 * s, .18 * s),
                       (-.12 * s, 1.75 * s, .33 * s),
                       (.29 * s, 1.27 * s, .24 * s),
                       (.38 * s, 1.12 * s, -.12 * s),
                       (.17 * s, 1.50 * s, -.44 * s)],
                      .020 * s, mats["leather"], npc)
        api.add_ico(f"{npc}_Satchel", (.38 * s, 1.13 * s, .07 * s),
                    (.12 * s, .16 * s, .10 * s), mats["leather"], npc)
    elif npc == "Bandit":
        for side in (-1, 1):
            api.add_curve(f"{npc}_LeatherLacing{api.side_label(side)}",
                          [(side * .30 * s, 1.78 * s, .31 * s),
                           (side * .35 * s, 1.58 * s, .29 * s),
                           (side * .29 * s, 1.39 * s, .30 * s)],
                          .015 * s, mats["leather"], npc)
    elif npc == "Matriarch":
        for side in (-1, 1):
            api.add_curve(f"{npc}_BroodSleeve{api.side_label(side)}",
                          [(side * .42 * s, 1.98 * s, .08 * s),
                           (side * .59 * s, 1.76 * s, .095 * s),
                           (side * .59 * s, 1.62 * s, .10 * s)],
                          .034 * s, mats["cloth_shade"], npc)
    elif npc == "Adjudicator":
        api.add_curve(f"{npc}_JudgementSeal", [(-.11 * s, 1.67 * s, .385 * s),
                      (0, 1.57 * s, .395 * s), (.11 * s, 1.67 * s, .385 * s)],
                      .018 * s, mats["accent"], npc)
    elif npc == "Companion":
        api.add_curve(f"{npc}_ShoulderStrap",
                      [(-.44 * s, 2.08 * s, .06 * s),
                       (-.26 * s, 1.77 * s, .35 * s),
                       (.22 * s, 1.19 * s, .30 * s),
                       (.38 * s, 1.19 * s, -.10 * s)],
                      .024 * s, mats["leather"], npc)
    elif npc == "Tidemother":
        for side in (-1, 1):
            api.add_curve(f"{npc}_TideHem{api.side_label(side)}",
                          [(side * .12 * s, .14 * s, .46 * s),
                           (side * .35 * s, .15 * s, .37 * s),
                           (side * .43 * s, .19 * s, .10 * s)],
                          .026 * s, mats["accent"], npc)
    elif npc == "Cragmother":
        for side in (-1, 1):
            api.add_ico(f"{npc}_HandStone{api.side_label(side)}",
                        (side * (.51 if side < 0 else .65) * s, 1.23 * s, .11 * s),
                        (.093 * s, .095 * s, .09 * s), mats["cloth_shade"], npc)
    elif npc == "Tollmaster":
        for index in range(3):
            api.add_curve(f"{npc}_Tally{index}",
                          [(-.22 * s, (1.49 + .075 * index) * s, .35 * s),
                           (-.10 * s, (1.47 + .075 * index) * s, .37 * s)],
                          .013 * s, mats["accent"], npc)
    elif npc == "Alchemist":
        api.add_curve(f"{npc}_VialHarness",
                      [(-.35 * s, 1.33 * s, .10 * s),
                       (0, 1.31 * s, .35 * s),
                       (.35 * s, 1.33 * s, .10 * s)],
                      .029 * s, mats["leather"], npc)
    elif npc == "OathlessCurate":
        api.add_curve(f"{npc}_ScriptBack",
                      [(0, 2.03 * s, -.39 * s),
                       (0, 1.64 * s, -.44 * s),
                       (0, 1.21 * s, -.43 * s)],
                      .023 * s, mats["accent"], npc)


def add_humanoid_body(npc, cfg, mats, api):
    s = cfg["scale"]
    undead = cfg["form"] == "undead"
    if not undead:
        mats["leather"] = api.base.material(f"{npc}_StudyLeather", api.rgba("#675039"), 0.02, 0.82)
        mats["linen"] = api.base.material(f"{npc}_StudyLinen", api.rgba("#A89B7F"), 0.0, 0.85)
        mats["armor_dark"] = api.base.material(f"{npc}_StudySteelShadow", api.rgba("#495765"), 0.42, 0.58)
    if undead:
        # The dark inner spine recedes; bone ribs must read on the back as well.
        # Only the spinal cavity is dark; the rib cage stays open front/back.
        api.lathe(f"{npc}_Torso", [(1.03 * s, .064 * s, .065 * s),
                  (1.42 * s, .060 * s, .065 * s),
                  (1.94 * s, .070 * s, .067 * s),
                  (2.20 * s, .072 * s, .070 * s)],
                  mats["dark"], npc, segments=10)
        for side in (-1, 1):
            api.add_curve(f"{npc}_Pelvis{api.side_label(side)}",
                          [(0, 1.13 * s, .12 * s), (side * .26 * s, 1.02 * s, .12 * s),
                           (side * .27 * s, 1.12 * s, -.11 * s), (0, 1.18 * s, -.16 * s)],
                          .055 * s, mats["bone"], npc)
        api.add_curve(f"{npc}_Sternum", [(0, 2.07 * s, .14 * s), (0, 1.75 * s, .18 * s),
                      (0, 1.45 * s, .17 * s)], .045 * s, mats["bone"], npc)
        api.add_curve(f"{npc}_VertebraeRear", [(0, 2.20 * s, -.14 * s),
                      (0, 1.72 * s, -.16 * s), (0, 1.14 * s, -.15 * s)],
                      .046 * s, mats["bone"], npc)
        for row, height in enumerate((1.48, 1.65, 1.82, 1.99)):
            for side in (-1, 1):
                api.add_curve(f"{npc}_Rib{row}{api.side_label(side)}",
                              [(0, height * s, .18 * s),
                               (side * .20 * s, (height + .045) * s, .20 * s),
                               (side * .31 * s, (height + .09) * s, .015 * s),
                               (side * .21 * s, (height + .05) * s, -.185 * s),
                               (0, (height + .02) * s, -.17 * s)],
                              .026 * s, mats["bone"], npc)
    else:
        api.lathe(f"{npc}_Torso", [(.89 * s, .31 * s, .24 * s), (1.15 * s, .36 * s, .27 * s),
                  (1.40 * s, .32 * s, .25 * s), (1.76 * s, .40 * s, .29 * s),
                  (2.02 * s, .43 * s, .29 * s), (2.18 * s, .27 * s, .21 * s),
                  (2.28 * s, .13 * s, .13 * s)], mats["cloth"], npc, segments=12)
        garment(npc, cfg, mats, s, api)
        api.add_curve(f"{npc}_BeltFront", [(-.35 * s, 1.14 * s, .14 * s),
                      (0, 1.13 * s, .31 * s), (.35 * s, 1.14 * s, .14 * s)],
                      .035 * s, mats["dark"], npc)
    add_study_head(npc, cfg, mats, s, api)

    arms = {"L": ((-.38 * s, 2.02 * s, 0), (-.56 * s, 1.72 * s, .02 * s),
                  (-.50 * s, 1.38 * s, .08 * s), (-.50 * s, 1.22 * s, .10 * s)),
            "R": ((.38 * s, 2.02 * s, 0), (.58 * s, 1.72 * s, .02 * s),
                  (.62 * s, 1.40 * s, .08 * s), (.64 * s, 1.24 * s, .10 * s))}
    for label, (shoulder, elbow, wrist, hand) in arms.items():
        # A single manifold sleeve crosses both bones; vertices around the
        # elbow receive blended weights rather than two rigid capped cylinders.
        joints = (between(shoulder, elbow, -.16), between(shoulder, elbow, .43),
                  between(shoulder, elbow, .79), elbow, between(elbow, wrist, .23),
                  between(elbow, wrist, .65), between(wrist, hand, .80))
        widths = ((.098, .109, .075, .070, .063, .085, .061) if undead else
                  (.15, .14, .115, .105, .10, .115, .078))
        shaped_limb(api, f"{npc}_ArmSleeve{label}", joints,
                    tuple(w * s for w in widths), tuple(w * .85 * s for w in widths),
                    mats["bone"] if undead else mats["cloth"], npc, smooth=not undead)
        if undead:
            shaped_limb(api, f"{npc}_HandPalm{label}",
                        ((hand[0], 1.25 * s, hand[2]),
                         (hand[0], 1.15 * s, hand[2] + .02 * s),
                         (hand[0], 1.11 * s, hand[2] + .035 * s)),
                        (.058 * s, .070 * s, .053 * s),
                        (.035 * s, .035 * s, .025 * s), mats["bone"], npc, smooth=False)
            for digit in (-1, 0, 1):
                api.add_curve(f"{npc}_HandDigit{digit + 1}{label}",
                              [(hand[0] + digit * .042 * s, 1.14 * s, hand[2] + .04 * s),
                               (hand[0] + digit * .049 * s, (1.00 + .035 * abs(digit)) * s,
                                hand[2] + .079 * s)],
                              .013 * s, mats["bone"], npc)
        else:
            shaped_limb(api, f"{npc}_Hand{label}",
                        ((hand[0], hand[1] + .08 * s, hand[2]),
                         (hand[0], hand[1], hand[2] + .015 * s),
                         (hand[0], hand[1] - .085 * s, hand[2] + .027 * s)),
                        (.060 * s, .076 * s, .062 * s),
                        (.040 * s, .049 * s, .037 * s), mats["skin"], npc)
            sign = -1 if label == "L" else 1
            api.add_ico(f"{npc}_HandThumb{label}",
                        (hand[0] - sign * .070 * s, hand[1] + .015 * s, hand[2] + .04 * s),
                        (.033 * s, .064 * s, .035 * s), mats["skin"], npc)
    legs = {"L": ((-.20 * s, 1.05 * s, 0), (-.25 * s, .58 * s, .02 * s), (-.27 * s, .10 * s, .05 * s)),
            "R": ((.20 * s, 1.05 * s, 0), (.25 * s, .58 * s, .02 * s), (.27 * s, .10 * s, .05 * s))}
    for label, (hip, knee, ankle) in legs.items():
        joints = (between(hip, knee, -.08), between(hip, knee, .44),
                  between(hip, knee, .83), knee, between(knee, ankle, .22),
                  between(knee, ankle, .65), between(knee, ankle, 1.04))
        widths = ((.115, .102, .072, .085, .069, .084, .061) if undead else
                  (.175, .17, .135, .12, .105, .11, .09))
        shaped_limb(api, f"{npc}_Legging{label}", joints,
                    tuple(w * s for w in widths), tuple(w * .82 * s for w in widths),
                    mats["bone"] if undead else mats["cloth"], npc, smooth=not undead)
        x = ankle[0]
        shaped_limb(api, f"{npc}_Boot{label}", ((x, .19 * s, .03 * s),
                    (x, .08 * s, .09 * s), (x, .035 * s, .11 * s)),
                    ((.072 if undead else .11) * s, (.12 if undead else .15) * s,
                     (.13 if undead else .15) * s),
                    ((.075 if undead else .10) * s, (.13 if undead else .20) * s,
                     (.16 if undead else .20) * s),
                    mats["dark"] if not undead else mats["bone"], npc)
    if not undead:
        identity_details(npc, mats, s, api)
    return arms, legs


def add_weapon(npc, cfg, mats, s, api):
    api.add_weapon(npc, cfg, mats, s)
    kind = cfg["weapon"]
    x = .68 * s
    def replace(*parts):
        for part in parts:
            old = api.bpy.data.objects.get(f"{npc}_{part}")
            if old: api.bpy.data.objects.remove(old, do_unlink=True)

    if kind in {"dagger", "knife", "sword", "swordshield"}:
        replace("WeaponShaft", "WeaponBlade", "WeaponGuard")
        small_blade = kind in {"dagger", "knife"}
        blade_length = (.48 if small_blade else .99) * s
        width = (.10 if small_blade else .15) * s
        base_y = 1.64 * s
        # Broad shoulders, narrow point and a shallow central ridge catch
        # light from three-quarter angles; the original was a flat triangle.
        verts = [(x + dx * width, base_y + fraction * blade_length, z * s)
                 for dx, fraction, z in ((-1, 0, .10), (-1.12, .18, .10),
                                         (-.76, .76, .10), (0, 1, .10),
                                         (.76, .76, .10), (1.12, .18, .10), (1, 0, .10))]
        api.mesh_object(f"{npc}_WeaponBlade", verts, [tuple(range(len(verts)))],
                        mats["steel"], npc, thickness=.055 * s, bevel=.012 * s)
        api.add_curve(f"{npc}_WeaponFuller", [(x, 1.69 * s, .146 * s),
                      (x, (1.64 * s + blade_length * .72), .146 * s)],
                      .010 * s, mats["bone"], npc)
        grip_bottom = 1.34 if small_blade else 1.13
        api.add_curve(f"{npc}_WeaponGrip", [(x, grip_bottom * s, .10 * s),
                      (x, 1.63 * s, .10 * s)], .038 * s,
                      mats["leather"] if kind != "sword" else mats["dark"], npc)
        heights = (1.41, 1.53) if small_blade else (1.23, 1.39, 1.54)
        for index, height in enumerate(heights):
            api.add_curve(f"{npc}_WeaponGripWrap{index}",
                          [(x - .035 * s, (height - .03) * s, .12 * s),
                           (x + .035 * s, (height + .03) * s, .12 * s)],
                          .012 * s, mats["accent"], npc)
        guard = .14 if small_blade else .19
        api.add_curve(f"{npc}_WeaponCrossguard", [(x - guard * s, 1.62 * s, .10 * s),
                      (x, 1.66 * s, .10 * s), (x + guard * s, 1.62 * s, .10 * s)],
                      .038 * s, mats["steel"], npc)
        api.add_ico(f"{npc}_WeaponPommel", (x, grip_bottom * s, .10 * s),
                    (.05 * s, .06 * s, .05 * s), mats["steel"], npc)
    elif kind in {"staff", "ringstaff", "kelp"}:
        replace("Staff", "StaffTop", "StaffRing")
        # Rake the upper shaft forward, clear of the head's strict side view.
        # Its grip still passes through the existing weapon-bone handle.
        def staff_z(y):
            return (.10 + .46 * max(0, min(1, (y - 1.29) / 1.18))) * s
        api.add_curve(f"{npc}_Staff", [(x, .14 * s, .10 * s),
                      (x - .03 * s, 1.29 * s, .10 * s),
                      (x, 2.47 * s, staff_z(2.47))],
                      .045 * s, mats["leather"], npc)
        for index, height in enumerate((.35, 1.35, 2.35)):
            api.add_torus(f"{npc}_StaffBand{index}", (x, height * s, staff_z(height)),
                          .047 * s, .012 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
        tip_z = staff_z(2.47)
        if kind == "ringstaff":
            for side in (-1, 1):
                api.add_curve(f"{npc}_StaffRingSocket{api.side_label(side)}",
                              [(x, 2.37 * s, staff_z(2.37)),
                               (x + side * .14 * s, 2.46 * s, tip_z)],
                              .027 * s, mats["accent"], npc)
            api.add_curve(f"{npc}_StaffRingSetting",
                          [(x - .14 * s, 2.46 * s, tip_z),
                           (x - .14 * s, 2.67 * s, tip_z),
                           (x, 2.78 * s, tip_z),
                           (x + .14 * s, 2.67 * s, tip_z),
                           (x + .14 * s, 2.46 * s, tip_z)],
                          .025 * s, mats["accent"], npc)
            api.add_ico(f"{npc}_StaffCrystal", (x, 2.60 * s, tip_z),
                        (.075 * s, .105 * s, .075 * s), mats["glow"], npc)
        elif kind == "kelp":
            for side in (-1, 1):
                api.add_curve(f"{npc}_StaffFrond{api.side_label(side)}",
                              [(x, 2.30 * s, staff_z(2.30)),
                               (x + side * .13 * s, 2.44 * s, tip_z),
                               (x + side * .19 * s, 2.68 * s, tip_z)],
                              .028 * s, mats["accent"], npc)
            api.add_ico(f"{npc}_StaffKnob", (x, 2.50 * s, tip_z),
                        (.075 * s, .10 * s, .072 * s), mats["cloth"], npc)
        else:
            api.add_curve(f"{npc}_StaffFork",
                          [(x - .085 * s, 2.43 * s, staff_z(2.43)),
                           (x, 2.58 * s, tip_z),
                           (x + .085 * s, 2.43 * s, staff_z(2.43))],
                          .025 * s, mats["leather"], npc)
            api.add_ico(f"{npc}_StaffKnob", (x, 2.57 * s, tip_z),
                        (.063 * s, .075 * s, .065 * s), mats["accent"], npc)
    elif kind == "greataxe":
        replace("AxeHead")
        verts = [(x + dx * s, y * s, .11 * s) for dx, y in (
            (-.07, 2.35), (.12, 2.35), (.50, 2.53), (.39, 2.26),
            (.41, 1.94), (.49, 1.73), (.10, 1.90), (-.07, 1.91))]
        api.mesh_object(f"{npc}_AxeBlade", verts, [(0, 1, 2, 3, 4, 5, 6, 7)],
                        mats["steel"], npc, thickness=.075 * s, bevel=.015 * s)
        api.add_curve(f"{npc}_AxeEdge", [(x + .50 * s, 2.53 * s, .14 * s),
                      (x + .41 * s, 2.27 * s, .14 * s),
                      (x + .41 * s, 1.95 * s, .14 * s),
                      (x + .49 * s, 1.73 * s, .14 * s)],
                      .018 * s, mats["bone"], npc)
        api.add_curve(f"{npc}_AxeSocket", [(x - .08 * s, 2.32 * s, .15 * s),
                      (x + .13 * s, 2.31 * s, .15 * s),
                      (x + .13 * s, 1.95 * s, .15 * s),
                      (x - .08 * s, 1.95 * s, .15 * s)],
                      .023 * s, mats["armor_dark"], npc)
        for height in (2.04, 2.22):
            api.add_ico(f"{npc}_AxeRivet{int(height * 100)}",
                        (x + .04 * s, height * s, .17 * s),
                        (.022 * s, .025 * s, .014 * s), mats["accent"], npc)
        for index, height in enumerate((1.13, 1.24, 1.35)):
            api.add_curve(f"{npc}_AxeGripWrap{index}",
                          [(x - .048 * s, height * s, .13 * s),
                           (x + .048 * s, (height + .06) * s, .13 * s)],
                          .012 * s, mats["leather"], npc)
    elif kind == "gavel":
        replace("GavelShaft", "GavelHead")
        api.add_curve(f"{npc}_GavelShaft", [(x, .67 * s, .10 * s),
                      (x, 2.23 * s, .10 * s)], .055 * s, mats["leather"], npc)
        # Octagonal barrel with chamfered ends instead of a single cuboid.
        rings = ((-.33, .105), (-.28, .15), (.28, .15), (.33, .105))
        vertices = [(x + along * s, (2.225 + radius * math.sin(math.tau * i / 8)) * s,
                     (.10 + radius * math.cos(math.tau * i / 8)) * s)
                    for along, radius in rings for i in range(8)]
        faces = [(row * 8 + i, row * 8 + (i + 1) % 8,
                  (row + 1) * 8 + (i + 1) % 8, (row + 1) * 8 + i)
                 for row in range(len(rings) - 1) for i in range(8)]
        faces += [tuple(reversed(range(8))), tuple(24 + i for i in range(8))]
        api.mesh_object(f"{npc}_GavelHead", vertices, faces, mats["accent"], npc)
        for side in (-1, 1):
            api.add_ico(f"{npc}_GavelEnd{api.side_label(side)}",
                        (x + side * .34 * s, 2.225 * s, .10 * s),
                        (.019 * s, .125 * s, .098 * s), mats["steel"], npc)
    elif kind == "flail":
        replace("FlailChain", "FlailWeight")
        for link in range(5):
            api.add_torus(f"{npc}_FlailLink{link}",
                          (x + (.035 + link * .057) * s,
                           (2.09 + link * .072) * s, .10 * s),
                          .056 * s, .012 * s, mats["steel"], npc,
                          (0, math.pi / 2 if link % 2 else 0, 0))
        api.add_ico(f"{npc}_FlailWeight", (x + .32 * s, 2.50 * s, .10 * s),
                    (.145 * s, .15 * s, .145 * s), mats["steel"], npc)
        for index in range(6):
            angle = math.tau * index / 6
            shaped_limb(api, f"{npc}_FlailSpike{index}",
                        [(x + (.32 + .12 * math.cos(angle)) * s, 2.50 * s,
                          (.10 + .12 * math.sin(angle)) * s),
                         (x + (.32 + .21 * math.cos(angle)) * s, 2.53 * s,
                          (.10 + .20 * math.sin(angle)) * s)],
                        (.045 * s, .005 * s), (.045 * s, .005 * s),
                        mats["steel"], npc, smooth=False)
    elif kind == "page":
        replace("Page")
        # Two bent leaves share a spine and show an open profile from behind.
        for side in (-1, 1):
            api.mesh_object(f"{npc}_PageLeaf{api.side_label(side)}",
                            [(x, 1.35 * s, .10 * s),
                             (x, 1.82 * s, .10 * s),
                             (x + side * .32 * s, 1.87 * s, .055 * s),
                             (x + side * .29 * s, 1.40 * s, .16 * s)],
                            [(0, 1, 2, 3)], mats["bone"], npc,
                            thickness=.026 * s)
        api.add_curve(f"{npc}_PageSpine", [(x, 1.34 * s, .10 * s),
                      (x, 1.84 * s, .10 * s)], .025 * s, mats["leather"], npc)
    elif kind == "vials":
        for index, dx in enumerate((-.12, 0, .12)):
            replace(f"Vial{index}")
            api.add_ico(f"{npc}_Vial{index}", (dx * s, 1.22 * s, .35 * s),
                        (.046 * s, .096 * s, .046 * s),
                        mats["glow"] if index == 1 else mats["accent"], npc)
            api.add_ico(f"{npc}_VialStopper{index}", (dx * s, 1.33 * s, .35 * s),
                        (.027 * s, .03 * s, .027 * s), mats["leather"], npc)
    elif kind == "pack":
        replace("Pack")
        vertices = [(dx * s, y * s, z * s)
                    for dx, y, z in ((-.37, 1.93, -.30), (.04, 1.93, -.30),
                                      (.04, 1.93, -.57), (-.37, 1.93, -.57),
                                      (-.37, 1.21, -.30), (.04, 1.21, -.30),
                                      (.04, 1.21, -.57), (-.37, 1.21, -.57))]
        api.mesh_object(f"{npc}_Pack", vertices,
                        [(0, 1, 2, 3), (4, 7, 6, 5), (0, 4, 5, 1),
                         (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)],
                        mats["leather"], npc, bevel=.04 * s)
        for side in (-1, 1):
            api.add_curve(f"{npc}_PackStrap{api.side_label(side)}",
                          [(side * .26 * s, 2.05 * s, -.13 * s),
                           (side * .27 * s, 1.64 * s, -.34 * s),
                           (side * .25 * s, 1.25 * s, -.35 * s)],
                          .026 * s, mats["linen"], npc)
    elif kind in {"claws", "fists"}:
        for side in (-1, 1):
            label = api.side_label(side)
            for index in range(3):
                replace(f"Claw{label}{index}")
                if kind == "claws":
                    x0 = (-.50 if side < 0 else .64) + (index - 1) * .045
                    shaped_limb(api, f"{npc}_HandClaw{index}{label}",
                                [(x0 * s, 1.21 * s, .155 * s),
                                 ((x0 + .012 * side) * s, 1.13 * s, .215 * s),
                                 ((x0 + .022 * side) * s, 1.04 * s, .25 * s)],
                                (.029 * s, .020 * s, .003 * s),
                                (.026 * s, .016 * s, .003 * s),
                                mats["bone"], npc, smooth=False)
