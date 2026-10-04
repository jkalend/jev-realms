#!/usr/bin/env python3
"""
Generate environmental atmospheric backdrops for all game zones in Jev Realms.
Saves 512x512 RGBA textures to assets/backdrops/.
"""

import math
import os
import random
from PIL import Image, ImageDraw

OUTPUT_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets", "backdrops")
os.makedirs(OUTPUT_DIR, exist_ok=True)

def lerp(c1, c2, t):
    t = max(0.0, min(1.0, t))
    return tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))

def create_backdrop(
    name: str,
    top_color: tuple,
    horizon_color: tuple,
    bottom_color: tuple,
    horizon_y: float = 0.55,
    haze_intensity: float = 0.15,
    particles: list = None,
    mist_bands: bool = False,
    seed: int = 42,
):
    rng = random.Random(seed)
    width, height = 512, 512
    img = Image.new("RGBA", (width, height))
    pixels = img.load()

    for y in range(height):
        ny = y / float(height - 1)
        if ny < horizon_y:
            # Sky / Ceiling to Horizon
            t = ny / horizon_y
            # smoothstep
            t = t * t * (3.0 - 2.0 * t)
            row_color = lerp(top_color, horizon_color, t)
        else:
            # Horizon to Abyss / Floor
            t = (ny - horizon_y) / (1.0 - horizon_y)
            t = t * t * (3.0 - 2.0 * t)
            row_color = lerp(horizon_color, bottom_color, t)

        for x in range(width):
            nx = (x - width * 0.5) / (width * 0.5)
            # Subtle radial vignette toward the corners
            dist = math.sqrt(nx * nx + (ny - horizon_y) * (ny - horizon_y) * 1.2)
            vignette = 1.0 - dist * 0.18
            vignette = max(0.75, min(1.0, vignette))

            # Horizontal haze curve
            haze = math.exp(-((ny - horizon_y) * 4.0) ** 2) * haze_intensity
            
            r = int(min(255, max(0, row_color[0] * vignette + haze * 255)))
            g = int(min(255, max(0, row_color[1] * vignette + haze * 255)))
            b = int(min(255, max(0, row_color[2] * vignette + haze * 255)))
            pixels[x, y] = (r, g, b, 255)

    from PIL import ImageFilter

    # Subtle layered mist bands if requested
    if mist_bands:
        mist_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        mist_draw = ImageDraw.Draw(mist_layer)
        for _ in range(6):
            my = rng.uniform(horizon_y - 0.12, horizon_y + 0.20) * height
            mw = rng.uniform(width * 0.7, width * 1.4)
            mh = rng.uniform(24, 52)
            mx = rng.uniform(width * 0.2, width * 0.8)
            alpha = rng.randint(14, 32)
            r = min(255, horizon_color[0] + 35)
            g = min(255, horizon_color[1] + 35)
            b = min(255, horizon_color[2] + 45)
            mist_draw.ellipse([mx - mw * 0.5, my - mh * 0.5, mx + mw * 0.5, my + mh * 0.5], fill=(r, g, b, alpha))
        mist_layer = mist_layer.filter(ImageFilter.GaussianBlur(radius=18))
        img = Image.alpha_composite(img, mist_layer)

    # Particle sparkles (stars, embers, soul motes)
    if particles:
        sparkle_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        sp_draw = ImageDraw.Draw(sparkle_layer)
        for ptype, count, p_color in particles:
            for _ in range(count):
                px = rng.randint(0, width - 1)
                py = rng.randint(0, height - 1)
                size = rng.choice([1, 1, 2])
                alpha = rng.randint(50, 180)
                if ptype == "star" and py > height * (horizon_y + 0.1):
                    continue # Stars mostly above horizon
                if ptype == "ember" and py < height * 0.35:
                    continue # Embers mostly below / near heat
                color_with_alpha = (p_color[0], p_color[1], p_color[2], alpha)
                sp_draw.rectangle([px, py, px + size - 1, py + size - 1], fill=color_with_alpha)
        img = Image.alpha_composite(img, sparkle_layer)

    path = os.path.join(OUTPUT_DIR, f"{name}.png")
    img.save(path)
    print(f"Generated {path}")

