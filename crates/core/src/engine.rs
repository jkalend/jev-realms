use crate::model::*;
use serde_json::json;

const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (1, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
];

fn score(npc: &Npc, key: &str) -> f32 {
    npc.answers
        .get(key)
        .map_or(0.0, |a| a.value.clamp(0.0, 1.0))
}

fn choice<'a>(npc: &'a Npc, key: &str) -> &'a str {
    npc.answers
        .get(key)
        .and_then(|a| a.choice.as_deref())
        .unwrap_or("")
}

fn spared(npc: &Npc) -> bool {
    npc.memory
        .events
        .iter()
        .any(|e| e.kind == "player_spared_me")
}

pub fn hostile(npc: &Npc) -> bool {
    !spared(npc) && (npc.archetype.hostile() || npc.intent == Intent::Attack)
}

impl Game {
    pub fn action(&mut self, action: Action) {
        if self.player.hp <= 0
            || matches!(
                self.modal,
                Modal::Title | Modal::Pause | Modal::Help | Modal::Death | Modal::Victory
            )
        {
            return;
        }
        match action {
            Action::Use(index) => {
                // Item use is resolved at the player's initiative, just like an attack.
                if index >= self.player.inventory.len() {
                    return;
                }
                if self.combat.is_some() {
                    if !matches!(
                        self.player.inventory[index],
                        Item::Potion
                            | Item::GreaterPotion
                            | Item::Ration
                            | Item::TravelerRation
                            | Item::Torch
                            | Item::AntiToxin
                            | Item::ManaTonic
                            | Item::Weapon(_)
                            | Item::Armour(_)
                            | Item::BossRelic(_)
                    ) {
                        return;
                    }
                    self.combat_action(action);
                } else {
                    self.use_item(index);
                }
            }
            Action::Cast(index) => {
                if index >= self.player.spells.len() {
                    return;
                }
                if self.combat.is_some() {
                    self.combat_action(action);
                } else {
                    self.cast_exploration(index);
                }
            }
            Action::Forge(index) => self.forge(index),
            Action::Buy(i) => self.buy(i),
            Action::Sell(i) => self.sell(i),
            Action::Haggle => self.haggle(),
            Action::Talk(i) => self.talk_choice(i),
            Action::Travel(i) => {
                if self.combat.is_none() {
                    self.travel(i);
                }
            }
            Action::Oracle => {
                if self.combat.is_none() {
                    self.oracle();
                }
            }
            Action::Interact => {
                if self.combat.is_some() {
                    self.log("Finish the fight, flee [X], or spare a retreating enemy [C].");
                } else {
                    self.interact();
                }
            }
            _ => {
                if !matches!(self.modal, Modal::None) {
                    return;
                }
                if self.combat.is_some() {
                    self.combat_action(action);
                } else {
                    self.exploration_action(action);
                }
            }
        }
    }

    fn exploration_action(&mut self, action: Action) {
        match action {
            Action::Move(dx, dy) => {
                if self.elapsed_ms < self.move_ready_ms
                    || dx.abs() > 1
                    || dy.abs() > 1
                    || (dx == 0 && dy == 0)
                {
                    return;
                }
                let next = self.player.pos.offset(dx, dy);
                if !self.map().can_step(self.player.pos, next) {
                    return;
                }
                if let Some(id) = self.npc_at(next) {
                    if hostile(&self.npcs[id]) {
                        self.npcs[id].intent = Intent::Attack;
                        self.refresh_combat();
                        self.combat_action(action);
                    } else {
                        self.log(format!(
                            "{} is here. Press E to interact.",
                            self.npcs[id].name
                        ));
                    }
                    return;
                }
                self.maps[self.player.map].set_door_open(next, true);
                self.player.pos = next;
                let mut cost = self.terrain_cost(self.map().tile(next));
                if self.player.class == Class::Waysworn
                    && self.player.class_state.trail_charges > 0
                {
                    // Open Road: the kill's tempo carries this move — no terrain toll, stamina paid back.
                    cost = 100;
                    self.player.class_state.trail_charges -= 1;
                    let refund = 1 + if self.player.has_talent(0, 2) {
                        self.player.branch_synergy(0)
                    } else {
                        0
                    };
                    self.player.stamina =
                        (self.player.stamina + i32::from(refund)).min(self.player.max_stamina);
                    self.log("The road answers your pace: no toll taken.");
                }
                self.move_ready_ms = self.elapsed_ms + 85 * u64::from(cost) / 100;
                // Steady Bit keeps the outrider fed; Swampwise feeds the fen-footed.
                // Gnawbone Crown (§6): +1 stamina to every dungeon move.
                let mut gain = 1
                    + i32::from(
                        self.in_dungeon() && self.player.relic == Some(BossRelic::GnawboneCrown),
                    );
                if self.player.class == Class::Waysworn && self.player.has_talent(0, 0) {
                    gain = 2;
                }
                if self.player.class == Class::Fensworn
                    && self.player.has_talent(1, 2)
                    && !matches!(self.map().kind, MapKind::City(_))
                {
                    gain += 1;
                }
                self.player.stamina = (self.player.stamina + gain).min(self.player.max_stamina);
                self.caltrop_check();
                self.scrounge_check();
                self.break_channel();
                self.reveal();
                self.refresh_combat();
            }
            Action::Attack => {
                if let Some(id) = self.adjacent_enemy(false) {
                    self.npcs[id].intent = Intent::Attack;
                    self.refresh_combat();
                    self.combat_action(Action::Attack);
                } else {
                    self.log("No enemy within reach. Move adjacent to attack.");
                }
            }
            Action::Mercy => {
                self.spare_enemy();
            }
            Action::Rest => {
                if self.city().is_some() {
                    self.rest_at_inn();
                    return;
                }
                if self.npcs.iter().any(|n| {
                    n.alive()
                        && n.map == self.player.map
                        && hostile(n)
                        && n.pos.distance(self.player.pos) <= 7
                }) {
                    self.log("Too dangerous to rest: an enemy is nearby.");
                    return;
                }
                if let Some(index) = self
                    .player
                    .inventory
                    .iter()
                    .position(|i| *i == Item::Ration)
                {
                    self.player.inventory.remove(index);
                    // Quartering and Forager make the roadside kinder (§5 Tollwright/Farwanderer).
                    let waysworn = self.player.class == Class::Waysworn;
                    let rest_bonus = if waysworn && self.player.has_talent(1, 1) {
                        2
                    } else {
                        0
                    } + if waysworn && self.player.has_talent(2, 1) {
                        4 + if self.player.has_talent(2, 2) {
                            2 * i32::from(self.player.branch_synergy(2))
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    self.player.hp = (self.player.hp + 10 + rest_bonus).min(self.player.max_hp);
                    self.player.stamina = self.player.max_stamina;
                    self.hour_ticks += 120;
                    self.log(format!(
                        "You rest over a ration: +{} HP and full stamina. The wilderness keeps moving.",
                        10 + rest_bonus
                    ));
                    for _ in 0..8 {
                        self.tick_world();
                        if self.combat.is_some() {
                            break;
                        }
                    }
                } else {
                    self.log("Resting outside an inn requires a trail ration.");
                }
            }
            Action::Wait | Action::Defend => {
                self.player.stamina = (self.player.stamina + 2).min(self.player.max_stamina);
                self.tick_world();
            }
            Action::Flee => self.log("You are not in combat."),
            _ => {}
        }
    }

    pub fn tick_world(&mut self) {
        for effect in &mut self.effects {
            effect.ttl = effect.ttl.saturating_sub(1);
        }
        self.effects.retain(|e| e.ttl > 0);
        if self.combat.is_some() || !matches!(self.modal, Modal::None) || self.player.hp <= 0 {
            return;
        }
        self.tick += 1;
        self.hour_ticks += 1;
        if self.player.mana < self.player.max_mana && self.tick.is_multiple_of(16) {
            self.player.mana += 1;
        }
        // E5 (§6): the fen wisp keeps night hours; upkeep runs on the clock's edge.
        if self.hour_ticks.is_multiple_of(480) {
            self.mirelight_upkeep();
        }
        // E7 (§6.5): the orders' trials run while time flows.
        self.trial_upkeep();
        let map = self.player.map;
        let player = self.player.pos;
        // Far-away NPCs have no pathfinding work; their schedule resumes on approach.
        for id in 0..self.npcs.len() {
            if !self.npcs[id].alive() {
                continue;
            }
            if self.npcs[id].map != map || self.npcs[id].pos.distance(player) > 16 {
                self.npcs[id].perceived = false;
                continue;
            }
            let distance = self.npcs[id].pos.distance(player);
            let visible = distance <= 7 && self.line_clear(self.npcs[id].pos, player);
            if visible && !self.npcs[id].perceived {
                self.request_decision(id, if self.npcs[id].archetype.boss() { 3 } else { 1 });
                self.npcs[id].perceived = self.npcs[id].decision_pending;
            } else if !visible {
                self.npcs[id].perceived = false;
            }
            let band = self.npcs[id].hp * 4 / self.npcs[id].max_hp.max(1);
            if band != self.npcs[id].last_hp_band && visible {
                self.request_decision(id, 1);
                if self.npcs[id].decision_pending {
                    self.npcs[id].last_hp_band = band;
                }
            }
            if self.npcs[id].archetype == Archetype::Thief && self.tick % 480 == id as u64 % 480 {
                self.request_decision(id, 1);
            }
            self.world_npc_turn(id);
            self.refresh_combat();
            if self.combat.is_some() {
                break;
            }
        }
        self.reveal();
    }

    fn world_npc_turn(&mut self, id: usize) {
        if self.npcs[id].cooldown > 0 {
            self.npcs[id].cooldown -= 1;
            return;
        }
        let npc = &self.npcs[id];
        let kind = npc.archetype;
        let pos = npc.pos;
        let home = npc.home;
        let distance = pos.distance(self.player.pos);
        if npc.intent == Intent::Flee {
            if npc.tactic == "parting_javelin" {
                self.npcs[id].tactic = "retreat".into();
                if distance <= 4 && self.line_clear(pos, self.player.pos) {
                    self.hurt_player(
                        (self.npcs[id].attack / 2 - self.defense_power()).max(1),
                        "A fleeing bandit's parting javelin",
                    );
                }
            }
            self.step_away(id, self.player.pos);
            return;
        }
        // E7 (§6.5): the escorted cart-guard walks for the road, not for home.
        if self.trials[5].stage == TrialStage::Active && Some(id) == self.trials[5].escort {
            if self.tick.is_multiple_of(3) {
                self.step_toward(id, Pos::new(7, 10));
            }
            return;
        }
        // E5 (§6): holes do not wander; crowned rats and truce wolves hold.
        if kind == Archetype::BroodHole
            || (kind == Archetype::Rat && self.player.relic == Some(BossRelic::GnawboneCrown))
            || (kind == Archetype::Wolf && self.wolf_truce_holds(npc))
        {
            return;
        }
        if kind == Archetype::FalseGlow {
            // The tell lights drift only when the wisp's lights strike together.
            let leader = self
                .npcs
                .iter()
                .find(|n| n.archetype == Archetype::Mirelight && n.alive() && n.map == self.player.map)
                .map(|n| n.id);
            if let Some(leader) = leader {
                if score(&self.npcs[leader], "strike_or_subside") > 0.5
                    && self.npcs[id].intent == Intent::Attack
                    && distance <= 8
                {
                    self.step_toward(id, self.player.pos);
                }
            }
            return;
        }
        if hostile(npc) && npc.intent == Intent::Attack && distance <= 8 {
            self.step_toward(id, self.player.pos);
            return;
        }
        match kind {
            Archetype::Vendor | Archetype::Oracle | Archetype::Smuggler | Archetype::Alchemist => {
                if pos != home {
                    self.step_toward(id, home);
                }
            }
            Archetype::Thief => {
                let active = self.hour() >= 22 || self.hour() < 4;
                if active
                    && !(self.player.map == 3 && self.smuggling_choice == Some(true))
                    && score(&self.npcs[id], "steal_from_player") > 0.55
                    && distance <= 6
                {
                    if distance <= 1 && self.line_clear(pos, self.player.pos) {
                        let amount = self.player.gold.min(3 + self.player.gold / 12);
                        self.player.gold -= amount;
                        self.npcs[id]
                            .memory
                            .remember(self.turn, &format!("stole:{amount}"), -0.2);
                        self.npcs[id].intent = Intent::Flee;
                        self.npcs[id].cooldown = 3;
                        self.log(format!(
                            "{} steals {amount} gold and bolts!",
                            self.npcs[id].name
                        ));
                    } else {
                        self.step_toward(id, self.player.pos);
                    }
                } else if pos != home {
                    self.step_toward(id, home);
                }
            }
            Archetype::Guard if npc.intent == Intent::Help => {
                let target = self
                    .npcs
                    .iter()
                    .filter(|n| {
                        n.alive()
                            && n.map == self.player.map
                            && n.archetype.hostile()
                            && n.pos.distance(pos) <= 8
                    })
                    .min_by_key(|n| n.pos.distance(pos))
                    .map(|n| n.id);
                if let Some(target) = target {
                    if self.can_melee(pos, self.npcs[target].pos) {
                        let damage = (self.npcs[id].attack - self.npcs[target].defense).max(1);
                        self.hurt_npc(target, damage, false);
                        self.npcs[id].cooldown = 3;
                    } else {
                        self.step_toward(id, self.npcs[target].pos);
                    }
                } else {
                    self.step_toward(id, home);
                }
            }
            Archetype::Companion if self.companion == Some(id) => {
                if distance > 2 {
                    self.step_toward(id, self.player.pos);
                }
            }
            Archetype::Commoner | Archetype::Traveller | Archetype::Guard => {
                if self.player.map == 0
                    && self.map().portals.iter().any(|p| p.pos.distance(home) <= 2)
                {
                    if pos != home {
                        self.step_toward(id, home);
                    }
                    return;
                }
                if self.tick % 4 != id as u64 % 4 {
                    return;
                }
                if self.night() || pos.distance(home) > 5 {
                    self.step_toward(id, home);
                } else {
                    let (dx, dy) = DIRECTIONS[self.random(8) as usize];
                    self.try_npc_step(id, pos.offset(dx, dy));
                }
            }
            _ => {
                if pos.distance(home) > 2 {
                    self.step_toward(id, home);
                } else if self.tick % 8 == id as u64 % 8 {
                    let (dx, dy) = DIRECTIONS[self.random(8) as usize];
                    self.try_npc_step(id, pos.offset(dx, dy));
                }
            }
        }
    }

    pub fn can_melee(&self, from: Pos, to: Pos) -> bool {
        from.distance(to) == 1 && self.map().can_step(from, to)
    }

    fn line_clear(&self, from: Pos, to: Pos) -> bool {
        let mut p = from;
        let dx = (to.x - from.x).abs();
        let dy = -(to.y - from.y).abs();
        let sx = (to.x - from.x).signum();
        let sy = (to.y - from.y).signum();
        let mut error = dx + dy;
        while p != to {
            let twice = 2 * error;
            let mut next = p;
            if twice >= dy {
                error += dy;
                next.x += sx;
            }
            if twice <= dx {
                error += dx;
                next.y += sy;
            }
            if !self.map().can_step(p, next) {
                return false;
            }
            p = next;
        }
        true
    }

    fn try_npc_step(&mut self, id: usize, next: Pos) -> bool {
        if next == self.player.pos
            || !self.map().can_step(self.npcs[id].pos, next)
            || self.npc_at(next).is_some()
        {
            return false;
        }
        self.maps[self.npcs[id].map].set_door_open(next, true);
        self.npcs[id].pos = next;
        true
    }

    fn step_toward(&mut self, id: usize, goal: Pos) {
        let start = self.npcs[id].pos;
        if start == goal {
            return;
        }
        // A bounded local BFS avoids allocations and cannot scan a whole world map.
        let mut cells = [Pos::new(0, 0); 256];
        let mut parents = [0usize; 256];
        cells[0] = start;
        let mut used = 1;
        let mut front = 0;
        let mut best = 0;
        while front < used && used < cells.len() {
            let here = cells[front];
            if here.distance(goal) < cells[best].distance(goal) {
                best = front;
            }
            if here == goal {
                best = front;
                break;
            }
            for (dx, dy) in DIRECTIONS {
                let next = here.offset(dx, dy);
                if used == cells.len() {
                    break;
                }
                if next.distance(start) > 10
                    || !self.map().can_step(here, next)
                    || cells[..used].contains(&next)
                {
                    continue;
                }
                if next != goal && (next == self.player.pos || self.npc_at(next).is_some()) {
                    continue;
                }
                cells[used] = next;
                parents[used] = front;
                if next.distance(goal) < cells[best].distance(goal) {
                    best = used;
                }
                used += 1;
            }
            front += 1;
        }
        if best == 0 {
            return;
        }
        while parents[best] != 0 {
            best = parents[best];
        }
        self.try_npc_step(id, cells[best]);
    }

    fn step_away(&mut self, id: usize, threat: Pos) -> bool {
        let pos = self.npcs[id].pos;
        let mut options = DIRECTIONS.map(|(x, y)| pos.offset(x, y));
        options.sort_by_key(|p| std::cmp::Reverse(p.distance(threat)));
        for next in options {
            if next.distance(threat) >= pos.distance(threat) && self.try_npc_step(id, next) {
                return true;
            }
        }
        false
    }

    pub fn refresh_combat(&mut self) {
        if self.player.hp <= 0 || self.won {
            self.combat = None;
            return;
        }
        let player = self.player.pos;
        let center = self.combat.as_ref().map_or(player, |c| c.center);
        let engaged = self.npcs.iter().any(|n| {
            n.alive()
                && n.map == self.player.map
                && hostile(n)
                && n.intent == Intent::Attack
                && self.can_melee(n.pos, player)
        });
        if self.combat.is_none() && !engaged {
            return;
        }
        let mut participants: Vec<usize> = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && hostile(n)
                    && n.intent != Intent::Flee
                    && n.pos.distance(center)
                        <= if self.player.class == Class::Fensworn && self.player.has_talent(1, 1) {
                            4
                        } else {
                            5
                        }
                    && self.line_clear(n.pos, player)
            })
            .map(|n| n.id)
            .collect();
        participants.sort_by_key(|id| (std::cmp::Reverse(self.npcs[*id].speed), *id));
        if participants.is_empty() || (player.distance(center) > 5 && !engaged) {
            if self.combat.take().is_some() {
                self.player.defending = false;
                self.log("Combat ends. Catch your breath; your next move is free.");
                // E3: settle the avalanche ground back down.
                for (m, p, t) in std::mem::take(&mut self.rubble) {
                    self.maps[m].set(p, t);
                }
                // E7 (§6.5): the unwritten silence ends with the fight that made it.
                self.curate_unwritten = [false; 3];
                if let Some(id) = self.companion {
                    if !self.npcs[id].alive() {
                        // Bond (§4.2): a sworn partner rises whole, not merely bruised.
                        self.npcs[id].hp = if self.player.class == Class::Fensworn {
                            self.npcs[id].max_hp
                        } else {
                            self.npcs[id].max_hp / 2
                        };
                        self.npcs[id].intent = Intent::Idle;
                        self.log(format!("{} rises, bruised but loyal.", self.npcs[id].name));
                    }
                }
            }
            return;
        }
        if let Some(combat) = &mut self.combat {
            combat.participants = participants;
        } else {
            self.log(
                "Combat begins. Time waits for your next action.",
            );
            for id in &participants {
                self.npcs[*id].intent = Intent::Attack;
                self.request_decision(
                    *id,
                    if self.npcs[*id].archetype.boss() {
                        3
                    } else {
                        1
                    },
                );
            }
            // Hollow Caller (Fensworn, Trapper 4): the first beast to engage loses its footing.
            let caller_target = if self.player.has_talent(2, 3) {
                participants.iter().copied().find(|id| {
                    matches!(
                        self.npcs[*id].archetype,
                        Archetype::Wolf | Archetype::Bear | Archetype::Rat
                    )
                })
            } else {
                None
            };
            self.combat = Some(Combat {
                center: player,
                round: 0,
                participants,
            });
            // §5 one-shot talents reload with each engagement.
            self.player.talent_once = 0;
            if let Some(id) = caller_target {
                self.npcs[id].cooldown = self.npcs[id].cooldown.max(4);
                self.log(format!(
                    "The Hollow Caller thumps — {} staggers.",
                    self.npcs[id].name
                ));
            }
        }
    }

