//! Camera-anchored, compact stone-and-brass HUD. The bottom rail holds live
//! health, mana, stamina, class, sigils and XP; the small upper plate shows
//! only facts from the current world snapshot.
//!
//! Rendering: world space, camera-anchored. (A second order-1 camera with
//! clear=None blacked the world pass on this stack — the post-chain +
//! multi-target interaction was not worth debugging over the simpler single
//! camera.) Everything sits at `CamFrame.pos + px · CamFrame.scale` with
//! transform scale = scale, so chrome is zoom-invariant. Side effect accepted
//! and documented: bevy_light_2d and the post stack also grade the chrome
//! (night HUD reads slightly dimmer — dark-room atmosphere).

use bevy::prelude::*;
use bevy::text::LetterSpacing;
use crossterm::event::{KeyCode as CrossKey, KeyEvent, KeyModifiers};
use laya_realms::model::{Game, Pos, Tile};

use crate::atlas::{self, Atlas};
use crate::camera::CamFrame;
use crate::palette;
use crate::sim::{MeterHue, WorldView};

#[derive(Component)]
pub struct OrbRim(pub OrbKind);
/// Chrome frame plate replacing the flat glass cap when the chrome sheet
/// exists (OrbFrameRed/Blue over the rim+fill composite).
#[derive(Component)]
pub struct OrbFrame(pub OrbKind);
/// Octagonal housing plate replacing the flat pip disc when chrome exists.
#[derive(Component)]
pub struct PipHousing(usize);
/// Recessed rail plate replacing the flat XP track when chrome exists.
#[derive(Component)]
pub struct XpRail;
/// Flat-drawing fallback pieces hid one-way once a chrome plate resolves.
#[derive(Component)]
pub struct FlatFallback;
#[derive(Component)]
pub struct HpFill;
#[derive(Component)]
pub struct ManaFill;
#[derive(Component)]
pub struct StaminaBg;
#[derive(Component)]
pub struct StaminaFill;
#[derive(Component)]
pub struct XpBg;
#[derive(Component)]
pub struct XpFill;
#[derive(Component)]
pub struct SigilPip(usize);
#[derive(Component)]
pub struct ChipBg;
#[derive(Component)]
pub struct MeterText;
/// Rail / top-plate pieces, laid out in logical pixels each frame.
#[derive(Component)]
pub struct HudPlate(usize);
/// 0 = top readout, 1 = HP, 2 = mana, 3 = stamina, 4 = XP,
/// 5 = gear caption, 6 = weapon tier, 7 = armour tier, 8 = sigil caption.
#[derive(Component)]
pub struct HudValue(usize);
/// Gear strip on the rail's left flank: `slot` true = the housing plate,
/// false = the item pictogram seated in it.
#[derive(Component)]
pub struct GearPart {
    pub index: usize,
    pub slot: bool,
}
/// Combat bubble: four thin frame-edge strips pulsing red while in_combat
/// (replaces the colored-Vignette attempt — its threshold color veiled the
/// whole frame on this backend).
#[derive(Component)]
pub struct CombatEdge(usize); // 0 N, 1 S, 2 W, 3 E

/// The five carved menu keys are deliberately inside the rail's hitbox.
#[derive(Component)]
pub struct MenuKey(pub usize);
#[derive(Component)]
pub struct MenuCaption(pub usize);
#[derive(Component)]
pub struct MapCell(pub usize);
#[derive(Component)]
pub struct MapFrame(pub usize);
#[derive(Component)]
pub struct MapCaption;

const MENU: [(&str, CrossKey); 5] = [
    ("MAP  M", CrossKey::Char('m')),
    ("BAG  I", CrossKey::Char('i')),
    ("TALENTS  T", CrossKey::Char('t')),
    ("SETTINGS  ESC", CrossKey::Esc),
    ("QUESTS  B", CrossKey::Char('b')),
];
const MENU_X: [f32; 5] = [-128.0, -64.0, 0.0, 64.0, 128.0];
const MAP_COLS: usize = 15;
const MAP_ROWS: usize = 11;

/// Screen-space hit test shared with the pointer lane; the entire carved rail
/// blocks world movement, not only the visible key surfaces.
pub fn hit_test(screen: Vec2, w: f32, h: f32) -> Option<Option<usize>> {
    let ui = hud_scale(w, h);
    let x = (screen.x - w * 0.5) / ui;
    let y = (h - screen.y - 8.0) / ui;
    if x.abs() > 383.0 || !(4.0..=108.0).contains(&y) {
        return None;
    }
    Some(MENU_X.iter().position(|cx| (x - cx).abs() <= 30.0 && (29.0..=51.0).contains(&y)))
}

