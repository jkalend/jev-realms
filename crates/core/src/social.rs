use crate::model::*;

const CAPACITY: usize = 20;
const CARAVAN: Pos = Pos { x: 102, y: 65 };
const PARCEL: Pos = Pos { x: 32, y: 31 };

fn caravan_secured(npc: &Npc) -> bool {
    !npc.alive()
        || npc
            .memory
            .events
            .iter()
            .any(|event| event.kind == "player_spared_me")
        || (npc.intent == Intent::Flee && npc.pos.distance(CARAVAN) > 8)
}

pub fn initial_quests() -> Vec<Quest> {
    [
        ("Rat catch", QuestStage::Available, 5, "Speak to Mara in Millbrook. Clear her cellar, then report for 45 gold, 45 XP and the Burrow seal."),
        ("The missing caravan", QuestStage::Available, 3, "Ask Edda or a Highgate merchant about the caravan. Recover its goods at the north ford (102,65); choose return or fence."),
        ("Saltmarsh smuggling", QuestStage::Available, 2, "Find Nessa at Saltmarsh's southeast docks (30,30). Collect her parcel and choose the watch or smugglers."),
        ("Sigil hunt", QuestStage::Available, 3, "Defeat the Chief in the Burrow, the Matriarch in Crimson Hollow and Vael, the Last Castellan, in the Underkeep. Three Sigils unlock the Final Trial."),
        ("The Final Trial", QuestStage::Locked, 1, "Collect all three dungeon Sigils to unlock the arena at (72,44). Defeat the Adjudicator to finish your campaign."),
    ].into_iter().map(|(title, stage, goal, description)| Quest {
        title: title.into(), stage, progress: 0, goal, description: description.into(),
    }).collect()
}

pub fn stock(city: usize) -> Vec<(Item, u32)> {
    let tier = (city.min(2) + 1) as u8;
    let mut items = vec![
        (Item::Potion, 12),
        (Item::Ration, 5),
        (Item::Torch, 4),
        (Item::Weapon(0), 5),
        (Item::Armour(0), 5),
        // D36: the alchemist's launch set is stocked by every city seller.
        (Item::AntiToxin, 14),
        (Item::ManaTonic, 16),
    ];
    for t in 1..=tier {
        items.push((Item::Weapon(t), [0, 35, 100, 230][t as usize]));
        items.push((Item::Armour(t), [0, 30, 90, 210][t as usize]));
    }
    for dungeon in 0..=city.min(2) {
        items.push((Item::Key(dungeon), [80, 130, 200][dungeon]));
    }
    items
}

pub fn sell_price(item: &Item) -> u32 {
    match item {
        Item::Potion => 6,
        Item::Ration => 2,
        Item::Torch => 2,
        Item::Weapon(t) => [2, 17, 50, 115][(*t).min(3) as usize],
        Item::Armour(t) => [2, 15, 45, 105][(*t).min(3) as usize],
        Item::AntiToxin => 7,
        Item::ManaTonic => 8,
        _ => 0,
    }
}

pub fn buy_price(game: &Game, npc: usize, base: u32) -> u32 {
    let Some(vendor) = game.npcs.get(npc) else {
        return base;
    };
    let city = npc_city(game, npc);
    let reputation = game.player.reputation[city];
    let rep = if reputation >= 75 {
        90
    } else if reputation < 25 {
        115
    } else {
        100
    };
    let cheat = if !vendor.decision_pending
        && vendor.personality.greedy >= 0.6
        && answer(vendor, "cheat_player") >= 0.65
    {
        115
    } else {
        100
    };
    let haggle = game
        .haggle
        .filter(|(id, _)| *id == npc)
        .map_or(100, |(_, percent)| percent);
    (u64::from(base) * rep * cheat * u64::from(haggle))
        .div_ceil(1_000_000)
        .max(1) as u32
}
/// Human-facing conversation labels shared by the window and terminal.
pub fn conversation_role(npc: &Npc) -> &'static str {
    match npc.archetype {
        Archetype::Vendor if npc.name.contains("Innkeeper") => "Innkeeper",
        Archetype::Vendor if is_smith(npc) => "Smith",
        Archetype::Vendor => "Merchant",
        Archetype::Guard if npc.name.contains("Guard Captain") => "Guard captain",
        Archetype::Guard => "Gate guard",
        Archetype::Thief => "Informant",
        Archetype::Traveller => "Traveller",
        Archetype::Smuggler => "Smuggler",
        Archetype::Oracle => "Oracle",
        Archetype::Companion => "Companion",
        Archetype::Alchemist => "Alchemist",
        _ => "Local",
    }
}

pub fn conversation_regard(npc: &Npc) -> &'static str {
    if npc.memory.disposition > 0.2 {
        "Warm"
    } else if npc.memory.disposition < -0.2 {
        "Wary"
    } else {
        "Measured"
    }
}


pub fn talk_options(game: &Game, npc: usize) -> Vec<String> {
    let Some(n) = game.npcs.get(npc) else {
        return vec!["Leave".into()];
    };
    let fixed: &[&str] = match n.archetype {
        Archetype::Thief => &[
            "Demand stolen gold back",
            "Pay for information (10 gold)",
            "Show mercy",
            "Leave",
        ],
        Archetype::Traveller => &["Ask for a rumour", "Ask about the Sigils", "Leave"],
        Archetype::Smuggler => &[
            "Accept the dockside job / collect instructions",
            "Deliver the parcel to the smugglers",
            "Fence the caravan goods",
            "Leave",
        ],
        // D32/D36: the brewing alchemist; the still keeps its own hours.
        Archetype::Alchemist => {
            let mut options: Vec<String> = vec![
                "Browse the draughts".into(),
                "Ask about fester".into(),
            ];
            if game.player.class == Class::Fensworn
                && matches!(game.trials[2].stage, TrialStage::Locked)
                && game.player.level >= 3
            {
                options.push("Ask about the old truce".into());
            }
            for (label, _) in transmute_offer(game) {
                options.push(label);
            }
            options.push("Leave".into());
            return options;
        }
        _ => &[],
    };
    if !fixed.is_empty() {
        let mut options: Vec<String> = fixed.iter().map(|s| (*s).into()).collect();
        // E7 (§6.5): the road warden walks with any caravan-worn traveller.
        if n.archetype == Archetype::Traveller
            && game.player.class == Class::Waysworn
            && matches!(game.trials[5].stage, TrialStage::Locked)
            && game.player.level >= 3
            && game.player.map == 0
        {
            options.insert(options.len() - 1, "Walk the oathroad with the caravan".into());
        }
        return options;
    }
    match n.archetype {
        Archetype::Vendor => {
            let mut options = vec![
                "Browse wares".into(),
                "Ask about work / report progress".into(),
                "Rent an inn bed (8 gold)".into(),
            ];
            if is_smith(n) {
                options.push("Forge two matching pieces into the next tier".into());
                // E7 (D32): the one-per-run masterwork beat, quest-gated on a sigil.
                if game.player.sigils.iter().any(|s| *s) && !game.masterwork_used {
                    options.push("Ask for the one-handed masterwork anvil".into());
                }
                // E7 (D31/§8.1): the smith's socket-punch (75g) — writes a Word
                // when both glyphs stand in your pack and the gear is fine+.
                for (wi, word) in WORDS.iter().enumerate() {
                    let equipped_tier = match word.slot {
                        WordSlot::Weapon => game.player.weapon,
                        WordSlot::Armour => game.player.armour,
                    };
                    let taken = match word.slot {
                        WordSlot::Weapon => game.player.word_weapon,
                        WordSlot::Armour => game.player.word_armour,
                    };
                    if equipped_tier >= 2
                        && taken != Some(wi as u8)
                        && game.player.inventory.contains(&Item::Glyph(word.seq[0]))
                        && game.player.inventory.contains(&Item::Glyph(word.seq[1]))
                    {
                        options.push(format!(
                            "Socket the {} word into your {} (75g) — {}",
                            word.name,
                            match word.slot {
                                WordSlot::Weapon => "weapon",
                                WordSlot::Armour => "armour",
                            },
                            word.effect
                        ));
                    }
                }
            }
            options.push("Leave".into());
            options
        }
        Archetype::Guard => {
            let mut options = vec![
                "Request gate passage".into(),
                "Offer a bribe (15 gold)".into(),
                "Report the smuggler's parcel".into(),
            ];
            if n.name.contains("Guard Captain") {
                options.push("Ask about bounties".into());
                // E7 (§6.5): the garrison's own trial for its own.
                if game.player.class == Class::Keepwarden
                    && matches!(game.trials[0].stage, TrialStage::Locked)
                    && npc_city(game, npc) == 1
                    && game.player.level >= 5
                {
                    options.push("Ask about the garrison's manual".into());
                }
                if game.player.class == Class::Redwake
                    && matches!(game.trials[3].stage, TrialStage::Locked)
                    && npc_city(game, npc) == 2
                    && game.player.level >= 7
                {
                    options.push("Ask about the Charter of the Tide".into());
                }
            }
            options.push("Leave".into());
            options
        }
        Archetype::Oracle => {
            // Seal-Reader (Ink 1): the schism reads the script with practiced ease.
            let discount = if game.player.class == Class::SigilSworn && game.player.has_talent(1, 0) {
                10
            } else {
                0
            };
            let tuition = match game.player.spells.len() {
                0 => 25u32,
                1 => 45,
                _ => 70,
            }
            .saturating_sub(discount);
            let mut options = vec![
                "Seek a prophecy".into(),
                "Identify a relic".into(),
                "Ask about the Sigils".into(),
                if Spell::next_unlearned(&game.player.spells).is_some() {
                    format!("Study the old runes ({tuition} gold)")
                } else {
                    "Study the old runes (all known)".into()
                },
            ];
            // The one free respec (D8): the oracles unwind your lessons, once.
            if game.player.class != Class::None && !game.player.talents.is_empty() {
                options.push(if game.player.respec_used {
                    "Unswear your gifts (already granted)".into()
                } else {
                    "Unswear your gifts (free, once)".into()
                });
            }
            // E7 (§6.5): the schism's verse awaits its reader (act II gate); the
            // garrison's true name waits for the act V road-worn.
            if game.player.class == Class::SigilSworn
                && matches!(game.trials[1].stage, TrialStage::Locked)
                && game.player.level >= 3
            {
                options.push("Ask about the forbidden verse".into());
            }
            if game.player.class == Class::Gravebound
                && matches!(game.trials[4].stage, TrialStage::Locked)
                && game.player.level >= 9
            {
                options.push("Ask for the Last Watch's name".into());
            }
            options.push("Leave".into());
            options
        }
        Archetype::Companion => {
            let mut options = vec![];
            if game.companion == Some(npc) {
                options.push("Stand down (dismiss companion)".into());
            } else if game.companion.is_none() {
                options.push("Hire as companion (50 gold)".into());
            }
            options.push("Ask for a rumour".into());
            options.push("Leave".into());
            options
        }
        _ => vec![
            "Greet the local".into(),
            "Ask for directions".into(),
            "Leave".into(),
        ],
    }
}

/// Display name for the three city indices used by city, reputation and keyring arrays.
pub fn city_name(city: usize) -> &'static str {
    ["Millbrook", "Highgate", "Saltmarsh"]
        .get(city)
        .copied()
        .unwrap_or("Millbrook")
}

fn is_smith(npc: &Npc) -> bool {
    npc.archetype == Archetype::Vendor
        && ["Blacksmith", "Weaponsmith", "Armourer", "Black Market"]
            .iter()
            .any(|mark| npc.name.contains(mark))
}

// ---------------------------------------------------------------------------
// E7 — The Second Watch systems (docs/D2_EVOLUTION.md §6.5, §8.1, §8.2)
// ---------------------------------------------------------------------------

/// E7 (§6.5 act order): which act this map answers to, for the journal's header.
pub fn act_of(map: usize) -> (u8, &'static str) {
    match map {
        1 | 4 | 15 => (1, "The Shire Below"),
        0 | 16 => (2, "The Ford Wars"),
        2 | 13 => (3, "The Ridge and the Hunt"),
        3 | 14 => (4, "Salt and Smoke"),
        17 => (3, "The Ridge and the Hunt"),
        18 => (3, "The Ridge and the Hunt"),
        9..=11 | 19 | 20 | 25 => (5, "The Oathless"),
        7 | 8 => (3, "The Ridge and the Hunt"),
        12 => (6, "The Second Watch"),
        _ => (2, "The Ford Wars"),
    }
}

