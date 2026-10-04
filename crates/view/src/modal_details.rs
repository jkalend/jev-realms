//! Equipment paper-doll and illustrated creation cards on the shared pane.
//! These are presentation controls; gameplay actions stay in core's modal input.
use bevy::prelude::*;
use bevy::text::FontSize;
use laya_realms::model::{Boon, Build, Class, Modal};

use crate::actors::player_plate_keys;
use crate::atlas::{self, Atlas};
use crate::camera::CamFrame;
use crate::modals::ModalBg;
use crate::sim::SimSlot;

const Z: f32 = 223.0;
const GOLD: Color = Color::srgb_u8(198, 157, 83);
const PALE: Color = Color::srgb_u8(232, 217, 182);
const BLUE: Color = Color::srgb_u8(133, 174, 181);
const DIM: Color = Color::srgb_u8(150, 145, 128);
const CARD: Color = Color::srgba(0.08, 0.10, 0.12, 0.98);
const SELECTED: Color = Color::srgba(0.30, 0.24, 0.14, 0.98);

#[derive(Component)]
pub(crate) struct Card(usize);
#[derive(Component)]
pub(crate) struct Label(usize);
#[derive(Component)]
pub(crate) struct Portrait;
#[derive(Component)]
pub(crate) struct PortraitRelic;
#[derive(Component)]
pub(crate) struct Preview(usize);

fn at(frame: &CamFrame, pos: Vec2, z: f32) -> Transform {
    Transform {
        translation: (frame.pos + pos * frame.scale).extend(z),
        scale: Vec3::new(frame.scale, frame.scale, 1.0),
        ..default()
    }
}

fn card(plates: &mut Vec<(Vec2, Vec2, Color)>, x: f32, y: f32, w: f32, h: f32, picked: bool) {
    plates.push((Vec2::new(x, y), Vec2::new(w, h), if picked { SELECTED } else { CARD }));
}
fn label(texts: &mut Vec<(Vec2, String, Color, f32)>, x: f32, y: f32, text: impl Into<String>, color: Color, size: f32) {
    texts.push((Vec2::new(x, y), text.into(), color, size));
}

fn choice_rect(pw: f32, ph: f32, step: u8, index: usize) -> Option<(Vec2, Vec2)> {
    match step {
        0 if index < 6 => {
            // Each column is one calling: Ward, Hunt, Speaker.
            let width = ((pw - 64.0) / 3.0 - 12.0).min(234.0);
            let x = (index / 2) as f32 * (width + 12.0) - width - 12.0;
            let y = ph * 0.5 - 160.0 - (index % 2) as f32 * 116.0;
            Some((Vec2::new(x, y), Vec2::new(width, 104.0)))
        }
        1 if index < 2 => {
            let width = ((pw - 66.0) / 2.0 - 12.0).min(300.0);
            let x = (index as f32 - 0.5) * (width + 16.0);
            Some((Vec2::new(x, ph * 0.5 - 202.0), Vec2::new(width, 206.0)))
        }
        2 if index < 3 => {
            let width = ((pw - 64.0) / 3.0 - 12.0).min(228.0);
            let x = (index as f32 - 1.0) * (width + 12.0);
            Some((Vec2::new(x, ph * 0.5 - 202.0), Vec2::new(width, 190.0)))
        }
        _ => None,
    }
}

// Copy is from the model, not a parallel set of display strings. Keep every
// complete word in the pane when a narrow viewport squeezes the caption.
fn caption_lines(
    texts: &mut Vec<(Vec2, String, Color, f32)>,
    copy: &str,
    x: f32,
    top: f32,
    width: f32,
    size: f32,
    tint: Color,
) -> f32 {
    let max_chars = (width / (size * 0.58)).floor().max(12.0) as usize;
    let mut line = String::new();
    let mut y = top;
    for word in copy.split_whitespace() {
        let word = if word == "—" { "-" } else { word }; // Default body font has no em dash.
        if !line.is_empty() && line.len() + 1 + word.len() > max_chars {
            label(texts, x, y, std::mem::take(&mut line), tint, size);
            y -= size + 3.0;
        }
        if !line.is_empty() { line.push(' '); }
        line.push_str(word);
    }
    if !line.is_empty() {
        label(texts, x, y, line, tint, size);
        y -= size + 3.0;
    }
    y
}

