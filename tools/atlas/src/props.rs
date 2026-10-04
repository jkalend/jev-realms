//! Prop plates — 64x64 transparent-cell props for the props sheet.
//!
//! Same C+ chunky finish as actors (ink offset outline via actors::chunky),
//! palette drawn from the environment plates so props sit naturally.

use crate::raster::*;
use image::RgbaImage;

const GOLD: Rgb = [232, 191, 89];
const SEA: Rgb = [102, 212, 196];

fn prop_torch_brazier(img: &mut RgbaImage) {
    let dark = [31, 35, 43];
    let steel = [158, 175, 185];
    let leather = [145, 102, 67];
    // Tripod legs.
    fill_poly(img, &[[22.0, 34.0], [26.0, 34.0], [20.0, 60.0], [16.0, 60.0]], dark);
    fill_poly(img, &[[42.0, 34.0], [38.0, 34.0], [44.0, 60.0], [48.0, 60.0]], dark);
    fill_poly(img, &[[30.0, 34.0], [34.0, 34.0], [34.0, 60.0], [30.0, 60.0]], shade(dark, 1.4));
    // Bowl + rim.
    fill_rect(img, 16.0, 26.0, 32.0, 10.0, shade(steel, 0.55));
    fill_rect(img, 14.0, 24.0, 36.0, 4.0, steel);
    fill_rect(img, 16.0, 34.0, 4.0, 3.0, shade(steel, 0.6));
    // Coals.
    fill_rect(img, 20.0, 22.0, 24.0, 4.0, [180, 60, 40]);
    // Flame: rust outer, amber mid, ivory core.
    let rust = [216, 110, 70];
    fill_poly(img, &[[24.0, 22.0], [32.0, 4.0], [40.0, 22.0]], rust);
    fill_poly(img, &[[27.0, 22.0], [33.0, 8.0], [38.0, 22.0]], [232, 181, 88]);
    fill_poly(img, &[[30.0, 22.0], [33.0, 13.0], [36.0, 22.0]], [233, 228, 210]);
    // Embers + handle hook.
    fill_rect(img, 20.0, 14.0, 2.0, 2.0, rust);
    fill_rect(img, 44.0, 17.0, 2.0, 2.0, GOLD);
    fill_rect(img, 46.0, 26.0, 4.0, 2.0, leather);
}

fn prop_potion(img: &mut RgbaImage) {
    let glass = [120, 150, 160];
    let leather = [145, 102, 67];
    // Neck + lip + cork.
    fill_rect(img, 28.0, 10.0, 8.0, 6.0, shade(glass, 0.85));
    fill_rect(img, 26.0, 8.0, 12.0, 3.0, glass);
    fill_rect(img, 29.0, 4.0, 6.0, 5.0, leather);
    fill_rect(img, 30.0, 4.0, 4.0, 2.0, shade(leather, 1.25));
    // Flask body (bulb).
    fill_circle(img, 32.0, 40.0, 16.0, shade(glass, 0.8));
    fill_circle(img, 32.0, 40.0, 14.0, shade(glass, 0.95));
    // Liquid: sea fill with a darker meniscus.
    fill_circle(img, 32.0, 43.0, 12.0, SEA);
    fill_rect(img, 21.0, 38.0, 22.0, 14.0, SEA);
    fill_rect(img, 22.0, 35.0, 20.0, 3.0, shade(SEA, 1.25));
    fill_circle(img, 29.0, 45.0, 3.0, shade(SEA, 1.3)); // bubble
    fill_circle(img, 35.0, 48.0, 2.0, shade(SEA, 1.35)); // bubble
    // Glass highlight + lip glint.
    blend_rect(img, 24.0, 32.0, 3.0, 12.0, [230, 240, 244], 150);
    blend_rect(img, 27.0, 8.0, 3.0, 2.0, [230, 240, 244], 170);
}

fn prop_rune_stone(img: &mut RgbaImage) {
    let stone = [113, 114, 105];
    // Leaning monolith slab with a lit west bevel.
    fill_poly(img, &[[18.0, 58.0], [22.0, 8.0], [44.0, 6.0], [46.0, 58.0]], shade(stone, 0.8));
    fill_poly(img, &[[22.0, 10.0], [42.0, 8.0], [44.0, 56.0], [20.0, 56.0]], shade(stone, 1.05));
    fill_rect(img, 22.0, 8.0, 20.0, 3.0, shade(stone, 1.25));
    // Runic strokes: one dominant vertical rune + two cross strokes.
    fill_rect(img, 28.0, 16.0, 4.0, 30.0, SEA);
    fill_poly(img, &[[32.0, 18.0], [38.0, 26.0], [36.0, 29.0], [31.0, 22.0]], SEA);
    fill_poly(img, &[[32.0, 30.0], [38.0, 40.0], [36.0, 43.0], [31.0, 34.0]], SEA);
    // Weathering flecks + moss at the base.
    fill_rect(img, 24.0, 46.0, 3.0, 3.0, shade(stone, 0.7));
    fill_rect(img, 40.0, 24.0, 2.0, 2.0, shade(stone, 0.75));
    fill_rect(img, 16.0, 54.0, 10.0, 5.0, [48, 76, 37]);
    fill_rect(img, 38.0, 55.0, 9.0, 4.0, [48, 76, 37]);
    fill_rect(img, 17.0, 56.0, 3.0, 2.0, [60, 90, 45]);
}

