//! Keyboard → Action verbs. Exploration holds walk through the pacemaker;
//! combat accepts one fresh press per round, never render-frame repeats.
//!
//! Modal parity (E6): while `game.modal != None`, every key forwards through
//! [`crate::input::modal_keys`] into the TUI's own `laya_realms::input::key`
//! — the sim owns all modal semantics (menus, creation, oaths, travel). With
//! no modal, the view keeps movement + Enter/X/C/Space/Shift+A pacing and
//! forwards the modal openers (Esc/F/I/B/M/T/Y/?/J/[/]/digits/Ctrl+C).
//!
//! Enter (or Shift+A) strikes; movement presses choose a direction in combat.

use bevy::prelude::*;
use crossterm::event::{KeyCode as CrossKey, KeyEvent, KeyModifiers};
use laya_realms::model::{Action, Modal};

use crate::sim::SimSlot;

/// (key, dx, dy) for all movement bindings, in scan order. Cardinal + diagonal.
/// Shared with the pointer lane (E6): any of these keys cancels click-to-move.
pub const DIRS: &[(KeyCode, i32, i32)] = &[
    (KeyCode::ArrowUp, 0, -1),
    (KeyCode::KeyW, 0, -1),
    (KeyCode::Numpad8, 0, -1),
    (KeyCode::ArrowDown, 0, 1),
    (KeyCode::KeyS, 0, 1),
    (KeyCode::Numpad2, 0, 1),
    (KeyCode::ArrowLeft, -1, 0),
    (KeyCode::KeyA, -1, 0),
    (KeyCode::Numpad4, -1, 0),
    (KeyCode::ArrowRight, 1, 0),
    (KeyCode::KeyD, 1, 0),
    (KeyCode::Numpad6, 1, 0),
    (KeyCode::Home, -1, -1),
    (KeyCode::Numpad7, -1, -1),
    (KeyCode::PageUp, 1, -1),
    (KeyCode::Numpad9, 1, -1),
    (KeyCode::End, -1, 1),
    (KeyCode::Numpad1, -1, 1),
    (KeyCode::PageDown, 1, 1),
    (KeyCode::Numpad3, 1, 1),
];

#[derive(Default, Resource)]
pub struct InputState {
    /// Most recently pressed movement key still held — gives continuous walk.
    held: Option<(KeyCode, i32, i32)>,
    /// One buffered move, exactly like `buffered_move` in src/main.rs.
    buffered: Option<(i32, i32)>,
    was_combat: bool,
    /// Shared by keyboard and pointer lanes: one deliberate input per frame.
    pub(crate) frame_consumed: bool,
}

impl InputState {
    /// Currently held movement direction (camera anticipation reads this;
    /// autowalk checks it to cede to the keyboard lane).
    pub fn held_dir(&self) -> Option<(i32, i32)> {
        self.held.map(|(_, dx, dy)| (dx, dy))
    }
}

impl InputState {
    /// A held movement key counts as move intent (pointer.rs autowalk halt).
    pub fn held_move(&self) -> bool {
        self.held.is_some()
    }
}

pub fn movement(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<InputState>, mut slot: ResMut<SimSlot>) {
    move_keys(&keys, &mut input, &mut slot.game);
}