/// Route exactly the same keys as the keyboard lane through the simulation's
/// input handler (which initializes selection and enforces modal semantics).
pub fn open_button(game: &mut Game, index: usize) {
    if let Some((_, code)) = MENU.get(index) {
        laya_realms::input::key(game, KeyEvent::new(*code, KeyModifiers::NONE));
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum OrbKind {
    Hp,
    Mana,
}

/// Display font (§7.4-8): Cinzel if the OFL plate exists on disk, else the
/// bundled default — the downgrade is reconciled at load time (info-logged).
#[derive(Resource, Default)]
pub struct DisplayFont(pub Option<Handle<Font>>);

pub const FONT_PATH: &str = "assets/fonts/Cinzel-Regular.ttf";

const Z: f32 = 100.0;
/// Keep the action visible even on narrow windows; the entire rail scales
/// together, rather than clipping its orbs or letting them cover the center.
pub(crate) fn hud_scale(w: f32, h: f32) -> f32 {
    ((w - 16.0) / 800.0).min((h - 16.0) / 610.0).clamp(0.1, 1.0)
}

fn bottom(h: f32, ui: f32, x: f32, y: f32) -> Vec2 {
    Vec2::new(x * ui, -h * 0.5 + 8.0 + y * ui)
}


pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let font = if std::path::Path::new(FONT_PATH).exists() {
        Some(asset_server.load("fonts/Cinzel-Regular.ttf"))
    } else {
        // WEBFONT-DOWNGRADE: serif plate absent → bundled default + letterspacing.
        info!("hud: {FONT_PATH} absent — bundled default font + letterspacing");
        None
    };
    commands.insert_resource(DisplayFont(font.clone()));

    let mut disc = |radius: f32, color: Color, material_set: &mut Assets<ColorMaterial>| {
        (
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(material_set.add(color)),
        )
    };
    let rim = Color::srgb_u8(56, 52, 48); // dark stone: reads on the black void
    let glass = Color::srgba(0.04, 0.05, 0.09, 0.55);
    for (kind, color) in [
        (OrbKind::Hp, Color::srgb_u8(180, 58, 64)),
        (OrbKind::Mana, Color::srgb_u8(82, 128, 196)),
    ] {
        let (m1, c1) = disc(34.0, rim, &mut materials);
        let (m2, c2) = disc(30.0, color, &mut materials);
        let (m3, c3) = disc(30.0, glass, &mut materials);
        commands.spawn((m1, c1, Transform::from_xyz(0.0, 0.0, Z), OrbRim(kind)));
        commands.spawn((
            m3,
            c3,
            Transform::from_xyz(0.0, 0.0, Z + 2.0),
            OrbRim(kind),
            FlatFallback,
        ));
        let mut entity = commands.spawn((m2, c2, Transform::from_xyz(0.0, 0.0, Z + 2.5)));
        match kind {
            OrbKind::Hp => entity.insert(HpFill),
            OrbKind::Mana => entity.insert(ManaFill),
        };
    }
    let quad = |w: f32, h: f32, color: Color, z: f32| {
        (
            Sprite::from_color(color, Vec2::new(w, h)),
            Transform::from_xyz(0.0, 0.0, z),
        )
    };
    // A hairline inset for stamina, subordinate to the two full-size orbs.
    commands.spawn((quad(96.0, 4.0, Color::srgba_u8(8, 17, 17, 255), Z + 0.4), StaminaBg));
    commands.spawn((quad(96.0, 2.0, Color::srgb_u8(102, 173, 129), Z + 1.0), StaminaFill));
    for i in 0..3usize {
        let (m, c) = disc(5.0, Color::srgba_u8(220, 183, 105, 60), &mut materials);
        commands.spawn((m, c, Transform::from_xyz(0.0, 0.0, Z), SigilPip(i), FlatFallback));
    }
    commands.spawn((
        quad(320.0, 6.0, Color::srgba_u8(18, 20, 23, 245), Z + 0.4),
        XpBg,
        FlatFallback,
    ));
    commands.spawn((
        quad(320.0, 3.0, palette::srgb8(palette::INK_LIGHT_YELLOW), Z + 1.0),
        XpFill,
    ));
    commands.spawn((quad(206.0, 22.0, Color::srgba_u8(14, 18, 20, 247), Z + 1.0), ChipBg));
    // Grounded rather than screen-wide: a dark inset, brass perimeter and
    // a restrained top status plate.
    for (index, width, height, color, z) in [
        (0, 766.0, 104.0, Color::srgba_u8(9, 12, 15, 242), Z - 3.0),
        (1, 754.0, 92.0, Color::srgba_u8(36, 34, 31, 245), Z - 2.8),
        (2, 740.0, 78.0, Color::srgba_u8(13, 17, 19, 250), Z - 2.6),
        // Stepped stone crown: a shaded recess, not a bright unbroken top lip.
        (3, 716.0, 4.0, Color::srgba_u8(48, 42, 34, 245), Z - 2.4),
        (4, 752.0, 2.0, Color::srgba_u8(102, 79, 51, 220), Z - 2.4),
        (5, 326.0, 68.0, Color::srgba_u8(23, 27, 27, 247), Z - 2.2),
        (6, 206.0, 1.0, Color::srgba_u8(117, 92, 55, 140), Z - 2.0),
    ] {
        commands.spawn((quad(width, height, color, z), HudPlate(index)));
    }
    // Short facets break the upper crown into carved blocks; no full-width
    // glowing stripe across the gameplay frame.
    for (index, _) in [-355.0, -286.0, -161.0, 161.0, 286.0, 355.0].into_iter().enumerate() {
        commands.spawn((
            quad(12.0, 3.0, Color::srgba_u8(112, 87, 53, 165), Z - 2.3),
            HudPlate(9 + index),
        ));
    }
    // Local wayfinder: 15×11 sampled explored cells, not a fabricated map.
    for (index, size, color) in [
        (0, Vec2::new(112.0, 78.0), Color::srgba_u8(92, 70, 43, 245)),
        (1, Vec2::new(108.0, 74.0), Color::srgba_u8(8, 12, 14, 250)),
    ] {
        commands.spawn((Sprite::from_color(color, size), Transform::from_xyz(0.0, 0.0, Z + index as f32 * 0.1), MapFrame(index)));
    }
    for index in 0..MAP_COLS * MAP_ROWS {
        commands.spawn((
            Sprite::from_color(Color::srgba_u8(21, 26, 27, 255), Vec2::splat(4.5)),
            Transform::from_xyz(0.0, 0.0, Z + 0.3),
            MapCell(index),
        ));
    }
    commands.spawn((
        Text2d::new("WAYFINDER"),
        TextFont {
            font: font.clone().map(bevy::text::FontSource::Handle).unwrap_or_default(),
            ..TextFont::from_font_size(8.0)
        },
        TextColor(Color::srgb_u8(201, 174, 115)),
        LetterSpacing::Px(0.8),
        Transform::from_xyz(0.0, 0.0, Z + 1.5),
        MapCaption,
    ));
    for (index, (caption, _)) in MENU.iter().enumerate() {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(75, 61, 40), Vec2::new(61.0, 24.0)),
            Transform::from_xyz(0.0, 0.0, Z + 1.1),
            MenuKey(index),
        ));
        commands.spawn((
            Text2d::new(*caption),
            TextFont {
                font: font.clone().map(bevy::text::FontSource::Handle).unwrap_or_default(),
                ..TextFont::from_font_size(8.5)
            },
            TextColor(Color::srgb_u8(231, 208, 158)),
            LetterSpacing::Px(0.2),
            Transform::from_xyz(0.0, 0.0, Z + 2.0),
            MenuCaption(index),
        ));
    }
    // Combat bubble strips; sized in the update pass, alpha-normalized always.
    for i in 0..4usize {
        let (r, g, b) = palette::INK_RED;
        commands.spawn((
            Sprite::from_color(Color::srgba_u8(r, g, b, 0), Vec2::new(1.0, 1.0)),
            Transform::from_xyz(0.0, -100_000.0, Z + 3.0),
            CombatEdge(i),
        ));
    }
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: font.clone().map(bevy::text::FontSource::Handle).unwrap_or_default(),
            ..TextFont::from_font_size(12.0)
        },
        TextLayout::justify(Justify::Center),
        TextColor(Color::srgb_u8(235, 222, 190)),
        LetterSpacing::Px(1.2),
        Transform::from_xyz(0.0, 0.0, Z + 3.0),
        MeterText,
        Visibility::Hidden,
    ));
    for (index, size, color) in [
        (0, 11.0, Color::srgb_u8(225, 210, 173)),
        (1, 11.0, Color::srgb_u8(248, 218, 203)),
        (2, 11.0, Color::srgb_u8(203, 223, 252)),
        (3, 10.0, Color::srgb_u8(171, 205, 164)),
        (4, 10.0, Color::srgb_u8(213, 193, 139)),
        // Sigils moved out of their own top-rail plate into the bar's right
        // flank, which was the only dead span left in the chrome.
        (5, 10.0, Color::srgb_u8(225, 210, 173)),
    ] {
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font: font.clone().map(bevy::text::FontSource::Handle).unwrap_or_default(),
                ..TextFont::from_font_size(size)
            },
            TextLayout::justify(Justify::Center),
            TextColor(color),
            LetterSpacing::Px(0.7),
            Transform::from_xyz(0.0, 0.0, Z + 3.2),
            HudValue(index),
        ));
    }
}

