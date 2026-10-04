//! E5 playtest rebalance harness (docs/D2_EVOLUTION.md §4.5, v0.9 review note).
//!
//! Re-runs the §4.5 numeric model against the **shipped** constants with the
//! adopted adjustments applied: A1 (D21: Keepwarden recoil = first attacker per
//! round, no +1 DEF bias), A2 (D22/D35: sellswords level-matched to the player —
//! `world::sellsword_stats`, bonded ×5/4 for Fensworn), A3 (D23: guaranteed flee
//! at 3 banked momentum — an escape check, not a damage number), and the E5 boss
//! ladder (side bosses + the cap-12 Adjudicator at 168 HP).
//!
//! The model is deliberately arithmetic, mirroring the engine formulas exactly:
//!   attack_power = attack + 3·weapon (+ Gravebound Last Vigil tier)
//!   defense_power = defense + 2·armour; melee = max(1, atk − def)
//!   brace (Hold) = (defense_power·3 + 1) / 2; no flanking (per §4.5 method)
//!   Spark = max(4, attack_power) + channel, ignores armour, costs 2 mana,
//!   mana regens +1 per combat round; growth table = engine's own loop.
//!
//! What the original model excluded applies here too: potions ("DIES" is the
//! intended consumable pressure), relics, talents, and position play. Boss
//! signatures are folded in as documented per-boss pressure knobs below — each
//! one names the mechanic it cheaply represents, so the table is a *ladder
//! sanity check*, not a DPS oracle.

use crate::model::{Archetype, Class, Pos};

/// Fresh-run starting sheet (world.rs `Game::new` player literal).
pub fn growth(level: u32) -> (i32, i32, i32, i32, i32) {
    let (mut hp, mut stamina, mut attack, mut defense, mut speed) = (20, 12, 3, 1, 10);
    for l in 2..=level {
        hp += 6;
        stamina += 1;
        if l % 2 == 0 {
            attack += 1;
        }
        if l % 3 == 0 {
            defense += 1;
        }
        if l == 5 || l == 9 {
            speed += 1;
        }
    }
    (hp, stamina, attack, defense, speed)
}

/// Forge cadence worn → masterwork across the campaign (§4.5: tiers 0/1/2/3 at
/// L1/4/7/10). Dungeon loot floors the same curve.
pub fn weapon_tier(level: u32) -> u8 {
    ((level.saturating_sub(1)) / 3).min(3) as u8
}

/// One fighter's sheet after order bias (§4.2; A1 keeps the Keepwarden's +1 DEF
/// dropped). `mana` is maximum mana; stamina is tracked where a policy burns it.
#[derive(Clone, Copy, Debug)]
pub struct Sheet {
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    pub mana: i32,
    pub weapon: u8,
    pub armour: u8,
}

pub fn class_sheet(class: Class, level: u32) -> Sheet {
    let (hp, _stamina, attack, defense, speed) = growth(level);
    let tier = weapon_tier(level);
    let mut sheet = Sheet {
        level,
        hp,
        max_hp: hp,
        attack,
        defense,
        speed,
        mana: 0,
        weapon: tier,
        armour: tier,
    };
    match class {
        Class::None => {}
        Class::Keepwarden => {
            sheet.hp += 4;
            sheet.max_hp += 4;
        }
        Class::Gravebound => {
            sheet.hp += 2;
            sheet.max_hp += 2;
            sheet.speed -= 1;
        }
        Class::Redwake => {
            sheet.speed += 2;
            sheet.defense -= 1;
        }
        Class::Waysworn => {
            sheet.speed += 1;
            sheet.hp -= 2;
            sheet.max_hp -= 2;
        }
        Class::SigilSworn => {
            sheet.hp -= 3;
            sheet.max_hp -= 3;
            sheet.attack -= 1;
            sheet.mana = 4;
        }
        Class::Fensworn => {}
    }
    sheet
}

impl Sheet {
    fn attack_power(&self) -> i32 {
        self.attack + i32::from(self.weapon) * 3
    }
    fn defense_power(&self) -> i32 {
        self.defense + i32::from(self.armour) * 2
    }
    /// Hold-the-Gate brace (the engine's integer math, stance selection from §4.2).
    fn braced_defense(&self) -> i32 {
        (self.defense_power() * 3 + 1) / 2
    }
}

