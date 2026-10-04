//! Shared in-window modal pane and plain-list content. The illustrated
//! creation, conversation and talent controls render in dedicated overlays;
//! gameplay actions still pass through `laya_realms::input::key`.
//!
//! Rendering: WORLD-SPACE chrome, the hud.rs pattern — bevy_ui never draws
//! under this post stack (the Light2d composite suppresses the UI pass, the
//! same multi-target interaction hud.rs documents), so the pane is four
//! camera-anchored entities: INK border quad + dark bg quad + title/body
//! Text2d, positioned at `CamFrame.pos + px · CamFrame.scale` with
//! transform scale = scale (zoom-invariant, like the orbs). Above every
//! other chrome band (nameplates top out at z≈24, the HUD sits at 100+).

use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::text::{FontSize, LetterSpacing, TextLayoutInfo};
use laya_realms::model::{
    talent_tree, Game, Item, MapKind, Modal, OathStance, QuestStage,
};
use laya_realms::social;
#[cfg(test)]
use laya_realms::model::{Build, Class};

use crate::atlas::{self, Atlas};
use crate::camera::CamFrame;
use crate::hud::DisplayFont;
use crate::palette;
use crate::sim::SimSlot;

#[derive(Component)]
pub struct ModalBorder;
#[derive(Component)]
pub struct ModalBg;
#[derive(Component)]
pub struct PaneTitle;
#[derive(Component)]
pub struct PaneBody;

/// Chrome band above the HUD (hud.rs uses Z=100..103).
const MZ: f32 = 220.0;
const PANEL_BG: Color = Color::srgb(0.04, 0.05, 0.07);
const TITLE_RGB: Color = Color::srgb_u8(240, 220, 150);
const INK_TEXT: Color = Color::srgb_u8(215, 205, 180);

/// Logical-pixel pane metrics (center-origin, y up — same space hud.rs uses).
const PANE_W: f32 = 620.0;
const PANE_PAD: f32 = 22.0;
const TITLE_H: f32 = 28.0;
const LINE_H: f32 = 18.5;
const TITLE_GAP: f32 = 12.0;
pub(crate) fn pane_width(modal: &Modal, create_step: u8, window_width: f32) -> f32 {
    let desired: f32 = match modal {
        Modal::Inventory => 900.0,
        Modal::Talents => 950.0,
        Modal::Talk(_) => 820.0,
        Modal::Create if create_step == 0 => 820.0,
        Modal::Create if create_step == 1 => 700.0,
        Modal::Create => 780.0,
        _ => PANE_W,
    };
    desired.min(window_width * 0.92)
}

/// One content line: body text plus a role so plain lists keep the TUI's
/// gold-header / ink-body / muted-footer reading order.
/// One glyph from the chrome `ItemIcons*` plates. Those plates carry 16 item
/// icons between them, four per 64px cell in a 2x2 grid of 32px sub-cells, so a
/// glyph is addressed as (plate, sub-column, sub-row).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Icon {
    plate: usize,
    col: u32,
    row: u32,
}

/// Atlas keys, in plate order — `Icon::plate` indexes this.
const ICON_PLATES: [&str; 4] = ["ItemIconsA", "ItemIconsB", "ItemIconsC", "ItemIconsD"];
/// Side of one icon sub-cell inside a 64px plate.
const ICON_SUB: u32 = 32;

impl Icon {
    const POTION_RED: Self = Self { plate: 0, col: 0, row: 0 };
    const POTION_BLUE: Self = Self { plate: 0, col: 1, row: 0 };
    const SWORD: Self = Self { plate: 1, col: 0, row: 0 };
    const ARMOUR: Self = Self { plate: 1, col: 1, row: 0 };
    const KEY: Self = Self { plate: 1, col: 1, row: 1 };
    const TORCH: Self = Self { plate: 2, col: 0, row: 0 };
    const RATION: Self = Self { plate: 2, col: 1, row: 0 };
    const HERB: Self = Self { plate: 2, col: 1, row: 1 };
    const CHEST: Self = Self { plate: 3, col: 0, row: 0 };
    const BOOK: Self = Self { plate: 3, col: 1, row: 0 };
    const COIN: Self = Self { plate: 3, col: 0, row: 1 };
    const RELIC: Self = Self { plate: 3, col: 1, row: 1 };

    /// This glyph's rect inside a plate cell whose rect is `cell`.
    fn rect(self, cell: Rect) -> Rect {
        let (x, y) = (cell.min.x, cell.min.y);
        let (s, c, r) = (ICON_SUB as f32, self.col as f32, self.row as f32);
        Rect::new(x + c * s, y + r * s, x + (c + 1.0) * s, y + (r + 1.0) * s)
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub text: String,
    pub role: Role,
    /// Optional leading glyph, drawn in the pane's icon gutter. `None` rows keep
    /// the text block's own left edge, so lists without icons are unaffected.
    pub icon: Option<Icon>,
    command: Option<MenuCommand>,
    selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Gold,
    Ink,
    Muted,
    Sea,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuCommand {
    Select(usize),
    Key(crossterm::event::KeyCode),
}

#[derive(Resource, Default)]
pub(crate) struct ModalInteraction {
    modal: Option<std::mem::Discriminant<Modal>>,
    targets: Vec<(Vec2, Vec2, MenuCommand)>,
    hovered: Option<MenuCommand>,
    scroll: usize,
    selected: usize,
}

#[derive(Component)]
pub(crate) struct PaneClose;

#[derive(Clone, Copy)]
struct BodyRow {
    selected: bool,
    icon: Option<Icon>,
    command: Option<MenuCommand>,
}

impl Line {
    fn gold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: Role::Gold,
            icon: None,
            command: None,
            selected: false,
        }
    }
    fn ink(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: Role::Ink,
            icon: None,
            command: None,
            selected: false,
        }
    }
    fn muted(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: Role::Muted,
            icon: None,
            command: None,
            selected: false,
        }
    }
    fn sea(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: Role::Sea,
            icon: None,
            command: None,
            selected: false,
        }
    }
    fn blank() -> Self {
        Self::ink("")
    }

    fn command(text: impl Into<String>, key: crossterm::event::KeyCode) -> Self {
        let mut line = Self::sea(text);
        line.command = Some(MenuCommand::Key(key));
        line
    }
}

