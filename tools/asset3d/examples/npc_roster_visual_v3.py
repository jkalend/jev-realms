"""Opt-in, non-runtime humanoid art overhaul (v3) for the Blender NPC roster.

High-fidelity archetype-specific anatomy, headgear, costuming, and props.
Provides:
  - Eliminated shared 'potato face' in favor of distinct facial contours and headgear
  - Authentic medieval headwear (chaperon, merchant hat, deep cowls, great helms, mitres)
  - Character-defining props (Vendor loaded pack, Adjudicator War-Maul, Curate unholy codex, etc.)
  - Distinct silhouettes at 96x96 isometric view
"""

from __future__ import annotations

import math
from character_surface import folded_panel


def shaped_limb(api, name, joints, widths, depths, material, npc, smooth=True):
    """One continuous tapered volume with elliptical cross-sections."""
    vertices = []
    sides = 12
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
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    metal = bsdf.inputs["Metallic"].default_value >= .4 if bsdf else False
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


def head_surface(api, name, rings, mat, npc, s, opening=0, thickness=0, orbits=False):
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
    return api.mesh_object(f"{npc}_{name}", vertices, faces, mat, npc, thickness=thickness * s)


def add_study_head(npc, cfg, mats, s, api):
    """Refined head proportions and archetype-specific headwear/visage."""
    kind = cfg["head"]
    if kind == "skull":
        add_skull(npc, mats, s, api)
        return

    # 1. Base head surface - refined facial profile
    mat_head = mats["accent"] if npc == "Adjudicator" else (mats["cloth_shade"] if npc == "Cragmother" else mats["skin"])
    head_surface(api, "Face", [
        (2.28, .075, .12, .10),
        (2.35, .15, .10, .17),
        (2.48, .21, .08, .23),
        (2.61, .23, .06, .26),
        (2.74, .21, .05, .25),
        (2.83, .13, .04, .17),
        (2.87, .015, .035, .025),
    ], mat_head, npc, s)

    # 2. Refined eyes, brows and cheek contours
    if npc == "Adjudicator":
        # Unified sculpted gold death mask features directly integrated into the golden head surface
        shaped_limb(api, f"{npc}_GoldNose",
                    [(0, 2.65 * s, .305 * s),
                     (0, 2.58 * s, .335 * s),
                     (0, 2.52 * s, .365 * s),
                     (0, 2.49 * s, .345 * s)],
                    (.016 * s, .022 * s, .024 * s, .018 * s),
                    (.016 * s, .023 * s, .024 * s, .016 * s),
                    mats["accent"], npc)
        api.add_ico(f"{npc}_GoldChin", (0, 2.345 * s, .245 * s),
                    (.085 * s, .042 * s, .045 * s), mats["accent"], npc)
        api.add_curve(f"{npc}_GoldMouth", [(-.055 * s, 2.415 * s, .285 * s),
                      (0, 2.405 * s, .300 * s), (.055 * s, 2.415 * s, .285 * s)],
                      .010 * s, mats["accent"], npc)
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_torus(f"{npc}_TempleRosette{lbl}", (side * .225 * s, 2.54 * s, .07 * s),
                          .045 * s, .012 * s, mats["accent"], npc, (0, math.pi / 2, 0))

    elif npc == "Cragmother":
        # Living granite monolith face: deep rock eye sockets, burning magma eyes, carved rock nose & mouth fissure
        for side in (-1, 1):
            lbl = api.side_label(side)
            # Deep chiseled eye socket in granite
            api.add_ico(f"{npc}_EyeSocket{lbl}", (side * .095 * s, 2.60 * s, .310 * s),
                        (.046 * s, .034 * s, .020 * s), mats["dark"], npc)
            # Burning molten magma eye glowing from within the rock
            api.add_ico(f"{npc}_MagmaEye{lbl}", (side * .095 * s, 2.60 * s, .325 * s),
                        (.028 * s, .020 * s, .014 * s), mats["glow"], npc)
            # Stepped rock cheek stratum
            api.add_ico(f"{npc}_StoneCheek{lbl}", (side * .160 * s, 2.50 * s, .285 * s),
                        (.055 * s, .045 * s, .035 * s), mats["cloth_shade"], npc)
            # Heavy monolithic stone brow ledge
            api.add_curve(f"{npc}_StoneEyebrow{lbl}",
                          [(side * .035 * s, 2.685 * s, .335 * s),
                           (side * .115 * s, 2.695 * s, .325 * s),
                           (side * .185 * s, 2.660 * s, .285 * s)],
                          .022 * s, mats["cloth_shade"], npc)
        # Chiseled granite nose monolith
        shaped_limb(api, f"{npc}_StoneNose",
                    [(0, 2.66 * s, .310 * s),
                     (0, 2.58 * s, .350 * s),
                     (0, 2.51 * s, .385 * s),
                     (0, 2.47 * s, .360 * s)],
                    (.026 * s, .034 * s, .038 * s, .028 * s),
                    (.024 * s, .032 * s, .036 * s, .026 * s),
                    mats["cloth_shade"], npc, smooth=False)
        # Glowing volcanic fissure mouth
        api.add_curve(f"{npc}_MagmaMouth", [
            (-.080 * s, 2.420 * s, .305 * s),
            (-.025 * s, 2.405 * s, .330 * s),
            (.030 * s, 2.415 * s, .325 * s),
            (.080 * s, 2.420 * s, .305 * s)
        ], .016 * s, mats["glow"], npc)
        # Angular rock chin shelf
        api.add_ico(f"{npc}_StoneChin", (0, 2.335 * s, .255 * s),
                    (.105 * s, .052 * s, .065 * s), mats["cloth_shade"], npc)

    else:
        for side in (-1, 1):
            label = api.side_label(side)
            # Eyes: refined almond beads sitting naturally in socket plane
            api.add_ico(f"{npc}_Eye{label}", (side * .088 * s, 2.60 * s, .315 * s),
                        (.030 * s, .018 * s, .010 * s), mats["dark"], npc)
            # Subtle cheek contour - not bulbous hamster cheeks
            api.add_ico(f"{npc}_Cheek{label}", (side * .145 * s, 2.50 * s, .285 * s),
                        (.038 * s, .035 * s, .014 * s), mats["skin"], npc)
            api.add_ico(f"{npc}_Ear{label}", (side * .235 * s, 2.49 * s, .06 * s),
                        (.035 * s, .065 * s, .045 * s), mats["skin"], npc)
            api.add_curve(f"{npc}_Brow{label}",
                          [(side * .030 * s, 2.665 * s, .320 * s),
                           (side * .095 * s, 2.675 * s, .310 * s),
                           (side * .155 * s, 2.650 * s, .275 * s)],
                          .012 * s, mats["dark"] if npc in {"Bandit", "Thief"} else mats["skin"], npc)

        # Refined nose bridge: sleek, natural taper rather than giant clown potato
        shaped_limb(api, f"{npc}_Nose",
                    [(0, 2.65 * s, .305 * s),
                     (0, 2.58 * s, .335 * s),
                     (0, 2.52 * s, .370 * s),
                     (0, 2.49 * s, .350 * s)],
                    (.016 * s, .022 * s, .026 * s, .020 * s),
                    (.016 * s, .023 * s, .024 * s, .016 * s),
                    mats["skin"], npc)

        api.add_ico(f"{npc}_FaceChin", (0, 2.345 * s, .245 * s),
                    (.085 * s, .042 * s, .045 * s), mats["skin"], npc)
        api.add_curve(f"{npc}_Mouth", [(-.055 * s, 2.415 * s, .285 * s),
                      (0, 2.405 * s, .300 * s), (.055 * s, 2.415 * s, .285 * s)],
                      .008 * s, mats["dark"], npc)

    # 3. Specialized Archetype Visage & Headwear
    if npc == "Thief":
        # Deep shadow cowl with face-mask covering lower face
        head_surface(api, "HoodShell", [
            (2.21, .31, -.025, .32),
            (2.45, .32, .015, .41),
            (2.68, .31, .035, .48),
            (2.81, .25, .065, .50),
            (2.93, .015, -.035, .07),
        ], mats["cloth_shade"], npc, s, opening=0.92, thickness=.035)
        # Pull-up dark cloth half-mask concealing lower face and chin
        api.mesh_object(f"{npc}_FaceMask", [
            (-.18 * s, 2.22 * s, .22 * s), (.18 * s, 2.22 * s, .22 * s),
            (.16 * s, 2.56 * s, .34 * s), (0, 2.58 * s, .38 * s), (-.16 * s, 2.56 * s, .34 * s)
        ], [(0, 1, 2, 3, 4)], mats["dark"], npc, thickness=.020 * s)
        # Forward cowl brow shadowing eyes
        api.mesh_object(f"{npc}_HoodBrow", [
            (-.17 * s, 2.76 * s, .35 * s), (0, 2.80 * s, .42 * s), (.17 * s, 2.76 * s, .35 * s),
            (-.012 * s, 2.90 * s, .01 * s), (.012 * s, 2.90 * s, .01 * s)
        ], [(0, 1, 3), (1, 4, 3), (1, 2, 4)], mats["cloth_shade"], npc, thickness=.025 * s)
        api.add_curve(f"{npc}_CowlTail", [(0, 2.78 * s, -.18 * s), (0, 2.45 * s, -.24 * s), (.04 * s, 2.12 * s, -.25 * s)], .035 * s, mats["cloth_shade"], npc)

    elif npc == "Bandit":
        # Ragged cut hood over iron brow with half-mask scarf
        head_surface(api, "HoodShell", [
            (2.24, .30, -.02, .30),
            (2.47, .30, .02, .38),
            (2.67, .29, .03, .43),
            (2.80, .23, .05, .43),
            (2.91, .014, -.03, .06),
        ], mats["cloth_shade"], npc, s, opening=1.05, thickness=.032)
        # Lower face bandit bandana / scarf
        api.mesh_object(f"{npc}_BanditMask", [
            (-.17 * s, 2.34 * s, .24 * s), (.17 * s, 2.34 * s, .24 * s),
            (.15 * s, 2.52 * s, .34 * s), (0, 2.54 * s, .37 * s), (-.15 * s, 2.52 * s, .34 * s),
            (0, 2.22 * s, .30 * s)
        ], [(0, 1, 2, 3, 4), (0, 4, 5), (1, 5, 2)], mats["cloth"], npc, thickness=.018 * s)
        # Iron brow reinforcement band
        api.add_curve(f"{npc}_IronBrowBand", [
            (-.22 * s, 2.68 * s, .20 * s), (0, 2.70 * s, .36 * s), (.22 * s, 2.68 * s, .20 * s)
        ], .016 * s, mats["steel"], npc)
        api.add_ico(f"{npc}_BandanaKnot", (0, 2.52 * s, -.24 * s), (.045 * s, .045 * s, .030 * s), mats["dark"], npc)
        api.add_curve(f"{npc}_BandanaTailL", [(0, 2.52 * s, -.24 * s), (-.08 * s, 2.35 * s, -.26 * s)], .016 * s, mats["dark"], npc)
        api.add_curve(f"{npc}_BandanaTailR", [(0, 2.52 * s, -.24 * s), (.07 * s, 2.30 * s, -.26 * s)], .016 * s, mats["dark"], npc)

    elif npc == "Traveller":
        # Hood with long trailing shoulder liripipe and short travel beard
        head_surface(api, "HoodShell", [
            (2.23, .30, -.025, .30),
            (2.47, .30, .015, .38),
            (2.67, .29, .025, .42),
            (2.79, .23, .055, .42),
            (2.91, .014, -.035, .06),
        ], mats["cloth"], npc, s, opening=1.10, thickness=.032)
        # Long liripipe cloth tail trailing over the shoulder
        api.add_curve(f"{npc}_Liripipe", [
            (0, 2.88 * s, -.08 * s),
            (-.14 * s, 2.65 * s, -.25 * s),
            (-.26 * s, 2.30 * s, -.18 * s),
            (-.32 * s, 1.95 * s, .05 * s)
        ], .032 * s, mats["cloth_shade"], npc)
        # Trimmed travel beard
        api.mesh_object(f"{npc}_TravelBeard", [
            (-.14 * s, 2.45 * s, .22 * s), (0, 2.43 * s, .27 * s), (.14 * s, 2.45 * s, .22 * s),
            (.10 * s, 2.28 * s, .27 * s), (0, 2.24 * s, .31 * s), (-.10 * s, 2.28 * s, .27 * s)
        ], [(0, 1, 4, 5), (1, 2, 3, 4)], mats["dark"], npc, thickness=.025 * s)

    elif npc == "Oracle":
        # Mystical cowl with sheer silk veil and glowing Third Eye
        head_surface(api, "HoodShell", [
            (2.22, .29, -.025, .30),
            (2.46, .29, .015, .37),
            (2.66, .28, .025, .41),
            (2.78, .22, .055, .41),
            (2.90, .014, -.035, .06),
        ], mats["cloth_shade"], npc, s, opening=1.12, thickness=.030)
        # Translucent silk ritual veil across lower face
        cloth_panel(api, f"{npc}_OracleVeil", [
            [(-.16 * s, 2.50 * s, .28 * s), (0, 2.52 * s, .35 * s), (.16 * s, 2.50 * s, .28 * s)],
            [(-.17 * s, 2.32 * s, .24 * s), (0, 2.30 * s, .32 * s), (.17 * s, 2.32 * s, .24 * s)],
            [(-.14 * s, 2.15 * s, .22 * s), (0, 2.12 * s, .28 * s), (.14 * s, 2.15 * s, .22 * s)]
        ], mats["linen"], npc, thickness=.012 * s)
        # Glowing Third-Eye forehead glyph
        api.add_ico(f"{npc}_ThirdEyeGlyph", (0, 2.72 * s, .305 * s),
                    (.025 * s, .040 * s, .015 * s), mats["glow"], npc)

    elif npc == "Alchemist":
        # Cowl with brass-rimmed ocular magnifying goggles and leather filter respirator
        head_surface(api, "HoodShell", [
            (2.23, .29, -.025, .29),
            (2.47, .29, .015, .37),
            (2.66, .28, .025, .41),
            (2.78, .22, .055, .41),
            (2.90, .014, -.035, .06),
        ], mats["cloth_shade"], npc, s, opening=1.08, thickness=.032)
        # Flipped-up brass ocular goggles on forehead
        api.add_curve(f"{npc}_GogglesStrap", [
            (-.24 * s, 2.73 * s, .05 * s), (0, 2.76 * s, .33 * s), (.24 * s, 2.73 * s, .05 * s)
        ], .014 * s, mats["leather"], npc)
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_torus(f"{npc}_GogglesRim{lbl}", (side * .085 * s, 2.76 * s, .31 * s),
                          .045 * s, .010 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
            api.add_ico(f"{npc}_GogglesLens{lbl}", (side * .085 * s, 2.76 * s, .31 * s),
                        (.036 * s, .036 * s, .010 * s), mats["glow"] if side < 0 else mats["accent"], npc)
        # Leather canister respirator over lower face
        api.add_ico(f"{npc}_RespiratorCup", (0, 2.42 * s, .31 * s),
                    (.095 * s, .075 * s, .085 * s), mats["leather"], npc)
        api.add_ico(f"{npc}_RespiratorFilter", (0, 2.39 * s, .39 * s),
                    (.045 * s, .045 * s, .035 * s), mats["steel"], npc)

    elif npc == "Chief":
        # Blackened iron war-helm with swept ram horns and thick barbarian beard
        head_surface(api, "HelmetShell", [
            (2.32, .32, -.02, .33),
            (2.48, .32, .02, .41),
            (2.68, .31, .03, .46),
            (2.80, .25, .06, .49),
            (2.93, .015, -.03, .07),
        ], mats["armor_dark"], npc, s, opening=0.88, thickness=.038)
        # Brow plate
        api.mesh_object(f"{npc}_HelmetBrow", [
            (-.18 * s, 2.76 * s, .33 * s), (0, 2.78 * s, .40 * s), (.18 * s, 2.76 * s, .33 * s),
            (-.012 * s, 2.88 * s, .01 * s), (.012 * s, 2.88 * s, .01 * s)
        ], [(0, 1, 3), (1, 4, 3), (1, 2, 4)], mats["armor_dark"], npc, thickness=.025 * s)
        # Nasal guard bar
        api.add_curve(f"{npc}_NasalGuard", [
            (0, 2.78 * s, .38 * s), (0, 2.58 * s, .41 * s), (0, 2.48 * s, .39 * s)
        ], .022 * s, mats["steel"], npc)
        # Sweeping ram horns with brass reinforcement bands
        for side in (-1, 1):
            lbl = api.side_label(side)
            shaped_limb(api, f"{npc}_Horn{lbl}", [
                (side * .23 * s, 2.75 * s, -.06 * s),
                (side * .38 * s, 2.96 * s, -.12 * s),
                (side * .52 * s, 3.16 * s, -.06 * s),
                (side * .48 * s, 3.32 * s, .08 * s),
                (side * .38 * s, 3.40 * s, .16 * s)
            ], (.085 * s, .072 * s, .052 * s, .032 * s, .008 * s),
               (.085 * s, .068 * s, .048 * s, .028 * s, .008 * s),
               mats["bone"], npc, smooth=True)
            api.add_torus(f"{npc}_HornBand{lbl}", (side * .38 * s, 2.96 * s, -.12 * s),
                          .075 * s, .014 * s, mats["accent"], npc)
        # Thick barbarian braided beard
        cloth_panel(api, f"{npc}_WarBeard", [
            [(-.16 * s, 2.46 * s, .25 * s), (0, 2.44 * s, .31 * s), (.16 * s, 2.46 * s, .25 * s)],
            [(-.18 * s, 2.25 * s, .29 * s), (0, 2.22 * s, .37 * s), (.18 * s, 2.25 * s, .29 * s)],
            [(-.12 * s, 2.00 * s, .30 * s), (0, 1.95 * s, .40 * s), (.12 * s, 2.00 * s, .30 * s)],
            [(-.05 * s, 1.80 * s, .31 * s), (0, 1.76 * s, .38 * s), (.05 * s, 1.80 * s, .31 * s)]
        ], mats["cloth_shade"], npc, thickness=.035 * s)
        # Braided hair cascading down the back
        api.add_curve(f"{npc}_BraidDorsal", [(0, 2.70 * s, -.18 * s), (0, 2.20 * s, -.25 * s), (.03 * s, 1.65 * s, -.26 * s)], .045 * s, mats["dark"], npc)
        api.add_torus(f"{npc}_BraidRing0", (0, 2.20 * s, -.25 * s), .055 * s, .014 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
        api.add_torus(f"{npc}_BraidRing1", (.03 * s, 1.70 * s, -.26 * s), .045 * s, .012 * s, mats["accent"], npc, (math.pi / 2, 0, 0))

    elif npc == "Adjudicator":
        # Impassive Golden Death Mask of Judgment with radiant solar halo.
        # Head and mask are a single seamless sculpted volume (no separate floating mask sphere).
        # Hammered steel/gold blindfold wraps cleanly across the eye plane
        api.mesh_object(f"{npc}_Blindfold", [
            (-.235 * s, 2.67 * s, .12 * s), (.235 * s, 2.67 * s, .12 * s),
            (.225 * s, 2.54 * s, .31 * s), (0, 2.55 * s, .345 * s), (-.225 * s, 2.54 * s, .31 * s)
        ], [(0, 1, 2, 3, 4)], mats["steel"], npc, thickness=.022 * s)
        # Multi-tiered radiant solar crown / halo
        api.add_torus(f"{npc}_CrownHaloRing", (0, 2.82 * s, .02 * s), .28 * s, .025 * s, mats["accent"], npc)
        for i in range(8):
            angle = math.tau * i / 8
            length = .22 if i % 2 == 0 else .14
            api.add_curve(f"{npc}_SolarRay{i}", [
                (.27 * math.sin(angle) * s, (2.82 + .27 * math.cos(angle)) * s, .02 * s),
                ((.27 + length) * math.sin(angle) * s, (2.82 + (.27 + length) * math.cos(angle)) * s, .02 * s)
            ], .018 * s, mats["accent"], npc)

    elif npc == "Matriarch":
        # Arachnid brood-queen: cracked porcelain mask, 6 crimson spider eyes, chitin spires
        api.add_ico(f"{npc}_PorcelainMask", (0, 2.57 * s, .20 * s),
                    (.215 * s, .265 * s, .155 * s), mats["bone"], npc)
        # 6 glowing crimson spider eyes in cluster
        eye_offsets = ((-.06, 2.64, .37), (.06, 2.64, .37),
                       (-.11, 2.58, .36), (.11, 2.58, .36),
                       (-.05, 2.52, .38), (.05, 2.52, .38))
        for idx, (ex, ey, ez) in enumerate(eye_offsets):
            api.add_ico(f"{npc}_SpiderEye{idx}", (ex * s, ey * s, ez * s),
                        (.024 * s, .024 * s, .015 * s), mats["accent"], npc)
        # Curved mandibular pincers at chin
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_curve(f"{npc}_Mandible{lbl}", [
                (side * .08 * s, 2.38 * s, .28 * s),
                (side * .14 * s, 2.31 * s, .33 * s),
                (side * .03 * s, 2.27 * s, .38 * s)
            ], .016 * s, mats["bone"], npc)
        # Multi-jointed chitin spider-leg crown spires
        for side in (-1, 1):
            lbl = api.side_label(side)
            for idx in range(3):
                shaped_limb(api, f"{npc}_SpiderSpire{lbl}{idx}", [
                    (side * (.10 + idx * .07) * s, 2.75 * s, -.12 * s),
                    (side * (.18 + idx * .11) * s, (3.02 + idx * .05) * s, -.16 * s),
                    (side * (.28 + idx * .14) * s, (3.28 - idx * .06) * s, -.08 * s)
                ], (.040 * s, .026 * s, .005 * s),
                   (.038 * s, .022 * s, .005 * s), mats["bone"], npc, smooth=False)

    elif npc == "OathlessCurate":
        # Cathedral split-mitre with gold embroidered heretical sigils & thorn halo
        # Split mitre (two tall peaked leaves with central cleft)
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.mesh_object(f"{npc}_MitrePeak{lbl}", [
                (side * .04 * s, 2.76 * s, -.18 * s), (side * .24 * s, 2.76 * s, -.14 * s),
                (side * .22 * s, 2.76 * s, .14 * s), (side * .04 * s, 2.76 * s, .18 * s),
                (side * .15 * s, 3.25 * s, 0)
            ], [(0, 1, 4), (1, 2, 4), (2, 3, 4), (3, 0, 4)], mats["cloth_shade"], npc, thickness=.025 * s)
        # Liturgical mitre infulae hanging down back
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_curve(f"{npc}_Infula{lbl}", [
                (side * .09 * s, 2.70 * s, -.22 * s),
                (side * .12 * s, 2.25 * s, -.26 * s),
                (side * .10 * s, 1.70 * s, -.27 * s)
            ], .030 * s, mats["cloth"], npc)
            api.add_ico(f"{npc}_InfulaFringe{lbl}", (side * .10 * s, 1.66 * s, -.27 * s),
                        (.040 * s, .030 * s, .015 * s), mats["accent"], npc)
        # Blackened iron thorned halo
        api.add_torus(f"{npc}_ThornHalo", (0, 2.92 * s, -.02 * s), .28 * s, .016 * s, mats["armor_dark"], npc)
        for i in range(8):
            ang = math.tau * i / 8
            api.add_ico(f"{npc}_HaloSpike{i}", ((.28 * math.sin(ang)) * s, (2.92 + .28 * math.cos(ang)) * s, -.02 * s),
                        (.025 * s, .050 * s, .025 * s), mats["armor_dark"], npc)
        # Dark ritual ink streaks running down gaunt cheeks
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_curve(f"{npc}_TearStreak{lbl}", [
                (side * .09 * s, 2.58 * s, .315 * s),
                (side * .10 * s, 2.45 * s, .310 * s),
                (side * .09 * s, 2.36 * s, .270 * s)
            ], .010 * s, mats["dark"], npc)

    elif npc == "Tollmaster":
        # Blackened executioner Great Helm with 3D brass cross-slit visor
        head_surface(api, "HelmetShell", [
            (2.28, .32, -.02, .32),
            (2.50, .33, .02, .41),
            (2.72, .32, .03, .45),
            (2.84, .27, .05, .46),
            (2.93, .015, -.03, .07),
        ], mats["armor_dark"], npc, s, opening=0, thickness=.040)
        # Bold 3D brass executioner visor cross-plate mounted on front of helm
        api.mesh_object(f"{npc}_VisorPlateH", [
            (-.21 * s, 2.65 * s, .45 * s), (.21 * s, 2.65 * s, .45 * s),
            (.19 * s, 2.55 * s, .46 * s), (0, 2.54 * s, .49 * s), (-.19 * s, 2.55 * s, .46 * s)
        ], [(0, 1, 2, 3, 4)], mats["accent"], npc, thickness=.026 * s)
        api.mesh_object(f"{npc}_VisorPlateV", [
            (-.035 * s, 2.78 * s, .48 * s), (.035 * s, 2.78 * s, .48 * s),
            (.030 * s, 2.38 * s, .45 * s), (-.030 * s, 2.38 * s, .45 * s)
        ], [(0, 1, 2, 3)], mats["accent"], npc, thickness=.028 * s)
        # Deep horizontal ocular eye slits with menacing glowing eyes peering through
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.mesh_object(f"{npc}_EyeSlit{lbl}", [
                (side * .038 * s, 2.615 * s, .485 * s),
                (side * .165 * s, 2.615 * s, .465 * s),
                (side * .165 * s, 2.585 * s, .465 * s),
                (side * .038 * s, 2.585 * s, .485 * s)
            ], [(0, 1, 2, 3)], mats["dark"], npc, thickness=.015 * s)
            api.add_ico(f"{npc}_VisorEye{lbl}", (side * .095 * s, 2.60 * s, .475 * s),
                        (.022 * s, .012 * s, .012 * s), mats["glow"], npc)
            for row in (0, 1):
                api.add_ico(f"{npc}_VentHole{row}{lbl}",
                            (side * (.07 + row * .045) * s, (2.48 - row * .055) * s, .455 * s),
                            (.012 * s, .012 * s, .010 * s), mats["dark"], npc)
        # Heavy brow reinforcement ridge
        api.add_curve(f"{npc}_BrowRidge", [
            (-.25 * s, 2.72 * s, .41 * s),
            (0, 2.74 * s, .49 * s),
            (.25 * s, 2.72 * s, .41 * s)
        ], .022 * s, mats["steel"], npc)
        # Flared reinforced neckplate in back
        api.add_curve(f"{npc}_HelmNeckPlate", [
            (-.22 * s, 2.28 * s, -.16 * s), (0, 2.25 * s, -.28 * s), (.22 * s, 2.28 * s, -.16 * s)
        ], .045 * s, mats["steel"], npc)

    elif npc == "Tidemother":
        # Abyssal crown of jagged coral, nautilus shells, and sea pearls
        api.add_torus(f"{npc}_CrownRing", (0, 2.76 * s, .02 * s), .26 * s, .025 * s, mats["accent"], npc)
        for i in range(5):
            angle = -math.pi * .4 + math.pi * .8 * (i / 4)
            shaped_limb(api, f"{npc}_CoralSpire{i}", [
                (.26 * math.sin(angle) * s, 2.76 * s, (.02 + .26 * math.cos(angle)) * s),
                ((.30 + .05 * (i % 2)) * math.sin(angle) * s, (2.95 + .08 * (i % 2)) * s, (.02 + .30 * math.cos(angle)) * s)
            ], (.035 * s, .012 * s), (.035 * s, .012 * s), mats["accent"], npc, smooth=False)
            api.add_ico(f"{npc}_SeaPearl{i}",
                        (.26 * math.sin(angle) * s, 2.76 * s, (.02 + .26 * math.cos(angle)) * s),
                        (.030 * s, .030 * s, .030 * s), mats["bone"], npc)
        # Webbed aquatic fin-ears
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.mesh_object(f"{npc}_FinEar{lbl}", [
                (side * .24 * s, 2.55 * s, .05 * s),
                (side * .36 * s, 2.68 * s, -.08 * s),
                (side * .34 * s, 2.48 * s, -.06 * s),
                (side * .28 * s, 2.38 * s, .02 * s)
            ], [(0, 1, 2, 3)], mats["accent"], npc, thickness=.015 * s)

    elif npc == "Cragmother":
        # Angular rock-shelf brow overhang
        api.mesh_object(f"{npc}_StoneBrow", [
            (-.26 * s, 2.70 * s, .12 * s), (.26 * s, 2.70 * s, .12 * s),
            (.24 * s, 2.78 * s, .35 * s), (-.24 * s, 2.78 * s, .35 * s)
        ], [(0, 1, 2, 3)], mats["cloth_shade"], npc, thickness=.045 * s)
        # Lichen moss beard cascading beneath the magma mouth
        cloth_panel(api, f"{npc}_LichenBeard", [
            [(-.18 * s, 2.38 * s, .24 * s), (0, 2.36 * s, .30 * s), (.18 * s, 2.38 * s, .24 * s)],
            [(-.15 * s, 2.16 * s, .26 * s), (0, 2.12 * s, .34 * s), (.15 * s, 2.16 * s, .26 * s)],
            [(-.08 * s, 1.96 * s, .25 * s), (0, 1.92 * s, .31 * s), (.08 * s, 1.96 * s, .25 * s)]
        ], mats["cloth"], npc, thickness=.035 * s)
        # Glowing magma fracture veins across cranial rock plates
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_curve(f"{npc}_MagmaVein{lbl}", [
                (side * .06 * s, 2.72 * s, .32 * s),
                (side * .14 * s, 2.75 * s, .28 * s),
                (side * .22 * s, 2.68 * s, .20 * s)
            ], .014 * s, mats["glow"], npc)

    elif npc == "Commoner":
        # Medieval soft linen chaperon / coif with folded brim roll
        head_surface(api, "Cap", [
            (2.63, .248, .017, .27),
            (2.76, .245, .022, .275),
            (2.88, .16, .016, .195),
            (2.92, .015, .015, .025)
        ], mats["cloth_shade"], npc, s)
        api.add_torus(f"{npc}_CapRoll", (0, 2.74 * s, .08 * s), .255 * s, .038 * s, mats["cloth"], npc, (math.pi / 2, 0, 0))
        # Chaperon liripipe scarf tail falling over back shoulder
        api.add_curve(f"{npc}_ChaperonLiripipe", [(.12 * s, 2.72 * s, -.12 * s), (.22 * s, 2.30 * s, -.18 * s), (.18 * s, 1.75 * s, -.22 * s)], .038 * s, mats["cloth"], npc)

    elif npc == "Vendor":
        # Broad-brimmed felt merchant hat
        head_surface(api, "MerchantHatCrown", [
            (2.75, .255, .01, .28),
            (2.87, .250, .01, .28),
            (2.97, .210, .01, .23),
            (3.02, .015, .01, .025)
        ], mats["cloth_shade"], npc, s)
        # Broad circular felt brim extending outward horizontally
        api.add_torus(f"{npc}_MerchantHatBrim", (0, 2.76 * s, .06 * s), .38 * s, .035 * s, mats["cloth_shade"], npc, (math.pi / 2, 0, 0))
        api.add_torus(f"{npc}_HatBand", (0, 2.78 * s, .06 * s), .265 * s, .016 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
        # Merchant hat ribbon trailing down back of neck
        api.add_curve(f"{npc}_HatRibbon", [(0, 2.76 * s, -.24 * s), (.04 * s, 2.50 * s, -.28 * s), (-.02 * s, 2.25 * s, -.26 * s)], .018 * s, mats["accent"], npc)
        # Merchant mustache
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_curve(f"{npc}_Mustache{lbl}", [
                (0, 2.46 * s, .33 * s),
                (side * .08 * s, 2.44 * s, .32 * s),
                (side * .14 * s, 2.47 * s, .26 * s)
            ], .016 * s, mats["dark"], npc)

    elif npc == "Smuggler":
        # Ribbed fisherman's watch-cap (beanie) with rolled cuff
        head_surface(api, "WatchCap", [
            (2.62, .245, .015, .265),
            (2.75, .240, .020, .265),
            (2.86, .155, .015, .185),
            (2.91, .015, .015, .025)
        ], mats["cloth_shade"], npc, s)
        api.add_torus(f"{npc}_WatchCapRoll", (0, 2.70 * s, .07 * s), .255 * s, .032 * s, mats["cloth"], npc, (math.pi / 2, 0, 0))
        # Gold hoop earring on left ear
        api.add_torus(f"{npc}_Earring", (-.245 * s, 2.43 * s, .06 * s), .030 * s, .008 * s, mats["accent"], npc, (0, math.pi / 2, 0))

    elif npc == "Companion":
        # Steel kettle hat / open-faced bascinet over quilted coif
        head_surface(api, "KettleHelm", [
            (2.55, .265, .015, .28),
            (2.72, .260, .020, .28),
            (2.88, .190, .015, .20),
            (2.95, .015, .015, .025)
        ], mats["steel"], npc, s)
        # Flared steel brim catching highlights
        api.add_torus(f"{npc}_KettleBrim", (0, 2.68 * s, .07 * s), .34 * s, .025 * s, mats["steel"], npc, (math.pi / 2, 0, 0))
        # Flared kettle helm neck flange in back
        api.add_curve(f"{npc}_HelmNeckFlange", [(-.26 * s, 2.62 * s, -.18 * s), (0, 2.60 * s, -.30 * s), (.26 * s, 2.62 * s, -.18 * s)], .035 * s, mats["steel"], npc)


def add_skull(npc, mats, s, api):
    """Anatomical bone skull with deep orbits, glowing soul-embers, and tooth gaps."""
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
        # Glowing soul embers inside the deep eye sockets!
        api.add_ico(f"{npc}_SoulEmber{label}", (side * .115 * s, 2.605 * s, .335 * s),
                    (.025 * s, .025 * s, .015 * s), mats["glow"], npc)
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
    # Realistic jagged teeth with natural gaps (not uniform pills)
    for i in (0, 1, 3, 5):
        api.add_ico(f"{npc}_SkullTooth{i}",
                    ((i - 2.5) * .036 * s, 2.338 * s, .345 * s),
                    (.014 * s, .026 * s, .011 * s), mats["bone"], npc)


def garment(npc, cfg, mats, s, api):
    kind = cfg["garment"]
    def rows(data):
        return [[(x * s, y * s, z * s) for x, y, z in row] for row in data]

    if kind == "apron":
        # Commoner / Vendor: tailored work vest & naturally pleated apron
        api.lathe(f"{npc}_WorkVest", [(1.08 * s, .36 * s, .28 * s),
                  (1.43 * s, .36 * s, .28 * s), (1.81 * s, .43 * s, .30 * s),
                  (2.08 * s, .37 * s, .26 * s)], mats["cloth_shade"], npc, segments=14)
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
        api.add_curve(f"{npc}_RopeBelt", [(-.35 * s, 1.14 * s, .14 * s),
                      (0, 1.13 * s, .32 * s), (.35 * s, 1.14 * s, .14 * s)],
                      .030 * s, mats["leather"], npc)

    elif kind == "cloak":
        # Cloak with layered shoulder pelerine drape
        cloth_panel(api, f"{npc}_Cloak", rows([
            ((-.32, 2.19, -.13), (0, 2.21, -.36), (.32, 2.19, -.13)),
            ((-.43, 1.80, -.19), (0, 1.78, -.49), (.43, 1.80, -.19)),
            ((-.46, 1.21, -.22), (0, 1.17, -.53), (.46, 1.21, -.22)),
            ((-.52, .63, -.22), (0, .52, -.58), (.52, .63, -.22)),
        ]), mats["cloth_shade"], npc, .045 * s)
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
        # Flared robe with layered drapery
        api.lathe(f"{npc}_Skirt", [(.07 * s, .50 * s, .42 * s), (.26 * s, .48 * s, .40 * s),
                  (.65 * s, .42 * s, .35 * s), (1.08 * s, .36 * s, .29 * s),
                  (1.17 * s, .33 * s, .27 * s)], mats["cloth"], npc, segments=14)
        cloth_panel(api, f"{npc}_RobeFront", rows([
            ((-.20, 1.17, .28), (0, 1.17, .32), (.20, 1.17, .28)),
            ((-.27, .72, .33), (0, .72, .40), (.27, .72, .33)),
            ((-.34, .12, .39), (0, .10, .49), (.34, .12, .39)),
        ]), mats["cloth_shade"], npc)
        api.add_curve(f"{npc}_Sash", [(-.33 * s, 1.17 * s, .14 * s),
                      (0, 1.14 * s, .34 * s), (.33 * s, 1.17 * s, .14 * s)], .05 * s, mats["accent"], npc)
        cloth_panel(api, f"{npc}_RobeBack", rows([
            ((-.28, 2.14, -.17), (0, 2.15, -.34), (.28, 2.14, -.17)),
            ((-.36, 1.73, -.22), (0, 1.70, -.40), (.36, 1.73, -.22)),
            ((-.37, 1.23, -.23), (0, 1.19, -.41), (.37, 1.23, -.23)),
            ((-.37, .69, -.30), (0, .59, -.48), (.37, .69, -.30)),
        ]), mats["cloth_shade"], npc)

    elif kind == "brigandine":
        # Fitted steel cuirass with contoured breastplates and tassets
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
            for height in (1.42, 1.83):
                api.add_ico(f"{npc}_Rivet{label}{int(height * 100)}", (side * .30 * s, height * s, .30 * s),
                            (.025 * s, .028 * s, .016 * s), mats["accent"], npc)
            cloth_panel(api, f"{npc}_Pauldron{label}", rows([
                ((side * .34, 2.15, -.12), (side * .49, 2.17, 0), (side * .58, 2.11, .08)),
                ((side * .41, 1.99, -.13), (side * .55, 2.01, .01), (side * .62, 1.94, .10)),
                ((side * .42, 1.82, -.10), (side * .55, 1.83, .02), (side * .59, 1.79, .10)),
            ]), mats["steel"], npc, .04 * s)

    elif kind == "vest":
        for side in (-1, 1):
            cloth_panel(api, f"{npc}_Vest{api.side_label(side)}", rows([
                ((side * .08, 2.06, .28), (side * .23, 2.08, .26), (side * .37, 2.00, .17)),
                ((side * .10, 1.55, .32), (side * .24, 1.55, .34), (side * .40, 1.51, .18)),
                ((side * .14, 1.07, .28), (side * .28, 1.03, .29), (side * .37, 1.02, .16)),
            ]), mats["cloth_shade"], npc)


def identity_details(npc, mats, s, api):
    """Archetype-specific high-impact silhouette props."""
    if npc == "Chief":
        # Grizzly bear fur mantle draped over shoulders and back
        for side in (-1, 1):
            lbl = api.side_label(side)
            cloth_panel(api, f"{npc}_FurMantle{lbl}", [
                [(side * .28 * s, 2.22 * s, -.18 * s), (side * .45 * s, 2.20 * s, 0), (side * .40 * s, 2.15 * s, .22 * s)],
                [(side * .32 * s, 1.92 * s, -.22 * s), (side * .58 * s, 1.90 * s, 0), (side * .46 * s, 1.85 * s, .25 * s)],
                [(side * .30 * s, 1.62 * s, -.20 * s), (side * .52 * s, 1.60 * s, 0), (side * .40 * s, 1.55 * s, .22 * s)]
            ], mats["dark"], npc, thickness=.055 * s)

    elif npc == "Adjudicator":
        # Trailing parchment scrolls with red wax seals bearing Sigils
        for side in (-1, 1):
            lbl = api.side_label(side)
            cloth_panel(api, f"{npc}_SigilScroll{lbl}", [
                [(side * .12 * s, 1.55 * s, .38 * s), (side * .17 * s, 1.55 * s, .37 * s), (side * .22 * s, 1.55 * s, .36 * s)],
                [(side * .13 * s, 1.10 * s, .36 * s), (side * .18 * s, 1.10 * s, .35 * s), (side * .23 * s, 1.10 * s, .34 * s)],
                [(side * .11 * s, .65 * s, .37 * s), (side * .16 * s, .65 * s, .36 * s), (side * .21 * s, .65 * s, .35 * s)],
            ], mats["linen"], npc, thickness=.015 * s)
            api.add_ico(f"{npc}_WaxSeal{lbl}", (side * .17 * s, 1.56 * s, .39 * s),
                        (.035 * s, .035 * s, .015 * s), mats["accent"], npc)
        # Ceremonial golden clerical back stole with sacred sun crest
        cloth_panel(api, f"{npc}_BackStole", [
            [(-.18 * s, 2.10 * s, -.24 * s), (0, 2.12 * s, -.26 * s), (.18 * s, 2.10 * s, -.24 * s)],
            [(-.16 * s, 1.55 * s, -.26 * s), (0, 1.55 * s, -.28 * s), (.16 * s, 1.55 * s, -.26 * s)],
            [(-.14 * s, 1.00 * s, -.27 * s), (0, 1.00 * s, -.29 * s), (.14 * s, 1.00 * s, -.27 * s)]
        ], mats["accent"], npc, thickness=.035 * s)

    elif npc == "Tollmaster":
        # Heavy crossed chains draped over the torso
        api.add_curve(f"{npc}_CrossChainA", [
            (-.36 * s, 2.08 * s, .15 * s), (0, 1.65 * s, .36 * s), (.36 * s, 1.20 * s, .15 * s)
        ], .026 * s, mats["steel"], npc)
        api.add_curve(f"{npc}_CrossChainB", [
            (.36 * s, 2.08 * s, .15 * s), (0, 1.65 * s, .36 * s), (-.36 * s, 1.20 * s, .15 * s)
        ], .026 * s, mats["steel"], npc)
        # Executioner back harness and heavy rear key ring
        api.add_curve(f"{npc}_BackHarnessL", [(-.32 * s, 2.05 * s, -.20 * s), (0, 1.55 * s, -.26 * s)], .026 * s, mats["steel"], npc)
        api.add_curve(f"{npc}_BackHarnessR", [(.32 * s, 2.05 * s, -.20 * s), (0, 1.55 * s, -.26 * s)], .026 * s, mats["steel"], npc)
        api.add_torus(f"{npc}_RearKeyRing", (0, 1.12 * s, -.28 * s), .065 * s, .012 * s, mats["steel"], npc)
        # Heavy iron toll-keys and tokens at hip
        api.add_torus(f"{npc}_KeyRing", (.38 * s, 1.05 * s, .12 * s), .055 * s, .010 * s, mats["steel"], npc)
        for i in range(3):
            api.add_ico(f"{npc}_TollKey{i}", (.38 * s, (.95 - i * .06) * s, (.12 + i * .02) * s),
                        (.015 * s, .050 * s, .015 * s), mats["accent"], npc)

    elif npc == "Cragmother":
        # Basalt crags erupting from shoulders and dorsal column
        for side in (-1, 1):
            lbl = api.side_label(side)
            for idx in range(3):
                api.add_ico(f"{npc}_BasaltCrag{lbl}{idx}",
                            (side * (.28 + idx * .09) * s, (2.05 + idx * .06) * s, (-.05 + idx * .06) * s),
                            ((.07 + idx * .02) * s, (.11 + idx * .02) * s, (.07 + idx * .02) * s),
                            mats["cloth_shade"], npc)
        for idx, y_pos in enumerate((2.15, 1.85, 1.55, 1.25)):
            api.add_ico(f"{npc}_DorsalCrag{idx}", (0, y_pos * s, (-.24 - (idx % 2) * .04) * s),
                        ((.08 + idx * .02) * s, .12 * s, .09 * s), mats["cloth_shade"], npc)

    elif npc == "Tidemother":
        # Undulating sea-kelp flounces flaring at hem
        for side in (-1, 1):
            lbl = api.side_label(side)
            cloth_panel(api, f"{npc}_KelpFlounce{lbl}", [
                [(side * .14 * s, .75 * s, .38 * s), (side * .26 * s, .75 * s, .32 * s), (side * .38 * s, .75 * s, .12 * s)],
                [(side * .18 * s, .42 * s, .44 * s), (side * .34 * s, .42 * s, .38 * s), (side * .48 * s, .42 * s, .16 * s)],
                [(side * .22 * s, .12 * s, .52 * s), (side * .42 * s, .12 * s, .44 * s), (side * .58 * s, .12 * s, .20 * s)],
            ], mats["accent"], npc, thickness=.025 * s)
        # Flowing kelp / dorsal fin crest down the spine
        api.add_curve(f"{npc}_DorsalKelp", [(0, 2.25 * s, -.20 * s), (0, 1.65 * s, -.28 * s), (0, 1.05 * s, -.32 * s)], .045 * s, mats["accent"], npc)

    elif npc == "Bandit":
        # Spiked leather pauldron on right weapon shoulder
        cloth_panel(api, f"{npc}_SpikedPauldronR", [
            [(.34 * s, 2.18 * s, -.12 * s), (.49 * s, 2.20 * s, 0), (.58 * s, 2.14 * s, .08 * s)],
            [(.41 * s, 2.02 * s, -.13 * s), (.55 * s, 2.04 * s, .01 * s), (.62 * s, 1.97 * s, .10 * s)],
            [(.42 * s, 1.85 * s, -.10 * s), (.55 * s, 1.86 * s, .02 * s), (.59 * s, 1.82 * s, .10 * s)],
        ], mats["leather"], npc, thickness=.045 * s)
        api.add_ico(f"{npc}_PauldronSpikeR", (.54 * s, 2.05 * s, .02 * s),
                    (.035 * s, .075 * s, .035 * s), mats["steel"], npc)
        # Brigandine backplate with leather cross-buckles
        for idx, y_pos in enumerate((1.45, 1.75)):
            api.add_curve(f"{npc}_BackBuckle{idx}", [(-.25 * s, y_pos * s, -.24 * s), (.25 * s, y_pos * s, -.24 * s)], .020 * s, mats["leather"], npc)

    elif npc == "Alchemist":
        # Chemical vial harness across chest
        api.add_curve(f"{npc}_VialHarness", [
            (-.35 * s, 1.33 * s, .10 * s), (0, 1.31 * s, .35 * s), (.35 * s, 1.33 * s, .10 * s)
        ], .029 * s, mats["leather"], npc)
        # Hip-slung copper alembic distilling flask with coiled glass tube
        api.add_ico(f"{npc}_AlembicFlask", (-.38 * s, 1.05 * s, .08 * s),
                    (.095 * s, .110 * s, .095 * s), mats["accent"], npc)
        api.add_curve(f"{npc}_AlembicTube", [
            (-.38 * s, 1.15 * s, .08 * s), (-.42 * s, 1.28 * s, .12 * s), (-.35 * s, 1.34 * s, .15 * s)
        ], .012 * s, mats["glow"], npc)
        # Corrugated brass respirator breathing tube wrapping to back filter canister
        api.add_curve(f"{npc}_RespiratorTubeBack", [
            (.14 * s, 2.45 * s, .24 * s), (.24 * s, 2.38 * s, .05 * s), (.16 * s, 2.25 * s, -.20 * s), (0, 2.15 * s, -.26 * s)
        ], .018 * s, mats["accent"], npc)
        api.add_ico(f"{npc}_BackFilterCanister", (0, 1.95 * s, -.28 * s), (.12 * s, .18 * s, .10 * s), mats["accent"], npc)
        api.add_torus(f"{npc}_BackFilterCap", (0, 2.08 * s, -.28 * s), .08 * s, .016 * s, mats["steel"], npc, (math.pi / 2, 0, 0))
        api.add_ico(f"{npc}_BackFilterGauge", (0, 1.95 * s, -.36 * s), (.045 * s, .045 * s, .020 * s), mats["glow"], npc)

    elif npc == "Smuggler":
        # Waterproof waxed contraband crate wrapped in rope tucked under left arm
        api.mesh_object(f"{npc}_ContrabandCrate", [
            (-.52 * s, 1.10 * s, -.06 * s), (-.28 * s, 1.10 * s, -.06 * s),
            (-.28 * s, 1.10 * s, .24 * s), (-.52 * s, 1.10 * s, .24 * s),
            (-.52 * s, 1.42 * s, -.06 * s), (-.28 * s, 1.42 * s, -.06 * s),
            (-.28 * s, 1.42 * s, .24 * s), (-.52 * s, 1.42 * s, .24 * s),
        ], [(0, 1, 2, 3), (4, 7, 6, 5), (0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)],
           mats["leather"], npc)
        api.add_torus(f"{npc}_ContrabandRope", (-.40 * s, 1.26 * s, .09 * s), .18 * s, .014 * s, mats["linen"], npc)
        # Split seafarer coat vent with brass buttons down the back spine
        api.add_curve(f"{npc}_CoatVent", [(0, 1.45 * s, -.25 * s), (0, .85 * s, -.28 * s)], .015 * s, mats["accent"], npc)
        for idx, y_pos in enumerate((1.15, 1.35, 1.55)):
            api.add_ico(f"{npc}_SpineButton{idx}", (0, y_pos * s, -.26 * s), (.022 * s, .022 * s, .015 * s), mats["accent"], npc)

    elif npc == "Traveller":
        # Detailed travel satchel with rolled map case
        api.add_curve(f"{npc}_SatchelStrap", [
            (-.30 * s, 2.08 * s, .18 * s), (-.12 * s, 1.75 * s, .33 * s),
            (.29 * s, 1.27 * s, .24 * s), (.38 * s, 1.12 * s, -.12 * s)
        ], .020 * s, mats["leather"], npc)
        api.add_ico(f"{npc}_Satchel", (.38 * s, 1.13 * s, .07 * s),
                    (.12 * s, .16 * s, .10 * s), mats["leather"], npc)
        api.add_ico(f"{npc}_ScrollCase", (.36 * s, 1.26 * s, .09 * s),
                    (.045 * s, .18 * s, .045 * s), mats["linen"], npc)
        # Rolled shoulder capelet / travel mantle over back
        cloth_panel(api, f"{npc}_BackMantle", [
            [(-.35 * s, 2.10 * s, -.22 * s), (0, 2.12 * s, -.25 * s), (.35 * s, 2.10 * s, -.22 * s)],
            [(-.38 * s, 1.75 * s, -.25 * s), (0, 1.72 * s, -.28 * s), (.38 * s, 1.75 * s, -.25 * s)]
        ], mats["cloth"], npc, thickness=.035 * s)

    elif npc == "Companion":
        # Sword scabbard at left hip and shoulder strap
        api.add_curve(f"{npc}_Scabbard", [
            (-.36 * s, 1.25 * s, .12 * s), (-.45 * s, .65 * s, -.08 * s)
        ], .032 * s, mats["leather"], npc)
        # Leather sword baldric strap across the back
        api.add_curve(f"{npc}_BaldricBack", [
            (.32 * s, 2.05 * s, -.18 * s), (0, 1.60 * s, -.26 * s), (-.34 * s, 1.25 * s, -.12 * s)
        ], .024 * s, mats["leather"], npc)

    elif npc == "Matriarch":
        # Brood egg sacs clustered beneath bustle
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_ico(f"{npc}_EggSac{lbl}", (side * .22 * s, .85 * s, -.32 * s),
                        (.11 * s, .13 * s, .11 * s), mats["accent"], npc)
        # Segmented chitin dorsal carapace plates cascading down the spine
        for idx, (y_pos, w, d) in enumerate(((2.05, .22, .14), (1.75, .26, .16), (1.45, .30, .18), (1.15, .32, .20))):
            api.add_ico(f"{npc}_CarapacePlate{idx}", (0, y_pos * s, (-.22 - idx * .03) * s),
                        (w * s, .12 * s, d * s), mats["accent"], npc)

    elif npc == "Commoner":
        # Apron bow knot and tied laces at small of back
        api.add_ico(f"{npc}_ApronBowKnot", (0, 1.15 * s, -.26 * s), (.045 * s, .045 * s, .030 * s), mats["linen"], npc)
        api.add_curve(f"{npc}_ApronBowL", [(0, 1.15 * s, -.26 * s), (-.08 * s, 1.02 * s, -.28 * s)], .014 * s, mats["linen"], npc)
        api.add_curve(f"{npc}_ApronBowR", [(0, 1.15 * s, -.26 * s), (.08 * s, .98 * s, -.28 * s)], .014 * s, mats["linen"], npc)

    elif npc == "Thief":
        # Crossed leather sheath harness with twin dagger scabbards on the back
        api.add_curve(f"{npc}_BackHarnessA", [(-.30 * s, 2.05 * s, -.22 * s), (.30 * s, 1.25 * s, -.22 * s)], .020 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_BackHarnessB", [(.30 * s, 2.05 * s, -.22 * s), (-.30 * s, 1.25 * s, -.22 * s)], .020 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_BackScabbardA", [(-.16 * s, 1.85 * s, -.25 * s), (.12 * s, 1.45 * s, -.25 * s)], .028 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_BackScabbardB", [(.16 * s, 1.85 * s, -.25 * s), (-.12 * s, 1.45 * s, -.25 * s)], .028 * s, mats["leather"], npc)

    elif npc == "Vendor":
        # Leather pack shoulder harness straps
        api.add_curve(f"{npc}_PackHarnessL", [(-.28 * s, 2.10 * s, -.25 * s), (-.28 * s, 1.80 * s, .30 * s), (-.25 * s, 1.25 * s, .25 * s)], .022 * s, mats["leather"], npc)
        api.add_curve(f"{npc}_PackHarnessR", [(.05 * s, 2.10 * s, -.25 * s), (.05 * s, 1.80 * s, .30 * s), (.05 * s, 1.25 * s, .25 * s)], .022 * s, mats["leather"], npc)
        # Wooden walking stick strapped to pack side
        api.add_curve(f"{npc}_PackWalkingStick", [(-.42 * s, .70 * s, -.40 * s), (-.42 * s, 2.25 * s, -.40 * s)], .026 * s, mats["leather"], npc)


def add_humanoid_body(npc, cfg, mats, api):
    s = cfg["scale"]
    undead = cfg["form"] == "undead"
    if not undead:
        mats["leather"] = api.base.material(f"{npc}_StudyLeather", api.rgba("#675039"), 0.02, 0.82)
        mats["linen"] = api.base.material(f"{npc}_StudyLinen", api.rgba("#A89B7F"), 0.0, 0.85)
        mats["armor_dark"] = api.base.material(f"{npc}_StudySteelShadow", api.rgba("#495765"), 0.42, 0.58)

    if undead:
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

    add_study_head(npc, cfg, mats, s, api)

    arms = {"L": ((-.38 * s, 2.02 * s, 0), (-.56 * s, 1.72 * s, .02 * s),
                  (-.50 * s, 1.38 * s, .08 * s), (-.50 * s, 1.22 * s, .10 * s)),
            "R": ((.38 * s, 2.02 * s, 0), (.58 * s, 1.72 * s, .02 * s),
                  (.62 * s, 1.40 * s, .08 * s), (.64 * s, 1.24 * s, .10 * s))}
    for label, (shoulder, elbow, wrist, hand) in arms.items():
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
    x = .64 * s
    def replace(*parts):
        for part in parts:
            old = api.bpy.data.objects.get(f"{npc}_{part}")
            if old: api.bpy.data.objects.remove(old, do_unlink=True)

    if kind in {"dagger", "knife", "sword", "swordshield"}:
        replace("WeaponShaft", "WeaponBlade", "WeaponGuard")
        small_blade = kind in {"dagger", "knife"}
        blade_length = (.52 if small_blade else 1.04) * s
        width = (.09 if small_blade else .16) * s
        base_y = 1.64 * s
        verts = [(x + dx * width, base_y + fraction * blade_length, z * s)
                 for dx, fraction, z in ((-1, 0, .10), (-1.10, .20, .10),
                                         (-.70, .80, .10), (0, 1, .10),
                                         (.70, .80, .10), (1.10, .20, .10), (1, 0, .10))]
        api.mesh_object(f"{npc}_WeaponBlade", verts, [tuple(range(len(verts)))],
                        mats["steel"], npc, thickness=.055 * s, bevel=.012 * s)
        api.add_curve(f"{npc}_WeaponFuller", [(x, 1.69 * s, .146 * s),
                      (x, (1.64 * s + blade_length * .72), .146 * s)],
                      .010 * s, mats["accent"], npc)
        grip_bottom = 1.34 if small_blade else 1.13
        api.add_curve(f"{npc}_WeaponGrip", [(x, grip_bottom * s, .10 * s),
                      (x, 1.63 * s, .10 * s)], .038 * s,
                      mats["leather"] if kind != "sword" else mats["dark"], npc)
        guard = .14 if small_blade else .22
        api.add_curve(f"{npc}_WeaponCrossguard", [(x - guard * s, 1.62 * s, .10 * s),
                      (x, 1.66 * s, .10 * s), (x + guard * s, 1.62 * s, .10 * s)],
                      .038 * s, mats["steel"], npc)
        api.add_ico(f"{npc}_WeaponPommel", (x, grip_bottom * s, .10 * s),
                    (.055 * s, .065 * s, .055 * s), mats["steel"], npc)

        if kind == "swordshield":
            replace("Shield")
            # Knightly heater shield with heraldic crest
            sx = -.52 * s
            cloth_panel(api, f"{npc}_ShieldBody", [
                [(sx - .22 * s, 1.85 * s, .12 * s), (sx, 1.88 * s, .24 * s), (sx + .22 * s, 1.85 * s, .12 * s)],
                [(sx - .24 * s, 1.50 * s, .16 * s), (sx, 1.52 * s, .28 * s), (sx + .24 * s, 1.50 * s, .16 * s)],
                [(sx - .18 * s, 1.15 * s, .18 * s), (sx, 1.15 * s, .26 * s), (sx + .18 * s, 1.15 * s, .18 * s)],
                [(sx - .04 * s, .80 * s, .18 * s), (sx, .76 * s, .22 * s), (sx + .04 * s, .80 * s, .18 * s)],
            ], mats["cloth"], npc, thickness=.045 * s)
            # Steel rim and shield boss
            api.add_ico(f"{npc}_ShieldBoss", (sx, 1.50 * s, .31 * s),
                        (.075 * s, .075 * s, .050 * s), mats["steel"], npc)
            # Forearm strap anchoring shield to arm
            api.add_torus(f"{npc}_ShieldStrapUpper", (sx + .04 * s, 1.62 * s, .16 * s),
                          .10 * s, .018 * s, mats["leather"], npc, (0, math.pi / 2, 0))
            api.add_torus(f"{npc}_ShieldStrapLower", (sx + .04 * s, 1.38 * s, .18 * s),
                          .09 * s, .018 * s, mats["leather"], npc, (0, math.pi / 2, 0))

    elif kind in {"staff", "ringstaff", "kelp"}:
        replace("Staff", "StaffTop", "StaffRing")
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
            # Nested orbital armillary rings with floating mana crystal
            api.add_torus(f"{npc}_StaffRingOuter", (x, 2.62 * s, tip_z),
                          .18 * s, .020 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
            api.add_torus(f"{npc}_StaffRingInner", (x, 2.62 * s, tip_z),
                          .13 * s, .015 * s, mats["accent"], npc, (math.pi / 2, math.pi / 4, 0))
            api.add_ico(f"{npc}_StaffCrystal", (x, 2.62 * s, tip_z),
                        (.085 * s, .115 * s, .085 * s), mats["glow"], npc)
        elif kind == "kelp":
            for side in (-1, 1):
                api.add_curve(f"{npc}_StaffFrond{api.side_label(side)}",
                              [(x, 2.30 * s, staff_z(2.30)),
                               (x + side * .16 * s, 2.48 * s, tip_z),
                               (x + side * .24 * s, 2.76 * s, tip_z)],
                              .028 * s, mats["accent"], npc)
            api.add_ico(f"{npc}_StaffKnob", (x, 2.55 * s, tip_z),
                        (.085 * s, .11 * s, .082 * s), mats["glow"], npc)
        else:
            # Traveller staff with hanging brass lantern
            api.add_curve(f"{npc}_StaffFork",
                          [(x - .085 * s, 2.43 * s, staff_z(2.43)),
                           (x, 2.58 * s, tip_z),
                           (x + .085 * s, 2.43 * s, staff_z(2.43))],
                          .025 * s, mats["leather"], npc)
            api.add_ico(f"{npc}_StaffLanternGlow", (x - .12 * s, 2.25 * s, tip_z),
                        (.055 * s, .075 * s, .055 * s), mats["glow"], npc)
            api.add_torus(f"{npc}_StaffLanternRing", (x - .12 * s, 2.34 * s, tip_z),
                          .045 * s, .010 * s, mats["accent"], npc)

    elif kind == "greataxe":
        replace("AxeHead")
        # Massive double-beveled barbarian greataxe
        verts = [(x + dx * s, y * s, .11 * s) for dx, y in (
            (-.09, 2.42), (.15, 2.42), (.58, 2.65), (.45, 2.26),
            (.48, 1.88), (.56, 1.62), (.14, 1.82), (-.09, 1.83),
            (-.42, 1.68), (-.35, 2.05), (-.38, 2.30), (-.45, 2.52))]
        api.mesh_object(f"{npc}_AxeBlade", verts, [tuple(range(len(verts)))],
                        mats["steel"], npc, thickness=.085 * s, bevel=.018 * s)
        api.add_curve(f"{npc}_AxeSocket", [(x - .10 * s, 2.38 * s, .15 * s),
                      (x + .15 * s, 2.37 * s, .15 * s), (x + .15 * s, 1.88 * s, .15 * s),
                      (x - .10 * s, 1.88 * s, .15 * s)], .028 * s, mats["armor_dark"], npc)
        for height in (1.98, 2.26):
            api.add_ico(f"{npc}_AxeRivet{int(height * 100)}", (x + .03 * s, height * s, .17 * s),
                        (.025 * s, .028 * s, .016 * s), mats["accent"], npc)

    elif kind == "gavel":
        # Monumental War-Maul of Decrees with radiant gold filigree
        replace("GavelShaft", "GavelHead")
        api.add_curve(f"{npc}_GavelShaft", [(x, .55 * s, .10 * s), (x, 2.32 * s, .10 * s)],
                      .065 * s, mats["armor_dark"], npc)
        rings = ((-.38, .12), (-.32, .18), (.32, .18), (.38, .12))
        vertices = [(x + along * s, (2.28 + radius * math.sin(math.tau * i / 8)) * s,
                     (.10 + radius * math.cos(math.tau * i / 8)) * s)
                    for along, radius in rings for i in range(8)]
        faces = [(row * 8 + i, row * 8 + (i + 1) % 8, (row + 1) * 8 + (i + 1) % 8, (row + 1) * 8 + i)
                 for row in range(len(rings) - 1) for i in range(8)]
        faces += [tuple(reversed(range(8))), tuple(24 + i for i in range(8))]
        api.mesh_object(f"{npc}_GavelHead", vertices, faces, mats["accent"], npc)
        for side in (-1, 1):
            lbl = api.side_label(side)
            api.add_ico(f"{npc}_MaulGlow{lbl}", (x + side * .39 * s, 2.28 * s, .10 * s),
                        (.025 * s, .110 * s, .095 * s), mats["glow"], npc)

    elif kind == "flail":
        replace("FlailChain", "FlailWeight")
        # Heavy flail on slackened links with natural gravity drape
        chain_points = [
            (x + .05 * s, 2.05 * s, .10 * s),
            (x + .14 * s, 2.12 * s, .10 * s),
            (x + .22 * s, 2.22 * s, .10 * s),
            (x + .30 * s, 2.34 * s, .10 * s),
            (x + .36 * s, 2.48 * s, .10 * s),
        ]
        for idx, (lx, ly, lz) in enumerate(chain_points):
            api.add_torus(f"{npc}_FlailLink{idx}", (lx, ly, lz),
                          .055 * s, .014 * s, mats["steel"], npc,
                          (0, math.pi / 2 if idx % 2 else 0, 0))
        api.add_ico(f"{npc}_FlailWeight", (x + .42 * s, 2.60 * s, .10 * s),
                    (.165 * s, .170 * s, .165 * s), mats["armor_dark"], npc)
        for index in range(8):
            angle = math.tau * index / 8
            shaped_limb(api, f"{npc}_FlailSpike{index}",
                        [(x + (.42 + .14 * math.cos(angle)) * s, 2.60 * s, (.10 + .14 * math.sin(angle)) * s),
                         (x + (.42 + .25 * math.cos(angle)) * s, 2.62 * s, (.10 + .24 * math.sin(angle)) * s)],
                        (.045 * s, .005 * s), (.045 * s, .005 * s), mats["steel"], npc, smooth=False)

    elif kind == "page":
        replace("Page")
        # The Unwritten Gospel: Chained iron-bound codex held open
        # Front and back covers
        api.mesh_object(f"{npc}_BookCoverL", [
            (x - .05 * s, 1.25 * s, .10 * s), (x - .05 * s, 1.82 * s, .10 * s),
            (x - .40 * s, 1.85 * s, .16 * s), (x - .40 * s, 1.28 * s, .16 * s)
        ], [(0, 1, 2, 3)], mats["armor_dark"], npc, thickness=.035 * s)
        api.mesh_object(f"{npc}_BookCoverR", [
            (x - .05 * s, 1.25 * s, .10 * s), (x - .05 * s, 1.82 * s, .10 * s),
            (x + .30 * s, 1.85 * s, .06 * s), (x + .30 * s, 1.28 * s, .06 * s)
        ], [(0, 1, 2, 3)], mats["armor_dark"], npc, thickness=.035 * s)
        # Gilded open parchment pages
        api.mesh_object(f"{npc}_BookPages", [
            (x - .04 * s, 1.28 * s, .11 * s), (x - .04 * s, 1.79 * s, .11 * s),
            (x - .36 * s, 1.82 * s, .17 * s), (x - .36 * s, 1.31 * s, .17 * s)
        ], [(0, 1, 2, 3)], mats["bone"], npc, thickness=.025 * s)
        # Floating necrotic sigil rune
        api.add_ico(f"{npc}_BookRuneSigil", (x - .18 * s, 1.55 * s, .25 * s),
                    (.065 * s, .085 * s, .035 * s), mats["glow"], npc)

    elif kind == "vials":
        # Multi-colored glowing chemical potion bottles
        vial_colors = ("accent", "glow", "steel")
        for index, dx in enumerate((-.14, 0, .14)):
            replace(f"Vial{index}")
            api.add_ico(f"{npc}_Vial{index}", (dx * s, 1.20 * s, .36 * s),
                        (.048 * s, .105 * s, .048 * s), mats[vial_colors[index]], npc)
            api.add_ico(f"{npc}_VialStopper{index}", (dx * s, 1.32 * s, .36 * s),
                        (.028 * s, .032 * s, .028 * s), mats["leather"], npc)

    elif kind == "pack":
        replace("Pack")
        # Loaded merchant expedition rig: wooden framed pack with bedroll and wares
        api.mesh_object(f"{npc}_PackFrame", [
            (dx * s, y * s, z * s) for dx, y, z in (
                (-.38, 1.95, -.28), (.08, 1.95, -.28),
                (.08, 1.95, -.58), (-.38, 1.95, -.58),
                (-.38, 1.15, -.28), (.08, 1.15, -.28),
                (.08, 1.15, -.58), (-.38, 1.15, -.58)
            )
        ], [(0, 1, 2, 3), (4, 7, 6, 5), (0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)],
           mats["leather"], npc, bevel=.035 * s)
        # Rolled wool bedroll on top
        api.add_ico(f"{npc}_Bedroll", (-.15 * s, 2.08 * s, -.43 * s),
                    (.26 * s, .09 * s, .12 * s), mats["cloth"], npc)
        # Hanging brass balance scales on the flank
        api.add_curve(f"{npc}_ScaleBeam", [
            (-.42 * s, 1.70 * s, -.32 * s), (-.42 * s, 1.70 * s, -.54 * s)
        ], .014 * s, mats["accent"], npc)
        api.add_ico(f"{npc}_ScalePanA", (-.42 * s, 1.52 * s, -.34 * s),
                    (.045 * s, .015 * s, .045 * s), mats["accent"], npc)
        api.add_ico(f"{npc}_ScalePanB", (-.42 * s, 1.52 * s, -.52 * s),
                    (.045 * s, .015 * s, .045 * s), mats["accent"], npc)
        # Twin trade flasks on right flank
        for idx in range(2):
            api.add_ico(f"{npc}_PackFlask{idx}", (.12 * s, (1.68 - idx * .18) * s, -.44 * s),
                        (.045 * s, .085 * s, .045 * s), mats["accent"], npc)

    elif kind in {"claws", "fists"}:
        for side in (-1, 1):
            label = api.side_label(side)
            if kind == "claws":
                for index in range(3):
                    replace(f"Claw{label}{index}")
                    x0 = (-.50 if side < 0 else .64) + (index - 1) * .055
                    shaped_limb(api, f"{npc}_HandClaw{index}{label}",
                                [(x0 * s, 1.22 * s, .155 * s),
                                 ((x0 + .018 * side) * s, 1.12 * s, .235 * s),
                                 ((x0 + .032 * side) * s, 1.00 * s, .30 * s)],
                                (.034 * s, .022 * s, .004 * s),
                                (.030 * s, .018 * s, .004 * s),
                                mats["bone"], npc, smooth=False)
            elif kind == "fists":
                # Bedrock tectonic gauntlets with glowing magma fissures
                replace(f"HandStone{label}")
                for index in range(3):
                    replace(f"Claw{label}{index}")
                api.add_ico(f"{npc}_GauntletStone{label}",
                            (side * (.54 if side < 0 else .68) * s, 1.20 * s, .11 * s),
                            (.115 * s, .120 * s, .110 * s), mats["cloth_shade"], npc)
                api.add_curve(f"{npc}_GauntletVein{label}", [
                    (side * (.54 if side < 0 else .68) * s, 1.28 * s, .08 * s),
                    (side * (.57 if side < 0 else .71) * s, 1.20 * s, .18 * s),
                    (side * (.52 if side < 0 else .66) * s, 1.12 * s, .12 * s)
                ], .018 * s, mats["accent"], npc)
