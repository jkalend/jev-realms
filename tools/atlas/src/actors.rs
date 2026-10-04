//! Actor plates — sprites::actor() transplanted to the 64px bake grid.
//!
//! Every paint() call below is a verbatim port of src/sprites.rs::actor()
//! (32 authored units, scaled x2 into the 64x64 cell). The C+ hybrid finish
//! comes from gfxlab::draw_actor_styled (Arm::Chunky with hybrid lift):
//! a near-black INK silhouette pass offset +-3px on both axes, then the
//! figure with the accent lifted x1.22 (the "+20% read lift" vs the grim
//! environment plates). Ground shadow and the allegiance mark are drawn
//! outside the silhouette so outlines hug the figure, not the feet.

use crate::raster::*;
use image::RgbaImage;
use laya_realms::model::{Archetype, Class};

/// Chunky-arm ink (sprites::INK) used for the offset outline pass.
const INKC: Rgb = [9, 11, 12];
const GOLD: Rgb = [232, 191, 89];
const SEA: Rgb = [102, 212, 196];
#[allow(dead_code)]
const RED: Rgb = [245, 89, 92];
const WHITE: Rgb = [255, 255, 255];

/// The hybrid brightness lift (gfxlab 1.22).
const LIFT: f32 = 1.22;

pub fn class_hue(class: Class) -> Rgb {
    match class {
        Class::Keepwarden => [232, 181, 88],  // amber gold
        Class::Gravebound => [233, 228, 210], // ivory
        Class::Redwake => [216, 110, 70],     // rust
        Class::Waysworn => [170, 160, 90],    // olive
        Class::SigilSworn => [152, 225, 222], // ice cyan
        Class::Fensworn => [170, 105, 186],   // viridian violet
        Class::None => [235, 200, 90],        // unsworn: default hero gold
    }
}

/// Canonical per-archetype accent (plate-batch choice; drives cloth/fold
/// masses and the allegiance mark). Hostiles keep the gfxlab red-family
/// read; civvies get distinct but muted service hues.
pub fn accent(kind: Archetype) -> Rgb {
    match kind {
        Archetype::Guard => [135, 161, 240], // production blue (sprites.rs)
        Archetype::Bandit | Archetype::Wolf | Archetype::Bear | Archetype::Rat
        | Archetype::Skeleton | Archetype::Chief => [200, 90, 80],
        Archetype::Matriarch => [180, 90, 120],
        Archetype::Lich => [120, 90, 150], // the Curate's violet
        Archetype::Adjudicator => [220, 183, 105], // the Judge's gold
        Archetype::Oracle => [102, 205, 195], // rune cyan (SEA family)
        Archetype::Tidemother => [70, 130, 140], // drowned deep-water teal
        Archetype::Cragmother => [140, 100, 70], // ridge-stone ochre
        Archetype::Vendor => [200, 150, 70],
        Archetype::Thief => [120, 120, 100],
        Archetype::Traveller => [170, 160, 90],
        Archetype::Smuggler => [160, 140, 110],
        Archetype::Companion => [159, 216, 228], // sellsword ice steel
        Archetype::Commoner => [190, 190, 170],
        // E5 (core lane, §6; silhouette plates are ART-TODO — hues land now).
        Archetype::GnawThane => [150, 110, 90], // fen-splashed rat royalty
        Archetype::Tollmaster => [190, 140, 60], // toll-iron and coin
        Archetype::Mirelight => [150, 226, 214], // drowned wisplight
        Archetype::PaleStag => [230, 225, 210], // ghost-white hart
        Archetype::Alchemist => [170, 130, 190], // the still's violet
        Archetype::BroodHole => [46, 36, 28], // gnawed dark
        Archetype::FalseGlow => [190, 240, 235], // borrowed light
        // E7 (§6.5): the covenant violet, same line as the schism it broke.
        Archetype::OathlessCurate => [120, 90, 150],
    }
}

pub enum ActorFigure {
    Npc(Archetype),
    /// Archetype under a manifest states-variant pose ("PaleStag.at_bay").
    NpcState(Archetype, &'static str),
    /// Player hero, mid gear (weapon 1 / armour 1), cape + mark in class hue.
    Player { class_hue: Rgb },
}

fn gear_color(tier: u8) -> Rgb {
    match tier {
        0 => [145, 102, 67],
        1 => [158, 175, 185],
        2 => [159, 216, 228],
        _ => GOLD,
    }
}

/// 32-unit painter surface (paint coordinates identical to sprites.rs).
struct P<'a> {
    img: &'a mut RgbaImage,
}

impl P<'_> {
    fn paint(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgb) {
        fill_rect(self.img, x * 2.0, y * 2.0, w * 2.0, h * 2.0, c);
    }
}