/// Meter hue → Ink triples (src/gui.rs color() values; custom RGB hues minted
/// identically to ui.rs `class_meter` colors).
fn hue_rgb(hue: MeterHue) -> (u8, u8, u8) {
    match hue {
        MeterHue::LightYellow => (249, 215, 128),
        MeterHue::Ivory225 => (240, 235, 215),
        MeterHue::RustOrange => (220, 120, 60),
        MeterHue::Olive => (175, 160, 95),
        MeterHue::LightCyan => (152, 225, 222),
        MeterHue::Violet => (170, 120, 220),
    }
}

fn ratio(x: i32, m: i32) -> f32 {
    if m > 0 {
        (x as f32 / m as f32).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Logical-pixel point (center origin, y up) → world translation at this frame.
fn at(frame: &CamFrame, px: Vec2, z: f32) -> Vec3 {
    (frame.pos + px * frame.scale).extend(z)
}

/// Combat bubble: red frame-edge strips pulsing with the fight (§7.4-3 DD
/// transfer; placeholder chrome until the baked frame lands with D30).
pub fn combat_bubble(
    view: Res<WorldView>,
    frame: Res<CamFrame>,
    time: Res<Time>,
    windows: Query<&Window, ()>,
    mut strips: Query<(&CombatEdge, &mut Transform, &mut Sprite)>,
) {
    let Ok(window) = windows.single() else { return };
    let s = frame.scale.max(0.0001);
    let (w, h) = (window.width(), window.height());
    let t = 6.0f32; // strip thickness in px
    let alpha = if view.in_combat {
        0.30 + 0.22 * (0.5 + 0.5 * (time.elapsed_secs() * 4.0).sin())
    } else {
        0.0
    };
    for (edge, mut transform, mut sprite) in strips.iter_mut() {
        let (pos, size) = match edge.0 {
            0 => (Vec2::new(0.0, h * 0.5 - t * 0.5), Vec2::new(w, t)),
            1 => (Vec2::new(0.0, -h * 0.5 + t * 0.5), Vec2::new(w, t)),
            2 => (Vec2::new(-w * 0.5 + t * 0.5, 0.0), Vec2::new(t, h)),
            _ => (Vec2::new(w * 0.5 - t * 0.5, 0.0), Vec2::new(t, h)),
        };
        transform.translation = at(&frame, pos, Z + 1.0);
        transform.scale = Vec3::new(s, s, 1.0);
        sprite.custom_size = Some(size);
        sprite.color.set_alpha(alpha);
    }
}

/// Gauges: orb rims, orb fill discs, sigil pips. Chrome variants
/// (OrbFrame plates, octagonal housings, XPSliver rail) are spawned lazily
/// upon first decoherent notify; flat-drawing fallbacks stay when absent.
pub fn update_fills(
    mut commands: Commands,
    view: Res<WorldView>,
    frame: Res<CamFrame>,
    windows: Query<&Window, ()>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    mut set: ParamSet<(
        Query<(&OrbRim, &mut Transform, Option<&FlatFallback>)>,
        Query<&mut Transform, With<HpFill>>,
        Query<&mut Transform, With<ManaFill>>,
        Query<(&SigilPip, &mut Transform, &MeshMaterial2d<ColorMaterial>)>,
        Query<(&OrbFrame, &mut Transform), Without<OrbRim>>,
        Query<(&PipHousing, &mut Transform, &mut Sprite)>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut chrome_ready: Local<bool>,
) {
    let Ok(window) = windows.single() else { return };
    let (w, h) = (window.width(), window.height());
    let ui = hud_scale(w, h);
    let s = frame.scale.max(0.0001) * ui;
    let hp_c = bottom(h, ui, -330.0, 55.0);
    let mana_c = bottom(h, ui, 330.0, 55.0);
    let hp_frac = ratio(view.player_hp, view.player_max_hp);
    let mana_frac = ratio(view.player_mana, view.player_max_mana);
    let fill_y = |c: Vec2, frac: f32| c.y - 30.0 * ui + 30.0 * ui * frac;

    // Lazy chrome spawn: plate keys resolve once the sheet finishes decoding.
    if !*chrome_ready {
        let have_all = images.as_deref().is_some_and(|imgs| {
            atlas.as_deref().is_some_and(|a| {
                ["OrbFrameRed", "OrbFrameBlue", "SigilPip"]
                    .iter()
                    .all(|k| atlas::chrome_ref(a, imgs, k).is_some())
            })
        });
        if !have_all {
            // Not landed yet: run the fallback path unperturbed.
            for (rim, mut t, _) in set.p0().iter_mut() {
                let c = match rim.0 {
                    OrbKind::Hp => hp_c,
                    OrbKind::Mana => mana_c,
                };
                t.translation = at(&frame, c, t.translation.z);
                t.scale = Vec3::new(s, s, 1.0);
            }
            for mut t in set.p1().iter_mut() {
                t.translation = at(&frame, Vec2::new(hp_c.x, fill_y(hp_c, hp_frac)), Z + 2.5);
                t.scale = Vec3::new(s, hp_frac.max(0.001) * s, 1.0);
            }
            for mut t in set.p2().iter_mut() {
                t.translation = at(&frame, Vec2::new(mana_c.x, fill_y(mana_c, mana_frac)), Z + 2.5);
                t.scale = Vec3::new(s, mana_frac.max(0.001) * s, 1.0);
            }
            for (pip, mut t, material) in set.p3().iter_mut() {
                t.translation = at(&frame, bottom(h, ui, -248.0 + pip.0 as f32 * 20.0, 54.0), Z + 1.0);
                t.scale = Vec3::new(s, s, 1.0);
                let lit = view.player_sigils.get(pip.0).copied().unwrap_or(false);
                if let Some(mut mat) = materials.get_mut(&material.0) {
                    mat.color = if lit {
                        Color::srgb_u8(220, 183, 105)
                    } else {
                        Color::srgba_u8(220, 183, 105, 60)
                    };
                }
            }
            return;
        };
        *chrome_ready = true;
        for (kind, key) in [(OrbKind::Hp, "OrbFrameRed"), (OrbKind::Mana, "OrbFrameBlue")] {
            if let Some((image, rect)) = images.as_deref().and_then(|i| {
                atlas
                    .as_deref()
                    .and_then(|a| atlas::chrome_ref(a, i, key))
            }) {
                commands.spawn((
                    Sprite {
                        image: image.clone(),
                        rect: Some(rect),
                        custom_size: Some(Vec2::splat(68.0)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, Z + 3.0),
                    OrbFrame(kind),
                ));
            }
        }
        for i in 0..3usize {
            if let Some((image, rect)) = images.as_deref().and_then(|i| {
                atlas
                    .as_deref()
                    .and_then(|a| atlas::chrome_ref(a, i, "SigilPip"))
            }) {
                commands.spawn((
                    Sprite {
                        image: image.clone(),
                        rect: Some(rect),
                        custom_size: Some(Vec2::splat(20.0)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, Z + 1.0),
                    PipHousing(i),
                ));
            }
        }
    }

    let chrome_here = *chrome_ready;
    for (rim, mut t, fallback) in set.p0().iter_mut() {
        let c = match rim.0 {
            OrbKind::Hp => hp_c,
            OrbKind::Mana => mana_c,
        };
        t.translation = at(&frame, c, t.translation.z);
        t.scale = Vec3::new(s, s, 1.0);
        if fallback.is_some() && chrome_here {
            t.scale = Vec3::ZERO; // glass cap retires once the frame plate lands
        }
    }
    for mut t in set.p1().iter_mut() {
        t.translation = at(&frame, Vec2::new(hp_c.x, fill_y(hp_c, hp_frac)), Z + 2.5);
        t.scale = Vec3::new(s, hp_frac.max(0.001) * s, 1.0);
    }
    for mut t in set.p2().iter_mut() {
        t.translation = at(&frame, Vec2::new(mana_c.x, fill_y(mana_c, mana_frac)), Z + 2.5);
        t.scale = Vec3::new(s, mana_frac.max(0.001) * s, 1.0);
    }
    for (plate, mut t) in set.p4().iter_mut() {
        let c = match plate.0 {
            OrbKind::Hp => hp_c,
            OrbKind::Mana => mana_c,
        };
        t.translation = at(&frame, c, t.translation.z);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (housing, mut t, mut sprite) in set.p5().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, -248.0 + housing.0 as f32 * 20.0, 54.0), Z + 1.0);
        t.scale = Vec3::new(s, s, 1.0);
        let lit = view.player_sigils.get(housing.0).copied().unwrap_or(false);
        sprite.color = if lit {
            Color::srgb(1.0, 1.0, 1.0)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.32)
        };
    }
    for (pip, mut t, material) in set.p3().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, -248.0 + pip.0 as f32 * 20.0, 54.0), Z + 1.0);
        t.scale = if chrome_here {
            Vec3::ZERO // flat discs retire once housings land
        } else {
            Vec3::new(s, s, 1.0)
        };
        let lit = view.player_sigils.get(pip.0).copied().unwrap_or(false);
        if let Some(mut mat) = materials.get_mut(&material.0) {
            mat.color = if lit {
                Color::srgb_u8(220, 183, 105)
            } else {
                Color::srgba_u8(220, 183, 105, 60)
            };
        }
    }
}