/// E7 (D32/§8.2): the chalk-board transmute set — discovery by possession:
/// a recipe's row appears when its ingredients stand in your pack; performing
/// it writes the recipe to the codex, permanently (Qud recipe memory).
pub fn transmute_recipes() -> Vec<(&'static str, Vec<Item>, Vec<Item>)> {
    vec![
        ("3 healing potions → greater potion", vec![Item::Potion, Item::Potion, Item::Potion], vec![Item::GreaterPotion]),
        ("2 trail rations → traveler's ration", vec![Item::Ration, Item::Ration], vec![Item::TravelerRation]),
        ("2 gem dust → glyph shard", vec![Item::GemDust, Item::GemDust], vec![Item::GlyphShard]),
        ("2 herb clusters → anti-toxin", vec![Item::HerbCluster, Item::HerbCluster], vec![Item::AntiToxin]),
        ("ore flake + healing potion → mana tonic", vec![Item::OreFlake, Item::Potion], vec![Item::ManaTonic]),
        ("herb cluster + ore flake → 2 gem dust", vec![Item::HerbCluster, Item::OreFlake], vec![Item::GemDust, Item::GemDust]),
        ("2 glyph shards → a seal-glyph (the still remembers its shape)", vec![Item::GlyphShard, Item::GlyphShard], vec![]),
    ]
}

fn transmute_offer(game: &Game) -> Vec<(String, Option<usize>)> {
    transmute_recipes()
        .into_iter()
        .enumerate()
        .filter(|(_, (_, inputs, _))| {
            let mut have = game.player.inventory.clone();
            inputs.iter().all(|want| {
                have.iter()
                    .position(|i| i == want)
                    .map(|p| have.remove(p))
                    .is_some()
            })
        })
        .map(|(i, (name, _, _))| (format!("Transmute: {name}"), Some(i)))
        .collect()
}


fn answer(npc: &Npc, key: &str) -> f32 {
    npc.answers.get(key).map_or(0.0, |a| a.value)
}

fn npc_city(game: &Game, id: usize) -> usize {
    let n = &game.npcs[id];
    if let MapKind::City(city) = game.maps[n.map].kind {
        return city;
    }
    game.maps[0]
        .portals
        .iter()
        .filter_map(|p| {
            if let MapKind::City(city) = game.maps[p.destination].kind {
                Some((n.pos.distance(p.pos), city))
            } else {
                None
            }
        })
        .min_by_key(|(distance, _)| *distance)
        .map_or(game.player.last_city, |(_, city)| city)
}

impl Game {
    fn social_event(&mut self, npc: usize, event: SocialEvent, tier: u8) {
        if self
            .npcs
            .get(npc)
            .is_none_or(|n| !n.alive() || n.map != self.player.map)
        {
            return;
        }
        if self.social_pending.is_some() {
            self.log("Still judging your previous request. You may close this window and keep exploring.");
            return;
        }
        if matches!(event, SocialEvent::Greet) {
            self.haggle = None;
        }
        // Fresh event requests replace earlier tactical requests; only this version may apply the effect.
        self.npcs[npc].decision_pending = false;
        self.npcs[npc].answers.clear();
        self.request_decision(npc, tier);
        if !self.npcs[npc].decision_pending {
            self.log("The nearby decision queue is busy. Try this interaction again in a moment.");
            return;
        }
        if let Some(request) = self.outbox.last_mut() {
            request.state["interaction"] = serde_json::json!({"event":format!("{event:?}"),"bribe_gold":15,"discount_percent":20});
        }
        self.social_pending = Some((npc, self.npcs[npc].decision_version, event));
        self.log(format!("{} is judging your request…", self.npcs[npc].name));
    }