/// Paint the actor's silhouette layer (no ground shadow, no mark).
fn figure_body(img: &mut RgbaImage, accent: Rgb, figure: &ActorFigure) {
    let accent = shade(accent, LIFT);
    let skin = [225, 179, 132];
    let skin_shadow = [166, 115, 86];
    let bone = [224, 218, 187];
    let steel = gear_color(1);
    let leather = gear_color(0);
    let dark = [31, 35, 43];
    let cloth = shade(accent, 0.48);
    let fold = shade(accent, 0.32);
    let mut p = P { img };
    let (kind, is_player, state) = match figure {
        ActorFigure::Npc(kind) => (*kind, false, None),
        ActorFigure::NpcState(kind, s) => (*kind, false, Some(*s)),
        ActorFigure::Player { .. } => (Archetype::Companion, true, None),
    };
    match kind {
        Archetype::PaleStag => {
            // Ghost hart, custom branch: the shared quadruped hips can't read
            // a stag (slim barrel, long legs, antler crown). at_bay lifts the
            // whole head/neck/antler group 3px — the states proof-case.
            let bayed = state == Some("at_bay");
            let hy = if bayed { 3.0 } else { 0.0 };
            let body = [221, 216, 199];
            let shade_fur = [186, 178, 160];
            let bone = [224, 218, 187];
            // Four slim legs; far pair darker. Hooves near-black.
            for (lx, far) in [(9.0, true), (13.0, false), (19.0, true), (23.0, false)] {
                let c = if far { shade_fur } else { body };
                p.paint(lx, 20.0, 2.0, 8.0, c);
                p.paint(lx, 28.0, 2.0, 2.0, INKC);
            }
            // Slim barrel + belly tuft, tail tuft at the rump.
            p.paint(7.0, 13.0, 16.0, 9.0, shade_fur);
            p.paint(8.0, 12.0, 15.0, 9.0, body);
            p.paint(9.0, 12.5, 8.0, 2.0, WHITE);
            p.paint(4.0, 14.0, 3.0, 4.0, shade_fur);
            // Rising neck + doe-clean head (whole group shifts under at_bay).
            p.paint(20.0, 8.0 - hy, 4.0, 6.0, shade_fur);
            p.paint(21.0, 4.5 - hy, 6.0, 5.0, body);
            p.paint(26.0, 6.5 - hy, 3.0, 2.0, shade_fur);
            p.paint(28.0, 7.0 - hy, 2.0, 2.0, INKC);
            p.paint(23.0, 6.0 - hy, 1.5, 1.5, INKC);
            // Antler crown v2: long forking beams (v1 read goat-small at
            // first review) — taller stems, three tines per side, brighter
            // bone so the crown pops against the ivory mantle.
            for (ax, tines) in [
                (21.0, [(19.0, 0.5), (21.5, 0.0), (22.0, 1.0)]),
                (25.0, [(27.0, 0.5), (24.0, 0.0), (24.5, 1.5)]),
            ] {
                p.paint(ax, 0.0 - hy, 1.5, 5.5, shade(bone, 1.12));
                for (tx, ty) in tines {
                    p.paint(tx, ty * 2.0 - hy, 2.0, 2.0, shade(bone, 1.12));
                }
            }
            // Ghost mantle sparkles along the back + white ruff.
            p.paint(9.0, 13.0, 2.0, 1.5, WHITE);
            p.paint(13.0, 12.0, 2.0, 1.5, WHITE);
            p.paint(20.0, 10.5 - hy, 3.0, 2.0, WHITE);
        }
        Archetype::Rat | Archetype::Matriarch | Archetype::Wolf | Archetype::Bear
        | Archetype::Cragmother | Archetype::GnawThane => {
            let (body, back, belly, head) = match kind {
                Archetype::Rat => ([148, 128, 112], 19.0, 26.0, 18.0),
                Archetype::Matriarch => ([137, 83, 108], 11.0, 25.0, 13.0),
                Archetype::Wolf => ([142, 156, 165], 16.0, 24.0, 11.0),
                // The Thane: a longer, darker dire rat under his shard crown.
                Archetype::GnawThane => ([120, 100, 84], 12.0, 26.0, 14.0),
                // Plate-batch decision: the Cragmother is a quadruped ridge
                // bear (sprites.rs leaves her a generic humanoid; §6 calls
                // her an ancient ridge bear, so the plate gives her a real
                // boss read — longer, lower, stone-ridged).
                Archetype::Cragmother => ([100, 88, 72], 7.0, 26.0, 9.0),
                _ => ([119, 78, 49], 9.0, 25.0, 10.0),
            };
            let body = shade(body, LIFT);
            let fur = shade(body, 0.66);
            let ridge = shade(body, 1.18);
            let rat = matches!(kind, Archetype::Rat | Archetype::Matriarch | Archetype::GnawThane);
            let stride = 0.0;
            p.paint(10.0, belly - 2.0 - stride, 3.0, 5.0, fur);
            p.paint(21.0, belly - 2.0 + stride, 3.0, 5.0, fur);
            p.paint(6.0, back + 2.0, 19.0, belly - back - 1.0, dark);
            p.paint(7.0, back, 15.0, belly - back - 1.0, body);
            p.paint(9.0, back - 1.0, 9.0, 3.0, body);
            p.paint(8.0, back + 1.0, 9.0, 2.0, ridge);
            p.paint(7.0, belly - 4.0, 13.0, 3.0, fur);
            p.paint(7.0, belly - 4.0 + stride, 5.0, 4.0, body);
            p.paint(8.0, belly + stride, 3.0, 3.0, fur);
            p.paint(8.0, belly + 2.0 + stride, 5.0, 1.0, if rat { skin } else { fur });
            p.paint(20.0, belly - 5.0 - stride, 4.0, 6.0, fur);
            p.paint(21.0, belly + 1.0 - stride, 5.0, 2.0, if rat { skin } else { body });
            // Neck, cheek plane, brow, muzzle, dark nose.
            p.paint(19.0, head + 3.0, 7.0, belly - head - 4.0, fur);
            p.paint(21.0, head, 7.0, 8.0, body);
            p.paint(22.0, head, 5.0, 2.0, ridge);
            p.paint(24.0, head + 5.0, 6.0, 3.0, if rat { skin_shadow } else { ridge });
            p.paint(29.0, head + 5.0, 2.0, 2.0, INKC);
            p.paint(25.0, head + 3.0, 2.0, 2.0, INKC); // idle: un-angered eye
            if rat {
                p.paint(2.0, 25.0, 5.0, 2.0, skin_shadow);
                p.paint(1.0, 22.0, 2.0, 4.0, skin_shadow);
                p.paint(2.0, 21.0, 3.0, 2.0, skin);
                p.paint(20.0, head - 3.0, 5.0, 5.0, fur);
                p.paint(21.0, head - 2.0, 3.0, 3.0, skin_shadow);
                p.paint(22.0, head - 2.0, 2.0, 2.0, skin);
                p.paint(27.0, head + 8.0, 2.0, 2.0, bone);
                p.paint(25.0, head + 9.0, 5.0, 1.0, fur);
                p.paint(11.0, back + 4.0, 4.0, 2.0, fur);
                if kind == Archetype::Matriarch {
                    // Broken bone spines and ragged mane: the brood boss.
                    p.paint(8.0, 8.0, 3.0, 5.0, bone);
                    p.paint(13.0, 5.0, 3.0, 7.0, bone);
                    p.paint(18.0, 8.0, 2.0, 5.0, bone);
                    p.paint(8.0, 12.0, 13.0, 3.0, fur);
                    p.paint(10.0, 15.0, 3.0, 4.0, ridge);
                    p.paint(15.0, 14.0, 3.0, 3.0, ridge);
                    p.paint(28.0, 21.0, 2.0, 4.0, bone);
                }
                if kind == Archetype::GnawThane {
                    // Shard crown over the skull, twin whisker whips, mange
                    // mantle; red eyes bake on the idle frame (he is the
                    // warren's wrath, not the clone of the Matriarch).
                    p.paint(20.0, head - 3.0, 2.0, 3.0, bone);
                    p.paint(23.0, head - 4.0, 2.0, 4.0, bone);
                    p.paint(26.0, head - 3.0, 2.0, 3.0, bone);
                    p.paint(19.0, head - 1.0, 2.0, 2.0, shade(bone, 0.75));
                    p.paint(27.0, head - 1.0, 2.0, 2.0, shade(bone, 0.75));
                    p.paint(24.0, head + 4.0, 3.0, 1.0, [180, 158, 140]);
                    p.paint(24.0, head + 6.0, 3.0, 1.0, [180, 158, 140]);
                    p.paint(7.0, back + 2.0, 14.0, 3.0, shade(body, 0.72));
                    p.paint(25.0, head + 2.0, 2.0, 2.0, [200, 60, 50]);
                }
            } else if kind == Archetype::Wolf {
                p.paint(2.0, 14.0, 6.0, 4.0, fur);
                p.paint(1.0, 12.0, 3.0, 3.0, body);
                p.paint(20.0, 6.0, 3.0, 7.0, fur);
                p.paint(21.0, 8.0, 2.0, 3.0, skin_shadow);
                p.paint(25.0, 8.0, 2.0, 4.0, fur);
                p.paint(19.0, 18.0, 5.0, 3.0, ridge);
                p.paint(20.0, 21.0, 3.0, 3.0, ridge);
                p.paint(27.0, 18.0, 2.0, 2.0, bone);
                p.paint(11.0, 18.0, 8.0, 2.0, fur);
            } else {
                p.paint(8.0, 8.0, 10.0, 5.0, body);
                p.paint(9.0, 9.0, 6.0, 2.0, ridge);
                p.paint(20.0, 7.0, 4.0, 4.0, fur);
                p.paint(25.0, 7.0, 4.0, 4.0, fur);
                p.paint(21.0, 8.0, 2.0, 2.0, leather);
                p.paint(26.0, 8.0, 2.0, 2.0, leather);
                // Bear cheek stays fur-toned (saddle leather read as a box
                // on the head at 64px in the first contact-sheet pass).
                p.paint(24.0, 15.0, 5.0, 4.0, shade(body, 1.14));
                p.paint(29.0, 15.0, 2.0, 2.0, INKC);
                p.paint(17.0, 15.0, 3.0, 7.0, fur);
                p.paint(10.0, 20.0, 4.0, 2.0, ridge);
                p.paint(9.0, 28.0 + stride, 4.0, 1.0, bone);
                p.paint(22.0, 27.0 - stride, 4.0, 1.0, bone);
                if kind == Archetype::Cragmother {
                    // Stone ridge plates along the spine, heavy cheek ruff:
                    // the boss reads granite-backed, the cub reads brown.
                    let stone = [113, 114, 105];
                    for i in 0..4u64 {
                        let x = 7.0 + i as f32 * 3.5;
                        let lift_up = (i % 2) as f32;
                        p.paint(x, back - 2.0 - lift_up, 3.0, 2.0, shade(stone, 0.9));
                        p.paint(x, back - 3.0 - lift_up, 2.0, 1.0, shade(stone, 1.2));
                    }
                    p.paint(27.0, head + 1.0, 3.0, 4.0, shade(body, 0.75));
                    p.paint(28.0, head + 3.0, 1.0, 1.0, INKC);
                    p.paint(6.0, belly + 1.0, 4.0, 1.0, stone);
                    p.paint(24.0, belly + 1.0, 4.0, 1.0, stone);
                }
            }
        }
        _ => {
            let undead = matches!(kind, Archetype::Skeleton | Archetype::Lich);
            let face = if undead { bone } else { skin };
            let face_shade = if undead { shade(bone, 0.68) } else { skin_shadow };
            // Plate-batch decision: Tidemother gets the gown treatment (no
            // visible legs) so the drowned matriarch reads as one wet mass,
            // not a villager with kelp patches (sprites' detail set kept).
            // The Curate robes violet; the Tollmaster broads with his brood.
            let robe = matches!(
                kind,
                Archetype::Oracle
                    | Archetype::Lich
                    | Archetype::Adjudicator
                    | Archetype::Tidemother
                    | Archetype::OathlessCurate
            );
            let broad = matches!(kind, Archetype::Chief | Archetype::Adjudicator | Archetype::Tollmaster);
            // Spectral wisps: legs dissolve into a tapering tail.
            let spectral = matches!(kind, Archetype::Mirelight | Archetype::FalseGlow);
            // Cape behind the body: SEA for the unsworn hero, class hue for
            // classed players, sellsword accent for the Companion NPC.
            if kind == Archetype::Companion {
                let cape = match figure {
                    ActorFigure::Player { class_hue } => shade(*class_hue, LIFT),
                    _ => accent,
                };
                p.paint(5.0, 10.0, 7.0, 15.0, shade(cape, 0.60));
                p.paint(4.0, 21.0, 7.0, 6.0, shade(cape, 0.60));
                p.paint(6.0, 12.0, 3.0, 13.0, cape);
                p.paint(4.0, 26.0, 4.0, 2.0, cape);
            }
            let stride = 0.0;
            if spectral {
                // Wisp tail: fading rungs of glow-mass into transparency.
                let glow = accent;
                blend_rect(p.img, 20.0, 44.0, 24.0, 6.0, shade(glow, 0.30), 150);
                blend_rect(p.img, 24.0, 50.0, 16.0, 5.0, shade(glow, 0.26), 100);
                blend_rect(p.img, 28.0, 55.0, 8.0, 4.0, shade(glow, 0.22), 60);
            } else {
                for (x, step) in [(10.0, stride), (18.0, -stride)] {
                    p.paint(x, 22.0 + step, 4.0, 5.0, fold);
                    p.paint(x, 25.0 + step, 4.0, 3.0, leather);
                    p.paint(x, 28.0 + step, 6.0, 2.0, dark);
                    p.paint(x, 25.0 + step, 4.0, 1.0, shade(leather, 1.2));
                }
            }
            p.paint(8.0, 12.0, 16.0, 12.0, dark);
            p.paint(10.0, 12.0, 12.0, 11.0, cloth);
            p.paint(10.0, 14.0, 3.0, 8.0, shade(accent, 0.75));
            p.paint(18.0, 15.0, 3.0, 7.0, fold);
            p.paint(7.0, 14.0, 3.0, 5.0, cloth);
            p.paint(7.0, 19.0, 3.0, 3.0, fold);
            p.paint(8.0, 21.0, 3.0, 3.0, face_shade);
            p.paint(22.0, 14.0, 3.0, 5.0, cloth);
            p.paint(23.0, 19.0, 3.0, 4.0, face_shade);
            p.paint(24.0, 20.0, 2.0, 2.0, face);
            p.paint(10.0, 22.0, 12.0, 2.0, leather);
            p.paint(16.0, 22.0, 3.0, 2.0, GOLD);
            // Hair/hood outline, shaded cheek, ear, nose, brow.
            p.paint(11.0, 3.0, 12.0, 10.0, dark);
            p.paint(13.0, 4.0, 8.0, 8.0, face_shade);
            p.paint(15.0, 4.0, 7.0, 6.0, face);
            p.paint(12.0, 7.0, 2.0, 3.0, face);
            p.paint(22.0, 8.0, 2.0, 2.0, face);
            p.paint(19.0, 6.0, 3.0, 1.0, face_shade);
            p.paint(20.0, 7.0, 2.0, 2.0, INKC);
            p.paint(18.0, 11.0, 4.0, 1.0, face_shade);
            if robe {
                p.paint(9.0, 20.0, 15.0, 7.0, cloth);
                p.paint(7.0, 27.0, 19.0, 2.0, dark);
                p.paint(10.0, 20.0, 2.0, 8.0, shade(accent, 0.75));
                p.paint(15.0, 22.0, 2.0, 6.0, fold);
                p.paint(21.0, 20.0, 2.0, 8.0, fold);
                p.paint(9.0, 27.0, 15.0, 1.0, leather);
            }
            if broad {
                p.paint(4.0, 12.0, 23.0, 5.0, shade(steel, 0.7));
                p.paint(5.0, 12.0, 7.0, 2.0, steel);
                p.paint(20.0, 12.0, 6.0, 2.0, steel);
                p.paint(10.0, 15.0, 13.0, 8.0, shade(steel, 0.65));
                p.paint(12.0, 16.0, 4.0, 6.0, steel);
                p.paint(17.0, 17.0, 4.0, 2.0, steel);
            }
            match kind {
                Archetype::Commoner => {
                    p.paint(9.0, 4.0, 15.0, 3.0, leather);
                    p.paint(13.0, 1.0, 8.0, 4.0, leather);
                    p.paint(13.0, 3.0, 8.0, 2.0, shade(leather, 0.65));
                    p.paint(13.0, 14.0, 8.0, 9.0, [168, 152, 119]);
                    p.paint(14.0, 14.0, 2.0, 8.0, bone);
                    p.paint(16.0, 19.0, 4.0, 2.0, shade(leather, 0.8));
                    p.paint(25.0, 17.0, 2.0, 12.0, leather);
                    p.paint(24.0, 15.0, 4.0, 3.0, steel);
                }
                Archetype::Vendor => {
                    p.paint(2.0, 13.0, 7.0, 13.0, shade(leather, 0.65));
                    p.paint(2.0, 13.0, 6.0, 3.0, leather);
                    p.paint(4.0, 14.0, 2.0, 10.0, GOLD);
                    p.paint(9.0, 4.0, 15.0, 3.0, GOLD);
                    p.paint(13.0, 1.0, 8.0, 4.0, cloth);
                    p.paint(14.0, 1.0, 6.0, 1.0, accent);
                    p.paint(14.0, 14.0, 8.0, 10.0, bone);
                    p.paint(18.0, 14.0, 2.0, 10.0, shade(bone, 0.75));
                    p.paint(15.0, 19.0, 6.0, 4.0, leather);
                    p.paint(17.0, 19.0, 2.0, 2.0, GOLD);
                }
                Archetype::Traveller => {
                    p.paint(2.0, 10.0, 7.0, 15.0, shade(leather, 0.7));
                    p.paint(1.0, 9.0, 8.0, 4.0, bone);
                    p.paint(2.0, 10.0, 6.0, 1.0, shade(bone, 0.7));
                    p.paint(4.0, 13.0, 2.0, 11.0, leather);
                    p.paint(9.0, 4.0, 15.0, 2.0, leather);
                    p.paint(13.0, 1.0, 8.0, 4.0, leather);
                    p.paint(14.0, 3.0, 7.0, 1.0, GOLD);
                    p.paint(12.0, 13.0, 2.0, 9.0, leather);
                    p.paint(14.0, 21.0, 7.0, 4.0, leather);
                    p.paint(27.0, 9.0, 2.0, 21.0, leather);
                    p.paint(27.0, 9.0, 3.0, 2.0, bone);
                }
                Archetype::Thief | Archetype::Bandit | Archetype::Smuggler => {
                    p.paint(10.0, 2.0, 13.0, 4.0, cloth);
                    p.paint(9.0, 5.0, 4.0, 9.0, cloth);
                    p.paint(11.0, 3.0, 8.0, 2.0, shade(accent, 0.65));
                    p.paint(14.0, 10.0, 9.0, 3.0, dark);
                    p.paint(13.0, 14.0, 3.0, 4.0, leather);
                    p.paint(16.0, 17.0, 3.0, 4.0, leather);
                    if kind == Archetype::Bandit {
                        p.paint(9.0, 14.0, 13.0, 3.0, leather);
                        p.paint(10.0, 14.0, 3.0, 1.0, steel);
                        p.paint(19.0, 14.0, 2.0, 1.0, steel);
                        p.paint(27.0, 11.0, 3.0, 12.0, steel);
                        p.paint(28.0, 10.0, 2.0, 3.0, bone);
                        p.paint(26.0, 22.0, 6.0, 2.0, leather);
                        p.paint(28.0, 24.0, 2.0, 4.0, leather);
                    } else if kind == Archetype::Smuggler {
                        p.paint(1.0, 18.0, 8.0, 9.0, leather);
                        p.paint(2.0, 16.0, 6.0, 3.0, shade(leather, 1.2));
                        p.paint(4.0, 17.0, 2.0, 10.0, bone);
                        p.paint(1.0, 23.0, 8.0, 2.0, shade(leather, 0.65));
                        p.paint(19.0, 23.0, 4.0, 4.0, leather);
                    } else {
                        p.paint(7.0, 14.0, 3.0, 12.0, dark);
                        p.paint(25.0, 20.0, 5.0, 2.0, steel);
                        p.paint(29.0, 19.0, 2.0, 2.0, bone);
                        p.paint(24.0, 19.0, 2.0, 4.0, leather);
                        p.paint(13.0, 23.0, 3.0, 4.0, dark);
                    }
                }
                Archetype::Guard => {
                    p.paint(11.0, 1.0, 11.0, 5.0, steel);
                    p.paint(13.0, 1.0, 3.0, 4.0, bone);
                    p.paint(10.0, 5.0, 14.0, 2.0, shade(steel, 0.7));
                    p.paint(11.0, 7.0, 3.0, 5.0, steel);
                    p.paint(12.0, 14.0, 10.0, 7.0, steel);
                    p.paint(13.0, 15.0, 3.0, 5.0, shade(steel, 1.2));
                    p.paint(3.0, 16.0, 9.0, 10.0, steel);
                    p.paint(5.0, 17.0, 5.0, 7.0, cloth);
                    p.paint(7.0, 17.0, 2.0, 7.0, accent);
                    p.paint(5.0, 24.0, 5.0, 3.0, steel);
                    p.paint(28.0, 7.0, 2.0, 23.0, leather);
                    p.paint(27.0, 3.0, 4.0, 5.0, steel);
                    p.paint(28.0, 1.0, 2.0, 6.0, bone);
                }
                Archetype::Skeleton => {
                    p.paint(11.0, 3.0, 12.0, 8.0, bone);
                    p.paint(11.0, 4.0, 2.0, 6.0, shade(bone, 0.65));
                    p.paint(13.0, 6.0, 3.0, 3.0, INKC);
                    p.paint(19.0, 6.0, 3.0, 3.0, INKC);
                    p.paint(17.0, 9.0, 2.0, 2.0, INKC);
                    p.paint(14.0, 11.0, 7.0, 2.0, bone);
                    p.paint(16.0, 11.0, 1.0, 2.0, INKC);
                    p.paint(19.0, 11.0, 1.0, 2.0, INKC);
                    p.paint(10.0, 14.0, 12.0, 9.0, dark);
                    for y in [14.0, 17.0, 20.0] {
                        p.paint(12.0, y, 8.0, 2.0, bone);
                        p.paint(11.0, y + 1.0, 2.0, 1.0, shade(bone, 0.65));
                    }
                    p.paint(16.0, 14.0, 2.0, 9.0, bone);
                    p.paint(12.0, 23.0, 9.0, 2.0, bone);
                    p.paint(7.0, 15.0, 2.0, 5.0, bone);
                    p.paint(8.0, 20.0, 2.0, 4.0, bone);
                    p.paint(23.0, 15.0, 2.0, 5.0, bone);
                    p.paint(11.0, 25.0 + stride, 2.0, 4.0, bone);
                    p.paint(19.0, 25.0 - stride, 2.0, 4.0, bone);
                }
                Archetype::Chief => {
                    p.paint(7.0, 1.0, 3.0, 7.0, bone);
                    p.paint(5.0, 0.0, 3.0, 3.0, bone);
                    p.paint(24.0, 1.0, 3.0, 7.0, bone);
                    p.paint(26.0, 0.0, 3.0, 3.0, bone);
                    p.paint(10.0, 3.0, 14.0, 3.0, steel);
                    p.paint(14.0, 3.0, 5.0, 2.0, GOLD);
                    p.paint(13.0, 10.0, 10.0, 5.0, leather);
                    p.paint(15.0, 12.0, 6.0, 4.0, shade(leather, 0.7));
                    p.paint(5.0, 14.0, 5.0, 3.0, bone);
                    p.paint(7.0, 16.0, 3.0, 3.0, bone);
                    p.paint(11.0, 22.0, 12.0, 3.0, leather);
                    p.paint(16.0, 22.0, 4.0, 3.0, GOLD);
                    p.paint(27.0, 10.0, 2.0, 20.0, leather);
                    p.paint(25.0, 8.0, 7.0, 7.0, steel);
                    p.paint(30.0, 7.0, 2.0, 9.0, bone);
                    p.paint(25.0, 9.0, 2.0, 4.0, shade(steel, 0.65));
                }
                Archetype::Oracle | Archetype::Lich => {
                    p.paint(10.0, 2.0, 13.0, 3.0, cloth);
                    p.paint(9.0, 4.0, 4.0, 10.0, cloth);
                    p.paint(10.0, 5.0, 2.0, 7.0, shade(accent, 0.7));
                    p.paint(27.0, 9.0, 2.0, 21.0, leather);
                    p.paint(25.0, 5.0, 6.0, 6.0, GOLD);
                    p.paint(27.0, 6.0, 2.0, 4.0, if kind == Archetype::Lich { SEA } else { bone });
                    p.paint(12.0, 15.0, 2.0, 11.0, shade(accent, 0.8));
                    p.paint(18.0, 16.0, 2.0, 10.0, shade(accent, 0.7));
                    if kind == Archetype::Lich {
                        for x in [11.0, 16.0, 21.0] {
                            p.paint(x, 0.0, 2.0, 5.0, GOLD);
                        }
                        p.paint(11.0, 4.0, 12.0, 2.0, GOLD);
                        p.paint(14.0, 7.0, 3.0, 2.0, INKC);
                        p.paint(19.0, 7.0, 3.0, 2.0, INKC);
                        p.paint(15.0, 7.0, 2.0, 2.0, SEA);
                        p.paint(20.0, 7.0, 2.0, 2.0, SEA);
                        p.paint(15.0, 11.0, 6.0, 2.0, dark);
                        p.paint(7.0, 13.0, 5.0, 3.0, bone);
                        p.paint(21.0, 13.0, 4.0, 3.0, bone);
                        p.paint(15.0, 18.0, 3.0, 4.0, GOLD);
                        p.paint(8.0, 28.0, 3.0, 2.0, cloth);
                        p.paint(20.0, 28.0, 4.0, 2.0, cloth);
                    } else {
                        p.paint(13.0, 7.0, 10.0, 2.0, GOLD);
                        p.paint(14.0, 8.0, 7.0, 1.0, shade(GOLD, 0.7));
                        p.paint(14.0, 14.0, 6.0, 2.0, bone);
                        p.paint(3.0, 18.0, 7.0, 6.0, leather);
                        p.paint(4.0, 18.0, 5.0, 4.0, bone);
                        p.paint(6.0, 18.0, 1.0, 4.0, shade(leather, 0.6));
                    }
                }
                Archetype::Adjudicator => {
                    p.paint(10.0, 1.0, 14.0, 11.0, shade(GOLD, 0.65));
                    p.paint(11.0, 2.0, 11.0, 8.0, GOLD);
                    p.paint(13.0, 2.0, 3.0, 4.0, bone);
                    p.paint(13.0, 7.0, 9.0, 2.0, INKC);
                    p.paint(17.0, 9.0, 3.0, 4.0, GOLD);
                    p.paint(18.0, 9.0, 1.0, 3.0, bone);
                    p.paint(5.0, 12.0, 7.0, 2.0, GOLD);
                    p.paint(21.0, 12.0, 6.0, 2.0, GOLD);
                    p.paint(13.0, 16.0, 8.0, 2.0, GOLD);
                    p.paint(16.0, 15.0, 2.0, 8.0, GOLD);
                    p.paint(10.0, 25.0, 13.0, 2.0, GOLD);
                    p.paint(27.0, 13.0, 2.0, 17.0, leather);
                    p.paint(23.0, 8.0, 9.0, 6.0, shade(GOLD, 0.65));
                    p.paint(23.0, 8.0, 9.0, 2.0, GOLD);
                    p.paint(24.0, 10.0, 2.0, 3.0, bone);
                    p.paint(30.0, 10.0, 2.0, 3.0, GOLD);
                }
                Archetype::Companion => {
                    p.paint(11.0, 3.0, 11.0, 3.0, leather);
                    p.paint(12.0, 3.0, 5.0, 1.0, shade(leather, 1.25));
                    if !is_player {
                        p.paint(13.0, 14.0, 8.0, 7.0, steel);
                        p.paint(14.0, 15.0, 2.0, 5.0, bone);
                        p.paint(18.0, 19.0, 3.0, 2.0, shade(steel, 0.65));
                        p.paint(27.0, 15.0, 2.0, 10.0, steel);
                        p.paint(26.0, 24.0, 5.0, 2.0, leather);
                    }
                }
                Archetype::Tidemother => {
                    let kelp = [93, 122, 89];
                    p.paint(11.0, 2.0, 11.0, 3.0, kelp);
                    p.paint(13.0, 1.0, 2.0, 2.0, WHITE);
                    p.paint(18.0, 1.0, 2.0, 2.0, WHITE);
                    // Drowned read v2: sea-glass pupils over ink sockets (the
                    // first pass's lone grey eye was unreadable at 64px),
                    // wet sheen streaks down the gown.
                    p.paint(13.0, 6.0, 3.0, 3.0, INKC);
                    p.paint(19.0, 6.0, 3.0, 3.0, INKC);
                    p.paint(14.0, 6.0, 1.0, 2.0, SEA);
                    p.paint(20.0, 6.0, 1.0, 2.0, SEA);
                    p.paint(8.0, 12.0, 4.0, 8.0, kelp);
                    p.paint(20.0, 12.0, 4.0, 8.0, kelp);
                    p.paint(9.0, 19.0, 14.0, 3.0, [70, 96, 84]);
                    blend_rect(p.img, 24.0, 24.0, 2.0, 12.0, [116, 143, 166], 130);
                    blend_rect(p.img, 17.0, 40.0, 2.0, 14.0, [116, 143, 166], 110);
                    // Trawl-net hem and wet hem-glow (gown continues the read).
                    p.paint(10.0, 27.0, 13.0, 2.0, [70, 96, 84]);
                    p.paint(12.0, 28.0, 3.0, 2.0, [70, 130, 140]);
                    p.paint(19.0, 28.0, 3.0, 2.0, [70, 130, 140]);
                }
                Archetype::Rat | Archetype::Matriarch | Archetype::Wolf | Archetype::Bear
                | Archetype::Cragmother => {}
                // Batch 3 (E5 lanes): beasts patched to their own painter
                // branches above; gown proxies removed (their details kept
                // only where the humanoid is the actual body).
                Archetype::GnawThane | Archetype::PaleStag => {}
                Archetype::Tollmaster => {
                    // Coin sash, twin toll-coins, the flat warlord's cap and
                    // the toll-hook polearm (proxy details kept; the body is
                    // broad-armoured like the Chief from the shared pass).
                    p.paint(10.0, 13.0, 12.0, 2.0, GOLD);
                    p.paint(14.0, 13.0, 1.0, 1.0, WHITE);
                    p.paint(17.0, 13.0, 1.0, 1.0, WHITE);
                    p.paint(11.0, 2.0, 12.0, 4.0, leather);
                    p.paint(9.0, 4.0, 16.0, 2.0, shade(leather, 0.7));
                    p.paint(27.0, 8.0, 2.0, 17.0, steel);
                    p.paint(27.0, 8.0, 5.0, 2.0, steel);
                    p.paint(30.0, 8.0, 2.0, 4.0, INKC);
                }
                Archetype::Mirelight | Archetype::FalseGlow => {}
                Archetype::Alchemist => {
                    // Stained apron and a belt of vials.
                    p.paint(11.0, 14.0, 10.0, 12.0, leather);
                    p.paint(11.0, 14.0, 10.0, 1.0, shade(leather, 0.65));
                    p.paint(12.0, 14.0, 2.0, 3.0, [120, 170, 110]);
                    p.paint(15.0, 14.0, 2.0, 3.0, [116, 143, 166]);
                    p.paint(18.0, 14.0, 2.0, 3.0, [180, 140, 90]);
                }
                // BroodHole ships as a PROP plate (the maw is terrain, not a
                // figure) — no actor detail here; see props.rs.
                Archetype::BroodHole => {}
                // E7 (§6.5): the torn violet mitre and the page he never
                // stops folding; the robe pass gowns him like his broken order.
                Archetype::OathlessCurate => {
                    let violet = [120, 90, 150];
                    p.paint(11.0, 1.0, 12.0, 4.0, violet);
                    p.paint(13.0, 0.0, 8.0, 3.0, shade(violet, 0.6));
                    p.paint(14.0, 14.0, 6.0, 10.0, bone);
                    p.paint(15.0, 15.0, 4.0, 1.0, [12, 10, 8]);
                    p.paint(15.0, 18.0, 4.0, 1.0, [12, 10, 8]);
                    p.paint(15.0, 21.0, 4.0, 1.0, [12, 10, 8]);
                    p.paint(9.0, 12.0, 4.0, 8.0, violet);
                    p.paint(21.0, 12.0, 4.0, 8.0, violet);
                }
            }
            if spectral {
                // Veil overlay after the shared body: hood drapes swallow the
                // head sides so the hollow glow-face sits in a cavity, ink
                // sockets, drowned waist band, drift motes.
                let glow = accent;
                p.paint(11.0, 1.0, 12.0, 5.0, cloth);
                p.paint(10.0, 4.0, 4.0, 9.0, cloth);
                p.paint(21.0, 4.0, 4.0, 9.0, cloth);
                p.paint(11.0, 8.0, 2.0, 6.0, shade(accent, 0.62));
                p.paint(23.0, 8.0, 2.0, 6.0, shade(accent, 0.62));
                p.paint(13.0, 6.0, 6.0, 5.0, shade(glow, 0.5));
                if kind == Archetype::Mirelight {
                    p.paint(15.0, 7.0, 2.0, 2.0, INKC);
                    p.paint(19.0, 7.0, 2.0, 2.0, INKC);
                }
                p.paint(3.0, 12.0, 2.0, 2.0, glow);
                p.paint(28.0, 9.0, 2.0, 2.0, glow);
                if kind == Archetype::Mirelight {
                    p.paint(24.0, 24.0, 2.0, 2.0, glow);
                    p.paint(6.0, 24.0, 2.0, 2.0, glow);
                }
                p.paint(9.0, 19.0, 14.0, 3.0, shade(glow, 0.35));
            }
            if let ActorFigure::Player { .. } = figure {
                let (weapon, armour) = (1u8, 1u8);
                let plate = gear_color(armour);
                let plate_shadow = shade(plate, 0.65);
                p.paint(11.0, 13.0, 12.0, 10.0, plate_shadow);
                p.paint(12.0, 14.0, 5.0, 7.0, plate);
                p.paint(18.0, 14.0, 4.0, 6.0, plate);
                p.paint(12.0, 14.0, 2.0, 5.0, shade(plate, 1.2));
                p.paint(12.0, 21.0, 10.0, 2.0, leather);
                p.paint(16.0, 21.0, 3.0, 2.0, GOLD);
                if armour == 0 {
                    p.paint(13.0, 13.0, 2.0, 4.0, shade(leather, 0.6));
                    p.paint(19.0, 13.0, 2.0, 4.0, shade(leather, 0.6));
                    p.paint(15.0, 17.0, 5.0, 2.0, shade(leather, 0.6));
                } else {
                    p.paint(8.0, 12.0, 6.0, 4.0, plate_shadow);
                    p.paint(20.0, 12.0, 6.0, 4.0, plate_shadow);
                    p.paint(9.0, 12.0, 5.0, 2.0, plate);
                    p.paint(20.0, 12.0, 5.0, 2.0, plate);
                    p.paint(12.0, 23.0, 4.0, 2.0, plate);
                    p.paint(19.0, 23.0, 4.0, 2.0, plate);
                    p.paint(23.0, 18.0, 3.0, 2.0, plate);
                }
                if armour >= 2 {
                    p.paint(11.0, 1.0, 12.0, 5.0, plate_shadow);
                    p.paint(12.0, 1.0, 9.0, 3.0, plate);
                    p.paint(14.0, 1.0, 2.0, 3.0, bone);
                    p.paint(11.0, 6.0, 3.0, 5.0, plate);
                    p.paint(21.0, 5.0, 3.0, 2.0, plate);
                }
                if armour >= 3 {
                    p.paint(14.0, 0.0, 5.0, 2.0, SEA);
                    p.paint(16.0, 15.0, 3.0, 4.0, SEA);
                    p.paint(9.0, 14.0, 4.0, 1.0, bone);
                    p.paint(21.0, 14.0, 4.0, 1.0, bone);
                }
                let metal = gear_color(weapon);
                let tip = 14.0 - f32::from(weapon.min(3)) * 2.0;
                p.paint(27.0, tip + 2.0, 3.0, 22.0 - tip, metal);
                p.paint(28.0, tip, 2.0, 4.0, metal);
                p.paint(27.0, tip + 3.0, 1.0, 19.0 - tip, if weapon == 0 { steel } else { bone });
                p.paint(26.0, 23.0, 6.0, 2.0, metal);
                p.paint(28.0, 25.0, 2.0, 4.0, leather);
                p.paint(28.0, 29.0, 2.0, 1.0, metal);
                if weapon >= 2 {
                    p.paint(28.0, 16.0, 2.0, 3.0, SEA);
                    p.paint(26.0, 22.0, 2.0, 2.0, metal);
                    p.paint(30.0, 22.0, 2.0, 2.0, metal);
                }
                if weapon >= 3 {
                    p.paint(28.0, 10.0, 2.0, 2.0, bone);
                    p.paint(29.0, 25.0, 1.0, 2.0, GOLD);
                }
                // Wide gold feet (no relic on the idle plate).
                p.paint(9.0, 30.0, 15.0, 2.0, GOLD);
            }
        }
    }
}

