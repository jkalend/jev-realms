//! Chrome plate baker (D30): UI plate family for in-window chrome —
//! orb frames, panel 9-patch kit, boss nameplate, button rows, sigil pip,
//! XP sliver. Same D38 authenticity as bake3d/bake_actor: plates bake a
//! per-pixel bevel *depth map* first, then one lighting pass (NW key, warm
//! fill, depth-discontinuity AO, facet/spec glints where cut gem faces
//! exist), so the chrome reads torch-lit, not flat-fill.
//!
//! Vocabulary (gothic frame): brass ridge [(177,131,62) family] with a
//! high lip and dark undercut, dark stone inset [(66,66,61)/(44,42,39)
//! family] with mottle, per-hue gems with a cut facet + white spec chip.
//! Palette hues sampled from the tool's painters: GOLD [232,191,89],
//! LEATHER brass range [177,131,62]<>[102,72,37], INK [9,11,12], SEA.
//! Bloom discipline learned from the nameplate-gold bleed: gem/highlights
//! stay under the 1.30 lum clamp and bright chips stay <= 2px, so the
//! post stack's bloom threshold never picks them up.

use crate::raster::*;
use image::RgbaImage;

// ---------------------------------------------------------------------------
// Palette arms (from tools/atlas painter codes, verbatim families).

const BRASS: Rgb = [177, 131, 62];
const BRASS_HI: Rgb = [232, 191, 89];
const BRASS_LO: Rgb = [102, 72, 37];
#[allow(dead_code)]
const STONE: Rgb = [66, 66, 61];
const STONE_DARK: Rgb = [44, 42, 39];
const INK: Rgb = [9, 11, 12];
const SEA: Rgb = [102, 212, 196];
const HP_RED: Rgb = [168, 44, 40];
const MANA_BLUE: Rgb = [64, 92, 178];

// Item-icon arms (the inventory/slot pictograms): forge steel, cold glass,
// tallow flame, parchment, jungle leaf — kept inside the same ±1.30 lum
// clamp so the post stack's bloom threshold never picks an icon up.
const STEEL: Rgb = [150, 154, 160];
const STEEL_HI: Rgb = [212, 216, 222];
const IRON_LO: Rgb = [88, 92, 100];
const WOOD: Rgb = [116, 82, 44];
const FLAME: Rgb = [222, 128, 52];
const FLAME_HI: Rgb = [244, 208, 126];
const GLASS: Rgb = [132, 172, 194];
const LIQUID_RED: Rgb = [176, 48, 46];
const LIQUID_BLUE: Rgb = [64, 96, 186];
const LEAF: Rgb = [102, 152, 90];
const LEAF_HI: Rgb = [140, 186, 118];
const BERRY: Rgb = [186, 84, 96];
const PARCHMENT: Rgb = [198, 184, 152];
const VIOLET: Rgb = [152, 100, 190];
const WAX: Rgb = [172, 58, 56];
const SAND: Rgb = [140, 128, 100];

// ---------------------------------------------------------------------------
// Depth canvas: albedo + relief per pixel at 64x64 (plates are 1:1 cells).

struct Chrome64 {
    h: [f32; 4096],
    alb: [(u8, u8, u8); 4096],
    alpha: [u8; 4096],
    emissive: [f32; 4096],
}

