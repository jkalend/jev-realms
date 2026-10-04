//! Live three-lane oath diagram. The core still owns point costs and all unlock
//! rules; this layer only projects its twelve nodes into illustrated controls.
use bevy::prelude::*;
use bevy::text::{FontSize, TextLayoutInfo};
use laya_realms::model::{talent_tree, Game, Modal, TalentTreeDef};

use crate::camera::CamFrame;
use crate::modals::{pane_width, ModalBg};
use crate::sim::SimSlot;

const Z: f32 = 223.0;
const GOLD: Color = Color::srgb_u8(218, 177, 101);
const INK: Color = Color::srgb_u8(226, 215, 188);
const SEA: Color = Color::srgb_u8(127, 178, 183);
const MUTED: Color = Color::srgb_u8(135, 140, 145);
const ROW_STEP: f32 = 75.0;

#[derive(Component)]
pub(crate) enum Plate {
    Lane(usize),
    Link(usize),
    Glow(usize),
    Emblem(usize),
    Learn,
}

#[derive(Component)]
pub(crate) enum Inscription {
    Summary,
    Branch(usize),
    Node(usize),
    Detail,
    Effect,
    Hint,
    Learn,
    Close,
}

fn at(frame: &CamFrame, pos: Vec2, z: f32) -> Transform {
    Transform {
        translation: (frame.pos + pos * frame.scale).extend(z),
        scale: Vec3::splat(frame.scale),
        ..default()
    }
}

fn point(width: f32, height: f32, index: usize) -> Vec2 {
    let branch = index / 4;
    let tier = index % 4;
    Vec2::new((branch as f32 - 1.0) * width * 0.32,
              height * 0.5 - 145.0 - tier as f32 * ROW_STEP)
}

fn can_learn(game: &Game, tree: &TalentTreeDef, index: usize) -> bool {
    let branch = index / 4;
    let tier = index % 4;
    let def = &tree.branches[branch].nodes[tier];
    game.player.level >= u32::from(def.level)
        && game.player.talent_rank(index as u8) < def.cost
        && game.player.talent_points_available() > 0
        && (tier == 0 || game.player.talent_rank((index - 1) as u8)
            >= tree.branches[branch].nodes[tier - 1].cost)
}

fn status(game: &Game, tree: &TalentTreeDef, index: usize) -> &'static str {
    let def = &tree.branches[index / 4].nodes[index % 4];
    let rank = game.player.talent_rank(index as u8);
    if rank >= def.cost { "HELD" }
    else if rank > 0 { "PARTIALLY LEARNED" }
    else if game.player.level < u32::from(def.level) { "LEVEL LOCKED" }
    else if !index.is_multiple_of(4) && game.player.talent_rank((index - 1) as u8)
        < tree.branches[index / 4].nodes[index % 4 - 1].cost { "NEEDS PRIOR NODE" }
    else if game.player.talent_points_available() == 0 { "NO POINTS" }
    else { "READY" }
}

fn class_row(game: &Game) -> usize {
    use laya_realms::model::Class;
    match game.player.class {
        Class::Keepwarden => 0, Class::Gravebound => 1, Class::Redwake => 2,
        Class::Waysworn => 3, Class::SigilSworn => 4, Class::Fensworn => 5,
        Class::None => 0,
    }
}

/// Single click inspects a seal. LEARN spends a point through the same core
/// Enter action as the keyboard; clicking a seal never spends by accident.
pub(crate) fn clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    panes: Query<&Sprite, With<ModalBg>>,
    mut slot: ResMut<SimSlot>,
    mut input: ResMut<crate::input::InputState>,
) {
    if input.frame_consumed || !buttons.just_pressed(MouseButton::Left) || !matches!(slot.game.modal, Modal::Talents) {
        return;
    }
    let Some(tree) = talent_tree(slot.game.player.class) else { return; };
    let Ok(window) = windows.single() else { return; };
    let Some(pane) = panes.iter().next() else { return; };
    let width = pane_width(&slot.game.modal, slot.game.create_step, window.width());
    let height = pane.custom_size.map(|size| size.y + if pane.rect.is_some() { 64.0 } else { 0.0 })
        .unwrap_or(530.0);
    let Some(cursor) = window.cursor_position() else { return; };
    let cursor = Vec2::new(cursor.x - window.width() * 0.5,
                           window.height() * 0.5 - cursor.y);
    input.frame_consumed = true;
    for index in 0..12 {
        let center = point(width, height, index);
        if (cursor - center).abs().cmple(Vec2::splat(24.0)).all() {
            slot.game.selected = index;
            return;
        }
    }
    if cursor.x >= width * 0.5 - 184.0 && cursor.x <= width * 0.5 - 36.0
        && (cursor.y - (-height * 0.5 + 61.0)).abs() <= 19.0
        && can_learn(&slot.game, tree, slot.game.selected.min(11))
    {
        laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE));
    } else if cursor.x >= width * 0.5 - 73.0
        && (cursor.y - (height * 0.5 - 39.0)).abs() <= 24.0
    {
        laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
    }
}

