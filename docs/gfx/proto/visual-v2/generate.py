from __future__ import annotations

import json
import math
import random
from pathlib import Path


from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter, ImageFont, ImageStat

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
CONTACT_2D = ROOT / "docs/gfx/proto/atlas-actors-contact.png"
CURRENT_3D = ROOT / "assets/atlas/actors.png"
MAP_FIXTURE = OUT / "fixtures/millbrook-seed42.json"
DISPLAY_FONT = ROOT / "assets/fonts/Cinzel-Regular.ttf"
BODY_FONT = Path("C:/Windows/Fonts/segoeui.ttf")
MONO_FONT = Path("C:/Windows/Fonts/consola.ttf")

BG = (19, 21, 27)
BG_2 = (27, 29, 35)
INK = (232, 222, 197)
MUTED = (148, 145, 133)
GOLD = (205, 166, 79)
RED = (176, 53, 57)
BLUE = (55, 112, 176)
GREEN = (76, 133, 82)

ACTOR_CELLS = [
    ("Commoner", 0, 0), ("Vendor", 1, 0), ("Guard", 2, 0), ("Thief", 3, 0),
    ("Traveller", 4, 0), ("Bandit", 5, 0), ("Wolf", 6, 0), ("Bear", 7, 0),
    ("Rat", 0, 1), ("Skeleton", 1, 1), ("Chief", 2, 1), ("Matriarch", 3, 1),
    ("Lich", 4, 1), ("Adjudicator", 5, 1), ("Smuggler", 6, 1), ("Oracle", 7, 1),
    ("Companion", 0, 2), ("Tidemother", 1, 2), ("Cragmother", 2, 2),
    ("GnawThane", 3, 2), ("Tollmaster", 4, 2), ("Mirelight", 5, 2),
    ("PaleStag", 6, 2), ("Alchemist", 7, 2),
    ("Mirelight", 1, 3), ("Alchemist", 2, 3), ("PaleStag.at_bay", 4, 3),
]

CURRENT_3D_CELLS = {
    "Commoner": (0, 0), "Vendor": (1, 0), "Guard": (2, 0), "Thief": (3, 0),
    "Traveller": (4, 0), "Bandit": (5, 0), "Wolf": (6, 0), "Bear": (7, 0),
    "Rat": (0, 1), "Skeleton": (1, 1), "Chief": (2, 1), "Matriarch": (3, 1),
    "Lich": (4, 1), "Adjudicator": (5, 1), "Smuggler": (6, 1), "Oracle": (7, 1),
    "Companion": (0, 2), "Tidemother": (1, 2), "Cragmother": (2, 2),
    "GnawThane": (3, 2), "Tollmaster": (4, 2), "Mirelight": (5, 2),
    "PaleStag": (6, 2), "Alchemist": (7, 2),
    "Keepwarden": (1, 4), "Gravebound": (2, 4), "Redwake": (3, 4),
    "Waysworn": (4, 4), "SigilSworn": (5, 4), "Fensworn": (6, 4),
}

REVIEW_ACTORS = [
    "Commoner", "Guard", "Skeleton", "Oracle", "Tidemother", "Cragmother",
    "Adjudicator", "PaleStag", "GnawThane", "Mirelight", "Tollmaster",
]


def clamp(v: float, lo: float = 0.0, hi: float = 255.0) -> int:
    return int(max(lo, min(hi, round(v))))