/// A hostile: (name, hp, attack, defense, speed). Boss rows come from the shipped
/// `world::make_npc` table — the harness cannot drift from the game.
#[derive(Clone, Debug)]
pub struct Foe {
    pub name: &'static str,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
}

pub fn foe(name: &'static str, hp: i32, attack: i32, defense: i32, speed: i32) -> Foe {
    Foe {
        name,
        hp,
        attack,
        defense,
        speed,
    }
}

/// Wilderness trash pairs per §4.5 ("at-level bandit pairs" / "trash pairs" read
/// here as wolves — the wilderness fast control). Stats are the worldgen rows.
pub fn bandit_pair(level: u32) -> Vec<Foe> {
    let l = level as i32;
    vec![
        foe("bandit", 7 + l * 3, 2 + l, l / 3, 10),
        foe("bandit", 7 + l * 3, 2 + l, l / 3, 10),
    ]
}
pub fn trash_pair(level: u32) -> Vec<Foe> {
    let l = level as i32;
    vec![
        foe("wolf", 7 + l * 3, 2 + l, l / 3, 12),
        foe("wolf", 7 + l * 3, 2 + l, l / 3, 12),
    ]
}

/// A boss from the shipped ladder. `player_level` is the natural quest level the
/// §6 tiers name it at.
pub fn boss(kind: Archetype) -> Foe {
    let npc = crate::world::make_npc(0, kind, 0, Pos::new(0, 0), 1, 0, 42);
    Foe {
        name: match kind {
            Archetype::GnawThane => "Gnaw-Thane (L2)",
            Archetype::Tollmaster => "Tollmaster (L4)",
            Archetype::Chief => "Chief (L5)",
            Archetype::Mirelight => "Mirelight (L5)",
            Archetype::Cragmother => "Cragmother (L6)",
            Archetype::Matriarch => "Matriarch (L7)",
            Archetype::PaleStag => "Pale Stag (L7)",
            Archetype::Tidemother => "Tidemother (L8)",
            Archetype::Lich => "Vael/Lich (L9)",
            Archetype::OathlessCurate => "Oathless Curate (L10)",
            Archetype::Adjudicator => "Adjudicator (L10/12)",
            _ => "boss",
        },
        hp: npc.hp,
        attack: npc.attack,
        defense: npc.defense,
        speed: npc.speed,
    }
}

/// The natural player level each boss is faced at (§6 ladder + act table).
pub fn boss_level(kind: Archetype) -> u32 {
    match kind {
        Archetype::GnawThane => 2,
        Archetype::Tollmaster => 4,
        Archetype::Chief => 5,
        Archetype::Mirelight => 5,
        Archetype::Cragmother => 6,
        Archetype::Matriarch | Archetype::PaleStag => 7,
        Archetype::Tidemother => 8,
        Archetype::Lich => 9,
        Archetype::OathlessCurate => 10,
        Archetype::Adjudicator => 10,
        _ => 5,
    }
}

/// Signature pressure — one documented knob per boss, folded into the exchange:
/// * `chip(round)`: extra boss-side damage dealt to the player that round, on
///   top of its melee (Tollmaster's dunk + caltrop, Tidemother's undertow,
///   Gnaw-Thane's tide rats as an aggregate gnaw stream).
/// * `regen(round, hp, max_hp)`: boss-side recovery (Pale Stag's Break loop —
///   burst-only counter, no Hartshorn in hand yet).
/// * `wrong_strikes`: per-boss count of strikes the player lands into the wrong
///   target first (Mirelight's information check): those heal the boss 6 each.
#[derive(Clone, Copy, Debug)]
pub struct Pressure {
    pub chip_every: u32,
    pub chip_from: u32,
    pub chip: i32,
    pub regen_below_half: i32,
    pub wrong_strikes: u32,
}

impl Pressure {
    pub const NONE: Pressure = Pressure {
        chip_every: 0,
        chip_from: 0,
        chip: 0,
        regen_below_half: 0,
        wrong_strikes: 0,
    };
    const fn chip(every: u32, from: u32, chip: i32) -> Pressure {
        Pressure {
            chip_every: every,
            chip_from: from,
            chip,
            regen_below_half: 0,
            wrong_strikes: 0,
        }
    }
}