fn wrap_effect(text: &str, columns: usize) -> String {
    let mut wrapped = String::new();
    let mut line = 0;
    for word in text.split_whitespace() {
        if line > 0 && line + word.len() + 1 > columns {
            wrapped.push('\n');
            line = 0;
        } else if line > 0 {
            wrapped.push(' ');
            line += 1;
        }
        for ch in word.chars() { wrapped.push(if ch == '–' { '-' } else { ch }); }
        line += word.len();
    }
    wrapped
}

pub(crate) fn render(
    mut commands: Commands,
    slot: Res<SimSlot>,
    frame: Res<CamFrame>,
    windows: Query<&Window>,
    panes: Query<&Sprite, With<ModalBg>>,
    assets: Res<AssetServer>,
    mut spawned: Local<bool>,
    mut plates: Query<(&Plate, &mut Transform, &mut Sprite, &mut Visibility), (Without<Inscription>, Without<ModalBg>)>,
    mut inscriptions: Query<(&Inscription, &mut Transform, &mut Text2d, &mut TextFont, &mut TextColor, &mut Visibility, &TextLayoutInfo), Without<Plate>>,
) {
    let game = &slot.game;
    let tree = if matches!(game.modal, Modal::Talents) { talent_tree(game.player.class) } else { None };
    if !*spawned {
        if tree.is_none() { return; }
        let image: Handle<Image> = assets.load("ui/talent-emblems.png");
        for branch in 0..3 {
            commands.spawn((Plate::Lane(branch), Sprite::from_color(Color::BLACK, Vec2::ONE), Transform::default(), Visibility::Hidden));
            commands.spawn((Inscription::Branch(branch), Text2d::new(""), TextFont::from_font_size(14.0), TextColor(GOLD), Transform::default(), Visibility::Hidden));
        }
        for index in 0..12 {
            if index % 4 > 0 {
                commands.spawn((Plate::Link(index), Sprite::from_color(SEA, Vec2::ONE), Transform::default(), Visibility::Hidden));
            }
            commands.spawn((Plate::Glow(index), Sprite::from_color(GOLD, Vec2::ONE), Transform::default(), Visibility::Hidden));
            commands.spawn((Plate::Emblem(index), Sprite::from_image(image.clone()), Transform::default(), Visibility::Hidden));
            commands.spawn((Inscription::Node(index), Text2d::new(""), TextFont::from_font_size(11.0), TextColor(INK), Transform::default(), Visibility::Hidden));
        }
        for part in [Inscription::Summary, Inscription::Detail, Inscription::Effect, Inscription::Hint,
                     Inscription::Learn, Inscription::Close] {
            commands.spawn((part, Text2d::new(""), TextFont::from_font_size(13.0), TextColor(INK), Transform::default(), Visibility::Hidden));
        }
        commands.spawn((Plate::Learn, Sprite::from_color(GOLD, Vec2::ONE), Transform::default(), Visibility::Hidden));
        *spawned = true;
        return;
    }
    let Some(tree) = tree else {
        for (_, _, _, mut visibility) in &mut plates { *visibility = Visibility::Hidden; }
        for (_, _, _, _, _, mut visibility, _) in &mut inscriptions { *visibility = Visibility::Hidden; }
        return;
    };
    let Some(window) = windows.iter().next() else { return; };
    let Some(pane) = panes.iter().next() else { return; };
    let width = pane_width(&game.modal, game.create_step, window.width());
    let height = pane.custom_size.map(|size| size.y + if pane.rect.is_some() { 64.0 } else { 0.0 })
        .unwrap_or(530.0);
    let selected = game.selected.min(11);
    let def = &tree.branches[selected / 4].nodes[selected % 4];
    let ready = can_learn(game, tree, selected);
    let col_width = (width * 0.29).min(264.0);
    for (part, mut tf, mut sprite, mut visibility) in &mut plates {
        let (pos, size, z, color) = match *part {
            Plate::Lane(branch) => (
                Vec2::new((branch as f32 - 1.0) * width * 0.32, height * 0.5 - 252.0),
                Vec2::new(col_width, 343.0), Z - 0.2,
                Color::srgba(0.09, 0.13, 0.16, 0.92),
            ),
            Plate::Link(index) => {
                let paid = game.player.talent_rank((index - 1) as u8)
                    >= tree.branches[index / 4].nodes[index % 4 - 1].cost;
                (point(width, height, index) + Vec2::new(0.0, ROW_STEP * 0.5 - 11.0),
                 Vec2::new(if paid { 3.0 } else { 2.0 }, 11.0), Z + 0.1,
                 if paid { GOLD } else { Color::srgb_u8(77, 96, 105) })
            }
            Plate::Glow(index) => {
                let rank = game.player.talent_rank(index as u8);
                let paid = rank >= tree.branches[index / 4].nodes[index % 4].cost;
                (point(width, height, index), Vec2::splat(if index % 4 == 3 { 38.0 } else if selected == index { 37.0 } else { 32.0 }),
                 Z + 0.2, if selected == index { GOLD } else if paid { SEA } else { Color::srgb_u8(40, 51, 55) })
            }
            Plate::Emblem(index) => {
                let rank = game.player.talent_rank(index as u8);
                let paid = rank >= tree.branches[index / 4].nodes[index % 4].cost;
                let gate = &tree.branches[index / 4].nodes[index % 4];
                sprite.rect = Some(Rect::new((index / 4) as f32 * 64.0,
                    class_row(game) as f32 * 64.0,
                    (index / 4 + 1) as f32 * 64.0, (class_row(game) + 1) as f32 * 64.0));
                (point(width, height, index), Vec2::splat(if index % 4 == 3 { 46.0 } else { 42.0 }), Z + 0.3,
                 if selected == index || paid { Color::WHITE }
                 else if rank > 0 { Color::srgb_u8(218, 190, 144) }
                 else if game.player.level < u32::from(gate.level) { Color::srgb_u8(96, 102, 103) }
                 else { Color::srgb_u8(166, 169, 158) })
            }
            Plate::Learn => (
                Vec2::new(width * 0.5 - 110.0, -height * 0.5 + 61.0),
                Vec2::new(148.0, 38.0), Z + 0.3,
                if ready { Color::srgba(0.38, 0.29, 0.14, 1.0) } else { Color::srgba(0.15, 0.17, 0.19, 1.0) },
            ),
        };
        *tf = at(&frame, pos, z);
        if matches!(part, Plate::Glow(_)) { tf.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4); }
        sprite.custom_size = Some(size);
        sprite.color = color;
        *visibility = Visibility::Visible;
    }
    for (part, mut tf, mut text, mut font, mut tint, mut visibility, info) in &mut inscriptions {
        let (pos, value, color, size) = match *part {
            Inscription::Summary => (
                Vec2::new(0.0, height * 0.5 - 68.0),
                format!("{}  /  {}     /     {} POINTS IN HAND", game.player.class.name().to_uppercase(),
                    game.player.class.calling().to_uppercase(), game.player.talent_points_available()), GOLD, 12.0),
            Inscription::Branch(branch) => (
                Vec2::new((branch as f32 - 1.0) * width * 0.32, height * 0.5 - 100.0),
                tree.branches[branch].name.to_uppercase(), SEA, 14.0),
            Inscription::Node(index) => {
                let node = &tree.branches[index / 4].nodes[index % 4];
                let rank = game.player.talent_rank(index as u8);
                let color = if selected == index { GOLD } else if rank >= node.cost { INK } else { MUTED };
                (point(width, height, index) + Vec2::new(0.0, -36.0),
                 format!("{}  /  L{}{}", node.name, node.level,
                     if node.cost == 2 { format!("  {rank}/2") } else { String::new() }), color, 11.5)
            }
            Inscription::Detail => (
                Vec2::new(-width * 0.5 + 50.0, -height * 0.5 + 87.0),
                format!("{} / {}   /   {}   /   {}", tree.branches[selected / 4].name.to_uppercase(),
                    def.name.to_uppercase(), status(game, tree, selected),
                    if def.cost == 2 { "TWO POINTS" } else { "ONE POINT" }), GOLD, 12.0),
            Inscription::Effect => (
                Vec2::new(-width * 0.5 + 50.0, -height * 0.5 + 59.0),
                wrap_effect(def.text, 82), INK, 12.5),
            Inscription::Hint => (
                Vec2::new(-width * 0.5 + 50.0, -height * 0.5 + 22.0),
                "W/S TIERS   A/D BRANCHES   CLICK A SEAL TO INSPECT   ENTER LEARN   ESC CLOSE".to_owned(), MUTED, 10.0),
            Inscription::Learn => (
                Vec2::new(width * 0.5 - 110.0, -height * 0.5 + 61.0),
                if ready { "LEARN  /  ENTER" } else { "LOCKED  /  HELD" }.to_owned(),
                if ready { GOLD } else { MUTED }, 11.0),
            Inscription::Close => (
                Vec2::new(width * 0.5 - 55.0, height * 0.5 - 38.0),
                "CLOSE  X".to_owned(), MUTED, 11.0),
        };
        let left_aligned = matches!(part, Inscription::Detail | Inscription::Effect | Inscription::Hint);
        let pos = if left_aligned { pos + Vec2::new(info.size.x * 0.5, 0.0) } else { pos };
        *tf = at(&frame, pos, Z + 0.6);
        if text.0 != value { text.0 = value; }
        if font.font_size != FontSize::Px(size) { font.font_size = FontSize::Px(size); }
        tint.0 = color;
        *visibility = Visibility::Visible;
    }
}
