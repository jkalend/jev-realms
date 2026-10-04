use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }
    pub fn offset(self, x: i32, y: i32) -> Self {
        Self::new(self.x + x, self.y + y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tile {
    Road,
    Grass,
    Forest,
    DeepForest,
    Mountain,
    Rock,
    River,
    Ford,
    Ruins,
    Wall,
    Floor,
    Door,
    Up,
    Down,
    Shrine,
    Chest,
}
impl Tile {
    pub fn walkable(self) -> bool {
        !matches!(self, Self::Rock | Self::River | Self::Wall)
    }
    pub fn cost(self) -> u32 {
        match self {
            Self::Forest | Self::Ford => 150,
            Self::DeepForest => 200,
            Self::Mountain => 300,
            _ => 100,
        }
    }
    pub fn glyph(self) -> char {
        // Eastern-Asian-width-neutral characters only: every terminal renders
        // these as a single column, keeping map rows aligned in any font.
        match self {
            Self::Road => '.',
            Self::Grass => '░',
            Self::Forest => '♣',
            Self::DeepForest => '♠',
            Self::Mountain => '▲',
            Self::Rock => '▓',
            Self::River => '≈',
            Self::Ford => '-',
            Self::Ruins => '&',
            Self::Wall => '█',
            Self::Floor => '.',
            Self::Door => '+',
            Self::Up => '<',
            Self::Down => '>',
            Self::Shrine => 'Ω',
            Self::Chest => '$',
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapKind {
    Overworld,
    City(usize),
    Dungeon(usize, usize),
    Cellar,
    Arena,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portal {
    pub pos: Pos,
    pub destination: usize,
    pub arrival: Pos,
    pub label: String,
    pub requirement: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Map {
    pub name: String,
    pub kind: MapKind,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub portals: Vec<Portal>,
    pub explored: Vec<bool>,
    /// Operable thresholds remain traversable: stepping through opens the leaf.
    /// Older saves omit this field and load with every door closed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub open_doors: Vec<Pos>,
}
impl Map {
    pub fn new(name: &str, kind: MapKind, width: i32, height: i32, tile: Tile) -> Self {
        Self {
            name: name.into(),
            kind,
            width,
            height,
            tiles: vec![tile; (width * height) as usize],
            portals: Vec::new(),
            explored: vec![false; (width * height) as usize],
            open_doors: Vec::new(),
        }
    }
    pub fn index(&self, p: Pos) -> Option<usize> {
        if p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height {
            Some((p.y * self.width + p.x) as usize)
        } else {
            None
        }
    }
    pub fn tile(&self, p: Pos) -> Tile {
        self.index(p).map(|i| self.tiles[i]).unwrap_or(Tile::Wall)
    }
    pub fn set(&mut self, p: Pos, tile: Tile) {
        if let Some(i) = self.index(p) {
            self.tiles[i] = tile;
            self.open_doors.retain(|door| *door != p);
        }
    }
    pub fn door_open(&self, p: Pos) -> bool {
        self.tile(p) == Tile::Door && self.open_doors.contains(&p)
    }

    pub fn set_door_open(&mut self, p: Pos, open: bool) {
        if self.tile(p) != Tile::Door {
            return;
        }
        if open {
            if !self.open_doors.contains(&p) {
                self.open_doors.push(p);
            }
        } else {
            self.open_doors.retain(|door| *door != p);
        }
    }

    pub fn can_step(&self, from: Pos, to: Pos) -> bool {
        self.tile(to).walkable()
            && (from.x == to.x
                || from.y == to.y
                || (self.tile(Pos::new(from.x, to.y)).walkable()
                    && self.tile(Pos::new(to.x, from.y)).walkable()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Archetype {
    Commoner,
    Vendor,
    Guard,
    Thief,
    Traveller,
    Bandit,
    Wolf,
    Bear,
    Rat,
    Skeleton,
    Chief,
    Matriarch,
    Lich,
    Adjudicator,
    Smuggler,
    Oracle,
    Companion,
    Tidemother,
    Cragmother,
    // E5 side bosses and their support cast (§6; eight arcade bosses follow E3).
    GnawThane,
    Tollmaster,
    Mirelight,
    PaleStag,
    Alchemist,
    BroodHole,
    FalseGlow,
    // E7 (§6.5 act V): the covenant-breaker under the Underkeep's new depths.
    OathlessCurate,
}

/// The two body builds the creation screen offers. A presentation choice only:
/// no stat branches on it. It selects the player model and the sprite key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Build {
    #[default]
    Male,
    Female,
}

impl Build {
    /// Create-screen order: 0 masculine frame, 1 feminine frame.
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Female,
            _ => Self::Male,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Male => "Masculine",
            Self::Female => "Feminine",
        }
    }
    /// Second token of the model/sprite key: `Player.Keepwarden.Male`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }
    pub fn note(self) -> &'static str {
        match self {
            Self::Male => "Broader shoulders, heavier plate.",
            Self::Female => "Narrower shoulders, lighter plate.",
        }
    }
}
impl Archetype {
    pub fn hostile(self) -> bool {
        matches!(
            self,
            Self::Bandit
                | Self::Wolf
                | Self::Bear
                | Self::Rat
                | Self::Skeleton
                | Self::Chief
                | Self::Matriarch
                | Self::Lich
                | Self::Adjudicator
                | Self::Tidemother
                | Self::Cragmother
                | Self::GnawThane
                | Self::Tollmaster
                | Self::Mirelight
                | Self::PaleStag
                | Self::BroodHole
                | Self::FalseGlow
                | Self::OathlessCurate
        )
    }
    pub fn boss(self) -> bool {
        matches!(
            self,
            Self::Chief
                | Self::Matriarch
                | Self::Lich
                | Self::Adjudicator
                | Self::Tidemother
                | Self::Cragmother
                | Self::GnawThane
                | Self::Tollmaster
                | Self::Mirelight
                | Self::PaleStag
                | Self::OathlessCurate
        )
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Commoner => 'c',
            Self::Vendor => 'V',
            Self::Guard => 'G',
            Self::Thief => 't',
            Self::Traveller => 'v',
            Self::Bandit => 'b',
            Self::Wolf => 'w',
            Self::Bear => 'B',
            Self::Rat => 'r',
            Self::Skeleton => 's',
            Self::Chief => 'K',
            Self::Matriarch => 'W',
            Self::Lich => 'L',
            Self::Adjudicator => 'A',
            Self::Smuggler => 'S',
            Self::Oracle => '?',
            Self::Companion => 'p',
            Self::Tidemother => 'Ω',
            Self::Cragmother => 'Θ',
            Self::GnawThane => 'R',
            Self::Tollmaster => 'T',
            Self::Mirelight => 'M',
            Self::PaleStag => 'H',
            Self::Alchemist => 'a',
            Self::BroodHole => 'o',
            Self::FalseGlow => 'f',
            Self::OathlessCurate => 'C',
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spell {
    Spark,
    Mend,
    Ward,
}
impl Spell {
    pub fn name(self) -> &'static str {
        match self {
            Self::Spark => "Spark bolt",
            Self::Mend => "Mend wounds",
            Self::Ward => "Iron ward",
        }
    }
    pub fn cost(self) -> i32 {
        match self {
            Self::Spark | Self::Mend => 2,
            Self::Ward => 3,
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Self::Spark => "Strike the nearest enemy within 5 tiles, ignoring armor.",
            Self::Mend => "Restore 12 HP, in or out of combat.",
            Self::Ward => "+2 defense and a braced stance for two rounds.",
        }
    }
    pub fn next_unlearned(known: &[Spell]) -> Option<Spell> {
        [Self::Spark, Self::Mend, Self::Ward]
            .into_iter()
            .find(|s| !known.contains(s))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Personality {
    pub brave: f32,
    pub greedy: f32,
    pub gullibility: f32,
    pub spite: f32,
    pub chatty: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub turn: u64,
    pub kind: String,
    pub weight: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memory {
    pub events: VecDeque<MemoryEvent>,
    pub disposition: f32,
}
impl Memory {
    pub fn remember(&mut self, turn: u64, kind: &str, weight: f32) {
        if self.events.len() == 12 {
            // Mercy and debt records must survive a crowded recent history:
            // evict the oldest ordinary event before touching them. Quest outcomes
            // (spared caravan wolves, reclaimable thefts) depend on these.
            let critical = |event: &MemoryEvent| {
                event.kind == "player_spared_me" || event.kind.starts_with("stole:")
            };
            let evict = self
                .events
                .iter()
                .position(|event| !critical(event))
                .unwrap_or(0);
            self.events.remove(evict);
        }
        self.events.push_back(MemoryEvent {
            turn,
            kind: kind.into(),
            weight,
        });
        self.disposition = (self.disposition + weight).clamp(-1.0, 1.0);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Idle,
    Attack,
    Flee,
    Help,
    Patrol,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Npc {
    pub id: usize,
    pub name: String,
    pub archetype: Archetype,
    pub map: usize,
    pub pos: Pos,
    pub home: Pos,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    pub level: u32,
    pub personality: Personality,
    pub memory: Memory,
    pub pack: usize,
    pub intent: Intent,
    pub decision_pending: bool,
    pub decision_version: u64,
    pub last_decision: u64,
    pub last_hp_band: i32,
    pub perceived: bool,
    pub tactic: String,
    pub phase: u8,
    pub cooldown: u32,
    pub mercy_used: bool,
    pub telegraph: Option<(Pos, String)>,
    pub answers: BTreeMap<String, Answer>,
}
impl Npc {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    Potion,
    Ration,
    Torch,
    Weapon(u8),
    Armour(u8),
    Key(usize),
    CaravanGoods,
    Contraband,
    Relic,
    BossRelic(BossRelic),
    Delivery(usize),
    // E5 (D32/D36): the alchemist's launch draughts and the boss essence (D8).
    AntiToxin,
    ManaTonic,
    Essence,
    // E7 (§6.5/D8): the covenant-breaker's ransom — one more unwinding, and the
    // key to the arbiter's question after the Trial.
    FirstWrit,
    // E7 (D32 transmute set + scrounge chain; §8.2).
    GreaterPotion,
    TravelerRation,
    GemDust,
    HerbCluster,
    OreFlake,
    GlyphShard,
    // E7 (D31 seal-glyphs §8.1): named fragments of seal-script, per GLYPHS.
    Glyph(u8),
}
impl Item {
    pub fn name(&self) -> String {
        match self {
            Self::Potion => "Healing potion (+18 HP)".into(),
            Self::Ration => "Trail ration (+8 HP, stamina)".into(),
            Self::Torch => "Torch (6-tile night vision)".into(),
            Self::Weapon(t) => format!("{} sword", Self::tier(*t)),
            Self::Armour(t) => format!("{} armour", Self::tier(*t)),
            Self::Key(i) => format!(
                "{} seal",
                ["Burrow", "Crimson Hollow", "Underkeep"]
                    .get(*i)
                    .unwrap_or(&"Dungeon")
            ),
            Self::CaravanGoods => "Caravan goods".into(),
            Self::Contraband => "Smuggler's parcel".into(),
            Self::Relic => "Unidentified relic".into(),
            Self::BossRelic(relic) => relic.name().into(),
            Self::Delivery(city) => format!(
                "Sealed delivery ({})",
                ["Millbrook", "Highgate", "Saltmarsh"]
                    .get(*city)
                    .unwrap_or(&"unknown")
            ),
            Self::AntiToxin => "Anti-toxin (purges fester; steels you against it)".into(),
            Self::ManaTonic => "Mana tonic (+4 mana)".into(),
            Self::Essence => "Unravelled essence (one more unwinding of your gifts)".into(),
            Self::FirstWrit => "The First Writ (the arbiter's question; an unwinding besides)".into(),
            Self::GreaterPotion => "Greater potion (+32 HP)".into(),
            Self::TravelerRation => "Traveler's ration (+12 HP, full stamina)".into(),
            Self::GemDust => "Gem dust (alchemist's grit)".into(),
            Self::HerbCluster => "Herb cluster (fen-moss and wispwort)".into(),
            Self::OreFlake => "Ore flake (ridge iron, forge-bright)".into(),
            Self::GlyphShard => "Glyph shard (seal-script sand)".into(),
            Self::Glyph(g) => format!("Seal-glyph: {}", glyph_name(*g)),
        }
    }
    pub fn tier(t: u8) -> &'static str {
        ["Worn", "Standard", "Fine", "Masterwork"][t.min(3) as usize]
    }
}

/// E7 (D31, §8.1): the six named seal-glyph fragments, index-stable.
pub const GLYPHS: [&str; 6] = ["ash", "fen", "gate", "seal", "hart", "crown"];
pub fn glyph_name(index: u8) -> &'static str {
    GLYPHS.get(index as usize).copied().unwrap_or("unk")
}

/// One Word (§8.1): an exact ordered pair of glyphs socketed in order.
/// Words sit on plain gear only — never on boss relics (D31 quardrail).
#[derive(Clone, Copy, Debug)]
pub struct WordDef {
    pub name: &'static str,
    /// Ordered sequence; sockets cap at 2 (fine+ gear), and every word is a pair.
    pub seq: [u8; 2],
    /// Which gear kind carries it.
    pub slot: WordSlot,
    pub effect: &'static str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordSlot {
    Weapon,
    Armour,
}
const fn w(name: &'static str, seq: [u8; 2], slot: WordSlot, effect: &'static str) -> WordDef {
    WordDef { name, seq, slot, effect }
}
pub const WORDS: [WordDef; 6] = [
    w("Vigil", [3, 2], WordSlot::Armour, "+1 defense; once per combat your brace ignores the first hit."),
    w("Ember", [0, 1], WordSlot::Weapon, "Your strikes scorch: +2 attack power."),
    w("Gale", [4, 3], WordSlot::Armour, "+1 speed."),
    w("Censer", [0, 3], WordSlot::Armour, "+1 defense under a roof or in the deep."),
    w("Truce", [1, 4], WordSlot::Weapon, "Your partner's strikes bite +1 deeper."),
    w("Lode", [2, 5], WordSlot::Armour, "Bounty work pays +10% — the code reads the metal."),
];

/// Epilogue stance (§6.5): the arbiter's question, answered at the verdict
/// shrine with the First Writ in hand after the Trial.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Epilogue {
    #[default]
    Unanswered,
    Sealed,
    Watch,
}

/// One order's class-quest trial (§6.5): offer/act gate is checked at the
/// glib NPC; the trial fight itself runs in its own pocket arena.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialStage {
    #[default]
    Locked,
    Offered,
    Active,
    Done,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClassTrial {
    pub stage: TrialStage,
    pub wave: u8,
    /// Escort trial: the cargo-cart guard and the dead-or-alive verdict bookkeeping.
    pub escort: Option<usize>,
    pub failed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossRelic {
    Rallybreaker,
    Fangmantle,
    Graveglass,
    Saltcrown,
    Stoneheart,
    // E5 side-boss relics (§6: each arena pays a lesson, not a sigil — D11).
    GnawboneCrown,
    TollcoinCharm,
    WisplightLantern,
    Hartshorn,
}
impl BossRelic {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rallybreaker => "Rallybreaker",
            Self::Fangmantle => "Fangmantle",
            Self::Graveglass => "Graveglass",
            Self::Saltcrown => "Saltcrown",
            Self::Stoneheart => "Stoneheart",
            Self::GnawboneCrown => "Gnawbone Crown",
            Self::TollcoinCharm => "Tollcoin Charm",
            Self::WisplightLantern => "Wisplight Lantern",
            Self::Hartshorn => "Hartshorn",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Self::Rallybreaker => "+2 damage when an attack immediately follows defending.",
            Self::Fangmantle => "+1 defense while two or more enemies are adjacent.",
            Self::Graveglass => "Runes cost 1 less mana, to a minimum of 1.",
            Self::Saltcrown => "No tide, shove or pull can move you one tile against your will.",
            Self::Stoneheart => "+1 defense; rubble and rough ground toll you no more than a road.",
            Self::GnawboneCrown => "Rats never turn on you; +1 stamina restored by dungeon moves.",
            Self::TollcoinCharm => "Bribes and tolls cost half; +1 attack on road and ford tiles.",
            Self::WisplightLantern => "+2 light radius at night.",
            Self::Hartshorn => "+1 speed; fleeing enemies never recover while you pursue.",
        }
    }
}
/// The Vigil's fallen orders (docs/D2_EVOLUTION.md §4.2). `None` survives only for
/// saves written before class creation shipped — creation offers the six orders only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    #[default]
    None,
    Keepwarden,
    Gravebound,
    Redwake,
    Waysworn,
    SigilSworn,
    Fensworn,
}
impl Class {
    /// Create-screen order: Ward pair, Hunt pair, Speaker pair.
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Keepwarden,
            1 => Self::Gravebound,
            2 => Self::Redwake,
            3 => Self::Waysworn,
            4 => Self::SigilSworn,
            5 => Self::Fensworn,
            _ => Self::None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Unsworn",
            Self::Keepwarden => "Keepwarden",
            Self::Gravebound => "Gravebound",
            Self::Redwake => "Redwake",
            Self::Waysworn => "Waysworn",
            Self::SigilSworn => "Sigil-Sworn",
            Self::Fensworn => "Fensworn",
        }
    }
    pub fn calling(self) -> &'static str {
        match self {
            Self::Keepwarden | Self::Gravebound => "Ward",
            Self::Redwake | Self::Waysworn => "Hunt",
            Self::SigilSworn | Self::Fensworn => "Speaker",
            Self::None => "Free",
        }
    }
    /// One-line identity blurb for the creation screen.
    pub fn blurb(self) -> &'static str {
        match self {
            Self::None => "No order claimed you. The road judges everyone alike.",
            Self::Keepwarden => "The gate holds. Garrison drilled in the brace-and-stand.",
            Self::Gravebound => "Keeper of funerary rites. Fights hardest when nearly fallen.",
            Self::Redwake => "Saltmarsh gutter-royalty. Kill, shift, strike — tide tactics.",
            Self::Waysworn => "Road warden. The oathroads still remember their keepers.",
            Self::SigilSworn => "Oracle schismatic. Wields seal-script, not just guards it.",
            Self::Fensworn => "Hollow folk. Walks with beasts as equals; the pack knows your name.",
        }
    }
    /// Mechanical diff line previewed on the creation screen.
    pub fn diff(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Keepwarden => "+4 HP, -2 mana (floor 2). Oath defends: Hold / Break / Breathe.",
            Self::Gravebound => "+2 HP, -1 SPD. Last Vigil: +2 ATK below half HP, +4 below quarter.",
            Self::Redwake => "+2 SPD, -1 DEF, +1 stamina on wait/brace. Kills and escapes bank.",
            Self::Waysworn => "+1 SPD, +2 stamina, -2 HP. Roads/fords half; kills open the road.",
            Self::SigilSworn => "Spark + 4 mana, -3 HP. Channel: same rune twice charges it.",
            Self::Fensworn => "Hire 25g, partner +25% HP. Every third partner strike doubles.",
        }
    }
}
/// Keepwarden's Bulwark stances (§4.2, D21). Hold matches the classic brace; the
/// default keeps every other class's defend behaviour identical to pre-class balance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OathStance {
    #[default]
    Hold,
    Break,
    Breathe,
}
impl OathStance {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hold => "Hold the Gate",
            Self::Break => "Break Their Line",
            Self::Breathe => "Breathe",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Self::Hold => "+50% defense; your first attacker each round takes 1 recoil.",
            Self::Break => "Half defense this round, +2 to your next strike.",
            Self::Breathe => "No defense bonus; +1 extra stamina back.",
        }
    }
}
/// Origin boon offered once at creation (D2 ratified).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boon {
    None,
    Gold,
    Stamina,
}
impl Boon {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Take nothing",
            Self::Gold => "Tithe pouch",
            Self::Stamina => "Drilled marches",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Self::None => "The Vigil owes you nothing; you owe it less.",
            Self::Gold => "+20 gold — a pouch the tithe box held back.",
            Self::Stamina => "+1 max stamina — you marched the oathroads before you were asked.",
        }
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Gold,
            1 => Self::Stamina,
            _ => Self::None,
        }
    }
}
/// One talent node (§5): tiers gate at player level 1/3/6/9; capstones cost 2 points.
#[derive(Clone, Copy, Debug)]
pub struct TalentNodeDef {
    pub name: &'static str,
    pub level: u8,
    pub cost: u8,
    pub text: &'static str,
}
/// A branch of four; each class tree holds three (§5: 3×4, 12 points over the campaign).
#[derive(Clone, Copy, Debug)]
pub struct TalentBranchDef {
    pub name: &'static str,
    pub nodes: [TalentNodeDef; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct TalentTreeDef {
    pub branches: [TalentBranchDef; 3],
}
const fn n(name: &'static str, level: u8, cost: u8, text: &'static str) -> TalentNodeDef {
    TalentNodeDef {
        name,
        level,
        cost,
        text,
    }
}
const fn b(name: &'static str, nodes: [TalentNodeDef; 4]) -> TalentBranchDef {
    TalentBranchDef { name, nodes }
}
const KEEPWARDEN_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Bulwark",
            [
                n("Plate Drills", 1, 1, "+1 DEF while braced."),
                n("Second Wind", 3, 1, "Braced rounds restore 2 extra stamina."),
                n("Anchor", 6, 1, "Displacements pull you 1 tile less; +1 recoil per Bulwark 1–2."),
                n("Last Bastion", 9, 2, "Once per combat: a lethal hit on your brace leaves 1 HP."),
            ],
        ),
        b(
            "Oathblade",
            [
                n("Drilled Counters", 1, 1, "The first strike after each defend deals +2."),
                n("Judgment", 3, 1, "Braced: flankers lose their flank bonus on you."),
                n("Overwatch", 6, 1, "Enemies stepping into your brace's reach take 1 (+1 per Oathblade 1–2)."),
                n("Gatecrash", 9, 2, "Once per combat: the first strike after a defend deals +4."),
            ],
        ),
        b(
            "Vigilator",
            [
                n("Watch-Fires", 1, 1, "+2 sight radius at night."),
                n("Discipline", 3, 1, "+1 max stamina."),
                n("Sentinel", 6, 1, "+1 DEF in dungeons."),
                n("Unyielding", 9, 2, "Once per combat: the first curse does not take."),
            ],
        ),
    ],
};
const GRAVEBOUND_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Sepulchre",
            [
                n("Rite of Names", 1, 1, "Last Vigil opens earlier: +2 ATK below 60% HP."),
                n("Gravedigger's Pace", 3, 1, "Kills below half HP restore 2 HP."),
                n("Widow's Lantern", 6, 1, "Below a quarter HP: +2 more ATK; Gravedigger heals +1 per Sepulchre 1–2."),
                n("Methuselah Vow", 9, 2, "Once per combat: a lethal hit leaves 1 HP, braced or not."),
            ],
        ),
        b(
            "Censer",
            [
                n("Ash-Writhe", 1, 1, "Enemies adjacent to you below half HP strike with -1."),
                n("Bell Toll", 3, 1, "First time you drop below half in a combat, adjacent enemies lose 4 beats."),
                n("Hollow Court", 6, 1, "Your flee margin is +2 while below half HP."),
                n("Requiem", 9, 2, "Kills below a quarter HP restore all stamina."),
            ],
        ),
        b(
            "Barrown",
            [
                n("Grave Goods", 1, 1, "Kills yield +1 gold."),
                n("Grave Comforts", 3, 1, "Potions heal +4 more."),
                n("Paupers' Saint", 6, 1, "Dungeon movement costs half time; Grave Goods +1 per Barrown 1–2."),
                n("Plunder the Ossuary", 9, 2, "Once per combat below a quarter HP: a potion drinks itself, free."),
            ],
        ),
    ],
};
const REDWAKE_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Assassin",
            [
                n("Salt-Blood", 1, 1, "Momentum banks to 4."),
                n("Cutpurse", 3, 1, "Kills yield +3 gold."),
                n("Ebb", 6, 1, "After each flee, the next kill banks 2; momentum spends +1 damage per Assassin 1–2."),
                n("Red Tide", 9, 2, "A strike at a full bank ignores defense."),
            ],
        ),
        b(
            "Dockhand",
            [
                n("Gutter-Law", 1, 1, "Bounties pay +20%."),
                n("Rotgut", 3, 1, "Potions also restore 3 stamina."),
                n("Fence's Friend", 6, 1, "Sales pay 25% more; bounties +5% per Dockhand 1–2."),
                n("Crow-Broker", 9, 2, "Once per combat: a spare attempt lands when its target is below half."),
            ],
        ),
        b(
            "Foamrunner",
            [
                n("Rip Line", 1, 1, "Fled enemies regroup slower: cooldown 12."),
                n("Second Tide", 3, 1, "The first failed flee each combat costs no stamina."),
                n("Slipstream", 6, 1, "+1 speed."),
                n("Rip Current", 9, 2, "Once per combat, any flee attempt escapes."),
            ],
        ),
    ],
};
const WAYSWORN_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Outrider",
            [
                n("Steady Bit", 1, 1, "Exploration moves restore 2 stamina."),
                n("Waylaid Reading", 3, 1, "Ambush rounds cannot flank you."),
                n("Track Stand", 6, 1, "The road banks 2 trail charges; each refunds +1 stamina per Outrider 1–2."),
                n("Road-Sworn", 9, 2, "Once per combat: a kill on road or ford strikes again at once."),
            ],
        ),
        b(
            "Tollwright",
            [
                n("Rights of Way", 1, 1, "Bribes and tolls cost half."),
                n("Quartering", 3, 1, "Rations and resting heal +2 more."),
                n("Oathpath Discount", 6, 1, "Forge prices -10%."),
                n("Caravan Code", 9, 2, "Bounty work pays double."),
            ],
        ),
        b(
            "Farwanderer",
            [
                n("Landmark Lore", 1, 1, "+1 sight radius by day."),
                n("Forager", 3, 1, "Resting heals +4 more."),
                n("Pathfinder", 6, 1, "Mountains cost 150; Forager heals +2 per Farwanderer 1–2."),
                n("Surveyor", 9, 2, "Each map's gates show themselves on arrival."),
            ],
        ),
    ],
};
const SIGIL_SWORN_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Storm",
            [
                n("Conductor", 1, 1, "The channel holds 3 charges."),
                n("Static Pact", 3, 1, "A broken channel keeps 1 charge."),
                n("Overcharge", 6, 1, "Charged sparks splash 1 (+1 per Storm 1–2) to enemies beside the target."),
                n("Tempest", 9, 2, "Each combat's first rune costs 0 mana."),
            ],
        ),
        b(
            "Ink",
            [
                n("Seal-Reader", 1, 1, "Oracle tuition costs 10 less."),
                n("Redact", 3, 1, "Wards hold a third round."),
                n("Illuminator", 6, 1, "Mend heals +4 (+1 more per Ink 1–2)."),
                n("Wordsmith", 9, 2, "Once per combat: cast one rune free; the anchor resets."),
            ],
        ),
        b(
            "Focus",
            [
                n("Plumb Line", 1, 1, "Your runes deal +1 damage."),
                n("Still Water", 3, 1, "Warded defense bonus is +3 instead of +2."),
                n("Slow Burn", 6, 1, "Spark kills grant +1 charge."),
                n("Cascade", 9, 2, "Each combat's first spark repeats onto a second target at half force."),
            ],
        ),
    ],
};
const FENSWORN_TREE: TalentTreeDef = TalentTreeDef {
    branches: [
        b(
            "Alpha",
            [
                n("Lean Bite", 1, 1, "The partner's strikes deal +1."),
                n("Thick Coat", 3, 1, "The partner takes -1 from every hit."),
                n("Bloodline", 6, 1, "The doubled strike heals the partner 2 (+1 per Alpha 1–2)."),
                n("Packlord", 9, 2, "Every second striking round doubles."),
            ],
        ),
        b(
            "Fenlore",
            [
                n("Sedgeskin", 1, 1, "Forest and deep-forest tolls halved."),
                n("Camouflage", 3, 1, "Enemies spot you one step closer."),
                n("Swampwise", 6, 1, "Moves restore 1 extra stamina off the Road and cities."),
                n("Bogside Vigil", 9, 2, "Once per combat: the partner stands at 5 instead of falling."),
            ],
        ),
        b(
            "Trapper",
            [
                n("Snare Craft", 1, 1, "Kills on forest tiles restore 1 stamina."),
                n("Skinning", 3, 1, "Beast kills yield +2 gold."),
                n("Lure", 6, 1, "Your flee margin is +1; Snare heals +1 per Trapper 1–2."),
                n("Hollow Caller", 9, 2, "Once per combat: an engaging beast loses 4 beats."),
            ],
        ),
    ],
};
/// The talent tree of an order, or `None` for pre-class saves (E2, D27).
pub fn talent_tree(class: Class) -> Option<&'static TalentTreeDef> {
    match class {
        Class::Keepwarden => Some(&KEEPWARDEN_TREE),
        Class::Gravebound => Some(&GRAVEBOUND_TREE),
        Class::Redwake => Some(&REDWAKE_TREE),
        Class::Waysworn => Some(&WAYSWORN_TREE),
        Class::SigilSworn => Some(&SIGIL_SWORN_TREE),
        Class::Fensworn => Some(&FENSWORN_TREE),
        Class::None => None,
    }
}
/// Flattened 0..12 node lookup for a class tree (branch-major).
pub fn talent_def(class: Class, index: u8) -> Option<&'static TalentNodeDef> {
    let tree = talent_tree(class)?;
    let branch = index as usize / 4;
    let tier = index as usize % 4;
    tree.branches.get(branch).map(|b| &b.nodes[tier])
}
/// Per-combat one-shot bits (cleared when a combat begins; §5 capstones and novas).
pub mod once {
    pub const BASTION: u32 = 1;
    pub const METHUSELAH: u32 = 2;
    pub const UNYIELDING: u32 = 4;
    pub const GATECRASH: u32 = 8;
    pub const TEMPEST: u32 = 16;
    pub const WORDSMITH: u32 = 32;
    pub const CROW_BROKER: u32 = 64;
    pub const RIP_CURRENT: u32 = 128;
    pub const OSSUARY: u32 = 256;
    pub const BELL: u32 = 512;
    pub const BOGSIDE: u32 = 1024;
    pub const ROAD_SWORN: u32 = 2048;
    pub const SECOND_TIDE: u32 = 4096;
    pub const CASCADE: u32 = 8192;
    // E7 (§8.1): the Vigil word's once-per-combat first-hit refusal.
    pub const VIGIL_WORD: u32 = 16384;
}
/// Signature counters for the class mechanics (§4.3 meters visualize these — no new
/// spending economy). All fields default for old saves.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ClassState {
    pub stance: OathStance,
    pub resolve: u8,
    pub break_armed: bool,
    pub recoil_used_round: Option<u64>,
    pub channel: u8,
    pub channel_spell: Option<Spell>,
    pub bond: u8,
    pub momentum: u8,
    /// Banked Open-Road charges: 0..=1 normally, 0..=2 with Track Stand.
    pub trail_charges: u8,
    /// Ebb: the kill after a Redwake flee banks double.
    pub ebb_armed: bool,
    /// Drilled Counters: the brace's answer is cocked.
    pub riposte_armed: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub kills: u32,
    pub mercy: u32,
    pub thefts: u32,
    pub bribes: u32,
    pub fled: u32,
    pub quests: u32,
    pub deaths: u32,
    pub attacks: u32,
    pub defended: u32,
    pub potions: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub map: usize,
    pub pos: Pos,
    pub hp: i32,
    pub max_hp: i32,
    pub stamina: i32,
    pub max_stamina: i32,
    pub attack: i32,
    pub defense: i32,
    pub speed: i32,
    pub level: u32,
    pub xp: u32,
    pub gold: u32,
    pub inventory: Vec<Item>,
    pub weapon: u8,
    pub armour: u8,
    #[serde(default)]
    pub relic: Option<BossRelic>,
    pub reputation: [i32; 3],
    pub sigils: [bool; 3],
    pub keys: [bool; 3],
    pub last_city: usize,
    pub defending: bool,
    pub torch_until: u64,
    pub curse_until: u64,
    pub last_hazard: String,
    pub mana: i32,
    pub max_mana: i32,
    pub spells: Vec<Spell>,
    pub ward_until: u64,
    pub history: History,
    #[serde(default)]
    pub class: Class,
    /// Chosen at creation. Presentation only — no stat reads it.
    #[serde(default)]
    pub build: Build,
    #[serde(default)]
    pub class_state: ClassState,
    /// Points spent, one entry per point; node index = branch*4 + tier (§5).
    #[serde(default)]
    pub talents: Vec<u8>,
    /// Points earned over the campaign: 1 per level-up (L2–L10) + 1 per sigil claim = 12.
    #[serde(default)]
    pub talent_points: u8,
    /// Per-combat one-shot bits (the `once` module); cleared at every combat start.
    #[serde(default)]
    pub talent_once: u32,
    /// The one free Oracle reset (D8); essence-craft resets arrive with E5 content.
    #[serde(default)]
    pub respec_used: bool,
    /// Fester stacks (D36): barrow bites fester; every third stack costs 1 max
    /// stamina, paid at infection and returned when the alchemist or an inn cures it.
    #[serde(default)]
    pub fester: u8,
    /// Anti-toxin afterglow: bites taken before this turn cannot stack fester.
    #[serde(default)]
    pub fester_guard_until: u64,
    /// E7 (D31 §8.1): the Word written into the equipped weapon/armour pieces
    /// (index into WORDS). Plain gear only — relics never socket.
    #[serde(default)]
    pub word_weapon: Option<u8>,
    #[serde(default)]
    pub word_armour: Option<u8>,
}
impl Player {
    /// Points currently held at one node (0..2 where 2 completes a capstone).
    pub fn talent_rank(&self, index: u8) -> u8 {
        self.talents.iter().filter(|&&t| t == index).count() as u8
    }
    /// Whether a node's effect is live (fully paid).
    pub fn talent_active(&self, index: u8) -> bool {
        talent_def(self.class, index)
            .is_some_and(|def| self.talent_rank(index) >= def.cost)
    }
    pub fn has_talent(&self, branch: u8, tier: u8) -> bool {
        self.talent_active(branch * 4 + tier)
    }
    /// §5 mini-synergy: points held in a branch's first two nodes feed its third.
    pub fn branch_synergy(&self, branch: u8) -> u8 {
        self.talent_rank(branch * 4) + self.talent_rank(branch * 4 + 1)
    }
    /// Unspent treasury.
    pub fn talent_points_available(&self) -> u8 {
        self.talent_points.saturating_sub(self.talents.len() as u8)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStage {
    Locked,
    Available,
    Active,
    Ready,
    Complete,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub title: String,
    pub stage: QuestStage,
    pub progress: u32,
    pub goal: u32,
    pub description: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Modal {
    None,
    Title,
    Create,
    Help,
    Pause,
    Inventory,
    Journal,
    Atlas,
    Trade(usize),
    Talk(usize),
    Cast,
    Forge(usize),
    Oath,
    Talents,
    Death,
    Victory,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Combat {
    pub center: Pos,
    pub round: u64,
    pub participants: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FloatingText {
    pub pos: Pos,
    pub text: String,
    pub ttl: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Question {
    #[serde(rename = "type")]
    pub kind: String,
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Answer {
    pub value: f32,
    pub choice: Option<String>,
    pub probabilities: BTreeMap<String, f32>,
    pub confidence: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub seed: u64,
    pub npc: usize,
    pub npc_name: String,
    pub version: u64,
    pub turn: u64,
    pub tier: u8,
    pub state: Value,
    pub questions: BTreeMap<String, Question>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionResult {
    pub request: DecisionRequest,
    pub provider: String,
    pub answers: BTreeMap<String, Answer>,
    pub latency_ms: u64,
    pub input_tokens: u64,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub result: DecisionResult,
    pub applied_rule: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiStats {
    pub provider: String,
    pub requests: u64,
    pub degraded: u64,
    pub consecutive_failures: u32,
    pub switched: bool,
    pub input_tokens: u64,
    pub latency_ms: u64,
    pub pending: usize,
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Move(i32, i32),
    Attack,
    Interact,
    Wait,
    Rest,
    Defend,
    Flee,
    Mercy,
    Use(usize),
    Buy(usize),
    Sell(usize),
    Haggle,
    Talk(usize),
    Travel(usize),
    Oracle,
    Cast(usize),
    Forge(usize),
    OathPledge(usize),
    Learn(usize),
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SocialEvent {
    Greet,
    Haggle,
    Passage,
    Bribe,
    Rumour,
    Reclaim,
    Mercy,
    Prophecy,
    Identify,
    Study,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BountyKind {
    Cull(Archetype),
    Deliver(usize),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bounty {
    pub kind: BountyKind,
    pub origin: usize,
    pub gold: u32,
    pub xp: u32,
    pub reputation: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub seed: u64,
    pub rng: u64,
    pub maps: Vec<Map>,
    pub npcs: Vec<Npc>,
    pub player: Player,
    pub tick: u64,
    pub turn: u64,
    pub hour_ticks: u64,
    pub combat: Option<Combat>,
    pub modal: Modal,
    pub selected: usize,
    pub inspector: bool,
    pub log: VecDeque<String>,
    pub decisions: VecDeque<DecisionRecord>,
    pub outbox: Vec<DecisionRequest>,
    pub ai: AiStats,
    pub quests: Vec<Quest>,
    pub effects: Vec<FloatingText>,
    pub visited: [bool; 3],
    pub haggle: Option<(usize, u32)>,
    pub move_ready_ms: u64,
    pub elapsed_ms: u64,
    pub quit: bool,
    pub won: bool,
    pub rats_killed: u32,
    pub caravan_recovered: bool,
    pub smuggling_choice: Option<bool>,
    pub social_pending: Option<(usize, u64, SocialEvent)>,
    pub gate_permit: Option<(usize, usize)>,
    pub recent_actions: VecDeque<String>,
    pub companion: Option<usize>,
    pub bounties: Vec<Bounty>,
    pub next_bounty: u64,
    #[serde(default)]
    pub atlas_zoom: bool,
    #[serde(default)]
    pub create_step: u8,
    #[serde(default)]
    pub create_class: usize,
    /// Body build chosen on create step 1; `Build::from_index` of this.
    #[serde(default)]
    pub create_build: usize,
    /// Settled avalanche ground: (map, pos, original tile) restored when a combat ends (E3).
    #[serde(default)]
    pub rubble: Vec<(usize, Pos, Tile)>,
    /// Tollmaster's sown ground: (map, pos, original tile); bites once, then restores.
    #[serde(default)]
    pub caltrops: Vec<(usize, Pos, Tile)>,
    /// E5 bounty escalation (§6): cull contracts settled per kind — [wolves, bandits].
    /// Three of one kind earns the captain's tip-off about the Pale Stag.
    #[serde(default)]
    pub cull_wins: [u8; 2],
    /// D13 cruel terms: sworn at the Trial's shrine; the sworn cannot flee judgment.
    #[serde(default)]
    pub trial_oath: bool,
    /// E7 (§6.5): sigils the Curate's Unwrit has silenced for the current fight;
    /// cleared when combat ends, like rubble.
    #[serde(default)]
    pub curate_unwritten: [bool; 3],
    /// E7 (§6.5): the player showed the oathbreaker mercy — the Adjudicator's
    /// final rating reads this sincerity.
    #[serde(default)]
    pub mercy_oathbreaker: bool,
    /// E7 (§6.5): the arbiter's question, asked at the verdict shrine.
    #[serde(default)]
    pub epilogue: Epilogue,
    /// E7 (D32): transmute recipes learned permanently on first success (Qud memory).
    #[serde(default)]
    pub codex: Vec<String>,
    /// E7 (D32): the one-per-run masterwork anvil beat is spent.
    #[serde(default)]
    pub masterwork_used: bool,
    /// E7 (§8.2): the seeded one-shot scrounge sites (0 herb, 1 ore), and the
    /// ones already stripped. Old saves simply have no sites planted.
    #[serde(default)]
    pub scrounge_sites: Vec<(Pos, u8)>,
    #[serde(default)]
    pub scrounged: Vec<(usize, Pos)>,
    /// E7 (§6.5): the six class-quest trials, indexed by trial_index(class).
    #[serde(default)]
    pub trials: [ClassTrial; 6],
}
impl Game {
    pub fn log(&mut self, message: impl Into<String>) {
        if self.log.len() >= 80 {
            self.log.pop_front();
        }
        self.log.push_back(message.into());
    }
    pub fn random(&mut self, upper: u32) -> u32 {
        self.rng = mix(self.rng);
        if upper == 0 {
            0
        } else {
            (self.rng % upper as u64) as u32
        }
    }
    pub fn hour(&self) -> u32 {
        ((8 + self.hour_ticks / 480) % 24) as u32
    }
    pub fn night(&self) -> bool {
        let h = self.hour();
        !(6..20).contains(&h)
    }
    pub fn map(&self) -> &Map {
        &self.maps[self.player.map]
    }
    pub fn npc_at(&self, pos: Pos) -> Option<usize> {
        self.npcs
            .iter()
            .position(|n| n.alive() && n.map == self.player.map && n.pos == pos)
    }
    pub fn city(&self) -> Option<usize> {
        if let MapKind::City(i) = self.map().kind {
            Some(i)
        } else {
            None
        }
    }
    pub fn attack_power(&self) -> i32 {
        self.player.attack
            + i32::from(self.player.weapon) * 3
            + if self.player.class == Class::Gravebound {
                // Last Vigil: the deeper the wound, the harder the stand (§4.2).
                // Widow's Lantern adds +2 more in the deep tier; Rite of Names opens early.
                let half_open = if self.player.hp * 2 < self.player.max_hp {
                    true
                } else {
                    self.player.has_talent(0, 0) && self.player.hp * 10 < self.player.max_hp * 6
                };
                if self.player.hp * 4 < self.player.max_hp {
                    4 + i32::from(self.player.has_talent(0, 2)) * 2
                } else if half_open {
                    2
                } else {
                    0
                }
            } else {
                0
            }
            // Tollcoin Charm (§6): the ford's own steel bites on road and ford tiles.
            + i32::from(
                self.player.relic == Some(BossRelic::TollcoinCharm)
                    && matches!(self.map().tile(self.player.pos), Tile::Road | Tile::Ford),
            )
            // Ember (§8.1): the written blade scorches.
            + self.word_on(WordSlot::Weapon, "Ember") * 2
    }
    /// Hartshorn (§6): the hunt quickens the hunter; Gale reads faster still.
    pub fn player_speed(&self) -> i32 {
        self.player.speed
            + i32::from(self.player.relic == Some(BossRelic::Hartshorn))
            + self.word_on(WordSlot::Armour, "Gale")
    }
    /// §8.1: does the named Word live on the player's gear in the named slot?
    pub fn word_on(&self, slot: WordSlot, name: &str) -> i32 {
        let word = match slot {
            WordSlot::Weapon => self.player.word_weapon,
            WordSlot::Armour => self.player.word_armour,
        };
        let lives = word
            .and_then(|w| WORDS.get(w as usize))
            .is_some_and(|def| def.name == name && def.slot == slot);
        i32::from(lives)
    }
    /// E7 (§6.5): has the Curate's Unwrit silenced this sigil for this fight?
    pub fn sigil_unwritten(&self, index: usize) -> bool {
        self.curate_unwritten.get(index).copied().unwrap_or(false)
    }
    /// Defending defense after the stance pick. Hold is the classic 1.5x brace, so
    /// non-Keepwarden classes (default Hold) keep their pre-class numbers exactly.
    /// Plate Drills (Bulwark 1) adds its flat point before the multiplier.
    pub fn braced_defense(&self) -> i32 {
        let plated = i32::from(
            self.player.class == Class::Keepwarden && self.player.has_talent(0, 0),
        );
        let defense = self.defense_power() + plated;
        match self.player.class_state.stance {
            OathStance::Hold => (defense * 3 + 1) / 2,
            OathStance::Break => defense / 2,
            OathStance::Breathe => defense,
        }
    }
    /// Effective movement-time cost of a tile; Waysworn oaths halve road and ford tolls (§4.2).
    pub fn terrain_cost(&self, tile: Tile) -> u32 {
        let cost = tile.cost();
        if self.player.relic == Some(BossRelic::Stoneheart) {
            // Stoneheart: rough ground is a road under your step.
            return cost.min(Tile::Road.cost());
        }
        match (self.player.class, tile) {
            (Class::Waysworn, Tile::Road | Tile::Ford) => (cost / 2).max(1),
            (Class::Waysworn, Tile::Mountain) if self.player.has_talent(2, 2) => 150,
            (Class::Fensworn, Tile::Forest | Tile::DeepForest) if self.player.has_talent(1, 0) => {
                (cost / 2).max(1)
            }
            (Class::Gravebound, _) if self.player.has_talent(2, 2) && self.in_dungeon() => {
                (cost / 2).max(1)
            }
            _ => cost,
        }
    }
    pub fn in_dungeon(&self) -> bool {
        matches!(self.map().kind, MapKind::Dungeon(..))
    }
    /// Redwake bank ceiling; Salt-Blood raises it to 4 (§5 Assassin 1).
    pub fn momentum_cap(&self) -> u8 {
        if self.player.class == Class::Redwake {
            3 + u8::from(self.player.has_talent(0, 0))
        } else {
            3
        }
    }
    /// Sigil-Sworn channel ceiling; Conductor raises it to 3 (§5 Storm 1).
    pub fn channel_cap(&self) -> u8 {
        if self.player.class == Class::SigilSworn {
            2 + u8::from(self.player.has_talent(0, 0))
        } else {
            2
        }
    }
    /// Sigil-Sworn channel breaks on movement and melee (D24: wait, items, defend preserve).
    /// Static Pact (Storm 2): a broken anchor keeps one charge.
    pub fn break_channel(&mut self) {
        let keep = self.player.class == Class::SigilSworn
            && self.player.has_talent(0, 1)
            && self.player.class_state.channel > 0;
        self.player.class_state.channel = u8::from(keep);
        self.player.class_state.channel_spell = None;
    }
    pub fn defense_power(&self) -> i32 {
        let base = self.player.defense
            + i32::from(self.player.armour) * 2
            + i32::from(self.player.relic == Some(BossRelic::Stoneheart))
            + self.word_on(WordSlot::Armour, "Vigil")
            + if self.in_dungeon() {
                self.word_on(WordSlot::Armour, "Censer")
            } else {
                0
            };
        (base
            + if self.player.relic == Some(BossRelic::Fangmantle)
                && !self.sigil_unwritten(1)
                && self
                    .npcs
                    .iter()
                    .filter(|npc| {
                        npc.alive()
                            && npc.map == self.player.map
                            && npc.archetype.hostile()
                            && npc.pos.distance(self.player.pos) <= 1
                            && !npc
                                .memory
                                .events
                                .iter()
                                .any(|event| event.kind == "player_spared_me")
                    })
                    .take(2)
                    .count()
                    == 2
            {
                1
            } else {
                0
            }
            + if self.turn < self.player.ward_until {
                // Still Water (Focus 2): the ward answers with +3, not +2.
                2 + i32::from(
                    self.player.class == Class::SigilSworn && self.player.has_talent(2, 1),
                )
            } else {
                0
            }
            + i32::from(
                // Sentinel (Vigilator 3): the order keeps its garrison habits.
                self.player.class == Class::Keepwarden
                    && self.player.has_talent(2, 2)
                    && self.in_dungeon(),
            )
            - if self.turn < self.player.curse_until {
                2
            } else {
                0
            })
        .max(0)
    }
    pub fn spell_cost(&self, spell: Spell) -> i32 {
        (spell.cost()
            - if self.player.relic == Some(BossRelic::Graveglass) && !self.sigil_unwritten(2) {
                1
            } else {
                0
            })
        .max(1)
    }
    pub fn reveal(&mut self) {
        let p = self.player.pos;
        // Fog clearing mirrors the drawn vision radius exactly: torchlight clears
        // six tiles at night, never the daylight radius.
        let radius = if !self.night() {
            // Landmark Lore (Farwanderer 1): the roads read one step farther.
            12 + i32::from(self.player.class == Class::Waysworn && self.player.has_talent(2, 0))
        } else if self.tick < self.player.torch_until {
            // Watch-Fires (Vigilator 1): the garrison pattern extends torchlight.
            6 + if self.player.class == Class::Keepwarden && self.player.has_talent(2, 0) {
                2
            } else {
                0
            } + if self.player.relic == Some(BossRelic::WisplightLantern) {
                2
            } else {
                0
            }
        } else {
            4 + if self.player.class == Class::Keepwarden && self.player.has_talent(2, 0) {
                2
            } else {
                0
            } + if self.player.relic == Some(BossRelic::WisplightLantern) {
                2
            } else {
                0
            }
        };
        let map = &mut self.maps[self.player.map];
        for y in (p.y - radius)..=(p.y + radius) {
            for x in (p.x - radius)..=(p.x + radius) {
                if let Some(i) = map.index(Pos::new(x, y)) {
                    map.explored[i] = true;
                }
            }
        }
    }
}
pub fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