def mix(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[int, int, int]:
    return tuple(clamp(a[i] * (1.0 - t) + b[i] * t) for i in range(3))  # type: ignore[return-value]


def shade(c: tuple[int, int, int], k: float) -> tuple[int, int, int]:
    return tuple(clamp(v * k) for v in c)  # type: ignore[return-value]


def font(size: int, display: bool = False, mono: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [DISPLAY_FONT] if display else ([MONO_FONT] if mono else [BODY_FONT, DISPLAY_FONT])
    for candidate in candidates:
        if candidate.exists():
            return ImageFont.truetype(str(candidate), size)
    return ImageFont.load_default()


def alpha_bbox(image: Image.Image) -> tuple[int, int, int, int]:
    bbox = image.getchannel("A").getbbox()
    return bbox or (0, 0, image.width, image.height)


def trim(image: Image.Image, pad: int = 0) -> Image.Image:
    box = alpha_bbox(image)
    if box == (0, 0, image.width, image.height) and pad == 0:
        return image
    x0, y0, x1, y1 = box
    x0 = max(0, x0 - pad)
    y0 = max(0, y0 - pad)
    x1 = min(image.width, x1 + pad)
    y1 = min(image.height, y1 + pad)
    return image.crop((x0, y0, x1, y1))


def contact_cell(contact: Image.Image, col: int, row: int) -> Image.Image:
    # The contact generator uses 72px slots, 4px padding, then a 2x export.
    x = col * 144 + 8
    y = row * 144 + 8
    return contact.crop((x, y, x + 128, y + 128)).resize((64, 64), Image.Resampling.NEAREST)


def build_grass_background(contact: Image.Image) -> Image.Image:
    samples: list[list[tuple[int, int, int]]] = [[] for _ in range(64 * 64)]
    seen: set[tuple[int, int]] = set()
    for _, col, row in ACTOR_CELLS:
        if (col, row) in seen:
            continue
        seen.add((col, row))
        tile = contact_cell(contact, col, row)
        for y in range(64):
            for x in range(64):
                samples[y * 64 + x].append(tile.getpixel((x, y))[:3])
    background = Image.new("RGBA", (64, 64))
    out = background.load()
    for y in range(64):
        for x in range(64):
            values = samples[y * 64 + x]
            channels = []
            for c in range(3):
                ordered = sorted(v[c] for v in values)
                channels.append(ordered[len(ordered) // 2])
            out[x, y] = (*channels, 255)
    return background


def extract_2d_sprite(contact: Image.Image, background: Image.Image, col: int, row: int) -> Image.Image:
    tile = contact_cell(contact, col, row)
    out = Image.new("RGBA", tile.size)
    for y in range(64):
        for x in range(64):
            r, g, b, _ = tile.getpixel((x, y))
            br, bg, bb, _ = background.getpixel((x, y))
            dist = math.sqrt((r - br) ** 2 + (g - bg) ** 2 + (b - bb) ** 2)
            if dist <= 7:
                alpha = 0
            elif dist >= 25:
                alpha = 255
            else:
                alpha = clamp((dist - 7) / 18 * 255)
            out.putpixel((x, y), (r, g, b, alpha))
    # The source is hard-edged pixel art. Restore tiny antialias gaps without
    # bloating the silhouette.
    alpha = out.getchannel("A").point(lambda a: 255 if a >= 72 else 0)
    out.putalpha(alpha)
    return out


def extract_current_3d(name: str) -> Image.Image:
    col, row = CURRENT_3D_CELLS[name]
    with Image.open(CURRENT_3D) as atlas:
        return atlas.convert("RGBA").crop((col * 64, row * 64, col * 64 + 64, row * 64 + 64))


def offset_mask(mask: Image.Image, dx: int, dy: int) -> Image.Image:
    return ImageChops.offset(mask, dx, dy)


def make_sculpted_sprite(source: Image.Image, seed: int = 0, depth: int = 7) -> Image.Image:
    """2.5D pre-render: approved pixel front + sculpted silhouette depth.

    This is intentionally not a production 64px plate. The 96px result is a
    review-scale bake that tests whether 2D identity can survive a real depth
    silhouette, directional face lighting, and grounded contact shadow.
    """
    source = trim(source)
    front = ImageEnhance.Color(source).enhance(1.08)
    front = ImageEnhance.Contrast(front).enhance(1.055)
    front = ImageEnhance.Brightness(front).enhance(1.025)
    w, h = front.size
    canvas = Image.new("RGBA", (96, 96), (0, 0, 0, 0))
    x = (96 - w) // 2 - 2
    y = max(3, 5 + max(0, 64 - h) // 5)

    avg = tuple(ImageStat.Stat(front.convert("RGB")).mean[:3])
    avg_int = (clamp(avg[0]), clamp(avg[1]), clamp(avg[2]))
    silhouette = Image.new("L", canvas.size, 0)
    silhouette.paste(front.getchannel("A"), (x, y))

    # Soft, offset contact shadow. It remains inside the transparent 96px cell.
    shadow = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    shadow_w = max(28, min(58, w + 8))
    sd.ellipse((x + (w - shadow_w) // 2, y + h - 8, x + (w + shadow_w) // 2, y + h + 6), fill=(3, 5, 8, 132))
    shadow = shadow.filter(ImageFilter.GaussianBlur(4.2))
    canvas.alpha_composite(shadow)

    # Contour extrusion. Each layer is a true shifted silhouette rather than
    # a blurred drop shadow, so it reads as a solid side at sprite scale.
    for d in range(depth, 0, -1):
        t = d / max(1, depth)
        side = mix(shade(avg_int, 0.24), (18, 24, 31), 0.48 * t)
        side = mix(side, shade(avg_int, 0.42), (1.0 - t) * 0.28)
        dx = round(d * 0.92)
        dy = round(d * 0.46)
        mask = offset_mask(silhouette, dx, dy)
        edge_fade = clamp(145 + (depth - d) * 14)
        mask = mask.point(lambda a, e=edge_fade: a * e // 255)
        layer = Image.new("RGBA", canvas.size, side + (0,))
        layer.putalpha(mask)
        canvas.alpha_composite(layer)

    # A near-black silhouette seat keeps the front pixel outline crisp.
    dilated = silhouette.filter(ImageFilter.MaxFilter(5))
    seat_mask = ImageChops.subtract(dilated, silhouette)
    seat = Image.new("RGBA", canvas.size, (5, 7, 10, 0))
    seat.putalpha(seat_mask.point(lambda a: a * 175 // 255))
    canvas.alpha_composite(seat)

    placed = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    placed.alpha_composite(front, (x, y))
    canvas.alpha_composite(placed)

    # Directional bevel around the retained pixel silhouette. The source already
    # owns its black outline; these are narrow glints, not a replacement edge.
    front_alpha = placed.getchannel("A")
    upper_ring = ImageChops.subtract(offset_mask(front_alpha, -1, -1), front_alpha)
    lower_ring = ImageChops.subtract(offset_mask(front_alpha, 1, 1), front_alpha)
    upper = Image.new("RGBA", canvas.size, (230, 211, 166, 0))
    upper.putalpha(upper_ring.point(lambda a: a * 105 // 255))
    lower = Image.new("RGBA", canvas.size, (5, 8, 13, 0))
    lower.putalpha(lower_ring.point(lambda a: a * 125 // 255))
    canvas.alpha_composite(lower)
    canvas.alpha_composite(upper)

    # Restrained warm key on the upper-left-facing pixels.
    px = front.load()
    keyed = front.copy()
    kp = keyed.load()
    for yy in range(h):
        for xx in range(w):
            r, g, b, a = px[xx, yy]
            if not a:
                continue
            fall = max(0.0, 1.0 - ((xx / max(1, w - 1)) * 0.58 + (yy / max(1, h - 1)) * 0.72))
            lift = fall * 0.075
            kp[xx, yy] = (clamp(r + (245 - r) * lift), clamp(g + (225 - g) * lift), clamp(b + (180 - b) * lift), a)
    keyed_placed = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    keyed_placed.alpha_composite(keyed, (x, y))
    canvas.alpha_composite(keyed_placed)

    # Fine deterministic color texture keeps large flats from looking plastic.
    rng = random.Random(seed * 977 + w * 31 + h)
    texture = Image.new("RGBA", front.size, (0, 0, 0, 0))
    tp = texture.load()
    for yy in range(h):
        for xx in range(w):
            r, g, b, a = front.getpixel((xx, yy))
            if not a:
                continue
            n = rng.choice((-2, -1, 0, 0, 0, 1, 2))
            tp[xx, yy] = (clamp(r + n), clamp(g + n), clamp(b + n), 36)
    textured = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    textured.alpha_composite(texture, (x, y))
    canvas.alpha_composite(textured)
    return canvas


def make_actor_sheet(sprites: dict[str, Image.Image]) -> Image.Image:
    sheet = Image.new("RGBA", (8 * 96, 4 * 96), (0, 0, 0, 0))
    for name, col, row in ACTOR_CELLS:
        key = name.split(".")[0]
        if name == "PaleStag.at_bay":
            source = sprites["PaleStag.at_bay"]
        else:
            source = sprites[key]
        sheet.alpha_composite(make_sculpted_sprite(source, seed=ACTOR_CELLS.index((name, col, row))), (col * 96, row * 96))
    return sheet


def add_grain(image: Image.Image, seed: int, opacity: int = 10) -> Image.Image:
    rng = random.Random(seed)
    noise = Image.new("RGBA", image.size, (0, 0, 0, 0))
    pixels = noise.load()
    for y in range(image.height):
        for x in range(image.width):
            v = rng.randrange(0, 256)
            pixels[x, y] = (v, v, v, opacity)
    return Image.alpha_composite(image, noise)


def draw_text(draw: ImageDraw.ImageDraw, xy: tuple[float, float], text: str, size: int,
              fill: tuple[int, int, int] = INK, display: bool = False, mono: bool = False,
              anchor: str | None = None) -> None:
    draw.text(xy, text, font=font(size, display, mono), fill=fill, anchor=anchor)


def paste_contain(dst: Image.Image, src: Image.Image, box: tuple[int, int, int, int], scale: int = 1) -> None:
    x0, y0, x1, y1 = box
    target = (x1 - x0, y1 - y0)
    copy = src.copy()
    copy.thumbnail(target, Image.Resampling.NEAREST)
    px = x0 + (target[0] - copy.width) // 2
    py = y0 + (target[1] - copy.height) // 2
    dst.alpha_composite(copy, (px, py))


def make_actor_contact(approved: dict[str, Image.Image], current: dict[str, Image.Image],
                       sculpted: dict[str, Image.Image]) -> Image.Image:
    w, h = 1500, 2050
    img = Image.new("RGBA", (w, h), BG + (255,))
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, w, 154), fill=(13, 15, 20, 255))
    d.line((54, 153, w - 54, 153), fill=(83, 72, 48, 255), width=2)
    draw_text(d, (58, 32), "SCULPTED SPRITE BAKE / 2.5D", 38, INK, True)
    draw_text(d, (60, 88), "Approved 2D identity retained; solid silhouette depth, warm key, cool occlusion and grounded contact shadow added.", 17, MUTED)
    draw_text(d, (w - 60, 45), "REVIEW ONLY", 18, GOLD, True, anchor="ra")

    cols = [(80, 395, "APPROVED 2D"), (520, 835, "CURRENT 3D BAKE"), (960, 1275, "NEW 2.5D CANDIDATE")]
    for x0, x1, label in cols:
        d.rounded_rectangle((x0, 184, x1, 236), 7, fill=(31, 33, 39, 255), outline=(74, 70, 58, 255), width=2)
        draw_text(d, ((x0 + x1) // 2, 210), label, 16, GOLD, True, anchor="mm")

    row_h = 126
    top = 256
    for i, name in enumerate(REVIEW_ACTORS):
        y0 = top + i * row_h
        y1 = y0 + row_h - 8
        d.rounded_rectangle((54, y0, w - 54, y1), 8, fill=(24, 26, 32, 255), outline=(47, 48, 51, 255), width=1)
        d.text((72, y0 + 37), name.replace("_", " "), font=font(15, mono=True), fill=MUTED)
        approved_img = trim(approved[name], 2)
        current_img = trim(current[name], 2)
        new_img = trim(sculpted[name], 2)
        for src, column in zip((approved_img, current_img, new_img), cols):
            x0, x1 = column[0], column[1]
            paste_contain(img, src, (x0, y0 + 4, x1, y1 - 4))

    y0 = top + len(REVIEW_ACTORS) * row_h + 10
    d.rounded_rectangle((54, y0, w - 54, h - 54), 10, fill=(18, 21, 27, 255), outline=(69, 65, 52, 255), width=2)
    draw_text(d, (78, y0 + 24), "WORLD-SCALE READ", 17, GOLD, True)
    draw_text(d, (78, y0 + 56), "Depth is strongest on silhouettes with cloth, weapons and quadruped mass. Faces intentionally inherit the approved 2D read.", 15, MUTED)
    cast = ["Commoner", "Oracle", "Tidemother", "Wolf", "PaleStag", "Tollmaster"]
    for i, name in enumerate(cast):
        x = 95 + i * 218
        y = y0 + 90
        # small iso ground diamond
        d.polygon([(x, y + 48), (x + 58, y + 77), (x, y + 106), (x - 58, y + 77)], fill=(52, 54, 50, 255), outline=(86, 82, 68, 255))
        paste_contain(img, trim(sculpted[name], 1), (x - 76, y - 35, x + 76, y + 90))
        draw_text(d, (x, y + 120), name, 12, MUTED, mono=True, anchor="ma")
    draw_text(d, (58, h - 34), "Separate review asset — production atlas, manifest and renderer are untouched.", 14, (170, 139, 67), mono=True)
    return add_grain(img, 0xA671, 8)


# ---------------------------------------------------------------------------
# Isometric environment prototypes. All drawing happens at 2x and is reduced
# with Lanczos so edges retain the slightly soft, pre-rendered D2 read.

SS = 2


def sp(v: float) -> int:
    return int(round(v * SS))


def spts(points: list[tuple[float, float]]) -> list[tuple[int, int]]:
    return [(sp(x), sp(y)) for x, y in points]


def sbox(box: tuple[float, float, float, float]) -> tuple[int, int, int, int]:
    return tuple(sp(v) for v in box)  # type: ignore[return-value]


def draw_line(d: ImageDraw.ImageDraw, points: list[tuple[float, float]], fill: tuple[int, int, int, int], width: float = 1) -> None:
    d.line(spts(points), fill=fill, width=max(1, sp(width)))


def draw_poly(d: ImageDraw.ImageDraw, points: list[tuple[float, float]], fill: tuple[int, int, int, int] | None = None,
              outline: tuple[int, int, int, int] | None = None, width: float = 1) -> None:
    d.polygon(spts(points), fill=fill)
    if outline:
        d.line(spts(points + [points[0]]), fill=outline, width=max(1, sp(width)), joint="curve")


def draw_ellipse(d: ImageDraw.ImageDraw, box: tuple[float, float, float, float], fill: tuple[int, int, int, int] | None = None,
                 outline: tuple[int, int, int, int] | None = None, width: float = 1) -> None:
    d.ellipse(sbox(box), fill=fill, outline=outline, width=max(1, sp(width)))


def draw_text2(d: ImageDraw.ImageDraw, xy: tuple[float, float], text: str, size: int,
               fill: tuple[int, int, int, int] = INK + (255,), display: bool = False,
               mono: bool = False, anchor: str | None = None) -> None:
    d.text((sp(xy[0]), sp(xy[1])), text, font=font(sp(size), display, mono), fill=fill, anchor=anchor)


def irregular_diamond(cx: float, cy: float, tw: float, th: float, rng: random.Random, amount: float = 0.035) -> list[tuple[float, float]]:
    base = [(cx, cy - th / 2), (cx + tw / 2, cy), (cx, cy + th / 2), (cx - tw / 2, cy)]
    out = []
    for x, y in base:
        out.append((x + rng.uniform(-tw * amount, tw * amount), y + rng.uniform(-th * amount, th * amount)))
    return out


def floor_material(kind: str, cx: float, cy: float, tw: float, th: float, seed: int) -> tuple[Image.Image, Image.Image]:
    image = Image.new("RGBA", (sp(tw + 28), sp(th + 34)), (0, 0, 0, 0))
    d = ImageDraw.Draw(image)
    rng = random.Random(seed)
    palette = {
        "flagstone": (91, 88, 78),
        "road": (117, 91, 58),
        "grass": (53, 76, 48),
        "water": (33, 68, 78),
    }
    base = palette[kind]
    cx_img, cy_img = sp((tw + 28) / 2), sp((th + 34) / 2 - 3)

    # This material is painted into the supersampled buffer directly. Its
    # coordinates are already scaled; the generic environment helpers below
    # intentionally scale their own inputs.
    def poly_px(points: list[tuple[float, float]], fill: tuple[int, int, int, int],
                outline: tuple[int, int, int, int] | None = None, width: float = 1) -> None:
        d.polygon(points, fill=fill)
        if outline:
            d.line(points + [points[0]], fill=outline, width=max(1, sp(width)), joint="curve")

    def line_px(points: list[tuple[float, float]], fill: tuple[int, int, int, int], width: float = 1) -> None:
        d.line(points, fill=fill, width=max(1, sp(width)), joint="curve")

    def ellipse_px(box: tuple[float, float, float, float], fill: tuple[int, int, int, int]) -> None:
        d.ellipse(box, fill=fill)

    outer = irregular_diamond(cx_img, cy_img + sp(4), sp(tw), sp(th), rng, 0.026)
    poly_px([(x, y + sp(5)) for x, y in outer], (7, 9, 11, 82))
    poly_px(outer, shade(base, 0.64) + (255,), (18, 19, 19, 78), 0.6)
    inner = irregular_diamond(cx_img, cy_img, sp(tw * 0.985), sp(th * 0.975), rng, 0.018)
    poly_px(inner, base + (255,))

    # Broad, low-contrast mineral variation instead of a hard repeated motif.
    for _ in range(42 if kind != "water" else 18):
        px = rng.uniform(cx_img - sp(tw) * 0.42, cx_img + sp(tw) * 0.42)
        py = rng.uniform(cy_img - sp(th) * 0.40, cy_img + sp(th) * 0.40)
        rr = rng.uniform(2, 11)
        tone = rng.choice((-8, -5, 4, 7, 10))
        ellipse_px((px - rr, py - rr * 0.35, px + rr, py + rr * 0.35), shade(base, 1.0) + (tone + 40,))
    line_px([inner[3], inner[0], inner[1]], mix(base, (214, 198, 157), 0.28) + (82,), 0.8)
    line_px([inner[1], inner[2], inner[3]], shade(base, 0.58) + (118,), 0.9)

    if kind == "flagstone":
        for idx in (0, 1, 2, 3):
            if rng.random() < 0.58:
                x, y = inner[idx]
                ellipse_px((x - 8, y - 4, x + 8, y + 4), shade(base, 0.67) + (120,))
        start = inner[rng.randrange(4)]
        end = inner[(inner.index(start) + 2) % 4]
        pts = [start, ((start[0] + end[0]) / 2 + rng.uniform(-16, 16), (start[1] + end[1]) / 2 + rng.uniform(-6, 6)), end]
        line_px(pts, (24, 25, 25, 165), 1.2)
        if rng.random() < 0.7:
            mid = pts[1]
            line_px([mid, (mid[0] + rng.uniform(-18, 18), mid[1] + rng.uniform(5, 15))], (25, 27, 27, 120), 0.8)
        for _ in range(12):
            x = rng.uniform(cx_img - sp(tw) * 0.35, cx_img + sp(tw) * 0.35)
            y = rng.uniform(cy_img - sp(th) * 0.32, cy_img + sp(th) * 0.32)
            r = rng.uniform(1.2, 3.0)
            ellipse_px((x - r, y - r, x + r, y + r), mix(base, (150, 137, 102), 0.5) + (80,))
        for _ in range(7):
            x = rng.choice([cx_img - sp(tw) * 0.38, cx_img + sp(tw) * 0.38]) + rng.uniform(-14, 14)
            y = rng.uniform(cy_img - sp(th) * 0.28, cy_img + sp(th) * 0.28)
            ellipse_px((x - 5, y - 3, x + 5, y + 3), (53, 75, 43, 95))
    elif kind == "road":
        for sign in (-1, 1):
            rut = []
            for i in range(8):
                t = i / 7
                x = cx_img + (t - 0.5) * sp(tw * 0.68)
                y = cy_img + sign * sp(th * 0.12) + math.sin(t * math.pi) * sp(th * 0.05)
                rut.append((x, y))
            line_px(rut, shade(base, 0.66) + (80,), 4)
        for _ in range(68):
            x = rng.uniform(cx_img - sp(tw) * 0.43, cx_img + sp(tw) * 0.43)
            y = rng.uniform(cy_img - sp(th) * 0.39, cy_img + sp(th) * 0.39)
            r = rng.uniform(0.8, 2.8)
            ellipse_px((x - r, y - r * 0.6, x + r, y + r * 0.6), mix(base, (190, 158, 103), rng.random()) + (115,))
    elif kind == "grass":
        for _ in range(32):
            x = rng.uniform(cx_img - sp(tw) * 0.42, cx_img + sp(tw) * 0.42)
            y = rng.uniform(cy_img - sp(th) * 0.36, cy_img + sp(th) * 0.36)
            for dx in (-2, 0, 2):
                line_px([(x + dx * SS, y + 4 * SS), (x + dx, y - rng.uniform(3, 8) * SS)], (77, 99, 57, 135), 0.8)
    elif kind == "water":
        for band in range(5):
            y = cy_img - sp(th * 0.28) + band * sp(th * 0.14)
            pts = []
            for i in range(10):
                t = i / 9
                pts.append((cx_img - sp(tw * 0.32) + t * sp(tw * 0.64), y + math.sin(t * math.pi * 2 + band) * sp(2.2)))
            line_px(pts, mix(base, (113, 183, 184), 0.38) + (85,), 0.8)
        for _ in range(15):
            x = rng.uniform(cx_img - sp(tw * 0.36), cx_img + sp(tw * 0.36))
            y = rng.uniform(cy_img - sp(th * 0.30), cy_img + sp(th * 0.30))
            line_px([(x, y), (x + rng.uniform(3, 8), y - 1)], (159, 210, 204, 95), 0.7)
    return image, trim(image)


def paste_floor_tile(canvas: Image.Image, kind: str, cx: float, cy: float, tw: float, th: float, seed: int) -> None:
    tile, cropped = floor_material(kind, cx, cy, tw, th, seed)
    x = sp(cx) - cropped.width // 2
    y = sp(cy) - cropped.height // 2
    canvas.alpha_composite(cropped, (x, y))


def draw_floor_patch(d: ImageDraw.ImageDraw, origin: tuple[float, float], tw: float, th: float,
                     rect: tuple[float, float, float, float], kind: str, seed: int) -> None:
    gx, gy, width, depth = rect
    rng = random.Random(seed)
    corners = [
        world_xy(gx, gy, origin, tw, th),
        world_xy(gx + width, gy, origin, tw, th),
        world_xy(gx + width, gy + depth, origin, tw, th),
        world_xy(gx, gy + depth, origin, tw, th),
    ]
    points = [
        (x + rng.uniform(-5, 5), y + rng.uniform(-2.5, 2.5))
        for x, y in corners
    ]
    base = (88, 84, 75) if kind == "flagstone" else (113, 88, 57)
    draw_poly(d, points, base + (148,), (27, 28, 27, 78), 0.8)
    draw_line(d, [points[3], points[0], points[1]], mix(base, (213, 198, 159), 0.24) + (75,), 0.8)
    if kind == "flagstone":
        mid = ((points[0][0] + points[2][0]) / 2, (points[0][1] + points[2][1]) / 2)
        draw_line(d, [points[0], (mid[0] - 8, mid[1] + 2), points[2]], (27, 27, 25, 105), 0.9)
    for _ in range(30 if kind == "flagstone" else 42):
        u = rng.uniform(0.08, 0.92)
        v = rng.uniform(0.08, 0.92)
        x, y = world_xy(gx + width * u, gy + depth * v, origin, tw, th)
        r = rng.uniform(1.0, 4.8 if kind == "road" else 3.0)
        if kind == "flagstone":
            color = mix(base, (145, 136, 108), rng.random()) + (72,)
        else:
            color = mix(base, (190, 155, 98), rng.random()) + (82,)
        draw_ellipse(d, (x - r, y - r * 0.45, x + r, y + r * 0.45), color)


def make_floor_sheet() -> Image.Image:
    w, h = 1500, 980
    img = Image.new("RGBA", (sp(w), sp(h)), BG + (255,))
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, sp(w), sp(132)), fill=(13, 15, 20, 255))
    draw_text2(d, (56, 28), "FLOOR MATERIAL STUDY", 38, INK + (255,), True)
    draw_text2(d, (58, 84), "Irregular edges, low-contrast joins, material-scale detail. The tile grid supports the room; it does not draw itself.", 17, MUTED + (255,))
    panels = [(48, 154, 738, 526), (762, 154, 1452, 526), (48, 546, 738, 924), (762, 546, 1452, 924)]
    specs = [
        ("FLAGSTONE", "Cool bone-grey slabs; chipped corners, hairline cracks, restrained moss.", "flagstone"),
        ("PACKED ROAD", "Warm compacted earth; wheel wear and mineral grit instead of stripe noise.", "road"),
        ("MOSS OVERGROWTH", "Soft broken color and tufts; used sparingly against architecture.", "grass"),
        ("BLACK WATER", "Near-black cyan troughs with sparse reflected sky glints.", "water"),
    ]
    for panel, (title, desc, kind) in zip(panels, specs):
        x0, y0, x1, y1 = panel
        d.rounded_rectangle(sbox(panel), radius=sp(10), fill=(27, 29, 35, 255), outline=(65, 65, 61, 255), width=sp(1))
        paste_floor_tile(img, kind, (x0 + x1) / 2, y0 + 225, 500, 250, 0xF100 + specs.index((title, desc, kind)) * 97)
        draw_text2(d, (x0 + 24, y0 + 320), title, 18, GOLD + (255,), True)
        draw_text2(d, (x0 + 24, y0 + 348), desc, 14, MUTED + (255,))
    return img.resize((w, h), Image.Resampling.LANCZOS)


def draw_wall_block(d: ImageDraw.ImageDraw, cx: float, cy: float, tw: float, th: float,
                    height: float, seed: int, broken: bool = False) -> None:
    rng = random.Random(seed)
    top = (cx, cy - height)
    top_p = (cx, cy - height - th / 2)
    top_r = (cx + tw / 2, cy - height)
    top_b = (cx, cy - height + th / 2)
    top_l = (cx - tw / 2, cy - height)
    ground_r = (cx + tw / 2, cy)
    ground_b = (cx, cy + th / 2)
    ground_l = (cx - tw / 2, cy)

    # Ground occlusion and the two visible masonry faces.
    draw_poly(d, [ground_l, ground_b, (ground_b[0] + 6, ground_b[1] + 7), (ground_l[0] - 5, ground_l[1] + 5)], (5, 7, 9, 105))
    left_base = (83, 81, 73) if not broken else (69, 69, 65)
    right_base = (56, 58, 56) if not broken else (49, 51, 50)
    draw_poly(d, [top_l, top_b, ground_b, ground_l], left_base + (255,), shade(left_base, 0.60) + (255,), 1.1)
    draw_poly(d, [top_b, top_r, ground_r, ground_b], right_base + (255,), shade(right_base, 0.58) + (255,), 1.1)

    courses = max(5, int(height / 32))
    for row in range(1, courses):
        t = row / courses
        lx = top_l[0] + (ground_l[0] - top_l[0]) * t
        ly = top_l[1] + (ground_l[1] - top_l[1]) * t
        bx = top_b[0] + (ground_b[0] - top_b[0]) * t
        by = top_b[1] + (ground_b[1] - top_b[1]) * t
        rx = top_r[0] + (ground_r[0] - top_r[0]) * t
        ry = top_r[1] + (ground_r[1] - top_r[1]) * t
        wobble = rng.uniform(-1.5, 1.5)
        draw_line(d, [(lx, ly + wobble), (bx, by - wobble)], shade(left_base, 0.67) + (210,), 1.2)
        draw_line(d, [(bx, by - wobble), (rx, ry + wobble)], shade(right_base, 0.65) + (220,), 1.2)
        # Staggered vertical joints.
        for face, a, b, base_col in (("L", (lx, ly), (bx, by), left_base), ("R", (bx, by), (rx, ry), right_base)):
            for s in (0.34, 0.68):
                if (row + (1 if face == "R" else 0)) % 3 == 0:
                    continue
                jx = a[0] + (b[0] - a[0]) * s + rng.uniform(-2, 2)
                jy_top = a[1] + (b[1] - a[1]) * s
                jy_bottom = jy_top + (height / courses) * rng.uniform(0.68, 0.90)
                draw_line(d, [(jx, jy_top), (jx + rng.uniform(-1, 1), jy_bottom)], shade(base_col, 0.72) + (185,), 0.9)

    # Top cap: large-scale bevel, not a bright tile grid.
    cap = (105, 101, 89) if not broken else (79, 79, 73)
    draw_poly(d, [top_p, top_r, top_b, top_l], cap + (255,), shade(cap, 0.56) + (255,), 1.2)
    draw_line(d, [top_l, top_p, top_r], mix(cap, (223, 205, 161), 0.35) + (165,), 1.3)
    draw_line(d, [top_r, top_b, top_l], shade(cap, 0.65) + (205,), 1.2)

    # Chips, mineral blooms and sparse moss.
    for _ in range(13):
        t = rng.random()
        side = rng.random()
        if side < 0.5:
            x = top_l[0] + (top_b[0] - top_l[0]) * t
            y = top_l[1] + (top_b[1] - top_l[1]) * t + rng.uniform(0, height * 0.35)
            r = rng.uniform(1.2, 3.2)
            draw_ellipse(d, (x - r, y - r * 0.5, x + r, y + r * 0.5), shade(left_base, rng.uniform(0.55, 0.82)) + (95,))
        else:
            x = top_b[0] + (top_r[0] - top_b[0]) * t
            y = top_b[1] + (top_r[1] - top_b[1]) * t + rng.uniform(0, height * 0.35)
            r = rng.uniform(1.0, 2.8)
            draw_ellipse(d, (x - r, y - r * 0.5, x + r, y + r * 0.5), shade(right_base, rng.uniform(0.58, 0.85)) + (95,))
    for _ in range(4 if not broken else 7):
        t = rng.uniform(0.18, 0.95)
        x = top_b[0] + (top_r[0] - top_b[0]) * t
        y = top_b[1] + (ground_b[1] - top_b[1]) * t
        draw_ellipse(d, (x - 4, y - 2, x + 4, y + 2), (48, 67, 40, 75))

    if broken:
        # Remove a readable bite from the upper silhouette with dark chips.
        bite = (top_l[0] + tw * 0.12, top_l[1] - 2, top_l[0] + tw * 0.31, top_l[1] + 18)
        draw_poly(d, [bite[:2], (bite[2], bite[1]), (bite[2] - 3, bite[3]), (bite[0] + 2, bite[3] - 2)], (26, 29, 29, 220))


def draw_wall_curtain_segment(d: ImageDraw.ImageDraw, start: tuple[float, float], count: int,
                              tw: float, th: float, height: float,
                              direction: tuple[float, float], seed: int) -> list[tuple[float, float]]:
    if count <= 0:
        return []
    rng = random.Random(seed)
    ux, uy = direction[0] * tw / 2, direction[1] * th / 2
    vx, vy = -direction[0] * tw / 2, direction[1] * th / 2
    half_run = (count - 1) / 2
    cx = start[0] + ux * half_run
    cy = start[1] + uy * half_run
    tc = (cx, cy - height)
    thick = 0.62
    a = (tc[0] - ux * half_run - vx * thick, tc[1] - uy * half_run - vy * thick)
    b = (tc[0] + ux * half_run - vx * thick, tc[1] + uy * half_run - vy * thick)
    c = (tc[0] + ux * half_run + vx * thick, tc[1] + uy * half_run + vy * thick)
    e = (tc[0] - ux * half_run + vx * thick, tc[1] - uy * half_run + vy * thick)
    ag = (a[0], a[1] + height)
    bg = (b[0], b[1] + height)
    cg = (c[0], c[1] + height)
    eg = (e[0], e[1] + height)

    front = (82, 80, 72)
    end = (57, 59, 56)
    cap = (119, 113, 96)
    # A continuous long face and one narrow return face: the silhouette is a
    # wall, not a row of full-depth cubes.
    draw_poly(d, [e, c, (cg[0] + 5, cg[1] + 6), (eg[0] - 4, eg[1] + 5)], (5, 7, 9, 120))
    draw_poly(d, [e, c, cg, eg], front + (255,), shade(front, 0.63) + (255,), 1.0)
    draw_poly(d, [c, b, bg, cg], end + (255,), shade(end, 0.62) + (255,), 1.0)

    # Shallow buttress bands sit on the long face instead of protruding as
    # separate blocks. This gives vertical rhythm without another cube read.
    for s in (0.08, 0.50, 0.92):
        if count == 1 and s == 0.50:
            continue
        half = 0.022
        p0 = (e[0] + (c[0] - e[0]) * (s - half), e[1] + (c[1] - e[1]) * (s - half))
        p1 = (e[0] + (c[0] - e[0]) * (s + half), e[1] + (c[1] - e[1]) * (s + half))
        p2 = (eg[0] + (cg[0] - eg[0]) * (s + half), eg[1] + (cg[1] - eg[1]) * (s + half))
        p3 = (eg[0] + (cg[0] - eg[0]) * (s - half), eg[1] + (cg[1] - eg[1]) * (s - half))
        draw_poly(d, [p0, p1, p2, p3], mix(front, (150, 143, 120), 0.18) + (105,))
        draw_line(d, [p0, p3], (194, 180, 143, 72), 0.8)

    courses = max(5, int(height / 30))
    for row in range(1, courses):
        t = row / courses
        p0 = (e[0] + (eg[0] - e[0]) * t, e[1] + (eg[1] - e[1]) * t)
        p1 = (c[0] + (cg[0] - c[0]) * t, c[1] + (cg[1] - c[1]) * t)
        wobble = rng.uniform(-1.0, 1.0)
        draw_line(d, [(p0[0], p0[1] + wobble), (p1[0], p1[1] - wobble)], shade(front, 0.69) + (205,), 1.0)
        q0 = (c[0] + (cg[0] - c[0]) * t, c[1] + (cg[1] - c[1]) * t)
        q1 = (b[0] + (bg[0] - b[0]) * t, b[1] + (bg[1] - b[1]) * t)
        draw_line(d, [q0, q1], shade(end, 0.66) + (210,), 1.0)

    for i in range(1, count):
        s = i / count
        top = (e[0] + (c[0] - e[0]) * s, e[1] + (c[1] - e[1]) * s)
        bottom = (eg[0] + (cg[0] - eg[0]) * s, eg[1] + (cg[1] - eg[1]) * s)
        draw_line(d, [top, bottom], shade(front, 0.66) + (210,), 1.0)

    draw_poly(d, [a, b, c, e], cap + (255,), shade(cap, 0.60) + (255,), 1.0)
    draw_line(d, [a, b, c], mix(cap, (228, 208, 163), 0.38) + (165,), 1.1)
    draw_line(d, [c, e, a], shade(cap, 0.67) + (205,), 1.0)

    anchors = []
    for i in range(count):
        s = (i + 0.5) / count
        p = (e[0] + (c[0] - e[0]) * s, e[1] + (c[1] - e[1]) * s)
        merlon_h = 24 + (i % 2) * 5
        draw_line(d, [(p[0] - tw * 0.13, p[1] + 2), (p[0] + tw * 0.13, p[1] + 2)], shade(cap, 0.58) + (255,), 2.2)
        draw_wall_block(d, p[0], p[1] + 4, tw * 0.20, th * 0.20, merlon_h, seed + 300 + i * 31, False)
        anchors.append((p[0], p[1] - merlon_h))

    for _ in range(18):
        t = rng.random()
        depth = rng.uniform(0.08, 0.94)
        x = e[0] + (c[0] - e[0]) * t
        y = e[1] + (eg[1] - e[1]) * depth
        r = rng.uniform(1.0, 2.8)
        draw_ellipse(d, (x - r, y - r * 0.5, x + r, y + r * 0.5), shade(front, rng.uniform(0.58, 0.86)) + (105,))
    return anchors


def draw_wall_run(d: ImageDraw.ImageDraw, start: tuple[float, float], count: int, tw: float, th: float,
                  height: float, direction: tuple[float, float], seed: int,
                  broken_index: set[int] | None = None) -> list[tuple[float, float]]:
    broken_index = broken_index or set()
    ux, uy = direction[0] * tw / 2, direction[1] * th / 2
    anchors: list[tuple[float, float]] = []
    segment: list[int] = []

    def flush() -> None:
        if not segment:
            return
        seg_start = (start[0] + ux * segment[0], start[1] + uy * segment[0])
        anchors.extend(draw_wall_curtain_segment(
            d, seg_start, len(segment), tw, th, height - (segment[0] % 3) * 4,
            direction, seed + segment[0] * 211,
        ))
        segment.clear()

    for i in range(count):
        if i in broken_index:
            flush()
        else:
            segment.append(i)
    flush()
    return anchors


def make_wall_sheet() -> Image.Image:
    w, h = 1500, 860
    img = Image.new("RGBA", (sp(w), sp(h)), BG + (255,))
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, sp(w), sp(132)), fill=(13, 15, 20, 255))
    draw_text2(d, (56, 28), "WALL MODULE STUDY", 38, INK + (255,), True)
    draw_text2(d, (58, 84), "Tall modular curtains, deep side values, readable courses and capped merlons. No full-grid cube read.", 17, MUTED + (255,))
    panels = [(48, 154, 738, 812), (762, 154, 1452, 812)]
    for panel in panels:
        d.rounded_rectangle(sbox(panel), radius=sp(10), fill=(25, 27, 32, 255), outline=(65, 65, 61, 255), width=sp(1))
    # Left: receding straight run.
    draw_wall_run(d, (210, 620), 5, 180, 90, 225, (1, 0.50), 0xA11, {3})
    draw_text2(d, (76, 188), "CURTAIN RUN", 20, GOLD + (255,), True)
    draw_text2(d, (76, 738), "Long faces, broken modules, narrow merlons. Side values carry depth without outlining every brick.", 14, MUTED + (255,))
    # Right: gate shoulder / buttress composition.
    draw_wall_run(d, (835, 650), 4, 185, 92, 245, (1, 0.50), 0xB22, {2})
    draw_wall_run(d, (1235, 720), 2, 155, 78, 190, (1, 0.50), 0xB33, set())
    draw_wall_block(d, 785, 605, 160, 80, 170, 0xB44, True)
    draw_text2(d, (790, 188), "GATE SHOULDER", 20, GOLD + (255,), True)
    draw_text2(d, (790, 738), "Large masses, fewer seams, strong vertical buttresses. Torchlight can own the local contrast at runtime.", 14, MUTED + (255,))
    return img.resize((w, h), Image.Resampling.LANCZOS)


def world_xy(gx: float, gy: float, origin: tuple[float, float], tw: float, th: float) -> tuple[float, float]:
    return origin[0] + (gx - gy) * tw / 2, origin[1] + (gx + gy) * th / 2


def paste_actor_anchor(canvas: Image.Image, sprite: Image.Image, x: float, y: float, scale: float = 1.0) -> None:
    actor = trim(sprite, 1)
    if scale != 1.0:
        actor = actor.resize((max(1, round(actor.width * scale)), max(1, round(actor.height * scale))), Image.Resampling.NEAREST)
    ax = sp(x) - actor.width // 2
    ay = sp(y) - actor.height + sp(5)
    canvas.alpha_composite(actor, (ax, ay))


def radial_glow(size: tuple[int, int], center: tuple[float, float], radius: float, color: tuple[int, int, int], strength: float) -> Image.Image:
    mask = Image.new("L", size, 0)
    d = ImageDraw.Draw(mask)
    x, y = center
    d.ellipse((x - radius, y - radius, x + radius, y + radius), fill=clamp(strength * 255))
    mask = mask.filter(ImageFilter.GaussianBlur(radius * 0.42))
    glow = Image.new("RGB", size, color)
    return Image.composite(glow, Image.new("RGB", size, (0, 0, 0)), mask)


def load_map_fixture() -> dict:
    with MAP_FIXTURE.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def fixture_tile(fixture: dict, x: int, y: int) -> str:
    map_data = fixture["map"]
    if x < 0 or y < 0 or x >= map_data["width"] or y >= map_data["height"]:
        return "Wall"
    return map_data["tiles"][y * map_data["width"] + x]


def validate_fixture_scene(fixture: dict, left: int, top: int, right: int, bottom: int) -> tuple[int, int]:
    player = fixture["player"]["pos"]
    px, py = player["x"], player["y"]
    assert fixture_tile(fixture, px, py) == "Floor", "player must stand on a real walkable floor cell"
    assert fixture_tile(fixture, px + 2, py) == "Wall", "east building wall missing from fixture"
    assert fixture_tile(fixture, px, py + 2) == "Wall", "south building wall missing from fixture"

    visible_npcs = []
    for npc in fixture["npcs"]:
        x, y = npc["pos"]["x"], npc["pos"]["y"]
        if left <= x <= right and top <= y <= bottom:
            assert fixture_tile(fixture, x, y) != "Wall", f"NPC {npc['name']} overlaps a wall"
            visible_npcs.append(npc)
    wall_count = sum(
        fixture_tile(fixture, x, y) == "Wall"
        for y in range(top, bottom + 1)
        for x in range(left, right + 1)
    )
    return wall_count, len(visible_npcs)


def draw_map_wall_cell(d: ImageDraw.ImageDraw, cx: float, cy: float, tw: float, th: float,
                       height: float, draw_left: bool, draw_right: bool, seed: int) -> None:
    rng = random.Random(seed)
    top_p = (cx, cy - height - th / 2)
    top_r = (cx + tw / 2, cy - height)
    top_b = (cx, cy - height + th / 2)
    top_l = (cx - tw / 2, cy - height)
    ground_r = (cx + tw / 2, cy)
    ground_b = (cx, cy + th / 2)
    ground_l = (cx - tw / 2, cy)

    # Faces are unstroked and share exact vertices. Only exposed faces exist,
    # so adjacent wall cells read as one mesh instead of stacked rectangles.
    if draw_left:
        left = (88, 85, 76)
        draw_poly(d, [top_l, top_b, ground_b, ground_l], left + (255,))
        courses = max(4, int(height / 24))
        for row in range(1, courses):
            t = row / courses
            a = (top_l[0] + (ground_l[0] - top_l[0]) * t, top_l[1] + (ground_l[1] - top_l[1]) * t)
            b = (top_b[0] + (ground_b[0] - top_b[0]) * t, top_b[1] + (ground_b[1] - top_b[1]) * t)
            draw_line(d, [a, b], shade(left, 0.70) + (155,), 0.65)
        draw_line(d, [ground_l, ground_b], (20, 22, 22, 95), 1.2)
        draw_line(d, [top_l, top_b], mix(left, (220, 202, 158), 0.30) + (105,), 0.7)
    if draw_right:
        right = (58, 60, 57)
        draw_poly(d, [top_b, top_r, ground_r, ground_b], right + (255,))
        courses = max(4, int(height / 24))
        for row in range(1, courses):
            t = row / courses
            a = (top_b[0] + (ground_b[0] - top_b[0]) * t, top_b[1] + (ground_b[1] - top_b[1]) * t)
            b = (top_r[0] + (ground_r[0] - top_r[0]) * t, top_r[1] + (ground_r[1] - top_r[1]) * t)
            draw_line(d, [a, b], shade(right, 0.69) + (165,), 0.65)
        draw_line(d, [ground_b, ground_r], (18, 20, 21, 110), 1.2)
        draw_line(d, [top_b, top_r], mix(right, (190, 178, 145), 0.25) + (80,), 0.7)

    # Expanded, unstroked roof diamonds overlap by sub-pixel amounts. The roof
    # union hides the cell mesh while exposed boundary edges still catch light.
    cap = (112, 106, 90)
    pad_x, pad_y = 0.9, 0.45
    cap_points = [
        (cx, cy - height - th / 2 - pad_y),
        (cx + tw / 2 + pad_x, cy - height),
        (cx, cy - height + th / 2 + pad_y),
        (cx - tw / 2 - pad_x, cy - height),
    ]
    draw_poly(d, cap_points, cap + (255,))
    for _ in range(7):
        x = rng.uniform(cx - tw * 0.38, cx + tw * 0.38)
        y = rng.uniform(cy - height + 3, cy - 2)
        r = rng.uniform(0.6, 1.8)
        draw_ellipse(d, (x - r, y - r * 0.45, x + r, y + r * 0.45), (48, 65, 40, rng.randrange(22, 58)))


def draw_map_prop(d: ImageDraw.ImageDraw, tile: str, x: float, y: float, seed: int) -> None:
    if tile == "Door":
        draw_line(d, [(x - 9, y + 7), (x - 9, y - 17), (x + 9, y - 17), (x + 9, y + 7)], (63, 48, 32, 255), 3)
        draw_line(d, [(x - 7, y + 5), (x - 7, y - 14), (x + 7, y - 14), (x + 7, y + 5)], (112, 87, 49, 220), 1.2)
    elif tile == "Shrine":
        draw_poly(d, [(x, y - 22), (x + 10, y - 5), (x, y + 5), (x - 10, y - 5)], (118, 91, 146, 255), (202, 171, 213, 180), 1)
        draw_line(d, [(x, y - 14), (x, y + 10)], (230, 213, 235, 220), 1.2)
    elif tile == "Chest":
        draw_poly(d, [(x - 10, y), (x - 7, y - 8), (x + 8, y - 8), (x + 11, y)], (104, 68, 36, 255), (198, 147, 65, 200), 1)
        draw_line(d, [(x, y - 7), (x, y + 7)], (216, 173, 83, 220), 1.2)
    elif tile in ("Up", "Down"):
        for i in range(3):
            offset = i * 3 if tile == "Down" else (2 - i) * 3
            draw_line(d, [(x - 9, y + offset), (x + 9, y + offset - 3)], (173, 163, 133, 220), 1.2)


def draw_fixture_minimap(d: ImageDraw.ImageDraw, fixture: dict, left: int, top: int,
                         right: int, bottom: int, wall_count: int, actor_count: int) -> None:
    width, height = right - left + 1, bottom - top + 1
    x0, y0 = 1310, 54
    cell = 8
    d.rounded_rectangle(sbox((x0, y0, x0 + 232, y0 + 286)), radius=sp(7), fill=(11, 14, 18, 232), outline=(102, 82, 46, 225), width=sp(1))
    draw_text2(d, (x0 + 14, y0 + 13), "REAL MAP SLICE", 11, GOLD + (255,), mono=True)
    for y in range(height):
        for x in range(width):
            tile = fixture_tile(fixture, left + x, top + y)
            if tile == "Wall":
                color = (177, 167, 139, 255)
            elif tile == "Road":
                color = (113, 79, 43, 255)
            elif tile in ("Floor", "Door"):
                color = (62, 66, 61, 255)
            elif tile == "Grass":
                color = (35, 57, 35, 255)
            else:
                color = (77, 61, 83, 255)
            px = x0 + 14 + x * cell
            py = y0 + 39 + y * cell
            d.rectangle(sbox((px, py, px + cell - 1, py + cell - 1)), fill=color)
    for npc in fixture["npcs"]:
        x, y = npc["pos"]["x"], npc["pos"]["y"]
        if left <= x <= right and top <= y <= bottom:
            px = x0 + 14 + (x - left) * cell + cell / 2
            py = y0 + 39 + (y - top) * cell + cell / 2
            draw_ellipse(d, (px - 2, py - 2, px + 2, py + 2), (89, 170, 205, 255))
    player = fixture["player"]["pos"]
    px = x0 + 14 + (player["x"] - left) * cell + cell / 2
    py = y0 + 39 + (player["y"] - top) * cell + cell / 2
    draw_ellipse(d, (px - 3.5, py - 3.5, px + 3.5, py + 3.5), (246, 220, 153, 255), (255, 245, 208, 255), 1)
    draw_text2(d, (x0 + 14, y0 + 250), f"WALL CELLS {wall_count}", 9, MUTED + (255,), mono=True)
    draw_text2(d, (x0 + 14, y0 + 266), f"ACTORS {actor_count + 1} / PLAYER", 9, MUTED + (255,), mono=True)


def ground_class(tile: str) -> str:
    if tile == "Road":
        return "road"
    if tile == "Grass":
        return "grass"
    return "stone"


def draw_continuous_ground(d: ImageDraw.ImageDraw, fixture: dict, left: int, top: int,
                           right: int, bottom: int, origin: tuple[float, float],
                           tw: float, th: float) -> None:
    crop_w, crop_h = right - left + 1, bottom - top + 1
    plane = [
        world_xy(0, 0, origin, tw, th),
        world_xy(crop_w, 0, origin, tw, th),
        world_xy(crop_w, crop_h, origin, tw, th),
        world_xy(0, crop_h, origin, tw, th),
    ]
    draw_poly(d, [(x + 3, y + 8) for x, y in plane], (4, 6, 8, 170))
    draw_poly(d, plane, (43, 57, 40, 255))

    palette = {
        "grass": (43, 57, 40),
        "stone": (78, 76, 68),
        "road": (111, 82, 49),
    }
    # Same-material cells share fill and overlap by a fraction of a pixel. No
    # per-cell stroke or texture means no support-grid outlines survive.
    for material in ("road", "stone"):
        for ry in range(crop_h):
            for rx in range(crop_w):
                ax, ay = left + rx, top + ry
                if ground_class(fixture_tile(fixture, ax, ay)) != material:
                    continue
                cx, cy = world_xy(rx, ry, origin, tw, th)
                points = [
                    (cx, cy - th * 0.505),
                    (cx + tw * 0.505, cy),
                    (cx, cy + th * 0.505),
                    (cx - tw * 0.505, cy),
                ]
                draw_poly(d, points, palette[material] + (255,))

    rng = random.Random(0x51A7)
    # Large-scale mineral variation crosses cell boundaries continuously.
    for _ in range(950):
        gx = rng.uniform(0.05, crop_w - 0.05)
        gy = rng.uniform(0.05, crop_h - 0.05)
        ax = left + min(crop_w - 1, int(gx))
        ay = top + min(crop_h - 1, int(gy))
        material = ground_class(fixture_tile(fixture, ax, ay))
        x, y = world_xy(gx, gy, origin, tw, th)
        radius = rng.uniform(0.45, 2.2)
        tone = rng.choice((-7, -4, 3, 5, 8))
        draw_ellipse(d, (x - radius, y - radius * 0.42, x + radius, y + radius * 0.42), shade(palette[material], 1.0) + (tone + 34,))

    # Sparse multi-cell cracks and soft patches replace the old per-diamond
    # crack stamp, so detail belongs to the material rather than the grid.
    for _ in range(26):
        gx = rng.uniform(1, crop_w - 2)
        gy = rng.uniform(1, crop_h - 2)
        points = []
        angle = rng.uniform(0, math.tau)
        for _ in range(rng.randint(3, 6)):
            gx += math.cos(angle) * rng.uniform(0.25, 0.8)
            gy += math.sin(angle) * rng.uniform(0.25, 0.8)
            gx = max(0.05, min(crop_w - 0.05, gx))
            gy = max(0.05, min(crop_h - 0.05, gy))
            points.append(world_xy(gx, gy, origin, tw, th))
        draw_line(d, points, (24, 25, 24, rng.randrange(55, 105)), rng.uniform(0.45, 1.0))
    for _ in range(34):
        gx = rng.uniform(0.5, crop_w - 0.5)
        gy = rng.uniform(0.5, crop_h - 0.5)
        x, y = world_xy(gx, gy, origin, tw, th)
        rx, ry = rng.uniform(5, 18), rng.uniform(2, 7)
        material = ground_class(fixture_tile(fixture, left + int(gx), top + int(gy)))
        draw_ellipse(d, (x - rx, y - ry, x + rx, y + ry), shade(palette[material], 1.08) + (18,))


def make_environment_scene(actor_sprites: dict[str, Image.Image], camera: str = "standard") -> Image.Image:
    fixture = load_map_fixture()
    w, h = 1600, 1000
    left, top, right, bottom = 4, 9, 21, 26
    wall_count, visible_npc_count = validate_fixture_scene(fixture, left, top, right, bottom)
    player = fixture["player"]["pos"]
    player_rx, player_ry = player["x"] - left, player["y"] - top

    img = Image.new("RGBA", (sp(w), sp(h)), (10, 12, 16, 255))
    d = ImageDraw.Draw(img)
    tw = 78
    th, wall_height, torch_lift = (45, 60, 42) if camera == "top" else (39, 72, 47)
    origin = (800, 520 - (player_rx + player_ry) * th / 2)
    crop_w, crop_h = right - left + 1, bottom - top + 1

    # One continuous material surface per class. The map still decides which
    # material occupies each region, but no per-cell floor plate is rendered.
    draw_continuous_ground(d, fixture, left, top, right, bottom, origin, tw, th)

    events: list[tuple[float, float, tuple]] = []
    for ry in range(crop_h):
        for rx in range(crop_w):
            ax, ay = left + rx, top + ry
            tile = fixture_tile(fixture, ax, ay)
            x, y = world_xy(rx, ry, origin, tw, th)
            if tile == "Wall":
                events.append((ax + ay, 0.0, ("wall", x, y, ax, ay)))
            elif tile in ("Door", "Shrine", "Chest", "Up", "Down"):
                events.append((ax + ay, 0.5, ("prop", tile, x, y)))

    nearby_npcs = []
    for npc in fixture["npcs"]:
        x, y = npc["pos"]["x"], npc["pos"]["y"]
        if left <= x <= right and top <= y <= bottom and npc["archetype"] in actor_sprites:
            nearby_npcs.append(((x - player["x"]) ** 2 + (y - player["y"]) ** 2, npc))
    for _, npc in sorted(nearby_npcs, key=lambda item: item[0])[:12]:
        ax, ay = npc["pos"]["x"], npc["pos"]["y"]
        rx, ry = ax - left, ay - top
        x, y = world_xy(rx, ry, origin, tw, th)
        events.append((ax + ay, 1.0, ("actor", npc["archetype"], x, y, 0.78, False)))
    player_x, player_y = world_xy(player_rx, player_ry, origin, tw, th)
    if "Commoner" in actor_sprites:
        events.append((player["x"] + player["y"], 1.1, ("actor", "Commoner", player_x, player_y, 0.90, True)))

    for _, _, event in sorted(events, key=lambda item: (item[0], item[1])):
        if event[0] == "wall":
            _, x, y, ax, ay = event
            draw_map_wall_cell(
                d, x, y, tw, th, wall_height,
                fixture_tile(fixture, ax, ay + 1) != "Wall",
                fixture_tile(fixture, ax + 1, ay) != "Wall",
                0xC000 + ax * 131 + ay * 17,
            )
        elif event[0] == "prop":
            _, tile, x, y = event
            draw_map_prop(d, tile, x, y, 0xD000 + int(x) * 13 + int(y) * 7)
        else:
            _, name, x, y, scale, is_player = event
            if is_player:
                draw_ellipse(d, (x - 16, y - 5, x + 16, y + 7), (214, 177, 89, 68))
                draw_ellipse(d, (x - 11, y - 3, x + 11, y + 4), (238, 215, 151, 95))
            paste_actor_anchor(img, actor_sprites[name], x, y, scale)

    # Torches sit on real doorway cells nearest the player, not decorative
    # free-floating coordinates.
    door_cells = []
    for ry in range(crop_h):
        for rx in range(crop_w):
            ax, ay = left + rx, top + ry
            if fixture_tile(fixture, ax, ay) == "Door":
                door_cells.append(((ax - player["x"]) ** 2 + (ay - player["y"]) ** 2, ax, ay))
    torch_anchors = []
    for _, ax, ay in sorted(door_cells)[:3]:
        x, y = world_xy(ax - left, ay - top, origin, tw, th)
        y -= torch_lift
        torch_anchors.append((x, y))
        draw_line(d, [(x, y + 17), (x - 6, y + 8), (x - 10, y + 12)], (31, 29, 27, 255), 2.5)
        draw_ellipse(d, (x - 8, y + 6, x + 7, y + 16), (40, 31, 24, 255), (100, 70, 34, 255), 1)
        draw_poly(d, [(x - 4, y + 9), (x - 2, y - 9), (x + 1, y - 18), (x + 5, y - 5), (x + 3, y + 9)], (238, 119, 32, 235))
        draw_poly(d, [(x - 1, y + 8), (x, y - 4), (x + 2, y - 11), (x + 3, y + 7)], (255, 218, 112, 245))

    rgb = img.convert("RGB")
    light = Image.new("RGB", rgb.size, (0, 0, 0))
    for x, y in torch_anchors:
        glow = radial_glow(rgb.size, (sp(x), sp(y)), sp(118), (255, 116, 28), 0.66)
        light = ImageChops.lighter(light, glow)
    rgb = ImageChops.screen(rgb, light.point(lambda value: clamp(value * 0.62)))

    vignette = Image.new("L", rgb.size, 0)
    vd = ImageDraw.Draw(vignette)
    margin_x, margin_y = sp(150), sp(90)
    vd.ellipse((margin_x, margin_y, rgb.width - margin_x, rgb.height - margin_y), fill=218)
    vignette = vignette.filter(ImageFilter.GaussianBlur(105))
    rgb = Image.composite(rgb, Image.new("RGB", rgb.size, (4, 6, 9)), vignette)
    rgb = ImageEnhance.Color(rgb).enhance(0.93)
    rgb = ImageEnhance.Contrast(rgb).enhance(1.05)

    overlay = Image.new("RGBA", rgb.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    od.rounded_rectangle(sbox((54, 48, 520, 116)), radius=sp(4), fill=(13, 15, 19, 224), outline=(105, 82, 44, 225), width=sp(1))
    draw_text2(od, (76, 65), "MILLBROOK / REAL MAP 1", 18, INK + (255,), True)
    draw_text2(od, (76, 94), f"SEED 42 / PLAYER 12,19 / {camera.upper()} CAMERA", 10, (182, 150, 76, 255), mono=True)
    draw_fixture_minimap(od, fixture, left, top, right, bottom, wall_count, visible_npc_count)
    draw_text2(od, (58, 966), "GOLD: PLAYER   BLUE: NPC   IVORY: WALL   SOURCE: REAL GAME MAP", 10, (166, 157, 136, 220), mono=True)
    rgb = rgb.convert("RGBA")
    rgb.alpha_composite(overlay)
    rgb = add_grain(rgb, 0xBADBEEF, 7)
    print(f"MAP PLAN OK: {wall_count} wall cells, {visible_npc_count} visible NPCs, player enclosed at (12,19)")
    return rgb.resize((w, h), Image.Resampling.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with Image.open(CONTACT_2D) as opened:
        contact = opened.convert("RGBA")
    background = build_grass_background(contact)

    approved_by_name: dict[str, Image.Image] = {}
    current_by_name: dict[str, Image.Image] = {}
    sculpted_by_name: dict[str, Image.Image] = {}
    for name, col, row in ACTOR_CELLS:
        if name in approved_by_name:
            continue
        approved = extract_2d_sprite(contact, background, col, row)
        approved_by_name[name] = approved
        sculpted_by_name[name] = make_sculpted_sprite(approved, seed=ACTOR_CELLS.index((name, col, row)))

    # Explicit state cell for Pale Stag.
    approved_by_name["PaleStag.at_bay"] = extract_2d_sprite(contact, background, 4, 3)
    sculpted_by_name["PaleStag.at_bay"] = make_sculpted_sprite(approved_by_name["PaleStag.at_bay"], 77)

    for name in CURRENT_3D_CELLS:
        current_by_name[name] = extract_current_3d(name)

    sheet = make_actor_sheet(sculpted_by_name)
    sheet.save(OUT / "actors-2d5d-sheet.png")
    make_actor_contact(approved_by_name, current_by_name, sculpted_by_name).convert("RGB").save(OUT / "actors-v2.png", quality=95)
    make_floor_sheet().convert("RGB").save(OUT / "floor-v2.png", quality=95)
    make_wall_sheet().convert("RGB").save(OUT / "wall-v2.png", quality=95)
    make_environment_scene(approved_by_name, "standard").convert("RGB").save(OUT / "environment-v5-seamless-standard.png", quality=95)
    make_environment_scene(approved_by_name, "top").convert("RGB").save(OUT / "environment-v5-seamless-top.png", quality=95)
    print("generated standalone visual-v2 prototypes in", OUT)


if __name__ == "__main__":
    main()
