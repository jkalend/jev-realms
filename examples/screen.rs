//! Renders the actual game UI to a text-mode backend and prints the frame.
//! Usage: cargo run --release --example screen -- [seed] [scenario]
//! Scenarios: start (default), combat, forge, cast, shop

use laya_realms::{model::*, ui};
use ratatui::{backend::TestBackend, Terminal};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(42);
    let scenario = args.next().unwrap_or_else(|| "start".into());
    let mut game = Game::new(seed);
    game.modal = Modal::None;
    match scenario.as_str() {
        "combat" => {
            // Place the player beside two bandits in the arena and start a fight.
            game.maps[0] = Map::new("Demo arena", MapKind::Arena, 30, 30, Tile::Floor);
            game.maps[0].portals.clear();
            game.player.map = 0;
            game.player.pos = Pos::new(10, 10);
            game.player.gold = 145;
            game.player.spells = vec![Spell::Spark, Spell::Mend, Spell::Ward];
            game.player.max_mana = 8;
            game.player.mana = 6;
            game.maps[0].explored.fill(true);
            game.npcs.retain(|_| false);
            for (i, pos) in [Pos::new(11, 10), Pos::new(9, 9)].into_iter().enumerate() {
                let mut bandit =
                    laya_realms::world::make_npc(i, Archetype::Bandit, 0, pos, 3, 700, seed);
                bandit.intent = Intent::Attack;
                game.npcs.push(bandit);
            }
            let blade = game.npcs.len();
            game.npcs.push(laya_realms::world::make_npc(
                blade,
                Archetype::Companion,
                0,
                Pos::new(10, 11),
                3,
                900,
                seed,
            ));
            game.companion = Some(blade);
            game.combat = Some(Combat {
                center: game.player.pos,
                round: 2,
                participants: vec![0, 1],
            });
            game.player.defending = true;
        }
        "forge" => {
            let smith = game
                .npcs
                .iter()
                .position(|n| {
                    n.archetype == Archetype::Vendor
                        && n.name.contains("Blacksmith")
                        && n.map == game.player.map
                })
                .expect("smith");
            game.player.inventory = vec![
                Item::Weapon(0),
                Item::Weapon(0),
                Item::Armour(1),
                Item::Armour(1),
                Item::Potion,
            ];
            game.player.gold = 145;
            game.modal = Modal::Forge(smith);
        }
        "cast" => {
            game.player.spells = vec![Spell::Spark, Spell::Mend, Spell::Ward];
            game.player.max_mana = 8;
            game.player.mana = 6;
            game.modal = Modal::Cast;
        }
        "atlas" => {
            game.atlas_zoom = args.next().is_some_and(|v| v == "local");
            game.modal = Modal::Atlas;
        }
        "shop" => {
            let vendor = game
                .npcs
                .iter()
                .position(|n| {
                    n.archetype == Archetype::Vendor
                        && n.map == game.player.map
                        && !n.name.contains("Innkeeper")
                })
                .expect("vendor");
            game.modal = Modal::Trade(vendor);
        }
        _ => {}
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 44)).expect("backend");
    terminal.draw(|frame| ui::draw(frame, &game)).expect("draw");
    let buffer = terminal.backend().buffer();
    // "dump" works as the scenario (start frame) or as a trailing flag (any scenario).
    if scenario == "dump" || args.next().is_some_and(|a| a == "dump") {
        // Machine-readable frame for the docs/gfx style-comparison page.
        let rgb = |c: ratatui::style::Color| match c {
            ratatui::style::Color::Rgb(r, g, b) => (r, g, b),
            ratatui::style::Color::Black => (0, 0, 0),
            ratatui::style::Color::Red => (204, 51, 51),
            ratatui::style::Color::Green => (0, 170, 0),
            ratatui::style::Color::Yellow => (187, 187, 0),
            ratatui::style::Color::Blue => (85, 85, 238),
            ratatui::style::Color::Magenta => (187, 85, 187),
            ratatui::style::Color::Cyan => (85, 187, 187),
            ratatui::style::Color::Gray => (187, 187, 187),
            ratatui::style::Color::DarkGray => (102, 102, 102),
            ratatui::style::Color::LightRed => (255, 102, 102),
            ratatui::style::Color::LightGreen => (102, 255, 102),
            ratatui::style::Color::LightYellow => (255, 255, 102),
            ratatui::style::Color::LightBlue => (102, 102, 255),
            ratatui::style::Color::LightMagenta => (255, 102, 255),
            ratatui::style::Color::LightCyan => (102, 255, 255),
            ratatui::style::Color::White => (255, 255, 255),
            _ => (187, 187, 187),
        };
        print!(
            "{{\"width\":{},\"height\":{},\"cells\":[",
            buffer.area.width, buffer.area.height
        );
        let mut first = true;
        let mut fills: Vec<String> = Vec::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                let symbol = cell.symbol();
                let (r, g, b) = rgb(cell.fg);
                let (br, bgc, bb) = rgb(cell.bg);
                if symbol.trim().is_empty() {
                    // Glyph-less fills (gauge bars, panels) still carry color.
                    if !matches!(
                        cell.bg,
                        ratatui::style::Color::Black | ratatui::style::Color::Reset
                    ) {
                        fills.push(format!("{{\"x\":{x},\"y\":{y},\"bg\":[{br},{bgc},{bb}]}}"));
                    }
                    continue;
                }
                if !first {
                    print!(",");
                }
                first = false;
                let escaped = symbol.replace('\\', "\\\\").replace('"', "\\\"");
                print!(
                    "{{\"x\":{x},\"y\":{y},\"ch\":\"{escaped}\",\"fg\":[{r},{g},{b}],\"bg\":[{br},{bgc},{bb}]}}"
                );
            }
        }
        print!("],\"fills\":[{}]}}", fills.join(","));
        println!();
        return;
    }
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            row.push_str(if symbol.is_empty() { " " } else { symbol });
        }
        println!("{}", row.trim_end());
    }
}