/// The brood maw (E5): a gnawed pit with bone shards at the rim. Terrain
/// decal, not a figure — it lives on props.png per the pipeline doc.
fn prop_brood_hole(img: &mut RgbaImage) {
    let earth = [46, 36, 28];
    let maw = [12, 10, 8];
    let bone = [224, 218, 187];
    // Gnaw-dark halo, then the pit itself.
    fill_circle(img, 32.0, 36.0, 24.0, shade(earth, 0.7));
    fill_circle(img, 32.0, 36.0, 21.0, earth);
    fill_circle(img, 32.0, 37.0, 18.0, shade(earth, 0.8));
    fill_circle(img, 32.0, 38.0, 14.0, maw);
    // Lip highlight arc along the north rim (picks the hole out of turf).
    fill_circle(img, 32.0, 30.0, 14.0, shade(earth, 1.3));
    fill_circle(img, 32.0, 33.0, 14.0, shade(earth, 0.8));
    fill_circle(img, 32.0, 36.0, 12.0, maw);
    // Bone shards gnawed into the rim, angled inward.
    fill_poly(img, &[[17.0, 28.0], [21.0, 26.0], [20.0, 30.0]], bone);
    fill_poly(img, &[[43.0, 26.0], [47.0, 28.0], [44.0, 30.0]], bone);
    fill_poly(img, &[[49.0, 38.0], [47.0, 34.0], [45.0, 38.0]], shade(bone, 0.85));
    fill_poly(img, &[[15.0, 38.0], [19.0, 35.0], [17.0, 39.0]], shade(bone, 0.85));
    // Fallen debris.
    fill_rect(img, 24.0, 44.0, 2.0, 2.0, shade(earth, 1.2));
    fill_rect(img, 40.0, 45.0, 2.0, 2.0, shade(earth, 1.15));
}

fn prop_relic_pedestal(img: &mut RgbaImage) {
    let wall = [66, 66, 61];
    // Stepped plinth.
    fill_rect(img, 14.0, 48.0, 36.0, 10.0, shade(wall, 0.85));
    fill_rect(img, 18.0, 40.0, 28.0, 8.0, shade(wall, 1.05));
    fill_rect(img, 16.0, 38.0, 32.0, 3.0, shade(wall, 1.25));
    fill_rect(img, 22.0, 44.0, 20.0, 2.0, shade(wall, 0.7)); // front groove
    fill_rect(img, 30.0, 44.0, 4.0, 10.0, GOLD); // gold inlay
    // Felt cushion.
    fill_rect(img, 24.0, 34.0, 16.0, 5.0, [137, 83, 108]);
    fill_rect(img, 24.0, 34.0, 16.0, 2.0, [168, 102, 128]);
    // Boss relic orb: gold shell, sea core, white spec.
    fill_circle(img, 32.0, 24.0, 9.0, shade(GOLD, 0.8));
    fill_circle(img, 31.0, 23.0, 8.0, GOLD);
    fill_circle(img, 32.0, 24.0, 4.0, SEA);
    fill_circle(img, 28.0, 19.0, 2.0, [255, 255, 255]);
}

/// All prop plates, keyed (key, image).
pub fn prop_plates() -> Vec<(&'static str, RgbaImage)> {
    let seeds: Vec<(&'static str, RgbaImage)> = vec![
        ("TorchBrazier", {
            let mut i = blank(64, 64);
            prop_torch_brazier(&mut i);
            i
        }),
        ("Potion", {
            let mut i = blank(64, 64);
            prop_potion(&mut i);
            i
        }),
        ("RuneStone", {
            let mut i = blank(64, 64);
            prop_rune_stone(&mut i);
            i
        }),
        ("RelicPedestal", {
            let mut i = blank(64, 64);
            prop_relic_pedestal(&mut i);
            i
        }),
        ("BroodHole", {
            let mut i = blank(64, 64);
            prop_brood_hole(&mut i);
            i
        }),
    ];
    seeds
        .into_iter()
        .map(|(k, body)| (k, crate::actors::chunky(&body, 3)))
        .collect()
}