    fn adjacent_enemy(&self, fleeing_only: bool) -> Option<usize> {
        self.npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && self.can_melee(self.player.pos, n.pos)
                    && if fleeing_only {
                        n.intent == Intent::Flee && !spared(n)
                    } else {
                        hostile(n)
                    }
            })
            .min_by_key(|n| (n.hp, n.id))
            .map(|n| n.id)
    }

    fn combat_action(&mut self, action: Action) {
        let valid = match action {
            Action::Move(dx, dy) if dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0) => {
                let next = self.player.pos.offset(dx, dy);
                self.map().can_step(self.player.pos, next)
                    && self.npc_at(next).is_none_or(|id| hostile(&self.npcs[id]))
            }
            Action::Attack => self.adjacent_enemy(false).is_some(),
            Action::Mercy => self.adjacent_enemy(true).is_some(),
            Action::Wait | Action::Defend | Action::Flee | Action::OathPledge(_) => true,
            Action::Use(i) => match self.player.inventory.get(i) {
                Some(Item::Potion | Item::GreaterPotion) => self.player.hp < self.player.max_hp,
                Some(Item::Ration | Item::TravelerRation) => {
                    self.player.hp < self.player.max_hp
                        || self.player.stamina < self.player.max_stamina
                }
                Some(Item::Weapon(t)) => *t != self.player.weapon,
                Some(Item::Armour(t)) => *t != self.player.armour,
                Some(Item::BossRelic(relic)) => Some(*relic) != self.player.relic,
                Some(Item::Torch) => true,
                Some(Item::AntiToxin) => {
                    self.player.fester > 0 || self.turn >= self.player.fester_guard_until
                }
                Some(Item::ManaTonic) => self.player.mana < self.player.max_mana,
                _ => false,
            },
            Action::Cast(i) => match self.player.spells.get(i).copied() {
                Some(Spell::Spark) => {
                    (self.player.mana >= self.spell_cost(Spell::Spark)
                        || self.free_cast().is_some())
                        && self.spark_target().is_some()
                }
                Some(Spell::Mend) => {
                    (self.player.mana >= self.spell_cost(Spell::Mend)
                        || self.free_cast().is_some())
                        && self.player.hp < self.player.max_hp
                }
                Some(Spell::Ward) => {
                    self.player.mana >= self.spell_cost(Spell::Ward)
                        || self.free_cast().is_some()
                }
                None => false,
            },
            _ => false,
        };
        if !valid {
            self.log("That action has no valid target; no turn passes.");
            return;
        }
        let Some(combat) = &self.combat else {
            return;
        };
        // Preserve a directional strike's chosen opponent across faster turns.
        // Re-querying the tile afterward can hit an entirely different actor.
        let strike_target = match action {
            Action::Move(dx, dy) => self.npc_at(self.player.pos.offset(dx, dy)),
            _ => None,
        };
        let initiative = combat.participants.clone();
        self.turn += 1;
        if self.recent_actions.len() == 8 {
            self.recent_actions.pop_front();
        }
        self.recent_actions.push_back(
            match action {
                Action::Move(dx, dy) if self.npc_at(self.player.pos.offset(dx, dy)).is_some() => {
                    "attack"
                }
                Action::Move(_, _) => "move",
                Action::Attack => "attack",
                Action::Defend => "defend",
                Action::OathPledge(_) => "defend",
                Action::Flee => "flee",
                Action::Use(_) => "item",
                Action::Mercy => "mercy",
                Action::Cast(index) => match self.player.spells.get(index) {
                    Some(Spell::Spark) => "attack",
                    Some(Spell::Ward) => "defend",
                    _ => "item",
                },
                _ => "wait",
            }
            .to_string(),
        );
        if let Some(combat) = &mut self.combat {
            combat.round += 1;
        }
        // Defense covers exactly the next enemy round, including faster opponents.
        let defended_before = self.player.defending;
        self.player.defending = matches!(action, Action::Defend | Action::OathPledge(_));
        if self.player.class == Class::Keepwarden {
            // Resolve builds while the oath is held and bleeds off the moment it isn't.
            let state = &mut self.player.class_state;
            state.resolve = if self.player.defending {
                (state.resolve + 1).min(3)
            } else {
                0
            };
        }
        for id in initiative
            .iter()
            .copied()
            .filter(|id| self.npcs[*id].speed > self.player_speed())
            .collect::<Vec<_>>()
        {
            self.enemy_turn(id);
            if self.player.hp <= 0 {
                return;
            }
        }
        match action {
            Action::Move(dx, dy) => {
                let next = self.player.pos.offset(dx, dy);
                if let Some(id) = strike_target {
                    let target = &self.npcs[id];
                    if target.alive()
                        && target.map == self.player.map
                        && hostile(target)
                        && self.can_melee(self.player.pos, target.pos)
                    {
                        self.player_attack(id, defended_before);
                    }
                } else if self.npc_at(next).is_none() && self.map().can_step(self.player.pos, next) {
                    self.maps[self.player.map].set_door_open(next, true);
                    self.player.pos = next;
                    // Gnawbone Crown (§6): dungeon ground feeds your wind.
                    let crown = i32::from(
                        self.in_dungeon() && self.player.relic == Some(BossRelic::GnawboneCrown),
                    );
                    self.player.stamina =
                        (self.player.stamina + 1 + crown).min(self.player.max_stamina);
                    self.caltrop_check();
                    self.scrounge_check();
                    self.break_channel();
                    self.reveal();
                }
            }
            Action::Attack => {
                if let Some(id) = self.adjacent_enemy(false) {
                    self.player_attack(id, defended_before);
                }
            }
            Action::Defend => {
                self.player.history.defended += 1;
                let redwake = i32::from(self.player.class == Class::Redwake);
                // Second Wind (Bulwark 2): the brace feeds the lungs.
                let wind = i32::from(
                    self.player.class == Class::Keepwarden && self.player.has_talent(0, 1),
                ) * 2;
                self.player.stamina =
                    (self.player.stamina + 3 + redwake + wind).min(self.player.max_stamina);
                if self.player.class == Class::Keepwarden && self.player.has_talent(1, 0) {
                    self.player.class_state.riposte_armed = true;
                }
                self.log(format!(
                    "You brace: +50% defense this round; +{} stamina.",
                    3 + redwake + wind
                ));
            }
            Action::OathPledge(index) => {
                let stance = match index {
                    1 => OathStance::Break,
                    2 => OathStance::Breathe,
                    _ => OathStance::Hold,
                };
                self.player.class_state.stance = stance;
                self.player.class_state.break_armed = stance == OathStance::Break;
                self.player.history.defended += 1;
                let wind = i32::from(self.player.has_talent(0, 1)) * 2;
                let stamina = 3 + i32::from(stance == OathStance::Breathe) + wind;
                self.player.stamina = (self.player.stamina + stamina).min(self.player.max_stamina);
                if self.player.has_talent(1, 0) {
                    self.player.class_state.riposte_armed = true;
                }
                self.log(format!("Bulwark oath — {}: +{stamina} stamina.", stance.name()));
                self.modal = Modal::None;
            }
            Action::Wait => {
                let redwake = i32::from(self.player.class == Class::Redwake);
                self.player.stamina =
                    (self.player.stamina + 2 + redwake).min(self.player.max_stamina);
            }
            Action::Use(index) => {
                self.use_item(index);
            }
            Action::Cast(index) => {
                self.cast_spell(index);
            }
            Action::Mercy => {
                self.spare_enemy();
            }
            Action::Flee if self.flee_combat() => {
                return;
            }
            _ => {}
        }
        if self.won {
            return;
        }
        // The hired blade strikes right after the player's initiative.
        if let Some(companion) = self.companion {
            self.companion_turn(companion);
            if self.player.hp <= 0 {
                return;
            }
        }
        for id in initiative {
            if self.npcs[id].speed <= self.player_speed() {
                self.enemy_turn(id);
            }
            if self.player.hp <= 0 {
                return;
            }
        }
        let retreating: Vec<_> = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && n.intent == Intent::Flee
                    && n.pos.distance(self.player.pos) <= 12
            })
            .map(|n| n.id)
            .collect();
        for id in retreating {
            self.world_npc_turn(id);
            if self.player.hp <= 0 {
                return;
            }
        }
        // The brace deliberately survives round end: it is reassigned at the start
        // of the next combat action, so the HUD can truthfully show DEFENDING
        // until the player acts again. Combat end, death, flee and respawn clear it.
        self.player.mana = (self.player.mana + 1).min(self.player.max_mana);
        self.refresh_combat();
        self.update_quests();
        let bosses: Vec<usize> = self
            .combat
            .as_ref()
            .map(|c| {
                c.participants
                    .iter()
                    .copied()
                    .filter(|id| self.npcs[*id].archetype.boss())
                    .collect()
            })
            .unwrap_or_default();
        for id in bosses {
            let phase = if self.npcs[id].hp * 3 <= self.npcs[id].max_hp {
                3
            } else if self.npcs[id].hp * 3 <= self.npcs[id].max_hp * 2 {
                2
            } else {
                1
            };
            if phase > self.npcs[id].phase {
                self.npcs[id].phase = phase;
                self.log(format!(
                    "{} enters phase {phase}! Watch its next tactic.",
                    self.npcs[id].name
                ));
                self.request_decision(id, 3);
            } else if self.combat.as_ref().is_some_and(|c| c.round % 5 == 0) {
                self.request_decision(id, 3);
            }
        }
    }

    fn spark_target(&self) -> Option<usize> {
        self.npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && hostile(n)
                    && n.pos.distance(self.player.pos) <= 5
                    && self.line_clear(n.pos, self.player.pos)
            })
            .min_by_key(|n| (n.pos.distance(self.player.pos), n.id))
            .map(|n| n.id)
    }

    /// A rune already paid for this round by a once-per-combat lesson (§5 Storm/Ink capstones).
    fn free_cast(&self) -> Option<u32> {
        if self.player.class != Class::SigilSworn {
            return None;
        }
        if self.player.has_talent(0, 3) && self.player.talent_once & once::TEMPEST == 0 {
            return Some(once::TEMPEST);
        }
        if self.player.has_talent(1, 3) && self.player.talent_once & once::WORDSMITH == 0 {
            return Some(once::WORDSMITH);
        }
        None
    }

    fn cast_spell(&mut self, index: usize) {
        let Some(spell) = self.player.spells.get(index).copied() else {
            return;
        };
        let wordsmith = self.free_cast() == Some(once::WORDSMITH);
        let cost = match self.free_cast() {
            Some(bit) => {
                self.player.talent_once |= bit;
                if wordsmith {
                    self.log("Wordsmith: the rune writes itself, mana untouched.");
                } else {
                    self.log("Tempest: the first rune of the fight rides free.");
                }
                0
            }
            None => self.spell_cost(spell),
        };
        self.player.mana = (self.player.mana - cost).max(0);
        if wordsmith {
            // The anchor resets: the next repeated rune starts its own ladder.
            self.player.class_state.channel = 0;
            self.player.class_state.channel_spell = None;
        }
        // Sigil-Sworn channel (§4.2): same rune on consecutive rounds builds charge.
        let chan_cap = self.channel_cap();
        if self.player.class == Class::SigilSworn {
            let state = &mut self.player.class_state;
            state.channel = if state.channel_spell == Some(spell) {
                (state.channel + 1).min(chan_cap)
            } else {
                0
            };
            state.channel_spell = Some(spell);
        }
        match spell {
            Spell::Spark => {
                let plumb = i32::from(self.player.has_talent(2, 0));
                let damage =
                    self.attack_power().max(4) + i32::from(self.player.class_state.channel) + plumb;
                if let Some(id) = self.spark_target() {
                    self.log(format!(
                        "Your spark bolt arcs to {}: {damage} damage, ignoring armour.",
                        self.npcs[id].name
                    ));
                    self.hurt_npc(id, damage, true);
                    let killed = !self.npcs[id].alive();
                    if killed && self.player.has_talent(2, 2) {
                        // Slow Burn: the kill feeds the anchor, not the reset.
                        let state = &mut self.player.class_state;
                        state.channel = (state.channel + 1).min(chan_cap);
                    }
                    // Overcharge: charged sparks leap to the crowd around the target.
                    if self.player.has_talent(0, 2) && self.player.class_state.channel > 0 {
                        let splash = 1 + self.player.branch_synergy(0);
                        let crowd: Vec<usize> = self
                            .npcs
                            .iter()
                            .filter(|n| {
                                n.alive()
                                    && n.map == self.player.map
                                    && hostile(n)
                                    && n.id != id
                                    && n.pos.distance(self.npcs[id].pos) <= 1
                            })
                            .map(|n| n.id)
                            .collect();
                        for other in crowd {
                            self.log(format!("The charge leaps to {}: {splash}.", self.npcs[other].name));
                            self.hurt_npc(other, i32::from(splash.max(1)), true);
                        }
                    }
                    // Cascade: the fight's first spark splits onto a second mark.
                    if self.player.has_talent(2, 3) && self.player.talent_once & once::CASCADE == 0 {
                        let second = self
                            .npcs
                            .iter()
                            .filter(|n| {
                                n.alive()
                                    && n.map == self.player.map
                                    && hostile(n)
                                    && n.id != id
                                    && n.pos.distance(self.player.pos) <= 5
                                    && self.line_clear(n.pos, self.player.pos)
                            })
                            .min_by_key(|n| (n.pos.distance(self.player.pos), n.id))
                            .map(|n| n.id);
                        if let Some(other) = second {
                            self.player.talent_once |= once::CASCADE;
                            let forked = (damage / 2).max(1);
                            self.log(format!(
                                "The bolt forks onto {}: {forked} damage.",
                                self.npcs[other].name
                            ));
                            self.hurt_npc(other, forked, true);
                        }
                    }
                } else {
                    self.log("The spark finds no living target and grounds itself.");
                }
            }
            Spell::Mend => {
                let heal = 12
                    + if self.player.has_talent(1, 2) {
                        4 + i32::from(self.player.branch_synergy(1))
                    } else {
                        0
                    };
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                self.log(format!("Warm light knits your wounds: +{heal} HP."));
            }
            Spell::Ward => {
                let rounds = 2 + u64::from(self.player.has_talent(1, 1));
                self.player.ward_until = self.turn + rounds;
                self.player.defending = true;
                self.log(format!(
                    "An invisible shield settles over you: +2 defense and braced for {rounds} rounds.",
                ));
            }
        }
    }

    fn cast_exploration(&mut self, index: usize) {
        let Some(spell) = self.player.spells.get(index).copied() else {
            return;
        };
        let cost = self.spell_cost(spell);
        if self.player.mana < cost {
            self.log(format!(
                "Not enough mana for {} ({} needed). Rest or let it return slowly.",
                spell.name(),
                cost
            ));
            return;
        }
        match spell {
            Spell::Mend => {
                self.player.mana -= cost;
                self.player.hp = (self.player.hp + 12).min(self.player.max_hp);
                self.log("You mend your wounds between battles: +12 HP.");
            }
            Spell::Spark => self.log("A spark needs a living enemy. Save it for battle."),
            Spell::Ward => self.log("The ward only matters once blades are out."),
        }
    }

    fn hurt_companion(&mut self, id: usize, damage: i32, attacker: usize) {
        // Thick Coat: the partner's hide was fed on fen-milk.
        let padded = i32::from(
            self.player.class == Class::Fensworn && self.player.has_talent(0, 1),
        );
        let damage = (damage - padded).max(1);
        let hp_after = (self.npcs[id].hp - damage).max(0);
        self.npcs[id].hp = hp_after;
        // Bogside Vigil: once per fight the truce speaks — the partner stands at 5.
        if hp_after == 0
            && self.player.class == Class::Fensworn
            && self.player.has_talent(1, 3)
            && self.player.talent_once & once::BOGSIDE == 0
        {
            self.player.talent_once |= once::BOGSIDE;
            self.npcs[id].hp = 5;
            self.log(format!(
                "The bogs hold {} upright. Five breaths remain.",
                self.npcs[id].name
            ));
            self.effects.push(FloatingText {
                pos: self.npcs[id].pos,
                text: format!("-{damage}"),
                ttl: 4,
            });
            return;
        }
        self.effects.push(FloatingText {
            pos: self.npcs[id].pos,
            text: format!("-{damage}"),
            ttl: 4,
        });
        self.log(format!(
            "{} hits {} for {damage}.",
            self.npcs[attacker].name, self.npcs[id].name
        ));
        if hp_after == 0 {
            self.npcs[id].intent = Intent::Idle;
            self.log(format!(
                "{} is down! They will rise when the fight is over.",
                self.npcs[id].name
            ));
        }
    }

    fn companion_turn(&mut self, id: usize) {
        if !self.npcs[id].alive() || self.npcs[id].map != self.player.map {
            return;
        }
        let pos = self.npcs[id].pos;
        let target = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive() && n.map == self.player.map && hostile(n) && self.can_melee(pos, n.pos)
            })
            .min_by_key(|n| (n.hp, n.id))
            .map(|n| n.id);
        if let Some(target) = target {
            // Bond (§4.2): the Fensworn's partner bites twice on the cadence — Packlord hastens it.
            let cadence: u8 = if self.player.class == Class::Fensworn
                && self.player.has_talent(0, 3)
            {
                2
            } else {
                3
            };
            let strikes = if self.player.class == Class::Fensworn {
                self.player.class_state.bond += 1;
                if self.player.class_state.bond >= cadence {
                    self.player.class_state.bond = 0;
                    2
                } else {
                    1
                }
            } else {
                1
            };
            // Lean Bite: the partner was taught where the hamstring runs.
            let lean = i32::from(self.player.class == Class::Fensworn && self.player.has_talent(0, 0))
                // Truce (§8.1): the written oath carries to every bite.
                + self.word_on(WordSlot::Weapon, "Truce");
            for strike in 0..strikes {
                if !self.npcs[target].alive() {
                    break;
                }
                let damage = (self.npcs[id].attack + lean - self.npcs[target].defense).max(1);
                self.log(format!(
                    "{} strikes {} for {damage}{}.",
                    self.npcs[id].name,
                    self.npcs[target].name,
                    if strike == 1 { " — the bond answers" } else { "" }
                ));
                self.hurt_npc(target, damage, true);
                if strike == 1 && self.player.class == Class::Fensworn && self.player.has_talent(0, 2) {
                    // Bloodline: the doubled blow returns as breath.
                    let heal = 2 + self.player.branch_synergy(0);
                    self.npcs[id].hp = (self.npcs[id].hp + i32::from(heal)).min(self.npcs[id].max_hp);
                }
            }
            return;
        }
        let threat = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == self.player.map
                    && hostile(n)
                    && n.pos.distance(self.player.pos) <= 4
            })
            .min_by_key(|n| (n.pos.distance(pos), n.id))
            .map(|n| n.pos);
        if let Some(goal) = threat {
            self.step_toward(id, goal);
        } else if pos.distance(self.player.pos) > 1 {
            self.step_toward(id, self.player.pos);
        }
    }

    fn player_attack(&mut self, id: usize, defended_before: bool) {
        let exhausted = self.player.stamina < 2;
        self.player.stamina = (self.player.stamina - 2).max(0);
        self.player.history.attacks += 1;
        self.npcs[id]
            .memory
            .remember(self.turn, "player_attacked_me", -0.25);
        // Three False Lights (§6): a mistaken glow drinks the blow for its mistress.
        if self.npcs[id].archetype == Archetype::FalseGlow {
            if let Some(mistress) = self.npcs.iter().position(|n| {
                n.archetype == Archetype::Mirelight && n.alive() && n.map == self.player.map
            }) {
                self.npcs[mistress].hp =
                    (self.npcs[mistress].hp + 6).min(self.npcs[mistress].max_hp);
                self.log("The light drinks your blow — the true wisp brightens.");
            }
        }
        let relic_bonus = if defended_before
            && self.player.relic == Some(BossRelic::Rallybreaker)
            && !self.sigil_unwritten(0)
        {
            2
        } else {
            0
        };
        // Redwake momentum (§4.2): every banked kill or escape sharpens this blow.
        let momentum = if self.player.class == Class::Redwake {
            self.player.class_state.momentum
        } else {
            0
        };
        self.player.class_state.momentum = 0;
        // Ebb's synergy: each pip spends for +1 more per Assassin node 1–2.
        let payout = 2
            + if self.player.class == Class::Redwake && self.player.has_talent(0, 2) {
                self.player.branch_synergy(0)
            } else {
                0
            };
        // Red Tide: a strike fed the full bank ignores defense entirely.
        let red_tide = self.player.class == Class::Redwake
            && self.player.has_talent(0, 3)
            && momentum >= self.momentum_cap()
            && momentum > 0;
        // Keepwarden Break Their Line: the traded defense owed this strike +2.
        let broken = std::mem::take(&mut self.player.class_state.break_armed);
        // Drilled Counters: the brace's answer is already cocked.
        let riposte = std::mem::take(&mut self.player.class_state.riposte_armed);
        let mut riposte_bonus = i32::from(riposte) * 2;
        // Gatecrash: once per combat the counter blows the gate, not the door.
        if riposte
            && defended_before
            && self.player.has_talent(1, 3)
            && self.player.talent_once & once::GATECRASH == 0
        {
            self.player.talent_once |= once::GATECRASH;
            riposte_bonus += 4;
        }
        self.break_channel();
        let foe_defense = if red_tide { 0 } else { self.npcs[id].defense };
        let damage =
            (self.attack_power() + relic_bonus + i32::from(momentum) * i32::from(payout)
                + riposte_bonus
                + i32::from(broken) * 2
                - foe_defense
                - i32::from(exhausted))
            .max(1);
        self.log(format!(
            "You strike {} for {damage}{}{}{}{}.",
            self.npcs[id].name,
            if exhausted { " (exhausted)" } else { "" },
            if relic_bonus > 0 {
                " — Rallybreaker answers your brace"
            } else {
                ""
            },
            if momentum > 0 {
                if red_tide {
                    " — the Red Tide takes them whole".to_string()
                } else {
                    format!(" — {momentum} momentum spent")
                }
            } else {
                String::new()
            },
            if riposte {
                " — the brace answers"
            } else if broken {
                " — their line breaks"
            } else {
                ""
            }
        ));
        self.hurt_npc(id, damage, true);
        // Road-Sworn: once per combat, a kill on oathroad rolls straight into the next blow.
        if !self.npcs[id].alive()
            && self.player.class == Class::Waysworn
            && self.player.has_talent(0, 3)
            && self.player.talent_once & once::ROAD_SWORN == 0
            && matches!(self.map().tile(self.player.pos), Tile::Road | Tile::Ford)
        {
            if let Some(next) = self.adjacent_enemy(false) {
                self.player.talent_once |= once::ROAD_SWORN;
                self.log("The road rolls you into the next blow.");
                self.player_attack(next, false);
            }
        }
    }

    fn hurt_npc(&mut self, id: usize, damage: i32, reward: bool) {
        if !self.npcs[id].alive() {
            return;
        }
        self.effects.push(FloatingText {
            pos: self.npcs[id].pos,
            text: format!("-{damage}"),
            ttl: 4,
        });
        if damage >= self.npcs[id].hp {
            if reward {
                self.kill_npc(id);
            } else {
                self.npcs[id].hp = 0;
                self.npcs[id].decision_pending = false;
                self.npcs[id].decision_version += 1;
                self.log(format!("The watch defeats {}.", self.npcs[id].name));
                self.update_quests();
            }
        } else {
            self.npcs[id].hp -= damage;
            let band = self.npcs[id].hp * 4 / self.npcs[id].max_hp.max(1);
            if band != self.npcs[id].last_hp_band {
                self.request_decision(id, 1);
                if self.npcs[id].decision_pending {
                    self.npcs[id].last_hp_band = band;
                }
            }
        }
    }

    fn hurt_player(&mut self, damage: i32, source: &str) {
        let damage = damage.max(1);
        self.player.hp = (self.player.hp - damage).max(0);
        self.player.last_hazard = source.to_string();
        self.effects.push(FloatingText {
            pos: self.player.pos,
            text: format!("-{damage}"),
            ttl: 4,
        });
        self.log(format!("{source}: {damage} damage."));
        // Plunder the Ossuary: below a quarter, the pack answers for you, once.
        if self.player.class == Class::Gravebound
            && self.player.has_talent(2, 3)
            && self.player.talent_once & once::OSSUARY == 0
            && self.player.hp * 4 < self.player.max_hp
        {
            if let Some(index) = self
                .player
                .inventory
                .iter()
                .position(|item| *item == Item::Potion)
            {
                self.player.talent_once |= once::OSSUARY;
                self.player.inventory.remove(index);
                let heal = self.potion_heal();
                self.player.hp = (self.player.hp + heal).min(self.player.max_hp);
                self.log("The ossuary opens: a potion drinks itself for you — +{heal} HP.");
            }
        }
        // Bell Toll: the first crossing below half ring-adjacent enemies back 4 beats.
        if self.player.class == Class::Gravebound
            && self.player.has_talent(1, 1)
            && self.player.talent_once & once::BELL == 0
            && self.player.hp > 0
            && self.player.hp * 2 < self.player.max_hp
        {
            self.player.talent_once |= once::BELL;
            let mut rang = 0;
            for npc in self.npcs.iter_mut() {
                if npc.alive() && npc.map == self.player.map && hostile(npc) && npc.pos.distance(self.player.pos) <= 1 {
                    npc.cooldown = npc.cooldown.max(4);
                    rang += 1;
                }
            }
            if rang > 0 {
                self.log("The bell tolls: death's neighbors pause.");
            }
        }
        if self.player.hp == 0 {
            // Lethal intercepts, in order: the brace first (Bastion), then the vow (Methuselah).
            let bastion = self.player.class == Class::Keepwarden
                && self.player.has_talent(0, 3)
                && self.player.defending
                && self.player.talent_once & once::BASTION == 0;
            let methuselah = !bastion
                && self.player.class == Class::Gravebound
                && self.player.has_talent(0, 3)
                && self.player.talent_once & once::METHUSELAH == 0;
            if bastion || methuselah {
                self.player.talent_once |= if bastion { once::BASTION } else { once::METHUSELAH };
                self.player.hp = 1;
                self.log(if bastion {
                    "Last Bastion: the wound refuses the brace. You stand at 1 HP."
                } else {
                    "Methuselah Vow: death bows and steps back. You stand at 1 HP."
                });
                return;
            }
            self.combat = None;
            self.modal = Modal::Death;
            self.player.defending = false;
            self.log("You fall. Press Enter to return to your last city; the world remembers.");
        }
    }

    fn flanked(&self, attacker: usize) -> bool {
        let player = self.player.pos;
        let pos = self.npcs[attacker].pos;
        let opposite = player.offset((player.x - pos.x).signum(), (player.y - pos.y).signum());
        if !self.map().can_step(player, opposite) {
            return false;
        }
        self.npcs.iter().any(|n| {
            n.id != attacker
                && n.alive()
                && n.map == self.player.map
                && (hostile(n) || Some(n.id) == self.companion)
                && n.intent != Intent::Flee
                && n.pos == opposite
        })
    }

    fn enemy_turn(&mut self, id: usize) {
        // Overwatch (Oathblade 3): enemies that step INTO the brace's reach pay for it.
        let was_adjacent = self.npcs[id].alive()
            && self.npcs[id].map == self.player.map
            && self.can_melee(self.npcs[id].pos, self.player.pos);
        self.enemy_turn_impl(id);
        if !was_adjacent
            && self.npcs[id].alive()
            && self.npcs[id].map == self.player.map
            && self.player.hp > 0
            && self.can_melee(self.npcs[id].pos, self.player.pos)
            && self.player.defending
            && self.player.class == Class::Keepwarden
            && self.player.has_talent(1, 2)
        {
            let bite = 1 + self.player.branch_synergy(1);
            self.log(format!(
                "Overwatch: the brace bites {} for {bite} as it closes.",
                self.npcs[id].name
            ));
            self.hurt_npc(id, i32::from(bite), true);
        }
    }

    fn enemy_turn_impl(&mut self, id: usize) {
        if !self.npcs[id].alive()
            || self.npcs[id].map != self.player.map
            || self.player.hp <= 0
            || !hostile(&self.npcs[id])
        {
            return;
        }
        if self.npcs[id].intent == Intent::Flee {
            return;
        }
        let kind = self.npcs[id].archetype;
        // Brood-holes never act; they only feed the tide and die (§6).
        if kind == Archetype::BroodHole {
            return;
        }
        // Gnawbone Crown (§6): the warren's debts are paid; rats cannot turn on you.
        if kind == Archetype::Rat && self.player.relic == Some(BossRelic::GnawboneCrown) {
            return;
        }
        // Bond-wolf truce (D5): the pack knows the Packlord's name.
        if kind == Archetype::Wolf && self.wolf_truce_holds(&self.npcs[id]) {
            return;
        }
        if kind.boss() && self.boss_turn(id) {
            return;
        }
        let target = choice(&self.npcs[id], "target_pick");
        if target == "packmate" || score(&self.npcs[id], "retreat_to_group") > 0.65 {
            let ally = self
                .npcs
                .iter()
                .filter(|n| {
                    n.id != id
                        && n.alive()
                        && n.map == self.player.map
                        && n.pack == self.npcs[id].pack
                        && n.pos.distance(self.npcs[id].pos) > 1
                })
                .min_by_key(|n| n.pos.distance(self.npcs[id].pos))
                .map(|n| n.pos);
            if let Some(pos) = ally {
                if !self.can_melee(self.npcs[id].pos, self.player.pos) {
                    self.step_toward(id, pos);
                    return;
                }
            }
        }
        if target == "cattle" && self.npcs[id].hp * 2 < self.npcs[id].max_hp {
            self.npcs[id].intent = Intent::Flee;
            self.log(format!(
                "{} abandons you for easier prey.",
                self.npcs[id].name
            ));
            self.step_away(id, self.player.pos);
            return;
        }
        if !self.can_melee(self.npcs[id].pos, self.player.pos) {
            if let Some(companion) = self.companion.filter(|c| {
                self.npcs[*c].alive()
                    && self.npcs[*c].map == self.player.map
                    && self.can_melee(self.npcs[id].pos, self.npcs[*c].pos)
            }) {
                // A bodyguard in reach draws the blow meant for their employer.
                let damage = (self.npcs[id].attack - self.npcs[companion].defense).max(1);
                self.hurt_companion(companion, damage, id);
                return;
            }
        }
        if self.can_melee(self.npcs[id].pos, self.player.pos) {
            let mut attack = self.npcs[id].attack;
            // Ash-Writhe (Censer 1): the half-dead swing softer at the Gravebound.
            if self.player.class == Class::Gravebound
                && self.player.has_talent(1, 0)
                && self.npcs[id].hp * 2 < self.npcs[id].max_hp
            {
                attack -= 1;
            }
            if self.npcs[id].tactic == "rallied" {
                attack += 1;
            }
            if kind == Archetype::Bear {
                if self.npcs[id].cooldown == 0 {
                    self.npcs[id].cooldown = 1;
                    self.log(format!(
                        "{} rears up! Move away or defend against its next crushing blow.",
                        self.npcs[id].name
                    ));
                    return;
                }
                self.npcs[id].cooldown = 0;
                attack += 2;
            }
            // Judgment denies the flank against the brace outright; Waylaid Reading denies
            // it only in the ambush round.
            let ambush = self.combat.as_ref().is_some_and(|c| c.round <= 1);
            let flank_denied = (self.player.defending
                && self.player.class == Class::Keepwarden
                && self.player.has_talent(1, 1))
                || (ambush
                    && self.player.class == Class::Waysworn
                    && self.player.has_talent(0, 1));
            let flanked = self.flanked(id) && !flank_denied;
            let defense = if self.player.defending {
                self.braced_defense()
            } else {
                self.defense_power()
            };
            let damage = (attack - defense).max(1);
            let damage = if flanked {
                (damage * 5 + 3) / 4
            } else {
                damage
            };
            let name = format!(
                "{}{}",
                self.npcs[id].name,
                if flanked { " flanks you" } else { " strikes" }
            );
            // Vigil (§8.1): once per combat the written brace refuses the first hit.
            if self.player.defending
                && self.word_on(WordSlot::Armour, "Vigil") == 1
                && self.player.talent_once & once::VIGIL_WORD == 0
            {
                self.player.talent_once |= once::VIGIL_WORD;
                self.log("The written brace flares: the Vigil-word turns the first blow aside entirely.");
            } else {
                self.hurt_player(damage, &name);
            }
            // Plague Tide bites plague (D36): the fen barrow's rats stack fester.
            if kind == Archetype::Rat
                && matches!(
                    self.maps[self.npcs[id].map].kind,
                    MapKind::Dungeon(4, _)
                )
            {
                self.infect_fester();
            }
            // Hold the Gate (§4.2, A1/D21): the oath bites back at one attacker per round.
            if self.player.class == Class::Keepwarden
                && self.player.class_state.stance == OathStance::Hold
                && self.player.defending
                && self.player.hp > 0
            {
                let round = self.combat.as_ref().map(|c| c.round);
                if round.is_some() && self.player.class_state.recoil_used_round != round {
                    self.player.class_state.recoil_used_round = round;
                    // Anchor (Bulwark 3): the stance bites harder per bulwark node 1–2.
                    let recoil = 1
                        + if self.player.has_talent(0, 2) {
                            self.player.branch_synergy(0)
                        } else {
                            0
                        };
                    self.log(format!(
                        "The gate answers: {} takes {recoil} recoil.",
                        self.npcs[id].name
                    ));
                    self.hurt_npc(id, i32::from(recoil), true);
                }
            }
        } else {
            // Wolves seek the opposite side of an existing attacker rather than queueing.
            if matches!(kind, Archetype::Wolf | Archetype::Matriarch) {
                if let Some(ally) = self.npcs.iter().find(|n| {
                    n.id != id
                        && n.alive()
                        && n.map == self.player.map
                        && n.pack == self.npcs[id].pack
                        && self.can_melee(n.pos, self.player.pos)
                }) {
                    let goal = self.player.pos.offset(
                        (self.player.pos.x - ally.pos.x).signum(),
                        (self.player.pos.y - ally.pos.y).signum(),
                    );
                    if self.map().tile(goal).walkable() && self.npc_at(goal).is_none() {
                        self.step_toward(id, goal);
                        return;
                    }
                }
            }
            self.step_toward(id, self.player.pos);
        }
    }

    fn flee_combat(&mut self) -> bool {
        let enemies = self
            .combat
            .as_ref()
            .map(|c| c.participants.clone())
            .unwrap_or_default();
        // D13 cruel terms: sworn at the verdict shrine, the sworn stand to the end.
        if self.trial_oath
            && enemies
                .iter()
                .any(|id| self.npcs[*id].archetype == Archetype::Adjudicator)
        {
            self.log("The sworn terms hold you fast: there is no retreat from judgment.");
            return false;
        }
        let speed = enemies
            .iter()
            .filter(|id| self.npcs[**id].alive())
            .map(|id| self.npcs[*id].speed)
            .max()
            .unwrap_or(0);
        let mercy = enemies
            .iter()
            .copied()
            .find(|id| !self.npcs[*id].mercy_used && score(&self.npcs[*id], "mercy") > 0.65);
        // A3/D23: three banked momentum buys a guaranteed exit — the tide takes you.
        let paid_exit = self.player.class == Class::Redwake
            && self.player.class_state.momentum >= 3;
        let mut escape = paid_exit;
        // Rip Current (Foamrunner 4): one bubble of the tide per fight is simply yours.
        if !escape
            && self.player.class == Class::Redwake
            && self.player.has_talent(2, 3)
            && self.player.talent_once & once::RIP_CURRENT == 0
        {
            self.player.talent_once |= once::RIP_CURRENT;
            escape = true;
            self.log("The rip current takes you. No one follows.");
        }
        // Hollow Court and Lure widen your margin when you need them most.
        let margin = 2
            - 2 * i32::from(
                self.player.class == Class::Gravebound
                    && self.player.has_talent(1, 2)
                    && self.player.hp * 2 < self.player.max_hp,
            )
            - i32::from(self.player.class == Class::Fensworn && self.player.has_talent(2, 2));
        let escape = escape || self.player_speed() + self.random(7) as i32 >= speed + margin.max(0);
        if !escape && mercy.is_none() {
            // Second Tide (Foamrunner 2): the first failed exit is free.
            let toll = if self.player.class == Class::Redwake
                && self.player.has_talent(2, 1)
                && self.player.talent_once & once::SECOND_TIDE == 0
            {
                self.player.talent_once |= once::SECOND_TIDE;
                0
            } else {
                1
            };
            self.player.stamina = (self.player.stamina - toll).max(0);
            self.log(if toll == 0 {
                "Your escape is cut off — but the tide forgives the first."
            } else {
                "Your escape is cut off! Defend for stamina or try again."
            });
            return false;
        }
        if !escape {
            if let Some(id) = mercy {
                self.npcs[id].mercy_used = true;
                self.log(format!(
                    "{} grants you one retreat. It will not show mercy twice.",
                    self.npcs[id].name
                ));
            }
        }
        // Trace a legal escape route; never teleport through a wall or occupied square.
        let mut pos = self.player.pos;
        for _ in 0..7 {
            let current_safety = enemies
                .iter()
                .filter(|id| self.npcs[**id].alive())
                .map(|id| self.npcs[*id].pos.distance(pos))
                .min()
                .unwrap_or(10);
            let next = DIRECTIONS
                .iter()
                .map(|(x, y)| pos.offset(*x, *y))
                .filter(|next| self.map().can_step(pos, *next) && self.npc_at(*next).is_none())
                .max_by_key(|next| {
                    enemies
                        .iter()
                        .filter(|id| self.npcs[**id].alive())
                        .map(|id| self.npcs[*id].pos.distance(*next))
                        .min()
                        .unwrap_or(10)
                });
            if let Some(next) = next {
                let safety = enemies
                    .iter()
                    .filter(|id| self.npcs[**id].alive())
                    .map(|id| self.npcs[*id].pos.distance(next))
                    .min()
                    .unwrap_or(10);
                if safety < current_safety {
                    break;
                }
                pos = next;
            } else {
                break;
            }
        }
        if enemies
            .iter()
            .any(|id| self.npcs[*id].alive() && self.can_melee(self.npcs[*id].pos, pos))
        {
            self.log("There is no clear escape route. Make space first!");
            return false;
        }
        self.maps[self.player.map].set_door_open(pos, true);
        self.player.pos = pos;
        self.player.history.fled += 1;
        self.player.defending = false;
        if self.player.class == Class::Redwake {
            // A bought exit spends the bank; a natural one grows it (A3/D23, §4.2, Ebb arms).
            let cap = self.momentum_cap();
            let ebb = !paid_exit && self.player.has_talent(0, 2);
            let state = &mut self.player.class_state;
            state.momentum = if paid_exit {
                0
            } else {
                (state.momentum + 1).min(cap)
            };
            state.ebb_armed = ebb;
        }
        // Rip Line (Foamrunner 1): fled enemies take longer prying themselves free.
        let regroup = if self.player.class == Class::Redwake && self.player.has_talent(2, 0) {
            12
        } else {
            8
        };
        for id in enemies {
            self.npcs[id].cooldown = regroup;
            self.npcs[id]
                .memory
                .remember(self.turn, "player_fled", 0.05);
            if self.npcs[id].archetype == Archetype::Guard
                && score(&self.npcs[id], "pursue_fleeing_player") < 0.6
            {
                self.npcs[id].intent = Intent::Patrol;
            }
        }
        self.combat = None;
        self.log("You escape the combat bubble. Enemies need a moment to regroup.");
        self.reveal();
        true
    }

    fn spare_enemy(&mut self) -> bool {
        // Crow-Broker (Dockhand 4): once per combat, a beaten foe buys its life with gratitude.
        let broker = self.adjacent_enemy(false).filter(|id| {
            self.player.class == Class::Redwake
                && self.player.has_talent(1, 3)
                && self.player.talent_once & once::CROW_BROKER == 0
                && self.npcs[*id].hp * 2 < self.npcs[*id].max_hp
        });
        let Some(id) = self.adjacent_enemy(true).or(broker) else {
            self.log("Only an adjacent fleeing enemy can be spared.");
            return false;
        };
        if broker == Some(id) {
            self.player.talent_once |= once::CROW_BROKER;
            self.log("The crow picks its price: the beaten accept your mercy.");
        }
        self.npcs[id]
            .memory
            .remember(self.turn, "player_spared_me", 0.8);
        self.npcs[id].intent = Intent::Flee;
        self.npcs[id].decision_version += 1;
        self.npcs[id].decision_pending = false;
        self.player.history.mercy += 1;
        // E7 (§6.5): mercy for the oathbreaker — the Adjudicator's final rating
        // reads whether this answer was sincere.
        if self.npcs[id].archetype == Archetype::OathlessCurate {
            self.mercy_oathbreaker = true;
            self.give_loot(Item::FirstWrit);
            self.combat = None;
            self.log("The Curate's hands stop shaking. 'Then the covenant dies a third death — by kindness.' He presses a folded writ into your palm and is gone from the cell.");
            self.log("The First Writ is yours. Carry it past the Trial: the arbiter will have a question for you.");
            return true;
        }
        self.gain_xp(5 + self.npcs[id].level * 5);
        self.log(format!(
            "You lower your weapon. {} will remember your mercy.",
            self.npcs[id].name
        ));
        self.refresh_combat();
        true
    }

    pub fn kill_npc(&mut self, id: usize) {
        if id >= self.npcs.len() || !self.npcs[id].alive() {
            return;
        }
        let kind = self.npcs[id].archetype;
        let pack = self.npcs[id].pack;
        let map = self.npcs[id].map;
        let level = self.npcs[id].level;
        self.npcs[id].hp = 0;
        self.npcs[id].decision_pending = false;
        self.npcs[id].decision_version += 1;
        self.player.history.kills += 1;
        // Signature payouts (§4.2): the Redwake banks momentum; the Waysworn opens the road.
        match self.player.class {
            Class::Redwake => {
                // Ebb: the kill after a successful flee banks double.
                let ebb = self.player.class_state.ebb_armed;
                let gain = u8::from(ebb && self.player.has_talent(0, 2)) + 1;
                let cap = self.momentum_cap();
                let state = &mut self.player.class_state;
                state.ebb_armed = false;
                state.momentum = (state.momentum + gain).min(cap);
                if self.player.has_talent(0, 1) {
                    self.player.gold += 3; // Cutpurse
                }
            }
            Class::Waysworn => {
                let cap = 1 + u8::from(self.player.has_talent(0, 2));
                let state = &mut self.player.class_state;
                state.trail_charges = (state.trail_charges + 1).min(cap);
            }
            Class::Gravebound => {
                let wounded = self.player.hp * 2 < self.player.max_hp;
                let deep = self.player.hp * 4 < self.player.max_hp;
                if wounded && self.player.has_talent(0, 1) {
                    let heal = 2
                        + if self.player.has_talent(0, 2) {
                            self.player.branch_synergy(0)
                        } else {
                            0
                        };
                    self.player.hp = (self.player.hp + i32::from(heal)).min(self.player.max_hp);
                }
                if deep && self.player.has_talent(0, 3) {
                    self.player.stamina = self.player.max_stamina;
                    self.log("Requiem: the fall feeds you. Stamina restored.");
                }
                if self.player.has_talent(2, 0) {
                    let goods = 1
                        + if self.player.has_talent(2, 2) {
                            self.player.branch_synergy(2)
                        } else {
                            0
                        };
                    self.player.gold += u32::from(goods);
                }
            }
            Class::Fensworn => {
                let tile = self.map().tile(self.player.pos);
                if matches!(tile, Tile::Forest | Tile::DeepForest) && self.player.has_talent(2, 0) {
                    let heal = 1
                        + if self.player.has_talent(2, 2) {
                            self.player.branch_synergy(2)
                        } else {
                            0
                        };
                    self.player.stamina =
                        (self.player.stamina + i32::from(heal)).min(self.player.max_stamina);
                }
                if self.player.has_talent(2, 1)
                    && matches!(
                        kind,
                        Archetype::Wolf | Archetype::Bear | Archetype::Rat
                    )
                {
                    self.player.gold += 2; // Skinning
                }
            }
            _ => {}
        }
        if kind == Archetype::Rat {
            self.rats_killed += 1;
        }
        let gold = if kind.boss() {
            70 + 15 * level
        } else {
            3 + level * 3
        };
        self.player.gold += gold;
        self.log(format!("{} falls. +{gold} gold.", self.npcs[id].name));
        self.gain_xp(if kind.boss() {
            100 + level * 15
        } else {
            8 + level * 8
        });
        let sigil = match kind {
            Archetype::Chief => Some(0),
            Archetype::Matriarch => Some(1),
            Archetype::Lich => Some(2),
            _ => None,
        };
        self.bounty_progress(kind);
        // E3 arena relics (D11: sigil bosses keep sigils; arena bosses pay relics only).
        match kind {
            Archetype::Tidemother => self.give_boss_relic(BossRelic::Saltcrown),
            Archetype::Cragmother => self.give_boss_relic(BossRelic::Stoneheart),
            // E5 side-boss relics (§6); D8 gives the essence two living sources.
            Archetype::GnawThane => self.give_boss_relic(BossRelic::GnawboneCrown),
            Archetype::Tollmaster => self.give_boss_relic(BossRelic::TollcoinCharm),
            Archetype::Mirelight => {
                self.give_boss_relic(BossRelic::WisplightLantern);
                self.give_loot(Item::Essence);
                for glow in &mut self.npcs {
                    if glow.archetype == Archetype::FalseGlow && glow.alive() {
                        glow.hp = 0;
                    }
                }
                self.log("The false lights gutter out with their mistress.");
            }
            Archetype::PaleStag => {
                self.give_boss_relic(BossRelic::Hartshorn);
                self.give_loot(Item::Essence);
            }
            // E7 (§6.5): the covenant-breaker's ransom. Killing him pays it too —
            // the mercy path is the alternative, not the only door.
            Archetype::OathlessCurate => {
                self.give_loot(Item::FirstWrit);
                self.log("The First Writ is yours. The arbiter will have a question for whoever carries it past the Trial.");
            }
            _ => {}
        }
        // E7 (D31 §8.1): glyph drops ride the elite line — every boss passes on
        // the fragment of seal-script its order once read.
        if kind.boss() {
            let fragment = match kind {
                Archetype::GnawThane | Archetype::Matriarch => 1, // fen
                Archetype::Tollmaster => 2,                       // gate
                Archetype::Mirelight | Archetype::Chief => 0,     // ash
                Archetype::PaleStag => 4,                         // hart
                Archetype::OathlessCurate | Archetype::Lich | Archetype::Tidemother => 3, // seal
                _ => 5,                                           // crown
            };
            self.give_loot(Item::Glyph(fragment));
        }
        if let Some(index) = sigil {
            self.player.sigils[index] = true;
            self.player.hp = self.player.max_hp;
            self.player.stamina = self.player.max_stamina;
            // Talent income (§5): a sigil claim is a boss-bonus talent point.
            self.player.talent_points += 1;
            self.log(format!(
                "The {} Sigil is yours! Its light restores you.",
                ["Burrow", "Crimson", "Underkeep"][index]
            ));
            // E7 (§6.5 act V): the sigils key two doors. The third claim wakes
            // the covenant-breaker in the depths below.
            if self.player.sigils.iter().all(|s| *s) {
                self.spawn_curate();
            }
            let reward = [Item::Weapon(1), Item::Armour(2), Item::Weapon(3)][index].clone();
            self.give_loot(reward);
            self.give_boss_relic(
                [
                    BossRelic::Rallybreaker,
                    BossRelic::Fangmantle,
                    BossRelic::Graveglass,
                ][index],
            );
            self.give_loot(Item::Potion);
        } else if kind == Archetype::Adjudicator {
            self.won = true;
            self.combat = None;
            self.modal = Modal::Victory;
            self.log("The Adjudicator kneels. Your deeds, not your origin, have shaped the realm.");
            if self.trial_oath {
                self.log("Sworn and kept: not one step was taken back. Its last nod is for the Watch it hoped to find.");
            }
        } else if mix(self.seed ^ id as u64).is_multiple_of(4) {
            self.give_loot(Item::Potion);
        }
        if !kind.hostile() {
            if let Some(city) = self.city() {
                self.player.reputation[city] = (self.player.reputation[city] - 25).max(0);
            }
        }
        let survivors: Vec<usize> = self
            .npcs
            .iter()
            .filter(|n| n.alive() && n.map == map && n.pack == pack && n.archetype.hostile())
            .map(|n| n.id)
            .collect();
        for ally in survivors {
            self.npcs[ally]
                .memory
                .remember(self.turn, "player_killed_packmate", -0.4);
            self.apply_morale(ally);
            if self.npcs[ally].pos.distance(self.player.pos) <= 10 {
                self.request_decision(ally, 1);
            }
        }
        self.update_quests();
    }

    pub(crate) fn give_loot(&mut self, item: Item) {
        if self.player.inventory.len() < 20 {
            self.log(format!("Loot: {}.", item.name()));
            self.player.inventory.push(item);
        } else {
            self.player.gold += 12;
            self.log(format!("Pack full: {} exchanged for 12 gold.", item.name()));
        }
    }

    fn give_boss_relic(&mut self, relic: BossRelic) {
        if self.player.relic.is_none() {
            self.player.relic = Some(relic);
            self.log(format!(
                "Boss relic equipped: {} — {}",
                relic.name(),
                relic.describe()
            ));
        } else {
            if self.player.inventory.len() >= 20 {
                self.log("Your full pack strains, but a unique boss relic is never discarded.");
            }
            self.player.inventory.push(Item::BossRelic(relic));
            self.log(format!(
                "Boss relic secured: {} — {}",
                relic.name(),
                relic.describe()
            ));
        }
    }

    pub fn gain_xp(&mut self, amount: u32) {
        if self.player.level >= 12 {
            return;
        }
        self.player.xp += amount;
        while self.player.level < 12 {
            let needed = 25 + self.player.level * 15;
            if self.player.xp < needed {
                break;
            }
            self.player.xp -= needed;
            self.player.level += 1;
            self.player.max_hp += 6;
            self.player.hp = (self.player.hp + 8).min(self.player.max_hp);
            self.player.max_stamina += 1;
            self.player.stamina = self.player.max_stamina;
            if self.player.level.is_multiple_of(2) {
                self.player.attack += 1;
            }
            if self.player.level.is_multiple_of(3) {
                self.player.defense += 1;
            }
            if self.player.level == 5 || self.player.level == 9 {
                self.player.speed += 1;
            }
            // Talent income (§5, D34): one point per level L2–L10; sigils add the
            // other three. Levels 11–12 are mastery levels — growth, no point.
            if self.player.level <= 10 {
                self.player.talent_points += 1;
            }
            // D35: a sworn blade grows with its employer, not with its hiring day.
            if let Some(c) = self.companion {
                if self.npcs[c].archetype == Archetype::Companion {
                    let (hp, attack, defense, speed) =
                        crate::world::sellsword_stats(self.player.level);
                    let bonded = self.npcs[c]
                        .memory
                        .events
                        .iter()
                        .any(|e| e.kind == "bond_sworn");
                    let new_max = if bonded { hp * 5 / 4 } else { hp };
                    let lift = new_max - self.npcs[c].max_hp;
                    self.npcs[c].level = self.player.level;
                    self.npcs[c].max_hp = new_max;
                    self.npcs[c].hp = (self.npcs[c].hp + lift).clamp(0, new_max);
                    self.npcs[c].attack = attack;
                    self.npcs[c].defense = defense;
                    self.npcs[c].speed = speed;
                }
            }
            self.log(if self.player.level <= 10 {
                format!(
                    "Level {}! +6 maximum HP; strength and stamina grow. A talent point glimmers [T].",
                    self.player.level
                )
            } else {
                format!(
                    "Level {} — a mastery plateau: +6 maximum HP; the body remembers what the tree cannot teach.",
                    self.player.level
                )
            });
        }
        if self.player.level == 12 {
            self.player.xp = 0;
        }
    }

    /// Spend one talent point on a flattened branch-major node index (§5).
    /// Gates: player level, predecessor paid, cap not met, points in hand.
    pub fn learn_talent(&mut self, index: usize) {
        let index = index as u8;
        let Some(def) = talent_def(self.player.class, index) else {
            self.log("Swear an order first; the orders teach their own.");
            return;
        };
        if self.player.level < u32::from(def.level) {
            self.log(format!(
                "{} asks for level {}; you are level {}.",
                def.name, def.level, self.player.level
            ));
            return;
        }
        let tier = index % 4;
        if tier > 0 && self.player.talent_rank(index - 1) < talent_def(self.player.class, index - 1).map_or(1, |d| d.cost) {
            self.log("The deeper lesson wants the one before it first.");
            return;
        }
        if self.player.talent_rank(index) >= def.cost {
            self.log(format!("{} already holds.", def.name));
            return;
        }
        if self.player.talent_points_available() == 0 {
            self.log("No talent points yet — levels and sigils bring them.");
            return;
        }
        self.player.talents.push(index);
        // Stat-delta talents apply on payment and roll back exactly on respec.
        if self.player.class == Class::Keepwarden && index == 2 * 4 + 1 {
            // Vigilator 2: Discipline.
            self.player.max_stamina += 1;
            self.player.stamina += 1;
        }
        if self.player.class == Class::Waysworn && index == 6 {
            // Tollwright 3 (tier 1, slot 2): the Oathpath opens its books.
            for rep in self.player.reputation.iter_mut() {
                *rep = (*rep + 1).min(100);
            }
        }
        if self.player.talent_active(index) {
            self.log(format!("Learned {} — {}.", def.name, def.text));
        } else {
            self.log(format!(
                "One point into {}; it wants {} total.",
                def.name, def.cost
            ));
        }
    }

    /// The one free Oracle reset (D8): every point returns to the treasury.
    pub fn respec(&mut self) {
        let had = !self.player.talents.is_empty();
        for index in std::mem::take(&mut self.player.talents) {
            // Roll back the stat-delta talents the learn step applied.
            if self.player.class == Class::Keepwarden && index == 2 * 4 + 1 {
                self.player.max_stamina -= 1;
                self.player.stamina = self.player.stamina.min(self.player.max_stamina);
            }
            if self.player.class == Class::Waysworn && index == 6 {
                for rep in self.player.reputation.iter_mut() {
                    *rep = (*rep - 1).max(0);
                }
            }
        }
        self.player.talent_once = 0;
        if had {
            self.log(format!(
                "The Oracle unwinds your lessons. {} points return to your hand.",
                self.player.talent_points
            ));
        }
    }

    pub fn respawn(&mut self) {
        if self.player.hp > 0 {
            return;
        }
        let lost = self.player.gold / 4;
        self.player.gold -= lost;
        self.player.history.deaths += 1;
        self.player.hp = self.player.max_hp;
        self.player.stamina = self.player.max_stamina;
        self.player.mana = self.player.max_mana;
        self.player.defending = false;
        self.player.curse_until = 0;
        self.player.ward_until = 0;
        // Signature banks do not survive the crossing; the chosen stance does.
        self.player.class_state = ClassState {
            stance: self.player.class_state.stance,
            ..ClassState::default()
        };
        self.player.map = self.player.last_city.min(2) + 1;
        let center = Pos::new(7, 7);
        let map = self.map();
        let arrival = (0..map.height)
            .flat_map(|y| (0..map.width).map(move |x| Pos::new(x, y)))
            .filter(|p| map.tile(*p).walkable() && self.npc_at(*p).is_none())
            .min_by_key(|p| p.distance(center))
            .unwrap_or(center);
        self.maps[self.player.map].set_door_open(arrival, true);
        self.player.pos = arrival;
        self.combat = None;
        self.modal = Modal::None;
        self.effects.clear();
        self.outbox.clear();
        self.social_pending = None;
        self.gate_permit = None;
        self.haggle = None;
        for npc in &mut self.npcs {
            npc.decision_pending = false;
            npc.decision_version += 1;
            npc.perceived = false;
        }
        self.move_ready_ms = self.elapsed_ms;
        self.log(format!(
            "You awaken in {}. Lost {lost} gold; fallen enemies stay fallen.",
            self.map().name
        ));
        self.companion_follow();
        self.reveal();
    }

    fn pack_counts(&self, id: usize) -> (usize, usize) {
        let npc = &self.npcs[id];
        let mut alive = 0;
        let mut dead = 0;
        for ally in &self.npcs {
            let same_group = if npc.archetype.boss() {
                ally.pos.distance(npc.pos) <= 10
            } else {
                ally.pack == npc.pack
            };
            if ally.id != id && ally.map == npc.map && same_group && ally.archetype.hostile() {
                if ally.alive() {
                    alive += 1;
                } else {
                    dead += 1;
                }
            }
        }
        (alive, dead)
    }

    fn apply_morale(&mut self, id: usize) {
        if !self.npcs[id].alive() || self.npcs[id].archetype.boss() || spared(&self.npcs[id]) {
            return;
        }
        let (alive, dead) = self.pack_counts(id);
        let modifier = if dead > 0 && dead * 2 > alive + dead {
            0.2
        } else {
            0.0
        };
        let threshold = 0.45 + self.npcs[id].personality.brave * 0.45;
        if score(&self.npcs[id], "flee") + modifier >= threshold
            && self.npcs[id].intent != Intent::Flee
        {
            self.npcs[id].intent = Intent::Flee;
            self.npcs[id].tactic = if self.npcs[id].archetype == Archetype::Bandit
                && self.npcs[id].personality.spite > 0.65
            {
                "parting_javelin"
            } else {
                "retreat"
            }
            .into();
            self.log(format!(
                "{} breaks and flees{}!",
                self.npcs[id].name,
                if modifier > 0.0 {
                    " as its pack collapses"
                } else {
                    ""
                }
            ));
        }
    }

    pub fn request_decision(&mut self, id: usize, tier: u8) {
        if id >= self.npcs.len()
            || !self.npcs[id].alive()
            || self.npcs[id].map != self.player.map
            || self.npcs[id].decision_pending
        {
            return;
        }
        if self.npcs.iter().filter(|n| n.decision_pending).count() >= 8 {
            return;
        }
        let questions = crate::laya_client::question_bank(self.npcs[id].archetype, tier);
        if questions.is_empty() {
            return;
        }
        let (alive, dead) = self.pack_counts(id);
        let npc = &self.npcs[id];
        let witnesses = self
            .npcs
            .iter()
            .filter(|n| n.id != id && n.alive() && n.map == npc.map && n.pos.distance(npc.pos) <= 6)
            .count();
        let guards = self
            .npcs
            .iter()
            .filter(|n| {
                n.alive()
                    && n.map == npc.map
                    && n.archetype == Archetype::Guard
                    && n.pos.distance(npc.pos) <= 8
            })
            .count();
        let events: Vec<_> = npc
            .memory
            .events
            .iter()
            .rev()
            .take(3)
            .map(|e| json!({"turn":e.turn,"kind":e.kind,"weight":e.weight}))
            .collect();
        let city = self.city().unwrap_or(self.player.last_city).min(2);
        let mut state = json!({
            "npc":{"archetype":format!("{:?}",npc.archetype).to_lowercase(),"name":npc.name,"hp":npc.hp,"max_hp":npc.max_hp,"level":npc.level,"personality":npc.personality,"phase":npc.phase},
            "self":{"allies_alive":alive,"allies_dead":dead,"wounded":npc.hp*2<npc.max_hp},
            "player":{"hp":self.player.hp,"max_hp":self.player.max_hp,"level":self.player.level,"attack":self.attack_power(),"defense":self.defense_power(),"stamina":self.player.stamina,"visible":self.line_clear(npc.pos,self.player.pos),"distance":npc.pos.distance(self.player.pos),"looks_rich":self.player.gold>=100,"reputation":self.player.reputation[city]},
            "memory":{"disposition":npc.memory.disposition,"events":events},
            "environment":{"map":npc.map,"location":self.map().name,"time":self.hour(),"terrain":format!("{:?}",self.map().tile(npc.pos)).to_lowercase(),"guards_nearby":guards,"witnesses_nearby":witnesses},
            "combat":{"round":self.combat.as_ref().map_or(0,|c|c.round),"phase":npc.phase,"recent_actions":self.recent_actions}
        });
        if npc.archetype.boss() || tier == 4 {
            state["history"] = json!(self.player.history);
        }
        // Preserve representative recent memory while enforcing the latency budget.
        while serde_json::to_vec(&state).map_or(0, |v| v.len()) > 1500 {
            if let Some(events) = state["memory"]["events"].as_array_mut() {
                if !events.is_empty() {
                    events.pop();
                    continue;
                }
            }
            break;
        }
        self.npcs[id].decision_version += 1;
        self.npcs[id].decision_pending = true;
        self.npcs[id].last_decision = self.turn;
        self.outbox.push(DecisionRequest {
            seed: self.seed,
            npc: id,
            npc_name: self.npcs[id].name.clone(),
            version: self.npcs[id].decision_version,
            turn: self.turn,
            tier,
            state,
            questions,
        });
    }

    pub fn apply_decision(&mut self, result: &DecisionResult) -> String {
        let id = result.request.npc;
        let Some(npc) = self.npcs.get(id) else {
            return "discarded: unknown NPC".into();
        };
        if npc.decision_version != result.request.version {
            return "discarded: stale decision version".into();
        }
        self.npcs[id].decision_pending = false;
        if !self.npcs[id].alive()
            || self.npcs[id].map != self.player.map
            || result.request.state["environment"]["map"]
                .as_u64()
                .is_some_and(|map| map != self.player.map as u64)
        {
            return "discarded: dead or off-map NPC".into();
        }
        if spared(&self.npcs[id]) {
            return "discarded: NPC already spared".into();
        }
        for (key, answer) in &result.answers {
            if !answer.value.is_finite() {
                continue;
            }
            let mut answer = answer.clone();
            answer.value = answer.value.clamp(0.0, 1.0);
            self.npcs[id].answers.insert(key.clone(), answer);
        }
        let kind = self.npcs[id].archetype;
        if kind.boss() && score(&self.npcs[id], "aggro") > 0.5 {
            self.npcs[id].intent = Intent::Attack;
        }
        let mut rule = match kind {
            Archetype::Bandit => {
                if score(&self.npcs[id], "ambush_player") > 0.5 { self.npcs[id].intent = Intent::Attack; }
                else if self.combat.is_none() { self.npcs[id].intent = Intent::Idle; }
                "ambush >0.50; flee compared with bravery and pack morale"
            }
            Archetype::Wolf | Archetype::Bear => {
                // D5: the Packlord's truce is stronger than a hunting answer.
                if score(&self.npcs[id], "hunt_player") > 0.5 && !self.wolf_truce_holds(&self.npcs[id]) { self.npcs[id].intent = Intent::Attack; }
                "hunt >0.50; target selects player, pack support or easier prey"
            }
            Archetype::Rat | Archetype::Skeleton => {
                // Gnawbone Crown (§6): the warren's debts are paid.
                if !(kind == Archetype::Rat && self.player.relic == Some(BossRelic::GnawboneCrown))
                    && score(&self.npcs[id], "aggro") > 0.5 { self.npcs[id].intent = Intent::Attack; }
                "aggro >0.50; retreat_to_group >0.65 regroups before contact"
            }
            Archetype::Commoner => {
                if score(&self.npcs[id], "flee_from_player") > 0.6 { self.npcs[id].intent = Intent::Flee; }
                if score(&self.npcs[id], "alert_guards") > 0.65 { self.alert_neighbours(id); }
                "fear >0.60 flees; alert >0.65 spreads testimony to nearby guards"
            }
            Archetype::Guard => {
                if score(&self.npcs[id], "suspect_player") > 0.8 && self.npcs[id].memory.disposition < -0.3 { self.npcs[id].intent = Intent::Attack; }
                else if score(&self.npcs[id], "suspect_player") > 0.45 { self.npcs[id].intent = Intent::Help; }
                else if self.npcs[id].intent != Intent::Attack { self.npcs[id].intent = Intent::Patrol; }
                "suspect >0.80 plus hostile testimony pursues; otherwise guard patrol/protection"
            }
            Archetype::Thief => "steal >0.55 acts only 22:00–04:00; noticed thieves retreat",
            Archetype::Lich => {
                let tactic = choice(&self.npcs[id], "tactic").to_owned();
                if ["pressure", "summon", "curse", "retreat"].contains(&tactic.as_str()) { self.npcs[id].tactic = tactic; }
                if score(&self.npcs[id], "phase_shift") > 0.65 && self.npcs[id].hp * 3 < self.npcs[id].max_hp * 2 { self.npcs[id].phase = self.npcs[id].phase.max(2); }
                "tactic selects telegraphed pressure/summon/curse/retreat; wounded phase_shift >0.65"
            }
            Archetype::Adjudicator => {
                let tactic = choice(&self.npcs[id], "adapt_tactic").to_owned();
                if !tactic.is_empty() { self.npcs[id].tactic = tactic; }
                "mercy, greed and courage shape sentence; adapt_tactic changes phase attacks"
            }
            Archetype::Tidemother => {
                "undertow every 4th round drags all heroes to the water; drag_who marks the grip victim; wounded crews feed her"
            }
            Archetype::Cragmother => {
                "commit_slam >0.5 telegraphs the avalanche slam; wounded guard_cubs calls the roar; rubble chokes the den for the fight"
            }
            Archetype::GnawThane => {
                "call_the_tide >0.5 pours the brood every 3rd round; deep wounds scatter him between the holes"
            }
            Archetype::Tollmaster => {
                "collect_or_cut trades pressure for greed; shove_now >0.55 throws the crew's shoulders and sows caltrops"
            }
            Archetype::Mirelight => {
                "which_light marks the true glow each divide; strike_or_subside decides whether the lights commit"
            }
            Archetype::PaleStag => {
                "below half it breaks and breathes unless cornered; stand_ground >0.6 turns it at bay"
            }
            Archetype::OathlessCurate => {
                "each third of HP unwrites one sigil (which_sigil_falls); broken, mercy_for_the_oathbreaker decides the kneel"
            }
            Archetype::Chief => "rally >0.55 strengthens living minions; sacrifice >0.70 trades one minion for healing",
            Archetype::Matriarch => "howl >0.55 summons a capped pack; target_pick controls pack positioning",
            _ => "fresh social judgments cached for this interaction",
        }.to_string();
        self.apply_morale(id);
        if let Some(social) = self.apply_social_decision(result) {
            rule.push_str("; ");
            rule.push_str(&social);
        }
        // Applying an asynchronous answer never advances combat or deals damage.
        rule
    }

    fn alert_neighbours(&mut self, source: usize) {
        let pos = self.npcs[source].pos;
        let violent = self.npcs[source].memory.disposition < -0.3;
        let mut guards = Vec::new();
        for npc in &mut self.npcs {
            if npc.id == source
                || !npc.alive()
                || npc.map != self.player.map
                || npc.pos.distance(pos) > 7
            {
                continue;
            }
            match npc.archetype {
                Archetype::Commoner => {
                    npc.intent = Intent::Flee;
                    npc.memory.remember(
                        self.turn,
                        "heard_neighbour_alarm",
                        if violent { -0.4 } else { -0.1 },
                    );
                }
                Archetype::Guard => {
                    npc.memory.remember(
                        self.turn,
                        "heard_witness_testimony",
                        if violent { -0.5 } else { -0.1 },
                    );
                    guards.push(npc.id);
                }
                _ => {}
            }
        }
        self.log(format!(
            "{} raises an alarm; neighbours scatter and the watch takes notice.",
            self.npcs[source].name
        ));
        for id in guards {
            self.request_decision(id, 1);
        }
    }

    fn boss_turn(&mut self, id: usize) -> bool {
        if self.npcs[id].telegraph.is_some() && self.npcs[id].cooldown > 0 {
            self.npcs[id].cooldown -= 1;
            return true;
        }
        let kind = self.npcs[id].archetype;
        let name = self.npcs[id].name.clone();
        // A wind-up remembers its target and tactic. Later AI answers cannot move the warning.
        if let Some((target, tactic)) = self.npcs[id].telegraph.take() {
            self.npcs[id].cooldown = 2;
            match tactic.as_str() {
                "rally" => {
                    let pos = self.npcs[id].pos;
                    for ally in &mut self.npcs {
                        if ally.id != id
                            && ally.alive()
                            && ally.map == self.player.map
                            && ally.archetype.hostile()
                            && pos.distance(ally.pos) <= 10
                        {
                            ally.intent = Intent::Attack;
                            ally.tactic = "rallied".into();
                        }
                    }
                    self.log(format!(
                        "{name} rallies the warband: minions gain +1 attack."
                    ));
                }
                "sacrifice" => {
                    let victim = self
                        .npcs
                        .iter()
                        .find(|n| {
                            n.id != id
                                && n.alive()
                                && n.map == self.player.map
                                && n.archetype == Archetype::Bandit
                                && n.pos.distance(self.npcs[id].pos) <= 8
                        })
                        .map(|n| n.id);
                    if let Some(victim) = victim {
                        self.npcs[victim].hp = 0;
                        self.npcs[victim].decision_version += 1;
                        self.npcs[victim].decision_pending = false;
                        self.npcs[id].hp = (self.npcs[id].hp + 12).min(self.npcs[id].max_hp);
                        self.log(format!("{name} sacrifices a minion to recover 12 HP."));
                    } else {
                        self.log("The sacrifice fails: no minion remains.");
                    }
                }
                "summon" | "howl" => {
                    let count = self.summon_minions(
                        id,
                        if kind == Archetype::Matriarch {
                            Archetype::Wolf
                        } else {
                            Archetype::Skeleton
                        },
                    );
                    self.log(format!(
                        "{name} calls {count} reinforcements. The reserve is finite."
                    ));
                }
                "retreat" => {
                    self.step_away(id, self.player.pos);
                    self.npcs[id].hp = (self.npcs[id].hp + 5).min(self.npcs[id].max_hp);
                    self.log(format!("{name} falls back and recovers 5 HP."));
                }
                "endure" => {
                    self.player.stamina = self.player.stamina.saturating_sub(3).max(0);
                    self.log(format!(
                        "{name} tests your resolve: -3 stamina. Its guard leaves an opening."
                    ));
                }
                // Undertow (Tidemother, §6): heroes are dragged two tiles to the water; the
                // river itself is the progression check. Saltcrown roots the player.
                "undertow" => {
                    let saltcrown = self.player.relic == Some(BossRelic::Saltcrown);
                    // drag_who: Laya marks who the tide seizes by the legs for +4 grip damage.
                    let grip_companion = self.companion.is_some_and(|c| {
                        self.npcs[c].alive() && self.npcs[c].map == self.player.map
                    }) && choice(&self.npcs[id], "drag_who") == "companion";
                    let mut victims: Vec<(bool, usize, Pos)> =
                        vec![(true, usize::MAX, self.player.pos)];
                    if let Some(c) = self.companion {
                        if self.npcs[c].alive() && self.npcs[c].map == self.player.map {
                            victims.push((false, c, self.npcs[c].pos));
                        }
                    }
                    for (is_player, nid, hero_pos) in victims {
                        if is_player && saltcrown {
                            self.log("The Saltcrown holds you rooted while the tide slides off.");
                            continue;
                        }
                        let mut pos = hero_pos;
                        for _ in 0..2 {
                            let next = pos.offset(0, -1);
                            let tile = self.map().tile(next);
                            if self.npc_at(next).is_none()
                                && (matches!(tile, Tile::River) || tile.walkable())
                            {
                                pos = next;
                            } else {
                                break;
                            }
                        }
                        let soaked = matches!(self.map().tile(pos), Tile::River);
                        if is_player {
                            if soaked {
                                // Fallen in: the water takes its toll; you haul out at the dock edge.
                                self.hurt_player(10, "The undertow drags you under");
                                if self.player.hp > 0 {
                                    self.maps[self.player.map].set_door_open(hero_pos, true);
                                    self.player.pos = hero_pos;
                                }
                            } else {
                                self.maps[self.player.map].set_door_open(pos, true);
                                self.player.pos = pos;
                            }
                            if self.player.hp > 0 && !grip_companion && !saltcrown {
                                self.hurt_player(4, "The tide seizes your legs");
                            }
                        } else if soaked {
                            self.hurt_companion(nid, 8, id);
                            if self.npcs[nid].alive() {
                                self.maps[self.npcs[nid].map].set_door_open(hero_pos, true);
                                self.npcs[nid].pos = hero_pos;
                            }
                            if grip_companion {
                                self.hurt_companion(nid, 4, id);
                            }
                        } else {
                            self.maps[self.npcs[nid].map].set_door_open(pos, true);
                            self.npcs[nid].pos = pos;
                            if grip_companion {
                                self.hurt_companion(nid, 4, id);
                            }
                        }
                    }
                    self.log(format!("The tide rushes the boards under {name}!"));
                }
                // Avalanche Slam (Cragmother, §6): 3×3 impact; the struck ground stays rubble
                // for the rest of the fight, then the den is restored at combat end.
                "avalanche" => {
                    let pow = self.npcs[id].attack;
                    let center = target;
                    let caught = self.player.pos.distance(center) <= 1;
                    if caught {
                        let defense = if self.player.defending {
                            self.braced_defense()
                        } else {
                            self.defense_power()
                        };
                        self.hurt_player(
                            (pow + 2 - defense).max(1),
                            &format!("{name}'s avalanche slam"),
                        );
                    }
                    if let Some(c) = self.companion {
                        if self.npcs[c].alive()
                            && self.npcs[c].map == self.player.map
                            && self.npcs[c].pos.distance(center) <= 1
                        {
                            self.hurt_companion(c, 8, id);
                        }
                    }
                    let m = self.player.map;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let p = center.offset(dx, dy);
                            if matches!(self.maps[m].tile(p), Tile::Floor)
                                && p != self.player.pos
                                && p != self.npcs[id].pos
                                && !self.rubble.iter().any(|(rm, rp, _)| *rm == m && *rp == p)
                            {
                                let original = self.maps[m].tile(p);
                                self.rubble.push((m, p, original));
                                self.maps[m].set(p, Tile::Rock);
                            }
                        }
                    }
                    self.log(format!("{name} brings the hillside down: rubble chokes the floor!"));
                }
                // Her cubs answer from the nests — the reserve is finite, as with any howl.
                "cubs" => {
                    let count = self.summon_minions(id, Archetype::Bear);
                    self.log(format!("{name} roars for her young: {count} cubs answer!"));
                }
                // Plague Tide (Gnaw-Thane, §6): every surviving brood-hole spends
                // its strength; killing the holes cuts the tide off at the source.
                "plague_tide" => {
                    let pack = self.npcs[id].pack;
                    let map = self.npcs[id].map;
                    let mut poured = 0;
                    let holes: Vec<usize> = self
                        .npcs
                        .iter()
                        .filter(|n| {
                            n.archetype == Archetype::BroodHole
                                && n.alive()
                                && n.map == map
                                && n.pack == pack
                        })
                        .map(|n| n.id)
                        .collect();
                    for hole in holes {
                        if self.pack_rats_alive(pack) >= 5 {
                            break;
                        }
                        let pos = self.npcs[hole].pos;
                        poured += self.summon_from(pos, Archetype::Rat, pack, 2);
                    }
                    if poured > 0 {
                        self.log(format!(
                            "The brood-holes bulge: the Plague Tide pours {poured} fresh rats into the barrow!"
                        ));
                    } else {
                        self.log("The king shrieks for the tide, but the holes are spent.");
                    }
                }
                // Badly mauled, the Rat-King trades his bulk for the warren's cover.
                "scatter" => {
                    self.step_away(id, self.player.pos);
                    self.step_away(id, self.player.pos);
                    let pack = self.npcs[id].pack;
                    let map = self.npcs[id].map;
                    let mut poured = 0;
                    let holes: Vec<usize> = self
                        .npcs
                        .iter()
                        .filter(|n| {
                            n.archetype == Archetype::BroodHole
                                && n.alive()
                                && n.map == map
                                && n.pack == pack
                        })
                        .map(|n| n.id)
                        .collect();
                    for hole in holes {
                        if self.pack_rats_alive(pack) >= 5 {
                            break;
                        }
                        let pos = self.npcs[hole].pos;
                        poured += self.summon_from(pos, Archetype::Rat, pack, 2);
                    }
                    self.log(format!(
                        "{name} scatters squealing between the holes; the warren rises to cover him ({poured} rats pour out)."
                    ));
                }
                // Bridge Tax (Tollmaster, §6): the crew shoves two tiles toward the
                // river the camp is built against, then sows the ground with steel.
                "bridge_tax" => {
                    let saltcrown = self.player.relic == Some(BossRelic::Saltcrown);
                    let mut victims: Vec<(bool, usize, Pos)> =
                        vec![(true, usize::MAX, self.player.pos)];
                    if let Some(c) = self.companion {
                        if self.npcs[c].alive() && self.npcs[c].map == self.player.map {
                            victims.push((false, c, self.npcs[c].pos));
                        }
                    }
                    for (is_player, nid, from) in victims {
                        if is_player && saltcrown {
                            self.log("The Saltcrown holds you rooted against the crew's shoulders.");
                            continue;
                        }
                        let mut pos = from;
                        for _ in 0..2 {
                            let next = pos.offset(0, -1);
                            let tile = self.map().tile(next);
                            if self.npc_at(next).is_none()
                                && (matches!(tile, Tile::River) || tile.walkable())
                            {
                                pos = next;
                            } else {
                                break;
                            }
                        }
                        if is_player {
                            if matches!(self.map().tile(pos), Tile::River) {
                                self.hurt_player(
                                    8,
                                    "The Bridge Tax shoves you under the ford's foam",
                                );
                                if self.player.hp > 0 {
                                    self.maps[self.player.map].set_door_open(from, true);
                                    self.player.pos = from;
                                }
                            } else {
                                self.maps[self.player.map].set_door_open(pos, true);
                                self.player.pos = pos;
                            }
                        } else {
                            if matches!(self.map().tile(pos), Tile::River) {
                                self.hurt_companion(nid, 6, id);
                                if self.npcs[nid].alive() {
                                    self.maps[self.npcs[nid].map].set_door_open(from, true);
                                    self.npcs[nid].pos = from;
                                }
                            } else {
                                self.maps[self.npcs[nid].map].set_door_open(pos, true);
                                self.npcs[nid].pos = pos;
                            }
                        }
                    }
                    let m = self.player.map;
                    let mut sown = 0;
                    for _ in 0..10 {
                        if sown >= 2 {
                            break;
                        }
                        let (dx, dy) = DIRECTIONS[self.random(8) as usize];
                        let p = self.player.pos.offset(dx, dy);
                        if p == self.player.pos
                            || self.npc_at(p).is_some()
                            || !matches!(
                                self.maps[m].tile(p),
                                Tile::Floor | Tile::Road | Tile::Grass
                            )
                            || self.caltrops.iter().any(|(cm, cp, _)| *cm == m && *cp == p)
                        {
                            continue;
                        }
                        let original = self.maps[m].tile(p);
                        self.caltrops.push((m, p, original));
                        self.maps[m].set(p, Tile::Ruins);
                        sown += 1;
                    }
                    if sown > 0 {
                        self.log(format!(
                            "The crew roars and throws its shoulders — and {sown} fistfuls of caltrops scatter across the road."
                        ));
                    } else {
                        self.log("The crew roars and throws its shoulders: the Bridge Tax comes due!");
                    }
                }
                // Greed over pressure: the straps come off the caravan for a breath.
                "collect" => {
                    self.npcs[id].hp = (self.npcs[id].hp + 6).min(self.npcs[id].max_hp);
                    self.log(format!(
                        "{name} turns from you to the caravan's straps, counting coppers aloud. His greed buys you a breath."
                    ));
                }
                // Three False Lights (§6): the wisp and two borrowed glows. Laya's
                // tell (which_light) marks where the true light stands this phase.
                "divide" => {
                    let base = self.npcs[id].pos;
                    let tell = match choice(&self.npcs[id], "which_light") {
                        "west" => 0usize,
                        "east" => 2,
                        _ => 1,
                    };
                    let m = self.player.map;
                    for (index, pos) in [base.offset(-2, 0), base, base.offset(2, 0)]
                        .into_iter()
                        .enumerate()
                    {
                        if index == tell {
                            if pos == base
                                || (self.npc_at(pos).is_none()
                                    && self.maps[m].tile(pos).walkable())
                            {
                                self.maps[self.npcs[id].map].set_door_open(pos, true);
                                self.npcs[id].pos = pos;
                            }
                            continue;
                        }
                        if self.npc_at(pos).is_none() && self.maps[m].tile(pos).walkable() {
                            let glow_id = self.npcs.len();
                            let mut glow = crate::world::make_npc(
                                glow_id,
                                Archetype::FalseGlow,
                                m,
                                pos,
                                self.npcs[id].level,
                                self.npcs[id].pack,
                                self.seed,
                            );
                            glow.intent = Intent::Attack;
                            glow.name = "False light".into();
                            self.npcs.push(glow);
                        }
                    }
                    self.log(
                        "The wisp divides: three lights hover between the trees. Strike the wrong one and it drinks your blow.",
                    );
                }
                "subside" => {
                    self.log(
                        "The lights dim to embers and hold their places — nothing to strike but air.",
                    );
                }
                // Break (Pale Stag, §6): below half it runs and breathes — unless
                // cornered, or the Hartshorn is already on your shoulder.
                "break" => {
                    let hartshorn = self.player.relic == Some(BossRelic::Hartshorn);
                    let hard_bolt = score(&self.npcs[id], "break_and_run") > 0.5;
                    let mut ran = false;
                    if hard_bolt {
                        if let Some(gate) = self.map().portals.first().cloned() {
                            self.step_toward(id, gate.pos);
                            ran = true;
                        }
                    }
                    if !ran {
                        self.step_away(id, self.player.pos);
                    }
                    self.step_away(id, self.player.pos);
                    if hartshorn {
                        self.log(format!(
                            "{name} breaks for open ground — but the Hartshorn sings: no breath returns to it."
                        ));
                    } else {
                        self.npcs[id].hp = (self.npcs[id].hp + 5).min(self.npcs[id].max_hp);
                        self.log(format!(
                            "{name} breaks away and breathes: +5 HP returns. Corner it, or burst it down!"
                        ));
                    }
                }
                // Unwrit (§6.5): the named sigil's blessing and door-power die for
                // the rest of the fight. The warning was asked two rounds ago
                // (the phase judgment); now the words take.
                "unwrit" => {
                    let named = match choice(&self.npcs[id], "which_sigil_falls") {
                        "second" => 1usize,
                        "third" => 2,
                        _ => 0,
                    };
                    let candidate = if self.curate_unwritten[named] {
                        self.curate_unwritten.iter().position(|u| !*u)
                    } else {
                        Some(named)
                    };
                    if let Some(index) = candidate {
                        self.curate_unwritten[index] = true;
                        let sigil = ["Burrow", "Crimson", "Underkeep"][index];
                        self.log(format!(
                            "{name} lifts a page of the covenant and TEARS it. The {sigil} sigil's blessing dies in your hand — for the rest of this fight, it is paper."
                        ));
                        self.log(format!(
                            "({} of three now dark.)",
                            self.curate_unwritten.iter().filter(|u| **u).count()
                        ));
                    }
                }
                // The broken knee (§6.5): he folds his hands over the torn page.
                "kneel" => {
                    self.npcs[id].intent = Intent::Flee;
                    self.npcs[id].telegraph = None;
                    self.log(format!(
                        "{name}'s hands unfold. 'The covenant was an oath too proud to keep. I have paid it twice; will a third payment satisfy you?' (C to show mercy — or end it.)"
                    ));
                }
                _ => {
                    if self.player.pos.distance(target) <= 1
                        && self.line_clear(self.npcs[id].pos, self.player.pos)
                    {
                        if tactic == "curse" {
                            // Unyielding (Vigilator 4): the garrison refuses one hex per fight.
                            if self.player.class == Class::Keepwarden
                                && self.player.has_talent(2, 3)
                                && self.player.talent_once & once::UNYIELDING == 0
                            {
                                self.player.talent_once |= once::UNYIELDING;
                                self.log("The curse breaks on the oath. Unyielding holds.");
                            } else {
                                self.player.curse_until = self.turn + 4;
                                self.log("The curse catches you: -2 defense for four turns.");
                            }
                        } else {
                            let modifier = if kind == Archetype::Adjudicator {
                                if tactic == "duel" {
                                    -(score(&self.npcs[id], "rate_mercy") * 4.0) as i32
                                } else if tactic == "deceive" {
                                    (score(&self.npcs[id], "rate_greed") * 3.0) as i32
                                } else {
                                    (score(&self.npcs[id], "rate_courage") * 3.0) as i32
                                }
                            } else {
                                2
                            };
                            let defense = if self.player.defending {
                                self.braced_defense()
                            } else {
                                self.defense_power()
                            };
                            self.hurt_player(
                                (self.npcs[id].attack + modifier - defense).max(1),
                                &format!("{name}'s {tactic}"),
                            );
                            if tactic == "deceive" {
                                self.player.stamina = (self.player.stamina - 2).max(0);
                            }
                        }
                    } else {
                        self.log(format!(
                            "You evade {name}'s {tactic}. Strike while it recovers!"
                        ));
                    }
                }
            }
            return true;
        }
        if self.npcs[id].cooldown > 0 {
            self.npcs[id].cooldown -= 1;
            return false;
        }
        if self.npcs[id].pos.distance(self.player.pos) > 4 {
            return false;
        }
        let (allies, _) = self.pack_counts(id);
        let tactic = match kind {
            Archetype::Chief if allies > 0 && score(&self.npcs[id], "sacrifice_minion") > 0.7 => {
                "sacrifice"
            }
            Archetype::Chief if allies > 0 && score(&self.npcs[id], "rally_minions") > 0.55 => {
                "rally"
            }
            Archetype::Chief => "cleave",
            Archetype::Matriarch
                if score(&self.npcs[id], "howl_summon") > 0.55 && self.can_summon(id) =>
            {
                "howl"
            }
            Archetype::Matriarch => "pounce",
            Archetype::Lich => match self.npcs[id].tactic.as_str() {
                "summon" if self.can_summon(id) => "summon",
                "curse" => "curse",
                "retreat" => "retreat",
                _ => "pressure",
            },
            Archetype::Adjudicator => match self.npcs[id].tactic.as_str() {
                "punish" => "punish",
                "deceive" => "deceive",
                "endure" => "endure",
                _ => "duel",
            },
            // Undertow (§6): the water takes the fight every fourth round, capped crew the rest.
            Archetype::Tidemother
                if self.combat.as_ref().is_some_and(|c| c.round % 4 == 0) =>
            {
                "undertow"
            }
            Archetype::Tidemother
                if allies > 0 && score(&self.npcs[id], "sacrifice_crew") > 0.65 =>
            {
                "sacrifice"
            }
            Archetype::Tidemother => "drag",
            // The den: cubs come in phase two if she judges it; the slam staggers ground.
            Archetype::Cragmother
                if self.npcs[id].phase >= 2
                    && score(&self.npcs[id], "guard_cubs") > 0.5
                    && self.can_summon(id) =>
            {
                "cubs"
            }
            Archetype::Cragmother if score(&self.npcs[id], "commit_slam") > 0.5 => "avalanche",
            Archetype::Cragmother => "maul",
            // Gnaw-Thane (§6): the tide crests every third round while his holes
            // hold; thinned and dying, he scatters into his warren's cover.
            Archetype::GnawThane
                if self.combat.as_ref().is_some_and(|c| c.round % 3 == 0)
                    && self.living_holes(id) > 0
                    && score(&self.npcs[id], "call_the_tide") > 0.5 =>
            {
                "plague_tide"
            }
            Archetype::GnawThane
                if self.npcs[id].hp * 5 < self.npcs[id].max_hp * 2
                    && self.living_holes(id) > 0
                    && score(&self.npcs[id], "scatter_when_thinned") > 0.5 =>
            {
                "scatter"
            }
            Archetype::GnawThane => "gnaw",
            // Tollmaster (§6): greed and pressure trade place; the tax lands when
            // the crew's shoulders are willing.
            Archetype::Tollmaster if choice(&self.npcs[id], "collect_or_cut") == "loot" => {
                "collect"
            }
            Archetype::Tollmaster
                if self.combat.as_ref().is_some_and(|c| c.round % 3 == 1)
                    && score(&self.npcs[id], "shove_now") > 0.55 =>
            {
                "bridge_tax"
            }
            Archetype::Tollmaster => "tax",
            // Mirelight (§6): once split, the lights hold or strike together; the
            // true glow reforms when her borrowed lights are gone.
            Archetype::Mirelight
                if !self.has_living_glow(id)
                    && self.combat.as_ref().is_some_and(|c| c.round % 4 == 2) =>
            {
                "divide"
            }
            Archetype::Mirelight
                if self.has_living_glow(id)
                    && score(&self.npcs[id], "strike_or_subside") <= 0.5 =>
            {
                "subside"
            }
            Archetype::Mirelight => "glow-strike",
            // The Pale Stag (§6): the first boss whose retreat is the enemy's move —
            // at half health it breaks unless the rocks or its own judgment say stand.
            Archetype::PaleStag
                if self.npcs[id].hp * 2 < self.npcs[id].max_hp
                    && (self.cornered(id) || score(&self.npcs[id], "stand_ground") > 0.6) =>
            {
                "gore"
            }
            Archetype::PaleStag if self.npcs[id].hp * 2 < self.npcs[id].max_hp => "break",
            Archetype::PaleStag => "gore",
            // The Oathless Curate (§6.5): each third of his HP, another sigil is
            // unwritten — which_sigil_falls is asked at the phase, spent at fire.
            // Near the end, a broken kneel asks whether this traveller can spare him.
            Archetype::OathlessCurate
                if self.npcs[id].hp * 5 < self.npcs[id].max_hp
                    && !self.mercy_oathbreaker
                    && self.npcs[id].intent != Intent::Flee
                    && score(&self.npcs[id], "mercy_for_the_oathbreaker") > 0.5 =>
            {
                "kneel"
            }
            Archetype::OathlessCurate
                if (self.npcs[id].phase as usize) - 1
                    > self.curate_unwritten.iter().filter(|u| **u).count()
                    && !choice(&self.npcs[id], "which_sigil_falls").is_empty() =>
            {
                "unwrit"
            }
            Archetype::OathlessCurate => "writ",
            _ => return false,
        }
        .to_string();
        self.npcs[id].telegraph = Some((self.player.pos, tactic.clone()));
        self.npcs[id].cooldown = 1;
        if tactic == "undertow" {
            self.log(format!(
                "{name} calls the tide under the boards! Two actions until it takes everything standing."
            ));
        } else if tactic == "avalanche" {
            self.log(format!(
                "{name} rears for the Avalanche Slam! Two actions to leave the marked ground."
            ));
        } else {
            self.log(format!(
                "{name} prepares {tactic}! You have two actions to leave the red 3x3 area or defend."
            ));
        }
        true
    }

    /// Fester (D36): barrow bites stack; every third stack takes 1 max stamina.
    /// Inn beds and anti-toxin purge it whole (§6: "cured at inn").
    fn infect_fester(&mut self) {
        if self.turn < self.player.fester_guard_until {
            self.log("The anti-toxin's afterglow turns the bite's fever aside.");
            return;
        }
        if self.player.fester >= 12 {
            return;
        }
        self.player.fester += 1;
        if self.player.fester.is_multiple_of(3) {
            self.player.max_stamina = (self.player.max_stamina - 1).max(1);
            self.player.stamina = self.player.stamina.min(self.player.max_stamina);
            self.log(format!(
                "The bite festers: -1 max stamina ({} stacks). An inn or anti-toxin cleanses it.",
                self.player.fester
            ));
        } else {
            self.log(format!(
                "The bite burns with fen-fever (fester {}; every third stack saps your wind).",
                self.player.fester
            ));
        }
    }

    /// Purge every fester stack and hand back the stamina it took. True if any.
    pub fn cure_fester(&mut self) -> bool {
        if self.player.fester == 0 {
            return false;
        }
        let stacks = self.player.fester;
        self.player.max_stamina += i32::from(stacks / 3);
        self.player.max_stamina = self.player.max_stamina.max(1);
        self.player.stamina = self.player.stamina.min(self.player.max_stamina);
        self.player.fester = 0;
        self.log("The fever breaks; your wind returns in full.");
        true
    }

    /// Bond-wolf truce (D5, §4.2): once a Fensworn holds the Packlord's truce,
    /// the pack knows their name and no unprovoked wolf turns on them.
    pub fn wolf_truce_holds(&self, npc: &Npc) -> bool {
        npc.archetype == Archetype::Wolf
            && self.player.class == Class::Fensworn
            && self.player.has_talent(0, 3)
            && !npc
                .memory
                .events
                .iter()
                .any(|e| e.kind == "player_attacked_me")
    }

    /// E5: living brood-holes of this boss's warren party on its map.
    fn living_holes(&self, id: usize) -> usize {
        let (map, pack) = (self.npcs[id].map, self.npcs[id].pack);
        self.npcs
            .iter()
            .filter(|n| {
                n.archetype == Archetype::BroodHole
                    && n.alive()
                    && n.map == map
                    && n.pack == pack
            })
            .count()
    }

    fn pack_rats_alive(&self, pack: usize) -> usize {
        self.npcs
            .iter()
            .filter(|n| {
                n.archetype == Archetype::Rat
                    && n.alive()
                    && n.map == self.player.map
                    && n.pack == pack
            })
            .count()
    }

    /// Spawn one minion beside a fixed feeder point (a brood-hole), if any ring
    /// cell is free. Returns 1 when one crawls out.
    fn summon_from(&mut self, pos: Pos, kind: Archetype, pack: usize, level: u32) -> usize {
        for (dx, dy) in DIRECTIONS {
            let next = pos.offset(dx, dy);
            if next != self.player.pos
                && self.map().can_step(pos, next)
                && self.npc_at(next).is_none()
            {
                let mut npc = crate::world::make_npc(
                    self.npcs.len(),
                    kind,
                    self.player.map,
                    next,
                    level,
                    pack,
                    self.seed,
                );
                npc.intent = Intent::Attack;
                self.npcs.push(npc);
                return 1;
            }
        }
        0
    }

    /// E5: any of the wisp's borrowed lights still hovering?
    fn has_living_glow(&self, id: usize) -> bool {
        let (map, pack) = (self.npcs[id].map, self.npcs[id].pack);
        self.npcs.iter().any(|n| {
            n.archetype == Archetype::FalseGlow && n.alive() && n.map == map && n.pack == pack
        })
    }

    /// The Pale Stag's stand (§6): no step that opens air counts as a corner.
    fn cornered(&self, id: usize) -> bool {
        let pos = self.npcs[id].pos;
        let threat = self.player.pos;
        !DIRECTIONS.iter().any(|(dx, dy)| {
            let next = pos.offset(*dx, *dy);
            next.distance(threat) > pos.distance(threat)
                && self.map().can_step(pos, next)
                && next != threat
                && self.npc_at(next).is_none()
        })
    }

    /// E7 (§8.2): seeded one-shot scrounge feeds the alchemist — herb clusters
    /// in forest light, ore flakes on the ridge. Each site gives exactly once.
    fn scrounge_check(&mut self) {
        if self.player.map != 0 {
            return;
        }
        let pos = self.player.pos;
        let Some(index) = self
            .scrounge_sites
            .iter()
            .position(|(p, _)| *p == pos)
        else {
            return;
        };
        if self.scrounged.contains(&(0, pos)) {
            return;
        }
        let kind = self.scrounge_sites[index].1;
        let item = if kind == 0 {
            Item::HerbCluster
        } else {
            Item::OreFlake
        };
        if self.player.inventory.len() >= 20 {
            self.log(format!("Your pack is full; the {} stays for another trip.", item.name()));
            return;
        }
        self.scrounged.push((0, pos));
        self.player.inventory.push(item.clone());
        self.log(if kind == 0 {
            format!("An herb cluster bends under the boot-falls: gathered {}.", item.name())
        } else {
            format!("Ridge iron flakes off the stone in your hand: gathered {}.", item.name())
        });
    }

    /// The Tollmaster's planted steel (§6): bites once, then the ground is honest.
    fn caltrop_check(&mut self) {
        let m = self.player.map;
        let p = self.player.pos;
        if let Some(i) = self
            .caltrops
            .iter()
            .position(|(cm, cp, _)| *cm == m && *cp == p)
        {
            let (_, _, original) = self.caltrops.remove(i);
            self.maps[m].set(p, original);
            self.player.stamina = (self.player.stamina - 1).max(0);
            self.hurt_player(2, "The crew's sown caltrops");
            self.log("Caltrops bite through your boot: -2 HP, -1 stamina.");
        }
    }

    fn can_summon(&self, id: usize) -> bool {
        let pack = self.npcs[id].pack;
        let minions = self
            .npcs
            .iter()
            .filter(|n| n.pack == pack && n.id != id && !n.archetype.boss());
        minions.clone().count() < 6 && minions.filter(|n| n.alive()).count() < 3
    }

    fn summon_minions(&mut self, id: usize, kind: Archetype) -> usize {
        let mut summoned = 0;
        for (dx, dy) in DIRECTIONS {
            if summoned >= 2 || !self.can_summon(id) {
                break;
            }
            let pos = self.npcs[id].pos.offset(dx, dy);
            if pos == self.player.pos
                || !self.map().can_step(self.npcs[id].pos, pos)
                || self.npc_at(pos).is_some()
            {
                continue;
            }
            let mut npc = crate::world::make_npc(
                self.npcs.len(),
                kind,
                self.player.map,
                pos,
                self.npcs[id].level.saturating_sub(2).max(1),
                self.npcs[id].pack,
                self.seed,
            );
            npc.intent = Intent::Attack;
            self.npcs.push(npc);
            summoned += 1;
        }
        summoned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fight(kind: Archetype) -> Game {
        let mut game = Game::new(42);
        game.maps[0] = Map::new("Test arena", MapKind::Arena, 30, 30, Tile::Floor);
        game.player.map = 0;
        game.player.pos = Pos::new(10, 10);
        game.modal = Modal::None;
        game.npcs = vec![crate::world::make_npc(
            0,
            kind,
            0,
            Pos::new(11, 10),
            2,
            1,
            42,
        )];
        game.npcs[0].intent = Intent::Attack;
        game.refresh_combat();
        game
    }

    fn door_game() -> Game {
        let mut game = fight(Archetype::Rat);
        game.npcs.clear();
        game.combat = None;
        game.maps[0].set(Pos::new(11, 10), Tile::Door);
        game
    }

    #[test]
    fn door_passage_opens_in_one_step_and_only_clear_thresholds_close() {
        let mut game = door_game();
        let door = Pos::new(11, 10);
        game.action(Action::Move(1, 0));
        assert_eq!(game.player.pos, door);
        assert!(game.map().door_open(door));
        game.action(Action::Interact);
        assert!(game.map().door_open(door), "cannot shut on the player");
        game.move_ready_ms = 0;
        game.action(Action::Move(1, 0));
        game.action(Action::Interact);
        assert!(!game.map().door_open(door));
        game.action(Action::Interact);
        assert!(game.map().door_open(door));
        game.action(Action::Interact);
        assert!(!game.map().door_open(door));
        game.move_ready_ms = 0;
        game.action(Action::Move(-1, 0));
        assert_eq!(game.player.pos, door, "closing never strands the player");
        assert!(game.map().door_open(door));
    }

    #[test]
    fn door_npc_passage_opens_and_occupied_threshold_cannot_close() {
        let mut game = door_game();
        let door = Pos::new(11, 10);
        game.npcs.push(crate::world::make_npc(
            0, Archetype::Rat, 0, Pos::new(12, 10), 2, 1, 42,
        ));
        assert!(game.try_npc_step(0, door));
        assert!(game.map().door_open(door));
        game.action(Action::Interact);
        assert!(game.map().door_open(door), "cannot shut on another actor");
        assert!(game.try_npc_step(0, Pos::new(12, 10)));
        game.action(Action::Interact);
        assert!(!game.map().door_open(door));
    }

    #[test]
    fn door_states_roundtrip_and_legacy_maps_load_closed() {
        let mut game = door_game();
        let door = Pos::new(11, 10);
        game.maps[0].set_door_open(door, true);
        let json = serde_json::to_string(&game).unwrap();
        let loaded: Game = serde_json::from_str(&json).unwrap();
        assert!(loaded.maps[0].door_open(door));
        assert!(loaded.maps[0].can_step(Pos::new(10, 10), door));
        let mut old = serde_json::to_value(&game.maps[0]).unwrap();
        old.as_object_mut().unwrap().remove("open_doors");
        let legacy: Map = serde_json::from_value(old).unwrap();
        assert!(!legacy.door_open(door));
        assert!(legacy.can_step(Pos::new(10, 10), door));
    }

    #[test]
    fn directional_strike_never_hits_a_replacement_after_faster_turns() {
        let mut game = fight(Archetype::Rat);
        game.npcs[0].hp = 8;
        game.npcs[0].max_hp = 100;
        game.npcs[0].speed = game.player_speed() + 2;
        game.npcs[0].answers.insert("target_pick".into(), answer(1.0, Some("cattle")));
        let mut replacement = crate::world::make_npc(
            1, Archetype::Rat, 0, Pos::new(12, 10), 2, 1, 42,
        );
        replacement.speed = game.player_speed() + 1;
        replacement.intent = Intent::Attack;
        let replacement_hp = replacement.hp;
        game.npcs.push(replacement);
        game.refresh_combat();
        game.action(Action::Move(1, 0));
        assert_eq!(game.turn, 1);
        assert_eq!(game.npcs[1].pos, Pos::new(11, 10));
        assert_eq!(game.npcs[1].hp, replacement_hp);
        assert_eq!(game.player.history.attacks, 0, "the original target left melee reach");
    }

    #[test]
    fn wall_and_blocked_diagonal_never_spend_combat_resources() {
        let mut game = fight(Archetype::Rat);
        game.player.defending = true;
        game.maps[0].set(Pos::new(9, 10), Tile::Wall);
        let hp = game.player.hp;
        let stamina = game.player.stamina;
        for action in [Action::Move(-1, 0), Action::Move(-1, -1)] {
            game.action(action);
        }
        assert_eq!(game.turn, 0);
        assert_eq!(game.combat.as_ref().unwrap().round, 0);
        assert_eq!(game.player.hp, hp);
        assert_eq!(game.player.stamina, stamina);
        assert!(game.player.defending);
    }

    #[test]
    fn upgraded_combat_consumables_spend_only_useful_rounds() {
        let mut game = fight(Archetype::BroodHole);
        game.player.inventory = vec![Item::GreaterPotion, Item::TravelerRation];
        game.action(Action::Use(0));
        assert_eq!(game.turn, 0);
        assert_eq!(game.player.inventory[0], Item::GreaterPotion);
        game.player.hp -= 10;
        game.action(Action::Use(0));
        assert_eq!(game.turn, 1);
        assert_eq!(game.player.hp, game.player.max_hp);
        assert_eq!(game.player.inventory, vec![Item::TravelerRation]);
        game.action(Action::Use(0));
        assert_eq!(game.turn, 1);
        game.player.stamina -= 4;
        game.action(Action::Use(0));
        assert_eq!(game.turn, 2);
        assert_eq!(game.player.stamina, game.player.max_stamina);
        assert!(game.player.inventory.is_empty());
    }

    #[test]
    fn combat_waits_for_valid_actions_even_with_inventory_open() {
        let mut game = fight(Archetype::Rat);
        let hp = game.player.hp;
        for _ in 0..40 {
            game.tick_world();
        }
        assert_eq!(game.player.hp, hp);
        assert_eq!(game.turn, 0);
        game.maps[0].set(Pos::new(9, 10), Tile::Wall);
        game.action(Action::Move(-1, 0));
        game.modal = Modal::Inventory;
        game.action(Action::Use(0)); // Potion at full health must not burn a turn.
        assert_eq!(game.turn, 0);
        assert_eq!(game.player.inventory.len(), 7);
        game.modal = Modal::None;
        game.action(Action::Defend);
        assert_eq!(game.turn, 1);
        assert!(game.player.hp < hp);
    }

    #[test]
    fn faster_lethal_enemy_prevents_player_attack_and_respawn_preserves_world() {
        let mut game = fight(Archetype::Wolf);
        game.npcs[0].attack = 100;
        game.player.gold = 100;
        let enemy_hp = game.npcs[0].hp;
        game.action(Action::Attack);
        assert!(matches!(game.modal, Modal::Death));
        assert_eq!(game.npcs[0].hp, enemy_hp);
        game.respawn();
        assert_eq!(game.player.gold, 75);
        assert_eq!(game.player.hp, game.player.max_hp);
        assert_eq!(game.npcs[0].hp, enemy_hp);
        assert_eq!(game.player.map, 1);
    }

    #[test]
    fn delayed_answers_cannot_retarget_a_wound_up_boss_or_apply_after_death() {
        let mut game = fight(Archetype::Lich);
        game.npcs[0].tactic = "pressure".into();
        assert!(game.boss_turn(0));
        let old_target = game.player.pos;
        game.npcs[0].tactic = "curse".into();
        game.player.pos = old_target.offset(-2, 0);
        let hp = game.player.hp;
        assert!(game.boss_turn(0));
        assert!(game.boss_turn(0)); // Resolve the two-action warning, not merely its wind-up.
        assert_eq!(game.player.hp, hp);
        assert_eq!(game.player.curse_until, 0);
        let request = game.outbox.pop().unwrap();
        game.kill_npc(0);
        let result = crate::laya_client::heuristic(&request);
        assert!(game.apply_decision(&result).starts_with("discarded"));
        assert!(!game.npcs[0].alive());
        assert!(game.player.sigils[2]);
    }

    #[test]
    fn flanking_requires_a_living_opposite_attacker_and_open_escape_tile() {
        let mut game = fight(Archetype::Bandit);
        let mut second = crate::world::make_npc(1, Archetype::Bandit, 0, Pos::new(9, 10), 2, 1, 42);
        second.intent = Intent::Attack;
        game.npcs.push(second);
        assert!(game.flanked(0));
        game.npcs[0].attack = 10;
        game.player.defense = 6;
        let hp = game.player.hp;
        game.enemy_turn(0);
        assert_eq!(
            game.player.hp,
            hp - 5,
            "flanking adds 25% to damage after armor, not to attack"
        );
        game.maps[0].set(Pos::new(9, 10), Tile::Wall);
        assert!(!game.flanked(0));
        game.maps[0].set(Pos::new(9, 10), Tile::Floor);
        game.npcs[1].hp = 0;
        assert!(!game.flanked(0));
    }
    #[test]
    fn fleeing_npcs_leave_while_the_remaining_enemies_fight() {
        let mut game = fight(Archetype::Rat);
        let mut runner = crate::world::make_npc(1, Archetype::Bandit, 0, Pos::new(9, 10), 2, 2, 42);
        runner.intent = Intent::Flee;
        runner.tactic = "retreat".into();
        game.npcs.push(runner);
        game.action(Action::Defend);
        assert!(game.combat.is_some());
        assert!(game.npcs[1].pos.distance(game.player.pos) > 1);
    }

    #[test]
    fn spark_consumes_mana_and_strikes_past_armour() {
        let mut game = fight(Archetype::Rat);
        game.player.spells = vec![Spell::Spark, Spell::Mend, Spell::Ward];
        game.player.max_mana = 8;
        game.player.mana = 8;
        game.npcs[0].defense = 50; // armour is ignored by the bolt
        let hp = game.npcs[0].hp;
        game.action(Action::Cast(0));
        // 2 mana spent, 1 returned by round-end regen.
        assert_eq!(game.player.mana, 7);
        assert!(game.npcs[0].hp < hp, "bolt must land despite heavy armour");
        game.player.mana = 0;
        let turn = game.turn;
        game.action(Action::Cast(0));
        assert_eq!(turn, game.turn, "no mana means no turn is spent");
    }

    #[test]
    fn boss_relics_drop_and_apply_their_three_passives() {
        let mut game = fight(Archetype::Chief);
        game.kill_npc(0);
        assert_eq!(game.player.relic, Some(BossRelic::Rallybreaker));

        let mut game = fight(Archetype::Bandit);
        game.player.relic = Some(BossRelic::Rallybreaker);
        game.npcs[0].speed = 0;
        game.npcs[0].hp = 100;
        game.npcs[0].max_hp = 100;
        game.npcs[0].defense = 0;
        game.action(Action::Defend);
        let hp = game.npcs[0].hp;
        game.action(Action::Attack);
        assert_eq!(hp - game.npcs[0].hp, game.attack_power() + 2);

        game.player.relic = Some(BossRelic::Fangmantle);
        let base = game.player.defense + i32::from(game.player.armour) * 2;
        let mut second = crate::world::make_npc(1, Archetype::Wolf, 0, Pos::new(9, 10), 2, 2, 42);
        second.intent = Intent::Attack;
        game.npcs.push(second);
        assert_eq!(game.defense_power(), base + 1);

        game.player.relic = Some(BossRelic::Graveglass);
        assert_eq!(game.spell_cost(Spell::Spark), 1);
        assert_eq!(game.spell_cost(Spell::Ward), 2);
    }

    #[test]
    fn mend_and_ward_work_through_the_action_channel() {
        let mut game = fight(Archetype::Rat);
        game.player.spells = vec![Spell::Mend, Spell::Ward];
        game.player.max_mana = 6;
        game.player.mana = 6;
        game.player.hp = 5;
        let defense = game.defense_power();
        game.action(Action::Cast(1));
        assert_eq!(game.player.ward_until, game.turn + 2);
        assert_eq!(game.defense_power(), defense + 2);
        assert!(game.player.defending);
        game.action(Action::Wait);
        // 3 spent on the ward, one round of regen each for the cast and the wait.
        assert_eq!(game.player.mana, 5);
        game.kill_npc(0); // clear the field so the heal is measured cleanly
        let hurt = game.player.hp;
        game.action(Action::Cast(0));
        assert_eq!(game.player.hp, hurt + 12, "mend restores 12 HP");
    }

    #[test]
    fn companion_fights_soaks_blows_and_rises_after_combat() {
        let mut game = fight(Archetype::Bandit);
        let blade = game.npcs.len();
        let mut sellsword =
            crate::world::make_npc(blade, Archetype::Companion, 0, Pos::new(10, 11), 3, 900, 42);
        sellsword.attack = 50;
        game.npcs.push(sellsword);
        game.companion = Some(blade);
        game.action(Action::Defend);
        assert!(
            !game.npcs[0].alive(),
            "the blade strikes right after the player's action"
        );
        assert!(
            game.combat.is_none(),
            "no foes remain once the bandit falls"
        );
        let hp = game.npcs[blade].hp;
        let wolf = game.npcs.len();
        let mut wolf_npc =
            crate::world::make_npc(wolf, Archetype::Wolf, 0, Pos::new(10, 12), 4, 901, 42);
        wolf_npc.intent = Intent::Attack;
        game.npcs.push(wolf_npc);
        game.enemy_turn(wolf);
        assert_eq!(
            game.npcs[blade].hp,
            hp - (game.npcs[wolf].attack - game.npcs[blade].defense).max(1),
            "a bodyguard in reach draws the blow"
        );
        game.npcs[blade].hp = 0;
        game.combat = Some(Combat {
            center: game.player.pos,
            round: 1,
            participants: vec![wolf],
        });
        game.kill_npc(wolf);
        game.refresh_combat();
        assert!(game.combat.is_none());
        assert!(game.npcs[blade].hp > 0, "the blade rises when combat ends");
    }

    #[test]
    fn braced_status_survives_until_the_next_action() {
        let mut game = fight(Archetype::Bandit);
        game.action(Action::Defend);
        assert!(
            game.player.defending,
            "the brace must remain visible until the player acts again"
        );
        game.action(Action::Wait);
        assert!(!game.player.defending);
    }

    #[test]
    fn torch_clears_fog_only_within_its_own_radius_at_night() {
        let mut game = Game::new(42);
        game.player.pos = Pos::new(30, 30); // fresh fog, far from the daytime spawn reveal
        game.hour_ticks = 480 * 14; // 22:00: deep night.
        assert!(game.night());
        let map = game.player.map;
        let near = game.maps[map].index(game.player.pos.offset(5, 0)).unwrap();
        let far = game.maps[map].index(game.player.pos.offset(8, 0)).unwrap();
        game.reveal();
        assert!(!game.maps[map].explored[near]);
        assert!(!game.maps[map].explored[far]);
        let torch = game
            .player
            .inventory
            .iter()
            .position(|i| *i == Item::Torch)
            .unwrap();
        assert!(game.use_item(torch));
        game.reveal();
        assert!(game.maps[map].explored[near]);
        assert!(
            !game.maps[map].explored[far],
            "torchlight must not clear fog at the daylight radius"
        );
        game.hour_ticks = 480 * 4; // back to daylight.
        game.reveal();
        assert!(game.maps[map].explored[far]);
    }

    #[test]
    fn creation_applies_each_orders_sheet_and_the_boon() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Keepwarden, Build::Male, Boon::Gold);
        assert_eq!(game.player.class, Class::Keepwarden);
        assert_eq!(game.player.max_hp, 24);
        assert_eq!(game.player.max_mana, 2, "D7 floor: the oath keeps embers");
        assert_eq!(game.player.gold, 60);
        assert!(matches!(game.modal, Modal::None));

        let mut game = Game::new(42);
        game.apply_creation(Class::SigilSworn, Build::Male, Boon::Stamina);
        assert_eq!(game.player.max_hp, 17);
        assert_eq!(game.player.max_mana, 4);
        assert_eq!(game.player.spells, vec![Spell::Spark]);
        assert_eq!(game.player.max_stamina, 13);

        let mut game = Game::new(42);
        game.apply_creation(Class::Redwake, Build::Male, Boon::None);
        assert_eq!(game.player.speed, 12);
        assert_eq!(game.player.defense, 0);
        assert_eq!(game.player.gold, 40);
    }

    /// The creation flow is three beats - order, body frame, boon. A frame that
    /// never lands on the player, or a step that falls through, is a silent
    /// "my character ignored me" bug, so lock the whole path here.
    #[test]
    fn creation_persists_the_chosen_body_build() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

        let mut game = Game::new(42);
        game.modal = Modal::Create;
        game.create_step = 0;
        game.selected = 0;

        let press = |game: &mut Game, code: KeyCode| {
            let event = KeyEvent {
                code,
                modifiers: crossterm::event::KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            };
            crate::input::key(game, event);
        };

        press(&mut game, KeyCode::Down); // Gravebound
        press(&mut game, KeyCode::Enter); // order sworn -> frame step
        assert_eq!(game.create_step, 1);
        assert_eq!(game.create_class, 1);

        press(&mut game, KeyCode::Down); // feminine frame
        press(&mut game, KeyCode::Enter); // frame taken -> boon step
        assert_eq!(game.create_step, 2);
        assert_eq!(game.create_build, 1);

        press(&mut game, KeyCode::Enter); // boon taken -> road
        assert!(matches!(game.modal, Modal::None));
        assert_eq!(game.player.class, Class::Gravebound);
        assert_eq!(game.player.build, Build::Female);

        // Esc walks back down the steps instead of leaving creation.
        let mut game = Game::new(42);
        game.modal = Modal::Create;
        game.create_step = 2;
        game.create_build = 1;
        press(&mut game, KeyCode::Esc);
        assert_eq!(game.create_step, 1);
        assert_eq!(game.selected, 1, "the step resumes on the frame you had");
        press(&mut game, KeyCode::Esc);
        assert_eq!(game.create_step, 0);
        assert!(matches!(game.modal, Modal::Create));
        press(&mut game, KeyCode::Esc);
        assert!(matches!(game.modal, Modal::Title));
    }

    #[test]
    fn gravebound_last_vigil_deepens_below_half_and_quarter() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Gravebound, Build::Male, Boon::None);
        let base = game.player.attack + i32::from(game.player.weapon) * 3;
        assert_eq!(game.attack_power(), base);
        game.player.hp = game.player.max_hp / 2 - 1;
        assert_eq!(game.attack_power(), base + 2);
        game.player.hp = game.player.max_hp / 4 - 1;
        assert_eq!(game.attack_power(), base + 4);
        game.player.hp = game.player.max_hp;
        assert_eq!(game.attack_power(), base, "healing must close the window");
    }

    #[test]
    fn redwake_momentum_banks_a_kill_and_spends_it_on_one_strike() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Redwake;
        game.kill_npc(0);
        assert_eq!(game.player.class_state.momentum, 1);
        let id = game.npcs.len();
        game.npcs.push(crate::world::make_npc(
            id,
            Archetype::Bandit,
            0,
            Pos::new(11, 10),
            2,
            2,
            42,
        ));
        game.npcs[id].intent = Intent::Attack;
        game.player.speed = 30;
        let hp = game.npcs[id].hp;
        let expected = (game.attack_power() + 2 - game.npcs[id].defense).max(1);
        game.refresh_combat();
        game.action(Action::Attack);
        assert_eq!(hp - game.npcs[id].hp, expected, "banked momentum adds +2");
        assert_eq!(game.player.class_state.momentum, 0);
    }

    #[test]
    fn keepwarden_hold_the_gate_recoils_and_builds_resolve() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Keepwarden;
        game.player.speed = 30;
        game.npcs[0].attack = 8;
        let enemy_hp = game.npcs[0].hp;
        game.action(Action::OathPledge(0));
        assert!(game.player.defending);
        assert_eq!(game.player.class_state.resolve, 1);
        assert_eq!(
            enemy_hp - game.npcs[0].hp,
            1,
            "the oath recoils exactly 1 into the round's first attacker"
        );
        let hp = game.player.hp;
        game.player.defending = false;
        game.action(Action::Wait);
        assert!(game.player.hp < hp, "the brace ended when the oath moved on");
    }

    #[test]
    fn sigil_sworn_channel_builds_across_rounds_and_movement_breaks_it() {
        let mut game = fight(Archetype::Skeleton);
        game.apply_creation(Class::SigilSworn, Build::Male, Boon::None);
        game.player.mana = 20;
        game.npcs[0].max_hp = 500;
        game.npcs[0].hp = 500;
        game.npcs[0].attack = 0;
        let bolt = game.attack_power().max(4);
        let hp = game.npcs[0].hp;
        game.action(Action::Cast(0));
        assert_eq!(game.player.class_state.channel, 0, "first cast anchors");
        assert_eq!(hp - game.npcs[0].hp, bolt);
        let hp = game.npcs[0].hp;
        game.player.mana = 20;
        game.action(Action::Cast(0));
        assert_eq!(game.player.class_state.channel, 1);
        assert_eq!(hp - game.npcs[0].hp, bolt + 1);
        let hp = game.npcs[0].hp;
        game.player.mana = 20;
        game.action(Action::Cast(0));
        assert_eq!(game.player.class_state.channel, 2);
        assert_eq!(hp - game.npcs[0].hp, bolt + 2);
        game.action(Action::Move(-1, 0));
        assert_eq!(game.player.class_state.channel, 0, "movement broke the anchor");
    }

    #[test]
    fn waysworn_roads_charge_half_and_trail_carries_one_move() {
        let mut game = Game::new(42);
        game.player.class = Class::Waysworn;
        assert_eq!(game.terrain_cost(Tile::Road), 50);
        assert_eq!(game.terrain_cost(Tile::Ford), 75);
        assert_eq!(game.terrain_cost(Tile::Forest), 150);
        game.player.class = Class::None;
        assert_eq!(game.terrain_cost(Tile::Road), 100);

        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Waysworn;
        game.kill_npc(0);
        assert!(game.player.class_state.trail_charges > 0);
        game.combat = None;
        game.maps[0].set(Pos::new(11, 10), Tile::Forest);
        game.player.stamina = 5;
        game.elapsed_ms = 1000;
        game.move_ready_ms = 0;
        game.action(Action::Move(1, 0));
        assert_eq!(game.move_ready_ms, 1000 + 85, "the trail waived the forest toll");
        assert_eq!(game.player.class_state.trail_charges, 0);
        assert_eq!(game.player.stamina, 7, "standard +1 plus the refund");
    }

    #[test]
    fn fensworn_bond_bites_twice_every_third_striking_round() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Fensworn;
        game.npcs[0].max_hp = 500;
        game.npcs[0].hp = 500;
        let blade = game.npcs.len();
        let mut partner = crate::world::make_npc(
            blade,
            Archetype::Companion,
            0,
            Pos::new(10, 11),
            3,
            900,
            42,
        );
        partner.attack = 30;
        game.npcs.push(partner);
        game.companion = Some(blade);
        game.player.class_state.bond = 2;
        let hp = game.npcs[0].hp;
        let per_hit = (30 - game.npcs[0].defense).max(1);
        game.action(Action::Defend);
        assert_eq!(
            hp - game.npcs[0].hp,
            per_hit * 2,
            "every third striking round doubles"
        );
        assert_eq!(game.player.class_state.bond, 0);
    }

    #[test]
    fn talent_income_tallies_nine_levels_plus_three_sigils() {
        let mut game = Game::new(42);
        game.modal = Modal::None;
        game.apply_creation(Class::Redwake, Build::Male, Boon::None);
        game.player.talent_points = 0;
        // Levels 2..=10 pay one each (the L1 sheet pays nothing).
        for _ in 0..9 {
            game.gain_xp(25 + game.player.level * 15);
        }
        assert_eq!(game.player.level, 10);
        assert_eq!(game.player.talent_points, 9);
        // A sigil claim pays a point (§5 "boss-bonus"): the Lich carries one.
        // The boss's XP also buys level-ups, each paying its own point.
        let mut trial = fight(Archetype::Lich);
        trial.player.talent_points = 9;
        let levels_before = trial.player.level;
        assert!(!trial.player.sigils[2]);
        trial.kill_npc(0);
        assert!(trial.player.sigils[2], "sanity: the Lich drops the Underkeep sigil");
        assert_eq!(
            u32::from(trial.player.talent_points - 9),
            1 + (trial.player.level - levels_before),
            "one sigil point plus one per level the kill bought"
        );
    }

    #[test]
    fn talent_gates_enforce_levels_chains_and_capstone_cost() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Keepwarden, Build::Male, Boon::None);
        game.player.talent_points = 3;
        game.player.level = 2;
        // Tier 3 (level 6) is level-gated.
        game.learn_talent(2);
        assert!(game.player.talents.is_empty(), "Anchor is L6-gated");
        // Tier deeper than predecessor is chained.
        game.learn_talent(5);
        assert!(game.player.talents.is_empty(), "Judgment wants Drilled Counters first");
        // Holding a full branch costs 5 points exactly: 3×1 + 2.
        game.player.level = 9;
        game.player.talent_points = 5;
        game.learn_talent(0);
        game.learn_talent(1);
        game.learn_talent(2);
        game.learn_talent(3);
        assert!(!game.player.talent_active(3), "capstone half-paid is not live");
        game.learn_talent(3);
        assert!(game.player.talent_active(3), "capstone completes on the fifth point");
        assert_eq!(game.player.talent_points_available(), 0);
        game.learn_talent(3);
        assert_eq!(
            game.player.talents.iter().filter(|&&i| i == 3).count(),
            2,
            "a held capstone takes no third point"
        );
    }

    #[test]
    fn respec_returns_points_and_reverts_deltas() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Keepwarden, Build::Male, Boon::None);
        game.player.level = 9;
        game.player.talent_points = 3;
        let stamina = game.player.max_stamina;
        game.learn_talent(8); // Watch-Fires first: Discipline hangs on the chain.
        game.learn_talent(9); // Discipline: +1 max stamina
        assert_eq!(game.player.max_stamina, stamina + 1);
        game.learn_talent(0);
        assert_eq!(game.player.talent_points_available(), 0);
        game.respec();
        assert!(game.player.talents.is_empty());
        assert_eq!(game.player.talent_points_available(), 3);
        assert_eq!(game.player.max_stamina, stamina, "the delta rolls back exactly");
    }

    #[test]
    fn tempest_makes_the_combats_first_rune_free() {
        let mut game = fight(Archetype::Skeleton);
        game.apply_creation(Class::SigilSworn, Build::Male, Boon::None);
        game.player.mana = 4;
        game.player.level = 9;
        game.player.talent_points = 5;
        game.npcs[0].hp = 500;
        game.npcs[0].max_hp = 500;
        for node in [0, 1, 2, 3, 3] {
            game.learn_talent(node); // the Storm chain through Tempest (0,3)
        }
        assert!(game.player.has_talent(0, 3));
        game.action(Action::Cast(0));
        assert_eq!(game.player.mana, 4, "the fight's first spark rides free");
        assert!(game.player.talent_once & once::TEMPEST != 0);
        game.player.mana = 1;
        game.npcs[0].hp = 500;
        game.npcs[0].max_hp = 500;
        // Second cast pays its two mana — and the gate lets it through because mana suffices.
        game.player.mana = 2;
        let hp = game.npcs[0].hp;
        game.action(Action::Cast(0));
        assert_eq!(game.player.mana, 1, "2 spent on the rune; the round returns 1");
        assert!(hp > game.npcs[0].hp, "mana was spent and the rune fired anyway");
    }

    #[test]
    fn conductor_lets_the_channel_hold_three() {
        let mut game = fight(Archetype::Skeleton);
        game.apply_creation(Class::SigilSworn, Build::Male, Boon::None);
        game.player.talents.push(0); // Conductor
        game.player.talent_points = 1;
        game.player.mana = 20;
        game.npcs[0].max_hp = 900;
        game.npcs[0].hp = 900;
        game.npcs[0].attack = 0;
        for expected in [0u8, 1, 2, 3, 3] {
            game.player.mana = 20;
            game.action(Action::Cast(0));
            assert_eq!(game.player.class_state.channel, expected);
        }
    }

    #[test]
    fn bastion_leaves_the_braced_keepwarden_standing() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Keepwarden;
        game.player.talents = vec![3, 3]; // Last Bastion paid
        game.player.speed = 30;
        game.npcs[0].attack = 100;
        game.player.hp = 8;
        game.action(Action::OathPledge(0));
        assert_eq!(game.player.hp, 1, "Last Bastion refuses the grave");
        assert!(!matches!(game.modal, Modal::Death));
        assert!(game.player.talent_once & once::BASTION != 0);
    }

    #[test]
    fn overwatch_bites_enemies_stepping_into_the_brace() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Keepwarden;
        game.player.talents = vec![6]; // Overwatch
        game.player.speed = 30;
        game.npcs[0].pos = Pos::new(12, 10); // 2 tiles out: closes this turn
        game.npcs[0].attack = 0;
        let hp = game.npcs[0].hp;
        game.action(Action::OathPledge(0));
        assert_eq!(
            hp - game.npcs[0].hp,
            1,
            "stepping into the brace's reach costs 1"
        );
    }

    #[test]
    fn packlord_halves_the_bond_cadence() {
        let mut game = fight(Archetype::Bandit);
        game.player.class = Class::Fensworn;
        game.player.talents = vec![3, 3]; // Packlord (Alpha 4) — the two-point capstone
        game.npcs[0].max_hp = 500;
        game.npcs[0].hp = 500;
        let blade = game.npcs.len();
        let mut partner = crate::world::make_npc(
            blade,
            Archetype::Companion,
            0,
            Pos::new(10, 11),
            3,
            900,
            42,
        );
        partner.attack = 30;
        game.npcs.push(partner);
        game.companion = Some(blade);
        // Packlord cadence: the SECOND striking round doubles — bond index (0,3).
        game.player.class_state.bond = 1;
        let hp = game.npcs[0].hp;
        let per_hit = (30 - game.npcs[0].defense).max(1);
        game.action(Action::Defend);
        assert_eq!(
            hp - game.npcs[0].hp,
            per_hit * 2,
            "Packlord doubles every second striking round"
        );
    }

    #[test]
    fn ridge_den_landing_and_bosses_are_anchored_in_the_world() {
        let game = Game::new(42);
        assert_eq!(game.maps[13].name, "Crag Ridge — The Den");
        assert_eq!(game.maps[14].name, "Saltmarsh — The Tidemother's Landing");
        let crag = game
            .npcs
            .iter()
            .find(|n| n.archetype == Archetype::Cragmother)
            .unwrap();
        assert_eq!(crag.map, 13);
        assert_eq!(crag.name, "Cragmother");
        // Both arenas are linked to the world in both directions.
        let into_den = game.maps[0]
            .portals
            .iter()
            .any(|p| p.destination == 13 && p.pos == Pos::new(105, 10));
        let out_of_den = game.maps[13]
            .portals
            .iter()
            .any(|p| p.destination == 0);
        let into_landing = game.maps[3]
            .portals
            .iter()
            .any(|p| p.destination == 14 && p.pos == Pos::new(32, 32));
        let out_of_landing = game.maps[14]
            .portals
            .iter()
            .any(|p| p.destination == 3);
        assert!(into_den && out_of_den && into_landing && out_of_landing);
    }

    /// Builds a Tidemother fight on a dock-like board: water north of y=6.
    fn tideboard(has_saltcrown: bool) -> Game {
        let mut game = fight(Archetype::Tidemother);
        for x in 0..30 {
            for y in 0..=5 {
                game.maps[0].set(Pos::new(x, y), Tile::River);
            }
        }
        game.player.pos = Pos::new(10, 10);
        game.combat.as_mut().unwrap().round = 4;
        if has_saltcrown {
            game.player.relic = Some(BossRelic::Saltcrown);
        } else {
            game.player.relic = None;
        }
        game
    }

    #[test]
    fn undertow_drags_the_hero_two_tiles_and_the_tide_seizes_legs() {
        let mut game = tideboard(false);
        game.boss_turn(0); // arms the undertow
        game.boss_turn(0); // the warning round
        game.boss_turn(0); // the tide takes
        assert_eq!(game.player.pos, Pos::new(10, 8), "dragged two tiles north");
        assert_eq!(game.player.hp, 20 - 4, "the tide's grip exacts its 4");
    }

    #[test]
    fn undertow_over_water_dunks_the_hero_back_on_the_boards() {
        let mut game = tideboard(false);
        game.player.pos = Pos::new(10, 7); // two tiles from the water line itself
        game.combat.as_mut().unwrap().round = 4;
        game.boss_turn(0);
        game.boss_turn(0);
        game.boss_turn(0);
        assert_eq!(game.player.pos, Pos::new(10, 7), "hauled back onto the boards");
        assert_eq!(game.player.hp, 20 - 10 - 4, "the water's toll plus the grip");
    }

    #[test]
    fn saltcrown_roots_the_player_against_the_undertow() {
        let mut game = tideboard(true);
        let hp = game.player.hp;
        game.combat.as_mut().unwrap().round = 4;
        game.boss_turn(0);
        game.boss_turn(0);
        game.boss_turn(0);
        assert_eq!(game.player.pos, Pos::new(10, 10));
        assert_eq!(game.player.hp, hp, "no grip through the crown");
    }

    #[test]
    fn avalanche_slams_leave_rubble_that_the_den_outlives() {
        let mut game = fight(Archetype::Cragmother);
        game.player.pos = Pos::new(10, 14);
        game.npcs[0].answers.insert(
            "commit_slam".into(),
            crate::model::Answer {
                value: 0.9,
                choice: None,
                probabilities: Default::default(),
                confidence: 0.9,
            },
        );
        game.boss_turn(0); // arms the slam
        game.boss_turn(0);
        game.boss_turn(0); // the hillside comes down
        assert!(!game.rubble.is_empty());
        assert_eq!(game.maps[0].tile(Pos::new(10, 13)), Tile::Rock);
        assert_eq!(
            game.maps[0].tile(Pos::new(10, 14)),
            Tile::Floor,
            "the marked tile under the player stays walkable"
        );
        game.npcs[0].answers.insert(
            "commit_slam".into(),
            crate::model::Answer {
                value: 0.0,
                choice: None,
                probabilities: Default::default(),
                confidence: 0.0,
            },
        );
        game.npcs[0].cooldown = 0;
        game.npcs[0].telegraph = None;
        game.player.pos = Pos::new(4, 4); // far enough off the bubble that combat ends
        game.refresh_combat(); // players separate: combat ends, settles the ground
        assert!(game.rubble.is_empty());
        assert_eq!(game.maps[0].tile(Pos::new(10, 13)), Tile::Floor);
    }

    #[test]
    fn cragmother_calls_her_cubs_when_guarding_deep_wounds() {
        let mut game = fight(Archetype::Cragmother);
        game.npcs[0].phase = 2;
        game.npcs[0].answers.insert(
            "guard_cubs".into(),
            crate::model::Answer {
                value: 0.9,
                choice: None,
                probabilities: Default::default(),
                confidence: 0.9,
            },
        );
        let before = game.npcs.len();
        game.boss_turn(0);
        game.boss_turn(0);
        game.boss_turn(0);
        let bears = game
            .npcs
            .iter()
            .filter(|n| n.archetype == Archetype::Bear && n.alive())
            .count();
        assert!(
            game.npcs.len() > before && bears >= 1,
            "cubs answer the roar from the nests"
        );
    }

    #[test]
    fn arena_bosses_pay_their_relics_and_stoneheart_reads_the_ground() {
        let mut game = fight(Archetype::Tidemother);
        game.kill_npc(0);
        assert!(
            game.player.relic == Some(BossRelic::Saltcrown)
                || game
                    .player
                    .inventory
                    .contains(&Item::BossRelic(BossRelic::Saltcrown)),
            "the Tidemother pays Saltcrown"
        );

        let mut game = fight(Archetype::Cragmother);
        game.kill_npc(0);
        assert!(game.player.relic == Some(BossRelic::Stoneheart));
        assert_eq!(game.terrain_cost(Tile::Rock), Tile::Road.cost());
        assert_eq!(game.terrain_cost(Tile::Mountain), Tile::Road.cost());
    }

    #[test]
    fn the_landing_stirs_once_and_only_once() {
        let mut game = Game::new(42);
        assert!(
            !game
                .npcs
                .iter()
                .any(|n| n.archetype == Archetype::Tidemother),
            "the landing waits while the parcel line is open"
        );
        game.spawn_tidemother();
        let count = game
            .npcs
            .iter()
            .filter(|n| n.archetype == Archetype::Tidemother)
            .count();
        game.spawn_tidemother();
        assert_eq!(
            count,
            game.npcs
                .iter()
                .filter(|n| n.archetype == Archetype::Tidemother)
                .count(),
            "the water gives her up exactly once"
        );
    }

    #[test]
    fn arena_boss_question_banks_carry_the_new_levers() {
        let tide = crate::laya_client::question_bank(Archetype::Tidemother, 3);
        assert!(tide.contains_key("drag_who"));
        assert!(tide.contains_key("sacrifice_crew"));
        let crag = crate::laya_client::question_bank(Archetype::Cragmother, 3);
        assert!(crag.contains_key("commit_slam"));
        assert!(crag.contains_key("guard_cubs"));
        let gnaw = crate::laya_client::question_bank(Archetype::GnawThane, 3);
        assert!(gnaw.contains_key("call_the_tide"));
        assert!(gnaw.contains_key("scatter_when_thinned"));
        let toll = crate::laya_client::question_bank(Archetype::Tollmaster, 3);
        assert!(toll.contains_key("shove_now"));
        assert!(toll.contains_key("collect_or_cut"));
        let wisp = crate::laya_client::question_bank(Archetype::Mirelight, 3);
        assert!(wisp.contains_key("which_light"));
        assert!(wisp.contains_key("strike_or_subside"));
        let stag = crate::laya_client::question_bank(Archetype::PaleStag, 3);
        assert!(stag.contains_key("break_and_run"));
        assert!(stag.contains_key("stand_ground"));
    }

    fn answer(value: f32, choice: Option<&str>) -> crate::model::Answer {
        crate::model::Answer {
            value,
            choice: choice.map(str::to_string),
            probabilities: Default::default(),
            confidence: value,
        }
    }

    /// A Plague-Tide board: the king, his two holes, one open lane.
    fn barrow_board() -> Game {
        let mut game = fight(Archetype::GnawThane);
        game.maps[0].kind = MapKind::Dungeon(4, 0);
        game.player.pos = Pos::new(10, 12);
        game.npcs[0].pos = Pos::new(11, 12);
        game.npcs[0].pack = 7000;
        for (id, pos) in [(1usize, Pos::new(4, 12)), (2usize, Pos::new(18, 12))] {
            game.npcs.push(crate::world::make_npc(
                id,
                Archetype::BroodHole,
                0,
                pos,
                2,
                7000,
                42,
            ));
        }
        game
    }

    #[test]
    fn plague_tide_pours_from_living_holes_and_dies_with_them() {
        let mut game = barrow_board();
        game.npcs[0]
            .answers
            .insert("call_the_tide".into(), answer(0.9, None));
        game.combat.as_mut().unwrap().round = 3;
        game.boss_turn(0); // the king calls
        game.boss_turn(0);
        game.boss_turn(0); // the tide pours
        let rats = game
            .npcs
            .iter()
            .filter(|n| n.archetype == Archetype::Rat && n.alive())
            .count();
        assert_eq!(rats, 2, "each living hole spends one rat on the tide");
        // Burn both holes; the warren has nothing left to give.
        game.kill_npc(1);
        game.kill_npc(2);
        game.npcs[0].cooldown = 0;
        game.npcs[0].telegraph = None;
        game.combat.as_mut().unwrap().round = 6;
        for _ in 0..3 {
            game.boss_turn(0);
        }
        let rats = game
            .npcs
            .iter()
            .filter(|n| n.archetype == Archetype::Rat && n.alive())
            .count();
        assert_eq!(rats, 2, "no fresh tide once the holes are silenced");
        // A rushed king still pays his relic.
        game.kill_npc(0);
        assert_eq!(game.player.relic, Some(BossRelic::GnawboneCrown));
    }

    #[test]
    fn barrow_bites_fester_and_inns_and_tonic_cleanse() {
        let mut game = barrow_board();
        // Replace the board with one biting rat in striking range.
        game.npcs[0].pos = Pos::new(20, 20);
        let rat = game.npcs.len();
        let mut ratty = crate::world::make_npc(rat, Archetype::Rat, 0, Pos::new(11, 12), 1, 7001, 42);
        ratty.intent = Intent::Attack;
        game.npcs.push(ratty);
        let stamina = game.player.max_stamina;
        let turns = game.turn;
        for bite in 1..=3u8 {
            game.turn = turns + u64::from(bite);
            game.player.hp = game.player.max_hp;
            game.enemy_turn(rat);
            assert_eq!(game.player.fester, bite);
        }
        assert_eq!(
            game.player.max_stamina,
            stamina - 1,
            "every third stack saps the wind"
        );
        // The tonic's afterglow turns a bite, then the purge returns the wind.
        game.player.inventory.push(Item::AntiToxin);
        let slot = game.player.inventory.len() - 1;
        game.player.hp = 5;
        assert!(game.use_item(slot));
        assert_eq!(game.player.fester, 0);
        assert_eq!(game.player.max_stamina, stamina);
        game.player.hp = 5;
        let before = game.player.fester;
        game.enemy_turn(rat);
        assert_eq!(game.player.fester, before, "the afterglow turns the fever");
        // And the inn's hot broth is the working cure.
        game.player.fester = 5;
        game.player.max_stamina -= 1;
        game.combat = None;
        game.player.map = 1;
        game.player.pos = Pos::new(12, 19);
        game.player.gold = 10;
        assert!(game.rest_at_inn());
        assert_eq!(game.player.fester, 0);
    }

    #[test]
    fn gnawbone_crown_mutes_the_warren_and_feeds_dungeon_wind() {
        let mut game = barrow_board();
        game.player.relic = Some(BossRelic::GnawboneCrown);
        let rat = game.npcs.len();
        let mut ratty = crate::world::make_npc(rat, Archetype::Rat, 0, Pos::new(11, 12), 1, 7001, 42);
        ratty.intent = Intent::Attack;
        game.npcs.push(ratty);
        let hp = game.player.hp;
        game.enemy_turn(rat);
        assert_eq!(game.player.hp, hp, "crowned rats cannot turn on you");
        // Dungeon steps carry +1 extra stamina under the crown.
        game.player.stamina = 1;
        game.player.pos = Pos::new(6, 6);
        game.combat = None;
        game.move_ready_ms = 0;
        game.elapsed_ms = 100;
        game.exploration_action(Action::Move(0, 1));
        assert_eq!(
            game.player.stamina,
            3,
            "one for the step, one the crown's due"
        );
    }

    /// A dock-like toll board: river north of y=2, camp road through y=9.
    fn toll_board() -> Game {
        let mut game = fight(Archetype::Tollmaster);
        for x in 0..30 {
            for y in 0..=1 {
                game.maps[0].set(Pos::new(x, y), Tile::River);
            }
        }
        game.player.pos = Pos::new(10, 4);
        game.npcs[0].pos = Pos::new(10, 6);
        game
    }

    #[test]
    fn bridge_tax_shoves_to_the_water_and_sows_caltrops() {
        let mut game = toll_board();
        game.npcs[0]
            .answers
            .insert("shove_now".into(), answer(0.9, None));
        game.npcs[0]
            .answers
            .insert("collect_or_cut".into(), answer(0.5, Some("press")));
        game.combat.as_mut().unwrap().round = 4; // round % 3 == 1
        game.boss_turn(0); // the crew sets its shoulders
        game.boss_turn(0);
        game.boss_turn(0); // the tax comes due
        assert_eq!(game.player.pos.y, 2, "two tiles toward the ford");
        assert!(!game.caltrops.is_empty(), "the road behind is seeded");
        // Next tax: one more shove dunks and refunds to solid ground.
        game.npcs[0].cooldown = 0;
        game.npcs[0].telegraph = None;
        game.combat.as_mut().unwrap().round = 7;
        let hp = game.player.hp;
        game.boss_turn(0);
        game.boss_turn(0);
        game.boss_turn(0);
        assert!(game.player.hp < hp, "the ford's foam takes its due");
        assert_eq!(game.player.pos.y, 2, "dunked heroes haul out where they stood");
        // Stepping onto the sown ground bites once, then it is honest.
        let (cm, cp, _) = game.caltrops[0];
        game.player.pos = cp;
        let hurt = game.player.hp;
        game.caltrop_check();
        assert!(
            !game
                .caltrops
                .iter()
                .any(|(m2, p2, _)| *m2 == cm && *p2 == cp),
            "a spent caltrop clears its ground"
        );
        assert!(game.player.hp < hurt, "the planted steel bites once");
        game.kill_npc(0);
        assert_eq!(game.player.relic, Some(BossRelic::TollcoinCharm));
    }

    #[test]
    fn collect_or_cut_buys_a_breath_when_greed_wins() {
        let mut game = toll_board();
        game.npcs[0]
            .answers
            .insert("collect_or_cut".into(), answer(0.5, Some("loot")));
        game.npcs[0].hp = 20;
        game.boss_turn(0); // he turns to the straps
        game.boss_turn(0);
        game.boss_turn(0);
        assert_eq!(
            game.npcs[0].hp, 26,
            "a greedy Tollmaster counts coppers, not casualties"
        );
    }

    fn wisp_board() -> Game {
        let mut game = fight(Archetype::Mirelight);
        game.player.pos = Pos::new(10, 12);
        game.npcs[0].pos = Pos::new(12, 10);
        game.npcs[0].pack = 7002;
        game
    }

    #[test]
    fn three_false_lights_split_drink_and_gutter() {
        let mut game = wisp_board();
        game.npcs[0]
            .answers
            .insert("which_light".into(), answer(0.5, Some("centre")));
        game.combat.as_mut().unwrap().round = 2;
        game.boss_turn(0); // she divides
        game.boss_turn(0);
        game.boss_turn(0); // three lights hover
        let glows: Vec<usize> = game
            .npcs
            .iter()
            .enumerate()
            .filter(|(_, n)| n.archetype == Archetype::FalseGlow && n.alive())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(glows.len(), 2, "two borrowed lights carry the trick");
        assert_eq!(game.npcs[0].pos, Pos::new(12, 10), "the tell said centre");
        // A mistaken strike feeds the true wisp.
        game.npcs[0].hp = 30;
        let glow = glows[0];
        game.npcs[glow].pos = Pos::new(11, 10);
        game.player.pos = Pos::new(10, 10);
        game.player_attack(glow, false);
        assert_eq!(game.npcs[0].hp, 36, "the wrong light drinks the blow");
        // Strike her true and the borrowed lights die with her.
        game.kill_npc(0);
        assert!(game
            .npcs
            .iter()
            .all(|n| n.archetype != Archetype::FalseGlow || !n.alive()));
        assert_eq!(game.player.relic, Some(BossRelic::WisplightLantern));
        assert!(game
            .player
            .inventory
            .iter()
            .any(|i| *i == Item::Essence));
    }

    #[test]
    fn mirelight_keeps_night_hours() {
        let mut game = Game::new(42);
        assert!(!game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::Mirelight));
        game.hour_ticks = 480 * 14; // 22:00 — dusk
        game.mirelight_upkeep();
        assert!(game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::Mirelight && n.alive()));
        let wisp = game
            .npcs
            .iter()
            .position(|n| n.archetype == Archetype::Mirelight)
            .unwrap();
        let risen = game.npcs[wisp].pos;
        game.hour_ticks = 480 * 23; // 07:00 — dawn
        game.mirelight_upkeep();
        assert_eq!(
            game.npcs[wisp].pos,
            Pos::new(2, 2),
            "dawn sends her back to the sealed pool (was {risen:?})"
        );
    }

    #[test]
    fn pale_stag_breaks_and_breathes_but_stands_cornered() {
        let mut game = fight(Archetype::PaleStag);
        game.player.pos = Pos::new(10, 10);
        game.npcs[0].pos = Pos::new(11, 10);
        game.npcs[0].hp = 20; // below half
        let hp = game.npcs[0].hp;
        game.boss_turn(0); // the break
        game.boss_turn(0);
        game.boss_turn(0);
        assert_eq!(game.npcs[0].hp, hp + 5, "a clean break buys back five");
        assert!(
            game.npcs[0].pos.distance(game.player.pos) > 1,
            "the stag opens air"
        );
        // Cornered against rock: it turns at bay.
        game.npcs[0].telegraph = None;
        game.npcs[0].cooldown = 0;
        game.npcs[0].pos = Pos::new(1, 1);
        game.player.pos = Pos::new(2, 2);
        for x in 0..=2 {
            for y in 0..=2 {
                if (x, y) != (1, 1) && (x, y) != (2, 2) {
                    game.maps[0].set(Pos::new(x, y), Tile::Wall);
                }
            }
        }
        game.boss_turn(0);
        assert_eq!(
            game.npcs[0].telegraph.as_ref().map(|(_, t)| t.as_str()),
            Some("gore"),
            "at bay the stag fights"
        );
        game.kill_npc(0);
        assert_eq!(game.player.relic, Some(BossRelic::Hartshorn));
        assert_eq!(game.player_speed(), game.player.speed + 1);
    }

    #[test]
    fn three_settled_culls_of_one_kind_call_out_the_alpha() {
        let mut game = Game::new(42);
        game.player.map = 1;
        for kind in [0, 0, 1, 0] {
            game.quests.push(Quest {
                title: format!("cull {kind}"),
                stage: QuestStage::Ready,
                progress: 3,
                goal: 3,
                description: "test".into(),
            });
            game.bounties.push(Bounty {
                kind: BountyKind::Cull(if kind == 0 {
                    Archetype::Wolf
                } else {
                    Archetype::Bandit
                }),
                origin: 0,
                gold: 10,
                xp: 5,
                reputation: 1,
            });
        }
        game.pay_bounty(0, "wolf");
        game.pay_bounty(1, "wolf");
        assert!(
            !game.npcs.iter().any(|n| n.archetype == Archetype::PaleStag),
            "two settled wolf culls are not yet a pattern"
        );
        game.pay_bounty(3, "wolf");
        assert!(
            game.npcs.iter().any(|n| n.archetype == Archetype::PaleStag),
            "the third settled wolf cull tips the captain off"
        );
        game.pay_bounty(2, "bandit"); // a bandit cull does not un-spawn the stag
        assert_eq!(
            game.npcs
                .iter()
                .filter(|n| n.archetype == Archetype::PaleStag)
                .count(),
            1
        );
    }

    #[test]
    fn sworn_terms_refuse_retreat_from_the_trial() {
        let mut game = fight(Archetype::Adjudicator);
        game.trial_oath = true;
        game.player.stamina = 5;
        let pos = game.player.pos;
        assert!(!game.flee_combat());
        assert_eq!(game.player.pos, pos, "the oath holds fast");
    }

    #[test]
    fn hiring_scales_to_the_hirer_and_every_level_after() {
        let mut game = Game::new(42);
        game.player.gold = 100;
        game.player.level = 4;
        let blade = game
            .npcs
            .iter()
            .position(|n| n.archetype == Archetype::Companion && n.map == 1)
            .unwrap();
        let (hp, attack, _, _) = crate::world::sellsword_stats(4);
        game.toggle_companion(blade);
        assert_eq!(game.companion, Some(blade));
        assert_eq!(game.npcs[blade].max_hp, hp);
        assert_eq!(game.npcs[blade].attack, attack);
        // D35: every employer level re-derives the sheet.
        game.gain_xp(10_000);
        assert!(game.player.level > 4);
        let (hp2, attack2, _, _) = crate::world::sellsword_stats(game.player.level);
        assert_eq!(game.npcs[blade].max_hp, hp2, "the blade grows with the hirer");
        assert_eq!(game.npcs[blade].attack, attack2);
    }

    #[test]
    fn the_cap_is_twelve_and_mastery_levels_hold_their_points() {
        let mut game = Game::new(42);
        game.gain_xp(10_000);
        assert_eq!(game.player.level, 12, "D26: the campaign climbs to 12");
        assert_eq!(
            game.player.talent_points, 9,
            "D34: L11–L12 grow the body, not the tree (9 + 3 sigils = 12 total)"
        );
    }

    #[test]
    fn packlord_swears_the_old_truce_with_one_wolf() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Fensworn, Build::Male, Boon::None);
        game.player.talents = vec![0, 0, 0]; // Packlord is not yet sworn.
        let wolf = game.npcs.len();
        game.npcs.push(crate::world::make_npc(
            wolf,
            Archetype::Wolf,
            game.player.map,
            game.player.pos.offset(1, 0),
            3,
            9999,
            42,
        ));
        assert!(!game.wolf_truce_holds(&game.npcs[wolf].clone()));
        game.player.talents = vec![3, 3]; // Packlord (Alpha 4, a 2-point capstone).
        assert!(game.wolf_truce_holds(&game.npcs[wolf].clone()));
        game.swear_wolf_truce(wolf);
        assert_eq!(game.companion, Some(wolf));
        assert_eq!(game.npcs[wolf].archetype, Archetype::Companion);
        // One bond only: a second wolf stays a wolf.
        let other = game.npcs.len();
        game.npcs.push(crate::world::make_npc(
            other,
            Archetype::Wolf,
            game.player.map,
            game.player.pos.offset(-1, 0),
            3,
            9999,
            42,
        ));
        game.swear_wolf_truce(other);
        assert_eq!(
            game.npcs[other].archetype,
            Archetype::Wolf,
            "the truce holds one bond"
        );
    }

    #[test]
    fn essence_returns_every_sworn_point() {
        let mut game = Game::new(42);
        game.apply_creation(Class::Keepwarden, Build::Male, Boon::None);
        game.player.talent_points = 3;
        game.learn_talent(0);
        assert_eq!(game.player.talent_points_available(), 2);
        game.player.inventory.push(Item::Essence);
        let slot = game.player.inventory.len() - 1;
        assert!(game.use_item(slot));
        assert!(game.player.talents.is_empty(), "the essence unwrites");
        // The free shrine unwinding still stands apart (D8).
        assert!(!game.player.respec_used);
    }

    #[test]
    fn side_boss_spawns_hold_their_quest_gates() {
        let mut game = Game::new(42);
        assert!(!game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::GnawThane));
        game.spawn_gnawthane();
        assert!(game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::GnawThane));
        assert_eq!(
            game.npcs
                .iter()
                .filter(|n| n.archetype == Archetype::BroodHole)
                .count(),
            2
        );
        game.spawn_gnawthane(); // idempotent
        assert_eq!(
            game.npcs
                .iter()
                .filter(|n| n.archetype == Archetype::GnawThane)
                .count(),
            1
        );
        game.spawn_tollmaster();
        assert!(game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::Tollmaster));
    }

    #[test]
    fn e5_save_additions_default_for_pre_e5_journeys() {
        // Pre-E5 saves know nothing of fester, caltrops, the bag-o-culls tally
        // or the Trial's oath; every addition must land its serde default.
        let game = Game::new(42);
        let mut value = serde_json::to_value(&game).unwrap();
        let root = value.as_object_mut().unwrap();
        for key in [
            "caltrops",
            "cull_wins",
            "trial_oath",
            "curate_unwritten",
            "mercy_oathbreaker",
            "epilogue",
            "codex",
            "masterwork_used",
            "scrounge_sites",
            "scrounged",
            "trials",
        ] {
            root.remove(key);
        }
        let player = root
            .get_mut("player")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap();
        player.remove("fester");
        player.remove("fester_guard_until");
        player.remove("word_weapon");
        player.remove("word_armour");
        let restored: Game = serde_json::from_value(value).unwrap();
        assert_eq!(restored.player.fester, 0);
        assert_eq!(restored.player.fester_guard_until, 0);
        assert!(restored.caltrops.is_empty());
        assert_eq!(restored.cull_wins, [0, 0]);
        assert!(!restored.trial_oath);
        // E7 (§6.5/D31/D32): all E7 state rides the same default-hydration rule.
        assert_eq!(restored.curate_unwritten, [false; 3]);
        assert!(!restored.mercy_oathbreaker);
        assert_eq!(restored.epilogue, Epilogue::Unanswered);
        assert!(restored.codex.is_empty());
        assert!(restored.scrounged.is_empty());
        // A pre-E7 file carries no scrounge map and no written gear: both land empty.
        assert!(restored.scrounge_sites.is_empty());
        assert_eq!(restored.player.word_weapon, None);
        assert_eq!(restored.player.word_armour, None);
        assert!(restored
            .trials
            .iter()
            .all(|t| matches!(t.stage, TrialStage::Locked)));
    }

    #[test]
    fn the_third_sigil_wakes_the_covenant_breaker() {
        let mut game = Game::new(42);
        assert!(!game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::OathlessCurate));
        for (i, boss) in [Archetype::Chief, Archetype::Matriarch, Archetype::Lich]
            .into_iter()
            .enumerate()
        {
            let id = game.npcs.len();
            let mut npc = crate::world::make_npc(id, boss, 0, Pos::new(20, 20), 5, 1, 42);
            npc.intent = Intent::Attack;
            game.npcs.push(npc);
            game.kill_npc(id);
            assert!(game.player.sigils[i]);
        }
        assert!(game
            .npcs
            .iter()
            .any(|n| n.archetype == Archetype::OathlessCurate));
        let curate = game
            .npcs
            .iter()
            .find(|n| n.archetype == Archetype::OathlessCurate)
            .unwrap();
        assert_eq!(curate.hp, 150, "§6: the covenant-breaker holds ~150hp");
        assert_eq!(curate.map, 20, "the cell is the second depth");
        assert!(game.log.iter().any(|l| l.contains("two doors")));
    }

    #[test]
    fn unwrit_silences_a_sigil_for_one_fight_only() {
        let mut game = fight(Archetype::OathlessCurate);
        assert_eq!(game.spell_cost(Spell::Spark), 2, "baseline without relic");
        game.player.relic = Some(BossRelic::Graveglass);
        assert_eq!(game.spell_cost(Spell::Spark), 1, "the sigil's blessing");
        game.npcs[0].phase = 2;
        game.npcs[0].answers.insert(
            "which_sigil_falls".into(),
            answer(0.9, Some("third")),
        );
        game.boss_turn(0); // the page rises (telegraph)
        game.boss_turn(0);
        game.boss_turn(0); // the words take
        assert_eq!(game.curate_unwritten, [false, false, true]);
        assert_eq!(
            game.spell_cost(Spell::Spark),
            2,
            "the Unwrit kills the blessing mid-fight"
        );
        // ...and only one fall per third: a second call in the same third fails.
        game.npcs[0].cooldown = 0;
        game.npcs[0].telegraph = None;
        game.boss_turn(0);
        assert_eq!(
            game.curate_unwritten.iter().filter(|u| **u).count(),
            1,
            "one sigil per third of HP"
        );
        // The fight's end returns what the fight unmade.
        game.player.pos = Pos::new(2, 2);
        game.refresh_combat();
        assert_eq!(game.curate_unwritten, [false; 3]);
    }

    #[test]
    fn the_covenant_breaker_kneels_and_the_writ_changes_hands() {
        // Kill path: the Writ pays out either way (§6.5 "the alternative").
        let mut game = fight(Archetype::OathlessCurate);
        game.kill_npc(0);
        assert!(game
            .player
            .inventory
            .iter()
            .any(|i| *i == Item::FirstWrit));
        // Mercy path: broken, the Curate kneels; sparing pays the Writ too and
        // the Adjudicator's record marks the answer sincere.
        let mut game = fight(Archetype::OathlessCurate);
        game.npcs[0].hp = 4; // below a fifth of 150
        game.npcs[0]
            .answers
            .insert("mercy_for_the_oathbreaker".into(), answer(0.9, None));
        game.boss_turn(0); // the knee bends (telegraph)
        game.boss_turn(0);
        game.boss_turn(0);
        assert_eq!(game.npcs[0].intent, Intent::Flee, "the kneel offers itself");
        game.player.pos = Pos::new(10, 10);
        game.npcs[0].pos = Pos::new(11, 10);
        assert!(game.spare_enemy());
        assert!(game.mercy_oathbreaker, "the sincerity is on record");
        assert!(game
            .player
            .inventory
            .iter()
            .any(|i| *i == Item::FirstWrit));
        assert!(game.combat.is_none(), "the cell empties with mercy");
    }

    #[test]
    fn the_arbiter_asks_and_the_shrine_takes_the_answer() {
        let mut game = fight(Archetype::Rat);
        game.maps[0].set(Pos::new(5, 5), Tile::Shrine);
        game.player.pos = Pos::new(5, 4);
        game.combat = None;
        game.npcs[0].map = 9; // out of the way; the hall is quiet
        game.won = true;
        game.player.inventory.push(Item::FirstWrit);
        assert_eq!(game.epilogue, Epilogue::Unanswered);
        game.interact();
        assert_eq!(game.epilogue, Epilogue::Sealed);
        game.interact();
        assert_eq!(game.epilogue, Epilogue::Watch);
        game.interact();
        assert_eq!(game.epilogue, Epilogue::Sealed, "the page re-reads itself");
        // Without the Writ, the shrine stays a shrine.
        let mut game = fight(Archetype::Rat);
        game.maps[0].set(Pos::new(5, 5), Tile::Shrine);
        game.player.pos = Pos::new(5, 4);
        game.combat = None;
        game.npcs[0].map = 9;
        game.won = true;
        game.player.stamina = 3;
        game.interact(); // plain wayshrine heal, no arbiter
        assert_eq!(game.epilogue, Epilogue::Unanswered);
    }

    #[test]
    fn transmute_spends_inputs_and_the_codex_remembers_once() {
        let mut game = Game::new(42);
        game.player.inventory = vec![
            Item::Potion,
            Item::Potion,
            Item::Potion,
            Item::Ration,
            Item::Ration,
        ];
        game.transmute(0);
        assert!(game
            .player
            .inventory
            .iter()
            .any(|i| *i == Item::GreaterPotion));
        assert!(!game.player.inventory.iter().any(|i| *i == Item::Potion));
        assert_eq!(game.codex.len(), 1);
        game.player.inventory = vec![Item::Ration, Item::Ration];
        game.transmute(1);
        assert!(game
            .player
            .inventory
            .iter()
            .any(|i| *i == Item::TravelerRation));
        assert_eq!(game.codex.len(), 2);
        // A repeat brew teaches nothing new (Qud recipe memory).
        game.player.inventory = vec![Item::GemDust, Item::GemDust, Item::GemDust, Item::GemDust];
        game.transmute(2);
        let codex_len = game.codex.len();
        game.transmute(2);
        assert_eq!(game.codex.len(), codex_len, "known recipes stay learned once");
        assert_eq!(
            game.player
                .inventory
                .iter()
                .filter(|i| **i == Item::GlyphShard)
                .count(),
            2
        );
    }

    #[test]
    fn scrounge_gives_once_and_no_more() {
        let mut game = Game::new(42);
        assert!(
            !game.scrounge_sites.is_empty(),
            "§8.2: herb/ore sites are seeded at worldgen"
        );
        let (site, _) = game.scrounge_sites[0];
        game.player.map = 0;
        game.player.pos = site;
        let before = game.scrounge_sites.len();
        game.scrounge_check();
        assert_eq!(
            game.player
                .inventory
                .iter()
                .filter(|i| matches!(i, Item::HerbCluster | Item::OreFlake))
                .count(),
            1
        );
        game.scrounge_check();
        assert_eq!(
            game.scrounged.len(),
            1,
            "the one-shot strip marks the site ({before} sites planted)"
        );
        assert_eq!(
            game.player
                .inventory
                .iter()
                .filter(|i| matches!(i, Item::HerbCluster | Item::OreFlake))
                .count(),
            1,
            "no grind loop: the site is done"
        );
    }

    #[test]
    fn the_smiths_socket_writes_a_word_and_it_wakes() {
        let mut game = Game::new(42);
        game.player.weapon = 2; // fine steel carries two sockets (§8.1)
        game.player.armour = 2;
        game.player.inventory.push(Item::Glyph(0)); // ash
        game.player.inventory.push(Item::Glyph(1)); // fen
        game.player.gold = 100;
        let base = game.attack_power();
        game.socket_word(1); // Ember: ash·fen
        assert_eq!(game.player.word_weapon, Some(1));
        assert_eq!(game.attack_power(), base + 2, "the written blade scorches");
        assert!(!game
            .player
            .inventory
            .iter()
            .any(|i| matches!(i, Item::Glyph(_))));
        assert_eq!(game.player.gold, 25);
        // Armour-side words wake the same way (Gale: hart·seal → +1 speed).
        game.player.inventory.push(Item::Glyph(4));
        game.player.inventory.push(Item::Glyph(3));
        game.player.gold = 100;
        game.socket_word(2);
        assert_eq!(game.player.word_armour, Some(2));
        assert_eq!(game.player_speed(), game.player.speed + 1);
    }

    #[test]
    fn the_masterwork_beat_spends_once() {
        let mut game = Game::new(42);
        game.player.sigils[0] = true;
        assert_eq!(game.player.weapon, 0);
        game.masterwork_beat();
        assert_eq!(game.player.weapon, 1);
        game.masterwork_beat();
        assert_eq!(game.player.weapon, 1, "the anvil's favor is spent");
    }

    #[test]
    fn the_garrisons_trial_holds_three_waves() {
        let mut game = Game::new(42);
        game.player.level = 6;
        game.offer_trial(0);
        assert!(matches!(game.trials[0].stage, TrialStage::Offered));
        game.player.map = 21;
        game.player.pos = Pos::new(7, 7);
        game.trial_upkeep();
        assert!(matches!(game.trials[0].stage, TrialStage::Active));
        for expected_wave in 1..=3u8 {
            let members: Vec<usize> = game
                .npcs
                .iter()
                .filter(|n| n.pack == 6000 && n.alive() && n.map == 21)
                .map(|n| n.id)
                .collect();
            assert!(
                !members.is_empty(),
                "wave {expected_wave} stands in the Drill Yard"
            );
            for id in members {
                game.kill_npc(id);
            }
            game.trial_upkeep();
        }
        assert!(matches!(game.trials[0].stage, TrialStage::Done));
        assert!(game
            .log
            .iter()
            .any(|l| l.contains("the gate holds")));
    }

    #[test]
    fn the_old_truce_ends_only_when_mercy_does() {
        let mut game = Game::new(42);
        game.player.level = 5;
        game.offer_trial(2);
        game.player.map = 23;
        game.player.pos = Pos::new(7, 6);
        game.trial_upkeep();
        let alpha = game
            .npcs
            .iter()
            .position(|n| n.pack == 6020 && n.alive() && n.map == 23)
            .expect("the marsh holds a sick alpha");
        game.npcs[alpha].intent = Intent::Flee;
        game.npcs[alpha].pos = game.player.pos.offset(1, 0);
        game.combat = None;
        game.exploration_action(Action::Mercy);
        game.trial_upkeep();
        assert!(
            matches!(game.trials[2].stage, TrialStage::Done),
            "the fight the mercy verb can end"
        );
        // Killing it instead reads as the wrong answer and re-offers the truce.
        let mut game = Game::new(42);
        game.player.level = 5;
        game.offer_trial(2);
        game.player.map = 23;
        game.trial_upkeep();
        let alpha = game
            .npcs
            .iter()
            .position(|n| n.pack == 6020 && n.alive() && n.map == 23)
            .unwrap();
        game.kill_npc(alpha);
        game.trial_upkeep();
        assert!(
            matches!(game.trials[2].stage, TrialStage::Offered),
            "the marsh asks again until it is answered gently"
        );
    }
}