pub fn pressure(kind: Archetype) -> Pressure {
    match kind {
        // Plague Tide: from round 3 each cycle adds a rat's gnaw to the pile
        // (burst-the-king path — killing the holes instead removes this).
        Archetype::GnawThane => Pressure::chip(3, 3, 2),
        // Bridge Tax + caltrops: the shove's dunk (8) and one planted bite (2)
        // every third round while the crew presses.
        Archetype::Tollmaster => Pressure::chip(3, 1, 10),
        // Undertow: every fourth round the dock does 8.
        Archetype::Tidemother => Pressure::chip(4, 4, 8),
        // Avalanche Slam: the telegraphed 3×3 unless repositioned — the
        // progression check lands as +4 on the slam rounds.
        Archetype::Cragmother => Pressure::chip(3, 3, 4),
        Archetype::Mirelight => Pressure {
            wrong_strikes: 2,
            ..Pressure::NONE
        },
        Archetype::PaleStag => Pressure {
            regen_below_half: 5,
            ..Pressure::NONE
        },
        _ => Pressure::NONE,
    }
}

/// Skirmish outcome: clear-time (rounds, lower = faster) / rounds survived.
#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub clear: f32,
    pub survive: f32,
}

fn survival(sheet: &Sheet, foes: &[Foe], soaks: usize, braced: bool) -> f32 {
    let defense = if braced {
        sheet.braced_defense()
    } else {
        sheet.defense_power()
    };
    // A sworn blade soaks the blows meant for its employer (one attacker past
    // the first lands on the partner); partner death is excluded per §4.5.
    let hitting_player = foes.len().saturating_sub(soaks);
    let incoming: i32 = foes
        .iter()
        .take(hitting_player.max(usize::from(soaks == 0)) .min(foes.len()))
        .map(|f| (f.attack - defense).max(1))
        .sum();
    sheet.hp as f32 / incoming as f32
}

/// Whether a companion fights beside this sheet: always for the Fensworn (25g
/// of the 40g starting purse); the others hire from L2 up once the purse allows
/// (D22/D35 keep it level-matched from then on).
pub fn companion_for(class: Class, level: u32) -> bool {
    match class {
        Class::Fensworn => true,
        _ => level >= 2,
    }
}

/// D35 companion damage line at the boss/sheet level, bonded for the Fensworn.
/// Cadence: every third striking round doubles (Bond §4.2) → ×4/3 average.
fn companion_dps(class: Class, level: u32, foe_def: i32) -> f32 {
    let (_hp, attack, _def, _spd) = crate::world::sellsword_stats(level);
    let hit = (attack - foe_def).max(1) as f32;
    if class == Class::Fensworn {
        hit * 4.0 / 3.0
    } else {
        hit
    }
}

fn melee_hit(sheet: &Sheet, bonus: i32, foe_def: i32) -> i32 {
    (sheet.attack_power() + bonus - foe_def).max(1)
}