impl Chrome64 {
    fn new(fill_h: f32, fill_alb: Rgb) -> Self {
        Self {
            h: [fill_h; 4096],
            alb: [(fill_alb[0], fill_alb[1], fill_alb[2]); 4096],
            alpha: [255; 4096],
            emissive: [0.0; 4096],
        }
    }
    /// Fully transparent canvas: every painter CARVES the shape it draws, so
    /// frame plates can be a thin brass rail with nothing around it (the
    /// 9-patch then stretches only the rail, never a slab of stone).
    fn new_clear() -> Self {
        Self {
            h: [0.0; 4096],
            alb: [(0, 0, 0); 4096],
            alpha: [0; 4096],
            emissive: [0.0; 4096],
        }
    }
    fn at(&mut self, x: i32, y: i32) -> Option<usize> {
        if (0..64).contains(&x) && (0..64).contains(&y) {
            Some((y * 64 + x) as usize)
        } else {
            None
        }
    }
    /// Axis-aligned solid rectangle (rails, bars, icon parts).
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, depth: f32, alb: Rgb) {
        for py in (y.round() as i32)..=(y + h - 1.0).round() as i32 {
            for px in (x.round() as i32)..=(x + w - 1.0).round() as i32 {
                if let Some(i) = self.at(px, py) {
                    self.h[i] += depth;
                    self.alb[i] = (alb[0], alb[1], alb[2]);
                    self.emissive[i] = 0.0;
                    self.alpha[i] = 255;
                }
            }
        }
    }
    /// Thick line segment (blades, stems, key shafts, icon strokes).
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, half: f32, depth: f32, alb: Rgb) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for py in 0..64 {
            for px in 0..64 {
                let t = (((px as f32 - x0) * dx + (py as f32 - y0) * dy) / len2).clamp(0.0, 1.0);
                let (cx, cy) = (x0 + dx * t, y0 + dy * t);
                let d2 = (px as f32 - cx).powi(2) + (py as f32 - cy).powi(2);
                if d2 <= half * half {
                    if let Some(i) = self.at(px, py) {
                        self.h[i] += depth * (1.0 - d2 / (half * half)).sqrt();
                        self.alb[i] = (alb[0], alb[1], alb[2]);
                        self.emissive[i] = 0.0;
                        self.alpha[i] = 255;
                    }
                }
            }
        }
    }
    /// Solid triangle (the selection chevron).
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), cc: (f32, f32), depth: f32, alb: Rgb) {
        let sign = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
            (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1)
        };
        // Winding of the triangle itself decides which side is "inside".
        let neg = sign(a, b, cc) < 0.0;
        for py in 0..64 {
            for px in 0..64 {
                let p = (px as f32 + 0.5, py as f32 + 0.5);
                let (s1, s2, s3) = (sign(a, b, p), sign(b, cc, p), sign(cc, a, p));
                let inside = if neg {
                    s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0
                } else {
                    s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0
                };
                if inside {
                    if let Some(i) = self.at(px, py) {
                        self.h[i] += depth * 0.8;
                        self.alb[i] = (alb[0], alb[1], alb[2]);
                        self.emissive[i] = 0.0;
                        self.alpha[i] = 255;
                    }
                }
            }
        }
    }
    /// Vertical albedo ramp over the pixels already laid down (panel wells,
    /// button bodies, the nameplate brow).
    fn ramp(&mut self, y0: f32, y1: f32, top: Rgb, bot: Rgb) {
        for py in 0..64 {
            for px in 0..64 {
                let i = (py * 64 + px) as usize;
                if self.alpha[i] == 0 {
                    continue;
                }
                let t = ((py as f32 - y0) / (y1 - y0).max(1.0)).clamp(0.0, 1.0);
                let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
                self.alb[i] = (l(top[0], bot[0]), l(top[1], bot[1]), l(top[2], bot[2]));
            }
        }
    }
    /// Raise a disc with spherical-cap relief.
    fn disc(&mut self, cx: f32, cy: f32, r: f32, depth: f32, alb: Rgb, emissive: f32) {
        for py in (cy - r) as i32..=(cy + r) as i32 + 1 {
            for px in (cx - r) as i32..=(cx + r) as i32 + 1 {
                if let Some(i) = self.at(px, py) {
                    let dx = px as f32 - cx;
                    let dy = py as f32 - cy;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= r * r {
                        let relief = (1.0 - d2 / (r * r)).sqrt();
                        self.h[i] += depth * relief;
                        self.alb[i] = (alb[0], alb[1], alb[2]);
                        self.emissive[i] = emissive;
                        self.alpha[i] = 255;
                    }
                }
            }
        }
    }
    /// Ring band with a bevel that peaks at the band middle.
    fn ring(&mut self, cx: f32, cy: f32, r_out: f32, r_in: f32, depth: f32, alb: Rgb) {
        for py in 0..64 {
            for px in 0..64 {
                if let Some(i) = self.at(px, py) {
                    let d = ((px as f32 - cx).powi(2) + (py as f32 - cy).powi(2)).sqrt();
                    if (r_in..=r_out).contains(&d) {
                        // Bevel profile: rounded ridge across the band width.
                        let w = r_out - r_in;
                        let t = ((r_out - d) / w).min((d - r_in) / w);
                        self.h[i] += depth * (t * 2.0).sqrt();
                        self.alb[i] = (alb[0], alb[1], alb[2]);
                        self.alpha[i] = 255;
                    }
                }
            }
        }
    }
    /// Rounded-rect ring segment in a box frame (edge pieces, buttons).
    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, thick: f32, depth: f32, alb: Rgb) {
        for py in 0..64 {
            for px in 0..64 {
                if let Some(i) = self.at(px, py) {
                    let fx = px as f32 - x;
                    let fy = py as f32 - y;
                    if fx >= 0.0 && fx < w && fy >= 0.0 && fy < h {
                        let d_edge = fx.min(fy).min(w - fx - 1.0).min(h - fy - 1.0);
                        if d_edge < thick {
                            let t = (d_edge / thick).min(1.0 - d_edge / thick);
                            self.h[i] += depth * (t * 2.0 + 0.3).sqrt();
                            self.alb[i] = (alb[0], alb[1], alb[2]);
                            self.alpha[i] = 255;
                        }
                    }
                }
            }
        }
    }
    /// Solid rounded-rect slab with a top-ish bevel slope.
    fn slab(&mut self, x: f32, y: f32, w: f32, h: f32, bevel: f32, depth: f32, alb: Rgb) {
        for py in 0..64 {
            for px in 0..64 {
                if let Some(i) = self.at(px, py) {
                    let fx = px as f32 - x;
                    let fy = py as f32 - y;
                    if fx >= 0.0 && fx < w && fy >= 0.0 && fy < h {
                        // Slight dome toward the upper-left.
                        let sx = 1.0 - fx / w;
                        let sy = 1.0 - fy / h;
                        self.h[i] += depth * (0.4 + 0.6 * sx.min(sy) + bevel * 0.0);
                        self.alb[i] = (alb[0], alb[1], alb[2]);
                        self.alpha[i] = 255;
                    }
                }
            }
        }
    }
    /// Carved notch (cut down into the surface).
    #[allow(dead_code)]
    fn notch(&mut self, cx: f32, cy: f32, r: f32, depth: f32, alb: Rgb) {
        self.disc(cx, cy, r, -depth, alb, 0.0);
    }

    /// Facet gem: cut-stone circle with a darker lower half and a spec chip.
    fn gem(&mut self, cx: f32, cy: f32, r: f32, gem: Rgb, emissive: f32) {
        self.disc(cx, cy, r, 3.0, gem, emissive);
        for py in 0..64 {
            for px in 0..64 {
                if let Some(i) = self.at(px, py) {
                    let dx = px as f32 - cx;
                    let dy = py as f32 - cy;
                    if dx * dx + dy * dy <= r * r * 0.6 && dy > -r * 0.2 {
                        self.alb[i] = (
                            (gem[0] as f32 * 0.62) as u8,
                            (gem[1] as f32 * 0.62) as u8,
                            (gem[2] as f32 * 0.62) as u8,
                        );
                    }
                }
            }
        }
        // Spec chip, bloom-disciplined (2px, capped luminance in light pass).
        self.disc(cx - r * 0.35, cy - r * 0.4, 1.2, 4.0, [240, 240, 235], 0.0);
    }

    fn mottle(&mut self, seed: u64, amp: f32) {
        for py in 0..64 {
            for px in 0..64 {
                if let Some(i) = self.at(px, py) {
                    if self.alpha[i] == 0 {
                        continue; // never resurrect a carved-away pixel
                    }
                    let n = h01(seed, (py * 64 + px) as u64);
                    let (r, g, b) = self.alb[i];
                    let k = 1.0 + (n - 0.5) * amp;
                    self.alb[i] = (
                        (r as f32 * k).clamp(0.0, 255.0) as u8,
                        (g as f32 * k).clamp(0.0, 255.0) as u8,
                        (b as f32 * k).clamp(0.0, 255.0) as u8,
                    );
                }
            }
        }
    }

    /// The D38 light pass: normals from the depth gradient, NW key + warm
    /// fill + cavity AO + capped spec; writes the RGBA plate.
    fn finish(self) -> RgbaImage {
        let mut img = blank(64, 64);
        let h_of = |x: i32, y: i32| -> f32 {
            let cx = x.clamp(0, 63);
            let cy = y.clamp(0, 63);
            self.h[(cy * 64 + cx) as usize]
        };
        for py in 0..64i32 {
            for px in 0..64i32 {
                let i = (py * 64 + px) as usize;
                if self.alpha[i] == 0 {
                    continue;
                }
                let hx = (h_of(px + 1, py) - h_of(px - 1, py)) / 2.0;
                let hy = (h_of(px, py + 1) - h_of(px, py - 1)) / 2.0;
                let inv = 1.0 / (hx * hx + hy * hy + 3.2).sqrt();
                let (nx, ny, nz) = (-hx * inv, -hy * inv, 3.2_f32.sqrt() * inv);
                // Cavity AO: how much the neighbourhood walls surround us.
                let mut occ = 0.0;
                for (dx, dy) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
                    occ += (h_of(px + dx, py + dy) - self.h[i] - 1.2).max(0.0);
                }
                let ao = (1.0 / (1.0 + 0.35 * occ)).clamp(0.42, 1.0);
                let key = (nx * -0.42 + ny * 0.68 + nz * 0.55).max(0.0);
                let fill = (nx * 0.55 + ny * -0.25 + nz * 0.35).max(0.0);
                let mut lum = 0.36 * ao + 0.80 * key + 0.16 * fill + self.emissive[i];
                lum = lum.min(1.30);
                let alb = self.alb[i];
                let c = shade([alb.0, alb.1, alb.2], lum);
                img.put_pixel(px as u32, py as u32, image::Rgba([c[0], c[1], c[2], self.alpha[i]]));
            }
        }
        img
    }
}