/// Labels and ribbons: stamina, XP sliver, meter chip + text. The XPSliver
/// rail plate replaces the flat track lazily once the chrome sheet resolves.
pub fn update_labels(
    mut commands: Commands,
    view: Res<WorldView>,
    frame: Res<CamFrame>,
    windows: Query<&Window, ()>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    mut set: ParamSet<(
        Query<&mut Transform, With<StaminaBg>>,
        Query<&mut Transform, With<StaminaFill>>,
        Query<(&mut Transform, Option<&FlatFallback>), With<XpBg>>,
        Query<&mut Transform, With<XpFill>>,
        Query<&mut Transform, With<ChipBg>>,
        Query<(&mut Transform, &mut Text2d, &mut TextColor, &mut Visibility), With<MeterText>>,
        Query<&mut Transform, With<XpRail>>,
        Query<
            (&mut Transform, Option<&HudPlate>, Option<&HudValue>, Option<&mut Sprite>, Option<&mut Text2d>),
            Or<(With<HudPlate>, With<HudValue>)>,
        >,
    )>,
    mut xp_ready: Local<bool>,
    mut panel_ready: Local<bool>,
    mut last_values: Local<Option<[u64; 6]>>,
) {
    let Ok(window) = windows.single() else { return };
    let (w, h) = (window.width(), window.height());
    let ui = hud_scale(w, h);
    let s = frame.scale.max(0.0001) * ui;
    let sta_frac = ratio(view.player_stamina, view.player_max_stamina);
    let xp_frac = if view.player_level >= 10 {
        1.0
    } else {
        (view.player_xp as f32 / (25.0 + view.player_level as f32 * 15.0)).clamp(0.0, 1.0)
    };

    if !*xp_ready {
        if let Some((image, rect)) = images.as_deref().and_then(|i| {
            atlas.as_deref().and_then(|a| atlas::chrome_ref(a, i, "XPSliver"))
        }) {
            *xp_ready = true;
            commands.spawn((
                Sprite {
                    image: image.clone(),
                    rect: Some(rect),
                    custom_size: Some(Vec2::new(330.0, 12.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, Z - 0.5),
                XpRail,
            ));
        }
    }

    let panel_image = if !*panel_ready {
        images.as_deref().and_then(|i| {
            atlas.as_deref().and_then(|a| atlas::chrome_ref(a, i, "PanelFill"))
        })
    } else {
        None
    };
    for (mut t, plate, _, sprite, _) in set.p7().iter_mut() {
        let Some(plate) = plate else { continue };
        let Some(mut sprite) = sprite else { continue };
        let (pos, z) = match plate.0 {
            0 => (bottom(h, ui, 0.0, 56.0), Z - 3.0),
            1 => (bottom(h, ui, 0.0, 56.0), Z - 2.8),
            2 => (bottom(h, ui, 0.0, 56.0), Z - 2.6),
            3 => (bottom(h, ui, 0.0, 106.0), Z - 2.4),
            4 => (bottom(h, ui, 0.0, 5.0), Z - 2.4),
            5 => (bottom(h, ui, 0.0, 58.0), Z - 2.2),
            6 => (bottom(h, ui, 0.0, 92.0), Z - 2.0),
            7 => (Vec2::new(-w * 0.5 + 8.0 + 177.0 * ui, h * 0.5 - 8.0 - 17.0 * ui), Z - 3.0),
            8 => (Vec2::new(-w * 0.5 + 8.0 + 177.0 * ui, h * 0.5 - 8.0 - 32.0 * ui), Z - 2.8),
            9..=14 => (bottom(h, ui, [-355.0, -286.0, -161.0, 161.0, 286.0, 355.0][plate.0 - 9], 105.0), Z - 2.3),
            _ => continue,
        };
        t.translation = at(&frame, pos, z);
        t.scale = Vec3::new(s, s, 1.0);
        if plate.0 == 5 {
            if let Some((image, rect)) = panel_image.as_ref() {
                sprite.image = (*image).clone();
                sprite.rect = Some(*rect);
                sprite.color = Color::WHITE;
                *panel_ready = true;
            }
        }
    }

    for mut t in set.p0().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 0.0, 55.0), Z + 0.4);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for mut t in set.p1().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, (sta_frac - 1.0) * 48.0, 55.0), Z + 1.0);
        t.scale = Vec3::new(sta_frac.max(0.001) * s, s, 1.0);
    }
    for mut t in set.p6().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 0.0, 16.0), Z - 0.5);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (mut t, fallback) in set.p2().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 0.0, 16.0), Z + 0.4);
        t.scale = if fallback.is_some() && *xp_ready { Vec3::ZERO } else { Vec3::new(s, s, 1.0) };
    }
    for mut t in set.p3().iter_mut() {
        let span = if *xp_ready { 304.0 } else { 320.0 };
        t.translation = at(&frame, bottom(h, ui, (xp_frac - 1.0) * span * 0.5, 16.0), Z + 0.5);
        t.scale = Vec3::new(xp_frac.max(0.001) * (span / 320.0) * s, s, 1.0);
    }
    for mut t in set.p4().iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 0.0, 81.0), Z + 1.0);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (mut transform, mut text, mut color, mut visibility) in set.p5().iter_mut() {
        transform.translation = at(&frame, bottom(h, ui, 0.0, 81.0), Z + 3.0);
        transform.scale = Vec3::new(s, s, 1.0);
        match &view.class_meter {
            Some((str_value, hue)) => {
                *visibility = Visibility::Visible;
                if text.0 != *str_value {
                    text.0.clone_from(str_value);
                }
                let (r, g, b) = hue_rgb(*hue);
                color.0 = Color::srgb_u8(r, g, b);
            }
            // No class meter for this class yet. Blanking the slot left an empty
            // inset in the middle of the chrome and printing the class enum
            // literally spelled `NONE`, so it shows WHERE the player is instead:
            // always true, always useful, and it keeps the inset working.
            None => {
                if view.map_name.is_empty() {
                    *visibility = Visibility::Hidden;
                } else {
                    *visibility = Visibility::Visible;
                    let label = view.map_name.to_uppercase();
                    if text.0 != label {
                        text.0 = label;
                    }
                    color.0 = Color::srgb_u8(225, 210, 173);
                }
            }
        }
    }

    use std::fmt::Write as _;
    let sigils = view.player_sigils.iter().filter(|&&lit| lit).count();
    let xp_goal = 25 + view.player_level * 15;
    let values = [
        ((view.hour as u64) << 40) | ((view.player_level as u64) << 8) | sigils as u64,
        ((view.player_hp as u32 as u64) << 32) | view.player_max_hp as u32 as u64,
        ((view.player_mana as u32 as u64) << 32) | view.player_max_mana as u32 as u64,
        ((view.player_stamina as u32 as u64) << 32) | view.player_max_stamina as u32 as u64,
        ((view.player_level as u64) << 32) | view.player_xp as u64,
        sigils as u64,
    ];
    for (mut t, _, value, _, text) in set.p7().iter_mut() {
        let Some(value) = value else { continue };
        let Some(mut text) = text else { continue };
        // Time and sigils use the left flank; the right flank is the live
        // wayfinder. Neither encroaches on the central keys or the orbs.
        let pos = match value.0 {
            0 => bottom(h, ui, -228.0, 78.0),
            1 => bottom(h, ui, -330.0, 55.0),
            2 => bottom(h, ui, 330.0, 55.0),
            3 => bottom(h, ui, 0.0, 65.0),
            4 => bottom(h, ui, -228.0, 16.0),
            _ => bottom(h, ui, -228.0, 34.0),
        };
        t.translation = at(&frame, pos, Z + 4.0);
        t.scale = Vec3::new(s, s, 1.0);
        if last_values.as_ref().is_some_and(|old| old[value.0] == values[value.0]) {
            continue;
        }
        text.0.clear();
        match value.0 {
            0 => { let _ = write!(&mut text.0, "HOUR {:02}   LV {}", view.hour, view.player_level); }
            1 => { let _ = write!(&mut text.0, "{} / {}", view.player_hp, view.player_max_hp); }
            2 => { let _ = write!(&mut text.0, "{} / {}", view.player_mana, view.player_max_mana); }
            3 => { let _ = write!(&mut text.0, "STA  {} / {}", view.player_stamina, view.player_max_stamina); }
            4 if view.player_level >= 10 => text.0.push_str("XP MAX"),
            4 => { let _ = write!(&mut text.0, "XP  {} / {}", view.player_xp, xp_goal); }
            _ => { let _ = write!(&mut text.0, "SIGILS {}/3", sigils); }
        }
    }
    *last_values = Some(values);
}

