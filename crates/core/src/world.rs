use crate::model::*;
use std::collections::{BTreeMap, VecDeque};

pub const CITY_SITES: [Pos; 3] = [Pos::new(35, 115), Pos::new(145, 40), Pos::new(164, 132)];
pub const CARAVAN_SITE: Pos = Pos::new(102, 65);
pub const CARAVAN_PACK: usize = 900;
pub const PARCEL_SITE: Pos = Pos::new(32, 31);

fn roll(seed: u64, salt: u64, upper: u64) -> u64 {
    mix(seed ^ salt.wrapping_mul(0x9e3779b97f4a7c15)) % upper
}

fn rect(map: &mut Map, left: i32, top: i32, right: i32, bottom: i32, tile: Tile) {
    for y in top..=bottom {
        for x in left..=right {
            map.set(Pos::new(x, y), tile);
        }
    }
}

fn path(map: &mut Map, from: Pos, to: Pos, tile: Tile) {
    let mut p = from;
    map.set(p, tile);
    while p.x != to.x {
        p.x += (to.x - p.x).signum();
        map.set(p, tile);
    }
    while p.y != to.y {
        p.y += (to.y - p.y).signum();
        map.set(p, tile);
    }
}

fn route(map: &mut Map, points: &[Pos], tile: Tile) {
    for pair in points.windows(2) {
        path(map, pair[0], pair[1], tile);
    }
}

/// Dungeon room centres, row-major; each carves a single chamber.
const DUNGEON_CENTERS: [Pos; 9] = [
    Pos::new(5, 5),
    Pos::new(15, 5),
    Pos::new(24, 5),
    Pos::new(5, 15),
    Pos::new(15, 15),
    Pos::new(24, 15),
    Pos::new(5, 24),
    Pos::new(15, 24),
    Pos::new(24, 24),
];

/// The floor rect carved around a dungeon room centre, clipped to the 27-tile
/// inner extent so the south and east chambers keep a wall against the border.
fn room_bounds(center: Pos) -> (i32, i32, i32, i32) {
    (
        center.x - 3,
        center.y - 3,
        (center.x + 3).min(27),
        (center.y + 3).min(27),
    )
}

/// The first tile of the corridor from `from` toward `to` that lies outside
/// `from`'s chamber: the one-tile mouth where a door can stand a threshold.
/// Steps exactly as `path` does (x first, then y), so the mouth is always on the
/// corridor the generator actually carved. `None` when the chambers touch.
fn room_mouth(from: Pos, to: Pos) -> Option<Pos> {
    let (left, top, right, bottom) = room_bounds(from);
    let inside = |p: Pos| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;
    let mut p = from;
    while p != to {
        if p.x != to.x {
            p.x += (to.x - p.x).signum();
        } else {
            p.y += (to.y - p.y).signum();
        }
        if !inside(p) {
            return Some(p);
        }
    }
    None
}

fn overworld(seed: u64) -> Map {
    let mut map = Map::new(
        "The Three Realms",
        MapKind::Overworld,
        200,
        160,
        Tile::Grass,
    );
    for y in 1..159 {
        for x in 1..199 {
            let p = Pos::new(x, y);
            let patch = roll(seed, (x / 9 + (y / 8) * 23) as u64, 100);
            let scatter = roll(seed ^ 719, (x + y * 200) as u64, 100);
            let tile = if patch < 17 && scatter < 77 {
                Tile::DeepForest
            } else if patch < 54 && scatter < 74 {
                Tile::Forest
            } else if scatter == 99 && patch > 75 {
                Tile::Ruins
            } else {
                Tile::Grass
            };
            map.set(p, tile);
        }
    }
    rect(&mut map, 0, 0, 199, 0, Tile::Rock);
    rect(&mut map, 0, 159, 199, 159, Tile::Rock);
    rect(&mut map, 0, 0, 0, 159, Tile::Rock);
    rect(&mut map, 199, 0, 199, 159, Tile::Rock);
    // The ridge has two actual passes; the river has exactly two crossings.
    rect(&mut map, 1, 20, 198, 26, Tile::Mountain);
    rect(&mut map, 1, 23, 198, 24, Tile::Rock);
    rect(&mut map, 39, 20, 41, 26, Tile::Mountain);
    rect(&mut map, 144, 20, 146, 26, Tile::Mountain);
    rect(&mut map, 99, 1, 101, 158, Tile::River);
    for points in [
        vec![
            Pos::new(35, 115),
            Pos::new(35, 65),
            Pos::new(145, 65),
            Pos::new(145, 12),
        ],
        vec![
            Pos::new(35, 115),
            Pos::new(35, 120),
            Pos::new(164, 120),
            Pos::new(164, 132),
        ],
        vec![Pos::new(145, 65), Pos::new(145, 120)],
        vec![Pos::new(52, 78), Pos::new(52, 65)],
        vec![Pos::new(158, 82), Pos::new(145, 82)],
        vec![Pos::new(72, 44), Pos::new(72, 65)],
        vec![Pos::new(69, 104), Pos::new(35, 104)],
        vec![Pos::new(40, 12), Pos::new(40, 65)],
    ] {
        route(&mut map, &points, Tile::Road);
    }
    for y in [65, 120] {
        rect(&mut map, 99, y, 101, y, Tile::Ford);
    }
    // Side expeditions are optional; none stand between a city and the next quest.
    for i in 0..22 {
        let east = i % 2 == 0;
        let x = if east {
            110 + roll(seed, 4100 + i, 76) as i32
        } else {
            8 + roll(seed, 4100 + i, 80) as i32
        };
        let y = 32 + roll(seed, 5100 + i, 119) as i32;
        let p = Pos::new(x, y);
        if map.tile(p) != Tile::Road && CITY_SITES.iter().all(|c| c.distance(p) > 5) {
            rect(&mut map, x - 1, y - 1, x + 1, y + 1, Tile::Ruins);
            map.set(p, Tile::Chest);
        }
    }
    for p in [Pos::new(69, 104), Pos::new(175, 52), Pos::new(25, 42)] {
        map.set(p, Tile::Shrine);
    }
    map.set(CARAVAN_SITE, Tile::Chest);
    map
}

fn city(index: usize) -> Map {
    let mut map = Map::new(
        ["Millbrook", "Highgate", "Saltmarsh"][index],
        MapKind::City(index),
        40,
        40,
        Tile::Grass,
    );
    rect(&mut map, 1, 1, 38, 38, Tile::Wall);
    rect(&mut map, 2, 2, 37, 37, Tile::Floor);
    // Wide public streets wrap around six shops and homes. Each building keeps a
    // single door, on the wall that fronts a street: the row south of the cross
    // street opens north onto it; the upper rows open sideways toward the lanes.
    // One storefront per shop reads as a shop — three made the town a colander.
    for (x, y) in [(5, 5), (24, 5), (5, 14), (24, 14), (5, 25), (24, 25)] {
        rect(&mut map, x, y, x + 9, y + 7, Tile::Wall);
        rect(&mut map, x + 1, y + 1, x + 8, y + 6, Tile::Floor);
        let door = if y > 24 {
            Pos::new(x + 5, y) // north wall, on the cross street
        } else if x < 18 {
            Pos::new(x + 9, y + 4) // east wall, toward the lane
        } else {
            Pos::new(x, y + 4) // west wall, toward the lane
        };
        map.set(door, Tile::Door);
    }
    rect(&mut map, 18, 2, 22, 37, Tile::Road);
    rect(&mut map, 2, 22, 37, 24, Tile::Road);
    rect(&mut map, 18, 34, 22, 37, Tile::Road);
    map.set(Pos::new(20, 10), Tile::Shrine);
    if index == 0 {
        map.set(Pos::new(8, 18), Tile::Down);
    }
    if index == 2 {
        // The customs storehouse is accessible, but its moral choice is not automatic.
        map.set(PARCEL_SITE, Tile::Chest);
        for y in 3..34 {
            map.set(Pos::new(36, y), Tile::River);
        }
        for y in [12, 23, 32] {
            map.set(Pos::new(36, y), Tile::Ford);
        }
    }
    map
}

