//! Portrait-led conversation. Response ordering and all consequences stay in core.
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use laya_realms::model::{Archetype, Modal};
use laya_realms::social;

use crate::atlas::{self, Atlas};
use crate::camera::CamFrame;
use crate::hud::DisplayFont;
use crate::modals::ModalBg;
use crate::sim::SimSlot;

const Z: f32 = 223.2;
const GOLD: Color = Color::srgb_u8(207, 173, 107);
const PALE: Color = Color::srgb_u8(233, 218, 184);
const MUTED: Color = Color::srgb_u8(154, 169, 164);
const WELL: Color = Color::srgb_u8(17, 28, 31);
const CARD: Color = Color::srgb_u8(22, 32, 35);
const PICKED: Color = Color::srgb_u8(64, 53, 34);
const HOVER: Color = Color::srgb_u8(37, 51, 52);
const MAX_VISIBLE: usize = 5;

#[derive(Resource, Default)]
pub(crate) struct ConversationView {
    npc: Option<usize>,
    start: usize,
    selected: usize,
}

impl ConversationView {
    fn reconcile(&mut self, npc: usize, selected: usize, total: usize, visible: usize) {
        if self.npc != Some(npc) {
            self.npc = Some(npc);
            self.start = 0;
            self.selected = usize::MAX;
        }
        if self.selected != selected {
            self.start = self.start.min(selected).max(selected.saturating_sub(visible - 1));
            self.selected = selected;
        }
        self.start = self.start.min(total.saturating_sub(visible));
    }
}

#[derive(Component)]
pub(crate) struct Plate(usize);
#[derive(Component)]
pub(crate) struct Caption(usize);
#[derive(Component)]
pub(crate) struct Face;

fn at(frame: &CamFrame, x: f32, y: f32, z: f32) -> Transform {
    Transform {
        translation: (frame.pos + Vec2::new(x, y) * frame.scale).extend(z),
        scale: Vec3::splat(frame.scale),
        ..default()
    }
}

fn wrapped(copy: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut line = 0;
    for word in copy.split_whitespace() {
        let length = word.chars().count();
        if line > 0 && line + length + 1 > limit {
            out.push('\n');
            line = 0;
        } else if line > 0 {
            out.push(' ');
            line += 1;
        }
        out.push_str(word);
        line += length;
    }
    out
}

fn wrapped_lines(copy: &str, limit: usize) -> usize {
    let mut lines = 1;
    let mut width = 0;
    for word in copy.split_whitespace() {
        let length = word.chars().count();
        if width > 0 && width + length + 1 > limit {
            lines += 1;
            width = length;
        } else {
            width += length + usize::from(width > 0);
        }
    }
    lines
}

struct Layout {
    portrait_x: f32,
    portrait_w: f32,
    x: f32,
    width: f32,
    first_y: f32,
    row_h: f32,
    visible: usize,
    feedback: String,
    feedback_y: f32,
    columns: usize,
}

fn layout(pw: f32, ph: f32, options: &[String], feedback: &str) -> Layout {
    let portrait_w = (pw * 0.24).min(180.0);
    let portrait_x = -pw * 0.5 + 24.0 + portrait_w * 0.5;
    let width = pw - portrait_w - 76.0;
    let x = pw * 0.5 - 24.0 - width * 0.5;
    let columns = ((width - 28.0) / 7.0).floor().max(12.0) as usize;
    let feedback = wrapped(feedback, columns);
    let feedback_h = feedback.lines().count().max(1) as f32 * 15.0 + 18.0;
    let row_h = options.iter().map(|s| wrapped_lines(s, columns)).max().unwrap_or(1) as f32 * 15.0 + 20.0;
    let first_y = ph * 0.5 - 138.0 - feedback_h - row_h * 0.5;
    let available = ph - 174.0 - feedback_h;
    let visible = ((available / row_h).floor() as usize).clamp(1, MAX_VISIBLE);
    Layout { portrait_x, portrait_w, x, width, first_y, row_h, visible, feedback,
        feedback_y: ph * 0.5 - 127.0 - feedback_h * 0.5, columns }
}

fn choice_at(layout: &Layout, total: usize, start: usize, point: Vec2) -> Option<usize> {
    for row in 0..total.saturating_sub(start).min(layout.visible) {
        let y = layout.first_y - row as f32 * layout.row_h;
        if (point.x - layout.x).abs() <= layout.width * 0.5
            && (point.y - y).abs() <= (layout.row_h - 4.0) * 0.5 {
            return Some(start + row);
        }
    }
    None
}

fn leave_at(pw: f32, ph: f32, point: Vec2) -> bool {
    (point.x - (pw * 0.5 - 58.0)).abs() <= 42.0
        && (point.y - (-ph * 0.5 + 21.0)).abs() <= 13.0
}

fn feedback(game: &laya_realms::model::Game) -> &str {
    game.log.back().map(String::as_str).unwrap_or("Choose what you would like to discuss.")
}