def main():
    print("Generating environmental backdrops...")

    # 1. Town & Woodland - Daytime
    # Soft atmosphere: azure sky -> hazy sun horizon -> warm ground fog
    create_backdrop(
        "town_day",
        top_color=(42, 68, 92),       # #2a445c Azure sky
        horizon_color=(90, 122, 142), # #5a7a8e Warm atmospheric horizon haze
        bottom_color=(36, 48, 56),    # #243038 Deep earth/shadow
        horizon_y=0.52,
        haze_intensity=0.12,
        mist_bands=True,
        seed=101,
    )

    # 2. Town & Woodland - Night
    # Deep midnight indigo -> dusk teal-slate horizon -> dark shadow, with starlight
    create_backdrop(
        "town_night",
        top_color=(10, 14, 26),      # #0a0e1a Deep indigo
        horizon_color=(24, 38, 54),   # #182636 Dusky teal horizon
        bottom_color=(11, 15, 20),    # #0b0f14 Abyssal ground shadow
        horizon_y=0.55,
        haze_intensity=0.06,
        particles=[("star", 90, (210, 230, 255))],
        seed=102,
    )

    # 3. Cave / Crag Ridge Den
    # Volcanic caldera: deep basalt charcoal -> smoldering caldera ember glow -> magma shadow
    create_backdrop(
        "cave",
        top_color=(20, 15, 12),       # #140f0c Deep basalt ceiling
        horizon_color=(48, 24, 16),   # #301810 Smoldering caldera heat glow
        bottom_color=(28, 14, 10),    # #1c0e0a Magma abyss
        horizon_y=0.58,
        haze_intensity=0.10,
        mist_bands=True,
        particles=[("ember", 60, (255, 140, 50))],
        seed=103,
    )

    # 4. Crypt / Fen Barrow / Cellar
    # Subterranean catacomb: damp charcoal-slate -> humid moss-teal mist -> dark bedrock
    create_backdrop(
        "crypt",
        top_color=(11, 18, 16),       # #0b1210 Dark catacomb ceiling
        horizon_color=(22, 42, 36),   # #162a24 Damp subterranean moss-teal haze
        bottom_color=(12, 20, 18),    # #0c1412 Sunken bone trench
        horizon_y=0.54,
        haze_intensity=0.08,
        mist_bands=True,
        seed=104,
    )

    # 5. The Underkeep (Lich Crypt)
    # Cold necromantic abyss: deep obsidian violet -> ethereal soul-indigo mist -> midnight chasm
    create_backdrop(
        "underkeep",
        top_color=(14, 11, 24),       # #0e0b18 Obsidian violet
        horizon_color=(34, 24, 52),   # #221834 Spectral soul-indigo haze
        bottom_color=(16, 12, 26),    # #100c1a Underkeep chasm floor
        horizon_y=0.56,
        haze_intensity=0.08,
        mist_bands=True,
        particles=[("star", 50, (180, 160, 255))], # Soul dust motes
        seed=105,
    )

    # 6. Arena / Final Trial — The Hall of Verdicts
    # Imperial astral court: velvet starlight black -> majestic imperial purple-gold horizon -> royal void
    create_backdrop(
        "arena",
        top_color=(11, 9, 20),        # #0b0914 Imperial obsidian
        horizon_color=(30, 20, 48),   # #1e1430 Royal judicial amethyst horizon
        bottom_color=(14, 10, 24),    # #0e0a18 Astral abyss
        horizon_y=0.52,
        haze_intensity=0.09,
        mist_bands=True,
        particles=[("star", 75, (240, 215, 150))], # Celestial gold dust
        seed=106,
    )

    # 7. Dock / Saltmarsh Landing
    # Oceanic coastal sea-fog: maritime slate-navy -> coastal sea-mist -> deep oceanic trench
    create_backdrop(
        "dock",
        top_color=(10, 18, 26),       # #0a121a Maritime navy
        horizon_color=(28, 48, 62),   # #1c303e Coastal sea-fog horizon
        bottom_color=(12, 22, 30),    # #0c161e Deep water abyss
        horizon_y=0.53,
        haze_intensity=0.12,
        mist_bands=True,
        seed=107,
    )

    # 8. Sanctum / Arcane Depths
    # Arcane cosmic void: deep twilight -> radiant violet-magenta nebula -> ether abyss
    create_backdrop(
        "sanctum",
        top_color=(15, 10, 26),       # #0f0a1a Dark ether
        horizon_color=(40, 22, 60),   # #28163c Arcane nebula glow
        bottom_color=(18, 12, 32),    # #120c20 Cosmic void
        horizon_y=0.55,
        haze_intensity=0.10,
        mist_bands=True,
        particles=[("star", 65, (220, 180, 255))],
        seed=108,
    )

    print("All backdrops generated successfully!")

if __name__ == "__main__":
    main()