/// Inventory glyph for an item. Anything without a sensible match returns
/// `None`, and that row simply keeps the plain text block edge.
fn item_icon(item: &Item) -> Option<Icon> {
    Some(match item {
        Item::Potion | Item::AntiToxin => Icon::POTION_RED,
        Item::GreaterPotion | Item::ManaTonic => Icon::POTION_BLUE,
        Item::Ration | Item::TravelerRation => Icon::RATION,
        Item::Torch => Icon::TORCH,
        Item::Weapon(_) => Icon::SWORD,
        Item::Armour(_) => Icon::ARMOUR,
        Item::Key(_) => Icon::KEY,
        Item::Relic | Item::BossRelic(_) | Item::Essence => Icon::RELIC,
        Item::GemDust | Item::OreFlake => Icon::COIN,
        Item::HerbCluster => Icon::HERB,
        Item::Glyph(_) | Item::GlyphShard | Item::FirstWrit => Icon::BOOK,
        Item::CaravanGoods | Item::Contraband | Item::Delivery(_) => Icon::CHEST,
    })
}

/// Shared pane title and text rows. Illustrated modals provide only a title;
/// their dedicated overlays own the selectable body.
pub fn content(game: &Game) -> (String, Vec<Line>) {
    match game.modal {
        Modal::None => (String::new(), Vec::new()),
        Modal::Title => title(game),
        Modal::Create => create(game),
        Modal::Help => help(),
        Modal::Pause => pause(),
        Modal::Inventory => inventory(game),
        Modal::Trade(id) => trade(game, id),
        Modal::Talk(id) => talk(game, id),
        Modal::Journal => journal(game),
        Modal::Atlas => atlas(game),
        Modal::Cast => cast(game),
        Modal::Forge(id) => forge(game, id),
        Modal::Oath => oath(game),
        Modal::Talents => talents(game),
        Modal::Death => death(game),
        Modal::Victory => victory(game),
    }
}

fn selected_menu(items: &[String], selected: usize) -> Vec<Line> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut line = if index == selected { Line::gold(item) } else { Line::ink(item) };
            line.command = Some(MenuCommand::Select(index));
            line.selected = index == selected;
            line
        })
        .collect()
}

fn ai_label(game: &Game) -> String {
    if game.ai.switched {
        "AI DEGRADED / offline fallback".into()
    } else if game.ai.provider.to_ascii_lowercase().contains("laya") {
        "LAYA / local gateway".into()
    } else {
        "HEURISTICS / offline".into()
    }
}

fn title(game: &Game) -> (String, Vec<Line>) {
    let save_path = format!("saves/journey-{}.json", game.seed);
    let save = std::path::Path::new(&save_path);
    let lines = vec![
        Line::gold("J E V   R E A L M S"),
        Line::sea("Every judgment leaves a trace."),
        Line::blank(),
        Line::ink("Begin in Millbrook. Talk to Mara about cellar rats."),
        Line::muted("Earn your seals. Three Sigils open the Final Trial."),
        Line::blank(),
        Line::command("Begin journey", crossterm::event::KeyCode::Enter),
        Line::command("Field guide", crossterm::event::KeyCode::Char('?')),
        if save.exists() {
            Line::command("Continue saved journey", crossterm::event::KeyCode::Char('l'))
        } else {
            Line::muted("No saved journey for this seed")
        },
        Line::command("Leave", crossterm::event::KeyCode::Char('q')),
        Line::blank(),
        Line::sea(format!("  {} / seed {}", ai_label(game), game.seed)),
    ];
    ("AN ATLAS OF CHOICES".into(), lines)
}

fn create(game: &Game) -> (String, Vec<Line>) {
    let heading = match game.create_step {
        0 => "CHOOSE YOUR ORDER",
        1 => "CHOOSE YOUR FRAME",
        _ => "CHOOSE YOUR BOON",
    };
    (heading.into(), Vec::new())
}

fn help() -> (String, Vec<Line>) {
    (
        "FIELD GUIDE / Esc closes".into(),
        vec![
            Line::gold("THE ROAD"),
            Line::ink("Arrows / WASD   Move; bump a hostile to attack"),
            Line::ink("Home/PgUp/End/PgDn or numpad 1..9   Eight-way movement"),
            Line::ink("E interact   Space wait   R rest   Enter / Shift+A attack"),
            Line::ink("F defend   X flee   C offer mercy   I use/equip items"),
            Line::ink("Combat: one fresh press or click per action. Time waits."),
            Line::blank(),
            Line::gold("THE SATCHEL"),
            Line::ink("I inventory   B journal   M atlas   J decision inspector   Y runes"),
            Line::ink("T oath-trees (talents)   ? this guide   Esc pause menu"),
            Line::blank(),
            Line::gold("THE POINTER"),
            Line::ink("Click ground to walk. In combat, nearby ground moves once."),
            Line::ink("Click an enemy in reach to strike, or a nearby local to talk."),
            Line::ink("Click responses and menu rows; wheel scrolls open lists."),
            Line::ink("Outside menus, wheel zooms. Touch: tap or two-finger pan."),
        ],
    )
}

fn pause() -> (String, Vec<Line>) {
    (
        "CAMPFIRE / settings".into(),
        vec![
            Line::gold("DISPLAY / changes are saved automatically"),
            Line::ink("Bloom   B"),
            Line::ink("Vignette   V"),
            Line::ink("Light flicker   L"),
            Line::muted("Mouse wheel adjusts the field of view"),
            Line::blank(),
            Line::command("Return to the road   Enter", crossterm::event::KeyCode::Enter),
            Line::command("Controls and field guide   ?", crossterm::event::KeyCode::Char('?')),
            Line::command("Save and quit   S", crossterm::event::KeyCode::Char('s')),
            Line::command("Quit without saving   Q", crossterm::event::KeyCode::Char('q')),
            Line::muted("One save per seed. Load it from the title with L."),
        ],
    )
}

/// Inventory/trade gear delta annotation (mirrors ui.rs `gear_note`, a
/// display-only helper; identical arithmetic, no sim involvement).
fn gear_note(game: &Game, item: &Item) -> Option<String> {
    if matches!(item, Item::BossRelic(_)) {
        return Some(if matches!(item, Item::BossRelic(r) if game.player.relic == Some(*r)) {
            "equipped".into()
        } else {
            "boss relic".into()
        });
    }
    let delta = match item {
        Item::Weapon(t) => (i32::from(*t) - i32::from(game.player.weapon)) * 3,
        Item::Armour(t) => (i32::from(*t) - i32::from(game.player.armour)) * 2,
        _ => return None,
    };
    let stat = if matches!(item, Item::Weapon(_)) {
        "ATK"
    } else {
        "DEF"
    };
    Some(if delta > 0 {
        format!("+{delta} {stat}")
    } else if delta < 0 {
        format!("{delta} {stat}")
    } else {
        "equipped tier".into()
    })
}

