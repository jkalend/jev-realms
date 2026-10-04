//! Deliberate one-click combat verbs and live tactical context, above the HUD.
use std::borrow::Cow;
use bevy::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use laya_realms::model::{Action, Game, Item, Modal};

use crate::camera::CamFrame;
use crate::hud::hud_scale as scale;
use crate::sim::SimSlot;

const COUNT: usize = 6;
const STEP: f32 = 98.0;
const WIDTH: f32 = 92.0;
const Z: f32 = 110.0;
const LABELS: [&str; COUNT] = ["Strike", "Guard", "Runes", "Heal", "Recover", "Escape"];
const HINTS: [&str; COUNT] = ["Enter", "F  brace", "Y  choose", "Potion", "Space", "X  risk"];
const GOLD: Color = Color::srgb_u8(214, 180, 113);
const PALE: Color = Color::srgb_u8(238, 224, 193);
const MUTED: Color = Color::srgb_u8(148, 160, 157);

#[derive(Component)]
pub(crate) struct ControlPlate(usize);
#[derive(Component)]
pub(crate) struct ControlText(usize);

fn center(index: usize) -> f32 {
    (index as f32 - (COUNT - 1) as f32 * 0.5) * STEP
}

pub(crate) fn hit_test(screen: Vec2, w: f32, h: f32) -> Option<usize> {
    let ui = scale(w, h);
    let x = (screen.x - w * 0.5) / ui;
    let y = (h - screen.y - 8.0) / ui;
    if x.abs() > 294.0 || !(118.0..=222.0).contains(&y) {
        return None;
    }
    Some(if (120.0..=162.0).contains(&y) {
        (0..COUNT).find(|&i| (x - center(i)).abs() <= WIDTH * 0.5).unwrap_or(COUNT)
    } else {
        COUNT
    })
}

fn strike_target(game: &Game) -> Option<usize> {
    game.npcs.iter().filter(|n| {
        n.alive() && n.map == game.player.map
            && laya_realms::engine::hostile(n)
            && game.can_melee(game.player.pos, n.pos)
    }).min_by_key(|n| (n.hp, n.id)).map(|n| n.id)
}

fn available(game: &Game, index: usize) -> bool {
    match index {
        0 => strike_target(game).is_some(),
        2 => !game.player.spells.is_empty(),
        3 => game.player.hp < game.player.max_hp
            && game.player.inventory.iter().any(|i| matches!(i, Item::Potion | Item::GreaterPotion)),
        1 | 4 | 5 => true,
        _ => false,
    }
}