fn dungeon(index: usize, floor: usize, seed: u64) -> Map {
    let names = ["The Burrow", "Crimson Hollow", "The Underkeep"];
    let mut map = Map::new(
        &format!("{} — Floor {}", names[index], floor + 1),
        MapKind::Dungeon(index, floor),
        30,
        30,
        Tile::Wall,
    );
    let centers = DUNGEON_CENTERS;
    for p in centers {
        let (left, top, right, bottom) = room_bounds(p);
        rect(&mut map, left, top, right, bottom, Tile::Floor);
    }
    let edges: &[(usize, usize)] = match (index + floor) % 3 {
        0 => &[
            (0, 1),
            (1, 2),
            (1, 4),
            (4, 3),
            (3, 6),
            (4, 5),
            (5, 8),
            (8, 7),
        ],
        1 => &[
            (0, 3),
            (3, 4),
            (4, 1),
            (1, 2),
            (2, 5),
            (4, 7),
            (7, 6),
            (7, 8),
        ],
        _ => &[
            (0, 1),
            (1, 2),
            (2, 5),
            (5, 4),
            (4, 3),
            (3, 6),
            (6, 7),
            (7, 8),
        ],
    };
    for &(a, b) in edges {
        path(&mut map, centers[a], centers[b], Tile::Floor);
        if index == 1 {
            // Crimson Hollow is a natural hollow, not a keep: its corridors
            // sprout undergrowth at the halfway point, never doors.
            map.set(
                Pos::new(
                    (centers[a].x + centers[b].x) / 2,
                    (centers[a].y + centers[b].y) / 2,
                ),
                Tile::Forest,
            );
        }
    }
    // Sparse room pillars provide cover without obstructing any room mouth or centerline.
    for (i, p) in centers.iter().enumerate().skip(1).take(7) {
        let offset = if roll(seed, (floor * 17 + i) as u64, 2) == 0 {
            -2
        } else {
            2
        };
        map.set(
            p.offset(offset, -2),
            if index == 1 { Tile::Rock } else { Tile::Wall },
        );
    }
    for p in [Pos::new(3, 26), Pos::new(26, 3), Pos::new(13, 26)] {
        map.set(p, Tile::Chest);
    }
    // Three landmarks anchor every floor: the entry stair, the reliquary, the descent.
    let (up, shrine, down) = (Pos::new(4, 4), Pos::new(5, 24), Pos::new(25, 25));
    map.set(shrine, Tile::Shrine);
    map.set(up, Tile::Up);
    map.set(down, Tile::Down);
    // A door marks a chamber threshold, not every corridor: one at the mouth of
    // each landmark chamber, so a floor shows three real doorways instead of one
    // per edge. Doors auto-open on passage, so they frame a room without ever
    // sealing it; built keeps get them, the natural hollow does not.
    if index != 1 {
        for landmark in [up, shrine, down] {
            let Some(room) = centers.iter().position(|&c| {
                let (left, top, right, bottom) = room_bounds(c);
                landmark.x >= left
                    && landmark.x <= right
                    && landmark.y >= top
                    && landmark.y <= bottom
            }) else {
                continue;
            };
            let Some(&(a, b)) = edges.iter().find(|&&(a, b)| a == room || b == room) else {
                continue;
            };
            let neighbour = if a == room { centers[b] } else { centers[a] };
            if let Some(mouth) = room_mouth(centers[room], neighbour) {
                if map.tile(mouth) == Tile::Floor {
                    map.set(mouth, Tile::Door);
                }
            }
        }
    }
    map
}

impl Game {
    /// E3: resolving the Saltmarsh smuggling line — either way — stirs the landing (§6).
    /// The Tidemother rises the next time you look at the water.
    pub fn spawn_tidemother(&mut self) {
        if self
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::Tidemother)
        {
            return;
        }
        let tide = self.npcs.len();
        self.npcs.push(make_npc(
            tide,
            Archetype::Tidemother,
            14,
            Pos::new(12, 5),
            8,
            1800,
            self.seed,
        ));
        self.npcs[tide].name = "The Tidemother".into();
        self.npcs[tide].intent = Intent::Idle;
        self.log("From the landing comes a long, wet breath. The docks are hers again.");
    }

    /// E5 (§6): the rat catch's rot traced to its source — the barrow wakes when
    /// the cellar job settles. Two brood-holes and a first brood come with him.
    pub fn spawn_gnawthane(&mut self) {
        if self.npcs.iter().any(|n| n.archetype == Archetype::GnawThane) {
            return;
        }
        let pack = 2100;
        for (archetype, pos) in [
            (Archetype::GnawThane, Pos::new(12, 8)),
            (Archetype::BroodHole, Pos::new(4, 3)),
            (Archetype::BroodHole, Pos::new(21, 3)),
            (Archetype::Rat, Pos::new(10, 7)),
            (Archetype::Rat, Pos::new(14, 9)),
            (Archetype::Rat, Pos::new(12, 5)),
        ] {
            let id = self.npcs.len();
            let level = if archetype == Archetype::Rat { 1 } else { 2 };
            self.npcs.push(make_npc(id, archetype, 15, pos, level, pack, self.seed));
        }
        self.log("Mara's cellar rot has a source: something stirs in the fen barrow southwest of Millbrook.");
    }

    /// E5 (§6): the caravan line's climax — once the goods move, honestly or not,
    /// the Tollmaster's crew reopens the ford's chokepoint and waits for you.
    pub fn spawn_tollmaster(&mut self) {
        if self.npcs.iter().any(|n| n.archetype == Archetype::Tollmaster) {
            return;
        }
        let pack = 2200;
        for (archetype, pos) in [
            (Archetype::Tollmaster, Pos::new(11, 5)),
            (Archetype::Bandit, Pos::new(8, 7)),
            (Archetype::Bandit, Pos::new(14, 7)),
        ] {
            let id = self.npcs.len();
            let level = 4;
            self.npcs.push(make_npc(id, archetype, 16, pos, level, pack, self.seed));
        }
        self.log("Word from the ford: a toll-crew has raised a camp on the old toll road and bleeds every cart that passes.");
    }

    /// E5 (§6): bounty escalation — three settled cull contracts of one kind and
    /// a captain damns your name to the ridge: the Pale Stag stands at the brae.
    pub fn spawn_stag(&mut self, captain_city: usize) {
        if self.npcs.iter().any(|n| n.archetype == Archetype::PaleStag) {
            return;
        }
        let id = self.npcs.len();
        self.npcs.push(make_npc(
            id,
            Archetype::PaleStag,
            18,
            Pos::new(11, 5),
            7,
            2300,
            self.seed,
        ));
        self.log(format!(
            "The {} captain tips you off: culls draw out the alpha. A ghost-white stag holds the Hart's Stand above the ridge den — corner it, or it will run all day.",
            crate::social::city_name(captain_city)
        ));
    }

    /// E7 (§6.5 act V): the third sigil leaves Vael's hand and the cell wakes —
    /// the covenant-breaker stirs beneath the two new depths.
    pub fn spawn_curate(&mut self) {
        if self
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::OathlessCurate)
        {
            return;
        }
        let id = self.npcs.len();
        self.npcs.push(make_npc(
            id,
            Archetype::OathlessCurate,
            20,
            Pos::new(24, 23),
            10,
            2500,
            self.seed,
        ));
        self.log("Far below the keep, something old stops pretending to sleep. The oracles are screaming in three cities at once: the sigils key two doors, and the second just opened.");
    }

    /// E5 (§6): the fen wisp keeps night hours. The first dusk over the hollow
    /// calls her up; after that she rises at dusk whole again and gutters out at
    /// dawn. Killing her ends the haunting for good.
    pub fn mirelight_upkeep(&mut self) {
        const POOL: Pos = Pos::new(2, 2);
        const CLEARING: Pos = Pos::new(12, 10);
        let Some(wisp) = self
            .npcs
            .iter()
            .position(|n| n.archetype == Archetype::Mirelight)
        else {
            if self.night() {
                let id = self.npcs.len();
                self.npcs.push(make_npc(
                    id,
                    Archetype::Mirelight,
                    17,
                    CLEARING,
                    5,
                    2400,
                    self.seed,
                ));
                if self.player.map == 17 {
                    self.log("Three lights wake between the trees. The forest is not empty tonight.");
                }
            }
            return;
        };
        if !self.npcs[wisp].alive() {
            return;
        }
        if self.night() {
            if self.npcs[wisp].pos == POOL {
                self.npcs[wisp].hp = self.npcs[wisp].max_hp;
                self.npcs[wisp].pos = CLEARING;
                self.npcs[wisp].home = CLEARING;
                self.npcs[wisp].intent = Intent::Idle;
                self.npcs[wisp].telegraph = None;
                if self.player.map == 17 {
                    self.log("Three lights wake between the trees. The forest is not empty tonight.");
                }
            }
        } else if self.npcs[wisp].pos != POOL {
            self.npcs[wisp].pos = POOL;
            self.npcs[wisp].home = POOL;
            self.npcs[wisp].intent = Intent::Idle;
            self.npcs[wisp].telegraph = None;
            for glow in &mut self.npcs {
                if glow.archetype == Archetype::FalseGlow && glow.alive() {
                    glow.hp = 0;
                }
            }
            if self.player.map == 17 {
                self.log("Dawn: the lights gutter out and sink beneath the mire. It will wake again at dusk.");
            }
        }
    }
}