fn item_advice(item: Option<&Item>) -> String {
    match item {
        Some(Item::Potion) => "Enter: drink to restore 18 HP; consumes a combat turn.".into(),
        Some(Item::Ration) => "Enter: eat to restore HP and stamina.".into(),
        Some(Item::Torch) => "Enter: light for six-tile night vision.".into(),
        Some(Item::Weapon(_)) | Some(Item::Armour(_)) => {
            "Enter: equip. Better gear can turn a losing fight.".into()
        }
        Some(Item::BossRelic(relic)) => relic.describe().into(),
        Some(Item::Key(_)) => "A permanent dungeon permission; opens its entrance.".into(),
        Some(Item::CaravanGoods) => "Return the goods in Highgate, or consider the fence.".into(),
        Some(Item::Contraband) => "A Saltmarsh choice: report to the watch, or smuggle.".into(),
        Some(Item::Relic) => "Ask the oracle to identify this relic.".into(),
        Some(Item::Delivery(_)) => {
            "A bounty parcel: deliver it to the named city's guard captain.".into()
        }
        Some(Item::AntiToxin) => {
            "Enter: purge fester and steel yourself against new bites.".into()
        }
        Some(Item::ManaTonic) => "Enter: drink to recover 4 mana mid-fight.".into(),
        Some(Item::Essence) => {
            "Enter: unswear your gifts — every point returns to your hand.".into()
        }
        // E7 (§6.5, D31/D32 — core lane mirror of ui.rs advice lines).
        Some(Item::FirstWrit) => {
            "Enter: the unwinding — or carry it past the Trial for the arbiter's question.".into()
        }
        Some(Item::GreaterPotion) => {
            "Enter: drink to restore 32 HP; consumes a combat turn.".into()
        }
        Some(Item::TravelerRation) => {
            "Enter: eat to restore 12 HP and all stamina.".into()
        }
        Some(Item::GemDust) | Some(Item::HerbCluster) | Some(Item::OreFlake) | Some(Item::GlyphShard) => {
            "Alchemist's matter: try a transmute at any still.".into()
        }
        Some(Item::Glyph(_)) => {
            "A seal-glyph fragment: a smith's socket-punch (75g) writes it into plain gear.".into()
        }
        None => "Restock before descending into the dungeons.".into(),
    }
}

fn inventory(game: &Game) -> (String, Vec<Line>) {
    let mut lines = vec![
        Line::gold(format!(
            "{}/20 slots / {} gold",
            game.player.inventory.len(),
            game.player.gold
        )),
        Line::muted("Arrows/W,S select / Enter use or equip / Esc close"),
        Line::blank(),
    ];
    if game.player.inventory.is_empty() {
        lines.push(Line::muted("Your satchel is empty. Find a city vendor V."));
    } else {
        let items: Vec<String> = game
            .player
            .inventory
            .iter()
            .map(|item| match gear_note(game, item) {
                Some(note) => format!("{}  [{note}]", item.name()),
                None => item.name(),
            })
            .collect();
        let first_row = lines.len();
        lines.extend(selected_menu(&items, game.selected.min(items.len().saturating_sub(1))));
        // Each item row carries its own glyph, so the list can be scanned by
        // icon instead of by reading every name.
        for (line, item) in lines[first_row..].iter_mut().zip(game.player.inventory.iter()) {
            line.icon = item_icon(item);
        }
    }
    let selected = game.player.inventory.get(game.selected);
    let sell = selected.map(social::sell_price).unwrap_or(0);
    lines.push(Line::blank());
    lines.push(Line::sea(item_advice(selected)));
    lines.push(if sell > 0 {
        Line::command(format!("Sell beside a vendor   {sell} gold   V"), crossterm::event::KeyCode::Char('v'))
    } else {
        Line::muted("This item is not offered for sale.")
    });
    (
        "SATCHEL / inventory".into(),
        lines,
    )
}

fn trade(game: &Game, id: usize) -> (String, Vec<Line>) {
    let Some(npc) = game.npcs.get(id) else {
        return (String::new(), Vec::new());
    };
    let city = match game.maps.get(npc.map).map(|map| map.kind) {
        Some(MapKind::City(city)) => city,
        _ => game.player.last_city,
    };
    let haggle = match game.haggle {
        Some((vendor, discount)) if vendor == id => {
            format!("Negotiated reduction: {discount}%")
        }
        _ if npc.decision_pending => "Judgment pending; trading stays responsive.".into(),
        _ => "H: ask for a better price".into(),
    };
    let mut lines = vec![
        Line::gold(format!(
            "{} gold / {}/20 satchel slots",
            game.player.gold,
            game.player.inventory.len()
        )),
        Line::muted("Choose an item to buy   Arrows / W S   Enter"),
        Line::sea(haggle),
        Line::blank(),
    ];
    let items: Vec<String> = social::stock(city)
        .into_iter()
        .map(|(item, base)| {
            let price = social::buy_price(game, id, base);
            let mut row = format!("{price:>4}g  {}", item.name());
            if matches!(item, Item::Key(key) if game.player.keys.get(key).copied().unwrap_or(false)) {
                row.push_str("  [on keyring]");
            } else if let Some(note) = gear_note(game, &item) {
                row.push_str(&format!("  [{note}]"));
            }
            row
        })
        .collect();
    lines.extend(selected_menu(&items, game.selected.min(items.len().saturating_sub(1))));
    lines.push(Line::blank());
    lines.push(Line::muted(
        "Listed prices include your reputation and negotiation.",
    ));
    lines.push(Line::sea("To sell: Esc, I inventory, select an item, then V."));
    lines.push(Line::command("Work & conversation   Q", crossterm::event::KeyCode::Char('q')));
    lines.push(Line::command("Ask for a better price   H", crossterm::event::KeyCode::Char('h')));
    lines.push(Line::command("Open satchel   I", crossterm::event::KeyCode::Char('i')));
    (format!("{} / provisions", npc.name), lines)
}

fn talk(game: &Game, id: usize) -> (String, Vec<Line>) {
    if game.npcs.get(id).is_some() {
        ("CONVERSATION".into(), Vec::new())
    } else {
        (String::new(), Vec::new())
    }
}