pub(crate) fn activate(game: &mut Game, index: usize) {
    if game.combat.is_none() || !matches!(game.modal, Modal::None) || !available(game, index) {
        return;
    }
    match index {
        0 => game.action(Action::Attack),
        1 => game.action(Action::Defend),
        2 => laya_realms::input::key(game, KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
        3 => {
            if let Some(index) = game.player.inventory.iter().position(|i| matches!(i, Item::Potion | Item::GreaterPotion)) {
                game.action(Action::Use(index));
            }
        }
        4 => game.action(Action::Wait),
        5 => game.action(Action::Flee),
        _ => {}
    }
}

fn context(game: &Game) -> String {
    if let Some(npc) = game.npcs.iter().filter(|n| n.alive() && n.map == game.player.map)
        .filter(|n| n.telegraph.as_ref().is_some_and(|(target, _)| target.distance(game.player.pos) <= 1))
        .min_by_key(|n| (n.cooldown, n.id)) {
        if npc.cooldown == 0 && npc.speed > game.player_speed() {
            return "Impact before you can move. Guard to soften the blow.".into();
        }
        return format!("Leave the marked ground. Impact in {} action{}.", npc.cooldown + 1, if npc.cooldown == 0 { "" } else { "s" });
    }
    if game.player.stamina < 2 {
        return "Low stamina. Guard to recover, or move to make space.".into();
    }
    if game.player.defending {
        return "Braced. Protection lasts until your next action.".into();
    }
    if let Some(id) = strike_target(game) {
        let npc = &game.npcs[id];
        return format!("{}   {} / {} HP   {}", npc.name, npc.hp, npc.max_hp,
            if npc.speed > game.player_speed() { "acts before you" } else { "you act first" });
    }
    "Click nearby ground to reposition. Click an enemy in reach to strike.".into()
}

pub(crate) fn render(
    mut commands: Commands,
    slot: Res<SimSlot>,
    frame: Res<CamFrame>,
    windows: Query<&Window>,
    mut spawned: Local<bool>,
    mut last_context: Local<Option<(u64, laya_realms::model::Pos, i32, bool)>>,
    mut context_text: Local<String>,
    mut round_text: Local<String>,
    mut plates: Query<(&ControlPlate, &mut Sprite, &mut Transform, &mut Visibility), Without<ControlText>>,
    mut captions: Query<(&ControlText, &mut Text2d, &mut TextColor, &mut Transform, &mut Visibility), Without<ControlPlate>>,
) {
    if !*spawned {
        for index in 0..COUNT + 2 {
            commands.spawn((ControlPlate(index), Sprite::from_color(MUTED, Vec2::ONE), Transform::default(), Visibility::Hidden));
        }
        for index in 0..COUNT * 2 + 2 {
            commands.spawn((ControlText(index), Text2d::new(""), TextFont::from_font_size(if index < COUNT { 14.0 } else { 11.0 }), TextColor(PALE), Transform::default(), Visibility::Hidden));
        }
        *spawned = true;
        return;
    }
    let game = &slot.game;
    let visible = game.combat.is_some() && matches!(game.modal, Modal::None) && game.player.hp > 0;
    let Ok(window) = windows.single() else { return; };
    let (w, h) = (window.width(), window.height());
    let ui = scale(w, h);
    let hover = window.cursor_position().and_then(|p| hit_test(p, w, h));
    let enabled: [bool; COUNT] = std::array::from_fn(|index| available(game, index));
    let context_key = (game.turn, game.player.pos, game.player.stamina, game.player.defending);
    if visible && *last_context != Some(context_key) {
        *context_text = context(game);
        *round_text = format!("Round {}   Time waits for you", game.combat.as_ref().map_or(0, |c| c.round));
        *last_context = Some(context_key);
    } else if !visible {
        *last_context = None;
    }
    let at = |x: f32, y: f32, z: f32| Transform {
        translation: (frame.pos + Vec2::new(x * ui, -h * 0.5 + 8.0 + y * ui) * frame.scale).extend(z),
        scale: Vec3::splat(frame.scale * ui), ..default()
    };
    for (part, mut sprite, mut tf, mut visibility) in &mut plates {
        *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
        if !visible { continue; }
        if part.0 >= COUNT {
            let round = part.0 == COUNT + 1;
            sprite.custom_size = Some(Vec2::new(588.0, if round { 22.0 } else { 30.0 }));
            sprite.color = Color::srgba_u8(10, 18, 21, 245);
            *tf = at(0.0, if round { 211.0 } else { 184.0 }, Z);
        } else {
            let enabled = enabled[part.0];
            sprite.custom_size = Some(Vec2::new(WIDTH, 42.0));
            sprite.color = if enabled && hover == Some(part.0) { Color::srgb_u8(82, 65, 39) }
                else if enabled { Color::srgb_u8(28, 39, 41) } else { Color::srgb_u8(20, 27, 29) };
            *tf = at(center(part.0), 141.0, Z);
        }
    }
    for (part, mut text, mut color, mut tf, mut visibility) in &mut captions {
        *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
        if !visible { continue; }
        let (copy, x, y, tint): (Cow<'_, str>, _, _, _) = if part.0 < COUNT {
            (LABELS[part.0].into(), center(part.0), 149.0, if enabled[part.0] { PALE } else { MUTED })
        } else if part.0 < COUNT * 2 {
            let index = part.0 - COUNT;
            let hint = if index == 3 && !enabled[index] {
                if game.player.hp >= game.player.max_hp { "Health full" } else { "No potion" }
            } else if index == 2 && !enabled[index] { "Not learned" } else { HINTS[index] };
            (hint.into(), center(index), 131.0, GOLD)
        } else if part.0 == COUNT * 2 {
            (context_text.as_str().into(), 0.0, 184.0, GOLD)
        } else {
            (round_text.as_str().into(), 0.0, 211.0, MUTED)
        };
        if text.0 != copy { text.0 = copy.into_owned(); }
        color.0 = tint;
        *tf = at(x, y, Z + 0.5);
    }
}

#[derive(Component)]
pub(crate) struct FeedbackPlate;
#[derive(Component)]
pub(crate) struct FeedbackText;

pub(crate) fn feedback(
    mut commands: Commands,
    slot: Res<SimSlot>,
    frame: Res<CamFrame>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut last: Local<String>,
    mut copy: Local<String>,
    mut changed_at: Local<f32>,
    mut spawned: Local<bool>,
    mut plates: Query<(&mut Sprite, &mut Transform, &mut Visibility), (With<FeedbackPlate>, Without<FeedbackText>)>,
    mut captions: Query<(&mut Text2d, &mut Transform, &mut Visibility), (With<FeedbackText>, Without<FeedbackPlate>)>,
) {
    if !*spawned {
        commands.spawn((FeedbackPlate, Sprite::from_color(Color::srgba_u8(11, 20, 22, 245), Vec2::new(584.0, 58.0)), Transform::default(), Visibility::Hidden));
        commands.spawn((FeedbackText, Text2d::new(""), TextFont::from_font_size(12.0), TextColor(PALE), Transform::default(), Visibility::Hidden));
        *spawned = true;
    }
    if let Some(message) = slot.game.log.back() {
        if last.as_str() != message {
            last.clone_from(message);
            copy.clear();
            let mut line = 0;
            let mut lines = 1;
            for word in message.split_whitespace() {
                let count = word.chars().count();
                if line > 0 && line + count + 1 > 76 {
                    if lines == 3 { copy.push('…'); break; }
                    copy.push('\n');
                    line = 0;
                    lines += 1;
                } else if line > 0 {
                    copy.push(' ');
                    line += 1;
                }
                copy.push_str(word);
                line += count;
            }
            *changed_at = time.elapsed_secs();
        }
    }
    let Ok(window) = windows.single() else { return; };
    let ui = scale(window.width(), window.height());
    let visible = !copy.is_empty() && time.elapsed_secs() - *changed_at < 7.0
        && matches!(slot.game.modal, Modal::None);
    let y = if slot.game.combat.is_some() { 258.0 } else { 153.0 };
    let tf = Transform {
        translation: (frame.pos + Vec2::new(0.0, -window.height() * 0.5 + 8.0 + y * ui) * frame.scale).extend(Z),
        scale: Vec3::splat(frame.scale * ui), ..default()
    };
    for (mut sprite, mut transform, mut visibility) in &mut plates {
        sprite.custom_size = Some(Vec2::new(584.0, 14.0 + copy.lines().count() as f32 * 16.0));
        *transform = tf;
        *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
    }
    for (mut text, mut transform, mut visibility) in &mut captions {
        if text.0 != *copy { text.0.clone_from(&copy); }
        *transform = tf;
        transform.translation.z += 0.5;
        *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
    }
}