/// The Cragmother's volcanic caldera lair (§6, E3): sculpted organic grotto with
/// an ascending canyon defile, curved basalt walls, and twin northern nesting alcoves.
fn ridge_den() -> Map {
    let mut map = Map::new(
        "Crag Ridge — The Den",
        MapKind::Dungeon(3, 0),
        22,
        16,
        Tile::Rock,
    );
    // 1. Narrow southern defile ascending into the lair
    rect(&mut map, 10, 12, 12, 14, Tile::Floor);

    // 2. Main organic volcanic caldera bowl: elliptical cavern carved out of bedrock
    for y in 3..=12 {
        for x in 3..=18 {
            let dx = (x as f32 - 11.0) / 7.2;
            let dy = (y as f32 - 7.5) / 4.2;
            if dx * dx + dy * dy <= 1.0 {
                map.set(Pos::new(x, y), Tile::Floor);
            }
        }
    }

    // 3. Twin cub-nests scooped deep into the northern rockface
    rect(&mut map, 6, 2, 8, 4, Tile::Floor);
    rect(&mut map, 13, 2, 15, 4, Tile::Floor);
    map.set(Pos::new(7, 2), Tile::Ruins);
    map.set(Pos::new(14, 2), Tile::Ruins);

    // 4. Natural basalt boulders and volcanic debris for tactical combat cover
    for pos in [
        Pos::new(5, 7),
        Pos::new(16, 7),
        Pos::new(8, 5),
        Pos::new(14, 5),
        Pos::new(8, 9),
        Pos::new(14, 9),
    ] {
        map.set(pos, Tile::Rock);
    }

    map.set(Pos::new(11, 14), Tile::Up);
    map
}

/// The Gnaw-Thane's subterranean warren under the fens (§6, E5): organic curved
/// mud-cavern chambers; two brood-hole alcoves feed the Plague Tide; the king fights
/// in the open central feeding pit.
fn fen_barrow() -> Map {
    let mut map = Map::new(
        "The Fen Barrow",
        MapKind::Dungeon(4, 0),
        26,
        18,
        Tile::Wall,
    );
    // 1. Central sunken feeding chamber
    rect(&mut map, 7, 4, 18, 13, Tile::Floor);
    // 2. Western brood-alcove
    rect(&mut map, 3, 2, 7, 7, Tile::Floor);
    // 3. Eastern brood-alcove
    rect(&mut map, 18, 2, 22, 7, Tile::Floor);
    // 4. Southern entrance defile
    rect(&mut map, 10, 13, 14, 16, Tile::Floor);

    // Collapsed mud shelves and bone pillars pinching the chambers
    for pos in [
        Pos::new(8, 5),
        Pos::new(17, 5),
        Pos::new(8, 11),
        Pos::new(17, 11),
        Pos::new(12, 7),
    ] {
        map.set(pos, Tile::Rock);
    }
    // The brood-holes: ruined warren-mouths the tide pours from.
    map.set(Pos::new(4, 3), Tile::Ruins);
    map.set(Pos::new(21, 3), Tile::Ruins);
    map.set(Pos::new(12, 15), Tile::Up);
    map
}

/// The Tollmaster's fortified bridgehead & river gorge (§6, E5): a natural meandering
/// river cutting across the north bank, an ancient stone road spanning the ford, and
/// timber palisades enclosing the camp redoubt.
fn toll_camp() -> Map {
    let mut map = Map::new(
        "The Toll-Ford Camp",
        MapKind::Dungeon(5, 0),
        24,
        18,
        Tile::Wall,
    );
    // 1. Natural meandering river along the north bank
    for y in 1..=4 {
        for x in 1..=22 {
            let river_boundary = 3 + if x >= 8 && x <= 14 { 1 } else { 0 };
            if y <= river_boundary {
                map.set(Pos::new(x, y), Tile::River);
            }
        }
    }
    // 2. Open camp grounds
    rect(&mut map, 2, 5, 21, 14, Tile::Floor);
    // 3. Ancient toll road running east-west through the camp
    rect(&mut map, 1, 9, 22, 9, Tile::Road);
    // 4. River fords / shallows at the crossing
    for y in [3, 4] {
        map.set(Pos::new(11, y), Tile::Ford);
    }
    // 5. Timber stockade bastions and toll-keeper's posts
    for pos in [
        Pos::new(4, 6),
        Pos::new(19, 6),
        Pos::new(4, 12),
        Pos::new(19, 12),
        Pos::new(8, 6),
        Pos::new(15, 6),
    ] {
        map.set(pos, Tile::Rock);
    }
    rect(&mut map, 1, 15, 22, 15, Tile::Wall);
    map.set(Pos::new(11, 15), Tile::Floor);
    map.set(Pos::new(11, 16), Tile::Up);
    map
}

/// Mirelight's deep-forest hollow (§6, E5): a clearing in the dark wood; the
/// wisp sleeps the daylight away in a sealed mire pool you cannot reach.
fn mire_glade() -> Map {
    let mut map = Map::new(
        "The Mire — Fenlight Hollow",
        MapKind::Dungeon(6, 0),
        26,
        20,
        Tile::DeepForest,
    );
    rect(&mut map, 7, 5, 18, 15, Tile::Floor);
    // Bog pools catch the false lights' reflections.
    for pos in [Pos::new(9, 7), Pos::new(16, 12), Pos::new(12, 10)] {
        map.set(pos, Tile::River);
    }
    // The sealed pool: the wisp's daylight grave, walled off from the clearing.
    rect(&mut map, 1, 1, 3, 3, Tile::Wall);
    map.set(Pos::new(2, 2), Tile::River);
    map.set(Pos::new(12, 18), Tile::Up);
    map
}

