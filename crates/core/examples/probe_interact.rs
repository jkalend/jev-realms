//! Deterministic interaction replay: loads a journey and feeds the exact
//! crossterm keystrokes the window forwards — no window needed — printing the
//! modal at every step so a blind driver can follow Talk->trade paths.
//!
//! Usage: probe_interact [seed]

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use laya_realms::model::Game;
use laya_realms::persist;

fn press(game: &mut Game, code: KeyCode, label: &str) {
    let event = KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    };
    laya_realms::input::key(game, event);
    println!(
        "{:>10} -> modal {:?} sel {} gold {} inv {:?}",
        label,
        game.modal,
        game.selected,
        game.player.gold,
        game.player.inventory
    );
}

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1001);
    let mut game = Game::new(seed);
    persist::try_load(&mut game);
    println!(
        "boot       -> modal {:?}  gold {} inv_len {} pos {:?} map {}",
        game.modal,
        game.player.gold,
        game.player.inventory.len(),
        game.player.pos,
        game.player.map
    );
    press(&mut game, KeyCode::Char('e'), "E");
    press(&mut game, KeyCode::Char('s'), "s(nav)");
    press(&mut game, KeyCode::Char('s'), "s(nav)");
    press(&mut game, KeyCode::Enter, "BUY");
    press(&mut game, KeyCode::Char('q'), "Q->Talk");
    press(&mut game, KeyCode::Char('s'), "s(nav)");
    press(&mut game, KeyCode::Enter, "TALK");
    press(&mut game, KeyCode::Esc, "ESC");
    let last: Vec<_> = game.log.iter().skip(game.log.len().saturating_sub(4)).cloned().collect();
    for line in last {
        println!("  log: {line}");
    }
}