fn stage_label(stage: QuestStage) -> &'static str {
    match stage {
        QuestStage::Locked => "LOCKED",
        QuestStage::Available => "AVAILABLE",
        QuestStage::Active => "ACTIVE",
        QuestStage::Ready => "RETURN",
        QuestStage::Complete => "COMPLETE",
    }
}

fn journal(game: &Game) -> (String, Vec<Line>) {
    let mut lines = vec![
        Line::muted("Arrows/W,S select a thread / Esc closes"),
        Line::muted("Progress survives death. Save from the pause menu before quitting."),
        Line::blank(),
    ];
    let items: Vec<String> = game
        .quests
        .iter()
        .enumerate()
        .map(|(index, quest)| {
            format!(
                "{}. {}  {} {}/{}",
                index + 1,
                quest.title,
                stage_label(quest.stage),
                quest.progress,
                quest.goal
            )
        })
        .collect();
    lines.extend(selected_menu(
        &items,
        game.selected.min(items.len().saturating_sub(1)),
    ));
    if let Some(quest) = game
        .quests
        .get(game.selected.min(game.quests.len().saturating_sub(1)))
    {
        lines.push(Line::blank());
        lines.push(Line::ink(quest.description.clone()));
        lines.push(Line::muted(
            "Three lords hold Sigils. The fourth waits in the Final Trial.",
        ));
    }
    ("THE FIVE THREADS / quest journal".into(), lines)
}

fn cast(game: &Game) -> (String, Vec<Line>) {
    let mut lines = vec![
        Line::gold(format!(
            "Mana {}/{} — casting in battle consumes your action's turn.",
            game.player.mana, game.player.max_mana
        )),
        Line::muted("Arrows/W,S select / Enter cast / Esc close"),
        Line::blank(),
    ];
    if game.player.spells.is_empty() {
        lines.push(Line::ink("No runes learned yet."));
        lines.push(Line::muted(
            "Study with any city's shrine oracle (near ? by the shrine).",
        ));
    } else {
        let items: Vec<String> = game
            .player
            .spells
            .iter()
            .map(|spell| {
                let cost = game.spell_cost(*spell);
                format!("{:<14} {} mana", spell.name(), cost)
            })
            .collect();
        lines.extend(selected_menu(
            &items,
            game.selected.min(items.len().saturating_sub(1)),
        ));
    }
    let advice: String = match game.player.spells.get(game.selected) {
        Some(spell) => spell.describe().into(),
        None => "Sparks and wards need a live enemy; mend works anywhere.".into(),
    };
    lines.push(Line::blank());
    lines.push(Line::sea(advice));
    lines.push(Line::muted("Mana returns slowly over time and fully at an inn."));
    ("RUNIC ARTS / Y closes".into(), lines)
}

fn forge(game: &Game, id: usize) -> (String, Vec<Line>) {
    let Some(npc) = game.npcs.get(id) else {
        return (String::new(), Vec::new());
    };
    let mut lines = vec![
        Line::gold(format!(
            "{} gold. Trade two matching pieces plus coin for the next tier.",
            game.player.gold
        )),
        Line::muted("Arrows/W,S select / Enter forge / Esc leave"),
        Line::blank(),
    ];
    let options = game.forge_options();
    if options.is_empty() {
        lines.push(Line::ink("Nothing to forge."));
        lines.push(Line::muted(
            "Bring two swords or two armours of equal tier (worn, standard or fine).",
        ));
    } else {
        let items: Vec<String> = options
            .iter()
            .map(|(item, cost)| {
                let (parts, tier) = match item {
                    Item::Weapon(t) => ("swords", t - 1),
                    Item::Armour(t) => ("armours", t - 1),
                    _ => unreachable!("forge yields only gear"),
                };
                format!(
                    "{cost:>4}g  {}  (2x {} {})",
                    item.name(),
                    Item::tier(tier).to_lowercase(),
                    parts
                )
            })
            .collect();
        lines.extend(selected_menu(
            &items,
            game.selected.min(items.len().saturating_sub(1)),
        ));
    }
    lines.push(Line::blank());
    lines.push(Line::muted(
        "Forged gear lands in your pack; equip it from inventory (I).",
    ));
    lines.push(Line::sea(
        "Duplicate loot is a resource — salvage it into masterwork steel.",
    ));
    (format!("{} / forge", npc.name), lines)
}

fn atlas(game: &Game) -> (String, Vec<Line>) {
    let title = if game.atlas_zoom {
        "REALM ATLAS / local chart (Z: world)"
    } else {
        "REALM ATLAS / charted lands (Z: local)"
    };
    let mut lines = vec![
        Line::gold("1/2/3: travel to a discovered city / Z toggles world-local view / Esc closes"),
        Line::muted("Travel is deliberate, never automatic on cursor movement."),
        Line::muted("(The window already charts the realm around you; this is the ledger.)"),
        Line::blank(),
    ];
    if let Some(world) = game.maps.first() {
        for city in 0..3 {
            let pos = world
                .portals
                .iter()
                .find(|p| p.destination == city + 1)
                .map(|p| format!("[{},{}]", p.pos.x, p.pos.y))
                .unwrap_or_default();
            let line = format!(
                "{} {} {} / {}",
                city + 1,
                social::city_name(city),
                pos,
                if game.visited[city] {
                    "discovered"
                } else {
                    "unvisited"
                }
            );
            lines.push(if game.visited[city] {
                Line::sea(line)
            } else {
                Line::muted(line)
            });
        }
        lines.push(Line::blank());
        for portal in world
            .portals
            .iter()
            .filter(|p| !(1..=3).contains(&p.destination))
        {
            let access = match portal.requirement {
                Some(3) => {
                    if game.player.sigils.iter().all(|s| *s) {
                        "three Sigils held"
                    } else {
                        "needs all three Sigils"
                    }
                }
                Some(key) => {
                    if game.player.keys.get(key).copied().unwrap_or(false) {
                        "seal held"
                    } else {
                        "seal required"
                    }
                }
                None => "open road",
            };
            lines.push(Line::gold(format!(
                "* {} [{},{}] / {}",
                portal.label, portal.pos.x, portal.pos.y, access
            )));
        }
    }
    (title.into(), lines)
}