// ---------------------------------------------------------------------------
// The plate family.

fn orb_frame(gem: Rgb) -> RgbaImage {
    let mut c = Chrome64::new(0.0, STONE_DARK);
    // Outer brass ridge + inner lip, dark stone well, four rivets.
    c.ring(32.0, 32.0, 30.0, 22.0, 4.5, BRASS);
    c.ring(32.0, 32.0, 22.5, 20.0, 2.5, BRASS_LO);
    c.disc(32.0, 32.0, 20.0, -2.5, STONE_DARK, 0.0);
    // Four brass rivets at the diagonals.
    for (dx, dy) in [(-17.9, -17.9), (17.9, -17.9), (-17.9, 17.9), (17.9, 17.9)] {
        c.disc(32.0 + dx, 32.0 + dy, 2.6, 2.5, BRASS_HI, 0.0);
    }
    // Hue gem seated in the north rim.
    c.ring(32.0, 5.0, 6.5, 3.0, 2.0, BRASS_LO);
    c.gem(32.0, 5.0, 4.2, gem, 0.10);
    c.mottle(0xc011, 0.10);
    c.finish()
}

/// Brass rail thickness inside a 64px cell. The corner/edge plates are
/// CARVED to this rail: the 9-patch stretches the rail, never a slab of
/// stone (v2 shipped a 64px-thick stone band that read as the world showing
/// through the pane).
const RAIL: f32 = 3.0;