    pub fn interact(&mut self) {
        if self.combat.is_some() {
            self.log("Deal with the enemies before talking or travelling.");
            return;
        }
        self.update_quests();
        let pos = self.player.pos;
        if self.player.map == 0 && pos.distance(CARAVAN) <= 1 {
            self.recover_caravan();
            return;
        }
        if self.player.map == 3 && pos.distance(PARCEL) <= 1 {
            self.collect_parcel();
            return;
        }
        let portal = self
            .map()
            .portals
            .iter()
            .filter(|p| pos.distance(p.pos) <= 1)
            .min_by_key(|p| pos.distance(p.pos))
            .cloned();
        if let Some(portal) = portal {
            self.enter_portal(portal);
            return;
        }
        // Bond-wolf (D5, §4.2): a Packlord may swear the old truce with a wolf
        // that knows their name; it becomes the one companion, on wolf terms.
        let truce_wolf = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive() && n.map == self.player.map && self.wolf_truce_holds(n)
            })
            .filter(|n| n.pos.distance(pos) <= 1)
            .min_by_key(|n| n.pos.distance(pos))
            .map(|n| n.id);
        if let Some(id) = truce_wolf {
            self.swear_wolf_truce(id);
            return;
        }
        let npc = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && !n.archetype.hostile()
                    && n.pos.distance(pos) <= 1
            })
            .min_by_key(|n| n.pos.distance(pos))
            .map(|n| n.id);
        if let Some(id) = npc {
            self.selected = 0;
            if self.npcs[id].archetype == Archetype::Vendor
                && !self.npcs[id].name.contains("Innkeeper")
            {
                if self.night() {
                    self.log("The shop is closed (open 06:00–20:00). Inns remain open.");
                    return;
                }
                self.modal = Modal::Trade(id);
            } else {
                self.modal = Modal::Talk(id);
            }
            self.social_event(id, SocialEvent::Greet, 2);
            return;
        }
        let chest = (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| pos.offset(dx, dy)))
            .find(|p| self.map().tile(*p) == Tile::Chest);
        if let Some(chest) = chest {
            let tier = match self.map().kind {
                // Gears stop at masterwork even in the deep pockets (E5 arenas
                // share themes 4–7 with no tier tables above 3).
                MapKind::Dungeon(theme, _) => (theme.min(2) + 1) as u8,
                _ => 1,
            };
            let roll =
                mix(self.seed ^ self.player.map as u64 ^ ((chest.x as u64) << 16) ^ chest.y as u64);
            let item = if roll.is_multiple_of(5) {
                Item::Relic
            } else if roll.is_multiple_of(7) {
                // E7 (D31 §8.1): shrine-lost fragments surface in chest finds.
                Item::Glyph((roll % 6) as u8)
            } else if roll.is_multiple_of(3) {
                Item::Weapon(tier)
            } else {
                Item::Potion
            };
            if self.player.inventory.len() >= CAPACITY {
                self.log("Make room in your pack before opening this chest.");
                return;
            }
            self.maps[self.player.map].set(chest, Tile::Floor);
            let gold = 12 + u32::from(tier) * 8;
            self.player.gold += gold;
            self.log(format!("Treasure: {gold} gold and {}.", item.name()));
            self.player.inventory.push(item);
            return;
        }
        if (-1..=1).any(|dy| {
            (-1..=1).any(|dx| self.map().tile(Pos::new(pos.x + dx, pos.y + dy)) == Tile::Shrine)
        }) {
            let oracle_near = self.npcs.iter().any(|n| {
                n.alive()
                    && n.map == self.player.map
                    && n.archetype == Archetype::Oracle
                    && n.pos.distance(pos) <= 3
            });
            if oracle_near {
                self.oracle();
            } else if matches!(self.map().kind, MapKind::Arena)
                && !self.npcs.iter().any(|n| {
                    n.alive()
                        && n.map == self.player.map
                        && n.archetype == Archetype::Adjudicator
                })
                && self.player.inventory.contains(&Item::FirstWrit)
                && self.won
            {
                // E7 (§6.5): the arbiter's question, asked where the judgment
                // happened. E again re-reads the page and changes your answer.
                self.epilogue = match self.epilogue {
                    crate::model::Epilogue::Unanswered | crate::model::Epilogue::Watch => {
                        crate::model::Epilogue::Sealed
                    }
                    crate::model::Epilogue::Sealed => crate::model::Epilogue::Watch,
                };
                let sincerity = if self.mercy_oathbreaker {
                    "and saw your mercy for the oathbreaker was sincere"
                } else {
                    "and weighed what you chose over what he was owed"
                };
                self.log(match self.epilogue {
                    crate::model::Epilogue::Sealed => format!(
                        "The written page flares against your chest. 'Seal the cell forever and the seals hold unattended,' says the settling-stone voice, {sincerity}. (E here again to choose otherwise: Assume the Watch.)"
                    ),
                    crate::model::Epilogue::Watch => format!(
                        "'Assume the Watch,' the voice allows, {sincerity}. 'The post that held him — takes you. The second watch begins when you say the word.' (E here again to choose the sealing.)"
                    ),
                    crate::model::Epilogue::Unanswered => unreachable!(),
                }
                .to_string());
            } else if matches!(self.map().kind, MapKind::Arena)
                && self.npcs.iter().any(|n| {
                    n.alive()
                        && n.map == self.player.map
                        && n.archetype == Archetype::Adjudicator
                })
            {
                // D13 cruel terms: the Trial's shrine swears or withdraws them.
                // Under oath, fleeing its judgment is refused (and its final
                // rating notes that no step was taken back).
                self.trial_oath = !self.trial_oath;
                self.log(if self.trial_oath {
                    "You kneel at the verdict shrine and swear it aloud. A voice like settling stone answers: 'Then there shall be no retreat. Stand, and be weighed the truer.' (No fleeing this Trial; press E here again to take the words back.)"
                } else {
                    "You take back the hard words. 'Mercy, even here,' the stone allows. (The Trial will permit retreat once more.)"
                });
            } else if let Some(shrine) = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| pos.offset(dx, dy)))
                .find(|p| self.map().tile(*p) == Tile::Shrine)
            {
                self.maps[self.player.map].set(shrine, Tile::Floor);
                self.player.hp = self.player.max_hp;
                self.player.stamina = self.player.max_stamina;
                self.log("The wayshrine's last light restores your health and stamina. Its flame is spent.");
            }
            return;
        }
        let door = [pos, pos.offset(0, -1), pos.offset(1, 0), pos.offset(0, 1), pos.offset(-1, 0)]
            .into_iter()
            .find(|p| self.map().tile(*p) == Tile::Door);
        if let Some(door) = door {
            let open = self.map().door_open(door);
            if open && (door == pos || self.npc_at(door).is_some()) {
                self.log("The doorway is occupied. Step clear before closing it.");
            } else {
                self.maps[self.player.map].set_door_open(door, !open);
                self.log(if open { "You close the timber door." } else { "You open the timber door." });
            }
            return;
        }
        self.log("Nothing to interact with nearby. Stand beside a person, stair, gate, shrine or quest chest.");
    }

    fn enter_portal(&mut self, portal: Portal) {
        if !self.portal_unlocked(portal.requirement) {
            self.log(if portal.requirement == Some(3) {
                "The Trial requires all three Sigils: Chief, Matriarch and Lich."
            } else { "A dungeon seal is required. Complete a city's quest or buy its seal from a merchant." });
            return;
        }
        if self.player.map == 0
            && self.night()
            && matches!(self.maps[portal.destination].kind, MapKind::City(_))
        {
            let guard = self
                .npcs
                .iter()
                .filter(|n| {
                    n.alive()
                        && n.map == self.player.map
                        && n.archetype == Archetype::Guard
                        && n.pos.distance(portal.pos) <= 3
                })
                .min_by_key(|n| n.pos.distance(portal.pos))
                .map(|n| n.id);
            if let Some(guard) = guard {
                // The permit pays for passage through this gate, not for one specific
                // guard's shift: guards rotate, so only the destination is matched.
                if self.gate_permit.map(|(_, destination)| destination) != Some(portal.destination)
                {
                    self.modal = Modal::Talk(guard);
                    self.selected = 0;
                    self.log("The night gate is shut. Request passage or offer this gate's guard a bribe.");
                    self.social_event(guard, SocialEvent::Greet, 2);
                    return;
                }
            } else {
                self.log("The night gate is unstaffed. Wait until 06:00.");
                return;
            }
        }
        self.gate_permit = None;
        self.social_pending = None;
        self.haggle = None;
        self.player.map = portal.destination;
        self.maps[self.player.map].set_door_open(portal.arrival, true);
        self.player.pos = portal.arrival;
        self.survey_reveal();
        if let Some(city) = self.city() {
            self.player.last_city = city;
            self.visited[city] = true;
        }
        self.modal = Modal::None;
        self.selected = 0;
        self.hour_ticks += 8;
        self.log(format!("Entered {}.", self.map().name));
        self.companion_follow();
        self.reveal();
        self.update_quests();
        self.refresh_combat();
    }

    pub fn portal_unlocked(&self, requirement: Option<usize>) -> bool {
        match requirement {
            None => true,
            Some(3) => self.player.sigils.iter().all(|v| *v),
            Some(i) => self.player.keys.get(i).copied().unwrap_or(false),
        }
    }

    /// Potion strength with class draughts (Grave Comforts +4, Rotgut's variant is stamina-side).
    pub fn potion_heal(&self) -> i32 {
        18 + if self.player.class == Class::Gravebound && self.player.has_talent(2, 1) {
            4
        } else {
            0
        }
    }
    pub fn use_item(&mut self, index: usize) -> bool {
        let Some(item) = self.player.inventory.get(index).cloned() else {
            return false;
        };
        match item {
            Item::Potion => {
                if self.player.hp == self.player.max_hp {
                    self.log("You are already at full health.");
                    return false;
                }
                let heal = self.potion_heal();
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                if self.player.class == Class::Redwake && self.player.has_talent(1, 1) {
                    self.player.stamina = (self.player.stamina + 3).min(self.player.max_stamina);
                }
                self.player.history.potions += 1;
                self.player.inventory.remove(index);
                self.log(format!("Drank a healing potion: restored up to {heal} HP."));
            }
            Item::Ration => {
                if self.player.hp == self.player.max_hp
                    && self.player.stamina == self.player.max_stamina
                {
                    self.log("You are already fed and rested.");
                    return false;
                }
                // Quartering (Tollwright 2): warmer quarters on the road.
                let quartered = i32::from(
                    self.player.class == Class::Waysworn && self.player.has_talent(1, 1),
                ) * 2;
                self.player.hp = (self.player.hp + 8 + quartered).min(self.player.max_hp);
                self.player.stamina = self.player.max_stamina;
                self.player.inventory.remove(index);
                self.log(format!(
                    "A trail ration restores up to {} HP and all stamina.",
                    8 + quartered
                ));
            }
            Item::Torch => {
                self.player.torch_until = self.player.torch_until.max(self.tick) + 960;
                self.player.inventory.remove(index);
                self.log(
                    "Lit a torch: six-tile night vision for two hours (about four real minutes).",
                );
                self.reveal();
            }
            Item::Weapon(tier) => {
                if tier == self.player.weapon {
                    self.log("You already wield that grade of weapon.");
                    return false;
                }
                self.player.inventory[index] = Item::Weapon(self.player.weapon);
                self.player.weapon = tier;
                self.log(format!(
                    "Equipped {} sword; old sword stowed.",
                    Item::tier(tier)
                ));
            }
            Item::Armour(tier) => {
                if tier == self.player.armour {
                    self.log("You already wear that grade of armour.");
                    return false;
                }
                self.player.inventory[index] = Item::Armour(self.player.armour);
                self.player.armour = tier;
                self.log(format!(
                    "Equipped {} armour; old armour stowed.",
                    Item::tier(tier)
                ));
            }
            Item::BossRelic(relic) => {
                if self.player.relic == Some(relic) {
                    self.log("You already carry that boss relic.");
                    return false;
                }
                if let Some(previous) = self.player.relic.replace(relic) {
                    self.player.inventory[index] = Item::BossRelic(previous);
                } else {
                    self.player.inventory.remove(index);
                }
                self.log(format!("Equipped {}: {}", relic.name(), relic.describe()));
            }
            Item::Key(key) => {
                if let Some(owned) = self.player.keys.get_mut(key) {
                    *owned = true;
                }
                self.player.inventory.remove(index);
                self.log(
                    "The seal is now on your permanent keyring; it is never consumed at a gate.",
                );
            }
            Item::Relic => {
                self.log("Bring this relic to a shrine oracle for identification.");
                return false;
            }
            Item::Delivery(city) => {
                self.log(format!(
                    "This parcel must reach {}'s guard captain. Caravans from the atlas (M) make the trip easy.",
                    city_name(city)
                ));
                return false;
            }
            Item::CaravanGoods | Item::Contraband => {
                self.log(
                    "Quest cargo must be delivered to the appropriate contact. Check your journal.",
                );
                return false;
            }
            // D36: the launch draughts. Anti-toxin purges fester and leaves an
            // afterglow; the tonic is the caster's mid-fight runway (§8.2).
            Item::AntiToxin => {
                let purged = self.cure_fester();
                self.player.fester_guard_until = self.turn + 8;
                self.player.inventory.remove(index);
                self.log(if purged {
                    "The anti-toxin burns going down: the fester purges, and its afterglow steels you against new bites."
                } else {
                    "The anti-toxin's afterglow steels you against festering bites for a while."
                });
            }
            Item::ManaTonic => {
                if self.player.mana >= self.player.max_mana {
                    self.log("Your mana already brims; the tonic would spill.");
                    return false;
                }
                self.player.mana = (self.player.mana + 4).min(self.player.max_mana);
                self.player.inventory.remove(index);
                self.log("Bitter blue fire down the throat: +4 mana.");
            }
            // D8: the second unwinding the oracles will not grant twice.
            Item::Essence => {
                if self.player.talents.is_empty() {
                    self.log("No sworn gifts to unwind; the essence would be wasted.");
                    return false;
                }
                self.player.inventory.remove(index);
                self.log("The essence unwrites what the shrines taught.");
                self.respec();
            }
            // E7 (§6.5/D8): the covenant-breaker's ransom reads the same lesson,
            // but the arbiter's question is the better use of the page.
            Item::FirstWrit => {
                if self.player.talents.is_empty() {
                    self.log("The writ whispers: carry it past the Trial, and the arbiter will ask its question.");
                    return false;
                }
                self.player.inventory.remove(index);
                self.log("The First Writ unwrites what the shrines taught; its question stays unanswered.");
                self.epilogue = crate::model::Epilogue::Unanswered;
                self.respec();
            }
            Item::GreaterPotion => {
                if self.player.hp == self.player.max_hp {
                    self.log("You are already at full health.");
                    return false;
                }
                self.player.hp = (self.player.hp + 32).min(self.player.max_hp);
                self.player.inventory.remove(index);
                self.log("The greater draught floods your veins: restored up to 32 HP.");
            }
            Item::TravelerRation => {
                self.player.hp = (self.player.hp + 12).min(self.player.max_hp);
                self.player.stamina = self.player.max_stamina;
                self.player.inventory.remove(index);
                self.log("Traveler's fare: +12 HP, and the legs feel a hundred leagues lighter.");
            }
            Item::GemDust | Item::HerbCluster | Item::OreFlake => {
                self.log("Scrounge-fare for the still: bring it to an alchemist and try a transmute.");
                return false;
            }
            Item::GlyphShard => {
                self.log("Seal-script sand. The alchemist knows how to grind it back into glyphs.");
                return false;
            }
            Item::Glyph(_) => {
                self.log("A named fragment of seal-script. A smith can punch a socket and set words with it (75g).");
                return false;
            }
        }
        self.selected = self
            .selected
            .min(self.player.inventory.len().saturating_sub(1));
        self.update_quests();
        true
    }

    fn active_vendor(&self) -> Option<usize> {
        let id = match self.modal {
            Modal::Trade(id) => id,
            Modal::Inventory => {
                self.npcs
                    .iter()
                    .find(|n| {
                        n.alive()
                            && n.map == self.player.map
                            && n.archetype == Archetype::Vendor
                            && n.pos.distance(self.player.pos) <= 2
                    })?
                    .id
            }
            _ => return None,
        };
        let n = self.npcs.get(id)?;
        (n.alive()
            && n.archetype == Archetype::Vendor
            && n.map == self.player.map
            && n.pos.distance(self.player.pos) <= 2
            && !self.night()
            && self.combat.is_none())
        .then_some(id)
    }

    pub fn buy(&mut self, index: usize) {
        let Some(npc) = self.active_vendor() else {
            self.log("Find an open merchant to trade.");
            return;
        };
        if self.npcs[npc].decision_pending {
            self.log("The merchant is judging your terms…");
            return;
        }
        let Some((item, base)) = stock(npc_city(self, npc)).get(index).cloned() else {
            return;
        };
        if let Item::Key(key) = item {
            if self.player.keys[key] {
                self.log("You already hold that permanent seal.");
                return;
            }
        }
        // Seals go straight onto the keyring; a full pack must not block their purchase.
        if !matches!(item, Item::Key(_)) && self.player.inventory.len() >= CAPACITY {
            self.log("Your pack is full (20 slots). Sell or use an item first.");
            return;
        }
        let price = buy_price(self, npc, base);
        if self.player.gold < price {
            self.log(format!("You need {price} gold for {}.", item.name()));
            return;
        }
        self.player.gold -= price;
        let name = item.name();
        if let Item::Key(key) = item {
            self.player.keys[key] = true;
        } else {
            self.player.inventory.push(item);
        }
        self.npcs[npc]
            .memory
            .remember(self.turn, "fair_trade", 0.02);
        self.log(format!("Bought {name} for {price} gold."));
        self.update_quests();
    }

    pub fn sell(&mut self, index: usize) {
        let Some(npc) = self.active_vendor() else {
            self.log("Find an open merchant to trade.");
            return;
        };
        let Some(item) = self.player.inventory.get(index) else {
            return;
        };
        let mut price = sell_price(item);
        // Fence's Friend (Dockhand 3): a Redwake always knows a buyer.
        if self.player.class == Class::Redwake && self.player.has_talent(1, 2) {
            price = price * 5 / 4;
        }
        if price == 0 {
            self.log("That is quest cargo or a relic. Deliver or identify it; it cannot be sold as ordinary stock.");
            return;
        }
        let item = self.player.inventory.remove(index);
        self.player.gold += price;
        self.npcs[npc]
            .memory
            .remember(self.turn, "fair_trade", 0.01);
        self.log(format!("Sold {} for {price} gold.", item.name()));
        self.selected = self
            .selected
            .min(self.player.inventory.len().saturating_sub(1));
    }

    pub fn haggle(&mut self) {
        let Some(npc) = self.active_vendor() else {
            return;
        };
        if self.haggle.is_some_and(|(id, _)| id == npc) {
            self.log("These are the merchant's final terms for this visit.");
            return;
        }
        self.social_event(npc, SocialEvent::Haggle, 2);
    }

    pub fn talk_choice(&mut self, choice: usize) {
        let id = match self.modal {
            Modal::Talk(id) | Modal::Trade(id) => id,
            _ => return,
        };
        let Some(npc) = self.npcs.get(id) else {
            return;
        };
        if !npc.alive() || npc.map != self.player.map {
            return;
        }
        let archetype = npc.archetype;
        let city = npc_city(self, id);
        let options = talk_options(self, id);
        if choice >= options.len() {
            return;
        }
        if choice == options.len() - 1 {
            self.modal = Modal::None;
            self.haggle = None;
            return;
        }
        if self.social_pending.is_some() {
            self.log("Judging… please wait for this request to finish.");
            return;
        }
        match (archetype, choice) {
            (Archetype::Vendor, 0) => {
                if self.night() { self.log("Shops open at 06:00."); } else { self.modal = Modal::Trade(id); self.selected = 0; }
            }
            (Archetype::Vendor, 1) => {
                if city == 0 { self.rat_work(id); }
                else if city == 1 { self.caravan_work(id); }
                else if self.npcs[id].name.contains("Fence") { self.fence_caravan(id); }
                else { self.log("Nessa offers work at the southeast docks (30,30). Silas fences caravan goods. Seals are also for sale here."); }
            }
            (Archetype::Vendor, 2) => { self.rest_at_inn(); }
            (Archetype::Vendor, 3) => {
                self.modal = Modal::Forge(id);
                self.selected = 0;
            }
            (Archetype::Vendor, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("masterwork anvil")) =>
            {
                self.masterwork_beat();
            }
            (Archetype::Vendor, c)
                if options.get(c).is_some_and(|o| o.starts_with("Socket the ")) =>
            {
                let label = &options[c];
                if let Some(wi) = WORDS
                    .iter()
                    .position(|w| label.starts_with(&format!("Socket the {} ", w.name)))
                {
                    self.socket_word(wi);
                }
            }
            (Archetype::Guard, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("garrison's manual")) =>
            {
                self.offer_trial(0);
            }
            (Archetype::Guard, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("Charter of the Tide")) =>
            {
                self.offer_trial(3);
            }
            (Archetype::Oracle, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("forbidden verse")) =>
            {
                self.offer_trial(1);
            }
            (Archetype::Oracle, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("Last Watch's name")) =>
            {
                self.offer_trial(4);
            }
            (Archetype::Traveller, c)
                if options
                    .get(c)
                    .is_some_and(|o| o.contains("oathroad")) =>
            {
                self.offer_trial(5);
            }
            (Archetype::Alchemist, 0) => {
                // The still never cools; draughts sell at all hours (D32).
                self.modal = Modal::Trade(id);
                self.selected = 0;
            }
            (Archetype::Alchemist, 1) => {
                self.log(if self.player.fester > 0 {
                    format!(
                        "\"Fen-fever, {} stacks. Drink an anti-toxin now — or sleep it off at an inn before your wind gives out.\"",
                        self.player.fester
                    )
                } else {
                    "\"Barrow bites fester; that's what the anti-toxin is for — prevention tastes the same as cure. The blue bottle is a mana tonic: four embers for a seal-reader who runs dry mid-fight.\""
                        .into()
                });
            }
            (Archetype::Alchemist, c) => {
                // Trailing rows: the old-truce offer, then the chalk-board's
                // currently-possible transmutes, then Leave.
                let truce_row = 2;
                let offer_rows = transmute_offer(self);
                if self.player.class == Class::Fensworn && c == truce_row
                    && matches!(self.trials[2].stage, TrialStage::Locked)
                    && options.get(truce_row).is_some_and(|o| o.contains("old truce"))
                {
                    self.offer_trial(2);
                } else {
                    let base = truce_row
                        + usize::from(options.get(truce_row).is_some_and(|o| o.contains("old truce")));
                    if c >= base && c - base < offer_rows.len() {
                        if let Some(recipe) = offer_rows[c - base].1 {
                            self.transmute(recipe);
                        }
                    }
                }
            }
            (Archetype::Guard, 0) => self.social_event(id, SocialEvent::Passage, 2),
            (Archetype::Guard, 1) => self.social_event(id, SocialEvent::Bribe, 2),
            (Archetype::Guard, 2) => self.finish_smuggling(id, false),
            (Archetype::Guard, 3) => self.bounty_office(id),
            (Archetype::Companion, c) => {
                let hired = self.companion == Some(id);
                if c == 0 && (hired || self.companion.is_none()) {
                    self.toggle_companion(id);
                } else {
                    self.social_event(id, SocialEvent::Rumour, 2);
                }
            }
            (Archetype::Thief, 0) => self.social_event(id, SocialEvent::Reclaim, 2),
            (Archetype::Thief, 1) => {
                if self.player.gold < 10 { self.log("The informant wants 10 gold."); }
                else {
                    self.player.gold -= 10;
                    self.npcs[id].memory.remember(self.turn, "paid_for_information", 0.2);
                    self.log("The thief points to Nessa's dockside parcel (32,31), and a ruined arena at (72,44). Three Sigils open it.");
                }
            }
            (Archetype::Thief, 2) => self.social_event(id, SocialEvent::Mercy, 2),
            (Archetype::Traveller, 0) => self.social_event(id, SocialEvent::Rumour, 2),
            (Archetype::Traveller, 1) | (Archetype::Oracle, 2) => self.begin_sigil_hunt(),
            (Archetype::Smuggler, 0) => self.smuggling_work(),
            (Archetype::Smuggler, 1) => self.finish_smuggling(id, true),
            (Archetype::Smuggler, 2) => self.fence_caravan(id),
            (Archetype::Oracle, 0) => self.social_event(id, SocialEvent::Prophecy, 4),
            (Archetype::Oracle, 1) => {
                if !self.player.inventory.contains(&Item::Relic) { self.log("You have no unidentified relic. Search dungeon treasure chests."); }
                else { self.social_event(id, SocialEvent::Identify, 4); }
            }
            (Archetype::Oracle, 3) => {
                if Spell::next_unlearned(&self.player.spells).is_none() {
                    self.log("You have mastered every rune the shrines can teach.");
                } else {
                    self.social_event(id, SocialEvent::Study, 2);
                }
            }
            (Archetype::Oracle, 4) if !self.player.talents.is_empty() => {
                // The one free unwinding (D8); index 4 is "Leave" when no lessons exist.
                if self.player.respec_used {
                    self.log("The shrines teach humility: no unwinding remains for you here.");
                } else {
                    self.player.respec_used = true;
                    self.respec();
                    self.modal = Modal::None;
                }
            }
            (_, 0) => self.social_event(id, SocialEvent::Greet, 2),
            (_, 1) => self.log("Millbrook southwest (35,115); Highgate northeast (145,40); Saltmarsh southeast (164,132). Roads and the two river fords connect them."),
            _ => {}
        }
        self.update_quests();
    }

    /// Surveyor (Farwanderer 4): gates announce themselves to the seasoned warden.
    fn survey_reveal(&mut self) {
        if self.player.class != Class::Waysworn || !self.player.has_talent(2, 3) {
            return;
        }
        let map = &mut self.maps[self.player.map];
        let marks: Vec<Pos> = map.portals.iter().map(|p| p.pos).collect();
        let announced = !marks.is_empty();
        for pos in marks {
            if let Some(i) = map.index(pos) {
                map.explored[i] = true;
            }
        }
        if announced {
            self.log("The warden's eye traces the gates; the map marks itself.");
        }
    }

    pub fn rest_at_inn(&mut self) -> bool {
        if self.city().is_none() || self.combat.is_some() {
            return false;
        }
        if self.player.gold < 8 {
            self.log("An inn bed costs 8 gold. Eat a ration or earn coin first.");
            return false;
        }
        self.player.gold -= 8;
        self.player.hp = self.player.max_hp;
        self.player.stamina = self.player.max_stamina;
        self.player.mana = self.player.max_mana;
        // D36: the inn's hot broth is the working cure for fen-fever.
        let cleansed = self.cure_fester();
        self.hour_ticks += 8 * 480;
        self.haggle = None;
        self.gate_permit = None;
        self.log(if cleansed {
            "A safe inn bed restores all HP and stamina; the fever breaks overnight. Eight hours pass (8 gold)."
        } else {
            "A safe inn bed restores all HP and stamina. Eight hours pass (8 gold)."
        });
        true
    }

    pub fn travel(&mut self, city: usize) {
        let Some(origin) = self.city() else {
            self.log(
                "Caravans depart only from towns; you cannot fast-travel out of the wilderness.",
            );
            return;
        };
        if self.combat.is_some() {
            self.log("You cannot travel during combat.");
            return;
        }
        if city >= 3 || !self.visited[city] {
            self.log("Discover that city on foot before booking its caravan.");
            return;
        }
        if origin == city {
            self.log("You are already here.");
            return;
        }
        // Rights of Way: the toll register knows the code (halved fare).
        let fee = if self.player.class == Class::Waysworn && self.player.has_talent(1, 0) {
            5
        } else {
            10
        };
        // Tollcoin Charm (§6): tolls read at half, stack and all.
        let fee = if self.player.relic == Some(BossRelic::TollcoinCharm) {
            (fee / 2).max(1)
        } else {
            fee
        };
        if self.player.gold < fee {
            self.log(format!("A safe caravan seat costs {fee} gold."));
            return;
        }
        self.player.gold -= fee;
        self.player.map = city + 1;
        self.maps[self.player.map].set_door_open(Pos::new(20, 35), true);
        self.player.pos = Pos::new(20, 35);
        self.player.last_city = city;
        self.survey_reveal();
        self.companion_follow();
        self.hour_ticks += 3 * 480;
        self.social_pending = None;
        self.gate_permit = None;
        self.haggle = None;
        self.modal = Modal::None;
        self.log(format!(
            "The licensed caravan brings you safely to {} after three hours ({fee} gold).",
            city_name(city)
        ));
        self.reveal();
        self.update_quests();
    }

    pub fn oracle(&mut self) {
        if self.combat.is_some() {
            self.log("A shrine cannot hear you in battle.");
            return;
        }
        let oracle = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && n.archetype == Archetype::Oracle
                    && n.pos.distance(self.player.pos) <= 3
            })
            .min_by_key(|n| n.pos.distance(self.player.pos))
            .map(|n| n.id);
        if let Some(id) = oracle {
            self.modal = Modal::Talk(id);
            self.selected = 0;
        } else {
            self.log("Visit a shrine oracle; each city's shrine is near (20,10).");
        }
    }

    pub fn apply_social_decision(&mut self, result: &DecisionResult) -> Option<String> {
        let (id, version, event) = self.social_pending?;
        if id != result.request.npc || version != result.request.version {
            return None;
        }
        self.social_pending = None;
        if !self.npcs[id].alive() || self.npcs[id].map != self.player.map {
            return None;
        }
        let city = npc_city(self, id);
        let reputation = self.player.reputation[city];
        let value = |key: &str| result.answers.get(key).map_or(0.0, |a| a.value);
        let rule = match event {
            SocialEvent::Greet => {
                match self.npcs[id].archetype {
                    Archetype::Vendor => {
                        let offer = result.answers.get("offer_quest").and_then(|a| a.choice.as_deref()).unwrap_or("collection");
                        self.log(format!("{} offers {offer} work. Press Q in trade to discuss the local job; H asks for better prices.", self.npcs[id].name));
                    }
                    Archetype::Commoner => {
                        if value("alert_guards") >= 0.65 && reputation < 40 {
                            self.player.reputation[city] = (reputation - 2).max(0);
                            self.npcs[id].memory.remember(self.turn, "alarmed_by_player", -0.1);
                            self.log("The commoner calls for the watch; your local reputation suffers.");
                        } else if value("flee_from_player") >= 0.65 {
                            self.npcs[id].intent = Intent::Flee;
                            self.log("The local shrinks back from your weapons.");
                        } else { self.log("The local welcomes you. Ask for directions, or visit the market and shrine."); }
                    }
                    Archetype::Guard => self.log("The guard considers your reputation. Request passage, offer a bribe, or report contraband."),
                    Archetype::Thief => self.log(if self.smuggling_choice == Some(true) { "A fellow smuggler recognises you; the thieves will respect your protection." } else { "The thief watches your purse. You can demand restitution, buy information, or show mercy." }),
                    _ => self.log(format!("{} is ready to speak.", self.npcs[id].name)),
                }
                "social greeting follows current event judgments".to_string()
            }
            SocialEvent::Haggle => {
                let accepted = value("haggle_accept") >= 0.5;
                self.haggle = Some((id, if accepted { 80 } else { 100 }));
                self.npcs[id].memory.remember(
                    self.turn,
                    "haggled",
                    if accepted { 0.03 } else { -0.02 },
                );
                self.log(if accepted {
                    "The merchant agrees: 20% off this visit's prices."
                } else {
                    "The merchant refuses. These are the final prices for this visit."
                });
                format!("haggle_accept >= 0.50 -> discount={accepted}")
            }
            SocialEvent::Passage | SocialEvent::Bribe => {
                let bribe = matches!(event, SocialEvent::Bribe);
                let suspect = value("suspect_player").max(value("suspect"));
                // Rights of Way (Tollwright 1): the toll-keepers honor the code's paper.
                let mut toll = if self.player.class == Class::Waysworn && self.player.has_talent(1, 0) {
                    8u32
                } else {
                    15u32
                };
                // Tollcoin Charm (§6): every bribe and toll reads at half.
                if self.player.relic == Some(BossRelic::TollcoinCharm) {
                    toll = (toll / 2).max(1);
                }
                let accepted = if bribe {
                    self.player.gold >= toll
                        && self.npcs[id].personality.gullibility >= 0.35
                        && value("accept_bribe") >= 0.5
                } else {
                    suspect < 0.6 || reputation >= 65
                };
                if bribe && self.player.gold < toll {
                    self.log(format!("You cannot afford the {toll}-gold bribe."));
                } else if accepted {
                    if bribe {
                        self.player.gold -= toll;
                        self.player.history.bribes += 1;
                        self.player.reputation[city] = (reputation - 2).max(0);
                        self.npcs[id]
                            .memory
                            .remember(self.turn, "accepted_bribe", 0.1);
                    }
                    let destination = self.maps[self.npcs[id].map]
                        .portals
                        .iter()
                        .find(|p| {
                            p.destination == city + 1 && p.pos.distance(self.npcs[id].pos) <= 3
                        })
                        .map(|p| p.destination);
                    if let Some(destination) = destination {
                        self.gate_permit = Some((id, destination));
                        self.modal = Modal::None;
                        self.log("The guard grants passage for this encounter. Press E at this city gate to enter.");
                    } else {
                        self.log(
                            "The guard lets you go with a nod; this is not an exterior night gate.",
                        );
                    }
                } else {
                    self.npcs[id]
                        .memory
                        .remember(self.turn, "suspicious_approach", -0.04);
                    self.log("The guard refuses. Try an honest appeal, a bribe, or wait for the gates to open at 06:00.");
                }
                format!("guard passage: suspicion={suspect:.2}, reputation={reputation}, gullibility gate, accepted={accepted}")
            }
            SocialEvent::Rumour => {
                if value("share_rumour") >= 0.4 {
                    let next = self.player.sigils.iter().position(|v| !v);
                    self.log(match next {
                        Some(0) => "Rumour: the Burrow lies at (52,78). Clear Millbrook's cellar for its seal. The Chief rallies surviving bandits.",
                        Some(1) => "Rumour: Crimson Hollow lies at (158,82). Highgate's caravan reward includes its seal. Kill the Matriarch's pack before she howls.",
                        Some(2) => "Rumour: the Underkeep lies at (145,12). Saltmarsh's parcel job earns its seal. Below it waits Vael, the Last Castellan — he would not abandon his post, even in death.",
                        _ => "Rumour: the Adjudicator waits at (72,44). It remembers your mercy, greed and courage. Take masterwork gear and potions.",
                    });
                } else {
                    self.log("The traveller keeps their rumours to themselves. The journal still marks your next objective.");
                }
                if value("warn_of_danger") >= 0.5 {
                    self.log("Warning: wolves guard the northern ford. Stay on roads; carry rations and healing potions.");
                }
                "rumour and warning use current share_rumour/warn_of_danger".into()
            }
            SocialEvent::Reclaim => {
                let owed: u32 = self.npcs[id]
                    .memory
                    .events
                    .iter()
                    .filter_map(|e| {
                        e.kind
                            .strip_prefix("stole:")
                            .and_then(|s| s.parse::<u32>().ok())
                    })
                    .sum();
                if owed == 0 {
                    self.log("There is no stolen gold from this thief to reclaim.");
                } else if value("flee_when_noticed") < 0.65
                    || self.player.level > self.npcs[id].level
                {
                    self.player.gold += owed;
                    self.npcs[id]
                        .memory
                        .events
                        .retain(|e| !e.kind.starts_with("stole:"));
                    self.npcs[id]
                        .memory
                        .remember(self.turn, "returned_stolen_gold", -0.1);
                    self.log(format!("The thief returns {owed} stolen gold."));
                } else {
                    self.npcs[id].intent = Intent::Flee;
                    self.log("The thief bolts rather than return the purse. Catch up and confront them again.");
                }
                "reclaim actual recorded theft; flee judgment and level oppose demand".into()
            }
            SocialEvent::Mercy => {
                if self.npcs[id].mercy_used {
                    self.log("You have already spared this thief.");
                } else {
                    self.npcs[id].mercy_used = true;
                    self.player.history.mercy += 1;
                    self.npcs[id]
                        .memory
                        .remember(self.turn, "player_showed_mercy", 0.4);
                    self.npcs[id].intent = if value("flee_when_noticed") >= 0.5 {
                        Intent::Flee
                    } else {
                        Intent::Idle
                    };
                    self.log("You let the thief go. Your mercy becomes part of the record the Adjudicator will judge.");
                }
                "mercy recorded once; current flee judgment controls response".into()
            }
            SocialEvent::Prophecy => {
                self.begin_sigil_hunt();
                if !self.npcs[id].mercy_used {
                    self.npcs[id].mercy_used = true;
                    self.player.max_hp += 2;
                    self.player.hp = (self.player.hp + 2).min(self.player.max_hp);
                    self.log("The shrine grants its one-time blessing: +2 maximum HP.");
                }
                let prophecy = result
                    .answers
                    .get("prophecy")
                    .and_then(|a| a.choice.as_deref())
                    .unwrap_or("danger");
                self.log(match prophecy {
                    "fortune" => "Prophecy: trade fairly and recover lost cargo; the city's rewards purchase the steel you will need.",
                    "mercy" => "Prophecy: the final judge remembers each life spared. Mercy changes the duel awaiting you.",
                    _ => "Prophecy: three lords hold three Sigils. Break the Chief's rally, silence the Matriarch's howl, then outlast Vael's phases.",
                });
                format!("tier4 prophecy={prophecy}; shrine blessing once")
            }
            SocialEvent::Identify => {
                if let Some(index) = self.player.inventory.iter().position(|i| *i == Item::Relic) {
                    let worth = value("identify_relic");
                    let tier = if worth >= 0.67 { 3 } else { 2 };
                    let item = if worth >= 0.34 {
                        Item::Weapon(tier)
                    } else {
                        Item::Armour(tier)
                    };
                    self.log(format!("The oracle identifies your relic as {}. It replaces the relic in your pack.", item.name()));
                    self.player.inventory[index] = item;
                    format!(
                        "tier4 identify_relic={worth:.2} -> deterministic tier {tier} equipment"
                    )
                } else {
                    self.log("The relic is no longer in your pack.");
                    "identification ignored: no relic".into()
                }
            }
            SocialEvent::Study => {
                let tuition = match self.player.spells.len() {
                    0 => 25u32,
                    1 => 45,
                    _ => 70,
                }
                .saturating_sub(if self.player.class == Class::SigilSworn
                    && self.player.has_talent(1, 0)
                {
                    10
                } else {
                    0
                });
                let Some(spell) = Spell::next_unlearned(&self.player.spells) else {
                    return Some("nothing left to teach".into());
                };
                if self.player.gold < tuition {
                    self.log(format!("The oracle asks {tuition} gold for the next rune."));
                    "study refused: tuition unpaid".into()
                } else if value("share_rumour") >= 0.35 {
                    self.player.gold -= tuition;
                    self.player.spells.push(spell);
                    self.player.max_mana = self.player.spells.len() as i32 * 2 + 2;
                    self.player.mana = self.player.max_mana;
                    self.npcs[id]
                        .memory
                        .remember(self.turn, "taught_runes", 0.2);
                    self.log(format!(
                        "Rune learned: {} ({} mana). {}",
                        spell.name(),
                        self.spell_cost(spell),
                        spell.describe()
                    ));
                    self.log("Mana returns slowly; press Y to cast. Rest at an inn to refill.");
                    format!("oracle's share_rumour >= 0.35 -> rune taught, tuition {tuition} gold")
                } else {
                    self.npcs[id]
                        .memory
                        .remember(self.turn, "study_refused", -0.02);
                    self.log(
                        "The oracle judges you unready for runic work. Return with a better record or a kinder oracle.",
                    );
                    "oracle's share_rumour < 0.35 -> study refused".into()
                }
            }
        };
        self.update_quests();
        Some(rule)
    }

    /// Craftable upgrades: two matching gear pieces plus coin become the next tier.
    pub fn forge_options(&self) -> Vec<(Item, u32)> {
        let mut options = Vec::new();
        // Oathpath Discount (Tollwright 3): the roads remember the wardens' accounts.
        let costs = if self.player.class == Class::Waysworn && self.player.has_talent(1, 2) {
            [22, 63, 135]
        } else {
            [25, 70, 150]
        };
        for tier in 0..3u8 {
            let swords = self
                .player
                .inventory
                .iter()
                .filter(|i| **i == Item::Weapon(tier))
                .count();
            if swords >= 2 {
                options.push((Item::Weapon(tier + 1), costs[tier as usize]));
            }
            let armours = self
                .player
                .inventory
                .iter()
                .filter(|i| **i == Item::Armour(tier))
                .count();
            if armours >= 2 {
                options.push((Item::Armour(tier + 1), costs[tier as usize]));
            }
        }
        options
    }

    pub fn forge(&mut self, index: usize) {
        let Some((item, cost)) = self.forge_options().get(index).cloned() else {
            return;
        };
        if self.player.gold < cost {
            self.log(format!("The smith wants {cost} gold for that work."));
            return;
        }
        let ingredient = match item {
            Item::Weapon(t) => Item::Weapon(t - 1),
            Item::Armour(t) => Item::Armour(t - 1),
            _ => return,
        };
        for _ in 0..2 {
            if let Some(slot) = self.player.inventory.iter().position(|i| *i == ingredient) {
                self.player.inventory.remove(slot);
            }
        }
        self.player.gold -= cost;
        self.player.inventory.push(item.clone());
        self.log(format!(
            "The anvil rings: {name} forged from two matching pieces ({cost} gold).",
            name = item.name()
        ));
        self.selected = self
            .selected
            .min(self.forge_options().len().saturating_sub(1));
    }

    pub(crate) fn toggle_companion(&mut self, id: usize) {
        if self.companion == Some(id) {
            self.companion = None;
            self.npcs[id].memory.remember(self.turn, "dismissed", -0.03);
            self.log(format!(
                "{} returns to their post. Hire them again any time.",
                self.npcs[id].name
            ));
        } else if self.companion.is_some() {
            self.log("You already travel with a companion. Dismiss them first.");
        } else {
            // Bond (§4.2): the hollow folk hire a sworn partner, not hired muscle.
            let hire_cost: u32 = if self.player.class == Class::Fensworn {
                25
            } else {
                50
            };
            if self.player.gold < hire_cost {
                self.log(format!("The sellsword expects {hire_cost} gold up front."));
            } else {
                self.player.gold -= hire_cost;
                // D22: the blade signs on at the player's level — hiring early is
                // the identity; D35 keeps it matched from then on. A blade that
                // already swore the bond keeps its +25% sheet, never double-said.
                let (hp, attack, defense, speed) =
                    crate::world::sellsword_stats(self.player.level);
                let bonded = self.npcs[id]
                    .memory
                    .events
                    .iter()
                    .any(|event| event.kind == "bond_sworn");
                let max_hp = if bonded { hp * 5 / 4 } else { hp };
                self.npcs[id].level = self.player.level;
                self.npcs[id].max_hp = max_hp;
                self.npcs[id].hp = max_hp;
                self.npcs[id].attack = attack;
                self.npcs[id].defense = defense;
                self.npcs[id].speed = speed;
                self.companion = Some(id);
                if self.player.class == Class::Fensworn
                    && !self.npcs[id]
                        .memory
                        .events
                        .iter()
                        .any(|event| event.kind == "bond_sworn")
                {
                    self.npcs[id].max_hp = self.npcs[id].max_hp * 5 / 4;
                    self.npcs[id].hp = self.npcs[id].max_hp;
                    self.npcs[id].memory.remember(self.turn, "bond_sworn", 0.1);
                }
                self.npcs[id].memory.remember(self.turn, "hired", 0.3);
                self.log(format!(
                    "{} joins you: fights at your side, anchors flanks, and draws blows meant for you.",
                    self.npcs[id].name
                ));
            }
        }
    }

    /// Bond-wolf (D5, §4.2): the Packlord's truce turns one wild wolf into the
    /// sworn partner — one bond only; dismiss the hired blade first.
    pub fn swear_wolf_truce(&mut self, id: usize) {
        if self.companion.is_some() {
            self.log("The wolf knows your name, but the old truce holds one bond only. Stand your partner down first.");
            return;
        }
        self.companion = Some(id);
        let name = self.npcs[id].name.clone();
        self.npcs[id].archetype = Archetype::Companion;
        self.npcs[id].intent = Intent::Idle;
        self.npcs[id].decision_pending = false;
        self.npcs[id].memory.remember(self.turn, "truce_sworn", 0.4);
        // The Fensworn's sworn-partner edge applies to the wolf as to any blade.
        self.npcs[id].max_hp = self.npcs[id].max_hp * 5 / 4;
        self.npcs[id].hp = self.npcs[id].max_hp;
        self.npcs[id].memory.remember(self.turn, "bond_sworn", 0.1);
        self.log("You speak the old words, palm open. The wolf sets its brow against your hand and the truce is sealed.");
        self.log(format!(
            "{name} walks with you now — it bites, it guards, and it will not thank you for drowning it in packs."
        ));
    }

    /// Teleports a living companion next to the player after map changes.
    pub fn companion_follow(&mut self) {
        let Some(id) = self.companion else { return };
        if !self.npcs[id].alive() {
            self.npcs[id].hp = if self.player.class == Class::Fensworn {
                self.npcs[id].max_hp
            } else {
                self.npcs[id].max_hp / 2
            };
        }
        let player = self.player.pos;
        self.npcs[id].map = self.player.map;
        let map = self.player.map;
        let spot = (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| player.offset(dx, dy)))
            .find(|p| {
                *p != player
                    && self.maps[map].tile(*p).walkable()
                    && self.npc_at(*p).is_none_or(|n| n == id)
            })
            .unwrap_or(player);
        self.maps[self.npcs[id].map].set_door_open(spot, true);
        self.npcs[id].pos = spot;
    }

    pub fn bounty_progress(&mut self, kind: Archetype) {
        for i in 0..self.bounties.len() {
            let quest = 5 + i;
            if self.quests[quest].stage != QuestStage::Active {
                continue;
            }
            if !matches!(self.bounties[i].kind, BountyKind::Cull(k) if k == kind) {
                continue;
            }
            self.quests[quest].progress += 1;
            if self.quests[quest].progress >= self.quests[quest].goal {
                self.quests[quest].stage = QuestStage::Ready;
                let bounty = &self.bounties[i];
                self.log(format!(
                    "Bounty fulfilled. Report to {}'s guard captain for {} gold and {} XP.",
                    city_name(bounty.origin),
                    bounty.gold,
                    bounty.xp
                ));
            }
        }
    }

    pub(crate) fn pay_bounty(&mut self, i: usize, context: &str) {
        let bounty = self.bounties[i].clone();
        self.quests[5 + i].stage = QuestStage::Complete;
        // E5 (§4.2 bounty-economy pass): the road pays its own — Waysworn bounties
        // pay +25% before the Gutter-Law and Caravan Code riders.
        // Bounty economics: Gutter-Law (+20%, +5% per Dockhand 1–2), Caravan Code (×2).
        let code = self.player.class == Class::Waysworn && self.player.has_talent(1, 3);
        let road = u32::from(self.player.class == Class::Waysworn) * 25;
        let gutter = self.player.class == Class::Redwake && self.player.has_talent(1, 0);
        let pct = road
            + 10 * self.word_on(WordSlot::Armour, "Lode") as u32
            + if gutter {
                20 + 5
                    * if self.player.has_talent(1, 2) {
                        u32::from(self.player.branch_synergy(1))
                    } else {
                        0
                    }
            } else {
                0
            };
        let gold = if code {
            bounty.gold * 2
        } else {
            bounty.gold + bounty.gold * pct / 100
        };
        self.player.gold += gold;
        self.player.reputation[bounty.origin] =
            (self.player.reputation[bounty.origin] + bounty.reputation).clamp(0, 100);
        self.player.history.quests += 1;
        // E5 (§6): bounty escalation — three settled culls of one kind call out
        // the alpha; the captain's tip-off marks the Hart's Stand.
        let slot = match bounty.kind {
            BountyKind::Cull(Archetype::Wolf) => Some(0),
            BountyKind::Cull(Archetype::Bandit) => Some(1),
            _ => None,
        };
        if let Some(slot) = slot {
            self.cull_wins[slot] = self.cull_wins[slot].saturating_add(1);
            if self.cull_wins[slot] == 3 {
                self.spawn_stag(bounty.origin);
            }
        }
        self.gain_xp(bounty.xp);
        self.log(format!(
            "{context}: +{} gold, +{} XP, +{} {} reputation.",
            gold,
            bounty.xp,
            bounty.reputation,
            city_name(bounty.origin)
        ));
    }

    fn bounty_office(&mut self, captain: usize) {
        let city = npc_city(self, captain);
        // A delivered parcel settles here.
        for i in 0..self.bounties.len() {
            if !matches!(self.bounties[i].kind, BountyKind::Deliver(d) if d == city)
                || self.quests[5 + i].stage != QuestStage::Active
            {
                continue;
            }
            if let Some(slot) = self
                .player
                .inventory
                .iter()
                .position(|item| *item == Item::Delivery(city))
            {
                self.player.inventory.remove(slot);
                let destination = city_name(city).to_string();
                self.pay_bounty(i, &format!("Delivery accepted by {destination}"));
                return;
            }
        }
        // A fulfilled cull is paid out by the captain who posted it.
        for i in 0..self.bounties.len() {
            if self.bounties[i].origin == city
                && matches!(self.bounties[i].kind, BountyKind::Cull(_))
                && self.quests[5 + i].stage == QuestStage::Ready
            {
                self.pay_bounty(i, "Bounty paid");
                return;
            }
        }
        // Otherwise: status, or a fresh contract when the board has room.
        for i in 0..self.bounties.len() {
            let quest = &self.quests[5 + i];
            if matches!(quest.stage, QuestStage::Active)
                && (self.bounties[i].origin == city
                    || matches!(self.bounties[i].kind, BountyKind::Deliver(d) if d == city))
            {
                self.log(format!(
                    "Open contract: {} ({}/{}).",
                    quest.title, quest.progress, quest.goal
                ));
                return;
            }
        }
        let open = self
            .bounties
            .iter()
            .enumerate()
            .filter(|(i, _)| self.quests[5 + i].stage != QuestStage::Complete)
            .count();
        if open >= 3 {
            self.log("The board is full. Finish what you carry before taking more.");
            return;
        }
        self.offer_bounty(city);
    }

    fn offer_bounty(&mut self, city: usize) {
        let roll = mix(self.seed ^ self.next_bounty.wrapping_mul(0x9e3779b97f4a7c15));
        self.next_bounty += 1;
        let count = 3 + (roll % 3) as u32;
        let gold = 25 + count * 9 + (roll % 20) as u32 + self.player.level * 4;
        let xp = 20 + count * 9;
        if roll.is_multiple_of(2) {
            let (kind, label) = if roll.is_multiple_of(4) {
                (Archetype::Wolf, "wolves")
            } else {
                (Archetype::Bandit, "bandits")
            };
            self.quests.push(Quest {
                title: format!("Bounty: cull {count} {label}"),
                stage: QuestStage::Active,
                progress: 0,
                goal: count,
                description: format!(
                    "Slay {count} {label}, then report to {}'s guard captain. Reward: {gold} gold, {xp} XP, +5 reputation.",
                    city_name(city)
                ),
            });
            self.bounties.push(Bounty {
                kind: BountyKind::Cull(kind),
                origin: city,
                gold,
                xp,
                reputation: 5,
            });
            self.log(format!(
                "The captain posts a cull bounty: {count} {label} for {gold} gold and {xp} XP."
            ));
        } else {
            if self.player.inventory.len() >= CAPACITY {
                self.log("The parcel needs one free pack slot. Make room first.");
                return;
            }
            let destination = (city + 1 + (roll % 2) as usize) % 3;
            self.player.inventory.push(Item::Delivery(destination));
            self.quests.push(Quest {
                title: format!("Bounty: deliver a parcel to {}", city_name(destination)),
                stage: QuestStage::Active,
                progress: 0,
                goal: 1,
                description: format!(
                    "Carry the sealed parcel to {}'s guard captain. Reward: {gold} gold, {xp} XP, +5 reputation.",
                    city_name(destination)
                ),
            });
            self.bounties.push(Bounty {
                kind: BountyKind::Deliver(destination),
                origin: city,
                gold,
                xp,
                reputation: 5,
            });
            self.log(format!(
                "The captain hands you a sealed parcel for {}. Caravans (M) make the trip easy.",
                city_name(destination)
            ));
        }
        self.update_quests();
    }

    /// E7 (D31, §8.1): the smith punches the socket and sets the Word in order.
    /// Plain gear only (fine+ carries two sockets); relics never take words.
    pub(crate) fn socket_word(&mut self, word_index: usize) {
        let word = &WORDS[word_index];
        if self.player.gold < 75 {
            self.log("The smith wants 75 gold for the socket-punch.");
            return;
        }
        let mut inventory = self.player.inventory.clone();
        for g in word.seq {
            let Some(p) = inventory.iter().position(|i| *i == Item::Glyph(g)) else {
                self.log("The glyphs no longer stand in your pack — check what you carry.");
                return;
            };
            inventory.remove(p);
        }
        // Glyph sequence confirmed and consumed; the word writes itself.
        self.player.inventory = inventory;
        self.player.gold -= 75;
        let replaced = match word.slot {
            WordSlot::Weapon => self.player.word_weapon.replace(word_index as u8),
            WordSlot::Armour => self.player.word_armour.replace(word_index as u8),
        };
        if let Some(old) = replaced {
            self.log(format!("The old word ({} ) washes out as the new letters bite.", WORDS[old as usize].name));
        }
        self.log(format!(
            "The socket rings true: {} written — {}.",
            word.name, word.effect
        ));
    }

    /// E7 (D32): the one-per-run masterwork beat — quest-gated by a sigil claim,
    /// one piece carried up by hand.
    pub(crate) fn masterwork_beat(&mut self) {
        if self.masterwork_used {
            self.log("The anvil's debt is paid once per journey, no more.");
            return;
        }
        if self.player.weapon < 3 {
            self.player.weapon += 1;
            self.log(format!("One piece, by hand: your blade is coaxed up to {} steel. The anvil's favor is spent.", Item::tier(self.player.weapon)));
        } else if self.player.armour < 3 {
            self.player.armour += 1;
            self.log(format!("One piece, by hand: your mail is coaxed up to {} plate. The anvil's favor is spent.", Item::tier(self.player.armour)));
        } else {
            self.log("Your gear already outranks what his hands can add. Sell the favor to no one — it stays unspent.");
        }
        self.masterwork_used = true;
    }

    /// E7 (D32, §8.2): one chalk-board recipe — spend inputs, take outputs, and
    /// write the recipe into the codex permanently on first success.
    pub(crate) fn transmute(&mut self, recipe: usize) {
        let (name, inputs, outputs) = transmute_recipes()[recipe].clone();
        let mut inventory = self.player.inventory.clone();
        for want in &inputs {
            let Some(p) = inventory.iter().position(|i| i == want) else {
                self.log("The still goes cold: the ingredients are gone from your pack.");
                return;
            };
            inventory.remove(p);
        }
        self.player.inventory = inventory;
        if recipe == 6 {
            // The remembered shape cycles through the six fragments.
            let frag = (self.codex.len() % 6) as u8;
            self.give_loot(Item::Glyph(frag));
        } else {
            for out in outputs {
                self.give_loot(out);
            }
        }
        if !self.codex.iter().any(|r| r == name) {
            self.codex.push(name.into());
            self.log(format!("The still answers: {name}. The recipe settles into your codex (J), permanently."));
        } else {
            self.log(format!("The still answers: {name}."));
        }
    }

    /// E7 (§6.5): the class-quest offer — one written direction each.
    pub(crate) fn offer_trial(&mut self, index: usize) {
        self.trials[index].stage = TrialStage::Offered;
        let line = match index {
            0 => "'The drill manual's last pages are in the keep's outworks — so says every ghost who guards them. The Drill Yard waits behind Highgate's west wall. Hold the gate through three waves and take what the garrison owed you.'",
            1 => "'Read wrong on purpose — you know the verse I mean. Step through the Hollow Shrine beside my shrine and duel the schismatic shade for the rest of the stanza. Channel the same rune until it learns you.'",
            2 => "'An old truce, a sick wild alpha, no pack to avenge it. The marsh off the Hollow holds it still. Go with an open palm, not a blade — this fight ends when mercy does.'",
            3 => "'The Charter's mark is one the watch can't touch. The Tide Locker off our southeast sheds holds the crew you need to walk out from under. Escape the ambush; the tide does the rest.'",
            4 => "'You would lay the garrison to rest? Then learn the name they died holding. The Barrow Post north of the keep mouths into the old watch-line. Survive what answers; it will tell you the name.'",
            5 => "'The oathroad outlived the covenant that wrote it. Walk with my cart through the Rest east of the ford. Wolves will read us as meat; the cargo mustn't read the same.'",
            _ => "",
        };
        self.log(format!("The order's trial is offered: {line}"));
    }

    /// E7 (§6.5): trial arenas' upkeep — starts on entry, progresses waves,
    /// completes on their per-trial verdict.
    pub(crate) fn trial_upkeep(&mut self) {
        for index in 0..6_usize {
            let trial_map = 21 + index;
            match self.trials[index].stage {
                TrialStage::Offered if self.player.map == trial_map => {
                    self.trials[index].stage = TrialStage::Active;
                    self.trials[index].wave = 0;
                    self.trial_spawn(index);
                    self.log(match index {
                        0 => "The Drill Yard's drums answer your step: three waves. HOLD THE GATE.",
                        1 => "The Hollow Shrine breathes. A shade steps out of the verse, waiting to be out-read.",
                        2 => "The marsh parts for a sick, wrong-eyed alpha. It has no pack left to call. Your palm or your blade?",
                        3 => "The Tide Locker slams shut: the crew that owned the Charter's mark turns at the trapdoor.",
                        4 => "The watch-line graves answer before you finish kneeling. Six of them. Ride the Vigil.",
                        5 => "The cart rolls. Two wolves already come down from the ridge — stay beside the cargo.",
                        _ => "",
                    });
                }
                TrialStage::Active => self.trial_progress(index, trial_map),
                TrialStage::Locked | TrialStage::Done | TrialStage::Offered => {}
            }
        }
    }

    fn trial_progress(&mut self, index: usize, trial_map: usize) {
        let pack = 6000 + index * 10;
        let pack_alive = self
            .npcs
            .iter()
            .any(|n| n.pack == pack && n.alive() && n.map == trial_map);
        match index {
            // The gate holds three waves before it holds itself.
            0 => {
                if self.player.map == trial_map && !pack_alive {
                    self.trials[0].wave += 1;
                    if self.trials[0].wave >= 3 {
                        self.complete_trial(0);
                    } else {
                        self.trial_spawn(0);
                        self.log(format!("The drums strike the {} watch. Wave {} of three!", ["second", "third"][(self.trials[0].wave - 1) as usize], self.trials[0].wave + 1));
                    }
                }
            }
            1 | 4 => {
                if !pack_alive {
                    self.complete_trial(index);
                }
            }
            // Pacify or fail: the alpha's fate is the verdict.
            2 => {
                let alpha = self.npcs.iter().find(|n| n.pack == pack && n.map == trial_map);
                match alpha {
                    Some(npc) if !npc.alive() => {
                        self.trials[2].stage = TrialStage::Offered;
                        self.trials[2].failed = true;
                        self.log("The alpha falls, and the marsh goes quiet in the wrong way. The truce will take another answer — return with an open palm.");
                    }
                    Some(npc)
                        if npc
                            .memory
                            .events
                            .iter()
                            .any(|e| e.kind == "player_spared_me") =>
                    {
                        self.complete_trial(2);
                    }
                    _ => {}
                }
            }
            // The escape IS the win: leave the Locker breathing and it's done.
            3 => {
                if self.player.map != trial_map {
                    self.complete_trial(3);
                }
            }
            5 => {
                let Some(guard) = self.trials[5].escort else {
                    return;
                };
                if !self.npcs[guard].alive() {
                    self.trials[5].stage = TrialStage::Offered;
                    self.trials[5].escort = None;
                    self.trials[5].failed = true;
                    self.log("The cart splinters. The cargo reads as meat after all. The oathroad will take you again when you return to the Rest.");
                } else if self.npcs[guard].pos.distance(Pos::new(7, 10)) <= 2 {
                    self.complete_trial(5);
                }
            }
            _ => {}
        }
    }

    fn trial_spawn(&mut self, index: usize) {
        let pack = 6000 + index * 10;
        let trial_map = 21 + index;
        let level = self.player.level;
        let seed = self.seed;
        fn summon_at(
            npcs: &mut Vec<Npc>,
            archetype: Archetype,
            map: usize,
            pos: Pos,
            lvl: u32,
            pack: usize,
            seed: u64,
            name: Option<&str>,
        ) -> usize {
            let id = npcs.len();
            npcs.push(crate::world::make_npc(id, archetype, map, pos, lvl, pack, seed));
            if archetype.hostile() {
                npcs[id].intent = Intent::Attack;
            }
            if let Some(name) = name {
                npcs[id].name = name.into();
            }
            id
        }
        let mut summon = |archetype, pos: Pos, lvl: u32, name: Option<&str>| -> usize {
            summon_at(&mut self.npcs, archetype, trial_map, pos, lvl, pack, seed, name)
        };
        match index {
            0 => {
                let (count, bonus) = match self.trials[0].wave {
                    0 => (2, 0),
                    1 => (3, 1),
                    _ => (3, 2),
                };
                for i in 0..count {
                    summon(
                        Archetype::Bandit,
                        Pos::new(4 + i * 3, 4),
                        level + bonus,
                        None,
                    );
                }
            }
            1 => {
                let shade = summon(Archetype::Skeleton, Pos::new(7, 4), level + 2, Some("Schismatic Shade"));
                self.npcs[shade].max_hp = 42;
                self.npcs[shade].hp = 42;
                self.npcs[shade].defense = 6;
                self.npcs[shade].attack = 8;
            }
            2 => {
                let alpha = summon(Archetype::Bear, Pos::new(7, 5), 5, Some("Sick Alpha"));
                self.npcs[alpha].max_hp = 60;
                self.npcs[alpha].hp = 60;
                self.npcs[alpha].attack = 6;
            }
            3 => {
                for i in 0..4 {
                    summon(
                        Archetype::Bandit,
                        Pos::new(4 + i * 2, 3 + i * 2),
                        level + 1,
                        None,
                    );
                }
            }
            4 => {
                for i in 0..6 {
                    summon(
                        Archetype::Skeleton,
                        Pos::new(3 + i * 2, 3 + (i % 2) * 2),
                        level.max(9),
                        None,
                    );
                }
            }
            5 => {
                for i in 0..2 {
                    summon(Archetype::Wolf, Pos::new(4 + i * 6, 8), 3, None);
                }
                let guard = summon(
                    Archetype::Traveller,
                    Pos::new(7, 3),
                    4,
                    Some("Oathroad Cart-Guard"),
                );
                let _ = summon; // release the closure's borrow before direct edits
                self.npcs[guard].max_hp = 40;
                self.npcs[guard].hp = 40;
                self.trials[5].escort = Some(guard);
            }
            _ => {}
        }
    }

    fn complete_trial(&mut self, index: usize) {
        self.trials[index].stage = TrialStage::Done;
        self.player.history.quests += 1;
        self.gain_xp(120);
        self.log(match index {
            0 => "Three waves break against the brace and stop. The manual's last pages are yours — the gate holds. (Trial complete, +120 XP.)",
            1 => "The shade's verse folds back into the page, corrected. You read it wrong exactly on purpose. (Trial complete, +120 XP.)",
            2 => "The alpha's breath evens out under your palm. The old truce reads itself clean again. (Trial complete, +120 XP.)",
            3 => "You cross the sill with the tide at your heels. The Charter's mark burns blue in the dark behind you. (Trial complete, +120 XP.)",
            4 => "Six graves kneel back down. One answers you before it goes still: 'VAEL.' The watch has its name. (Trial complete, +120 XP.)",
            5 => "The cart passes out of the Rest untouched, and the oathroad carries your name forward a mile-marker. (Trial complete, +120 XP.)",
            _ => "The trial is complete.",
        });
    }

    fn rat_work(&mut self, npc: usize) {
        match self.quests[0].stage {
            QuestStage::Available => {
                self.quests[0].stage = QuestStage::Active;
                self.npcs[npc]
                    .memory
                    .remember(self.turn, "accepted_cellar_job", 0.1);
                self.log("Mara asks you to clear the cellar at (8,18). Return to a Millbrook merchant for your reward.");
            }
            QuestStage::Ready => {
                self.claim_quest(0, 45, 45, 0, 12);
                self.player.keys[0] = true;
                self.log("Cellar cleared! Received 45 gold, 45 XP, +12 Millbrook reputation and the permanent Burrow seal.");
                // E5 (§6): the cellar rot was fed from below — the barrow wakes.
                self.spawn_gnawthane();
                self.begin_sigil_hunt();
            }
            QuestStage::Complete => self.log(
                "Millbrook thanks you. The Burrow lies at overworld (52,78); your seal opens it.",
            ),
            _ => self.log(self.quests[0].description.clone()),
        }
    }

    fn caravan_work(&mut self, _npc: usize) {
        match self.quests[1].stage {
            QuestStage::Available => {
                self.quests[1].stage = QuestStage::Active;
                self.log("Edda's caravan vanished at the north ford (102,65). Defeat, spare or drive off its three wolves, then press E at the wreck.");
            }
            QuestStage::Ready => {
                if !self.take_cargo(Item::CaravanGoods) {
                    self.log("Bring the caravan goods to claim this reward.");
                    return;
                }
                self.claim_quest(1, 85, 85, 1, 15);
                self.player.keys[1] = true;
                // E5 (§6): the caravan line's climax — the crews show their hand.
                self.spawn_tollmaster();
                self.log("Goods returned: 85 gold, 85 XP, +15 Highgate reputation and the Crimson Hollow seal.");
                self.quests[1].description = "Complete: goods returned honestly to Highgate. Received 85 gold, 85 XP and the Crimson Hollow seal.".into();
            }
            QuestStage::Complete => {
                self.log("The caravan matter is settled. Crimson Hollow waits at (158,82).")
            }
            _ => self.log(self.quests[1].description.clone()),
        }
    }

    fn recover_caravan(&mut self) {
        if !matches!(self.quests[1].stage, QuestStage::Active | QuestStage::Ready) {
            self.log("An abandoned caravan. Ask Highgate's innkeeper or merchants about its missing cargo.");
            return;
        }
        if self.caravan_recovered {
            self.log("You already recovered this caravan's cargo. Return to Highgate or find Saltmarsh's fence.");
            return;
        }
        if self
            .npcs
            .iter()
            .any(|n| n.pack == 900 && !caravan_secured(n))
        {
            self.log("Wolves still circle the wreck. Defeat them, spare them, or drive them away from the cargo.");
            return;
        }
        if self.player.inventory.len() >= CAPACITY {
            self.log("Make one free pack slot for the caravan goods.");
            return;
        }
        self.player.inventory.push(Item::CaravanGoods);
        self.caravan_recovered = true;
        self.log("Recovered the caravan goods. Return them to Highgate, or fence them with Silas/Nessa in Saltmarsh for more gold and lost reputation.");
        self.update_quests();
    }

    fn fence_caravan(&mut self, npc: usize) {
        if self.quests[1].stage != QuestStage::Ready || !self.take_cargo(Item::CaravanGoods) {
            self.log("Recover the Highgate caravan goods at the northern ford before offering them to the fence.");
            return;
        }
        self.claim_quest(1, 125, 65, 2, 5);
        self.player.keys[1] = true;
        self.player.reputation[1] = (self.player.reputation[1] - 20).max(0);
        self.player.history.thefts += 1;
        // E5 (§6): honestly or not, the caravan line ends at the toll camp.
        self.spawn_tollmaster();
        self.npcs[npc]
            .memory
            .remember(self.turn, "fenced_caravan_goods", 0.2);
        self.log("Fenced the goods: 125 gold, 65 XP and Crimson Hollow's seal; Highgate reputation -20. Your greed is remembered.");
        self.quests[1].description = "Complete: caravan goods fenced in Saltmarsh for 125 gold, 65 XP and the Crimson Hollow seal. Highgate reputation lost.".into();
    }

    fn smuggling_work(&mut self) {
        match self.quests[2].stage {
            QuestStage::Available => {
                self.quests[2].stage = QuestStage::Active;
                self.quests[2].progress = 0;
                self.log("Nessa wants the sealed parcel from the dockside chest (32,31). Press E beside it, then choose Nessa or the Saltmarsh watch.");
            }
            QuestStage::Complete => self.log("The dockside affair is finished. Your Underkeep seal opens the northern stronghold (145,12)."),
            _ => self.log(self.quests[2].description.clone()),
        }
    }

    fn collect_parcel(&mut self) {
        if self.quests[2].stage != QuestStage::Active || self.quests[2].progress != 0 {
            self.log("The dockside chest belongs to Nessa. Speak to her at (30,30) for work.");
            return;
        }
        if self.player.inventory.len() >= CAPACITY {
            self.log("Make one free pack slot for the sealed parcel.");
            return;
        }
        self.player.inventory.push(Item::Contraband);
        self.quests[2].progress = 1;
        self.quests[2].stage = QuestStage::Ready;
        self.log("Collected the sealed parcel. Return to Nessa for gold and thief protection, or give it to Saltmarsh's watch for reputation.");
        self.update_quests();
    }

    fn finish_smuggling(&mut self, npc: usize, join: bool) {
        if npc_city(self, npc) != 2 {
            self.log("Take the evidence to Saltmarsh's watch, not another city's guards.");
            return;
        }
        if self.quests[2].stage != QuestStage::Ready || !self.take_cargo(Item::Contraband) {
            self.log("First accept Nessa's job and collect the parcel from (32,31).");
            return;
        }
        self.smuggling_choice = Some(join);
        // E3: the smuggling line's last ripple stirs the landing beneath the docks.
        self.spawn_tidemother();
        self.quests[2].progress = 2;
        self.claim_quest(
            2,
            if join { 150 } else { 80 },
            110,
            2,
            if join { -10 } else { 25 },
        );
        self.player.keys[2] = true;
        self.npcs[npc].memory.remember(
            self.turn,
            if join {
                "joined_smugglers"
            } else {
                "reported_smugglers"
            },
            0.35,
        );
        if join {
            self.player.history.thefts += 1;
            self.log("Joined the smugglers: 150 gold, 110 XP, Underkeep seal and Saltmarsh thief protection; reputation -10.");
        } else {
            self.log("Reported the smugglers: 80 gold, 110 XP, +25 Saltmarsh reputation and the Underkeep seal.");
        }
        self.quests[2].description = if join {
            "Complete: joined the smugglers. Earned 150 gold, 110 XP, the Underkeep seal and protection from Saltmarsh thieves."
        } else { "Complete: reported the parcel to the watch. Earned 80 gold, 110 XP, +25 reputation and the Underkeep seal." }.into();
    }

    fn take_cargo(&mut self, item: Item) -> bool {
        if let Some(index) = self.player.inventory.iter().position(|i| *i == item) {
            self.player.inventory.remove(index);
            true
        } else {
            false
        }
    }

    fn claim_quest(
        &mut self,
        quest: usize,
        gold: u32,
        xp: u32,
        city: usize,
        reputation: i32,
    ) -> bool {
        if self.quests[quest].stage != QuestStage::Ready {
            return false;
        }
        self.quests[quest].stage = QuestStage::Complete;
        self.player.gold += gold;
        self.player.history.quests += 1;
        self.player.reputation[city] = (self.player.reputation[city] + reputation).clamp(0, 100);
        self.gain_xp(xp);
        true
    }

    fn begin_sigil_hunt(&mut self) {
        if self.quests[3].stage == QuestStage::Available {
            self.quests[3].stage = QuestStage::Active;
            self.log("Sigil hunt begun: defeat the Burrow's Chief, Crimson Hollow's Matriarch and the Underkeep's Lich. City quests or merchants provide their seals.");
        } else {
            self.log(self.quests[3].description.clone());
        }
    }

    pub fn update_quests(&mut self) {
        let rats = self
            .npcs
            .iter()
            .filter(|n| n.map == 4 && n.archetype == Archetype::Rat)
            .count() as u32;
        self.quests[0].goal = rats.max(1);
        self.quests[0].progress = self.rats_killed.min(self.quests[0].goal);
        if self.quests[0].stage == QuestStage::Active
            && self.quests[0].progress >= self.quests[0].goal
        {
            self.quests[0].stage = QuestStage::Ready;
            self.log("The cellar is clear. Report to a Millbrook merchant for your reward and Burrow seal.");
        }
        self.quests[0].description = match self.quests[0].stage {
            QuestStage::Active => format!("Cellar rats defeated: {}/{}. Enter at Millbrook (8,18). Report to Mara for 45 gold, 45 XP, +12 reputation and the Burrow seal.", self.quests[0].progress, self.quests[0].goal),
            QuestStage::Ready => "Return to Mara or a Millbrook merchant: claim 45 gold, 45 XP, +12 reputation and the Burrow seal.".into(),
            QuestStage::Complete => "Complete: cellar cleared; received 45 gold, 45 XP and Burrow seal. The Burrow is at overworld (52,78).".into(),
            _ => self.quests[0].description.clone(),
        };
        if self.quests[1].stage == QuestStage::Active {
            self.quests[1].progress = self
                .npcs
                .iter()
                .filter(|n| n.pack == 900 && caravan_secured(n))
                .count() as u32;
            if self.caravan_recovered && self.player.inventory.contains(&Item::CaravanGoods) {
                self.quests[1].stage = QuestStage::Ready;
            }
            self.quests[1].description = format!("North ford (102,65): wolves overcome {}/3. Defeat, spare or drive them off, then E at the wreck (one pack slot). Return: 85 gold/85 XP/+15 rep; fence: 125 gold/65 XP/-20 Highgate rep. Both earn Hollow seal.", self.quests[1].progress);
        }
        if self.quests[1].stage == QuestStage::Ready {
            self.quests[1].description = "Cargo recovered. Deliver to Edda/Highgate merchant for 85 gold, 85 XP and +15 rep, OR Silas/Nessa in Saltmarsh for 125 gold, 65 XP and -20 Highgate rep. Both grant Crimson Hollow seal.".into();
        }
        if self.quests[2].stage == QuestStage::Active {
            self.quests[2].description = "Stage 1/2: collect Nessa's parcel at Saltmarsh docks (32,31), E beside the chest; one pack slot required. Then choose smugglers or watch.".into();
        } else if self.quests[2].stage == QuestStage::Ready {
            self.quests[2].description = "Stage 2/2: parcel collected. Give to Nessa (150 gold, thief protection, -10 rep) OR Saltmarsh watch (80 gold, +25 rep). Both award 110 XP and Underkeep seal.".into();
        }
        let sigils = self.player.sigils.iter().filter(|v| **v).count() as u32;
        self.quests[3].progress = sigils;
        if sigils > 0 && self.quests[3].stage == QuestStage::Available {
            self.quests[3].stage = QuestStage::Active;
        }
        if sigils == 3 && self.quests[3].stage != QuestStage::Complete {
            self.quests[3].stage = QuestStage::Ready;
            self.claim_quest(3, 150, 180, self.player.last_city, 10);
            self.quests[4].stage = QuestStage::Active;
            self.log("All three Sigils united: 150 gold, 180 XP, +10 reputation. The Final Trial at (72,44) is open.");
        }
        self.quests[3].description = if sigils == 3 {
            "Complete: all three Sigils recovered. Earned 150 gold and 180 XP. Enter the Final Trial at overworld (72,44).".into()
        } else {
            format!("Sigils {sigils}/3: Chief/Burrow (52,78) [{}], Matriarch/Hollow (158,82) [{}], Lich/Underkeep (145,12) [{}]. City quests or vendors provide seals. All three award 150 gold, 180 XP and Trial entry.",
                if self.player.sigils[0] { "done" } else { "missing" }, if self.player.sigils[1] { "done" } else { "missing" }, if self.player.sigils[2] { "done" } else { "missing" })
        };
        if self.quests[4].stage == QuestStage::Active {
            self.quests[4].description = "Final Trial unlocked: arena at (72,44). Prepare masterwork equipment and potions, then defeat the Adjudicator. It judges your mercy, greed and courage.".into();
            if self
                .npcs
                .iter()
                .any(|n| n.archetype == Archetype::Adjudicator && !n.alive())
            {
                self.quests[4].stage = QuestStage::Ready;
                self.quests[4].progress = 1;
                self.claim_quest(4, 250, 250, self.player.last_city, 20);
                self.quests[4].description = "Complete: the Adjudicator is defeated. You survived the Final Trial and finished the campaign. Awarded 250 gold and 250 XP.".into();
                self.won = true;
                self.modal = Modal::Victory;
                self.log("The Adjudicator falls. Your deeds stand judged: the realm is yours to remember.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_rewards_can_only_be_claimed_once() {
        let mut game = Game::new(17);
        game.quests[0].stage = QuestStage::Ready;
        let gold = game.player.gold;
        assert!(game.claim_quest(0, 45, 45, 0, 12));
        let xp = game.player.xp;
        assert!(!game.claim_quest(0, 45, 45, 0, 12));
        assert_eq!(game.player.gold, gold + 45);
        assert_eq!(game.player.xp, xp);
        assert_eq!(game.player.history.quests, 1);
    }

    #[test]
    fn full_inventory_rejects_purchase_and_equipped_gear_is_not_sold() {
        let mut game = Game::new(19);
        let vendor = game
            .npcs
            .iter()
            .find(|n| {
                n.map == 1 && n.archetype == Archetype::Vendor && !n.name.contains("Innkeeper")
            })
            .unwrap()
            .id;
        game.player.pos = game.npcs[vendor].pos;
        game.modal = Modal::Trade(vendor);
        game.player.gold = 500;
        game.player.inventory = vec![Item::Potion; CAPACITY];
        game.buy(0);
        assert_eq!(game.player.gold, 500);
        assert_eq!(game.player.inventory.len(), CAPACITY);
        game.player.inventory[0] = Item::Weapon(2);
        assert!(game.use_item(0));
        assert_eq!(game.player.weapon, 2);
        assert_eq!(game.player.inventory[0], Item::Weapon(0));
        game.sell(0);
        assert_eq!(game.player.weapon, 2);
        assert_eq!(game.player.gold, 502);
        assert_eq!(game.player.inventory.len(), CAPACITY - 1);
    }

    #[test]
    fn full_pack_never_blocks_a_keyring_seal_purchase() {
        let mut game = Game::new(19);
        let vendor = game
            .npcs
            .iter()
            .find(|n| {
                n.map == 1 && n.archetype == Archetype::Vendor && !n.name.contains("Innkeeper")
            })
            .unwrap()
            .id;
        game.player.pos = game.npcs[vendor].pos;
        game.modal = Modal::Trade(vendor);
        game.player.gold = 500;
        game.player.inventory = vec![Item::Potion; CAPACITY];
        let seal = stock(0)
            .iter()
            .position(|(item, _)| matches!(item, Item::Key(0)))
            .unwrap();
        game.buy(seal);
        assert!(
            game.player.keys[0],
            "seals belong on the keyring, not in a slot"
        );
        assert_eq!(game.player.inventory.len(), CAPACITY);
    }

    #[test]
    fn final_trial_requires_each_distinct_sigil_and_reward_is_idempotent() {
        let mut game = Game::new(23);
        for missing in 0..3 {
            game.player.sigils = [true; 3];
            game.player.sigils[missing] = false;
            assert!(!game.portal_unlocked(Some(3)));
        }
        game.player.sigils = [true; 3];
        assert!(game.portal_unlocked(Some(3)));
        game.update_quests();
        let gold = game.player.gold;
        let quests = game.player.history.quests;
        game.update_quests();
        assert_eq!(game.player.gold, gold);
        assert_eq!(game.player.history.quests, quests);
        assert_eq!(game.quests[4].stage, QuestStage::Active);
    }
    #[test]
    fn forge_trades_two_matching_pieces_and_coin_for_the_next_tier() {
        let mut game = Game::new(19);
        game.player.gold = 100;
        game.player.inventory = vec![Item::Weapon(0), Item::Weapon(0), Item::Armour(0)];
        let options = game.forge_options();
        assert_eq!(options.len(), 1, "only the sword pair qualifies");
        game.forge(0);
        assert!(game.player.inventory.contains(&Item::Weapon(1)));
        assert!(!game.player.inventory.contains(&Item::Weapon(0)));
        assert_eq!(game.player.gold, 75);
        game.player.gold = 10;
        assert_eq!(game.forge_options().len(), 0, "a single piece cannot forge");
    }

    #[test]
    fn companion_hires_follows_and_dismisses() {
        let mut game = Game::new(42);
        let blade = game
            .npcs
            .iter()
            .find(|n| n.archetype == Archetype::Companion && n.map == 1)
            .unwrap()
            .id;
        game.player.gold = 100;
        game.toggle_companion(blade);
        assert_eq!(game.companion, Some(blade));
        assert_eq!(game.player.gold, 50);
        let portal = game.maps[1]
            .portals
            .iter()
            .find(|p| p.destination == 0)
            .unwrap()
            .clone();
        game.player.pos = portal.pos.offset(1, 0);
        game.enter_portal(portal);
        assert_eq!(game.npcs[blade].map, 0, "the blade follows through gates");
        assert!(game.npcs[blade].pos.distance(game.player.pos) <= 1);
        game.toggle_companion(blade);
        assert_eq!(game.companion, None);
    }

    #[test]
    fn fensworn_bond_sworn_partner_is_cheap_stays_sworn_and_rises_whole() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Fensworn, Build::Male, Boon::None);
        let blade = game
            .npcs
            .iter()
            .find(|n| n.archetype == Archetype::Companion && n.map == 1)
            .unwrap()
            .id;
        game.player.gold = 100;
        game.toggle_companion(blade);
        assert_eq!(game.player.gold, 75, "a sworn partner costs 25 (§4.2)");
        let sworn_max = game.npcs[blade].max_hp;
        assert_eq!(game.npcs[blade].hp, sworn_max);
        game.toggle_companion(blade);
        game.toggle_companion(blade);
        assert_eq!(game.player.gold, 50);
        assert_eq!(
            game.npcs[blade].max_hp, sworn_max,
            "the +25% sworn bonus must not stack on re-hire"
        );
        game.npcs[blade].hp = 0;
        game.companion_follow();
        assert_eq!(game.npcs[blade].hp, sworn_max, "the partner rises whole");
    }

    #[test]
    fn cull_bounty_tracks_kills_and_pays_the_posted_reward() {
        let mut game = Game::new(42);
        game.quests.push(Quest {
            title: "Bounty: cull 2 bandits".into(),
            stage: QuestStage::Active,
            progress: 0,
            goal: 2,
            description: String::new(),
        });
        game.bounties.push(Bounty {
            kind: BountyKind::Cull(Archetype::Bandit),
            origin: 0,
            gold: 60,
            xp: 40,
            reputation: 5,
        });
        for _ in 0..2 {
            let id = game.npcs.len();
            let mut bandit =
                crate::world::make_npc(id, Archetype::Bandit, 0, Pos::new(3, 3), 2, 1, 42);
            bandit.intent = Intent::Attack;
            game.npcs.push(bandit);
            game.kill_npc(id);
        }
        game.bounty_progress(Archetype::Wolf); // wrong quarry never counts
        assert_eq!(game.quests[5].stage, QuestStage::Ready);
        let captain = game
            .npcs
            .iter()
            .find(|n| n.name.contains("Guard Captain") && n.map == 1)
            .unwrap()
            .id;
        game.player.pos = game.npcs[captain].pos;
        game.player.map = 1;
        let gold = game.player.gold;
        game.bounty_office(captain);
        assert_eq!(game.quests[5].stage, QuestStage::Complete);
        assert_eq!(game.player.gold, gold + 60);
    }

    #[test]
    fn parcel_delivery_settles_at_the_destination_captain() {
        let mut game = Game::new(42);
        game.quests.push(Quest {
            title: "Bounty: deliver a parcel to Highgate".into(),
            stage: QuestStage::Active,
            progress: 0,
            goal: 1,
            description: String::new(),
        });
        game.bounties.push(Bounty {
            kind: BountyKind::Deliver(1),
            origin: 0,
            gold: 50,
            xp: 30,
            reputation: 5,
        });
        game.player.inventory.push(Item::Delivery(1));
        let captain = game
            .npcs
            .iter()
            .find(|n| n.name.contains("Guard Captain") && n.map == 2)
            .unwrap()
            .id;
        game.player.pos = game.npcs[captain].pos;
        game.player.map = 2;
        let gold = game.player.gold;
        game.bounty_office(captain);
        assert_eq!(game.quests[5].stage, QuestStage::Complete);
        assert!(!game.player.inventory.contains(&Item::Delivery(1)));
        assert_eq!(game.player.gold, gold + 50);
    }

    #[test]
    fn study_grant_uses_the_oracles_judgment_and_sets_mana() {
        use crate::laya_client::heuristic;
        let mut game = Game::new(42);
        let oracle = game
            .npcs
            .iter()
            .find(|n| n.archetype == Archetype::Oracle && n.map == 1)
            .unwrap()
            .id;
        game.player.pos = game.npcs[oracle].pos;
        game.modal = Modal::Talk(oracle);
        game.social_event(oracle, SocialEvent::Study, 2);
        let request = game.outbox.pop().unwrap();
        let mut result = heuristic(&request);
        result.answers.get_mut("share_rumour").unwrap().value = 0.9;
        game.apply_decision(&result);
        assert_eq!(game.player.spells, vec![Spell::Spark]);
        assert_eq!(game.player.max_mana, 4);
        assert_eq!(game.player.mana, 4);
        assert_eq!(game.player.gold, 15, "tuition is 25 gold from 40");
    }

    #[test]
    fn caravan_can_be_recovered_after_driving_off_or_sparing_the_pack() {
        for mercy in [false, true] {
            let mut game = Game::new(42);
            game.modal = Modal::None;
            game.player.map = 0;
            game.player.pos = CARAVAN;
            game.quests[1].stage = QuestStage::Active;
            for n in game.npcs.iter_mut().filter(|n| n.pack == 900) {
                n.intent = Intent::Flee;
                n.pos = CARAVAN.offset(0, 5);
            }
            game.interact();
            assert!(
                !game.caravan_recovered,
                "nearby fleeing wolves still threaten the cargo"
            );
            for n in game.npcs.iter_mut().filter(|n| n.pack == 900) {
                if mercy {
                    n.memory.remember(0, "player_spared_me", 0.8);
                } else {
                    n.pos = CARAVAN.offset(0, 9);
                }
            }
            game.interact();
            assert!(
                game.caravan_recovered,
                "mercy or routed wolves must not softlock the quest"
            );
            assert_eq!(game.quests[1].stage, QuestStage::Ready);
        }
    }
}