fn oath(game: &Game) -> (String, Vec<Line>) {
    let stances = [OathStance::Hold, OathStance::Break, OathStance::Breathe];
    let items: Vec<String> = stances.iter().map(|s| s.name().to_string()).collect();
    let mut lines = selected_menu(&items, game.selected.min(2));
    let stance = stances[game.selected.min(2)];
    lines.push(Line::blank());
    lines.push(Line::gold(format!("  {}", stance.describe())));
    lines.push(Line::muted("  Enter swear / Esc hold position"));
    ("SWEAR YOUR OATH".into(), lines)
}

fn talents(game: &Game) -> (String, Vec<Line>) {
    if talent_tree(game.player.class).is_none() {
        return (
            "OATH-TREES".into(),
            vec![
                Line::blank(),
                Line::gold(
                    "  The Unsworn keep no lessons. The six orders each teach theirs.",
                ),
                Line::blank(),
                Line::muted("  (This journey predates the classes; its pages stay blank.)"),
                Line::muted("  Enter/esc closes. Swear an order from the title first."),
            ],
        );
    };
    // The illustrated live tree owns its seals, paths and selected lesson.
    ("OATH-TREES".into(), Vec::new())
}

fn death(game: &Game) -> (String, Vec<Line>) {
    let felled = if game.player.last_hazard.is_empty() {
        String::new()
    } else {
        format!("  Felled by: {}", game.player.last_hazard)
    };
    (
        "THE ROAD TAKES ITS TOLL".into(),
        vec![
            Line::blank(),
            Line::ink("  You fell. The world remembers."),
            Line::muted(felled),
            Line::blank(),
            Line::ink("  Enter returns you to your last city."),
            Line::muted("  You lose 25% of your gold, not your progress."),
            Line::muted("  Fallen enemies stay fallen. Quests endure."),
            Line::blank(),
            Line::gold(format!(
                "  Last refuge: {}",
                social::city_name(game.player.last_city)
            )),
            Line::sea("  [ Enter ]  Rise and try another route"),
        ],
    )
}

fn victory(game: &Game) -> (String, Vec<Line>) {
    let h = &game.player.history;
    (
        "THE FINAL VERDICT".into(),
        vec![
            Line::blank(),
            Line::gold("  THREE SIGILS. FOUR LORDS. ONE LAST JUDGMENT."),
            Line::blank(),
            Line::ink("  The Adjudicator has fallen. Your choices remain."),
            Line::blank(),
            Line::sea(format!(
                "  Level {}  /  {} gold  /  {} quests completed",
                game.player.level, game.player.gold, h.quests
            )),
            Line::ink(format!(
                "  {} kills  /  {} mercies  /  {} retreats",
                h.kills, h.mercy, h.fled
            )),
            Line::muted(format!(
                "  {} bribes  /  {} thefts  /  {} deaths",
                h.bribes, h.thefts, h.deaths
            )),
            Line::muted(format!(
                "  {} decisions  /  {} fallback judgments",
                game.ai.requests, game.ai.degraded
            )),
            Line::blank(),
            Line::gold("  [ Enter ]  Keep exploring this world"),
            Line::ink("  J inspects the decisions behind the journey."),
            Line::muted("  Esc, then S saves and quits; Q leaves without saving."),
        ],
    )
}

/// Logical-pixel point (center origin, y up) → world translation — verbatim
/// the hud.rs `at` helper so chrome layers stack identically.
fn at(frame: &CamFrame, px: Vec2, z: f32) -> Vec3 {
    (frame.pos + px * frame.scale).extend(z)
}

/// Menu-row button strips (ButtonNormal/Selected; pool reconciled per frame).
#[derive(Component)]
pub struct ModalIcon {
    index: usize,
    /// Glyph currently bound to this slot's texture; `None` means hidden.
    bound: Option<Icon>,
}

#[derive(Component)]
pub struct ButtonRow {
    index: usize,
    selected: bool,
    shown: bool,
}

const STRIP_POOL: usize = 48;
/// Icon gutter: the body block is pushed this far right so glyphs get a column
/// of their own. Rows without an icon keep the same left edge.
const ICON_GUTTER: f32 = 30.0;
/// Drawn size of one gutter glyph.
const ICON_SIZE: f32 = 22.0;
/// Gutter glyphs are reconciled per visible row, exactly like the strips.
const ICON_POOL: usize = STRIP_POOL;


/// Text2d's unbounded layout otherwise draws long dialogue choices through the
/// frame. Track physical rows so cursor bands and item glyphs follow wrapping.
fn body_rows(lines: &[Line], columns: usize) -> (String, Vec<BodyRow>) {
    let mut body = String::new();
    let mut markers = Vec::with_capacity(lines.len());
    for line in lines {
        if line.text.trim().is_empty() {
            if !markers.is_empty() { body.push('\n'); }
            markers.push(BodyRow { selected: false, icon: None, command: None });
            continue;
        }
        let mut row = String::new();
        let mut first = true;
        for word in line.text.split_whitespace() {
            if !row.is_empty() && row.chars().count() + word.chars().count() + 1 > columns {
                if !markers.is_empty() { body.push('\n'); }
                body.push_str(&row);
                markers.push(BodyRow { selected: line.selected, icon: if first { line.icon } else { None }, command: line.command });
                row.clear();
                first = false;
            }
            if !row.is_empty() { row.push(' '); }
            row.push_str(word);
        }
        if !markers.is_empty() { body.push('\n'); }
        body.push_str(&row);
        markers.push(BodyRow { selected: line.selected, icon: if first { line.icon } else { None }, command: line.command });
    }
    (body, markers)
}

/// Targets are the same measured rows the renderer draws. Hover is independent
/// of keyboard selection, so a parked pointer cannot steal arrow navigation.
pub(crate) fn clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    mut interaction: ResMut<ModalInteraction>,
    mut slot: ResMut<SimSlot>,
    mut input: ResMut<crate::input::InputState>,
) {
    let scroll: f32 = wheel.read().map(|event| event.y).sum();
    if matches!(slot.game.modal, Modal::None | Modal::Talk(_) | Modal::Create | Modal::Talents)
        || interaction.modal != Some(std::mem::discriminant(&slot.game.modal)) {
        interaction.hovered = None;
        return;
    }
    let Ok(window) = windows.single() else { return; };
    let Some(cursor) = window.cursor_position() else { interaction.hovered = None; return; };
    let point = Vec2::new(cursor.x - window.width() * 0.5, window.height() * 0.5 - cursor.y);
    interaction.hovered = interaction.targets.iter().find_map(|(center, size, command)| {
        ((point.x - center.x).abs() <= size.x * 0.5 && (point.y - center.y).abs() <= size.y * 0.5).then_some(*command)
    });
    if !matches!(slot.game.modal, Modal::Pause) && scroll != 0.0 {
        interaction.scroll = if scroll > 0.0 { interaction.scroll.saturating_sub(3) }
            else { interaction.scroll.saturating_add(3) };
    }
    if !buttons.just_pressed(MouseButton::Left) || input.frame_consumed { return; }
    let Some(command) = interaction.hovered else { return; };
    let key = match command {
        MenuCommand::Select(index) => {
            slot.game.selected = index;
            crossterm::event::KeyCode::Enter
        }
        MenuCommand::Key(key) => key,
    };
    laya_realms::input::key(&mut slot.game, crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::NONE));
    input.frame_consumed = true;
}