fn panel_corner(dx: f32, dy: f32) -> RgbaImage {
    // Brass elbow: arms along the two OUTER edges meeting at a corner rivet.
    // dx<0 => west cell: arm on LEFT edge; dy<0 => north cell: arm on TOP edge.
    let mut c = Chrome64::new_clear();
    let ax = if dx < 0.0 { 0.0 } else { 64.0 - RAIL };
    let ay = if dy < 0.0 { 0.0 } else { 64.0 - RAIL };
    let hi_y = if dy < 0.0 { 0.0 } else { 63.0 };
    let hi_x = if dx < 0.0 { 0.0 } else { 63.0 };
    let lo_y = if dy < 0.0 { RAIL } else { 64.0 - RAIL - 2.0 };
    let lo_x = if dx < 0.0 { RAIL } else { 64.0 - RAIL - 2.0 };
    c.rect(0.0, ay, 64.0, RAIL, 2.2, BRASS); // horizontal arm
    c.rect(ax, 0.0, RAIL, 64.0, 2.2, BRASS); // vertical arm
    c.rect(0.0, hi_y, 64.0, 1.0, 3.4, BRASS_HI); // high lip, outer edge
    c.rect(hi_x, 0.0, 1.0, 64.0, 3.4, BRASS_HI);
    c.rect(0.0, lo_y, 64.0, 2.0, -1.0, BRASS_LO); // undercut, inner side
    c.rect(lo_x, 0.0, 2.0, 64.0, -1.0, BRASS_LO);
    // Corner rivet at the elbow joint.
    let (bx, by) = (
        if dx < 0.0 { 6.0 } else { 58.0 },
        if dy < 0.0 { 6.0 } else { 58.0 },
    );
    c.disc(bx, by, 3.6, 2.8, BRASS_HI, 0.0);
    c.disc(bx, by, 1.4, -1.2, INK, 0.0);
    c.mottle(0xc022, 0.10);
    c.finish()
}

fn panel_edge(horizontal: bool) -> RgbaImage {
    let mut c = Chrome64::new_clear();
    if horizontal {
        c.rect(0.0, 0.0, 64.0, RAIL, 2.2, BRASS);
        c.rect(0.0, 0.0, 64.0, 1.0, 3.4, BRASS_HI);
        c.rect(0.0, RAIL, 64.0, 2.0, -1.0, BRASS_LO);
    } else {
        c.rect(0.0, 0.0, RAIL, 64.0, 2.2, BRASS);
        c.rect(0.0, 0.0, 1.0, 64.0, 3.4, BRASS_HI);
        c.rect(RAIL, 0.0, 2.0, 64.0, -1.0, BRASS_LO);
    }
    c.mottle(0xc033, 0.10);
    c.finish()
}

/// Pane well: the dark inset the content sits on. Stretched to the pane
/// interior, so it carries no horizontal or vertical detail — only a soft
/// top-lit ramp and whisper-level mottle (the old bright stone fill read as
/// daylight bleeding through the panel).
fn panel_fill() -> RgbaImage {
    let mut c = Chrome64::new(0.5, INK);
    c.ramp(0.0, 64.0, [40, 44, 49], [15, 18, 22]);
    c.mottle(0xc044, 0.05);
    c.finish()
}

/// Inventory/slot housing: a 1:1 plate (never stretched): dark well, thin
/// brass rim, and a soft inner shadow so an item icon reads as seated.
fn icon_slot() -> RgbaImage {
    let mut c = Chrome64::new_clear();
    c.rect(3.0, 3.0, 58.0, 58.0, 0.0, [26, 29, 34]);
    c.ramp(3.0, 61.0, [34, 38, 44], [16, 18, 22]);
    c.rect(3.0, 3.0, 58.0, 2.0, 2.4, BRASS);
    c.rect(3.0, 59.0, 58.0, 2.0, 2.4, BRASS);
    c.rect(3.0, 3.0, 2.0, 58.0, 2.4, BRASS);
    c.rect(59.0, 3.0, 2.0, 58.0, 2.4, BRASS);
    c.rect(3.0, 3.0, 1.0, 1.0, 3.0, BRASS_HI); // lit corner chips
    c.rect(60.0, 3.0, 1.0, 1.0, 3.0, BRASS_HI);
    c.mottle(0xc0a1, 0.08);
    c.finish()
}

