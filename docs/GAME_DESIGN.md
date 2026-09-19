# Game Design — Jev Realms

Terminal-only ASCII RPG. Player explores an overworld, three cities, and three dungeons; every NPC is driven by Jev judgments composed with deterministic Rust logic. This document defines the world, systems, and content. AI integration is specified in [AI_ARCHITECTURE.md](AI_ARCHITECTURE.md).

## 1. Pillars

1. **Decisions you can watch.** Every observable NPC behavior traces to a Jev question with a visible probability. When a bandit flees, the player can inspect `flee=0.58` — the AI is a gameplay surface, not a black box.
2. **Jev does judgment, Rust does everything else.** Pathfinding, damage math, world simulation: deterministic. Jev answers narrow questions ("is the player a threat?"), never generates prose.
3. **Session-scoped, one run.** No saves in v1. Death sends you back to the last city with partial gold loss. Keeps scope sane, adds stakes.

## 2. Overview

| Property | Value |
| --- | --- |
| Perspective | Top-down tile grid, scrolling camera |
| Viewport | 60×30 chars centered on player |
| World | 200×160 overworld tiles + 3 cities (≈40×40) + 3 dungeons (≈30×30, 2–3 floors) |
| Turn model | Real-time-with-pause: world ticks at 4 Hz; player input is immediate |
| Combat | Turn-based tactical, resolves on the grid (see §6) |
| Progression | Levels 1–10, XP from combat and quests, gear tiers |
| Endgame | Defeat the dungeon lords, retrieve the three Sigils, survive the Final Trial |

## 3. World

### 3.1 Overworld

One 200×160 map mixing hand-authored regions (roads connect the cities; a river with two fords; a mountain ridge in the north) with procedural fill (forests, clearings, ruins) seeded per-session.

Terrain types and movement/effect:

| Terrain | Char | Cost | Notes |
| --- | --- | --- | --- |
| Road | `.` | 1.0 | fast travel; guards patrol roads near cities |
| Grass | `"` | 1.0 | default |
| Forest | `♣` | 1.5 | bandits and wolves prefer; blocks cavalry-type enemies |
| Deep forest | `♠` | 2.0 | highest ambush frequency |
| Mountain | `▲` | 3.0 | slow; some tiles impassable |
| River | `≈` | — | impassable except at fords (`≈`→`-`) |
| Ford | `-` | 1.5 | bottleneck: classic ambush spot |
| Ruins | `∏` | 1.0 | dungeon entrance stubs, loot shards |

### 3.2 Cities

Three walled cities, each a ≈40×40 interior map with distinct flavor and a Jev personality mix:

| City | Flavor | Notable NPCs |
| --- | --- | --- |
| **Millbrook** | farming hub, friendly | general vendor, blacksmith, guard captain, 4–6 commoners, 1 thief (active at night), travellers |
| **Highgate** | mountain trade post, suspicious of strangers | weapon vendor, armour vendor, innkeeper (rumours → quest hooks), 4–6 commoners, 2 guards (strict) |
| **Saltmarsh** | port city, corrupt | black market vendor (stolen goods), fence, 2 thieves (bold), city watch (bribable via Jev `gullibility`), smuggler questline |

