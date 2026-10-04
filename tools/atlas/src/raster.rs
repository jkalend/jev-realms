//! Pure-RGBA raster helpers for offline plate painting.
//!
//! Ported from the throwaway macroquad painters in src/gfxlab.rs; here every
//! primitive rasterises pixel-centre-exact into an `image::RgbaImage` so the
//! bake is deterministic, GPU-free, and CI-runnable.

use image::{Rgba, RgbaImage};

pub type Rgb = [u8; 3];

/// Dark ink used by the chunky arm (sprites::INK, 0.035/0.043/0.047).
pub const INK: Rgb = [9, 11, 12];

#[inline]
fn shade1(v: u8, k: f32) -> u8 {
    (v as f32 * k).clamp(0.0, 255.0).round() as u8
}

/// gfxlab::shade — per-channel multiplier.
pub fn shade(c: Rgb, k: f32) -> Rgb {
    [shade1(c[0], k), shade1(c[1], k), shade1(c[2], k)]
}

/// gfxlab::paint_terrain — the grim painterly transform: desaturate ~40%,
/// deepen ~15%. Every environment plate colour passes through this.
pub fn grim(c: Rgb) -> Rgb {
    let gray = c[0] as f32 * 0.30 + c[1] as f32 * 0.59 + c[2] as f32 * 0.11;
    let sat = |v: u8| gray + (v as f32 - gray) * 0.60;
    shade(
        [sat(c[0]) as u8, sat(c[1]) as u8, sat(c[2]) as u8],
        0.85,
    )
}

/// Integer avalanche mix (sprites' `mix` role): stable coordinate hashing
/// that varies placement without touching game RNG.
pub fn hash64(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    x
}

/// Deterministic hash to [0,1).
pub fn h01(seed: u64, i: u64) -> f32 {
    let v = hash64(seed ^ i.wrapping_mul(0x9e37_79b9_7f4a_7c15)) >> 40;
    v as f64 as f32 / ((1u64 << 24) as f64 as f32)
}

pub fn blank(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]))
}

#[inline]
pub fn put(img: &mut RgbaImage, x: i32, y: i32, c: Rgb) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    if x >= 0 && y >= 0 && x < w && y < h {
        img.put_pixel(x as u32, y as u32, Rgba([c[0], c[1], c[2], 255]));
    }
}

/// Alpha-composite `c` over the existing pixel (edge-dither flecks rely on
/// this: they blend against whatever the renderer stacks beneath).
#[inline]
pub fn blend_px(img: &mut RgbaImage, x: i32, y: i32, c: Rgb, a: u8) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return;
    }
    let d = img.get_pixel(x as u32, y as u32).0;
    let sa = a as u32;
    let da = d[3] as u32;
    // src-over in straight alpha.
    let oa = sa + da * (255 - sa) / 255;
    let px = if oa == 0 {
        [0, 0, 0, 0]
    } else {
        let mix = |s: u8, dch: u8| -> u8 {
            ((s as u32 * sa * 255 + dch as u32 * da * (255 - sa)) / (oa * 255)) as u8
        };
        [
            mix(c[0], d[0]),
            mix(c[1], d[1]),
            mix(c[2], d[2]),
            oa.min(255) as u8,
        ]
    };
    img.put_pixel(x as u32, y as u32, Rgba(px));
}

pub fn fill_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, c: Rgb) {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            put(img, px, py, c);
        }
    }
}

pub fn blend_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, c: Rgb, a: u8) {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            blend_px(img, px, py, c, a);
        }
    }
}

/// Alpha-blended disc, for broad tonal passes that must not punch a hard edge
/// into a tile (grass patches, wake lines).
pub fn blend_circle(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, c: Rgb, a: u8) {
    let r2 = (r * r + 0.25) as f32;
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    let x0 = (cx - r).floor() as i32;
    let x1 = (cx + r).ceil() as i32;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                blend_px(img, px, py, c, a);
            }
        }
    }
}

pub fn fill_circle(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, c: Rgb) {
    let r2 = (r * r + 0.25) as f32;
    let y0 = (cy - r).floor() as i32;
    let y1 = (cy + r).ceil() as i32;
    let x0 = (cx - r).floor() as i32;
    let x1 = (cx + r).ceil() as i32;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                put(img, px, py, c);
            }
        }
    }
}

/// Even-odd scanline polygon fill at pixel centres — replaces macroquad's
/// draw_triangle/diamond quads (gfxlab::diamond, ridge, canopy silhouettes).
pub fn fill_poly(img: &mut RgbaImage, pts: &[[f32; 2]], c: Rgb) {
    let mut y0 = f32::MAX;
    let mut y1 = f32::MIN;
    let mut x0 = f32::MAX;
    let mut x1 = f32::MIN;
    for p in pts {
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
    }
    for py in y0.floor() as i32..=y1.ceil() as i32 {
        let sy = py as f32 + 0.5;
        for px in x0.floor() as i32..=x1.ceil() as i32 {
            let sx = px as f32 + 0.5;
            let mut inside = false;
            let n = pts.len();
            for i in 0..n {
                let [ax, ay] = pts[i];
                let [bx, by] = pts[(i + 1) % n];
                if (ay > sy) != (by > sy) && sx < (bx - ax) * (sy - ay) / (by - ay) + ax {
                    inside = !inside;
                }
            }
            if inside {
                put(img, px, py, c);
            }
        }
    }
}

/// gfxlab::diamond — the 2:1 iso diamond (two triangles in the mock, one
/// polygon here; identical footprint).
pub fn diamond(img: &mut RgbaImage, cx: f32, cy: f32, hw: f32, hh: f32, c: Rgb) {
    fill_poly(
        img,
        &[[cx, cy - hh], [cx + hw, cy], [cx, cy + hh], [cx - hw, cy]],
        c,
    );
}

/// gfxlab's side-face `quad` — a 4-point parallelogram.
pub fn quad(img: &mut RgbaImage, pts: [[f32; 2]; 4], c: Rgb) {
    fill_poly(img, &pts, c);
}
