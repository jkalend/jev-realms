# Roadmap — Jev Realms

Implementation has not started. Milestones are ordered so that each one is independently demo-able, and each one forces a real Jev integration decision before the next can proceed. AI denotes a live Jev call; HEUR denotes the offline deterministic provider.

## M0 — Walking skeleton

**Goal**: a thing you can run and move around in.

- [ ] Cargo project, `ratatui` + `crossterm` setup, main loop at 4 Hz with input polling
- [ ] 60×30 viewport, scrolling camera, world rendered from a static tile map
- [ ] Player `@`, 8-way movement, terrain costs from GAME_DESIGN §3.1
- [ ] HUD bar (HP/stamina/gold/level), event log panel, `?` help overlay

**Exit**: `cargo run` → walk a hand-authored 100×100 map, terrain slows you, HUD updates.

## M1 — The world

**Goal**: the GAME_DESIGN §3 world, content-complete for v1.

- [ ] Terrain generation: hand-authored regions + seeded procedural fill, 200×160
- [ ] Three cities: layout, gates, day/night tinting, NPC schedule scaffolding
- [ ] Three dungeons + Final Trial arena: template-based floor generation
- [ ] NPC entities: archetypes spawn per location/time tables; tier-0 reflex behaviors (wander, schedule-follow, pathfind)
- [ ] NPC memory records (GAME_DESIGN §8)

**Exit**: full world walkable; Millbrook commoners wander, vendors keep shop hours, thieves appear at night (all tier-0 scripted).

## M2 — First Jev blood

**Goal**: prove the AI architecture end-to-end with the cheapest possible slice.

- [ ] `jev_client.rs`: async gateway client, batching per NPC per window, 3 s timeout, retry ×1
- [ ] `DecisionProvider` trait + `JevProvider` + `HeuristicProvider`, `--ai jev|heuristics` CLI flag
- [ ] Bandit archetype: spawn in forests/fords, tier-1 question bank (ambush/target/flee), behavior rules applying answers
- [ ] Combat: the full §6 system (initiative, flanking, defend, flee, pack morale rule)
- [ ] Jev inspector panel (`J`): last 10 decisions, probabilities visible
- [ ] Replay log: every decision appended (provider, npc, questions, answers, latency, applied rule)
- [ ] Latency/cost instrumentation vs the AI_ARCHITECTURE §7 budget; resolve open questions 1–2

**Exit**: walk into a forest, get ambushed by bandits whose probabilities you can read in the inspector; same session seed replays identically under `--ai heuristics`.

## M3 — The social city

**Goal**: tier-2 social decisions, city gameplay loop.

- [ ] Guards: suspect/pursue/bribe question bank; night gates; road patrols
- [ ] Vendors: trade UI, haggle_accept/cheat_player; stock by city tier
- [ ] Thieves: steal mechanics, night activity, pursuit when caught
- [ ] Travellers + commoners: rumours, alert_guards contagion
- [ ] Reputation system: per-city scalar + NPC memory effects on disposition
- [ ] Quests 1–2 (rat catch, missing caravan) with quest state machine

**Exit**: bribe a Highgate guard, haggle in Millbrook, get robbed in Saltmarsh at 2 AM, and finish the first two quests.

## M4 — Dungeon delving

**Goal**: dungeons + first two bosses.

- [ ] Dungeon trash AI (aggro/retreat), group awareness, leash mechanics
- [ ] Bandit Chief (Burrow): rally/sacrifice, morale effects
- [ ] Wolf Matriarch (Crimson Hollow): pack target selection, howl summons
- [ ] Loot tables, gear tiers, XP/leveling 1–10, respawn rule
- [ ] Quests 3–4 (smuggling, sigil hunt)

**Exit**: clear both dungeons, fight both bosses, collect two Sigils, hit level cap range.

## M5 — The showcase fights

**Goal**: the two decisions-as-content set pieces.

- [ ] Lich (Underkeep): tier-3 strategic loop, tactic choice sub-behaviors, phase system
- [ ] The Adjudicator (Final Trial): player-history summarization into state, mercy/greed/courage ratings, adaptive arena phases
- [ ] Sigil gating + Final Trial unlock

**Exit**: fight the Lich twice with different tactics and watch it adapt; face the Adjudicator after a ruthless run vs an honourable run.

## M6 — Hardening

**Goal**: the demo-ready version.

- [ ] Failure policy end-to-end (AI_ARCHITECTURE §6.1): degraded mode, auto-switch, HUD banner
- [ ] Balance pass: XP curve, gear prices, boss tuning
- [ ] Polish: title screen, death screen, victory screen, keybinding sheet
- [ ] Performance: steady 4 Hz with 200+ NPCs at under 10% of one core (tier-0 must stay pure Rust)
- [ ] Write-up: A/B notes from replay logs (heuristics vs Jev), latency/cost report vs budget

**Exit**: record the 5-minute demo GIF: city social AI → forest ambush → Lich adaptation → Adjudicator verdict, with the Jev inspector open throughout.

## Non-goals (v1)

Saves, magic system, party members, crafting, procedural quests, multiplayer, mouse input. See GAME_DESIGN §11.