fn move_keys(keys: &ButtonInput<KeyCode>, input: &mut InputState, game: &mut laya_realms::model::Game) {
    let combat = game.combat.is_some();
    if combat != input.was_combat {
        input.held = None;
        input.buffered = None;
        input.was_combat = combat;
    }
    if input.frame_consumed || !matches!(game.modal, Modal::None) {
        input.held = None;
        input.buffered = None;
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let pressed = if shift {
        None
    } else {
        DIRS.iter().rev().find(|(code, _, _)| keys.just_pressed(*code)).copied()
    };
    if combat {
        input.held = None;
        input.buffered = None;
        if let Some((_, dx, dy)) = pressed {
            game.action(Action::Move(dx, dy));
            input.frame_consumed = true;
        }
        return;
    }
    if let Some(direction) = pressed {
        input.held = Some(direction);
    }
    if shift || input.held.is_some_and(|(code, _, _)| !keys.pressed(code)) {
        input.held = None;
    }
    if let Some((_, dx, dy)) = input.held {
        if game.elapsed_ms < game.move_ready_ms {
            input.buffered = Some((dx, dy));
        } else {
            input.buffered = None;
            game.action(Action::Move(dx, dy));
            input.frame_consumed = true;
        }
    } else if game.elapsed_ms >= game.move_ready_ms {
        if let Some((dx, dy)) = input.buffered.take() {
            game.action(Action::Move(dx, dy));
            input.frame_consumed = true;
        }
    }
    // A walking press may itself join combat or open a modal.
    if game.combat.is_some() || !matches!(game.modal, Modal::None) {
        input.held = None;
        input.buffered = None;
        input.was_combat = game.combat.is_some();
    }
}

pub fn verbs(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<InputState>,
    mut slot: ResMut<SimSlot>,
) {
    let game = &mut slot.game;
    if input.frame_consumed || !matches!(game.modal, Modal::None) {
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let action = if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || (shift && keys.just_pressed(KeyCode::KeyA))
    {
        Some(Action::Attack)
    } else if keys.just_pressed(KeyCode::KeyX) {
        Some(Action::Flee)
    } else if !ctrl && keys.just_pressed(KeyCode::KeyC) {
        Some(Action::Mercy)
    } else if keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Period)
        || keys.just_pressed(KeyCode::Numpad5)
    {
        Some(Action::Wait)
    } else {
        None
    };
    if let Some(action) = action {
        input.held = None;
        input.buffered = None;
        input.frame_consumed = true;
        game.action(action);
    }
}

/// Keys the view itself does not consume but `laya_realms::input::key`
/// handles with the road open (modal openers + global toggles). Movement
/// (DIRS), Enter, Space/Period/Numpad5, X, C, Shift+A stay view-owned.
fn is_opener(code: KeyCode, shift: bool, ctrl: bool) -> bool {
    match code {
        KeyCode::Escape
        | KeyCode::KeyE // interact — talks, trades, work benches, doors
        | KeyCode::KeyR // rest (inn / shrine per sim)
        | KeyCode::KeyF
        | KeyCode::KeyI
        | KeyCode::KeyB
        | KeyCode::KeyM
        | KeyCode::KeyT
        | KeyCode::KeyY
        | KeyCode::KeyJ
        | KeyCode::BracketLeft
        | KeyCode::BracketRight
        | KeyCode::Digit0
        | KeyCode::Digit1
        | KeyCode::Digit2
        | KeyCode::Digit3
        | KeyCode::Digit4
        | KeyCode::Digit5
        | KeyCode::Digit6
        | KeyCode::Digit7
        | KeyCode::Digit8
        | KeyCode::Digit9 => true,
        KeyCode::Slash => shift, // '?'
        KeyCode::KeyC => ctrl,   // Ctrl+C quits
        _ => false,
    }
}

/// Bevy KeyCode → the crossterm key event the TUI's input::key consumes:
/// the one translation seam of the E6 modal-input parity.
fn to_crossterm(code: KeyCode, shift: bool, ctrl: bool) -> Option<KeyEvent> {
    let modifiers = if ctrl {
        KeyModifiers::CONTROL
    } else {
        KeyModifiers::NONE
    };
    let cross = match code {
        KeyCode::ArrowUp => CrossKey::Up,
        KeyCode::ArrowDown => CrossKey::Down,
        KeyCode::ArrowLeft => CrossKey::Left,
        KeyCode::ArrowRight => CrossKey::Right,
        KeyCode::Home => CrossKey::Home,
        KeyCode::End => CrossKey::End,
        KeyCode::PageUp => CrossKey::PageUp,
        KeyCode::PageDown => CrossKey::PageDown,
        KeyCode::Enter => CrossKey::Enter,
        KeyCode::NumpadEnter => CrossKey::Enter,
        KeyCode::Escape => CrossKey::Esc,
        KeyCode::Space => CrossKey::Char(' '),
        KeyCode::Period => CrossKey::Char('.'),
        KeyCode::Comma => CrossKey::Char(','),
        KeyCode::Slash => CrossKey::Char(if shift { '?' } else { '/' }),
        KeyCode::BracketLeft => CrossKey::Char('['),
        KeyCode::BracketRight => CrossKey::Char(']'),
        KeyCode::Minus => CrossKey::Char('-'),
        KeyCode::Equal => CrossKey::Char('='),
        KeyCode::Semicolon => CrossKey::Char(';'),
        KeyCode::Quote => CrossKey::Char('\''),
        KeyCode::Backquote => CrossKey::Char('`'),
        KeyCode::Backslash => CrossKey::Char('\\'),
        KeyCode::Digit0 => CrossKey::Char('0'),
        KeyCode::Digit1 => CrossKey::Char('1'),
        KeyCode::Digit2 => CrossKey::Char('2'),
        KeyCode::Digit3 => CrossKey::Char('3'),
        KeyCode::Digit4 => CrossKey::Char('4'),
        KeyCode::Digit5 => CrossKey::Char('5'),
        KeyCode::Digit6 => CrossKey::Char('6'),
        KeyCode::Digit7 => CrossKey::Char('7'),
        KeyCode::Digit8 => CrossKey::Char('8'),
        KeyCode::Digit9 => CrossKey::Char('9'),
        KeyCode::Numpad0 => CrossKey::Char('0'),
        KeyCode::Numpad1 => CrossKey::Char('1'),
        KeyCode::Numpad2 => CrossKey::Char('2'),
        KeyCode::Numpad3 => CrossKey::Char('3'),
        KeyCode::Numpad4 => CrossKey::Char('4'),
        KeyCode::Numpad5 => CrossKey::Char('5'),
        KeyCode::Numpad6 => CrossKey::Char('6'),
        KeyCode::Numpad7 => CrossKey::Char('7'),
        KeyCode::Numpad8 => CrossKey::Char('8'),
        KeyCode::Numpad9 => CrossKey::Char('9'),
        _ => {
            const LETTERS: &[(KeyCode, char)] = &[
                (KeyCode::KeyA, 'a'), (KeyCode::KeyB, 'b'), (KeyCode::KeyC, 'c'),
                (KeyCode::KeyD, 'd'), (KeyCode::KeyE, 'e'), (KeyCode::KeyF, 'f'),
                (KeyCode::KeyG, 'g'), (KeyCode::KeyH, 'h'), (KeyCode::KeyI, 'i'),
                (KeyCode::KeyJ, 'j'), (KeyCode::KeyK, 'k'), (KeyCode::KeyL, 'l'),
                (KeyCode::KeyM, 'm'), (KeyCode::KeyN, 'n'), (KeyCode::KeyO, 'o'),
                (KeyCode::KeyP, 'p'), (KeyCode::KeyQ, 'q'), (KeyCode::KeyR, 'r'),
                (KeyCode::KeyS, 's'), (KeyCode::KeyT, 't'), (KeyCode::KeyU, 'u'),
                (KeyCode::KeyV, 'v'), (KeyCode::KeyW, 'w'), (KeyCode::KeyX, 'x'),
                (KeyCode::KeyY, 'y'), (KeyCode::KeyZ, 'z'),
            ];
            let Some((_, letter)) = LETTERS.iter().find(|(k, _)| *k == code) else {
                return None;
            };
            CrossKey::Char(if shift {
                letter.to_ascii_uppercase()
            } else {
                *letter
            })
        }
    };
    Some(KeyEvent::new(cross, modifiers))
}

/// Modal-input parity: while a modal is open, every translatable key goes
/// through the TUI's own `input::key` (menu nav, confirms, modal-local
/// shortcuts — the sim owns them all). With the road open, only the modal
/// openers forward; the view keeps its movement pacing and combat verbs.
pub fn modal_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<InputState>,
    mut slot: ResMut<SimSlot>,
) {
    input.frame_consumed = false;
    let game = &mut slot.game;
    let modal_open = !matches!(game.modal, Modal::None);
    if modal_open {
        input.held = None;
        input.buffered = None;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    for code in keys.get_just_pressed().copied() {
        if !modal_open && !is_opener(code, shift, ctrl) {
            continue;
        }
        if let Some(event) = to_crossterm(code, shift, ctrl) {
            input.held = None;
            input.buffered = None;
            input.frame_consumed = true;
            laya_realms::input::key(game, event);
            break; // a confirm/close must not feed another key into the road
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laya_realms::model::{Archetype, Game, Intent, Map, MapKind, Pos, Tile};

    fn app(combat: bool) -> App {
        let mut game = Game::new(42);
        game.modal = Modal::None;
        game.maps[0] = Map::new("Input arena", MapKind::Arena, 30, 30, Tile::Floor);
        game.player.map = 0;
        game.player.pos = Pos::new(10, 10);
        game.npcs.clear();
        if combat {
            add_enemy(&mut game);
        }
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<InputState>()
            .insert_resource(SimSlot { game, frame: 0, sim_ms: 0.0, shot: None })
            .add_systems(Update, (modal_keys, verbs, movement).chain());
        app
    }

    fn add_enemy(game: &mut Game) {
        let mut npc = laya_realms::world::make_npc(
            0, Archetype::Rat, 0, game.player.pos.offset(1, 0), 1, 1, 42,
        );
        npc.hp = 500;
        npc.max_hp = 500;
        npc.attack = 1;
        npc.intent = Intent::Attack;
        game.npcs.push(npc);
        game.refresh_combat();
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    }

    #[test]
    fn held_combat_direction_spends_one_round_until_repressed() {
        let mut app = app(true);
        press(&mut app, KeyCode::KeyD);
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(app.world().resource::<SimSlot>().game.turn, 1);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyD);
        press(&mut app, KeyCode::KeyD);
        assert_eq!(app.world().resource::<SimSlot>().game.turn, 2);
    }

    #[test]
    fn walking_holds_continue_but_combat_entry_requires_new_press() {
        let mut app = app(false);
        press(&mut app, KeyCode::KeyD);
        app.world_mut().resource_mut::<SimSlot>().game.elapsed_ms += 1000;
        app.update();
        assert_eq!(app.world().resource::<SimSlot>().game.player.pos, Pos::new(12, 10));
        add_enemy(&mut app.world_mut().resource_mut::<SimSlot>().game);
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(app.world().resource::<SimSlot>().game.turn, 0);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyD);
        press(&mut app, KeyCode::KeyD);
        assert_eq!(app.world().resource::<SimSlot>().game.turn, 1);
    }

    #[test]
    fn modal_close_does_not_resume_held_or_buffered_movement() {
        let mut app = app(false);
        press(&mut app, KeyCode::KeyW);
        app.update();
        app.world_mut().resource_mut::<SimSlot>().game.modal = Modal::Inventory;
        let origin = app.world().resource::<SimSlot>().game.player.pos;
        press(&mut app, KeyCode::Escape);
        app.world_mut().resource_mut::<SimSlot>().game.elapsed_ms += 1000;
        app.update();
        assert!(matches!(app.world().resource::<SimSlot>().game.modal, Modal::None));
        assert_eq!(app.world().resource::<SimSlot>().game.player.pos, origin);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyW);
        press(&mut app, KeyCode::KeyW);
        assert_eq!(app.world().resource::<SimSlot>().game.player.pos, origin.offset(0, -1));
    }

    #[test]
    fn simultaneous_attack_and_movement_commit_only_one_round() {
        let mut app = app(true);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyD);
        press(&mut app, KeyCode::Enter);
        let game = &app.world().resource::<SimSlot>().game;
        assert_eq!(game.turn, 1);
        assert_eq!(game.player.history.attacks, 1);
    }
}