/// The Pale Stag's stand (§6, E5): a tight brae of rock and thicket — corner it
/// between the stones or burst it down before the Break carries it away.
fn harts_stand() -> Map {
    let mut map = Map::new(
        "The Hart's Stand",
        MapKind::Dungeon(7, 0),
        22,
        16,
        Tile::Rock,
    );
    rect(&mut map, 2, 2, 19, 13, Tile::Grass);
    for pos in [(10, 4), (15, 7), (6, 9), (12, 11)] {
        rect(&mut map, pos.0, pos.1, pos.0 + 1, pos.1 + 1, Tile::Rock);
    }
    map.set(Pos::new(4, 4), Tile::Forest);
    map.set(Pos::new(17, 11), Tile::Forest);
    map.set(Pos::new(11, 14), Tile::Up);
    map
}

/// E7 (§6.5): the class-quest trial yards — one pocket arena per order.
/// 16x12 wards keyed to each trial's lesson.
fn trial_yard(name: &str, sub: usize) -> Map {
    let mut map = Map::new(name, MapKind::Dungeon(8, sub), 16, 12, Tile::Wall);
    rect(&mut map, 2, 2, 13, 9, Tile::Floor);
    map.set(Pos::new(7, 10), Tile::Up);
    map
}

/// The Tidemother's harbor wharf & ocean landing (§6, E3): a jutting wooden pier
/// surrounded by seawater on three flanks, with mooring posts and breakwater reefs.
fn docks_arena() -> Map {
    let mut map = Map::new(
        "Saltmarsh — The Tidemother's Landing",
        MapKind::Dungeon(3, 1),
        24,
        20,
        Tile::Wall,
    );
    // 1. Open seawater surrounds the dock on North, West, and East flanks
    for y in 1..=16 {
        for x in 1..=22 {
            if y <= 5 || x <= 3 || x >= 20 {
                map.set(Pos::new(x, y), Tile::River);
            }
        }
    }
    // 2. Jutting timber wharf / boardwalk projecting out into the sea
    rect(&mut map, 4, 5, 19, 16, Tile::Floor);
    // 3. Mooring posts and breakwater reefs for cover against the Undertow
    for pos in [
        Pos::new(5, 7),
        Pos::new(18, 7),
        Pos::new(5, 13),
        Pos::new(18, 13),
        Pos::new(10, 5),
        Pos::new(14, 5),
    ] {
        map.set(pos, Tile::Rock);
    }
    // 4. Harbor gatehouse wall leading back to Saltmarsh
    rect(&mut map, 1, 17, 22, 18, Tile::Wall);
    map.set(Pos::new(11, 17), Tile::Floor);
    map.set(Pos::new(11, 18), Tile::Up);
    map
}

fn cellar() -> Map {
    let mut map = Map::new(
        "Millbrook — Provisioner's Cellar",
        MapKind::Cellar,
        30,
        30,
        Tile::Wall,
    );
    for (x, y, r, b) in [
        (2, 2, 10, 10),
        (14, 3, 26, 11),
        (4, 16, 12, 26),
        (17, 17, 27, 27),
    ] {
        rect(&mut map, x, y, r, b, Tile::Floor);
    }
    route(
        &mut map,
        &[
            Pos::new(6, 6),
            Pos::new(20, 6),
            Pos::new(20, 21),
            Pos::new(8, 21),
        ],
        Tile::Floor,
    );
    // The route is the cellar's only corridor, and it cuts the walls between the
    // four stores. One door mid-cut marks each of those three real thresholds.
    map.set(Pos::new(12, 6), Tile::Door);
    map.set(Pos::new(20, 14), Tile::Door);
    map.set(Pos::new(15, 21), Tile::Door);
    map.set(Pos::new(4, 4), Tile::Up);
    map.set(Pos::new(25, 25), Tile::Chest);
    map
}

/// The Final Trial — The Hall of Verdicts (§6, E3): Grand Cruciform Cathedral &
/// Imperial High Court. A processional colonnaded nave, wide Latin cross transept
/// galleries, and an elevated northern tribunal apse where the Adjudicator presides.
fn arena() -> Map {
    let mut map = Map::new(
        "Final Trial — The Hall of Verdicts",
        MapKind::Arena,
        30,
        30,
        Tile::Wall,
    );
    // 1. Southern Narthex & Processional Nave (x: 11..=19, y: 13..=26)
    rect(&mut map, 11, 13, 19, 26, Tile::Floor);
    // Fluted colonnade pillars along the processional aisle
    for y in [19, 23] {
        map.set(Pos::new(11, y), Tile::Wall);
        map.set(Pos::new(19, y), Tile::Wall);
    }

    // 2. East & West Transepts (Latin Cross arms / tribunal observer galleries)
    rect(&mut map, 3, 13, 11, 18, Tile::Floor);
    rect(&mut map, 19, 13, 27, 18, Tile::Floor);
    // Transept corner buttresses & statuary pedestals
    for &(x, y) in &[(4, 13), (4, 18), (26, 13), (26, 18), (7, 15), (23, 15)] {
        map.set(Pos::new(x, y), Tile::Wall);
    }

    // 3. Central Crossing & Chancel (x: 9..=21, y: 8..=13)
    rect(&mut map, 9, 8, 21, 13, Tile::Floor);

    // 4. Northern Apse Dais (semi-octagonal elevated tribunal dais)
    for (y, x_min, x_max) in [
        (7, 10, 20),
        (6, 11, 19),
        (5, 12, 18),
        (4, 13, 17),
        (3, 14, 16),
    ] {
        rect(&mut map, x_min, y, x_max, y, Tile::Floor);
    }
    // High judicial colonnade pillars framing the apse
    for &(x, y) in &[(10, 9), (20, 9), (12, 6), (18, 6)] {
        map.set(Pos::new(x, y), Tile::Wall);
    }

    // High Altar Shrine of Verdicts at the head of the elevated apse dais
    map.set(Pos::new(15, 4), Tile::Shrine);
    // Entry threshold marker shrine
    map.set(Pos::new(15, 25), Tile::Shrine);
    map
}

// Each directed portal has a real return portal and an unoccupied arrival tile.
fn link(
    maps: &mut [Map],
    (a, at, a_arrival): (usize, Pos, Pos),
    (b, bt, b_arrival): (usize, Pos, Pos),
    requirement: Option<usize>,
) {
    let forward = format!("{} ({},{})", maps[b].name, bt.x, bt.y);
    let backward = format!("{} ({},{})", maps[a].name, at.x, at.y);
    maps[a].set(at, Tile::Down);
    maps[b].set(bt, Tile::Up);
    maps[a].portals.push(Portal {
        pos: at,
        destination: b,
        arrival: b_arrival,
        label: forward,
        requirement,
    });
    maps[b].portals.push(Portal {
        pos: bt,
        destination: a,
        arrival: a_arrival,
        label: backward,
        requirement: None,
    });
}

/// The catch-all companion sheet (D22/D35): sellswords are re-derived to the
/// player's level at hire and at every player level-up, so hiring early is the
/// identity and waiting is never the meta. Returns (hp, attack, defense, speed).
pub fn sellsword_stats(level: u32) -> (i32, i32, i32, i32) {
    let l = level as i32;
    (12 + l * 2, 2 + l, 0, 10)
}

