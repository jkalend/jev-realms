//! Boss nameplate (§7.4-8, E4-s4): display serif plate over the boss while it
//! fights or is aggro'd (`view.boss_plate`), fading out when combat ends.
//! Serif = Cinzel if the plate exists (hud.rs DisplayFont), WEBFONT-DOWNGRADE
//! otherwise → bundled font + letterspacing. HP bar bands mirror sim bands.
//!
//! The brow is sized to its MEASURED content, never to a fixed panel: the
//! name text is measured through `TextLayoutInfo` every frame, the chrome trio
//! spans that width plus padding only, and a name too long for the cap is
//! shrunk (to `NAME_MIN`) and finally ellipsized, so it can never spill past
//! the plate. Height is a compact 19-unit rail — a chrome brow over the boss,
//! not a banner.

use bevy::prelude::*;
use bevy::text::{FontSize, LetterSpacing, TextLayoutInfo};
use laya_realms::model::Pos;

use crate::atlas::{self, Atlas};
use crate::hud::DisplayFont;
use crate::projection::{Proj, T};
use crate::sim::WorldView;

#[derive(Component)]
pub struct PlateMarker;
#[derive(Component)]
pub struct PlateText;
#[derive(Component)]
pub struct PlateBg;
#[derive(Component)]
pub struct PlateFill;

// --- plate metrics, world units (1 unit = 2.625 px at the shipped zoom) ------

/// Name face size; shrunk toward [`NAME_MIN`] when the name runs long.
const NAME_SIZE: f32 = 11.0;
/// Floor of the shrink-to-fit; below this the name is ellipsized instead.
const NAME_MIN: f32 = 8.5;
/// Widest name box allowed. Past this the plate stops growing — the text
/// shrinks (then truncates), the plate does not.
const MAX_NAME_W: f32 = 150.0;
const LETTER_PX: f32 = 1.1;
/// Air between the measured name and the brow's end caps.
const PAD: f32 = 11.0;
/// Breathing room between the name and the end-cap. The caps are drawn INSIDE
/// `box_w`, so the middle segment — the only part the name may occupy — is
/// `box_w - 2 * CAP_W`; sizing the box from `PAD` alone left that segment
/// narrower than the text and the caps clipped its first and last letters.
const PAD_INNER: f32 = 6.0;
/// Narrowest brow (a one-word name like "Chief").
const MIN_W: f32 = 74.0;
/// End-cap width: the bracket plates are never stretched.
const CAP_W: f32 = 13.0;
/// Brow height — the v1 trio was 32 units tall over a 13pt face, which is why
/// the text towered over its own frame.
const BROW_H: f32 = 19.0;
const BAR_H: f32 = 2.4;
/// HP band inset from the brow's ends, so the band clears both caps.
const BAR_INSET: f32 = 15.0;
/// Brow bottom, above the tile centre. A boss sprite is `T * 1.75` tall and
/// centred on its tile, so its crown sits at +28 units: the brow stays clear
/// of the sprite's full height (the v1 comment's intent) while sitting 27
/// units closer to it than the old `T * 2.5` anchor did.
const BROW_BASE: f32 = T * 1.15;
/// Chrome stratum. Everything in the world now shares ONE band per tile
/// (`Proj::tile_band` + `Z_*`, topping out around +2 on the shipped maps), and
/// the HUD/modal chrome sits at 100+/220+: the nameplate keeps its own band
/// well clear of the world, exactly as before the z rework.
const PLATE_Z: f32 = 23.0;

const BANDS: [(u8, u8, u8); 5] = [
    (124, 34, 30),   // band 0: nearly dead
    (168, 66, 42),   // 1
    (190, 128, 54),  // 2
    (136, 158, 102), // 3
    (86, 148, 106),  // 4: full
];

fn band_rgb(band: i32) -> Color {
    let (r, g, b) = BANDS[band.clamp(0, 4) as usize];
    Color::srgb_u8(r, g, b)
}

/// Shrink-to-fit / ellipsis state for one name string. Shrink-only (each step
/// multiplies the face size by ≤ 1) so the fit can never oscillate, and the
/// box always follows the measurement rather than the estimate.
///
/// `pub` because both plate systems take it as `Local<Fit>`, and this module is
/// `pub mod`, so a private type there is not a usable system signature.
#[derive(Default)]
pub struct Fit {
    /// Name this fit was computed for; a change restarts the fit.
    name: String,
    /// What the plate actually draws (may be a truncated name).
    display: String,
    size: f32,
    /// Brow width in world units, driven by the measured text width.
    box_w: f32,
}