/// Selection chevron: a solid brass arrow, the pane's cursor for the active
/// menu row (drawn at ~10px, so the shape is deliberately blunt).
fn cursor_arrow() -> RgbaImage {
    let mut c = Chrome64::new_clear();
    c.tri((18.0, 12.0), (18.0, 52.0), (46.0, 32.0), 3.0, BRASS_HI);
    c.tri((22.0, 21.0), (22.0, 43.0), (38.0, 32.0), -1.6, BRASS_LO);
    c.mottle(0xc0a2, 0.08);
    c.finish()
}

/// Boss brow: a slim, full-cell rail (brass top and bottom, dark stone
/// body). The consumer scales it to the MEASURED name width, so nothing here
/// may carry horizontal detail — a stretched arch or gem collapses into a
/// smear. The caps carry the end brackets.
fn nameplate(cap: i32) -> RgbaImage {
    let mut c = Chrome64::new_clear();
    c.rect(0.0, 0.0, 64.0, 64.0, 0.0, STONE_DARK);
    c.ramp(0.0, 64.0, [38, 35, 32], [16, 15, 14]);
    c.rect(0.0, 0.0, 64.0, 2.0, 3.0, BRASS_HI);
    c.rect(0.0, 2.0, 64.0, 3.0, 2.2, BRASS);
    c.rect(0.0, 59.0, 64.0, 3.0, 2.2, BRASS);
    c.rect(0.0, 62.0, 64.0, 2.0, 3.0, BRASS_LO);
    if cap != 0 {
        let left = cap < 0;
        let bx = if left { 0.0 } else { 57.0 };
        let hx = if left { 0.0 } else { 63.0 };
        let cx0 = if left { 0.0 } else { 54.0 };
        c.rect(bx, 0.0, 7.0, 64.0, 3.0, BRASS); // end bracket post
        c.rect(hx, 0.0, 1.0, 64.0, 3.6, BRASS_HI); // its lit outer edge
        c.rect(cx0, 22.0, 10.0, 20.0, 2.4, BRASS_HI); // collar mid-post
        c.rect(cx0, 30.0, 10.0, 4.0, -1.2, BRASS_LO); // collar seal
    }
    c.mottle(0xc055, 0.08);
    c.finish()
}

/// Menu-row strip. The pane stretches it to the row width, so only its
/// VERTICAL profile may carry detail: dark body, brass rail top and bottom,
/// warmer body plus an inner band when selected. (The v2 framed box put a
/// 4.5px frame in a 48px cell — stretched to a 20px row those side frames
/// became two 26px brass slabs down the row's flanks.)
fn button(selected: bool) -> RgbaImage {
    let mut c = Chrome64::new(0.4, INK);
    let (top, bot) = if selected {
        ([68, 57, 34], [30, 25, 17])
    } else {
        ([30, 32, 35], [16, 18, 20])
    };
    c.ramp(0.0, 64.0, top, bot);
    let rail = if selected { BRASS_HI } else { BRASS };
    c.rect(0.0, 0.0, 64.0, 5.0, 2.6, rail);
    c.rect(0.0, 59.0, 64.0, 5.0, 2.6, rail);
    c.rect(0.0, 0.0, 64.0, 1.0, 3.4, BRASS_HI);
    c.rect(0.0, 63.0, 64.0, 1.0, 3.4, BRASS_LO);
    if selected {
        // Inner warm band just inside each rail: the "cursor" read.
        c.rect(0.0, 5.0, 64.0, 4.0, 0.6, [118, 90, 42]);
        c.rect(0.0, 55.0, 64.0, 4.0, 0.6, [118, 90, 42]);
    }
    c.mottle(if selected { 0xc067 } else { 0xc066 }, 0.06);
    c.finish()
}

// ---------------------------------------------------------------------------
// Item pictograms. Four 32px icons share one 64px plate cell (25 plates is the
// 8x3 sheet ceiling), and the consumer slices the cell into quadrants. Each
// icon must read at a glance inside a 20px inventory slot / 18px HUD slot, so
// the shapes are blunt and hard-edged rather than finely detailed.

/// Local-coordinate painter over one 32px quadrant of a 64px plate.
struct Icon<'a> {
    c: &'a mut Chrome64,
    ox: f32,
    oy: f32,
}