pub fn make_npc(
    id: usize,
    archetype: Archetype,
    map: usize,
    pos: Pos,
    level: u32,
    pack: usize,
    seed: u64,
) -> Npc {
    let l = level as i32;
    let (hp, attack, defense, speed) = match archetype {
        Archetype::Rat => (5, 2, 0, 8),
        Archetype::Wolf => (7 + l * 3, 2 + l, l / 3, 12),
        Archetype::Bear => (16 + l * 4, 4 + l, l / 2, 7),
        Archetype::Skeleton => (9 + l * 3, 2 + l, l / 3, 8),
        Archetype::Bandit => (7 + l * 3, 2 + l, l / 3, 10),
        Archetype::Chief => (64, 9, 2, 10),
        Archetype::Matriarch => (86, 12, 3, 13),
        Archetype::Lich => (108, 15, 4, 9),
        Archetype::Adjudicator => (168, 18, 5, 12), // D26: rises with the cap-12 campaign
        Archetype::Tidemother => (96, 14, 3, 12),
        Archetype::Cragmother => (112, 16, 4, 8),
        // §6 ladder: Gnaw-Thane ≈40hp (L2) · Tollmaster ≈70hp (L4) · Mirelight
        // ≈78hp (L5) · Pale Stag ≈124hp (L7).
        Archetype::GnawThane => (40, 6, 1, 9),
        Archetype::Tollmaster => (70, 11, 2, 11),
        Archetype::Mirelight => (78, 12, 2, 12),
        // The fastest thing on four legs; the Break needs the legs to matter.
        Archetype::PaleStag => (124, 17, 4, 14),
        // E7 (§6.5): the covenant-breaker at ≈150hp (L10), under the new depths.
        Archetype::OathlessCurate => (150, 16, 4, 10),
        Archetype::BroodHole => (14, 0, 0, 0),
        Archetype::FalseGlow => (10, 3, 0, 12),
        Archetype::Companion => sellsword_stats(level),
        Archetype::Guard => (28 + l * 4, 5 + l, 2 + l / 2, 10),
        _ => (12 + l * 2, 2 + l, 0, 10),
    };
    let given = [
        "Alda", "Bram", "Cora", "Dain", "Elin", "Finn", "Gwen", "Hale", "Iris", "Jory", "Kira",
        "Leif", "Mira", "Noll", "Orla", "Perrin", "Quinn", "Rhea", "Soren", "Tova", "Una", "Vann",
        "Wren", "Yara",
    ];
    let surnames = [
        "Reed", "Thorne", "Vale", "Ash", "Pike", "Moss", "Wick", "Stone", "Brook", "Holt", "Fen",
        "Grey",
    ];
    let person = format!(
        "{} {}",
        given[roll(seed, id as u64, given.len() as u64) as usize],
        surnames[roll(seed ^ 361, id as u64, surnames.len() as u64) as usize]
    );
    let name = match archetype {
        Archetype::Chief => "Red Jack, the Bandit Chief".into(),
        Archetype::Matriarch => "Ashfang, the Dire Matriarch".into(),
        Archetype::Lich => "Vael, the Last Castellan".into(),
        Archetype::Adjudicator => "The Adjudicator".into(),
        Archetype::Wolf => format!("{} Wolf", ["Briar", "Greyback", "Dusk", "Redfang"][id % 4]),
        Archetype::Bear => format!("{} Bear", ["Old Pine", "Scarred", "Honeyclaw"][id % 3]),
        Archetype::Rat => format!("Cellar Rat {}", id % 4 + 1),
        Archetype::Skeleton => format!(
            "{} of the Keep",
            [
                "Oathless Spearman",
                "Fallen Sentinel",
                "Bone Warden",
                "Ashen Sentry"
            ][id % 4]
        ),
        Archetype::Bandit => format!(
            "{}, {}",
            person,
            ["Road Reaver", "Red Sash", "Warren Scout", "Outlaw"][id % 4]
        ),
        Archetype::Guard => format!("Guard {}", person),
        Archetype::Traveller => format!("{}, Wayfarer", person),
        Archetype::Oracle => "Ilyra, Keeper of the Shrine".into(),
        Archetype::Smuggler => "Nessa the Smuggler".into(),
        Archetype::GnawThane => "Gnaw-Thane, the Rat-King Below".into(),
        Archetype::Tollmaster => "Tollmaster Grudge".into(),
        Archetype::Mirelight => "Mirelight, the Fen Wisp".into(),
        Archetype::PaleStag => "The Pale Stag".into(),
        Archetype::BroodHole => "Brood-hole".into(),
        Archetype::FalseGlow => "False light".into(),
        Archetype::OathlessCurate => "The Oathless Curate".into(),
        _ => person,
    };
    let axis = |n| roll(seed ^ mix(id as u64), n, 1001) as f32 / 1000.0;
    Npc {
        id,
        name,
        archetype,
        map,
        pos,
        home: pos,
        hp,
        max_hp: hp,
        attack,
        defense,
        speed,
        level,
        personality: Personality {
            brave: axis(1),
            greedy: axis(2),
            gullibility: axis(3),
            spite: axis(4),
            chatty: axis(5),
        },
        memory: Memory::default(),
        pack,
        intent: Intent::Idle,
        decision_pending: false,
        decision_version: 0,
        last_decision: 0,
        last_hp_band: 4,
        perceived: false,
        tactic: "pressure".into(),
        phase: 0,
        cooldown: 0,
        mercy_used: false,
        telegraph: None,
        answers: BTreeMap::new(),
    }
}

fn add(
    npcs: &mut Vec<Npc>,
    archetype: Archetype,
    map: usize,
    pos: Pos,
    level: u32,
    pack: usize,
    seed: u64,
) -> usize {
    let id = npcs.len();
    npcs.push(make_npc(id, archetype, map, pos, level, pack, seed));
    id
}