/// Creation cards are real controls, not screenshots. A click uses the same
/// Enter verb as the keyboard after selecting the clicked option.
pub(crate) fn clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    panes: Query<&Sprite, With<ModalBg>>,
    mut slot: ResMut<SimSlot>,
    mut input: ResMut<crate::input::InputState>,
) {
    if !buttons.just_pressed(MouseButton::Left) || input.frame_consumed || !matches!(slot.game.modal, Modal::Create) {
        return;
    }
    let Ok(window) = windows.single() else { return; };
    let Some(cursor) = window.cursor_position() else { return; };
    let Some(pane) = panes.iter().next() else { return; };
    let pw = crate::modals::pane_width(&slot.game.modal, slot.game.create_step, window.width());
    let ph = pane.custom_size.map(|s| s.y + if pane.rect.is_some() { 64.0 } else { 0.0 }).unwrap_or(490.0);
    let point = Vec2::new(cursor.x - window.width() * 0.5, window.height() * 0.5 - cursor.y);
    if (point.x - (-pw * 0.5 + 58.0)).abs() <= 42.0
        && (point.y - (-ph * 0.5 + 17.0)).abs() <= 13.0 {
        laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE,
        ));
        input.frame_consumed = true;
        return;
    }
    for index in 0..6 {
        let Some((center, size)) = choice_rect(pw, ph, slot.game.create_step, index) else { break };
        if (point.x - center.x).abs() <= size.x * 0.5 && (point.y - center.y).abs() <= size.y * 0.5 {
            slot.game.selected = index;
            laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE,
            ));
            input.frame_consumed = true;
            break;
        }
    }
}