/// Hover never changes the keyboard selection. Wheel moves the viewport only.
pub(crate) fn clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    panes: Query<&Sprite, With<ModalBg>>,
    mut view: ResMut<ConversationView>,
    mut slot: ResMut<SimSlot>,
    mut input: ResMut<crate::input::InputState>,
) {
    let scroll: f32 = wheel.read().map(|event| event.y).sum();
    let Modal::Talk(id) = slot.game.modal else { view.npc = None; return; };
    if scroll == 0.0 && (!buttons.just_pressed(MouseButton::Left) || input.frame_consumed) { return; }
    let Ok(window) = windows.single() else { return; };
    let Some(pane) = panes.iter().next() else { return; };
    let pw = crate::modals::pane_width(&slot.game.modal, slot.game.create_step, window.width());
    let ph = pane.custom_size.map(|s| s.y).unwrap_or(560.0);
    let options = social::talk_options(&slot.game, id);
    let layout = layout(pw, ph, &options, feedback(&slot.game));
    view.reconcile(id, slot.game.selected, options.len(), layout.visible);
    let Some(cursor) = window.cursor_position() else { return; };
    let point = Vec2::new(cursor.x - window.width() * 0.5, window.height() * 0.5 - cursor.y);
    if point.x.abs() <= pw * 0.5 && point.y.abs() <= ph * 0.5 && scroll != 0.0 {
        view.start = if scroll > 0.0 { view.start.saturating_sub(1) }
            else { (view.start + 1).min(options.len().saturating_sub(layout.visible)) };
    }
    if !buttons.just_pressed(MouseButton::Left) || input.frame_consumed { return; }
    let index = if leave_at(pw, ph, point) { Some(options.len().saturating_sub(1)) }
        else { choice_at(&layout, options.len(), view.start, point) };
    if let Some(index) = index {
        slot.game.selected = index;
        laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE,
        ));
        input.frame_consumed = true;
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
    font: Option<Res<DisplayFont>>,
    mut view: ResMut<ConversationView>,
    mut spawned: Local<bool>,
    mut portrait_keys: Local<Option<(Archetype, String, String)>>,
    mut plates: Query<(&Plate, &mut Transform, &mut Sprite, &mut Visibility), (Without<Face>, Without<ModalBg>)>,
    mut captions: Query<(&Caption, &mut Transform, &mut Text2d, &mut TextFont, &mut TextColor, &mut Visibility), (Without<Plate>, Without<Face>)>,
    mut face: Query<(&mut Transform, &mut Sprite, &mut Visibility), (With<Face>, Without<Caption>, Without<ModalBg>)>,
) {
    let Modal::Talk(id) = slot.game.modal else {
        for (_, _, _, mut visibility) in &mut plates { *visibility = Visibility::Hidden; }
        for (_, _, _, _, _, mut visibility) in &mut captions { *visibility = Visibility::Hidden; }
        for (_, _, mut visibility) in &mut face { *visibility = Visibility::Hidden; }
        return;
    };
    let Some(npc) = slot.game.npcs.get(id) else { return; };
    if !*spawned {
        for i in 0..(4 + MAX_VISIBLE * 2) {
            commands.spawn((Plate(i), Sprite::from_color(CARD, Vec2::ONE), Transform::default(), Visibility::Hidden));
        }
        for i in 0..(6 + MAX_VISIBLE) {
            commands.spawn((Caption(i), Text2d::new(""), TextLayout::justify(Justify::Center), TextFont::from_font_size(13.0), TextColor(PALE), Transform::default(), Visibility::Hidden));
        }
        commands.spawn((Face, Sprite::default(), Transform::default(), Visibility::Hidden));
        *spawned = true;
        return;
    }
    let (Ok(window), Some(pane)) = (windows.single(), panes.iter().next()) else { return; };
    let pw = crate::modals::pane_width(&slot.game.modal, slot.game.create_step, window.width());
    let ph = pane.custom_size.map(|s| s.y).unwrap_or(560.0);
    let options = social::talk_options(&slot.game, id);
    let selected = slot.game.selected.min(options.len().saturating_sub(1));
    let layout = layout(pw, ph, &options, feedback(&slot.game));
    view.reconcile(id, selected, options.len(), layout.visible);
    let point = window.cursor_position().map(|p| Vec2::new(p.x - window.width() * 0.5, window.height() * 0.5 - p.y));
    let hovered = point.and_then(|p| choice_at(&layout, options.len(), view.start, p));
    let leave_hover = point.is_some_and(|p| leave_at(pw, ph, p));
    for (plate, mut tf, mut sprite, mut visibility) in &mut plates {
        let (x, y, size, color, z) = match plate.0 {
            0 => (layout.portrait_x, 0.0, Vec2::new(layout.portrait_w + 2.0, 234.0), GOLD, Z),
            1 => (layout.portrait_x, 0.0, Vec2::new(layout.portrait_w, 232.0), WELL, Z + 0.1),
            2 => (layout.x, layout.feedback_y, Vec2::new(layout.width, layout.feedback.lines().count().max(1) as f32 * 15.0 + 18.0), WELL, Z + 0.1),
            3 => (pw * 0.5 - 58.0, -ph * 0.5 + 21.0, Vec2::new(84.0, 26.0), if leave_hover { HOVER } else { CARD }, Z + 0.1),
            i => {
                let row = (i - 4) / 2;
                if row >= layout.visible || view.start + row >= options.len() {
                    *visibility = Visibility::Hidden;
                    continue;
                }
                let chosen = view.start + row == selected;
                let hover = hovered == Some(view.start + row);
                let border = (i - 4) % 2 == 0;
                (layout.x, layout.first_y - row as f32 * layout.row_h,
                    Vec2::new(layout.width - if border { 0.0 } else { 2.0 }, layout.row_h - if border { 2.0 } else { 4.0 }),
                    if border { if chosen || hover { GOLD } else { Color::srgb_u8(52, 67, 68) } }
                    else if hover { HOVER } else if chosen { PICKED } else { CARD },
                    if border { Z + 0.1 } else { Z + 0.2 })
            }
        };
        *tf = at(&frame, x, y, z);
        sprite.color = color;
        sprite.custom_size = Some(size);
        *visibility = Visibility::Visible;
    }
    for (caption, mut tf, mut text, mut text_font, mut color, mut visibility) in &mut captions {
        let (copy, x, y, size, tint) = match caption.0 {
            0 => (wrapped(&npc.name, layout.columns), layout.x, ph * 0.5 - 77.0, 22.0, PALE),
            1 => (format!("{}   {} regard", social::conversation_role(npc), social::conversation_regard(npc)), layout.x, ph * 0.5 - 108.0, 11.5, GOLD),
            2 => (layout.feedback.clone(), layout.x, layout.feedback_y, 12.5, PALE),
            3 => ("Arrows / W S choose   Enter reply".into(), 0.0, -ph * 0.5 + 21.0, 10.0, MUTED),
            4 => (if options.len() > layout.visible { format!("{} to {} of {}   scroll", view.start + 1, (view.start + layout.visible).min(options.len()), options.len()) } else { "Your response".into() }, layout.portrait_x, -ph * 0.5 + 21.0, 10.0, GOLD),
            5 => ("Leave   Esc".into(), pw * 0.5 - 58.0, -ph * 0.5 + 21.0, 11.0, GOLD),
            i => {
                let row = i - 6;
                if row >= layout.visible || view.start + row >= options.len() {
                    *visibility = Visibility::Hidden;
                    continue;
                }
                (wrapped(&options[view.start + row], layout.columns), layout.x,
                    layout.first_y - row as f32 * layout.row_h, 12.5,
                    if selected == view.start + row || hovered == Some(view.start + row) { PALE } else { MUTED })
            }
        };
        *tf = at(&frame, x, y, Z + 0.6);
        if text.0 != copy { text.0 = copy; }
        text_font.font_size = bevy::text::FontSize::Px(size);
        if caption.0 == 0 {
            text_font.font = font.as_deref().and_then(|f| f.0.clone()).map(bevy::text::FontSource::Handle).unwrap_or_default();
        }
        color.0 = tint;
        *visibility = Visibility::Visible;
    }
    if portrait_keys.as_ref().map(|(kind, _, _)| *kind) != Some(npc.archetype) {
        let key = format!("{:?}", npc.archetype);
        let idle = format!("{key}.idle.front.000");
        *portrait_keys = Some((npc.archetype, key, idle));
    }
    let (_, key, idle) = portrait_keys.as_ref().expect("portrait keys initialized");
    for (mut tf, mut sprite, mut visibility) in &mut face {
        if let Some((image, rect)) = atlas.as_deref().zip(images.as_deref()).and_then(|(a, i)| {
            atlas::actor_ref(a, i, idle).or_else(|| atlas::actor_ref(a, i, key))
        }) {
            sprite.image = image.clone();
            sprite.rect = Some(rect);
            sprite.custom_size = Some(Vec2::splat((layout.portrait_w - 18.0).min(222.0)));
            sprite.color = Color::WHITE;
            *tf = at(&frame, layout.portrait_x, 0.0, Z + 0.4);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_viewport_keeps_selection_and_keyboard_reveals_it() {
        let mut view = ConversationView::default();
        view.reconcile(1, 7, 10, 4);
        assert_eq!(view.start, 4);
        view.start = 0;
        view.reconcile(1, 7, 10, 4);
        assert_eq!(view.start, 0);
        view.reconcile(1, 8, 10, 4);
        assert_eq!(view.start, 5);
        view.reconcile(2, 0, 3, 4);
        assert_eq!(view.start, 0);
    }

    #[test]
    fn click_targets_visible_scrolled_responses_and_ignores_gaps() {
        let options = vec!["Ask about the road".into(); 10];
        let layout = layout(820.0, 560.0, &options, "They consider your request.");
        let p = Vec2::new(layout.x, layout.first_y);
        assert_eq!(choice_at(&layout, 10, 5, p), Some(5));
        assert_eq!(choice_at(&layout, 10, 5, p - Vec2::Y * layout.row_h), Some(6));
        assert_eq!(choice_at(&layout, 10, 5, p - Vec2::Y * layout.row_h * 0.5), None);
    }
}
