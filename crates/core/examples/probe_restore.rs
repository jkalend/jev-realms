//! Restore semantics: boots a game the way the window does (Title modal with
//! a save on disk for the seed), fires 'l' exactly as input::key expects, and
//! prints the before/after state. Deterministically answers: does Title-L
//! restore, and what does the journey contain.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use laya_realms::model::Game;

fn main() {
    let mut game = Game::new(1001);
    println!(
        "fresh boot: modal={:?} class={:?} hp={} gold={} pos={:?}",
        game.modal, game.player.class, game.player.hp, game.player.gold, game.player.pos
    );
    laya_realms::input::key(
        &mut game,
        KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        },
    );
    println!(
        "after 'l' : modal={:?} class={:?} hp={} gold={} pos={:?}",
        game.modal, game.player.class, game.player.hp, game.player.gold, game.player.pos
    );
    let last: Vec<String> = game
        .log
        .iter()
        .skip(game.log.len().saturating_sub(3))
        .cloned()
        .collect();
    for line in last {
        println!("  log: {line}");
    }
}