/// Full 64x64 actor plate with shadow, class mark, ink outline, lift.
pub fn actor_plate(accent: Rgb, figure: &ActorFigure) -> RgbaImage {
    let mut out = blank(64, 64);
    // Ground shadow — outside the silhouette so the outline hugs the figure.
    blend_rect(&mut out, 8.0, 54.0, 48.0, 6.0, [0, 0, 0], 77);
    // Allegiance/class mark at the feet (accent for NPCs, class hue for players).
    let mark = match figure {
        ActorFigure::Player { class_hue } => shade(*class_hue, LIFT),
        _ => accent,
    };
    fill_rect(&mut out, 26.0, 60.0, 12.0, 4.0, mark);
    let mut body = blank(64, 64);
    figure_body(&mut body, accent, figure);
    let figures = chunky(&body, 3);
    blit_blend(&mut out, &figures, 0, 0);
    // Re-stamp the mark over the outline so it reads clean.
    fill_rect(&mut out, 26.0, 60.0, 12.0, 4.0, mark);
    out
}

/// The chunky-arm finish: near-black silhouette offset on both axes, then
/// the body composited over it. Shared by actors and props.
pub fn chunky(body: &RgbaImage, off: i32) -> RgbaImage {
    let mut ink = blank(64, 64);
    for (x, y, p) in body.enumerate_pixels() {
        if p[3] > 0 {
            ink.put_pixel(x, y, image::Rgba([INKC[0], INKC[1], INKC[2], 255]));
        }
    }
    let mut out = blank(64, 64);
    for (dx, dy) in [(-off, 0), (off, 0), (0, -off), (0, off)] {
        blit_blend(&mut out, &ink, dx, dy);
    }
    blit_blend(&mut out, body, 0, 0);
    out
}

pub fn blit_blend(dst: &mut RgbaImage, src: &RgbaImage, ox: i32, oy: i32) {
    let (dw, dh) = (dst.width() as i32, dst.height() as i32);
    for (x, y, p) in src.enumerate_pixels() {
        let a = p[3] as u32;
        if a == 0 {
            continue;
        }
        let (tx, ty) = (ox + x as i32, oy + y as i32);
        if tx < 0 || ty < 0 || tx >= dw || ty >= dh {
            continue;
        }
        dst.put_pixel(tx as u32, ty as u32, *p);
    }
}