/// Terrain, known landmarks and the player in a tightly cropped local chart.
/// A cell absent from the snapshot is fog, even when the world map is larger.
fn chart_color(view: &WorldView, pos: Pos) -> Color {
    if pos == view.player {
        return Color::srgb_u8(246, 213, 128);
    }
    let col = pos.x - view.left;
    let row = pos.y - view.top;
    if col < 0 || row < 0 || col >= view.cols as i32 || row >= view.rows as i32 {
        return Color::srgb_u8(21, 26, 27);
    }
    let Some(cell) = view.cells.get(row as usize * view.cols as usize + col as usize) else {
        return Color::srgb_u8(21, 26, 27);
    };
    if !cell.explored {
        return Color::srgb_u8(21, 26, 27);
    }
    if view.portals.contains(&pos) || matches!(cell.tile, Tile::Up | Tile::Down) {
        return Color::srgb_u8(95, 204, 206);
    }
    match cell.tile {
        Tile::Shrine => Color::srgb_u8(232, 188, 107),
        Tile::Chest => Color::srgb_u8(184, 127, 74),
        Tile::River | Tile::Ford => Color::srgb_u8(58, 111, 140),
        Tile::Wall | Tile::Rock | Tile::Mountain => Color::srgb_u8(78, 83, 80),
        Tile::Forest | Tile::DeepForest => Color::srgb_u8(68, 100, 79),
        Tile::Road | Tile::Door | Tile::Floor => Color::srgb_u8(154, 145, 117),
        _ => Color::srgb_u8(111, 117, 100),
    }
}