fn reserved(maps: &[Map], map: usize, pos: Pos, distance: i32) -> bool {
    maps[map]
        .portals
        .iter()
        .any(|p| p.pos.distance(pos) <= distance)
        || maps
            .iter()
            .flat_map(|m| &m.portals)
            .any(|p| p.destination == map && p.arrival.distance(pos) <= distance)
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut maps = vec![
            overworld(seed),
            city(0),
            city(1),
            city(2),
            cellar(),
            dungeon(0, 0, seed),
            dungeon(0, 1, seed),
            dungeon(1, 0, seed),
            dungeon(1, 1, seed),
            dungeon(2, 0, seed),
            dungeon(2, 1, seed),
            dungeon(2, 2, seed),
            arena(),
            ridge_den(),
            docks_arena(),
            // E5 arenas (§6): 15 fen barrow, 16 toll camp, 17 mire hollow, 18 stand.
            fen_barrow(),
            toll_camp(),
            mire_glade(),
            harts_stand(),
            // E7 (§6.5 act V): the two new Underkeep depths — the cell floors the
            // three sigils also key. 19 = unresting nave, 20 = the Curate's cell.
            dungeon(2, 3, seed),
            dungeon(2, 4, seed),
            // E7 (§6.5): the six orders' trial yards (21..26).
            trial_yard("Highgate — The Drill Yard", 0),
            trial_yard("Millbrook — The Hollow Shrine", 1),
            trial_yard("The Truce Marsh", 2),
            trial_yard("Saltmarsh — The Tide Locker", 3),
            trial_yard("The Barrow Post", 4),
            trial_yard("The Oathroad Rest", 5),
        ];
        for (i, site) in CITY_SITES.iter().copied().enumerate() {
            link(
                &mut maps,
                (0, site, site.offset(0, 1)),
                (i + 1, Pos::new(20, 37), Pos::new(20, 35)),
                None,
            );
        }
        link(
            &mut maps,
            (1, Pos::new(8, 18), Pos::new(9, 18)),
            (4, Pos::new(4, 4), Pos::new(5, 5)),
            None,
        );
        for (map, site, key) in [
            (5, Pos::new(52, 78), 0),
            (7, Pos::new(158, 82), 1),
            (9, Pos::new(145, 12), 2),
        ] {
            link(
                &mut maps,
                (0, site, site.offset(0, 1)),
                (map, Pos::new(4, 4), Pos::new(5, 5)),
                Some(key),
            );
        }
        // The ridge den mouths off the northern crest, east of the river (E3, open).
        link(
            &mut maps,
            (0, Pos::new(105, 10), Pos::new(105, 11)),
            (13, Pos::new(11, 14), Pos::new(11, 13)),
            None,
        );
        // The Tidemother's landing hangs off Saltmarsh's dockboard (E3).
        link(
            &mut maps,
            (3, Pos::new(32, 32), Pos::new(32, 31)),
            (14, Pos::new(11, 18), Pos::new(11, 16)),
            None,
        );
        // E5 mouths (§6): the barrow squats in the fen southwest of Millbrook; the
        // toll camp holds the old road by the north ford; the hollow hides in the
        // deep eastern wood; the stand overlooks the ridge den.
        link(
            &mut maps,
            (0, Pos::new(44, 122), Pos::new(44, 123)),
            (15, Pos::new(12, 15), Pos::new(12, 14)),
            None,
        );
        link(
            &mut maps,
            (0, Pos::new(96, 60), Pos::new(96, 61)),
            (16, Pos::new(11, 16), Pos::new(11, 14)),
            None,
        );
        link(
            &mut maps,
            (0, Pos::new(126, 116), Pos::new(126, 117)),
            (17, Pos::new(12, 18), Pos::new(12, 17)),
            None,
        );
        link(
            &mut maps,
            (0, Pos::new(112, 11), Pos::new(112, 12)),
            (18, Pos::new(11, 14), Pos::new(11, 13)),
            None,
        );
        for (a, b) in [(5, 6), (7, 8), (9, 10), (10, 11)] {
            link(
                &mut maps,
                (a, Pos::new(25, 25), Pos::new(24, 26)),
                (b, Pos::new(4, 4), Pos::new(5, 5)),
                None,
            );
        }
        // E7 depths: the sigils key this door as surely as the Trial's (§6.5).
        link(
            &mut maps,
            (11, Pos::new(25, 25), Pos::new(24, 26)),
            (19, Pos::new(4, 4), Pos::new(5, 5)),
            Some(3),
        );
        link(
            &mut maps,
            (19, Pos::new(25, 25), Pos::new(24, 26)),
            (20, Pos::new(4, 4), Pos::new(5, 5)),
            Some(3),
        );
        // E7 (§6.5): the six orders' trial yards hang near their offer sites.
        link(
            &mut maps,
            (2, Pos::new(34, 8), Pos::new(34, 9)),
            (21, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        link(
            &mut maps,
            (1, Pos::new(24, 8), Pos::new(24, 9)),
            (22, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        maps[0].set(Pos::new(152, 88), Tile::Grass);
        maps[0].set(Pos::new(152, 89), Tile::Grass);
        link(
            &mut maps,
            (0, Pos::new(152, 88), Pos::new(152, 89)),
            (23, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        link(
            &mut maps,
            (3, Pos::new(26, 30), Pos::new(26, 31)),
            (24, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        maps[0].set(Pos::new(140, 16), Tile::Grass);
        maps[0].set(Pos::new(140, 17), Tile::Grass);
        link(
            &mut maps,
            (0, Pos::new(140, 16), Pos::new(140, 17)),
            (25, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        maps[0].set(Pos::new(99, 72), Tile::Grass);
        maps[0].set(Pos::new(99, 73), Tile::Grass);
        link(
            &mut maps,
            (0, Pos::new(99, 72), Pos::new(99, 73)),
            (26, Pos::new(7, 10), Pos::new(7, 8)),
            None,
        );
        link(
            &mut maps,
            (0, Pos::new(72, 44), Pos::new(72, 45)),
            (12, Pos::new(15, 26), Pos::new(15, 24)),
            Some(3),
        );
        for map in [6, 8, 11] {
            maps[map].set(Pos::new(25, 25), Tile::Chest);
        }
        let mut npcs = Vec::new();
        let vendors = [
            [
                "Mara, General Provisioner",
                "Orrin, Blacksmith",
                "Tess, Innkeeper",
            ],
            ["Rook, Weaponsmith", "Sera, Armourer", "Edda, Innkeeper"],
            [
                "Vey, Black Market",
                "Silas, Fence",
                "Brin, Dockside Innkeeper",
            ],
        ];
        for city in 0..3 {
            let map = city + 1;
            for (i, pos) in [Pos::new(12, 18), Pos::new(28, 12), Pos::new(12, 28)]
                .into_iter()
                .enumerate()
            {
                maps[map].set(pos, Tile::Floor);
                let id = add(
                    &mut npcs,
                    Archetype::Vendor,
                    map,
                    pos,
                    city as u32 + 1,
                    100 + city,
                    seed,
                );
                npcs[id].name = vendors[city][i].into();
            }
            let positions = [
                Pos::new(18, 34),
                Pos::new(22, 34),
                Pos::new(18, 22),
                Pos::new(22, 22),
                Pos::new(8, 8),
                Pos::new(12, 8),
                Pos::new(27, 8),
                Pos::new(30, 9),
                Pos::new(7, 18),
                Pos::new(29, 18),
                Pos::new(6, 29),
                Pos::new(28, 28),
                Pos::new(17, 13),
                Pos::new(23, 13),
                Pos::new(17, 30),
                Pos::new(23, 30),
                Pos::new(3, 16),
                Pos::new(3, 28),
                Pos::new(34, 16),
                Pos::new(34, 28),
            ];
            for (i, pos) in positions.into_iter().enumerate() {
                let archetype = match i {
                    0..=3 => Archetype::Guard,
                    4..=9 => Archetype::Commoner,
                    10 => Archetype::Thief,
                    11 if city == 2 => Archetype::Thief,
                    _ => Archetype::Traveller,
                };
                maps[map].set(pos, Tile::Floor);
                let id = add(
                    &mut npcs,
                    archetype,
                    map,
                    pos,
                    city as u32 + 2,
                    110 + city,
                    seed,
                );
                if i == 0 {
                    npcs[id].name = format!("{} Guard Captain", maps[map].name);
                }
            }
            add(
                &mut npcs,
                Archetype::Oracle,
                map,
                Pos::new(21, 10),
                1,
                120 + city,
                seed,
            );
            // E5 (D32/D36): one brewing alchemist per big city; the still keeps
            // its own hours beside the shrine road.
            let alchemist = add(
                &mut npcs,
                Archetype::Alchemist,
                map,
                Pos::new(24, 10),
                1,
                140 + city,
                seed,
            );
            maps[map].set(Pos::new(24, 10), Tile::Floor);
            npcs[alchemist].name = [
                "Odette, Alchemist",
                "Linden, Alchemist",
                "Sorrel, Alchemist",
            ][city]
                .into();
            // One hirable blade per city; each can become the player's sole companion.
            let blade = add(
                &mut npcs,
                Archetype::Companion,
                map,
                Pos::new(10, 26),
                3,
                800 + city,
                seed,
            );
            npcs[blade].name = format!("{}, Sellsword", ["Garrick", "Hilde", "Pell"][city]);
            add(
                &mut npcs,
                Archetype::Guard,
                0,
                CITY_SITES[city].offset(-1, 0),
                city as u32 + 3,
                130 + city,
                seed,
            );
        }
        add(
            &mut npcs,
            Archetype::Smuggler,
            3,
            Pos::new(30, 30),
            4,
            150,
            seed,
        );
        for pos in [
            Pos::new(17, 7),
            Pos::new(24, 10),
            Pos::new(21, 22),
            Pos::new(7, 23),
        ] {
            add(&mut npcs, Archetype::Rat, 4, pos, 1, 200, seed);
        }
        for pos in [Pos::new(106, 64), Pos::new(108, 66), Pos::new(107, 69)] {
            maps[0].set(pos, Tile::Grass);
            add(&mut npcs, Archetype::Wolf, 0, pos, 2, CARAVAN_PACK, seed);
        }
        // Widely separated wilderness camps leave the road network a practical safe route.
        // Deterministic candidate scanning avoids retries and always fills the population.
        let mut wilderness = 0;
        for row in 0..11 {
            for col in 0..15 {
                if wilderness >= 78 {
                    break;
                }
                let pos = Pos::new(
                    9 + col * 12 + roll(seed, (row * 15 + col + 8000) as u64, 5) as i32,
                    34 + row * 11,
                );
                if !maps[0].tile(pos).walkable()
                    || reserved(&maps, 0, pos, 9)
                    || pos.distance(CARAVAN_SITE) < 12
                {
                    continue;
                }
                let near_road = (-4..=4).any(|dy| {
                    (-4..=4).any(|dx| {
                        matches!(maps[0].tile(pos.offset(dx, dy)), Tile::Road | Tile::Ford)
                    })
                });
                if near_road {
                    continue;
                }
                let archetype = match wilderness % 7 {
                    0 => Archetype::Bear,
                    1..=3 => Archetype::Wolf,
                    _ => Archetype::Bandit,
                };
                let level = if pos.y < 55 {
                    4
                } else if pos.x > 102 {
                    3
                } else {
                    2
                };
                add(
                    &mut npcs,
                    archetype,
                    0,
                    pos,
                    level,
                    300 + wilderness / 2,
                    seed,
                );
                wilderness += 1;
            }
        }
        let road_points: Vec<Pos> = (4..156)
            .flat_map(|y| (4..196).map(move |x| Pos::new(x, y)))
            .filter(|p| maps[0].tile(*p) == Tile::Road)
            .collect();
        let mut travellers = 0;
        for pos in road_points {
            if travellers >= 36 {
                break;
            }
            if reserved(&maps, 0, pos, 4)
                || pos.distance(CARAVAN_SITE) < 12
                || npcs.iter().any(|n| n.map == 0 && n.pos.distance(pos) < 7)
            {
                continue;
            }
            let archetype = if travellers % 4 == 0 {
                Archetype::Guard
            } else {
                Archetype::Traveller
            };
            add(&mut npcs, archetype, 0, pos, 3, 600 + travellers, seed);
            travellers += 1;
        }
        add(
            &mut npcs,
            Archetype::Oracle,
            0,
            Pos::new(70, 104),
            4,
            700,
            seed,
        );
        for (map, dungeon) in maps.iter_mut().enumerate().take(12).skip(5) {
            let (theme, floor) = match dungeon.kind {
                MapKind::Dungeon(a, b) => (a, b),
                _ => unreachable!(),
            };
            let level = (map - 3) as u32;
            let positions = [
                Pos::new(15, 4),
                Pos::new(17, 6),
                Pos::new(24, 4),
                Pos::new(26, 6),
                Pos::new(5, 14),
                Pos::new(7, 16),
                Pos::new(14, 23),
                Pos::new(16, 25),
            ];
            for (i, pos) in positions.into_iter().enumerate() {
                let archetype = match theme {
                    0 => Archetype::Bandit,
                    1 if i == 4 => Archetype::Bear,
                    1 => Archetype::Wolf,
                    _ => Archetype::Skeleton,
                };
                dungeon.set(pos, Tile::Floor);
                add(
                    &mut npcs,
                    archetype,
                    map,
                    pos,
                    level,
                    1000 + map * 10 + i / 2,
                    seed,
                );
            }
            if (theme < 2 && floor == 1) || (theme == 2 && floor == 2) {
                let archetype = [Archetype::Chief, Archetype::Matriarch, Archetype::Lich][theme];
                add(
                    &mut npcs,
                    archetype,
                    map,
                    Pos::new(24, 23),
                    (theme as u32 + 2) * 2,
                    1000 + map * 10 + 8,
                    seed,
                );
            }
        }
        add(
            &mut npcs,
            Archetype::Adjudicator,
            12,
            Pos::new(15, 8),
            10,
            2000,
            seed,
        );
        // E3: the ridge den is open — she is home, and she is not the welcoming kind.
        let crag = add(
            &mut npcs,
            Archetype::Cragmother,
            13,
            Pos::new(11, 4),
            6,
            1700,
            seed,
        );
        npcs[crag].name = "Cragmother".into();
        // E5 (§6): no wisp at worldgen — she first rises at dusk over the hollow
        // (mirelight_upkeep), keeping the worldgen boss ladder intact.
        // E7 (§6.5): the depths under Vael's floor are restless already.
        for (map, level) in [(19usize, 10u32), (20, 11)] {
            for (i, pos) in [
                Pos::new(15, 4),
                Pos::new(24, 4),
                Pos::new(7, 16),
                Pos::new(16, 25),
                Pos::new(24, 23),
            ]
            .into_iter()
            .enumerate()
            {
                if i == 4 && map == 20 {
                    // The cell's centre is his altar — leave it for the Curate.
                    continue;
                }
                maps[map].set(pos, Tile::Floor);
                add(&mut npcs, Archetype::Skeleton, map, pos, level, 1500 + map * 10 + i, seed);
            }
        }
        // E7 (§8.2): one-shot seeded scrounge — herbs in the forests, ore on the
        // ridge line. Evenly spaced over each kind's candidates; zero grind loop.
        let mut scrounge_sites: Vec<(Pos, u8)> = Vec::new();
        let pick = |tiles: Vec<Pos>, want: usize, kind: u8, acc: &mut Vec<(Pos, u8)>| {
            if want > 0 && !tiles.is_empty() {
                for i in 0..want {
                    let pos = tiles[i * tiles.len() / want];
                    if !npcs.iter().any(|n| n.map == 0 && n.pos == pos) {
                        acc.push((pos, kind));
                    }
                }
            }
        };
        let herbs: Vec<Pos> = (0..maps[0].height)
            .flat_map(|y| (0..maps[0].width).map(move |x| Pos::new(x, y)))
            .filter(|p| matches!(maps[0].tile(*p), Tile::Forest | Tile::DeepForest))
            .collect();
        let ores: Vec<Pos> = (0..maps[0].height)
            .flat_map(|y| (0..maps[0].width).map(move |x| Pos::new(x, y)))
            .filter(|p| matches!(maps[0].tile(*p), Tile::Mountain))
            .collect();
        pick(herbs, 6, 0, &mut scrounge_sites);
        pick(ores, 5, 1, &mut scrounge_sites);
        let mut game = Self {
            seed,
            rng: mix(seed),
            maps,
            npcs,
            player: Player {
                map: 1,
                pos: Pos::new(12, 19),
                hp: 20,
                max_hp: 20,
                stamina: 12,
                max_stamina: 12,
                attack: 3,
                defense: 1,
                speed: 10,
                level: 1,
                xp: 0,
                gold: 40,
                inventory: vec![
                    Item::Potion,
                    Item::Potion,
                    Item::Potion,
                    Item::Ration,
                    Item::Ration,
                    Item::Ration,
                    Item::Torch,
                ],
                weapon: 0,
                armour: 0,
                relic: None,
                reputation: [50; 3],
                sigils: [false; 3],
                keys: [false; 3],
                last_city: 0,
                defending: false,
                torch_until: 0,
                curse_until: 0,
                last_hazard: String::new(),
                mana: 0,
                max_mana: 0,
                spells: Vec::new(),
                ward_until: 0,
                history: History::default(),
                class: Class::None,
                build: Build::Male,
                class_state: ClassState::default(),
                talents: Vec::new(),
                talent_points: 0,
                talent_once: 0,
                respec_used: false,
                fester: 0,
                fester_guard_until: 0,
                word_weapon: None,
                word_armour: None,
            },
            tick: 0,
            turn: 0,
            hour_ticks: 0,
            combat: None,
            modal: Modal::Title,
            selected: 0,
            inspector: false,
            log: VecDeque::new(),
            decisions: VecDeque::new(),
            outbox: Vec::new(),
            ai: AiStats::default(),
            quests: crate::social::initial_quests(),
            effects: Vec::new(),
            visited: [true, false, false],
            haggle: None,
            move_ready_ms: 0,
            elapsed_ms: 0,
            quit: false,
            won: false,
            rats_killed: 0,
            caravan_recovered: false,
            smuggling_choice: None,
            social_pending: None,
            gate_permit: None,
            recent_actions: VecDeque::new(),
            companion: None,
            bounties: Vec::new(),
            next_bounty: 0,
            atlas_zoom: false,
            create_step: 0,
            create_class: 0,
            create_build: 0,
            rubble: Vec::new(),
            caltrops: Vec::new(),
            cull_wins: [0; 2],
            trial_oath: false,
            curate_unwritten: [false; 3],
            mercy_oathbreaker: false,
            epilogue: Epilogue::Unanswered,
            codex: Vec::new(),
            masterwork_used: false,
            scrounge_sites,
            scrounged: Vec::new(),
            trials: Default::default(),
        };
        game.log("Welcome to Millbrook. Mara is beside you: press E to trade or ask for work.");
        game.log("Roads connect all landmarks. M opens the atlas; J shows NPC judgments.");
        game.reveal();
        game
    }
    /// Applies the creation screen's class sheet and origin boon, then starts the
    /// journey (docs/D2_EVOLUTION.md §3, §4.2). Creation touches the starting sheet
    /// only — the seed already decided everything else.
    pub fn apply_creation(&mut self, class: Class, build: Build, boon: Boon) {
        let p = &mut self.player;
        p.class = class;
        p.build = build;
        match class {
            Class::None => {}
            Class::Keepwarden => {
                p.max_hp += 4;
                p.hp += 4;
                // D7 floor: the oath never strips the last ember of rune-light.
                p.max_mana = (p.max_mana - 2).max(2);
                p.mana = p.mana.min(p.max_mana);
            }
            Class::Gravebound => {
                p.max_hp += 2;
                p.hp += 2;
                p.speed -= 1;
            }
            Class::Redwake => {
                p.speed += 2;
                p.defense -= 1;
            }
            Class::Waysworn => {
                p.speed += 1;
                p.max_stamina += 2;
                p.stamina += 2;
                p.max_hp -= 2;
                p.hp -= 2;
            }
            Class::SigilSworn => {
                p.max_hp -= 3;
                p.hp -= 3;
                p.max_mana += 4;
                p.mana = p.max_mana;
                p.spells.push(Spell::Spark);
            }
            Class::Fensworn => {}
        }
        match boon {
            Boon::None => {}
            Boon::Gold => p.gold += 20,
            Boon::Stamina => {
                p.max_stamina += 1;
                p.stamina += 1;
            }
        }
        let report = format!(
            "You take the road as the {} ({}): {}",
            p.class.name(),
            p.class.calling(),
            p.class.diff()
        );
        let frame = format!("The Vigil records your frame as {}.", p.build.name());
        self.modal = Modal::None;
        self.log(report);
        self.log(frame);
        self.log("Millbrook needs you. Find the vendor V, press E, and ask about work.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn reachable(map: &Map, start: Pos) -> HashSet<Pos> {
        let mut seen = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(from) = queue.pop_front() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let to = from.offset(dx, dy);
                    if map.can_step(from, to) && seen.insert(to) {
                        queue.push_back(to);
                    }
                }
            }
        }
        seen
    }

    /// A door is a threshold at a chamber mouth, and a handful per map at most —
    /// not the one-per-corridor-midpoint the generator used to salt in. Doors
    /// auto-open on passage, so every room centre must stay reachable.
    #[test]
    fn dungeon_doors_mark_chamber_mouths_without_sealing_a_room() {
        for index in 0..3 {
            for floor in 0..5 {
                let map = dungeon(index, floor, 7);
                let seen = reachable(&map, Pos::new(4, 4));
                for (i, centre) in DUNGEON_CENTERS.iter().enumerate() {
                    assert!(
                        seen.contains(centre),
                        "dungeon {index} floor {floor}: room {i} at {centre:?} sealed off"
                    );
                }
                let doors = map.tiles.iter().filter(|t| **t == Tile::Door).count();
                if index == 1 {
                    assert_eq!(doors, 0, "the natural hollow takes undergrowth, not doors");
                } else {
                    assert_eq!(doors, 3, "one door per landmark chamber");
                }
            }
        }
        for map in &Game::new(7).maps {
            let doors = map.tiles.iter().filter(|t| **t == Tile::Door).count();
            assert!(doors <= 6, "{} is a colander: {doors} doors", map.name);
        }
    }

    #[test]
    fn every_portal_quest_and_boss_is_reachable_without_corner_cutting() {
        for seed in [0, 1, 42, u64::MAX] {
            let game = Game::new(seed);
            assert_eq!(game.maps.len(), 27, "E3/E5 arenas + E7 depths + six trial yards");
            assert!(game.npcs.len() > 200, "population {}", game.npcs.len());
            for (index, map) in game.maps.iter().enumerate() {
                let entrance = if index == game.player.map {
                    game.player.pos
                } else {
                    game.maps
                        .iter()
                        .flat_map(|m| &m.portals)
                        .find(|p| p.destination == index)
                        .unwrap()
                        .arrival
                };
                let seen = reachable(map, entrance);
                for portal in &map.portals {
                    assert!(
                        seen.contains(&portal.pos),
                        "seed {seed}: {} unreachable portal {}",
                        map.name,
                        portal.label
                    );
                    assert!(game.maps[portal.destination]
                        .tile(portal.arrival)
                        .walkable());
                    assert!(
                        !game
                            .npcs
                            .iter()
                            .any(|n| n.map == portal.destination && n.pos == portal.arrival),
                        "occupied arrival"
                    );
                    assert!(
                        game.maps[portal.destination]
                            .portals
                            .iter()
                            .any(|p| p.destination == index),
                        "one-way portal"
                    );
                }
                for (tile_index, tile) in map.tiles.iter().enumerate() {
                    if matches!(tile, Tile::Chest | Tile::Shrine) {
                        let p =
                            Pos::new(tile_index as i32 % map.width, tile_index as i32 / map.width);
                        assert!(
                            seen.contains(&p),
                            "seed {seed}: {} unreachable landmark {p:?}",
                            map.name
                        );
                    }
                }
                for npc in game.npcs.iter().filter(|n| n.map == index) {
                    assert!(
                        seen.contains(&npc.pos),
                        "seed {seed}: unreachable NPC {} in {}",
                        npc.name,
                        map.name
                    );
                }
            }
            assert!(game.npcs.iter().enumerate().all(|(id, n)| id == n.id));
            assert_eq!(
                game.npcs.iter().filter(|n| n.archetype.boss()).count(),
                5,
                "the four campaign judges plus the Cragmother at worldgen; the Tidemother rises later"
            );
            assert_eq!(
                game.npcs.iter().filter(|n| n.pack == CARAVAN_PACK).count(),
                3
            );
            assert_eq!(
                game.npcs
                    .iter()
                    .filter(|n| n.map == 4 && n.archetype == Archetype::Rat)
                    .count(),
                4
            );
            assert!(!game.npcs.iter().any(|n| n.map == game.player.map
                && n.archetype.hostile()
                && n.pos.distance(game.player.pos) < 8));
            let repeat = Game::new(seed);
            assert!(game
                .maps
                .iter()
                .zip(&repeat.maps)
                .all(|(a, b)| a.tiles == b.tiles));
            assert!(game
                .npcs
                .iter()
                .zip(&repeat.npcs)
                .all(|(a, b)| a.pos == b.pos
                    && a.name == b.name
                    && a.personality.brave == b.personality.brave));
        }
    }
}