impl Fit {
    fn restart(&mut self, name: &str) {
        self.name = name.to_string();
        self.display = name.to_string();
        self.size = NAME_SIZE;
        self.box_w = MIN_W;
    }

    /// Fold in this frame's measured text width (world units).
    fn refine(&mut self, measured: f32) {
        if measured <= 0.0 {
            return; // not laid out yet (first frame after spawn)
        }
        if measured > MAX_NAME_W {
            if self.size > NAME_MIN {
                self.size = (self.size * MAX_NAME_W / measured).max(NAME_MIN);
            } else {
                let keep = ((self.display.len() as f32) * MAX_NAME_W / measured).floor() as usize;
                let keep = keep.clamp(3, self.display.len().saturating_sub(1).max(3));
                if keep + 3 < self.display.len() {
                    self.display = self.display.chars().take(keep).collect::<String>() + "...";
                }
            }
        }
        // The box must contain the caps AND the name: the middle segment is
        // `box_w - 2 * CAP_W`, so the box is the text plus both caps plus the
        // inner margins. Following the raw measurement instead put the name
        // under the caps and clipped its first and last letters.
        self.box_w = (measured + 2.0 * (CAP_W + PAD_INNER)).max(MIN_W);
    }
}

/// Tap-inspect chip (E6): the pointer lane's `view.inspected` marker —
/// the nameplate lane's name-only surface for a tapped actor, positioned
/// exactly like the boss plate so the two read as one chrome family.
#[derive(Component)]
pub struct InspectMarker;
#[derive(Component)]
pub struct InspectText;
#[derive(Component)]
pub struct InspectBg;

/// Chip metrics: the same fit rule as the plate, a shorter rail.
const CHIP_H: f32 = 13.0;
const CHIP_BASE: f32 = T * 1.25;
const CHIP_SIZE: f32 = 10.5;
const CHIP_MIN: f32 = 8.0;
const CHIP_MAX_W: f32 = 150.0;

pub fn inspect(
    view: Res<WorldView>,
    proj: Res<Proj>,
    font: Option<Res<DisplayFont>>,
    mut commands: Commands,
    mut plates: Query<
        (
            Entity,
            Option<&InspectText>,
            &mut Transform,
        ),
        With<InspectMarker>,
    >,
    mut text: Query<(&mut Text2d, &mut TextColor, &mut TextFont, &TextLayoutInfo), With<InspectText>>,
    mut bg: Query<&mut Sprite, With<InspectBg>>,
    mut fit: Local<Fit>,
) {
    let name = view.inspected.and_then(|pos| {
        view.actors
            .iter()
            .find(|a| a.pos == pos)
            .map(|a| (a.name.clone(), pos))
    });
    let Some((name, pos)) = name else {
        for (entity, ..) in plates.iter_mut() {
            commands.entity(entity).despawn();
        }
        return;
    };
    if fit.name != name {
        fit.restart(&name);
    }
    // Measured width of the name currently drawn (0 until the layout lands).
    let measured = text
        .iter()
        .next()
        .map(|(_, _, _, info)| info.size.x)
        .unwrap_or_else(|| name.len() as f32 * CHIP_SIZE * 0.62);
    if measured > CHIP_MAX_W {
        if fit.size > CHIP_MIN {
            fit.size = (fit.size * CHIP_MAX_W / measured).max(CHIP_MIN);
        }
    }
    let box_w = (measured + PAD * 2.0).max(MIN_W);

    let top = proj.world(pos, 0.0).truncate() + Vec2::new(0.0, CHIP_BASE);
    if plates.is_empty() {
        let serif = font.as_deref().and_then(|f| f.0.clone());
        commands.spawn((
            Text2d::new(fit.display.clone()),
            TextFont {
                font: serif
                    .map(bevy::text::FontSource::Handle)
                    .unwrap_or_default(),
                ..TextFont::from_font_size(CHIP_SIZE)
            },
            TextLayout::justify(Justify::Center),
            TextColor(Color::srgb_u8(215, 205, 180)),
            LetterSpacing::Px(1.2),
            Transform::from_translation(top.extend(PLATE_Z + 1.0)),
            InspectMarker,
            InspectText,
        ));
        commands.spawn((
            Sprite::from_color(Color::srgba(0.04, 0.05, 0.07, 0.75), Vec2::new(box_w, CHIP_H)),
            Transform::from_translation((top + Vec2::new(0.0, -1.0)).extend(PLATE_Z)),
            InspectMarker,
            InspectBg,
        ));
    }
    for (entity, label, mut transform) in plates.iter_mut() {
        if label.is_some() {
            transform.translation = top.extend(PLATE_Z + 1.0);
            if let Ok((mut t, mut color, mut tf, _)) = text.get_mut(entity) {
                if t.0 != fit.display {
                    t.0 = fit.display.clone();
                }
                tf.font_size = FontSize::Px(fit.size);
                color.0 = Color::srgb_u8(215, 205, 180);
            }
        } else {
            transform.translation = (top + Vec2::new(0.0, -1.0)).extend(PLATE_Z);
        }
    }
    if let Ok(mut sprite) = bg.single_mut() {
        sprite.custom_size = Some(Vec2::new(box_w, CHIP_H));
    }
}