/// One engagement. Closed-form effective-DPS for the flat classes (matching the
/// v0.4 table's own derivations: Wanderer column is the calibration anchor);
/// round-simulated for the counter-driven classes (channel, momentum, vigil).
pub fn fight(class: Class, sheet: Sheet, foes_in: &[Foe], press: Pressure) -> Outcome {
    let soaks = usize::from(companion_for(class, sheet.level));
    let braced = class == Class::Keepwarden;
    let survive = survival(&sheet, foes_in, soaks, braced);
    let comp = |def: i32| {
        if companion_for(class, sheet.level) {
            companion_dps(class, sheet.level, def)
        } else {
            0.0
        }
    };
    let total: f32 = foes_in.iter().map(|f| f.hp as f32).sum();
    if foes_in.len() == 1 {
        let f = &foes_in[0];
        let clear = match class {
            // Wanderer: pure exchange — the calibration column.
            Class::None => boss_exchange(&sheet, f, per_melee(&sheet, 0, f, &comp), press),
            // Keepwarden (A1): +1 recoil to the round's first attacker, and the
            // Break-Their-Line credit worth half a point across the exchange.
            Class::Keepwarden => boss_exchange(
                &sheet,
                f,
                per_melee(&sheet, 0, f, &comp) + 1.5,
                press,
            ),
            // Redwake: melee + the scouting pip's +2 (A3 buys exits, not DPS).
            Class::Redwake => boss_exchange(&sheet, f, per_melee(&sheet, 2, f, &comp), press),
            // Gravebound: the Vigil tier tracks the fight's own attrition curve.
            Class::Gravebound => boss_exchange_vigil(&sheet, f, per_melee(&sheet, 0, f, &comp), press),
            // Waysworn/Fensworn: flat sheets; Fensworn's difference is the blade.
            Class::Waysworn | Class::Fensworn => {
                boss_exchange(&sheet, f, per_melee(&sheet, 0, f, &comp), press)
            }
            Class::SigilSworn => spark_exchange(&sheet, foes_in, press, comp(0)),
        };
        return Outcome { clear, survive };
    }
    // Pair skirmish: one strike per round at the lead target (pairs in this
    // harness are homogeneous), the blade on its own initiative.
    let def = foes_in[0].defense;
    let clear = match class {
        Class::None | Class::Waysworn | Class::Gravebound => {
            total / per_melee(&sheet, 0, &foes_in[0], &comp)
        }
        Class::Fensworn => total / per_melee(&sheet, 0, &foes_in[0], &comp),
        Class::Keepwarden => total / (per_melee(&sheet, 0, &foes_in[0], &comp) + 1.5),
        Class::Redwake => total / per_melee(&sheet, 2, &foes_in[0], &comp),
        Class::SigilSworn => spark_exchange(&sheet, foes_in, press, comp(def)),
    };
    Outcome { clear, survive }
}

fn per_melee(sheet: &Sheet, bonus: i32, f: &Foe, comp: &dyn Fn(i32) -> f32) -> f32 {
    melee_hit(sheet, bonus, f.defense) as f32 + comp(f.defense)
}

/// Sigil-Sworn: the sustained channel (§4.2). Cast while mana flows (+1/round
/// trickle, 2 per bolt); in dry rounds the knife comes out and the anchor drops
/// (D24: only move/melee break it). The bonded blade hits armoured flesh, never
/// the wrong light: seal-script reads the true tell, so Spark ignores the wisp's
/// false-light heal — a mage's answer to the information fight.
fn spark_exchange(sheet: &Sheet, foes_in: &[Foe], press: Pressure, comp_per: f32) -> f32 {
    let single = foes_in.len() == 1;
    let mut foes: Vec<Foe> = foes_in.to_vec();
    let mut rounds = 0.0f32;
    let mut mana = sheet.mana;
    let mut channel: i32 = 0;
    let mut wrong = press.wrong_strikes;
    while foes.iter().any(|f| f.hp > 0) && rounds < 400.0 {
        rounds += 1.0;
        let idx = foes
            .iter()
            .enumerate()
            .filter(|(_, f)| f.hp > 0)
            .map(|(i, _)| i)
            .next()
            .unwrap();
        let mut dealt = comp_per;
        if mana >= 2 {
            mana -= 2;
            if single && wrong > 0 {
                // The bolt hunts the glow Laya names — and the eye is fooled.
                wrong -= 1;
                foes[idx].hp = (foes[idx].hp + 6).min(foes_in[idx].hp);
                dealt = comp_per;
            } else {
                dealt += (sheet.attack_power().max(4) + channel) as f32;
                channel = (channel + 1).min(2);
            }
        } else {
            channel = 0;
            dealt += melee_hit(sheet, 0, foes[idx].defense) as f32;
        }
        foes[idx].hp -= dealt.round() as i32;
        if single && press.regen_below_half > 0 && foes[idx].hp > 0 && foes[idx].hp * 2 < foes_in[idx].hp {
            foes[idx].hp = (foes[idx].hp + press.regen_below_half).min(foes_in[idx].hp);
        }
        mana = (mana + 1).min(sheet.mana);
    }
    rounds
}