pub(crate) fn render(
    mut commands: Commands,
    slot: Res<SimSlot>,
    frame: Res<CamFrame>,
    panes: Query<&Sprite, With<ModalBg>>,
    windows: Query<&Window>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    asset_server: Res<AssetServer>,
    mut spawned: Local<bool>,
    mut boon_icons: Local<Option<Handle<Image>>>,
    mut cards: Query<(&Card, &mut Transform, &mut Sprite, &mut Visibility), (Without<Portrait>, Without<PortraitRelic>, Without<Preview>, Without<ModalBg>)>,
    mut labels: Query<(&Label, &mut Transform, &mut Text2d, &mut TextFont, &mut TextColor, &mut Visibility), (Without<Card>, Without<Portrait>, Without<PortraitRelic>, Without<Preview>)>,
    mut portrait: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<Portrait>, Without<PortraitRelic>, Without<Preview>, Without<ModalBg>)>,
    mut relic_portrait: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<PortraitRelic>, Without<Preview>, Without<ModalBg>)>,
    mut previews: Query<(&Preview, &mut Transform, &mut Sprite, &mut Visibility), (With<Preview>, Without<ModalBg>)>,
) {
    let game = &slot.game;
    if !*spawned {
        if !matches!(game.modal, Modal::Inventory | Modal::Create) { return; }
        for index in 0..36 {
            commands.spawn((Card(index), Sprite::from_color(CARD, Vec2::ONE), Transform::default(), Visibility::Hidden));
        }
        for index in 0..28 {
            commands.spawn((Label(index), Text2d::new(""), TextFont::from_font_size(13.0), TextColor(PALE), Transform::default(), Visibility::Hidden));
        }
        commands.spawn((Portrait, Sprite::default(), Transform::default(), Visibility::Hidden));
        commands.spawn((PortraitRelic, Sprite::default(), Transform::default(), Visibility::Hidden));
        for index in 0..6 {
            commands.spawn((Preview(index), Sprite::default(), Transform::default(), Visibility::Hidden));
        }
        *spawned = true;
        return;
    }
    if !matches!(game.modal, Modal::Inventory | Modal::Create) {
        for (_, _, _, mut visibility) in &mut cards { *visibility = Visibility::Hidden; }
        for (_, _, _, _, _, mut visibility) in &mut labels { *visibility = Visibility::Hidden; }
        for (_, _, mut visibility) in &mut portrait { *visibility = Visibility::Hidden; }
        for (_, _, mut visibility) in &mut relic_portrait { *visibility = Visibility::Hidden; }
        for (_, _, _, mut visibility) in &mut previews { *visibility = Visibility::Hidden; }
        return;
    }
    let Some(pane) = panes.iter().next() else { return; };
    let Some(window) = windows.iter().next() else { return; };
    let ph = pane.custom_size.map(|s| s.y + if pane.rect.is_some() { 64.0 } else { 0.0 }).unwrap_or(490.0);
    let pw = crate::modals::pane_width(&game.modal, game.create_step, window.width());
    let pointer = window.cursor_position().map(|p| Vec2::new(p.x - window.width() * 0.5, window.height() * 0.5 - p.y));
    let mut plates = Vec::with_capacity(8);
    let mut texts = Vec::with_capacity(28);
    let mut doll = None;
    let mut relic = None;
    let doll_pos = Vec2::new(-pw * 0.5 + 140.0, ph * 0.5 - 148.0);
    let doll_size = Vec2::new(100.0, 127.0);
    match game.modal {
        Modal::Inventory => {
            let x = -pw * 0.5 + 140.0;
            label(&mut texts, x, ph * 0.5 - 77.0, "EQUIPPED / WAYFARER", GOLD, 12.5);
            label(&mut texts, x, ph * 0.5 - 220.0, format!("{} / {}", game.player.class.name(), game.player.build.name()), PALE, 12.0);
            let gear = [
                ("WEAPON", format!("{} blade  /  +{} ATK", laya_realms::model::Item::tier(game.player.weapon), game.player.weapon as i32 * 3)),
                ("ARMOUR", format!("{} mail  /  +{} DEF", laya_realms::model::Item::tier(game.player.armour), game.player.armour as i32 * 2)),
                ("RELIC", game.player.relic.map(|r| r.name().to_owned()).unwrap_or_else(|| "Unbound".into())),
            ];
            for (index, (name, value)) in gear.into_iter().enumerate() {
                let y = ph * 0.5 - 264.0 - index as f32 * 52.0;
                card(&mut plates, x, y, 225.0, 45.0, false);
                label(&mut texts, x, y + 10.0, name, GOLD, 10.0);
                label(&mut texts, x, y - 9.0, value, PALE, 12.0);
            }
            let keys = player_plate_keys(game.player.class, game.player.build, game.player.weapon, game.player.armour);
            doll = atlas.as_deref().zip(images.as_deref()).and_then(|(a, i)| keys.iter().find_map(|key| atlas::actor_ref(a, i, key).map(|(image, rect)| (image.clone(), rect))));
            if let Some(relic_name) = game.player.relic {
                relic = atlas.as_deref().zip(images.as_deref()).and_then(|(a, i)| {
                    atlas::actor_ref(a, i, &crate::actors::relic_key(relic_name, game.player.build))
                        .map(|(image, rect)| (image.clone(), rect))
                });
            }
        }
        Modal::Create => {
            let step = game.create_step;
            let (count, heading) = match step {
                0 => (6, "THE SIX ORDERS   /   THREE CALLINGS"),
                1 => (2, "ONE OATH   /   TWO FRAMES"),
                _ => (3, "THREE GIFTS   /   ONE ROAD"),
            };
            label(&mut texts, 0.0, ph * 0.5 - 83.0, heading, GOLD, 12.5);
            for index in 0..count {
                let (pos, size) = choice_rect(pw, ph, step, index).expect("bounded by count");
                let (x, y) = (pos.x, pos.y);
                let picked = game.selected == index;
                let hovered = pointer.is_some_and(|p| (p.x - x).abs() <= size.x * 0.5 && (p.y - y).abs() <= size.y * 0.5);
                let accent = if picked { GOLD } else if step == 2 { BLUE } else {
                    match if step == 0 { index / 2 } else { index } {
                        0 => GOLD,
                        1 => BLUE,
                        _ => Color::srgb_u8(147, 169, 132),
                    }
                };
                // A fine illuminated edge rather than a solid selected block.
                plates.push((pos, size, if picked || hovered { GOLD } else { Color::srgb_u8(55, 65, 68) }));
                plates.push((pos, size - Vec2::splat(2.0), if hovered { Color::srgb_u8(37, 51, 52) } else if picked { SELECTED } else { CARD }));
                plates.push((Vec2::new(x, y + size.y * 0.5 - 2.0), Vec2::new(size.x - 2.0, 3.0), accent));
                match step {
                    0 => {
                        let class = Class::from_index(index);
                        let text_x = x + if size.x < 175.0 { 28.0 } else { 34.0 };
                        label(&mut texts, text_x, y + 22.0, class.calling().to_uppercase(), accent, 10.0);
                        label(&mut texts, text_x, y - 5.0, class.name(), PALE, if size.x < 175.0 { 11.5 } else { 13.5 });
                        label(&mut texts, text_x, y - 30.0, if picked { "CHOSEN ORDER" } else { "VIEW ORDER" }, DIM, 9.5);
                    }
                    1 => {
                        let build = Build::from_index(index);
                        label(&mut texts, x, y - 67.0, build.name(), PALE, 17.0);
                        label(&mut texts, x, y - 88.0, if picked { "FRAME CHOSEN" } else { "SELECT FRAME" }, accent, 10.0);
                    }
                    _ => {
                        let boon = Boon::from_index(index);
                        label(&mut texts, x, y - 65.0, boon.name(), PALE, if size.x < 175.0 { 13.0 } else { 15.0 });
                        let effect = if matches!(boon, Boon::None) {
                            "NO BONUS"
                        } else {
                            boon.describe().split('—').next().unwrap_or(boon.describe()).trim()
                        };
                        label(&mut texts, x, y - 85.0, effect.to_uppercase(), accent, 9.5);
                    }
                }
            }
            let selection = game.selected.min(count - 1);
            let caption_y = if step == 0 { ph * 0.5 - 357.0 } else { ph * 0.5 - 326.0 };
            let caption_width = pw - 94.0;
            let caption_size = if pw < 620.0 { 11.5 } else { 13.0 };
            let (eyebrow, name, description, effect) = match step {
                0 => {
                    let class = Class::from_index(selection);
                    (class.calling().to_uppercase(), class.name(), class.blurb(), Some(class.diff()))
                }
                1 => {
                    let build = Build::from_index(selection);
                    ("THE CHOSEN FRAME".to_owned(), build.name(), build.note(), None)
                }
                _ => {
                    let boon = Boon::from_index(selection);
                    ("THE ROAD'S OFFERING".to_owned(), boon.name(), boon.describe(), None)
                }
            };
            label(&mut texts, 0.0, caption_y, format!("{}   /   {}", eyebrow, name.to_uppercase()), GOLD, 12.0);
            let mut next = caption_lines(&mut texts, description, 0.0, caption_y - 24.0, caption_width, caption_size, PALE);
            if let Some(effect) = effect {
                next = caption_lines(&mut texts, effect, 0.0, next - 5.0, caption_width, caption_size, BLUE);
            }
            label(&mut texts, 0.0, (next - 8.0).max(-ph * 0.5 + 17.0), "Arrows choose   Enter confirm", DIM, 10.0);
            card(&mut plates, -pw * 0.5 + 58.0, -ph * 0.5 + 17.0, 84.0, 26.0, false);
            label(&mut texts, -pw * 0.5 + 58.0, -ph * 0.5 + 17.0, "Back   Esc", GOLD, 10.0);
        }
        _ => unreachable!(),
    }
    for (item, mut tf, mut sprite, mut visibility) in &mut cards {
        if let Some((pos, size, color)) = plates.get(item.0) {
            *tf = at(&frame, *pos, Z - 0.45 + if matches!(game.modal, Modal::Create) { item.0 as f32 * 0.001 } else { 0.0 });
            sprite.custom_size = Some(*size);
            sprite.color = *color;
            *visibility = Visibility::Visible;
        } else { *visibility = Visibility::Hidden; }
    }
    for (item, mut tf, mut text, mut font, mut color, mut visibility) in &mut labels {
        if let Some((pos, value, tint, size)) = texts.get(item.0) {
            *tf = at(&frame, *pos, Z + 0.3);
            if text.0 != *value { text.0.clone_from(value); }
            if font.font_size != FontSize::Px(*size) { font.font_size = FontSize::Px(*size); }
            color.0 = *tint;
            *visibility = Visibility::Visible;
        } else { *visibility = Visibility::Hidden; }
    }
    for (preview, mut tf, mut sprite, mut visibility) in &mut previews {
        let image = if matches!(game.modal, Modal::Create) {
            choice_rect(pw, ph, game.create_step, preview.0).and_then(|(pos, size)| {
                match game.create_step {
                    0 | 1 => {
                        let class = if game.create_step == 0 { Class::from_index(preview.0) } else { Class::from_index(game.create_class) };
                        let build = if game.create_step == 0 { Build::Male } else { Build::from_index(preview.0) };
                        let keys = player_plate_keys(class, build, 0, 0);
                        atlas.as_deref().zip(images.as_deref()).and_then(|(a, i)| {
                            keys.iter().find_map(|key| atlas::actor_ref(a, i, key))
                                .map(|(image, rect)| (pos, size, image.clone(), rect, if game.create_step == 0 { 86.0 } else { 146.0 }))
                        })
                    }
                    _ => {
                        let sheet = boon_icons.get_or_insert_with(|| asset_server.load("ui/boon-icons.png"));
                        let left = preview.0 as f32 * 128.0;
                        Some((pos, size, sheet.clone(), Rect::new(left, 0.0, left + 128.0, 128.0), 116.0))
                    }
                }
            })
        } else { None };
        if let Some((pos, size, image, rect, height)) = image {
            let image_pos = match game.create_step {
                0 => Vec2::new(pos.x - size.x * 0.5 + 32.0, pos.y),
                1 => Vec2::new(pos.x, pos.y + 25.0),
                _ => Vec2::new(pos.x, pos.y + 28.0),
            };
            *tf = at(&frame, image_pos, Z + 0.1);
            sprite.image = image;
            sprite.rect = Some(rect);
            sprite.custom_size = Some(Vec2::new(height * rect.width() / rect.height(), height));
            sprite.color = Color::WHITE;
            *visibility = Visibility::Visible;
        } else { *visibility = Visibility::Hidden; }
    }
    for (mut tf, mut sprite, mut visibility) in &mut portrait {
        if let Some((image, rect)) = &doll {
            *tf = at(&frame, doll_pos, Z + 0.1);
            sprite.image = image.clone();
            sprite.rect = Some(*rect);
            sprite.custom_size = Some(Vec2::new(doll_size.y * rect.width() / rect.height(), doll_size.y));
            sprite.color = Color::WHITE;
            *visibility = Visibility::Visible;
        } else { *visibility = Visibility::Hidden; }
    }
    for (mut tf, mut sprite, mut visibility) in &mut relic_portrait {
        if let Some((image, rect)) = &relic {
            *tf = at(&frame, doll_pos, Z + 0.2);
            sprite.image = image.clone();
            sprite.rect = Some(*rect);
            sprite.custom_size = Some(Vec2::new(doll_size.y * rect.width() / rect.height(), doll_size.y));
            sprite.color = Color::WHITE;
            *visibility = Visibility::Visible;
        } else { *visibility = Visibility::Hidden; }
    }
}