#[derive(Clone)]
struct PlateData {
    name: String,
    hp: i32,
    max_hp: i32,
    band: i32,
    pos: Pos,
}

#[derive(Default)]
pub struct PlateState {
    last: Option<PlateData>,
    fade: f32,
    fit: Fit,
}

/// Arched chrome brow: the BossNameplate trio (L/end bracket, M/stretched
/// span, R) lands over the text+bar once the chrome sheet resolves.
#[derive(Component)]
pub struct TrioPart(usize); // 0 L, 1 M, 2 R

pub fn render(
    view: Res<WorldView>,
    time: Res<Time>,
    proj: Res<Proj>,
    font: Option<Res<DisplayFont>>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    mut commands: Commands,
    mut set: ParamSet<(
        Query<
            (
                Entity,
                Option<&PlateText>,
                Option<&PlateBg>,
                Option<&PlateFill>,
                &mut Visibility,
                &mut Transform,
            ),
            With<PlateMarker>,
        >,
        Query<(&mut Text2d, &mut TextColor, &mut TextFont, &TextLayoutInfo), With<PlateText>>,
        Query<&mut Sprite, With<PlateFill>>,
        Query<(&TrioPart, &mut Transform, &mut Visibility, &mut Sprite), Without<PlateMarker>>,
    )>,
    mut state: Local<PlateState>,
    mut trio_ready: Local<bool>,
) {
    let _ = &mut commands;
    // Data path: boss in combat/aggro this frame, else the last-known plate
    // fading out post-combat.
    let current = view.boss_plate.as_ref().map(|p| PlateData {
        name: p.name.clone(),
        hp: p.hp,
        max_hp: p.max_hp,
        band: p.band,
        pos: p.pos,
    });
    let (data, alpha) = match current {
        Some(data) => {
            state.last = Some(PlateData { ..data.clone() });
            state.fade = 1.0;
            (data, 1.0)
        }
        None => {
            let Some(last) = state.last.clone() else {
                if !set.p0().is_empty() {
                    for (entity, ..) in set.p0().iter_mut() {
                        commands.entity(entity).despawn();
                    }
                    for (_, _, mut visibility, _) in set.p3().iter_mut() {
                        *visibility = Visibility::Hidden;
                    }
                }
                return;
            };
            state.fade -= time.delta_secs() * 1.4;
            if state.fade <= 0.0 {
                state.last = None;
                for (entity, ..) in set.p0().iter_mut() {
                    commands.entity(entity).despawn();
                }
                for (_, _, mut visibility, _) in set.p3().iter_mut() {
                    *visibility = Visibility::Hidden;
                }
                return;
            }
            (last, state.fade.max(0.0))
        }
    };

    // --- content-sized geometry --------------------------------------------
    // Fit the name against what was last rendered, then size every box to the
    // measurement. Nothing here is a fixed panel size.
    if state.fit.name != data.name {
        state.fit.restart(&data.name);
    }
    let measured = set
        .p1()
        .iter()
        .next()
        .map(|(_, _, _, info)| info.size.x)
        .unwrap_or(0.0);
    state.fit.refine(measured);
    let box_w = state.fit.box_w;
    let cap_w = CAP_W.min(box_w * 0.25);
    let mid_w = (box_w - cap_w * 2.0).max(8.0);
    let bar_w = (box_w - BAR_INSET * 2.0).max(8.0);

    // Brow bottom sits above the boss sprite's full height; the name row rides
    // the upper half of the rail and the HP band the lower half.
    let base = proj.world(data.pos, 0.0).truncate() + Vec2::new(0.0, BROW_BASE);
    let center = base + Vec2::new(0.0, BROW_H * 0.5);
    let bar_y = center.y - BROW_H * 0.27;

    // The chrome brow trio spawns once the BossNameplate plates resolve.
    if !*trio_ready
        && images.as_deref().is_some_and(|imgs| {
            atlas.as_deref().is_some_and(|a| {
                ["BossNameplateL", "BossNameplateM", "BossNameplateR"]
                    .iter()
                    .all(|k| atlas::chrome_ref(a, imgs, k).is_some())
            })
        })
    {
        *trio_ready = true;
        for (index, key) in ["BossNameplateL", "BossNameplateM", "BossNameplateR"]
            .iter()
            .enumerate()
        {
            if let Some((image, rect)) = images.as_deref().and_then(|i| {
                atlas
                    .as_deref()
                    .and_then(|a| atlas::chrome_ref(a, i, key))
            }) {
                commands.spawn((
                    Sprite {
                        image: image.clone(),
                        rect: Some(rect),
                        custom_size: Some(Vec2::new(cap_w, BROW_H)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, -100_000.0, PLATE_Z),
                    Visibility::Hidden,
                    TrioPart(index),
                ));
            }
        }
    }
    if set.p0().is_empty() {
        let serif = font.as_deref().and_then(|f| f.0.clone());
        commands.spawn((
            Text2d::new(state.fit.display.clone()),
            TextFont {
                font: serif
                    .map(bevy::text::FontSource::Handle)
                    .unwrap_or_default(),
                ..TextFont::from_font_size(state.fit.size)
            },
            TextLayout::justify(Justify::Center),
            TextColor(Color::srgba_u8(235, 222, 190, (alpha * 255.0) as u8)),
            LetterSpacing::Px(LETTER_PX),
            Transform::from_translation(
                Vec3::new(center.x, center.y + BROW_H * 0.19, PLATE_Z + 1.0),
            ),
            PlateMarker,
            PlateText,
        ));
        commands.spawn((
            Sprite::from_color(Color::srgba(0.04, 0.05, 0.07, 0.85), Vec2::new(bar_w, BAR_H)),
            Transform::from_translation(
                Vec3::new(center.x, bar_y, PLATE_Z + 0.5),
            ),
            PlateMarker,
            PlateBg,
        ));
        commands.spawn((
            Sprite::from_color(band_rgb(data.band), Vec2::new(bar_w, BAR_H)),
            Transform::from_translation(
                Vec3::new(center.x, bar_y, PLATE_Z + 0.6),
            ),
            PlateMarker,
            PlateFill,
        ));
    }
    let frac = (data.hp as f32 / data.max_hp.max(1) as f32).clamp(0.0, 1.0);
    for (_, label, _bg, fill, mut visibility, mut transform) in set.p0().iter_mut() {
        *visibility = if alpha > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if label.is_some() {
            transform.translation =
                Vec3::new(center.x, center.y + BROW_H * 0.19, PLATE_Z + 1.0);
        } else if fill.is_some() {
            let x = center.x + (frac - 1.0) * bar_w * 0.5;
            transform.translation = Vec3::new(x, bar_y, PLATE_Z + 0.6);
        } else {
            transform.translation = Vec3::new(center.x, bar_y, PLATE_Z + 0.5);
        }
    }
    for (mut t, mut color, mut tf, _) in set.p1().iter_mut() {
        if t.0 != state.fit.display {
            t.0 = state.fit.display.clone();
        }
        tf.font_size = FontSize::Px(state.fit.size);
        let base = Color::srgb_u8(235, 222, 190);
        color.0 = base.with_alpha(alpha);
    }
    for mut sprite in set.p2().iter_mut() {
        sprite.custom_size = Some(Vec2::new(bar_w * frac, BAR_H));
        let mut bar = band_rgb(data.band);
        bar.set_alpha(alpha);
        sprite.color = bar;
    }
    // Chrome brow trio: L=end bracket left, M=stretched span, R=right. Sized to
    // the measured name every frame, so the plate can never out-run its text.
    for (part, mut transform, mut visibility, mut sprite) in set.p3().iter_mut() {
        let (size, x) = match part.0 {
            0 => (
                Vec2::new(cap_w, BROW_H),
                center.x - (box_w * 0.5 - cap_w * 0.5),
            ),
            1 => (Vec2::new(mid_w, BROW_H), center.x),
            _ => (
                Vec2::new(cap_w, BROW_H),
                center.x + (box_w * 0.5 - cap_w * 0.5),
            ),
        };
        sprite.custom_size = Some(size);
        sprite.color = Color::WHITE.with_alpha(alpha);
        transform.translation = Vec3::new(x, center.y, PLATE_Z);
        *visibility = if alpha > 0.0 && *trio_ready {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
