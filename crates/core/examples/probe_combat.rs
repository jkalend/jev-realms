//! Deterministic combat replay: lands the player adjacent to a live wolf in
//! its home map (dev-harness teleport, NOT a game feature) and drives the
//! verb set the window forwards: walk-in -> Shift+A attack + F defend across
//! several rounds with damage accounting. Usage: probe_combat [seed]

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use laya_realms::model::Game;
use laya_realms::persist;

fn ev(code: KeyCode, shift: bool) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: if shift {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        },
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1001);
    let mut game = Game::new(seed);
    persist::try_load(&mut game);

    // Find a wolf anywhere and stand one tile west of it.
    let Some(wolf_ref) = game
        .npcs
        .iter()
        .find(|n| n.alive() && matches!(n.archetype, laya_realms::model::Archetype::Wolf))
        .map(|n| (n.id, n.map, n.pos))
    else {
        println!("no live wolf in this seed");
        return;
    };
    let (wolf_id, wolf_map, wolf_pos) = wolf_ref;
    println!("wolf {} on map {} at {:?}", wolf_id, wolf_map, wolf_pos);

    game.player.map = wolf_map;
    game.player.pos = wolf_pos.offset(-1, 0);
    println!(
        "teleported: map {} player {:?} adjacent target id {}",
        game.player.map, game.player.pos, wolf_id
    );

    let player_hp0 = game.player.hp;
    let wolf_hp0 = game.npcs.iter().find(|n| n.id == wolf_id).map(|n| n.hp).unwrap_or(-1);

    // Drive combat verbs exactly like the window forwards them.
    let actions = [
        ("A", ev(KeyCode::Char('A'), true)), // Shift+A = attack adjacent
        ("A", ev(KeyCode::Char('A'), true)),
        ("F", ev(KeyCode::Char('f'), false)), // defend
        ("A", ev(KeyCode::Char('A'), true)),
        ("A", ev(KeyCode::Char('A'), true)),
        ("F", ev(KeyCode::Char('f'), false)),
        ("A", ev(KeyCode::Char('A'), true)),
        ("A", ev(KeyCode::Char('A'), true)),
        ("A", ev(KeyCode::Char('A'), true)),
        ("A", ev(KeyCode::Char('A'), true)),
    ];
    for (label, event) in actions {
        laya_realms::input::key(&mut game, event);
        let wolf = game.npcs.iter().find(|n| n.id == wolf_id).map(|n| n.hp).unwrap_or(-1);
        println!(
            "{:>3} -> combat={} player_hp={} wolf_hp={} stamina={}",
            label,
            game.combat.is_some(),
            game.player.hp,
            wolf,
            game.player.stamina
        );
        if wolf <= 0 || game.player.hp <= 0 {
            break;
        }
    }
    // No-verb ticks: let world time flow so the wolf can act.
    for i in 0..4 {
        let before = game.player.pos;
        let _ = before;
        let start_hp = game.player.hp;
        laya_realms::input::key(&mut game, ev(KeyCode::Char(' '), false));
        println!(
            "wait {} -> player_hp {} stamina {} combat={}",
            i,
            game.player.hp,
            game.player.stamina,
            game.combat.is_some()
        );
        if game.player.hp < start_hp {
            println!("    (^ wolf landed a hit during passive time)");
        }
    }
    let wolf_hp1 = game.npcs.iter().find(|n| n.id == wolf_id).map(|n| n.hp).unwrap_or(-9);
    println!(
        "summary: player {}->{} wolf {}->{} combat_now={}",
        player_hp0,
        game.player.hp,
        wolf_hp0,
        wolf_hp1,
        game.combat.is_some()
    );
    let last: Vec<String> = game
        .log
        .iter()
        .skip(game.log.len().saturating_sub(6))
        .cloned()
        .collect();
    for line in last {
        println!("  log: {line}");
    }
}