/// Render/refresh the modal pane as world-space chrome (hud.rs pattern; the
/// bevy_ui pass never draws under this post stack). Runs after sim::sync so
/// the pointer lane's halt flags are current; reads the SimSlot read-only.
pub(crate) fn render(
    mut commands: Commands,
    slot: Res<SimSlot>,
    font: Option<Res<DisplayFont>>,
    frame: Res<CamFrame>,
    windows: Query<&Window>,
    atlas: Option<Res<Atlas>>,
    images: Option<Res<Assets<Image>>>,
    options: Option<Res<crate::options::ViewOptions>>,
    mut spawned: Local<bool>,
    mut chrome_spawned: Local<bool>,
    mut interaction: ResMut<ModalInteraction>,
    mut set: ParamSet<(
        Query<(&mut Transform, &mut Visibility, &mut Sprite), With<ModalBorder>>,
        Query<(&mut Transform, &mut Visibility, &mut Sprite), With<ModalBg>>,
        Query<(&mut Transform, &mut Text2d, &mut Visibility), With<PaneTitle>>,
        Query<(&mut Transform, &mut Text2d, &mut TextFont, &mut Visibility, &TextLayoutInfo), With<PaneBody>>,
        Query<(&mut ButtonRow, &mut Transform, &mut Visibility, &mut Sprite), Without<ModalBorder>>,
        Query<(&mut ModalIcon, &mut Transform, &mut Visibility, &mut Sprite), Without<ModalBorder>>,
        Query<(&mut Transform, &mut Visibility), With<PaneClose>>,
    )>,
) {
    let game = &slot.game;
    let modal_open = !matches!(game.modal, Modal::None);
    interaction.targets.clear();
    let modal_key = std::mem::discriminant(&game.modal);
    if interaction.modal != Some(modal_key) {
        interaction.modal = Some(modal_key);
        interaction.scroll = 0;
        interaction.selected = usize::MAX;
        interaction.hovered = None;
    }
    if !*spawned {
        if !modal_open {
            return;
        }
        spawn_pane(&mut commands, font.as_deref());
        *spawned = true;
        return; // entities exist next frame (deferred commands)
    }
    if !modal_open {
        for (_, mut visibility, _) in set.p0().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, mut visibility, _) in set.p1().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, _, mut visibility) in set.p2().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, _, _, mut visibility, _) in set.p3().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, _, mut visibility, _) in set.p4().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, _, mut visibility, _) in set.p5().iter_mut() {
            *visibility = Visibility::Hidden;
        }
        for (_, mut visibility) in set.p6().iter_mut() { *visibility = Visibility::Hidden; }
        return;
    }
    let (w, h) = windows
        .single()
        .map(|win| (win.width(), win.height()))
        .unwrap_or((1280.0, 720.0));
    let (title, mut lines) = content(game);
    if matches!(game.modal, Modal::Pause) {
        if let Some(options) = options.as_deref() {
            let state = |on: bool| if on { "ON" } else { "OFF" };
            lines.insert(1, Line::sea(format!(
                "BLOOM {}   /   VIGNETTE {}   /   FLICKER {}   /   ZOOM {:.2}",
                state(options.bloom), state(options.vignette), state(options.flicker), options.zoom
            )));
        }
    }
    if matches!(game.modal, Modal::Inventory | Modal::Trade(_) | Modal::Forge(_) | Modal::Cast) {
        if let Some(result) = game.log.back() { lines.insert(0, Line::sea(result.clone())); }
    }
    if !*chrome_spawned {
        spawn_chrome_kit(&mut commands);
        *chrome_spawned = true;
    }
    // Font cell ~8 logical px at 13.5px. Keep the text inside the inner frame;
    // narrow viewports simply admit fewer words per physical row.
    let pw = pane_width(&game.modal, game.create_step, w);
    let text_width = match game.modal {
        Modal::Inventory => pw - 310.0,
        _ => pw,
    };
    let body_offset = match game.modal {
        Modal::Inventory => 132.0,
        _ => 0.0,
    };
    let columns = ((text_width - PANE_PAD * 2.0 - ICON_GUTTER - 16.0) / 8.0)
        .floor().max(20.0) as usize;
    let (full_body, full_markers) = body_rows(&lines, columns);
    let capacity = (((h * 0.86 - PANE_PAD * 2.0 - TITLE_H - TITLE_GAP) / LINE_H).floor() as usize).max(4);
    if interaction.selected != game.selected {
        if let Some(index) = full_markers.iter().position(|row| row.selected) {
            interaction.scroll = interaction.scroll.min(index).max(index.saturating_sub(capacity - 1));
        }
        interaction.selected = game.selected;
    }
    interaction.scroll = interaction.scroll.min(full_markers.len().saturating_sub(capacity));
    let end = (interaction.scroll + capacity).min(full_markers.len());
    let markers = &full_markers[interaction.scroll..end];
    let body = full_body.lines().skip(interaction.scroll).take(capacity).collect::<Vec<_>>().join("\n");

    // Large lists scroll instead of shrinking the type to fit.
    let s = frame.scale.max(0.0001);
    let n = markers.len().max(1) as f32;
    let line_h = LINE_H;
    let min_height = match game.modal {
        Modal::Inventory => 490.0,
        Modal::Talents => 530.0,
        Modal::Create if game.create_step == 0 => 480.0,
        Modal::Create => 400.0,
        Modal::Talk(_) => 460.0,
        _ => 0.0,
    };
    let ph = (PANE_PAD * 2.0 + TITLE_H + TITLE_GAP + n * line_h).max(min_height).min(h * 0.92);

    for (mut transform, mut visibility, mut sprite) in set.p0().iter_mut() {
        transform.translation = at(&frame, Vec2::ZERO, MZ);
        transform.scale = Vec3::new(s, s, 1.0);
        sprite.color = palette::srgb8(palette::INK_LIGHT_YELLOW).with_alpha(0.45);
        sprite.custom_size = Some(Vec2::new(pw + 4.0, ph + 4.0));
        *visibility = Visibility::Visible;
    }
    for (mut transform, mut visibility, mut sprite) in set.p1().iter_mut() {
        transform.translation = at(&frame, Vec2::ZERO, MZ + 0.5);
        transform.scale = Vec3::new(s, s, 1.0);
        sprite.custom_size = Some(Vec2::new(pw, ph));
        *visibility = Visibility::Visible;
    }
    for (mut transform, mut text, mut visibility) in set.p2().iter_mut() {
        let py = ph * 0.5 - PANE_PAD - TITLE_H * 0.5;
        transform.translation = at(&frame, Vec2::new(0.0, py), MZ + 3.0);
        transform.scale = Vec3::new(s, s, 1.0);
        if text.0 != title {
            text.0 = title.clone();
        }
        *visibility = Visibility::Visible;
    }
    let close_shown = !matches!(game.modal, Modal::Title | Modal::Create | Modal::Talk(_) | Modal::Death | Modal::Victory);
    let close_center = Vec2::new(pw * 0.5 - 54.0, ph * 0.5 - 15.0);
    for (mut transform, mut visibility) in set.p6().iter_mut() {
        transform.translation = at(&frame, close_center, MZ + 3.1);
        transform.scale = Vec3::new(s, s, 1.0);
        *visibility = if close_shown { Visibility::Visible } else { Visibility::Hidden };
    }
    if close_shown {
        interaction.targets.push((close_center, Vec2::new(92.0, 24.0), MenuCommand::Key(crossterm::event::KeyCode::Esc)));
    }
    // Row grid, measured from the body block itself.
    //
    // `LINE_H` spaces the pane, but the body is ONE `Text2d` block laid out by
    // the FONT, whose real line height is a little smaller than `LINE_H`. A grid
    // built from `LINE_H` therefore drifts a few px further from the text on
    // every row: on the rendered pane the glyph column ran at a 32px pitch under
    // 28px text, so a long list eventually missed by more than a whole row.
    // `TextLayoutInfo` reports the block's true height, so the grid is simply
    // `block height / rows` measured from the block's top edge — no constant.
    let body_py = ph * 0.5 - PANE_PAD - TITLE_H - TITLE_GAP - n * line_h * 0.5;
    let mut row_h = line_h;
    let mut row_top = body_py + n * line_h * 0.5;

    // Left edge of the body block. The block is centre-anchored and hugs its
    // own measured width, so the gutter has to follow the measurement rather
    // than a fixed x; `TextLayoutInfo` lags one frame, which is invisible.
    let mut block_left = 0.0f32;
    for (mut transform, mut text, mut text_font, mut visibility, info) in set.p3().iter_mut() {
        let py = if matches!(game.modal, Modal::Create) {
            -ph * 0.5 + PANE_PAD + n * line_h * 0.5 + 14.0
        } else {
            body_py
        };
        let font_size = 13.5 * line_h / LINE_H;
        let target_size = FontSize::Px(font_size);
        if text_font.font_size != target_size {
            text_font.font_size = target_size;
        }
        block_left = -info.size.x * 0.5;
        if info.size.y > 0.0 && text.0 == body {
            row_h = info.size.y / n;
            // The block's top edge, in the same logical space as `body_py`.
            row_top = body_py + info.size.y * 0.5;
        }
        transform.translation = at(&frame, Vec2::new(body_offset, py), MZ + 3.0);
        transform.scale = Vec3::new(s, s, 1.0);
        if text.0 != body {
            text.0 = body.clone();
        }
        *visibility = Visibility::Visible;
    }


    // Quiet backgrounds for actionable rows; gold is reserved for selection.
    let strip_width = if matches!(game.modal, Modal::Inventory) { text_width - 8.0 } else { pw - PANE_PAD * 2.0 - 12.0 };
    let rows: Vec<(f32, bool, MenuCommand)> = markers.iter().enumerate().filter_map(|(i, row)| {
        row.command.map(|command| (row_top - (i as f32 + 0.5) * row_h, row.selected, command))
    }).collect();
    for &(py, _, command) in &rows {
        interaction.targets.push((Vec2::new(body_offset, py), Vec2::new(strip_width, row_h), command));
    }
    for (strip_index, (mut row, mut transform, mut visibility, mut sprite)) in
        set.p4().iter_mut().enumerate()
    {
        let Some((py, selected, command)) = rows.get(strip_index).copied() else {
            if row.shown {
                row.shown = false;
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        row.index = strip_index;
        row.selected = selected;
        // Flat color strips keep long wrapped responses readable.
        sprite.image = Handle::default();
        sprite.rect = None;
        sprite.color = if interaction.hovered == Some(command) {
            Color::srgb_u8(39, 57, 59)
        } else if selected {
            Color::srgb_u8(79, 62, 35)
        } else {
            Color::srgb_u8(23, 31, 36)
        };
        transform.translation = at(&frame, Vec2::new(body_offset, py), MZ + 2.7);
        transform.scale = Vec3::new(s, s, 1.0);
        sprite.custom_size = Some(Vec2::new(strip_width, row_h * 1.02));
        *visibility = Visibility::Visible;
        row.shown = true;
    }

    // Gutter glyphs: one slot per row that actually carries an icon.
    let glyph_rows: Vec<(f32, Icon)> = markers.iter().enumerate()
        .filter_map(|(i, row)| row.icon.map(|ic| (row_top - (i as f32 + 0.5) * row_h, ic)))
        .collect();
    for (slot, (mut icon, mut transform, mut visibility, mut sprite)) in
        set.p5().iter_mut().enumerate()
    {
        let Some((py, want)) = glyph_rows.get(slot).copied() else {
            if icon.bound.is_some() {
                icon.bound = None;
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        icon.index = slot;
        if icon.bound != Some(want) {
            icon.bound = Some(want);
            if let Some((image, cell)) = images.as_deref().and_then(|i| {
                atlas
                    .as_deref()
                    .and_then(|a| atlas::chrome_ref(a, i, ICON_PLATES[want.plate]))
            }) {
                sprite.image = image.clone();
                sprite.rect = Some(want.rect(cell));
            }
        }
        // Sit the glyph just outside the block's left edge, clamped inside the
        // pane well so a very wide body can never push it off the frame.
        let well_left = -pw * 0.5 + PANE_PAD;
        let icon_x = (block_left + body_offset - ICON_GUTTER * 0.5 - 1.0).max(well_left + ICON_SIZE * 0.5);
        transform.translation = at(&frame, Vec2::new(icon_x, py), MZ + 2.9);
        transform.scale = Vec3::new(s, s, 1.0);
        sprite.custom_size = Some(Vec2::splat(ICON_SIZE));
        *visibility = Visibility::Visible;
    }
}

/// Flat selection strips and item glyph slots do not depend on atlas readiness.
fn spawn_chrome_kit(commands: &mut Commands) {
    for index in 0..ICON_POOL {
        commands.spawn((
            Sprite::from_color(Color::WHITE, Vec2::splat(ICON_SIZE)),
            Transform::from_xyz(0.0, -100_000.0, MZ + 2.9),
            Visibility::Hidden,
            ModalIcon { index, bound: None },
        ));
    }
    for _ in 0..STRIP_POOL {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(23, 31, 36), Vec2::ONE),
            Transform::from_xyz(0.0, -100_000.0, MZ + 2.7),
            Visibility::Hidden,
            ButtonRow {
                index: usize::MAX,
                selected: false,
                shown: false,
            },
        ));
    }
}