/// Boss clear-time for the flat-DPS classes: boss HP minus signature pressure —
/// breaks/regen extend the fight; the player's chip intake never stalls DPS.
fn boss_exchange(_sheet: &Sheet, f: &Foe, per_round: f32, press: Pressure) -> f32 {
    let mut hp = f.hp;
    let mut rounds = 0.0f32;
    let mut wrong = press.wrong_strikes;
    while hp > 0 && rounds < 500.0 {
        rounds += 1.0;
        if wrong > 0 {
            wrong -= 1;
            hp = (hp + 6).min(f.hp);
        } else {
            hp -= per_round.ceil() as i32;
        }
        if press.regen_below_half > 0 && hp > 0 && hp * 2 < f.hp {
            hp = (hp + press.regen_below_half).min(f.hp);
        }
    }
    rounds
}

/// Gravebound's boss curve: the +2/+4 Vigil tiers engage as the fight costs HP.
fn boss_exchange_vigil(sheet: &Sheet, f: &Foe, base_per: f32, press: Pressure) -> f32 {
    let mut boss_hp = f.hp;
    let mut hp = sheet.hp;
    let mut rounds = 0.0f32;
    let mut wrong = press.wrong_strikes;
    while boss_hp > 0 && rounds < 500.0 {
        rounds += 1.0;
        let bonus = if hp * 4 < sheet.max_hp {
            4
        } else if hp * 2 < sheet.max_hp {
            2
        } else {
            0
        };
        let per = if wrong > 0 { base_per } else { base_per + bonus as f32 };
        if wrong > 0 {
            wrong -= 1;
            boss_hp = (boss_hp + 6).min(f.hp);
        } else {
            boss_hp -= per.ceil() as i32;
        }
        if press.regen_below_half > 0 && boss_hp > 0 && boss_hp * 2 < f.hp {
            boss_hp = (boss_hp + press.regen_below_half).min(f.hp);
        }
        // Boss pressure back: melee each round plus the signature chip.
        let chip = if press.chip_every > 0
            && rounds as u32 >= press.chip_from
            && (rounds as u32).is_multiple_of(press.chip_every)
        {
            press.chip
        } else {
            0
        };
        hp -= (f.attack - sheet.defense_power()).max(1) + chip;
    }
    rounds
}

/// The §6 ladder as natural-level boss rows, in fight order.
pub fn boss_ladder() -> Vec<(Archetype, Foe)> {
    [
        Archetype::GnawThane,
        Archetype::Tollmaster,
        Archetype::Chief,
        Archetype::Mirelight,
        Archetype::Cragmother,
        Archetype::Matriarch,
        Archetype::PaleStag,
        Archetype::Tidemother,
        Archetype::Lich,
        // E7 (§6.5): the act V second door, then the rung that judges the run.
        Archetype::OathlessCurate,
        Archetype::Adjudicator,
    ]
    .into_iter()
    .map(|k| (k, boss(k)))
    .collect()
}

pub const CLASSES: [Class; 7] = [
    Class::None,
    Class::Keepwarden,
    Class::Gravebound,
    Class::Redwake,
    Class::Waysworn,
    Class::SigilSworn,
    Class::Fensworn,
];

pub fn class_tag(class: Class) -> &'static str {
    match class {
        Class::None => "Wanderer",
        Class::Keepwarden => "Keepwarden",
        Class::Gravebound => "Gravebound",
        Class::Redwake => "Redwake",
        Class::Waysworn => "Waysworn",
        Class::SigilSworn => "SigilSworn",
        Class::Fensworn => "Fensworn",
    }
}

/// One skirmish row: every class vs the level's bandit or trash control pair.
pub fn skirmish_row(level: u32, trash: bool) -> Vec<(Class, Outcome)> {
    let foes = if trash {
        trash_pair(level)
    } else {
        bandit_pair(level)
    };
    CLASSES
        .into_iter()
        .map(|c| (c, fight(c, class_sheet(c, level), &foes, Pressure::NONE)))
        .collect()
}

