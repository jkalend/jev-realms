#!/usr/bin/env python3
"""Render the Laya Realms UI candidate screens: inventory, menu, NPC talk.

These are review mocks, not a runtime renderer: they paint the same chrome
palette as `tools/atlas/src/chrome.rs`, the same item plates as
`paint_item_icons.py`, and the real copy from `crates/view/src/modals.rs`, laid
out at the window size the Bevy view uses. The point is to agree on layout,
hierarchy and density before any of it is wired into `modals.rs`.

Run:
    python tools/asset3d/examples/make_ui_mockups.py \
      --items docs/gfx/proto/ui-v1/items/items.png \
      --output docs/gfx/proto/ui-v1/mockups
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

W, H = 1600, 1000

BRASS = (177, 131, 62)
BRASS_HI = (232, 191, 89)
BRASS_LO = (102, 72, 37)
STONE = (66, 66, 61)
STONE_DARK = (44, 42, 39)
INK = (9, 11, 12)
SEA = (102, 212, 196)
HP_RED = (168, 44, 40)
MANA_BLUE = (64, 92, 178)
IVORY = (231, 222, 198)
MUTED = (150, 145, 136)
GOLD = (205, 166, 79)
VIOLET = (150, 100, 176)

FONT_DIR = Path("assets/fonts")


def load_fonts() -> dict[str, ImageFont.FreeTypeFont]:
    cinzel = FONT_DIR / "Cinzel-Regular.ttf"
    faces = {
        "display": ImageFont.truetype(str(cinzel), 34),
        "title": ImageFont.truetype(str(cinzel), 22),
        "h": ImageFont.truetype(str(cinzel), 16),
        "body": ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 15),
        "small": ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 13),
        "tiny": ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 11),
        "num": ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 20),
    }
    return faces


F = load_fonts()


def backdrop(draw: ImageDraw.ImageDraw) -> None:
    """A dimmed, out-of-focus game frame so the panes read as overlays. Low
    contrast on purpose: anything busier competes with the panel chrome."""
    draw.rectangle((0, 0, W, H), fill=(12, 14, 18, 255))
    for row, y in enumerate(range(96, H - 60, 64)):
        band = 19 + (row % 3) * 2
        draw.rectangle((0, y, W, y + 44), fill=(band, band + 2, band + 1, 255))
        if row % 4 == 1:
            draw.rectangle((0, y + 18, W, y + 26), fill=(band + 4, band + 5, band + 3, 255))
    for row, y in enumerate(range(110, H - 70, 64)):
        for col, x in enumerate(range(24, W, 76)):
            if (row + col) % 3 == 0:
                continue
            tone = 25 if (row + col) % 2 else 33
            draw.rectangle((x, y, x + 58, y + 42), fill=(tone, tone + 4, tone + 1, 255))


def panel(draw: ImageDraw.ImageDraw, box, title: str | None = None, accent: tuple = BRASS) -> tuple[int, int, int, int]:
    """Brass-framed stone panel. Returns the inner content box."""
    x0, y0, x1, y1 = box
    draw.rectangle(box, fill=STONE_DARK + (242,), outline=BRASS + (255,), width=2)
    draw.rectangle((x0 + 4, y0 + 4, x1 - 4, y1 - 4), outline=STONE + (255,), width=1)
    for cx, cy in ((x0 + 9, y0 + 9), (x1 - 9, y0 + 9), (x0 + 9, y1 - 9), (x1 - 9, y1 - 9)):
        draw.rectangle((cx - 3, cy - 3, cx + 3, cy + 3), fill=accent + (255,))
    if title:
        band_h = 34
        draw.rectangle((x0 + 6, y0 + 6, x1 - 6, y0 + 6 + band_h), fill=STONE + (255,))
        draw.line((x0 + 6, y0 + 6 + band_h, x1 - 6, y0 + 6 + band_h), fill=BRASS_LO + (255,), width=1)
        draw.text((x0 + 16, y0 + 10), title, font=F["title"], fill=IVORY)
    return (x0 + 14, y0 + (48 if title else 14), x1 - 14, y1 - 14)


def slot(img: Image.Image, draw: ImageDraw.ImageDraw, box, icon: Image.Image | None, selected=False, count: int | None = None, equipped=False) -> None:
    x0, y0, x1, y1 = box
    fill = (30, 33, 38, 255) if not selected else (52, 45, 30, 255)
    draw.rectangle(box, fill=fill, outline=(BRASS_HI if selected else BRASS_LO) + (255,), width=2 if selected else 1)
    if equipped:
        draw.rectangle((x0 + 2, y1 - 6, x1 - 2, y1 - 2), fill=SEA + (255,))
    if icon is not None:
        edge = min(x1 - x0, y1 - y0) - 10
        art = icon.resize((edge, edge), Image.Resampling.NEAREST)
        img.alpha_composite(art, (x0 + (x1 - x0 - edge) // 2, y0 + (y1 - y0 - edge) // 2))
    if count:
        draw.text((x1 - 16, y1 - 20), str(count), font=F["small"], fill=IVORY)


def footer(draw: ImageDraw.ImageDraw, hints: list[tuple[str, str]]) -> None:
    x = 40
    for key, label in hints:
        w = 10 + 8 * len(key) + 6
        draw.rectangle((x, H - 56, x + w, H - 26), fill=STONE + (255,), outline=BRASS_LO + (255,), width=1)
        draw.text((x + 6, H - 52), key, font=F["tiny"], fill=BRASS_HI)
        draw.text((x + w + 8, H - 52), label, font=F["small"], fill=MUTED)
        x += w + 20 + 8 * len(label)


def load_items(path: Path) -> dict[str, Image.Image]:
    sheet = Image.open(path).convert("RGBA")
    manifest = json.loads(path.with_suffix(".json").read_text(encoding="utf-8"))
    cell = manifest["cell_size"]["width"]
    return {
        entry["name"]: sheet.crop(
            (entry["col"] * cell, entry["row"] * cell, (entry["col"] + 1) * cell, (entry["row"] + 1) * cell)
        )
        for entry in manifest["items"]
    }


# --------------------------------------------------------------------------- screens


def mock_inventory(items: dict[str, Image.Image]) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    backdrop(draw)

    # equipped column
    box = panel(draw, (40, 40, 330, 700), "EQUIPPED")
    equipped = [("Weapon.Fine", "FINE BLADE"), ("Armour.Standard", "HAUBERK"), ("BossRelic.Stoneheart", "RELIC"), ("Glyph.Fen", "WORD: FEN")]
    y = box[1]
    for key, label in equipped:
        slot(img, draw, (box[0], y, box[0] + 92, y + 92), items.get(key), equipped=True)
        draw.text((box[0] + 102, y + 30), label, font=F["small"], fill=GOLD)
        draw.text((box[0] + 102, y + 50), key.split(".")[-1][:12], font=F["tiny"], fill=MUTED)
        y += 104
    draw.line((box[0], y + 4, box[2], y + 4), fill=BRASS_LO + (255,), width=1)
    draw.text((box[0], y + 16), "KEEPWARDEN  Lv 3", font=F["h"], fill=IVORY)
    draw.text((box[0], y + 42), "45 / 120 XP      20/20 HP", font=F["small"], fill=MUTED)
    draw.text((box[0], y + 60), "ATK 7   DEF 4   SIGILS 1/3", font=F["small"], fill=MUTED)
    y += 92
    draw.line((box[0], y, box[2], y), fill=BRASS_LO + (255,), width=1)
    draw.text((box[0], y + 12), "IF YOU SWAP", font=F["small"], fill=MUTED)
    draw.text((box[0], y + 32), "Blade  ->  Masterwork", font=F["small"], fill=IVORY)
    draw.text((box[0], y + 52), "ATK 7 -> 10", font=F["body"], fill=SEA)
    draw.text((box[0], y + 74), "Carrying 21.4 / 40", font=F["tiny"], fill=GOLD)

    # satchel grid
    box = panel(draw, (352, 40, 1010, 700), "SATCHEL   12/20 SLOTS   340 GOLD")
    cols, size, gap = 5, 118, 8
    gx, gy = box[0], box[1]
    for i in range(20):
        col, row = i % cols, i // cols
        x = gx + col * (size + gap)
        y = gy + row * (size + gap)
        key = [
            "Potion", "Ration", "Torch", "Weapon.Masterwork", "Armour.Masterwork",
            "Relic", "GemDust", "HerbCluster", "OreFlake", "GlyphShard",
            "Glyph.Ash", "Glyph.Crown", "Key.Burrow", "Delivery.Millbrook", "CaravanGoods",
            "BossRelic.Hartshorn", "GreaterPotion", "Essence",
        ][i] if i < 18 else None
        counts = {"Potion": 4, "OreFlake": 7, "GemDust": 12, "Ration": 2}
        slot(img, draw, (x, y, x + size, y + size), items.get(key) if key else None, selected=(i == 6), count=counts.get(key or ""))
    draw.text((box[0], gy + 4 * (size + gap) + 6), "Carrying 21.4 / 40 weight", font=F["small"], fill=GOLD)

    # transmute bench
    box = panel(draw, (352, 722, 1010, H - 80), "TRANSMUTE BENCH")
    y = box[1]
    draw.text((box[0], y), "Insert  reagents to raise a gear tier. Words socket on Fine+ only.", font=F["small"], fill=MUTED)
    y += 26
    for i, (name, have, need) in enumerate((("OreFlake", "7", "3"), ("GemDust", "12", "2"), ("GlyphShard", "3", "2"))):
        x = box[0] + i * 130
        slot(img, draw, (x, y, x + 72, y + 72), items.get(name), selected=(i == 1))
        draw.text((x, y + 78), f"have {have}  need {need}", font=F["tiny"], fill=MUTED)
        draw.rectangle((x, y + 98, x + 72, y + 104), fill=(30, 36, 34, 255), outline=BRASS_LO + (255,), width=1)
        draw.rectangle((x + 2, y + 100, x + 2 + int(68 * min(1, int(have) / int(need)) / 2), y + 102), fill=SEA if int(have) >= int(need) else HP_RED)
    draw.text((box[2] - 210, y + 24), "REFINE", font=F["title"], fill=BRASS_HI, anchor="ra")
    draw.text((box[2] - 210, y + 52), "1 shard short", font=F["small"], fill=HP_RED, anchor="ra")

    # detail panel
    box = panel(draw, (1062, 40, W - 40, H - 80), "SELECTED")
    y = box[1]
    draw.text((box[0], y), "GEM DUST", font=F["display"], fill=IVORY)
    y += 44
    draw.text((box[0], y), "alchemist's grit", font=F["h"], fill=GOLD)
    y += 30
    for line, colour in (
        ("TYPE", MUTED), ("transmuter reagent", IVORY), ("", MUTED),
        ("EFFECT", MUTED), ("+1 gear quality on refine", IVORY), ("", MUTED),
        ("SOURCE", MUTED), ("Vendor, Highgate (rotates)", IVORY), ("", MUTED),
        ("VALUE", MUTED), ("18 gold", GOLD), ("", MUTED),
        ("WEIGHT", MUTED), ("0.4   carries 4", IVORY),
    ):
        draw.text((box[0], y), line, font=F["body"] if colour is IVORY else F["small"], fill=colour)
        y += 22 if colour is IVORY else 18
    y += 12
    draw.rectangle((box[0], y, box[2], y + 74), fill=(58, 46, 26, 255), outline=BRASS_LO + (255,), width=1)
    draw.text((box[0] + 10, y + 8), "TRANSMUTE", font=F["h"], fill=BRASS_HI)
    draw.text((box[0] + 10, y + 34), "needs 2x Glyph Shard", font=F["small"], fill=MUTED)
    y += 92
    draw.text((box[0], y), "STACK", font=F["small"], fill=MUTED)
    for i, name in enumerate(("GemDust", "GlyphShard", "OreFlake")):
        slot(img, draw, (box[0] + i * 60, y + 20, box[0] + i * 60 + 52, y + 72), items.get(name), count=(12, 3, 7)[i])
    y += 110
    draw.line((box[0], y, box[2], y), fill=BRASS_LO + (255,), width=1)
    y += 14
    draw.text((box[0], y), "ACTIONS", font=F["h"], fill=GOLD)
    y += 30
    for index, (label, note, enabled) in enumerate((
        ("Use", "no use for a reagent", False),
        ("Equip", "not a gear slot", False),
        ("Transmute", "2x Glyph Shard", True),
        ("Drop", "recover 18 gold", True),
    )):
        ry = y + index * 54
        draw.rectangle((box[0], ry, box[2], ry + 46), fill=(40, 34, 24, 255) if enabled else (26, 26, 28, 255), outline=(BRASS_LO if enabled else (58, 56, 52)) + (255,), width=1)
        draw.text((box[0] + 14, ry + 6), label, font=F["h"], fill=IVORY if enabled else (96, 92, 86))
        draw.text((box[0] + 14, ry + 27), note, font=F["tiny"], fill=MUTED if enabled else (78, 76, 72))
        if not enabled:
            draw.line((box[0] + 8, ry + 30, box[0] + 96, ry + 30), fill=(58, 56, 52, 255), width=1)

    footer(draw, [("I", "inventory"), ("Enter", "use / equip"), ("Del", "drop"), ("Esc", "close")])
    return img


def mock_menu(_items: dict[str, Image.Image]) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    backdrop(draw)
    draw.rectangle((0, 0, W, 120), fill=(10, 12, 16, 220))

    draw.text((W // 2, 26), "CAMPFIRE", font=F["display"], fill=IVORY, anchor="ma")
    draw.text((W // 2, 72), "paused - the Vigil keeps no clock while you rest", font=F["small"], fill=MUTED, anchor="ma")

    box = panel(draw, (300, 150, 900, 620), "PAUSE")
    entries = [
        ("Resume", "return to the road"),
        ("Save journey", "saves/journey-42.json"),
        ("Load journey", "3 slots"),
        ("Options", "display, keys, audio"),
        ("Field guide", "controls and systems"),
        ("Abandon run", "progress is kept; you restart at the gate"),
    ]
    y = box[1] + 6
    for index, (label, note) in enumerate(entries):
        selected = index == 1
        if selected:
            draw.rectangle((box[0] - 6, y - 6, box[2] + 6, y + 40), fill=(58, 46, 26, 255), outline=BRASS_HI + (255,), width=1)
            draw.text((box[0], y + 8), ">", font=F["body"], fill=BRASS_HI)
        draw.text((box[0] + 24, y + 4), label, font=F["title"], fill=IVORY if not selected else BRASS_HI)
        draw.text((box[0] + 24, y + 26), note, font=F["small"], fill=MUTED)
        y += 52

    box = panel(draw, (930, 150, W - 300, 620), "RUN")
    y = box[1]
    for label, value in (
        ("SEED", "42"), ("MAP", "Millbrook"), ("CLOCK", "08:00 day"),
        ("PLAYER", "Keepwarden Lv 3"), ("SIGILS", "0 / 3"),
        ("GOLD", "340"), ("DECISIONS", "0"),
    ):
        draw.text((box[0], y), label, font=F["small"], fill=MUTED)
        draw.text((box[2], y), value, font=F["body"], fill=IVORY, anchor="ra")
        draw.line((box[0], y + 22, box[2], y + 22), fill=(52, 48, 42, 255), width=1)
        y += 30
    y += 12
    draw.text((box[0], y), "TOGGLES", font=F["h"], fill=GOLD)
    y += 26
    for label, on in (("Bloom", True), ("Vignette", True), ("Edge pan", True), ("Show grid", False)):
        draw.text((box[0], y), label, font=F["body"], fill=IVORY)
        track = (box[2] - 56, y, box[2], y + 20)
        draw.rounded_rectangle(track, radius=10, fill=(86, 200, 150) if on else (62, 58, 52))
        knob_x = track[2] - 26 if on else track[0] + 6
        draw.ellipse((knob_x, y + 3, knob_x + 18, y + 21), fill=IVORY if on else (128, 122, 112))
        y += 30

    footer(draw, [("W,S", "move"), ("Enter", "choose"), ("Esc", "resume")])
    return img


def mock_talk(items: dict[str, Image.Image]) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    backdrop(draw)

    # speaker plate
    box = panel(draw, (40, 40, 700, 200), None, accent=GOLD)
    draw.text((box[0] + 8, box[1] + 2), "MARA", font=F["display"], fill=IVORY)
    draw.text((box[0] + 8, box[1] + 44), "Millbrook provisioner", font=F["h"], fill=GOLD)
    for i, (label, colour) in enumerate((("AVAILABLE", SEA), ("GIVES", GOLD), ("REPUTATION 50", MUTED))):
        x = box[0] + 8 + i * 150
        draw.rectangle((x, box[1] + 76, x + 132, box[1] + 100), fill=(30, 36, 38, 255), outline=colour + (255,), width=1)
        draw.text((x + 10, box[1] + 80), label, font=F["tiny"], fill=colour)
    draw.text((box[2], box[1] + 2), "TALK", font=F["h"], fill=BRASS_HI, anchor="ra")
    draw.text((box[2], box[1] + 24), "E to close", font=F["small"], fill=MUTED, anchor="ra")

    # dialogue
    box = panel(draw, (40, 222, 1140, 762), None, accent=GOLD)
    y = box[1] + 4
    for line in (
        "You made good time from the gate. The Burrow seal is still",
        "cold in your pack, and I will not pretend that surprises me.",
        "",
        "Clear her cellar before the tide turns and the road opens again.",
        "Forty-five coin, and my thanks - which in Millbrook is currency too.",
        "",
        "Bring the seal back here when it is done. Do not open it on the road.",
    ):
        draw.text((box[0], y), line, font=F["body"], fill=IVORY)
        y += 26
    y += 8
    draw.line((box[0], y, box[2], y), fill=BRASS_LO + (255,), width=1)
    y += 14
    draw.text((box[0], y), "REPUTATION", font=F["small"], fill=MUTED)
    draw.text((box[0] + 130, y), "50 / 50  (trusted)", font=F["body"], fill=SEA)
    draw.rectangle((box[0], y + 26, box[0] + 520, y + 34), fill=(30, 36, 34, 255), outline=BRASS_LO + (255,), width=1)
    draw.rectangle((box[0] + 2, y + 28, box[0] + 516, y + 32), fill=SEA)
    draw.text((box[0] + 560, y), "DEADLINE", font=F["small"], fill=MUTED)
    draw.text((box[0] + 690, y), "tide turns in 6 days", font=F["body"], fill=GOLD)
    y += 54
    draw.line((box[0], y, box[2], y), fill=BRASS_LO + (255,), width=1)
    y += 16
    draw.text((box[0], y), "ON COMPLETION", font=F["h"], fill=GOLD)
    y += 32
    for index, (name, amount, note) in enumerate((
        ("Key.Burrow", "the Burrow seal", "opens the underkeep gate"),
        ("Glyph.Gate", "gate glyph", "sockets on Fine+ gear"),
    )):
        x = box[0] + index * 420
        slot(img, draw, (x, y, x + 76, y + 76), items.get(name))
        draw.text((x + 90, y + 14), amount, font=F["h"], fill=IVORY)
        draw.text((x + 90, y + 38), note, font=F["small"], fill=MUTED)
    draw.text((box[2] - 250, y + 6), "45 GOLD", font=F["title"], fill=GOLD, anchor="ra")
    draw.text((box[2] - 250, y + 40), "45 XP   +1 REPUTATION", font=F["small"], fill=SEA, anchor="ra")

    # choices
    box = panel(draw, (1162, 222, W - 40, 762), "CHOICES")
    y = box[1]
    for index, (label, note) in enumerate((
        ("Accept the delivery", "45 gold  -  +1 Burrow access"),
        ("Ask about the cellar", "free"),
        ("Trade", "her stock rotates daily"),
        ("Goodbye", "closes the pane"),
    )):
        selected = index == 0
        if selected:
            draw.rectangle((box[0] - 4, y - 4, box[2] + 4, y + 54), fill=(58, 46, 26, 255), outline=BRASS_HI + (255,), width=1)
            draw.text((box[0], y + 10), ">", font=F["body"], fill=BRASS_HI)
        draw.text((box[0] + 22, y + 2), label, font=F["h"], fill=IVORY if not selected else BRASS_HI)
        draw.text((box[0] + 22, y + 24), note, font=F["small"], fill=MUTED)
        y += 66

    # quest thread
    box = panel(draw, (40, 784, W - 40, 930), "THREAD   ROADS CONNECT ALL LANDMARKS")
    y = box[1]
    for label, state, colour in (
        ("Clear her cellar, then report for 45 gold, 45 XP and the Burrow seal.", "AVAILABLE", SEA),
        ("Open the Burrow seal at the underkeep gate.", "LOCKED", MUTED),
        ("Question the arbiter about the First Writ.", "LOCKED", MUTED),
    ):
        draw.text((box[0], y), state, font=F["tiny"], fill=colour)
        draw.text((box[0] + 100, y - 2), label, font=F["body"], fill=IVORY)
        y += 30

    footer(draw, [("W,S", "choose"), ("Enter", "confirm"), ("T", "trade"), ("Esc", "leave")])
    return img


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--items", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    items = load_items(args.items)
    args.output.mkdir(parents=True, exist_ok=True)
    for name, screen in (
        ("ui-inventory", mock_inventory),
        ("ui-menu", mock_menu),
        ("ui-talk", mock_talk),
    ):
        path = args.output / f"{name}.png"
        screen(items).convert("RGB").save(path)
        print("ui-mockup", path)


if __name__ == "__main__":
    main()