impl<'a> Icon<'a> {
    /// q: 0 top-left, 1 top-right, 2 bottom-left, 3 bottom-right.
    fn at(c: &'a mut Chrome64, q: usize) -> Self {
        Self {
            c,
            ox: (q % 2) as f32 * 32.0,
            oy: (q / 2) as f32 * 32.0,
        }
    }
    fn r(&mut self, x: f32, y: f32, w: f32, h: f32, d: f32, a: Rgb) {
        self.c.rect(self.ox + x, self.oy + y, w, h, d, a)
    }
    fn disc(&mut self, x: f32, y: f32, r: f32, d: f32, a: Rgb, e: f32) {
        self.c.disc(self.ox + x, self.oy + y, r, d, a, e)
    }
    fn ring(&mut self, x: f32, y: f32, ro: f32, ri: f32, d: f32, a: Rgb) {
        self.c.ring(self.ox + x, self.oy + y, ro, ri, d, a)
    }
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, h: f32, d: f32, a: Rgb) {
        self.c
            .line(self.ox + x0, self.oy + y0, self.ox + x1, self.oy + y1, h, d, a)
    }
    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, d: f32, a: Rgb) {
        self.c.frame(self.ox + x, self.oy + y, w, h, t, d, a)
    }
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c2: (f32, f32), d: f32, a2: Rgb) {
        self.c.tri(
            (self.ox + a.0, self.oy + a.1),
            (self.ox + b.0, self.oy + b.1),
            (self.ox + c2.0, self.oy + c2.1),
            d,
            a2,
        )
    }
    fn gem(&mut self, x: f32, y: f32, r: f32, a: Rgb, e: f32) {
        self.c.gem(self.ox + x, self.oy + y, r, a, e)
    }
}

#[derive(Clone, Copy)]
enum IconKind {
    Potion,
    Tonic,
    Ration,
    Torch,
    Sword,
    Armour,
    Relic,
    Key,
    Rune,
    Dust,
    Ore,
    Herb,
    Crate,
    Scroll,
    Coin,
    Seal,
}

