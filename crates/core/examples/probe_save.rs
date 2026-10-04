//! Self-drive observability probe (agent playtest loop): loads a journey save
//! exactly like the view does and prints the input-relevant sim state so a
//! windowless driver can aim keys blindly — modal, player pos, and the NPCs
//! within reach worth pressing E toward.
//!
//! Usage: probe_save [seed]  (default 42)

use laya_realms::model::{Action, Game};
use laya_realms::persist;

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let mut game = Game::new(seed);
    persist::try_load(&mut game);
    println!("modal       = {:?}", game.modal);
    println!(
        "player      = {:?} on map {} ({})  hp {}/{} stamina {}",
        game.player.pos,
        game.player.map,
        game.map().name,
        game.player.hp,
        game.player.max_hp,
        game.player.stamina
    );
    println!("combat      = {}", game.combat.is_some());
    println!("class       = {:?}", game.player.class);
    println!("create_step = {}  create_class = {}", game.create_step, game.create_class);
    print!("near npcs   =");
    let mut near: Vec<_> = game
        .npcs
        .iter()
        .filter(|n| n.alive() && n.map == game.player.map)
        .map(|n| (n.pos.x.abs_diff(game.player.pos.x) + n.pos.y.abs_diff(game.player.pos.y), n))
        .collect();
    near.sort_by_key(|(d, _)| *d);
    for (d, n) in near.iter().take(6) {
        print!(" [{:?}@{:?} d{}]", n.archetype, n.pos, d);
    }
    println!();

    // Sanity on the movement gate — the two classic directions.
    for (dx, dy) in [(1, 0), (0, 1)] {
        let before = game.player.pos;
        game.action(Action::Move(dx, dy));
        if game.player.pos != before {
            println!("Move({dx},{dy}): {:?} -> {:?}", before, game.player.pos);
        } else {
            println!("Move({dx},{dy}): BLOCKED at {:?}", before);
        }
    }
}