fn spawn_pane(commands: &mut Commands, font: Option<&DisplayFont>) {
    let serif = font.and_then(|f| f.0.clone());
    // Border: 2px larger gold-tinted quad BEHIND the dark panel.
    commands.spawn((
        Sprite::from_color(
            palette::srgb8(palette::INK_LIGHT_YELLOW).with_alpha(0.45),
            Vec2::ONE,
        ),
        Transform::from_xyz(0.0, 0.0, MZ),
        ModalBorder,
        Visibility::Hidden,
    ));
    commands.spawn((
        Sprite::from_color(PANEL_BG, Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, MZ + 0.5),
        ModalBg,
        Visibility::Hidden,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: serif
                .clone()
                .map(bevy::text::FontSource::Handle)
                .unwrap_or_default(),
            ..TextFont::from_font_size(20.0)
        },
        TextLayout::justify(Justify::Center),
        TextColor(TITLE_RGB),
        LetterSpacing::Px(1.4),
        Transform::from_xyz(0.0, 0.0, MZ + 3.0),
        PaneTitle,
        Visibility::Hidden,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont::from_font_size(13.5),
        TextColor(INK_TEXT),
        Transform::from_xyz(0.0, 0.0, MZ + 3.0),
        PaneBody,
        Visibility::Hidden,
    ));
    commands.spawn((
        Text2d::new("Close   Esc"),
        TextFont::from_font_size(10.5),
        TextColor(TITLE_RGB),
        Transform::default(),
        PaneClose,
        Visibility::Hidden,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{SimSlot, WorldView};
    use laya_realms::model::Modal;

    fn headless_game() -> Game {
        // Fresh world: the real Title modal this time (boot default).
        Game::new(42)
    }

    fn press(app: &mut App, code: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(code);
        app.update();
        // Simulate the OS release (ButtonInput::press on a held key is a
        // no-op, so without this the next press of the same key never fires
        // just_pressed).
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(code);
        keys.clear();
    }

    /// E6 boot flow, end to end through the modal systems: the window opens
    /// on Title, Enter enters Create, a class is sworn, a boon taken, and
    /// the first world frame snapshots (pointer gates all down).
    #[test]
    fn title_to_world_first_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<WorldView>()
            .init_resource::<crate::camera::CamFrame>()
            .init_resource::<crate::pointer::PointerIns>()
            .init_resource::<ModalInteraction>()
            .init_resource::<crate::input::InputState>()
            .insert_resource(SimSlot {
                game: headless_game(),
                frame: 0,
                sim_ms: 0.0,
                shot: None,
            })
            .add_systems(Update, crate::input::modal_keys)
            .add_systems(Update, render.after(crate::input::modal_keys))
            .add_systems(
                Update,
                crate::sim::sync.after(render),
            );
        app.update(); // systems register; modal pane spawns
        app.update(); // second pass: body text lands
        let slot = app.world().resource::<SimSlot>();
        assert!(matches!(slot.game.modal, Modal::Title), "boot lands on Title");

        press(&mut app, KeyCode::Enter); // Title → Create (class list)
        {
            let slot = app.world().resource::<SimSlot>();
            assert!(matches!(slot.game.modal, Modal::Create));
            assert_eq!(slot.game.create_step, 0);
        }
        press(&mut app, KeyCode::KeyS); // nav down one order
        press(&mut app, KeyCode::Enter); // swear it → body build
        {
            let slot = app.world().resource::<SimSlot>();
            assert!(matches!(slot.game.modal, Modal::Create));
            assert_eq!(slot.game.create_step, 1);
            assert_eq!(slot.game.create_class, 1);
        }
        press(&mut app, KeyCode::KeyS); // nav down to the feminine frame
        press(&mut app, KeyCode::Enter); // take the frame → boon list
        {
            let slot = app.world().resource::<SimSlot>();
            assert!(matches!(slot.game.modal, Modal::Create));
            assert_eq!(slot.game.create_step, 2);
            assert_eq!(slot.game.create_build, 1);
        }
        press(&mut app, KeyCode::Enter); // take a boon → journey begins
        {
            let slot = app.world().resource::<SimSlot>();
            assert!(matches!(slot.game.modal, Modal::None));
            assert_eq!(slot.game.player.class, Class::from_index(1));
            assert_eq!(slot.game.player.build, Build::Female);
        }
        app.update(); // sync: first world frame
        let view = app.world().resource::<WorldView>();
        assert!(!view.modal_open, "world frame snapshots modal-free");
    }

    #[test]
    fn long_choice_stays_inside_pane_and_keeps_its_cursor() {
        let mut line = Line::ink("A sentence with several words ".repeat(5));
        line.command = Some(MenuCommand::Select(3));
        line.selected = true;
        let (body, markers) = body_rows(&[line], 32);
        assert!(body.lines().all(|row| row.chars().count() <= 32), "{body}");
        assert!(markers.iter().all(|row| row.selected && row.command == Some(MenuCommand::Select(3))));
    }
}
