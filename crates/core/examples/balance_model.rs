//! Renders the §4.5 re-run tables (the E5 playtest rebalance pass).
//! `cargo run -p laya-realms --example balance_model`

use laya_realms::balance::{
    bandit_pair, boss, boss_ladder, boss_level, boss_row, class_sheet, fight, skirmish_row,
    trash_pair, Pressure,
};
use laya_realms::model::{Archetype, Class};

fn cell(outcome: laya_realms::balance::Outcome) -> String {
    format!("{:.1}/{:.1}", outcome.clear, outcome.survive)
}

fn main() {
    println!("E5 playtest rebalance — docs/D2_EVOLUTION.md §4.5 re-run");
    println!("A1 (recoil first-attacker, no +1 DEF bias) · A2 (D22/D35 level-matched blades) · A3 (D23 paid exit)");
    println!("Excludes potions, flanking, relics, talents — a short 'survive' read is intended consumable pressure.\n");
    skirmish_table();
    boss_table();
    checks();
}

fn header(h: &str) {
    println!("== {h}");
    println!(
        "{:<24}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}",
        "", "Wanderer", "Keepwarden", "Gravebound", "Redwake", "Waysworn", "SigilSworn", "Fensworn"
    );
}

fn skirmish_table() {
    header("Skirmish ladder (clear / survive)");
    for level in [1u32, 4, 7, 10] {
        for trash in [false, true] {
            let label = format!("L{level} {}", if trash { "wolf pair" } else { "bandit pair" });
            let row = skirmish_row(level, trash);
            let cells: String = row
                .iter()
                .map(|(_, o)| format!("{:>12}", cell(*o)))
                .collect();
            println!("{:<24}{}", label, cells);
        }
    }
    println!();
}

fn boss_table() {
    header("Boss ladder (clear at natural level)");
    for (kind, _) in boss_ladder() {
        let level = boss_level(kind);
        let foe = boss(kind);
        let row = boss_row(kind, level);
        let cells: String = row
            .iter()
            .map(|(_, o)| format!("{:>12.1}", o.clear))
            .collect();
        println!("{:<24}{}", format!("{} @L{level}", foe.name), cells);
    }
    // Cap-12 rung (D26: the rung that rose to 168).
    let row = boss_row(Archetype::Adjudicator, 12);
    let cells: String = row
        .iter()
        .map(|(_, o)| format!("{:>12.1}", o.clear))
        .collect();
    println!("{:<24}{}", "Adjudicator 168 @L12", cells);
    println!();
}

fn checks() {
    println!("== Checks (A1–A3, §6 ladder sanity)");
    let f1 = fight(
        Class::Fensworn,
        class_sheet(Class::Fensworn, 1),
        &bandit_pair(1),
        Pressure::NONE,
    );
    println!(
        "A2 Fensworn L1 clear {:.1} (old broken-early clear was 2.1; identity should read 'smooth', not 'free')",
        f1.clear
    );
    println!("A3 Redwake paid exit at 3 pips: mechanical (engine flee path); no damage number to tune");
    for (kind, level) in [
        (Archetype::GnawThane, 2u32),
        (Archetype::Tollmaster, 4),
        (Archetype::Mirelight, 5),
        (Archetype::PaleStag, 7),
    ] {
        let boss_clear = boss_row(kind, level)
            .into_iter()
            .find(|(c, _)| *c == Class::None)
            .map(|(_, o)| o.clear)
            .unwrap();
        let control = fight(
            Class::None,
            class_sheet(Class::None, level),
            &trash_pair(level),
            Pressure::NONE,
        )
        .clear;
        let verdict = if boss_clear > control { "OK" } else { "VIOLATION" };
        println!(
            "{:?} L{level}: boss clear {:.1} vs trash control {:.1} — {verdict}",
            kind, boss_clear, control
        );
    }
    let s10 = boss_row(Archetype::Adjudicator, 10)
        .into_iter()
        .find(|(c, _)| *c == Class::SigilSworn)
        .map(|(_, o)| o.clear)
        .unwrap();
    let k10 = boss_row(Archetype::Adjudicator, 10)
        .into_iter()
        .find(|(c, _)| *c == Class::Keepwarden)
        .map(|(_, o)| o.clear)
        .unwrap();
    println!(
        "Adjudicator 168 @L10: SigilSworn {:.1} vs Keepwarden {:.1} (ratio {:.2}× — §4.5's intended caster attrition check)",
        s10, k10, s10 / k10
    );
}