pub fn update_wayfinder(
    view: Res<WorldView>,
    frame: Res<CamFrame>,
    windows: Query<&Window>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    mut cells: Query<(&MapCell, &mut Sprite, &mut Transform)>,
    mut frames: Query<(&MapFrame, &mut Transform), Without<MapCell>>,
    mut caption: Query<&mut Transform, (With<MapCaption>, Without<MapCell>, Without<MapFrame>, Without<MenuKey>, Without<MenuCaption>)>,
    mut keys: Query<(&MenuKey, &mut Sprite, &mut Transform), (Without<MapCell>, Without<MapFrame>, Without<MapCaption>, Without<MenuCaption>)>,
    mut labels: Query<(&MenuCaption, &mut Transform), (Without<MapCell>, Without<MapFrame>, Without<MapCaption>, Without<MenuKey>)>,
) {
    let Ok(window) = windows.single() else { return };
    let (w, h) = (window.width(), window.height());
    let ui = hud_scale(w, h);
    let s = frame.scale.max(0.0001) * ui;
    let hover = window.cursor_position().and_then(|p| hit_test(p, w, h)).flatten();
    for (part, mut t) in frames.iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 226.0, 56.0), Z + part.0 as f32 * 0.1);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for mut t in caption.iter_mut() {
        t.translation = at(&frame, bottom(h, ui, 226.0, 91.0), Z + 1.5);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (cell, mut sprite, mut t) in cells.iter_mut() {
        let col = cell.0 % MAP_COLS;
        let row = cell.0 / MAP_COLS;
        let pos = Pos::new(
            view.player.x + col as i32 - MAP_COLS as i32 / 2,
            view.player.y + row as i32 - MAP_ROWS as i32 / 2,
        );
        sprite.color = chart_color(&view, pos);
        t.translation = at(
            &frame,
            bottom(h, ui, 226.0 + (col as f32 - 7.0) * 5.0, 56.0 + (5.0 - row as f32) * 5.0),
            Z + 0.3,
        );
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (key, mut sprite, mut t) in keys.iter_mut() {
        let selected = hover == Some(key.0);
        let chrome = atlas.as_deref().zip(images.as_deref()).and_then(|(atlas, images)| {
            atlas::chrome_ref(atlas, images, if selected { "ButtonSelected" } else { "ButtonNormal" })
        });
        if let Some((image, rect)) = chrome {
            if sprite.rect != Some(rect) {
                sprite.image = image.clone();
                sprite.rect = Some(rect);
            }
            sprite.color = Color::WHITE;
        } else {
            sprite.color = if selected {
                Color::srgb_u8(147, 111, 56)
            } else {
                Color::srgb_u8(75, 61, 40)
            };
        }
        t.translation = at(&frame, bottom(h, ui, MENU_X[key.0], 40.0), Z + 1.1);
        t.scale = Vec3::new(s, s, 1.0);
    }
    for (label, mut t) in labels.iter_mut() {
        t.translation = at(&frame, bottom(h, ui, MENU_X[label.0], 40.0), Z + 2.0);
        t.scale = Vec3::new(s, s, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Cell;
    use laya_realms::model::Modal;

    #[test]
    fn menu_hitboxes_track_scaled_rail_and_do_not_open_world_below_it() {
        for (w, h) in [(1280.0, 720.0), (640.0, 360.0)] {
            let ui = hud_scale(w, h);
            for (index, x) in MENU_X.iter().enumerate() {
                let p = Vec2::new(w * 0.5 + x * ui, h - 8.0 - 40.0 * ui);
                assert_eq!(hit_test(p, w, h), Some(Some(index)));
            }
            assert_eq!(hit_test(Vec2::new(w * 0.5, h - 8.0 - 90.0 * ui), w, h), Some(None));
            assert_eq!(hit_test(Vec2::new(w * 0.5, h - 8.0 - 120.0 * ui), w, h), None);
        }
    }

    #[test]
    fn menu_keys_open_real_simulation_modals() {
        for (index, expected) in [Modal::Atlas, Modal::Inventory, Modal::Talents, Modal::Pause, Modal::Journal]
            .into_iter().enumerate()
        {
            let mut game = Game::new(42);
            game.modal = Modal::None;
            game.selected = 4;
            open_button(&mut game, index);
            assert_eq!(std::mem::discriminant(&game.modal), std::mem::discriminant(&expected));
            assert_eq!(game.selected, 0);
        }
    }

    #[test]
    fn wayfinder_hides_unknown_cells_and_marks_known_landmarks() {
        let mut view = WorldView {
            player: Pos::new(1, 1),
            cols: 3,
            rows: 1,
            left: 0,
            top: 0,
            cells: vec![
                Cell { pos: Pos::new(0, 0), explored: true, tile: Tile::Road, door_open: false, door_along_x: true },
                Cell { pos: Pos::new(1, 0), explored: false, tile: Tile::Shrine, door_open: false, door_along_x: true },
                Cell { pos: Pos::new(2, 0), explored: true, tile: Tile::Road, door_open: false, door_along_x: true },
            ],
            portals: vec![Pos::new(1, 0), Pos::new(2, 0)],
            ..default()
        };
        assert_ne!(chart_color(&view, Pos::new(0, 0)), chart_color(&view, Pos::new(1, 0)));
        assert_eq!(chart_color(&view, Pos::new(1, 0)), chart_color(&view, Pos::new(-1, 0)));
        assert_ne!(chart_color(&view, Pos::new(2, 0)), chart_color(&view, Pos::new(0, 0)));
        view.player = Pos::new(2, 0);
        assert_ne!(chart_color(&view, view.player), chart_color(&view, Pos::new(1, 0)));
    }
}
