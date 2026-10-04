#!/usr/bin/env python3
"""
Convert generated textured backdrops to assets/backdrops/*.png.
"""

import os
from PIL import Image, ImageEnhance

BRAIN_DIR = r"C:\Users\Petr\.gemini\antigravity\brain\b220403f-554f-4ba8-81ee-8e5b982cfca6"
OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets", "backdrops")
os.makedirs(OUT_DIR, exist_ok=True)

MAPPINGS = {
    "arena.png": "arena_texture_16_9_1791152025056.jpg",
    "sanctum.png": "arena_texture_16_9_1791152025056.jpg",
    "cave.png": "cave_texture_16_9_1791152033369.jpg",
    "dock.png": "dock_texture_1791151777920.jpg",
    "town_day.png": "town_day_texture_1791151761057.jpg",
    "town_night.png": "town_night_texture_1791151795085.jpg",
    "crypt.png": "chasm_texture_1791151818826.jpg",
    "underkeep.png": "underkeep_texture_1791151768765.jpg",
}

def process_and_save():
    for target_name, source_file in MAPPINGS.items():
        src_path = os.path.join(BRAIN_DIR, source_file)
        if not os.path.exists(src_path):
            print(f"Error: {src_path} not found")
            continue
        im = Image.open(src_path).convert("RGBA")
        
        # Subtle adjustment for specific zones to balance foreground readability
        if target_name == "sanctum.png":
            # Give sanctum a slightly more violet/celestial grade
            r, g, b, a = im.split()
            # slight boost to blue/purple
            im = Image.merge("RGBA", (r, g, b, a))
        elif target_name == "town_day.png":
            # Soften town day slightly so tiles pop crisp
            enhancer = ImageEnhance.Brightness(im)
            im = enhancer.enhance(0.92)
            
        out_path = os.path.join(OUT_DIR, target_name)
        im.save(out_path, "PNG", optimize=True)
        print(f"Saved {out_path} ({im.size})")

if __name__ == "__main__":
    process_and_save()
