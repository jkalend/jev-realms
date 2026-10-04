//! Keyboard controls shared by the terminal and windowed views.

use crate::{
    model::*,
    persist::{save_game, try_load},
    social,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn active(game: &Game) -> bool {
    matches!(game.modal, Modal::None)
}

fn menu_len(game: &Game) -> usize {
    match game.modal {
        Modal::Inventory => game.player.inventory.len(),
        Modal::Trade(id) => social::stock(match game.maps[game.npcs[id].map].kind {
            MapKind::City(c) => c,
            _ => 0,
        })
        .len(),
        Modal::Talk(id) => social::talk_options(game, id).len(),
        Modal::Journal => game.quests.len(),
        Modal::Cast => game.player.spells.len(),
        Modal::Forge(_) => game.forge_options().len(),
        Modal::Oath => 3,
        Modal::Talents => 12,
        _ => 0,
    }
}

pub fn key(game: &mut Game, event: KeyEvent) {
    if event.kind == KeyEventKind::Release {
        return;
    }
    // Holding a key may walk on the road, but never commits repeated rounds.
    if event.kind == KeyEventKind::Repeat && game.combat.is_some() {
        return;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
        game.quit = true;
        return;
    }
    let code = event.code;
    if code == KeyCode::Char('j') || code == KeyCode::Char('J') {
        game.inspector = !game.inspector;
        game.selected = 0;
        return;
    }
    if active(game) && game.inspector {
        match code {
            KeyCode::Char('[') => {
                game.selected = game.selected.saturating_sub(1);
                return;
            }
            KeyCode::Char(']') => {
                game.selected = (game.selected + 1).min(game.decisions.len().saturating_sub(1));
                return;
            }
            _ => {}
        }
    }
    if matches!(game.modal, Modal::Title) {
        match code {
            KeyCode::Enter => {
                game.modal = Modal::Create;
                game.create_step = 0;
                game.selected = 0;
            }
            KeyCode::Char('l') => try_load(game),
            KeyCode::Char('?') => game.modal = Modal::Help,
            KeyCode::Char('q') | KeyCode::Esc => game.quit = true,
            _ => {}
        }
        return;
    }
    if matches!(game.modal, Modal::Create) {
        // Three beats: order (six, grouped by calling), body build, then the
        // boon (D2). The step count is mirrored in `ui.rs` and `view/modals.rs`.
        let len = match game.create_step {
            0 => 6,
            1 => 2,
            _ => 3,
        };
        match code {
            KeyCode::Up | KeyCode::Char('w') => {
                game.selected = (game.selected + len - 1) % len;
            }
            KeyCode::Down | KeyCode::Char('s') => {
                game.selected = (game.selected + 1) % len;
            }
            KeyCode::Enter => match game.create_step {
                0 => {
                    game.create_class = game.selected;
                    game.create_step = 1;
                    game.selected = game.create_build;
                }
                1 => {
                    game.create_build = game.selected;
                    game.create_step = 2;
                    game.selected = 0;
                }
                _ => {
                    game.apply_creation(
                        Class::from_index(game.create_class),
                        Build::from_index(game.create_build),
                        Boon::from_index(game.selected),
                    );
                    game.selected = 0;
                }
            },
            KeyCode::Esc => match game.create_step {
                0 => game.modal = Modal::Title,
                1 => {
                    game.create_step = 0;
                    game.selected = game.create_class;
                }
                _ => {
                    game.create_step = 1;
                    game.selected = game.create_build;
                }
            },
            _ => {}
        }
        return;
    }
    if matches!(game.modal, Modal::Death | Modal::Victory) {
        if matches!(code, KeyCode::Enter | KeyCode::Esc) {
            if matches!(game.modal, Modal::Death) {
                game.respawn();
            } else {
                game.modal = Modal::None;
            }
        }
        return;
    }
    if code == KeyCode::Esc {
        game.modal = if active(game) {
            Modal::Pause
        } else {
            Modal::None
        };
        game.selected = 0;
        return;
    }
    if code == KeyCode::Char('?') {
        game.modal = if matches!(game.modal, Modal::Help) {
            Modal::None
        } else {
            Modal::Help
        };
        return;
    }
    if matches!(game.modal, Modal::Pause) {
        if code == KeyCode::Char('q') {
            game.quit = true;
        } else if code == KeyCode::Enter {
            game.modal = Modal::None;
        } else if code == KeyCode::Char('s') {
            if game.combat.is_some() {
                game.log("No saving with blades drawn. Finish the fight or flee first.");
            } else {
                match save_game(game) {
                    Ok(()) => {
                        game.log("Journey saved. Same seed restores it from the title screen.");
                        game.quit = true;
                    }
                    Err(error) => game.log(format!("Save failed: {error}")),
                }
            }
        }
        return;
    }
    if !active(game) {
        let len = menu_len(game);
        match code {
            KeyCode::Up | KeyCode::Char('w') if matches!(game.modal, Modal::Talents) => {
                game.selected = (game.selected / 4) * 4 + (game.selected % 4 + 3) % 4;
            }
            KeyCode::Down | KeyCode::Char('s') if matches!(game.modal, Modal::Talents) => {
                game.selected = (game.selected / 4) * 4 + (game.selected % 4 + 1) % 4;
            }
            KeyCode::Left | KeyCode::Char('a') if matches!(game.modal, Modal::Talents) => {
                game.selected = ((game.selected / 4 + 2) % 3) * 4 + game.selected % 4;
            }
            KeyCode::Right | KeyCode::Char('d') if matches!(game.modal, Modal::Talents) => {
                game.selected = ((game.selected / 4 + 1) % 3) * 4 + game.selected % 4;
            }
            KeyCode::Up | KeyCode::Char('w') => {
                if len > 0 {
                    game.selected = (game.selected + len - 1) % len;
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if len > 0 {
                    game.selected = (game.selected + 1) % len;
                }
            }
            KeyCode::Enter => match game.modal.clone() {
                Modal::Inventory => game.action(Action::Use(game.selected)),
                Modal::Trade(_) => game.action(Action::Buy(game.selected)),
                Modal::Talk(_) => game.action(Action::Talk(game.selected)),
                Modal::Cast => {
                    let turn = game.turn;
                    game.action(Action::Cast(game.selected));
                    if game.turn != turn && matches!(game.modal, Modal::Cast) {
                        game.modal = Modal::None;
                    }
                }
                Modal::Forge(_) => game.action(Action::Forge(game.selected)),
                Modal::Oath => game.action(Action::OathPledge(game.selected)),
                Modal::Talents => {
                    game.action(Action::Learn(game.selected));
                    game.selected = game.selected.min(11);
                }
                _ => {}
            },
            KeyCode::Char('h') if matches!(game.modal, Modal::Trade(_)) => {
                game.action(Action::Haggle)
            }
            KeyCode::Char('q') if matches!(game.modal, Modal::Trade(_)) => {
                if let Modal::Trade(id) = game.modal {
                    game.modal = Modal::Talk(id);
                    game.selected = 1;
                }
            }
            KeyCode::Char('i') if matches!(game.modal, Modal::Trade(_)) => {
                game.modal = Modal::Inventory;
                game.selected = 0;
            }
            KeyCode::Char('v') if matches!(game.modal, Modal::Inventory) => {
                game.action(Action::Sell(game.selected))
            }
            KeyCode::Char(c @ '1'..='3') if matches!(game.modal, Modal::Atlas) => {
                game.action(Action::Travel(c as usize - '1' as usize))
            }
            KeyCode::Char('m') if matches!(game.modal, Modal::Atlas) => {
                game.modal = Modal::None
            }
            KeyCode::Char('t') if matches!(game.modal, Modal::Talents) => {
                game.modal = Modal::None
            }
            KeyCode::Char('y') if matches!(game.modal, Modal::Cast) => game.modal = Modal::None,
            KeyCode::Char('b') if matches!(game.modal, Modal::Journal) => game.modal = Modal::None,
            KeyCode::Char('z') if matches!(game.modal, Modal::Atlas) => {
                game.atlas_zoom = !game.atlas_zoom
            }
            _ => {}
        }
        let len = menu_len(game);
        game.selected = game.selected.min(len.saturating_sub(1));
        return;
    }
    let action = match code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('8') => Some(Action::Move(0, -1)),
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('2') => Some(Action::Move(0, 1)),
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('4') => Some(Action::Move(-1, 0)),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('6') => Some(Action::Move(1, 0)),
        KeyCode::Home | KeyCode::Char('7') => Some(Action::Move(-1, -1)),
        KeyCode::PageUp | KeyCode::Char('9') => Some(Action::Move(1, -1)),
        KeyCode::End | KeyCode::Char('1') => Some(Action::Move(-1, 1)),
        KeyCode::PageDown | KeyCode::Char('3') => Some(Action::Move(1, 1)),
        KeyCode::Enter | KeyCode::Char('A') => Some(Action::Attack),
        KeyCode::Char('e') => Some(Action::Interact),
        KeyCode::Char(' ') | KeyCode::Char('.') | KeyCode::Char('5') => Some(Action::Wait),
        KeyCode::Char('r') => Some(Action::Rest),
        KeyCode::Char('f') => {
            if game.combat.is_some() && game.player.class == Class::Keepwarden {
                game.modal = Modal::Oath;
                game.selected = 0;
                None
            } else {
                Some(Action::Defend)
            }
        }
        KeyCode::Char('x') => Some(Action::Flee),
        KeyCode::Char('c') => Some(Action::Mercy),
        KeyCode::Char('i') => {
            game.modal = Modal::Inventory;
            game.selected = 0;
            None
        }
        KeyCode::Char('b') => {
            game.modal = Modal::Journal;
            game.selected = 0;
            None
        }
        KeyCode::Char('m') => {
            game.modal = Modal::Atlas;
            game.selected = 0;
            None
        }
        KeyCode::Char('t') => {
            game.modal = Modal::Talents;
            game.selected = 0;
            None
        }
        KeyCode::Char('y') => {
            game.modal = Modal::Cast;
            game.selected = 0;
            None
        }
        _ => None,
    };
    if let Some(action) = action {
        game.action(action);
    }
}

pub fn movement_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Char('w' | 'a' | 's' | 'd' | '1' | '2' | '3' | '4' | '6' | '7' | '8' | '9')
    )
}