City systems: day/night cycle (2 min real ≈ 1 in-game hour), NPC schedules (vendors open 6:00–20:00, thieves active 22:00–4:00), gates close at night (knock → guard interaction → Jev decides if you're let in).

### 3.3 Dungeons

| Dungeon | Theme | Floors | Boss |
| --- | --- | --- | --- |
| **The Burrow** | bandit warren under ruins | 2 | Bandit Chief (rallies minions via Jev `morale`) |
| **Crimson Hollow** | wolf/beast lair, forest cave | 2 | Dire Wolf Matriarch (pack tactics via Jev target selection) |
| **The Underkeep** | undead stronghold | 3 | Lich (strategic tier — the showcase fight; see AI_ARCHITECTURE §5) |
| **Final Trial** | arena unlocked by 3 Sigils | 1 | The Adjudicator — evaluates *your* play history |

Each dungeon floor is a ≈30×30 generated-from-template map. Dungeon entry requires a purchased or found key/permission; entering at low level is suicide, and the NPCs know it (`fear` gates aggression).

## 4. Player

- **Stats**: HP, stamina, attack, defense, speed. Level 1 start: HP 20, attack 3.
- **Actions**: move (8-way), attack (bump), interact (`E`), inventory (`I`), rest (`R`, risky in wilderness — Jev-watched), wait, flee combat.
- **Inventory**: 20 slots; gold; consumables (potions, rations, torches); gear (weapon/armour tiers: worn/standard/fine/masterwork).
- **Reputation**: per-city scalar 0–100, plus hidden per-NPC memory (see §8). Actions shift both.

## 5. NPC archetypes and their Jev questions

This is the core content table. Every archetype has a deterministic behavior skeleton; Jev fills the judgment slots.

| Archetype | Spawn location | Jev questions (per event) |
| --- | --- | --- |
| Commoner | cities | `flee_from_player` (noul), `alert_guards` (noul) |
| Vendor | cities | `haggle_accept` (noul), `cheat_player` (noul, personality-gated), `offer_quest` (choice: delivery/escort/collection) |
| Guard | cities + roads | `suspect_player` (noul), `accept_bribe` (noul, gullibility-gated), `pursue_fleeing_player` (noul) |
| Thief | cities (night) | `steal_from_player` (noul), `flee_when_noticed` (noul) |
| Traveller | roads | `share_rumour` (noul), `warn_of_danger` (noul) |
| Bandit | forests, fords | `ambush_player` (noul), `flee` (score), `target_pick` (choice) |
| Wolf / Bear | deep forest | `hunt_player` (noul), `flee` (score), `target_pick` (choice: player/packmate/cattle) |
| Dungeon trash | dungeons | `aggro` (noul), `retreat_to_group` (noul) |
| Bandit Chief | Burrow boss | `rally_minions` (noul), `sacrifice_minion` (noul) |
| Wolf Matriarch | Hollow boss | `howl_summon` (noul), `target_pick` (choice) |
| Lich | Underkeep boss | strategic tier: full plan re-evaluation (see AI_ARCHITECTURE §5) |
| The Adjudicator | Final Trial | evaluates player history; asks Jev to score your run (see §10) |

## 6. Combat

Turn-based tactical, initiated when hostiles are adjacent (or when an `ambush_player` succeeds from stealth range).

- **Initiative**: speed-based ordering, player included.
- **Grid**: local 11×11 combat bubble around engagement; positioning matters (flanking +25% damage; back-to-wall prevents flanking).
- **Actions**: move/attack/defend(+50% def one turn)/use item/flee (opposed speed check, Jev `mercy` may spare you... once).
- **Fleeing NPCs**: any NPC whose `flee` score crosses its archetype threshold breaks off; bandits may throw a parting javelin (`spite` personality axis).
- **Death**: NPC death is permanent. Player death → respawn at last city, lose 25% gold, world persists for the session.
- **Group AI**: enemies share a pack channel; if ≥50% of a pack dies, survivors' `flee` scores are nudged +0.2 via the morale modifier (Rust-side rule, not a Jev call — Jev judges the state, Rust applies pack rules).

## 7. Quests

Hand-authored hooks with Jev-mediated delivery:

1. **Rat catch** (Millbrook, L1): vendor wants cellar cleared. Tutorial: combat, loot, reputation.
2. **The missing caravan** (Highgate, L2): find wreck at the ford; fight wolves; recover goods; Jev `return_goods_or_sell_to_fence` decision on the player's conscience path.
3. **Saltmarsh smuggling** (L3–4): multi-step; ends in a choice — report to the watch (reputation) or join the smugglers (gold, future thief access).
4. **Sigil hunt** (L4+): the three dungeons; each Sigil is a boss drop.
5. **The Final Trial** (endgame): arena vs the Adjudicator.

Quest state is a Rust-side enum machine; Jev only ever influences *delivery* (which NPCs offer what, given their memory of you), never the state machine itself.

## 8. NPC memory

Per-NPC record, capped ring buffer of the last 12 significant events involving the player:

```
NPCMemory {
    events: [(turn, kind, weight)],   // e.g. (342, "player_attacked_me", -0.6)
    disposition: f32,                 // running sum, clamped [-1, 1]
}
```

Memory feeds the Jev state (see AI_ARCHITECTURE §3.1), so a guard who watched you bribe a colleague treats `suspect_player` differently than a fresh guard. Memory is session-scoped.

## 9. Rendering & input

- `ratatui` main layout: world viewport + bottom HUD (HP/stamina/gold/level) + right log panel (last 6 events).
- Color: 16 ANSI colors; deep forest darker green, night = dimmed palette + torch radius 6.
- Widgets: inventory modal, vendor trade modal (list-select), NPC dialogue modal (choice list), combat overlay (initiative queue, floating damage numbers as transient text).
- Input: arrows/WASD move, `E` interact, `A` attack adjacent, `I` inventory, `R` rest, `Esc` pause menu, `?` help.
- The **Jev inspector** (toggle `J`): side panel showing the last 10 Jev decisions this session with question, probabilities, and which NPC made them — pillar #1 made literal.

## 10. The Adjudicator (endgame showcase)

The Final Trial boss doesn't fight fair: it reviews your session. A log of your notable deeds (kills, mercy shown, thefts, bribes, fled battles) is summarized into the Jev state, and the Adjudicator's questions are computed against your record:

- `rate_mercy` (score), `rate_greed` (score), `rate_courage` (score) → its opening stance and available attacks;
- Each arena phase asks `adapt_tactic` (choice) against your performance in the previous phase.

Kill-stealing then fleeing every fight? It will fight dirty. Honourable run? It duels you straight. This is the demo-able moment of the whole project.

## 11. Scope guardrails

Not in v1, deliberately: no saves, no magic system, no party/companions, no crafting, no procedural quest generation, no multiplayer. Every one of these is a classic scope-creep trap; the Jev-decision showcase does not need them.