/// One ladder row per boss: each class clears at the natural level.
pub fn boss_row(kind: Archetype, player_level: u32) -> Vec<(Class, Outcome)> {
    let foe = boss(kind);
    let press = pressure(kind);
    CLASSES
        .into_iter()
        .map(|c| {
            let level = player_level;
            (c, fight(c, class_sheet(c, level), std::slice::from_ref(&foe), press))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(clear: f32, target: f32) -> bool {
        (clear - target).abs() / target <= 0.30
    }

    fn skirmish(class: Class, level: u32, trash: bool) -> Outcome {
        let sheet = class_sheet(class, level);
        let foes = if trash {
            trash_pair(level)
        } else {
            bandit_pair(level)
        };
        fight(class, sheet, &foes, Pressure::NONE)
    }

    /// E5 playtest band harness (§4.5 v0.4 cells, A1–A3 applied).
    ///
    /// What the re-run may anchor, policy-for-policy:
    /// * L1 cells for the v0.4 columns are untouched by A1–A3 (no blade is
    ///   hired at L1; recoil/breaker math is the shipped A1 form) → ±30%.
    /// * L4/L7/L10 and every boss row carried the **adopted** A2/D35 lift — a
    ///   level-matched blade joins from L2 — so the v0.4 cells become a
    ///   one-sided band: strictly ≤ ×1.05 (lift or tie), never below ×0.45
    ///   (the lift must not read as a different game entirely).
    /// * The new columns (Gravebound/Waysworn v0.5) pin against the Wanderer
    ///   band the v0.5 text claims (parity ±30%).
    #[test]
    fn balance_band_holds_against_the_v04_table() {
        // Model calibration: the Wanderer column is derived closed-form from
        // the shipped formulas; at L1 (pre-hire) it is exact-identical.
        let l1 = skirmish(Class::None, 1, false);
        assert!(
            (l1.clear - 6.7).abs() / 6.7 <= 0.03 && (l1.survive - 5.0).abs() / 5.0 <= 0.03,
            "model anchor drifted: Wanderer L1 {:.2}/{:.2} (expect 6.7/5.0)",
            l1.clear,
            l1.survive
        );
        // L1 anchors for the v0.4 columns (A1's shipped recoil reads exactly).
        let k1 = skirmish(Class::Keepwarden, 1, false);
        assert!(
            cell(k1.clear, 4.4) && cell(k1.survive, 12.0),
            "A1 Keepwarden L1 {:.1}/{:.1} vs table 4.4/12.0",
            k1.clear,
            k1.survive
        );
        let r1 = skirmish(Class::Redwake, 1, false);
        assert!(cell(r1.clear, 3.3), "Redwake L1 {:.1} vs 3.3", r1.clear);
        // F1: the caster reads weakest early without pool — ship semantics hold
        // the intent even where the v0.4 print (4.6, pool-artifact) does not.
        let s1 = skirmish(Class::SigilSworn, 1, false);
        assert!(
            s1.clear > l1.clear,
            "F1 intent: SigilSworn must read weakest early ({:.1} vs {:.1})",
            s1.clear,
            l1.clear
        );
        // A2/D35 lift band across the companion-bearing cells (W/K/S/R).
        for (level, w, k, s, r) in [
            (4u32, 5.4, 4.5, 5.5, 3.8),
            (7, 5.6, 4.9, 5.3, 4.3),
            (10, 5.3, 4.8, 4.8, 4.4),
        ] {
            for (class, target) in [
                (Class::None, w),
                (Class::Keepwarden, k),
                (Class::SigilSworn, s),
                (Class::Redwake, r),
            ] {
                let o = skirmish(class, level, false);
                let ratio = o.clear / target;
                assert!(
                    (0.45..=1.05).contains(&ratio),
                    "{class:?} L{level}: {:.1} vs v0.4 {target} (ratio {ratio:.2} outside the adopted A2 lift band)",
                    o.clear
                );
            }
        }
        // Boss ladder rows: same adopted-lift band against the v0.4 prints.
        for (kind, targets) in [
            (Archetype::Chief, (10.7, 9.1, 9.7, 7.1)),
            (Archetype::Matriarch, (9.6, 8.6, 8.3, 7.2)),
            (Archetype::Lich, (12.0, 10.8, 9.7, 9.0)),
        ] {
            let row = boss_row(kind, boss_level(kind));
            let get = |c: Class| row.iter().find(|(cc, _)| *cc == c).map(|(_, o)| o).unwrap();
            let (w, k, s, r) = targets;
            for (class, target) in [
                (Class::None, w),
                (Class::Keepwarden, k),
                (Class::SigilSworn, s),
                (Class::Redwake, r),
            ] {
                let o = get(class);
                let ratio = o.clear / target;
                assert!(
                    (0.45..=1.05).contains(&ratio),
                    "{class:?} vs {kind:?}: {:.1} vs v0.4 {target} (ratio {ratio:.2} outside lift band)",
                    o.clear
                );
            }
        }
        // D26 rung: 168 HP at the cap-12 sheet vs the adjusted v0.4 anchor
        // (12.3 at 148 → ~14.0 at 168), under the same adopted-lift semantics.
        let w12 = boss_row(Archetype::Adjudicator, 12)
            .into_iter()
            .find(|(c, _)| *c == Class::None)
            .map(|(_, o)| o)
            .unwrap();
        let ratio = w12.clear / 14.0;
        assert!(
            (0.45..=1.05).contains(&ratio),
            "Adjudicator 168 at L12: {:.1} vs anchor 14.0 (ratio {ratio:.2})",
            w12.clear
        );
        // §4.5's caster attrition check at the new rung: bounded, not infinite —
        // the level-matched blade may cover, but the bolt alone must not stall ±2.5× the garrison's pace.
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
        assert!(
            s10 / k10 <= 2.5,
            "caster attrition drifted: SigilSworn {:.1} vs Keepwarden {:.1} at the 168 rung",
            s10,
            k10
        );
        // v0.5 columns: a bias-parity read around the Wanderer band.
        for level in [1u32, 4, 7, 10] {
            let wanderer = skirmish(Class::None, level, false);
            for class in [Class::Gravebound, Class::Waysworn] {
                let o = skirmish(class, level, false);
                assert!(
                    cell(o.clear, wanderer.clear),
                    "{class:?} L{level} clear {:.1} vs Wanderer band {:.1} (v0.5 bias-parity claim)",
                    o.clear,
                    wanderer.clear
                );
            }
        }
    }

    /// A2/D35: Fensworn early stays the smooth read but no longer doubles the
    /// entire DPS at L1, and Blade-matched classes never clear slower solo.
    #[test]
    fn fensworn_curve_after_a2() {
        let f1 = skirmish(Class::Fensworn, 1, false);
        let w1 = skirmish(Class::None, 1, false);
        assert!(
            f1.clear < w1.clear,
            "Fensworn identity must still smooth the first hour ({:.1} vs {:.1})",
            f1.clear,
            w1.clear
        );
        assert!(
            f1.clear >= 2.4,
            "A2 must tame the broken-early burst ({:.1} was the old 2.1)",
            f1.clear
        );
        // Dead-late is gone: at the Lich rung, Fensworn no longer trails the pack.
        let lich = boss_row(Archetype::Lich, boss_level(Archetype::Lich));
        let f = lich.iter().find(|(c, _)| *c == Class::Fensworn).unwrap().1;
        let w = lich.iter().find(|(c, _)| *c == Class::None).unwrap().1;
        assert!(
            f.clear <= w.clear + 0.5,
            "Fensworn must not be the slowest at Lich rung ({:.1} vs {:.1})",
            f.clear,
            w.clear
        );
    }

    /// §6 side-boss ladder sanity: each side boss at its natural level clears
    /// SLOWER than the level's trash control for the baseline sheet — the
    /// superunique lesson must outlast two wolves, never out-race them.
    #[test]
    fn side_bosses_outlast_their_trash_controls() {
        for (kind, level) in [
            (Archetype::GnawThane, 2u32),
            (Archetype::Tollmaster, 4),
            (Archetype::Mirelight, 5),
            (Archetype::PaleStag, 7),
        ] {
            let row = boss_row(kind, level);
            let boss_clear = row
                .iter()
                .find(|(c, _)| *c == Class::None)
                .map(|(_, o)| o)
                .unwrap();
            let control = skirmish(Class::None, level, true);
            assert!(
                boss_clear.clear > control.clear,
                "{kind:?} at L{level} clears in {:.1} — faster than its trash control {:.1}; tune the ladder",
                boss_clear.clear,
                control.clear
            );
        }
    }
}