/// The 32px pictogram grid: sheet N holds kinds [4N .. 4N+4] in quadrant order.
fn icon_sheet(sheet: usize) -> RgbaImage {
    const GRID: [[IconKind; 4]; 4] = [
        [IconKind::Potion, IconKind::Tonic, IconKind::Ration, IconKind::Torch],
        [IconKind::Sword, IconKind::Armour, IconKind::Relic, IconKind::Key],
        [IconKind::Rune, IconKind::Dust, IconKind::Ore, IconKind::Herb],
        [IconKind::Crate, IconKind::Scroll, IconKind::Coin, IconKind::Seal],
    ];
    let mut c = Chrome64::new_clear();
    for (q, kind) in GRID[sheet].iter().enumerate() {
        let mut i = Icon::at(&mut c, q);
        match kind {
            IconKind::Potion => {
                i.r(12.5, 6.0, 7.0, 6.0, 0.0, GLASS);
                i.r(10.5, 3.5, 11.0, 3.0, 2.4, BRASS_HI); // stopper
                i.disc(16.0, 21.0, 9.0, 2.6, GLASS, 0.0);
                i.disc(16.0, 23.2, 6.8, 1.4, LIQUID_RED, 0.05);
                i.disc(12.6, 17.4, 1.7, 3.0, STEEL_HI, 0.0); // glint
            }
            IconKind::Tonic => {
                i.r(12.5, 5.0, 7.0, 6.0, 0.0, GLASS);
                i.r(10.5, 2.8, 11.0, 3.0, 2.4, BRASS_HI);
                i.r(9.5, 10.0, 13.0, 15.0, 2.4, GLASS);
                i.r(11.2, 17.5, 9.6, 6.6, 1.2, LIQUID_BLUE);
                i.disc(12.6, 13.0, 1.5, 3.0, STEEL_HI, 0.0);
            }
            IconKind::Ration => {
                i.disc(8.5, 17.5, 5.4, 2.4, [166, 126, 70], 0.0);
                i.disc(23.5, 17.5, 5.4, 2.4, [166, 126, 70], 0.0);
                i.r(8.0, 12.5, 16.0, 10.0, 2.4, [166, 126, 70]);
                i.disc(22.0, 14.0, 3.2, 2.0, [196, 156, 94], 0.0); // crust
                for k in 0..3 {
                    i.line(11.5 + k as f32 * 4.5, 13.0, 11.5 + k as f32 * 4.5, 21.0, 0.9, -1.2, [112, 80, 42]);
                }
            }
            IconKind::Torch => {
                i.line(15.0, 29.0, 16.5, 14.0, 2.0, 1.6, WOOD);
                i.r(13.0, 12.0, 6.0, 3.0, 2.4, BRASS_LO);
                i.disc(16.0, 8.5, 5.0, 2.0, FLAME, 0.26);
                i.disc(16.0, 7.5, 2.6, 2.6, FLAME_HI, 0.44);
                i.disc(11.6, 4.6, 1.1, 2.0, FLAME, 0.30);
                i.disc(20.8, 5.6, 0.9, 2.0, FLAME_HI, 0.30);
            }
            IconKind::Sword => {
                i.line(16.0, 2.5, 16.0, 20.0, 1.8, 3.0, STEEL_HI);
                i.line(16.0, 3.6, 16.0, 19.0, 0.7, 3.2, STEEL);
                i.r(9.0, 19.5, 14.0, 2.6, 2.4, BRASS);
                i.r(9.0, 19.5, 14.0, 1.0, 3.2, BRASS_HI);
                i.line(16.0, 22.4, 16.0, 27.0, 1.5, 1.4, WOOD);
                i.disc(16.0, 28.6, 2.2, 2.4, BRASS_HI, 0.0);
            }
            IconKind::Armour => {
                i.disc(7.6, 10.5, 3.6, 2.4, STEEL, 0.0);
                i.disc(24.4, 10.5, 3.6, 2.4, STEEL, 0.0);
                i.r(8.0, 9.0, 16.0, 13.0, 2.6, STEEL);
                i.r(8.0, 9.0, 16.0, 2.6, 1.0, STEEL_HI);
                i.tri((12.0, 9.0), (20.0, 9.0), (16.0, 14.0), -1.4, IRON_LO); // gorget
                i.r(10.0, 17.6, 12.0, 1.2, -0.8, IRON_LO);
                i.r(10.0, 20.2, 12.0, 1.2, -0.8, IRON_LO);
            }
            IconKind::Relic => {
                i.ring(16.0, 9.5, 6.0, 4.4, 2.0, BRASS); // chain ring
                i.line(16.0, 15.0, 16.0, 18.0, 1.2, 2.0, BRASS_HI); // bail
                i.gem(16.0, 23.0, 5.2, VIOLET, 0.05);
            }
            IconKind::Key => {
                i.ring(9.5, 16.0, 6.0, 3.6, 2.2, BRASS);
                i.line(14.0, 16.0, 27.0, 16.0, 1.5, 2.2, BRASS_HI);
                i.r(21.6, 16.0, 1.5, 5.0, 2.0, BRASS_HI);
                i.r(25.2, 16.0, 1.5, 3.4, 2.0, BRASS_HI);
            }
            IconKind::Rune => {
                i.r(7.0, 6.0, 18.0, 20.0, 2.6, [112, 114, 110]);
                i.r(7.0, 6.0, 18.0, 2.6, 1.0, [146, 148, 144]);
                i.line(11.0, 11.0, 21.0, 11.0, 1.0, -1.4, INK);
                i.line(16.0, 11.0, 16.0, 21.0, 1.0, -1.4, INK);
                i.line(16.0, 17.0, 21.0, 22.0, 1.0, -1.4, INK);
                i.disc(10.8, 8.8, 1.2, 2.2, SEA, 0.28);
            }
            IconKind::Dust => {
                i.disc(16.0, 23.5, 9.6, 2.0, SAND, 0.0);
                i.disc(11.6, 20.0, 5.2, 2.4, [158, 146, 116], 0.0);
                i.disc(20.6, 19.6, 4.4, 2.4, [158, 146, 116], 0.0);
                i.disc(16.0, 16.8, 3.0, 3.0, [178, 166, 132], 0.0);
                i.disc(10.4, 9.6, 1.2, 2.4, BRASS_HI, 0.30);
                i.disc(21.6, 8.4, 1.0, 2.4, BRASS_HI, 0.30);
                i.disc(16.0, 6.2, 0.9, 2.4, BRASS_HI, 0.30);
            }
            IconKind::Ore => {
                i.disc(16.0, 18.0, 9.0, 3.0, [122, 124, 120], 0.0);
                i.line(8.6, 16.0, 16.0, 10.6, 1.0, 1.4, [152, 154, 150]);
                i.line(16.0, 10.6, 23.6, 16.2, 1.0, 1.4, [152, 154, 150]);
                i.line(16.0, 11.0, 16.0, 26.0, 0.9, -0.8, [92, 94, 92]);
                i.disc(12.8, 17.2, 1.4, 3.0, BRASS_HI, 0.28);
                i.disc(19.2, 20.6, 1.0, 3.0, BRASS_HI, 0.28);
            }
            IconKind::Herb => {
                i.line(16.0, 28.0, 16.0, 12.0, 1.1, 1.4, [96, 140, 80]);
                i.disc(11.6, 13.0, 3.6, 1.8, LEAF, 0.0);
                i.disc(20.4, 13.0, 3.6, 1.8, LEAF, 0.0);
                i.disc(11.0, 19.6, 3.2, 1.8, LEAF_HI, 0.0);
                i.disc(21.0, 19.6, 3.2, 1.8, LEAF_HI, 0.0);
                i.disc(16.0, 9.0, 2.4, 2.4, BERRY, 0.0);
            }
            IconKind::Crate => {
                i.frame(5.0, 9.0, 22.0, 17.0, 2.6, 2.6, [140, 106, 58]);
                i.line(6.5, 10.5, 25.5, 24.5, 1.3, 1.2, [164, 128, 74]);
                i.line(25.5, 10.5, 6.5, 24.5, 1.3, 1.2, [164, 128, 74]);
                i.disc(16.0, 17.2, 2.2, 2.6, BRASS_HI, 0.0);
            }
            IconKind::Scroll => {
                i.r(9.0, 7.0, 14.0, 18.0, 2.0, PARCHMENT);
                i.disc(9.0, 16.0, 3.8, 2.4, [168, 152, 116], 0.0);
                i.disc(23.0, 16.0, 3.8, 2.4, [168, 152, 116], 0.0);
                i.r(12.6, 11.0, 6.4, 1.0, -0.8, [124, 108, 80]);
                i.r(12.6, 14.0, 6.4, 1.0, -0.8, [124, 108, 80]);
                i.r(12.6, 17.0, 4.0, 1.0, -0.8, [124, 108, 80]);
            }
            IconKind::Coin => {
                i.gem(16.0, 17.0, 10.0, BRASS_HI, 0.0);
                i.ring(16.0, 17.0, 7.4, 6.2, 1.6, BRASS_LO);
                i.r(14.0, 12.6, 4.0, 1.4, -1.2, BRASS_LO);
                i.r(14.0, 16.6, 4.0, 1.4, -1.2, BRASS_LO);
                i.r(15.3, 11.6, 1.4, 8.0, -1.2, BRASS_LO);
            }
            IconKind::Seal => {
                i.ring(16.0, 16.0, 12.0, 8.2, 2.6, BRASS);
                for k in 0..4 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_2;
                    i.disc(16.0 + a.cos() * 10.0, 16.0 + a.sin() * 10.0, 2.1, 2.0, BRASS_LO, 0.0);
                }
                i.disc(16.0, 16.0, 7.6, 2.4, WAX, 0.0);
                i.r(15.1, 12.0, 1.8, 8.6, -1.0, [118, 32, 32]);
                i.r(12.0, 15.1, 8.6, 1.8, -1.0, [118, 32, 32]);
            }
        }
    }
    c.finish()
}