#[cfg(test)]
mod talent_navigation_tests {
    use super::*;

    #[test]
    fn directions_follow_branches_and_tiers_in_the_oath_diagram() {
        let mut game = Game::new(42);
        game.modal = Modal::Talents;
        game.selected = 5; // middle branch, second tier
        for (code, expected) in [
            (KeyCode::Left, 1),
            (KeyCode::Right, 5),
            (KeyCode::Up, 4),
            (KeyCode::Up, 7), // wraps within the same branch
            (KeyCode::Char('d'), 11),
            (KeyCode::Char('s'), 8),
        ] {
            key(&mut game, KeyEvent::new(code, KeyModifiers::NONE));
            assert_eq!(game.selected, expected, "moving with {code:?}");
        }
    }
}

#[cfg(test)]
mod combat_press_tests {
    use super::*;

    fn combat_game() -> Game {
        let mut game = Game::new(42);
        game.modal = Modal::None;
        game.maps[0] = Map::new("Key arena", MapKind::Arena, 30, 30, Tile::Floor);
        game.player.map = 0;
        game.player.pos = Pos::new(10, 10);
        game.npcs = vec![crate::world::make_npc(
            0, Archetype::BroodHole, 0, Pos::new(11, 10), 1, 1, 42,
        )];
        game.npcs[0].hp = 500;
        game.npcs[0].max_hp = 500;
        game.npcs[0].intent = Intent::Attack;
        game.refresh_combat();
        game
    }

    #[test]
    fn combat_cast_returns_to_battle_only_when_a_round_is_committed() {
        let mut game = combat_game();
        game.player.spells = vec![Spell::Ward];
        game.player.mana = 0;
        game.modal = Modal::Cast;
        key(&mut game, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(game.modal, Modal::Cast));
        assert_eq!(game.turn, 0);
        game.player.max_mana = 10;
        game.player.mana = 10;
        key(&mut game, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(game.modal, Modal::None));
        assert_eq!(game.turn, 1);
        assert!(game.player.defending);
    }

    #[test]
    fn enter_strikes_once_and_combat_repeat_events_do_not_spend_rounds() {
        let mut game = combat_game();
        let hp = game.npcs[0].hp;
        key(&mut game, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(game.turn, 1);
        assert!(game.npcs[0].hp < hp);
        let stamina = game.player.stamina;
        for code in [KeyCode::Enter, KeyCode::Char('d'), KeyCode::Char('f'), KeyCode::Char(' ')] {
            let mut event = KeyEvent::new(code, KeyModifiers::NONE);
            event.kind = KeyEventKind::Repeat;
            for _ in 0..40 {
                key(&mut game, event);
            }
        }
        assert_eq!(game.turn, 1);
        assert_eq!(game.player.stamina, stamina);
        key(&mut game, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(game.turn, 2);
    }
}