fn sigil_pip() -> RgbaImage {
    let mut c = Chrome64::new(0.0, STONE_DARK);
    // Octagonal brass housing with a rune-lit core.
    c.ring(32.0, 32.0, 17.0, 12.0, 3.0, BRASS);
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2;
        c.disc(32.0 + a.cos() * 14.5, 32.0 + a.sin() * 14.5, 2.4, 2.0, BRASS_LO, 0.0);
    }
    c.disc(32.0, 32.0, 11.0, -1.5, INK, 0.0);
    c.disc(32.0, 32.0, 5.5, 1.5, SEA, 0.55);
    c.finish()
}

fn xp_sliver() -> RgbaImage {
    let mut c = Chrome64::new(0.0, STONE_DARK);
    // Slim rail housing with a recessed channel (fill is consumer-drawn).
    c.frame(2.0, 22.0, 60.0, 20.0, 3.0, 2.6, BRASS);
    c.slab(5.0, 25.0, 54.0, 14.0, 1.0, -1.8, STONE_DARK);
    for x in [12.0, 24.0, 36.0, 48.0] {
        c.slab(x, 25.0, 1.5, 14.0, 0.5, -0.8, BRASS_LO);
    }
    c.mottle(0xc088, 0.10);
    c.finish()
}

/// The chrome sheet: 8 cols x 3 rows (the sheet ceiling `verify` enforces),
/// keys in cell order.
pub fn chrome_plates() -> Vec<(&'static str, RgbaImage)> {
    vec![
        ("OrbFrameRed", orb_frame(HP_RED)),
        ("OrbFrameBlue", orb_frame(MANA_BLUE)),
        ("PanelCornerNW", panel_corner(-1.0, -1.0)),
        ("PanelCornerNE", panel_corner(1.0, -1.0)),
        ("PanelCornerSW", panel_corner(-1.0, 1.0)),
        ("PanelCornerSE", panel_corner(1.0, 1.0)),
        ("PanelEdgeN", panel_edge(true)),
        ("PanelEdgeS", {
            let mut img = panel_edge(true);
            img = flip_v(&img);
            img
        }),
        ("PanelEdgeW", panel_edge(false)),
        ("PanelEdgeE", {
            let mut img = panel_edge(false);
            img = flip_h(&img);
            img
        }),
        ("PanelFill", panel_fill()),
        ("BossNameplateL", nameplate(-1)),
        ("BossNameplateM", nameplate(0)),
        ("BossNameplateR", nameplate(1)),
        ("ButtonNormal", button(false)),
        ("ButtonSelected", button(true)),
        ("SigilPip", sigil_pip()),
        ("XPSliver", xp_sliver()),
        // Row 2 (last cells of the 8x3 ceiling): the item pictogram family.
        // Six plates, 24 cells — one short of the ceiling.
        ("IconSlot", icon_slot()),
        ("IconCursor", cursor_arrow()),
        ("ItemIconsA", icon_sheet(0)),
        ("ItemIconsB", icon_sheet(1)),
        ("ItemIconsC", icon_sheet(2)),
        ("ItemIconsD", icon_sheet(3)),
    ]
}

fn flip_h(img: &RgbaImage) -> RgbaImage {
    let mut out = blank(64, 64);
    for (x, y, p) in img.enumerate_pixels() {
        out.put_pixel(63 - x, y, *p);
    }
    out
}
fn flip_v(img: &RgbaImage) -> RgbaImage {
    let mut out = blank(64, 64);
    for (x, y, p) in img.enumerate_pixels() {
        out.put_pixel(x, 63 - y, *p);
    }
    out
}
