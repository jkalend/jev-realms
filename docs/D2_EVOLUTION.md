# Diablo II Evolution — Design Draft (for iteration, nothing implemented)

Status: **DRAFT v0.9 — design only.** Every open decision is numbered **D1…D36** so we can adjudicate line by line before any implementation.

v0.9 (2026-09-23): research-review round — all nine research files cross-checked against the live code (combat formulas, boss ladder `world.rs:344`, level growth, sellsword spawn, status systems). Fixes: Fensworn's gear-tier penalty cut (no-op — everyone starts at worn) and Redwake's stamina bias given a real knob (§4.2); D34 resolves the D26-vs-§5 talent arithmetic (cap 12 ⇒ 14 points unless levels 11–12 become mastery levels); D35 makes sellsword scaling continuous (amends D22 — at-hire-only invites wait-to-hire); D36 trims the alchemist set and prices the fester status field (amends D32 — professions.md's "we have poison and chill threats" was wrong: no such systems ship); §9 re-synced with D26/D27 (six classes, side bosses placed, new E7 campaign wave); Vael↔Lich mapping made explicit (§6); Laya-question count corrected 7→14 (§11).

v0.8 (2026-09-23): breadth expansions researched and verdicted (new §8): seal-glyphs/words itemization (D31), NPC-service professions (D32), keep-6-classes + no attribute screen (D33), pockets-not-inflation world scale (D29), baked-chrome HUD (D30). Art-style bake-off rendered (D28 reopened to the owner's pick; supports C). v0.7: art-style becomes a three-arm bake-off (D28) because art is AI-authored and agentic throughput changes the economics — **A grim painterly (D2 realism) vs B chunky cartoon (Rogue Legacy) vs C hybrid "grim-lit cartoon"** (environment grim-baked, actors chunky+outline+brightness-lift, unified by the light/post stack; the Dead Cells / Blizzard readability-rule hybrid). Bake-off = same-scene sheets from the gfxlab harness, decided by screenshots like D25. v0.6: art-fidelity doctrine locked (D19): **pre-rendered 3D → baked 2D atlas** (the actual D2/Titan-Quest-era sprite recipe — cheap source art, bake offline, engine reassembles), explicitly chosen over Titan-Quest-style real-time 3D (Target A economics) and BG/Pathfinder/Disco-style per-scene painted plates (structurally incompatible with a procedural, seed-generated world; recipe and reasoning in §7.3, D19). Sprite fidelity is independent of the Target C light/post wrapper — renderer unchanged, the atlas gets expensive. v0.5: campaign extended from one sigil-hunt to a six-act arc (*The Vigil's Second Watch*, §6.5) — lore-consistent antagonist (the Oathless Curate), class roster 4→6 (§4.2), boss roster 7→11 (8 main-arc + 3 side, §6), class quests (§6.5); dimensionality evidence rendered from the real art (`docs/gfx/proto/proj-*.png`) — isometric target with extruded-diorama bridge (D25, §7.6); level cap 10→12 consideration (D26). v0.4: §4.5 balance validation against the shipped combat math (modelled, all four classes × L1/4/7/10 × skirmish/dungeon/bosses): Sigil-Sworn mage curve confirmed as designed; three adjustments adopted as amendments — A1 Keepwarden bias cut + recoil cap (D21), A2 sellsword hire-level scaling (D22), A3 momentum-buys-flee (D23); channel cadence rule (D24). Engine alternatives comparison landed (research/engines.md): Bevy confirmed over Godot+gdext/Fyrox/wgpu/ggez/raylib/macroquad (§7.1, D17 unchanged). v0.3: visual-target competition resolved by research (visual-goals.md): **Target C "HD-2D pixel + lights" recommended** over Grim-Dawn-3D and painted-iso (D20, §7.3–7.4 added); Darkest Dungeon I/II thematic transfers folded in; §7.5 camera question re-scoped. Bevy research integrated (bevy-engine.md): platform facts in §7.1, light implementation routes in §7.4, sharpened E0 gate. v0.2: classes reworked around world lore (§4); per-class signature meters with distinct colors (§4.3, D15); Bevy migration authorized (§7, D17).

Context: the campaign, windowed sprite view, boss relics, and the higher-resolution art pass are all delivered (ROADMAP M0–M13). This document proposes the next arc inspired by Diablo II: character creation, classes with distinct mechanics, talent trees, more bosses (4 → 11: 8 main-arc + 3 side), a longer six-act campaign, and a D2-grade visual identity. It amends [GAME_DESIGN.md](GAME_DESIGN.md) §11 scope guardrails and extends [VISUAL_ROADMAP.md](VISUAL_ROADMAP.md).

Research backing (with per-claim sources, kept separate so this file stays a design doc):

- [research/d2-visuals.md](research/d2-visuals.md) — rendering identity, palette/lighting, UI language, transferability analysis
- [research/d2-bosses.md](research/d2-bosses.md) — act bosses, superuniques, affix systems, potion/regen economy
- [research/d2-classes-talents.md](research/d2-classes-talents.md) — the 7 classes, skill-tree structure, progression numbers

---

## 1. What we take from Diablo II (and what we leave)

D2's feel rests on four systems, all confirmed by the research:

1. **Class = a different game.** Seven classes each have a distinct resource loop and unit-control scheme (summoner army vs. aura swapping vs. charge-up combos), not just different starting stats.
2. **Talent trees create identity through scarcity.** 3 trees × ~10 skills, level-gated tiers, ~1 point per level, 20-point skill cap, one almost-free respec. You finish with a build, not a checklist.
3. **Bosses are checks with one readable signature.** Andariel = poison-resist check, Duriel = gear check in the smallest room, Mephisto = range-inversion check, Diablo = telegraph-dodge check, Baal = attrition check. Every boss is knowable by its one move.
4. **The look is darkness + discipline.** Desaturated low-value environments; saturation is *spent* on units, magic, UI, and loot. Monsters emerge from pitch black at the edge of your light radius. Item rarity is a color language.

Left behind in v0.1 on engine grounds (macroquad): isometric projection, real dynamic lighting, pre-rendered/high-detail sprite assets. **v0.2 lifts these three**: the owner has authorized a heavier engine (Bevy) to reach D2-level detail (§7, D17), which puts iso projection, GPU lighting, and authored asset pipelines back in scope. Still out on design merit regardless of engine: fixed-resolution identity, inventory Tetris.

## 2. Pillar compatibility

| Pillar (GAME_DESIGN §1) | Compatibility |
| --- | --- |
| **Decisions you can watch** | Preserved. Classes/talents are *player-side* deterministic systems. Each new boss adds 1–2 narrow Laya questions (§6), so new behavior stays inspectable in the `J` inspector. |
| **Laya judges, Rust simulates** | Preserved. Talent math, class passives, boss telegraphs, and damage are Rust tables and enums. No Laya question generates prose or stats. |
| **Session-scoped, one run** | Already amended by v2 saves. Class choice lives in the save; death/respawn rules unchanged (class persists). |

## 3. Character creation

Current flow: `Game::new(seed)` → `Modal::Title` → Enter applies the single fixed loadout (world.rs:750-830, input.rs:57). One seam serves both views: terminal (main.rs:161) and windowed (gui.rs:235) both pass through Title.

**Proposal.** After Enter on "New Journey", a `Modal::Create` (one screen, same menu widget family as Pause/Trade) offers:

- **Class choice** (six orders, presented as three callings of two orders each per D33-flavor: Ward — Keepwarden · Gravebound · Hunt — Redwake · Waysworn · Speaker — Sigil-Sworn · Fensworn) with a two-line identity blurb and the starting-stat/mechanic diff previewed.
- **D2: optional origin boon** — one of two minor bonuses (e.g. "+20 gold" / "+1 max stamina"). Skippable; adds replay texture at near-zero system cost.

On confirm: stat bias + class signature are applied to the fresh `Player`, `modal = None`, turn 0 begins. Deterministic per seed as today — creation changes only the player's starting sheet.

- **D3: no name input.** Names are pure UI cost (no text-entry widget exists) with no system payoff; the seed already names the journey (`saves/journey-<seed>.json`). Classes are referred to by class name ("the Keepwarden").
- **Back-compat:** `Player.class` is a new `#[serde(default)]` field; old saves deserialize as `Class::None` and play exactly as today. *(v0.9: D4 ratified hidden — the "no class" option is not offered at creation; `Class::None` survives only as the old-save default.)*

## 4. World lore and class identities

D2 research takeaway (*research/d2-classes-talents.md*): classes diverge by **resource loop + unit-control scheme** — and each class is a *people*, not a loadout. Ours must be rooted in the world that already exists (the three cities, the three dungeons, the Sigil hunt, the oracles, the judging Adjudicator). First the canon skeleton that makes the old content and the new classes one world; then the classes themselves.

### 4.1 Canon skeleton — "The Vigil" *(new lore; D16)*

> Long ago, three calamities were bound under three seals — the **Sigils** — by a sworn order, **the Vigil**. The seals were set in the deep places: a warren, a hollow, a keep. Seals fade with their keepers, and the keepers are gone.
>
> What remains: the **oracles**, the Vigil's remnant priesthood, who still teach fragments of seal-script to travelers (today's rune tuition); the **dungeon lords**, each grown fat on a fading seal's power (the Chief, the Matriarch, and **Vael** — once the Vigil's own garrison castellan, who would not abandon his post even in death); and **the Adjudicator**, the Vigil's last judgment, waiting to weigh whoever gathers the three seals and asks what they mean to do with them.
>
> A journeyer drawn to a fading seal is called **vigil-touched**. That is the player. Death is not the end for the vigil-touched — the seal-work refuses to let them rest (fits the existing respawn). The relics are splinters of seal-power the lords can no longer hold.

**(Extended in v0.5 — the covenant-breaker, D26-adjacent lore.)** The seals did not merely *fade*: their maintenance rites were deliberately broken by the Vigil's last high curate — **the Oathless Curate** — who judged the covenant itself an oath too proud to keep. Vael's garrison sealed him into the Underkeep's heart and the castellan took the eternal watch over the cell *("the Last Castellan would not abandon his post" — canon already says it; now it says why)*. **The three sigils the player gathers are the keys to two doors: the Final Trial, and the Curate's cell.** The Adjudicator knows. It is not guarding a trial — it is recruiting an executioner, or a new Watch.

This is deliberately thin — it *names* what the game already shows (sigils, oracles, Vael's title, a boss who judges instead of hates) without rewriting a single quest, name, or map. Every class below is an order that exists because the Vigil fell.

### 4.2 The six classes

Reusable mechanical loops (unchanged from v0.1): stamina/melee, mana/runes, the single companion slot, and speed/flank economy. The lore gives each loop a face:

| Class | Order & lore | D2 ancestor | Starting bias | Signature mechanic (exclusive) |
| --- | --- | --- | --- | --- |
| **Keepwarden** | The Last Castellan's garrison. When Vael held the Underkeep for the Vigil, his soldiers drilled the brace-and-stand that let exhausted men hold a gate for three days. The manuals survive in the keep's outworks; the Keepwardens are those who still swear *the gate holds*. Highgate guards respect them; Vael remembers them. | Paladin / Barbarian | +4 max HP; −2 max mana *(+1 DEF dropped in validation — A1/D21)* | **Bulwark oaths** — Defend (`F`) becomes a stance pick (Hold the Gate: +50% DEF, first attacker per round takes 1 recoil *(cap: A1/D21)* · Break Their Line: −DEF, +2 next strike · Breathe: no bonus, +1 stamina back). Turns an underused verb into a tactical core. |
| **Sigil-Sworn** | A schism of the oracle priesthood: those who chose to *wield* the seal-script rather than merely guard it. Oracles teach them their first rune freely — and never forget that they read it wrong on purpose. The scholar-caste of a dying order, carrying candles into the dark the seals once lit. | Sorceress | starts with *Spark* + 4 max mana; −3 max HP, −1 ATK | **Channel** — casting the same rune on consecutive rounds builds +1 charge (max 2); each charge +1 spell damage. Moving or meleeing breaks it. Rewards held ground. |
| **Fensworn** | The hollow folk. Before the seals faded, the trappers between Millbrook and the Crimson Hollow kept the old truce: *the pack does not hunt you if the pack knows your name*. They walk with beasts as equals; the Matriarch's fall grieves them even when they deal it. A Fensworn's sellsword is not hired muscle — it is a sworn partner. | Druid / Necromancer (lite) | companion hire cost 25 (vs 50), companion +25% HP *(v0.9: the drafted "one gear tier below standard" penalty is cut — the tier below standard is worn, which every class starts at: a no-op; the hire-economy identity carries the balance)* | **Bond** — companion strikes a second time every third round and never needs the post-combat revive rest. Talents extend the truce to one summoned wolf late-game (D5). |
| **Redwake** | Saltmarsh gutter-royalty: smuggler crews turned licensed bounty-takers. The captains post the contracts; the Redwake collect. They fight like the tide fights the dock — never where you push, always where you aren't. The watch denounces them by day and pays them by night. | Assassin / Amazon | +2 speed, +1 stamina on the wait and brace restores (3/4 vs base 2/3 — v0.9: the engine has fixed restore amounts, not a per-class regen rate); −1 DEF | **Momentum** — each kill or successful flee grants 1 momentum (max 3); attack consumes all for +2 damage each. Hit-and-run economics around the existing flee/flank rules. *(A3/D23: 3 momentum may instead buy a guaranteed flee.)* |
| **Gravebound** *(v0.5)* | The Oathbound Speakers — keepers of the garrison's funerary rites. When Vael's soldiers fell holding the Underkeep, someone had to speak the names so the dead would *stay* dead. The Gravebound still kneel at fresh barrows with candle and writ; the restless dead recognize the oath and hold their claws — mostly. They walk toward what the living flee, because somebody must. | Necromancer (mid) / Death Knight | +2 max HP; −1 speed | **Last Vigil** — power scales as HP falls: below 50% max HP gain +2 ATK; below 25% gain **+4 ATK** (replaces the +2). *(Modelled v0.5: the deep tier originally also carried +2 DEF — cut, because the class only works if the deep window stays lethal. Fight harder, not safer.)* Last-stand economics against the game's respawn rules — dying is information, not failure. **Undeader's peace** (passive): skeleton-type NPCs start neutral once per map (parley possible — fights become choices). |
| **Waysworn** *(v0.5)* | The Vigil's road wardens — the ones who kept the oathroads open so seal-keepers could march between the deep places. For they fell, the fords still remember their toll-lines, and the caravan code outlived the covenant that wrote it. A Waysworn reads a road the way a sailor reads water; ambushes are weather, not surprises. | (no D2 ancestor — the ranger/drifter slot) | +1 speed, +2 max stamina; −2 max HP *(−1 ATK cut in validation: it gutted the L1 tutorial clear, 6.7→10.0 rounds — the class's price should be fragility, not impotence)* | **Open Road** — after each kill, the next move costs no terrain penalty and refunds 1 stamina (road-tempo combat: kill, shift, strike). Road and ford traversal costs halved; caravan/escort/cull bounties pay +25% (the road pays its own). |

All six keep the universal verb set; class exclusivity is the signature + the talents (§5), not removed verbs.

### 4.3 Signature resource meters (new, D15)

HP red / stamina green / mana azure are universal (shipped 2026-09-23). Each class additionally visualizes its signature counter as a **class-tinted meter** on the HUD gauges row — the D2 "your build has a color" read, and the requested distinct resource colors:

| Class | Meter | Hue | Shows |
| --- | --- | --- | --- |
| Keepwarden | **Resolve** | amber gold | active oath + a thin bar building 0–3 while braced |
| Sigil-Sworn | **Channel** | ice cyan | 0–2 charge pips, draining glow when about to break |
| Fensworn | **Bond** | viridian violet | heart icon + cadence countdown to the companion's double strike (3→2→1) |
| Redwake | **Momentum** | rust orange | 0–3 momentum pips, lit when spendable |
| Gravebound | **Vigil** | pale ivory | last-stand tier glyph: dim (≥50% HP) · lit (<50%) · blazing (<25%) |
| Waysworn | **Trail** | olive tan | hoofprint pips: tempo-ready (free move) vs spent; road-discount ribbon while on road tiles |

Meters are *visualizations of counters the signatures already define*, not a new spending economy (D15): no mana-split, no second-consumable bookkeeping in v3. Exact RGBs at implementation beside the existing Ink palette (palette supports `Ink::Rgb` if the 16 colors are insufficiently distinct from HP/stamina/mana).

### 4.4 Class rules (unchanged from v0.1)

- **D1: 4 classes** vs 3 (cut Fensworn — its wolf is the only new-simulation-content ask). Recommendation: keep 4, wave-pack the wolf separately (D5).
- **D6: spell access gate.** Universal rune learning; Sigil-Sworn starts ahead (and oracles waver their first tuition — lore-consistent). Runes stay ungated; shrine content works for all classes.
- **D7: stat floors.** Signatures never drop a stat below viability (Sigil-Sworn keeps Mend access; Keepwarden keeps 2 minimum mana).

### 4.5 Balance validation (v0.4 — modelled against the shipped combat math)

Method: numeric model re-implementing the real formulas (`attack_power = attack + 3·weapon`, `defense_power = defense + 2·armour`, melee `max(1, atk−def)`, flank ×1.25, defend ×1.5 DEF, attack −2 stamina, Spark `max(4, attack_power)` ignoring armour, level growth +6HP/+1ST/+1ATK@even/+1DEF@3/6/9), run at L1/4/7/10 against at-level bandit pairs, trash pairs, and the four bosses at their natural quest levels (Chief L5, Matriarch L7, Lich L9, Adjudicator L10), assuming forge cadence worn→masterwork ≈ tiers 0/1/2/3. Excludes potions, flanking position play, and companion death — so "DIES" below means *potion-reliant*, matching the shipped game's intended consumable pressure (the baseline Wanderer also shows DIES at three of four bosses).

Outcome — skirmish clear-time (rounds, lower = faster) and rounds-survived:

| L | Wanderer | Keepwarden | Sigil-Sworn | Fensworn | Redwake |
| --- | --- | --- | --- | --- | --- |
| 1 | 6.7 / 5.0 | 4.4 / 12.0 | 4.6 / 4.2 | **2.1 / 10.0** | 3.3 / **3.3** |
| 4 | 5.4 / 9.5 | 4.5 / 21 | 5.5 / 8.8 | 4.1 / 19 | 3.8 / 6.3 |
| 7 | 5.6 / 14 | 4.9 / 30 | 5.3 / 13 | 5.1 / 28 | 4.3 / 9.3 |
| 10 | 5.3 / 19 | 4.8 / 39 | 4.8 / 18 | 5.4 / 37 | 4.4 / 12 |

Boss clear-time (rounds): Chief 10.7 / **9.1 K** / 9.7 / 9.1 / 7.1 · Matriarch 9.6 / 8.6 / 8.3 / 9.9 / 7.2 · Lich 12.0 / 10.8 / **9.7** / 14.7 / 9.0 · Adjudicator 12.3 / 11.4 / 9.9 / 14.3 / 9.9 (Wanderer / K / S / F / R).

**Findings:**

- **F1 — Sigil-Sworn matches the intended mage curve.** Weakest early clear without mana, fragile throughout (−3 HP shows: lowest survivability of the quartet at every checkpoint), catches baseline by L7 and wins boss clears by ~19% at the Lich through armour-ignore + channel. Weak-early/strong-late: confirmed; numbers kept as drafted.
- **F2 — Keepwarden strictly dominates.** Clears faster than base *and* never dies in the model from L1 (recoil + breaker offset the stance tax; the +1 DEF bias is pure surplus). Its stated cost (−2 mana) binds only after runes exist. **Adjustment A1 (D21):** drop the +1 DEF bias (keep +4 HP / −2 mana); cap Oath recoil to the **first** attacker per round (no per-attacker AoE recursion).
- **F3 — Fensworn is broken-early, dead-late.** The fixed level-3 sellsword (ATK 5 vs wilderness DEF 0–1) doubles party DPS at L1 (2.1-round clears) plus bodyguard-soaks a second attacker — then the same unscaled ATK is worth 1–2/round vs Lich/Adjudicator DEF 4–5 and the class clears *slowest* there. The fantasy is fine; the sellsword economy is the flaw. **Adjustment A2 (D22):** sellsword hire level scales to the player's level at hire (stats from the existing catch-all formula, benefits every class); Bond keeps +25% HP as the class's edge. Honesty note: the 25g Fensworn hire *is* reachable immediately (40g start, sellsword spawns in the starting city), so the strong first hour is accepted as identity ("smooth wilderness start"); mitigation is that an at-level-scaled companion at L1 has ~14 HP and dies fast in real fights — protecting it costs the Fensworn the actions the model gives it for free. E5 playtest watch item.
- **F4 — Redwake's boss model is assurance-only.** Momentum (kill/flee-granted) does nothing inside a solo boss duel beyond the +6 first-strike burst brought in from scouting; combined with −1 DEF it is the most potion-reliant boss fighter — coherent identity (wilderness assassin, boss=prepared duel) but on the edge. **Adjustment A3 (D23):** allow spending 3 momentum on a **guaranteed flee** — the hit-and-run loop closes inside boss arenas too; keep −1 DEF as the price. Watch item in E5 playtest.
- **F5 — Casting cadence rule needed (D24).** The sustained channel pattern (cast/wait/cast/wait) only works if plain waiting doesn't break the channel; moving and meleeing break it as drafted. Rule: `Wait`/item/defend preserve channel.

None of the classes violates the gameplay systems around them: stances ride the existing Defend code path (×1.5 DEF is already engine semantics), momentum and the flee spend reuse the opposed speed check, channel rides the mana trickle, Bond rides the one-slot companion; and the Adjudicator remains the attrition check for casters (pool 8 + 1/round vs 148 HP is the tightest resource fight in the game — intended).

**v0.5 extension — classes 5–6 modelled with the same harness.** Gravebound: at full HP ≈ baseline (+2 HP neutralizes nothing), <50% tier clears 15–25% faster, <25% tier clears up to 45% faster (2.9 vs 6.7 rounds at L1) — the risk-reward curve works *only if* the deep window stays lethal, hence the +2 DEF was cut from the bottom tier (fight harder, not safer). Waysworn: deliberately the weakest duelist of the six (≈20% slower clears at bias parity) because its power lives outside the duel model — post-kill free moves (initiative theft the model can't price), half-cost traversal, and +25% bounty economy that compounds through the whole campaign; bias penalty switched from −1 ATK (strangled the L1 tutorial: 10.0-round bandit pairs) to −2 max HP. Both stay on the E5 playtest watch list.
**v0.9 review note.** Adjustments A1–A3 (D21–D23) were adopted without re-running the numeric harness; E5's rebalance pass re-runs the model with all three applied (Fensworn's early curve under D35's continuously level-matched companion, Redwake's guaranteed-flee spend inside boss arenas). The harness formulas were re-verified against the shipped code this round: attack_power, defense_power, flank ×1.25, brace ×1.5, Spark floor 4, the growth table, and the boss ladder all match.

## 5. Talent trees

Scale truth: D2 hands out ~110 skill points over 99 levels across 30-skill classes; we have **11 level-ups (1→12, D26 — 9 of them talent-bearing, L11/12 are mastery levels per D34)**. Trees must be D2-shaped, not D2-sized.

**Proposal — "three branches of four":**

- Each class gets **3 branches × 4 nodes = 12 talents**, tier-gated at player level **1 / 3 / 6 / 9** — the D2 1/6/12/18/24/30 ladder compressed. (Research suggests 1/4/7/10 for 10-level games; we deliberately gate the capstone at 9 so it sees real play before the level-10 Final Trial instead of arriving at the finish line.)
- **Point income: 1 per level (9) + 1 per boss Sigil claim (3) = 12 total.** Boss points make the sigil hunt double as the build arc (D2 quest-bonus points: Den of Evil / Radament / Izual). *(v0.9/D34: under the cap-12 campaign the level count is 11 — levels 11–12 are "mastery levels" (stat growth, no talent point) so the total stays 12; the alternative 14-point income was rejected for dead half-capstones.)*
- **Branch capstones (row 4) require 2 points** — a durable choice, D2's 20-point-max skill in miniature. One full branch therefore costs 5 points (3×1 + 2): 12 points ≈ **two full branches (10) + 2 spare** — one named-build identity plus a support branch, matching D2's "one maxed pair + partial second" meta at our scale.
- Only tier-1 nodes are open at entry; every deeper node requires its predecessor (D2's prerequisite chains compressed — no flat pick-12 freedom).
- **Mini-synergy:** within a branch, node 3 gains a small bonus per node 1–2 point (the 1.10 synergy lesson: invest in your own branch, get compounding returns; cross-branch picks stay viable but weaker).
- **Respec:** one free full reset at any **Oracle** (fits existing NPC role; D2's Akara model), plus one craftable reset from a boss-dropped **Essence** item (D2's Token of Absolution — two new bosses give it two sources; D8).

Example — Keepwarden branch *Bulwark* (illustrative of shape, all numbers draft):

| Tier (level) | Talent | Effect |
| --- | --- | --- |
| 1 | Plate Drills | +1 DEF while defending |
| 3 | Second Wind | Defending restores 2 stamina |
| 6 | Anchor | Immune to knockback/displacement; synergy: +1 recoil per Plate Drills/Second Wind node |
| 9 | Last Bastion (2 pts) | Once per combat: surviving a lethal hit at braced leaves you at 1 HP |

Example — Redwake branch *Assassin*, Sigil-Sworn branch *Storm*, Fensworn branch *Alpha*: same 4-row shape, control over one lever each (mobility / charges / companion).

Implementation surface (from the codebase map): `Player.talents: Vec<u8>` (branch-node indices, `#[serde(default)]`), effect queries beside the existing combat math in engine.rs, and a `Modal::Talents` (key `T`) using the same select-list widget as Cast/Forge. **D9:** terminal and windowed share the tree screen (menus already shared); a graphical tree layout is Wave 4 garnish, not a parallel system.

## 6. Bosses: 4 → 11 (8 main-arc + 3 side)

Existing four stay canonical (rally / pack tactics / strategic reevaluation / run judgment — ROADMAP M4–M5). Research patterns applied (*research/d2-bosses.md* §4): one named signature, one progression check, economy pressure — each new boss also adds **1–2 narrow Laya questions** (pillar 1) and drops a **relic** (M13 pattern extends).

**Main-arc additions** (act placements in §6.5):

| Boss | Where | Signature (one readable move) | Progression check | Relic drop | New Laya questions |
| --- | --- | --- | --- | --- | --- |
| **The Tidemother** — drowned smuggler-matriarch | Saltmarsh docks arena (smuggler questline climax) | **Undertow** — telegraphed pull: every 4th round drags each engaged hero 2 tiles toward the water line (fall in = heavy damage + returned to dock edge) | Positioning economy (Duriel's "circumstance as difficulty": tight arena, pull + adds); crew respawns until she falls **(D10)** | **Saltcrown** — immunity to pull/knockback displacement | `drag_who` (choice) · `sacrifice_crew` (noul) |
| **Cragmother** — ancient ridge bear | Northern ridge den (new small map) | **Avalanche Slam** — 3×3 telegraphed slam on the existing wind-up machinery, leaves 2 rounds of rubble (slow terrain) | Mobility/terrain reading; tight den = our smallest boss room by design | **Stoneheart** — +1 DEF, ignore slow/rubble terrain | `commit_slam` (noul vs reposition) · `guard_cubs` (noul — cubs spawn second phase) |
| **The Pale Stag** — roaming alpha | Bounty escalation: 3 completed cull contracts of one kind trigger a captain tip-off | **Break** — below 50% HP it *disengages and flees*, regenerating (the D2 regen/TP-reset dynamic turned against the player); must be cornered or burst down | Pursuit control & burst check — first boss where **retreat is the enemy's move** | **Hartshorn** — +1 speed; fleeing enemies never recover while you pursue | `break_and_run` (score → direction) · `stand_ground` (noul while cornered) |
| **The Oathless Curate** — the covenant-breaker (v0.5) | Underkeep depths (2 new floors below Vael; act V) | **Unwrit** — each third of his HP, he *unwrites one of your three sigils* (its passive blessing and door-power die for the rest of the fight — the build-integrity check: walk in with three pillars, fight uphill with two, then one) | The campaign's hardest test: a build leaning on one sigil relic must prove it stands without it | **The First Writ** — counts as the respec essence mouthpiece (D8) and unlocks the epilogue stance (§6.5) | `which_sigil_falls` (choice, telegraphed 2 rounds ahead) · `mercy_for_the_oathbreaker` (noul — feeds the Adjudicator's final rating) |

**Side bosses** (non-sigil, quest-tied; the D2 superunique slot — fixed spawns teaching one lesson each):

| Boss | Where | Signature | Lesson | Relic drop | New Laya questions |
| --- | --- | --- | --- | --- | --- |
| **Gnaw-Thane, the Rat-King Below** | Fen barrow beneath Millbrook (act I, from the rat-catch arc) | **Plague Tide** — every 3rd round a rat wave re-spawns from barrow holes; bites stack fester (−1 max stamina per 3 stacks, cured at inn) | First summon-economy fight: kill the brood-holes *or* burst the king | **Gnawbone Crown** — rats never aggro; +1 stamina regen in dungeons | `call_the_tide` (noul) · `scatter_when_thinned` (noul) |
| **Tollmaster Grudge** | The ford warlord's camp (act II, caravan line climax) | **Bridge Tax** — crew shoves you toward the water edge (forced displacement + near-drowning tiles) and sows caltrop tiles | First displacement fight: denial-of-position (teaches the mechanic Tidemother graduates) | **Tollcoin Charm** — bribes and tolls cost half; +1 ATK on road/ford tiles | `shove_now` (noul) · `collect_or_cut` (choice: press player or loot the caravan) |
| **Mirelight** — the fen wisp | Deep forest at night only (act III, optional) | **Three False Lights** — splits into three glows; striking the wrong ones heals it; Laya picks the real one's tell each phase | First information fight: watch the *tell*, not the HP bar (pairs with the darkness visual pass — it hunts your light radius) | **Wisplight Lantern** — +2 light radius at night | `which_light` (choice) · `strike_or_subside` (noul) |

- Numbers anchored to the current ladder (`world.rs:344-347`: Chief 64hp / Matriarch 86 / Lich 108 / Adjudicator 148), all draft levers pending playtest: Gnaw-Thane ≈40hp (L2) · Tollmaster ≈70hp (L4) · Cragmother ≈112hp (L6) · Mirelight ≈78hp (L5) · Pale Stag ≈124hp (L7) · Tidemother ≈96hp (L8) · Oathless Curate ≈150hp (L10) · Adjudicator rises to ≈168hp with the cap at 12 (D26). **Vael is the existing Lich reworked** (rename + §4.1 lore, strategic-tier question set and ≈108hp baseline kept pending playtest — he was missing from this list; saves referencing the old archetype get a rename note).

- **D11: sigil gate unchanged.** Main-arc additions drop relics/essences, not sigils; the three-sigil Final Trial arc is intact (the Curate *uses* your gathered sigils against you — he needs no fourth).
- **D12:** gating as tabled + side bosses quest-tied as above.

### 6.5 The long campaign — "The Vigil's Second Watch" (six acts)

The current game is one sigil-hunt (roughly L1–10). v0.5 re-frames it as **two halves**: the sigil hunt (acts I–IV, largely existing content re-sequenced) and **the Stirring** (acts V–VI, new): once the third sigil leaves Vael's hands, the cell it also keys begins to wake. The skeleton key beats you carry home are the same ones the campaign's real final door answers to.

| Act | Name | Levels | Region | Spine | Bosses |
| --- | --- | --- | --- | --- | --- |
| I | The Shire Below | 1–3 | Millbrook & fens | Rat catch → cellar rot traced to a fen barrow → Burrow | Gnaw-Thane (side) → **Red Jack, the Chief** (sigil 1) |
| II | The Ford Wars | 3–5 | Roads & fords | Missing caravan → toll extortion strangling the ford → Hollow | Tollmaster Grudge (side) → **Ashfang, the Matriarch** (sigil 2) |
| III | The Ridge and the Hunt | 5–7 | Highgate & north ridge | Ridge den → bounty board escalation | **Cragmother** · Mirelight (side, optional) → **The Pale Stag** |
| IV | Salt and Smoke | 7–9 | Saltmarsh & docks | Smuggler chain → the tide turns on its own | **The Tidemother** |
| V | The Oathless | 9–11 | The Underkeep | Vael falls (sigil 3) → *the cell wakes*: oracles panic, seals vibrate → two new depths floors → the covenant-breaker | **Vael, the Last Castellan** → **The Oathless Curate** |
| VI | The Second Watch | 11–12 | Final Trial | The Adjudicator weighs the run — now explicitly a *recruitment judgment*; killing it isn't the win, being chosen is | **The Adjudicator** (+ optional cruel mode, D13) |

**Epilogue stance** (campaign length pays off narratively, not just in hours): with the First Writ in hand after the Trial, the Run screen offers the arbiter's question — *Seal the cell forever* (the Curate is unmade; the seals hold unattended) or **Assume the Watch** (the player takes Vael's old post; sets up the seed for a future roguelike+ endgame loop, if one is ever wanted). The Adjudicator's final rating (mercy/greed/courage telemetry that already exists) colors both endings — including whether it judged your `mercy_for_the_oathbreaker` answer sincere.

**Class quests** (one per class, auto-offered in the act that owns the theme; each ends in a custom trial fight that teaches the signature):

| Class | Quest | Offered | Trial fight |
| --- | --- | --- | --- |
| Keepwarden | "The Manual Pages" — recover the garrison drill manual from the Underkeep outworks | Highgate, act III | Hold a gate marker against three waves (Bulwark showcase) |
| Sigil-Sworn | "Read Wrong on Purpose" — reconstruct the forbidden seal-verse | any Oracle, act II | Channel-check duel against a schismatic shade at a shrine |
| Fensworn | "The Old Truce" — renew the truce with a sick wild alpha | Hollow edge, act II | Win by *pacifying* (reduction progress, not kill) — a fight the mercy verb can end |
| Redwake | "The Charter of the Tide" — collect a mark the watch can't touch | Saltmarsh, act IV | Escape an ambush extraction (Momentum/flee showcase) |
| Gravebound | "The Last Watch's Name" — learn Vael's true name to lay the garrison to rest | Underkeep entrance, act V | Survive a deliberately lopsided hold by riding Last Vigil windows |
| Waysworn | "The Oathroad" — re-walk the old vigil road with a caravan | road caravan, any act ≥II | Complete an ambushed escort without losing the cargo |

Class quests are content-small (one chain, one arena each) but they are what makes the roster *diegetic* — the lore orders teach you how to play them.

## 7. Visual evolution — target: D2-class density and depth

Full analysis and citations in *research/d2-visuals.md*. The owner's stated goal is not "D2 flavor" but D2-class *density and depth*: today's procedural 16–32-unit rectangle sprites read as a 1990s tile game. The reference frame was widened beyond D2 (Tyranny, Grim Dawn, Hades, Path of Exile, Disco Elysium, Octopath HD-2D, Songs of Conquest, Darkest Dungeon I/II — *research/visual-goals.md*, delivered): the engine decision in §7.1 is target-agnostic (every premium option needs a GPU-class engine), and the target competition itself is D20/§7.3 with a research-backed recommendation. Macroquad (immediate-mode 2D quads, CPU-authored art, no lighting pipeline in our use) has a hard ceiling below every candidate look; v0.2 therefore treats the engine itself as the design decision (D17).

### 7.1 Engine decision

| | **A — Macroquad polish** (v0.1 plan) | **B — Bevy 2.5D** | **C — Bevy 3D world, fixed ortho** |
| --- | --- | --- | --- |
| Ceiling | Dark, disciplined, readable — but visibly "small indie roguelike" | D2-adjacent: lit 2D art with normal maps, particles, post-processing; the *D2R-legacy* feel | D2R feel: real depth, real shadows, "nearly 3D" as the owner put it |
| Tech | Immediate quads; alpha blends only | wgpu via Bevy ECS; 2D lighting (bevy_light_2d or custom material) + normal-mapped sprites + bloom/vignette; egui/bevy_ui for menus | wgpu 3D; orthographic ~30° camera (the D2 camera per the GDC research); 3D terrain prisms + billboarded or meshed actors |
| Art required | More procedural rectangles (same style, more of it) | Hand-authored or AI-assisted sprite sheets + normal maps per tile/actor (new pipeline, new skill) | Low-poly meshes or prerendered 3D→2D sprites (exactly D2's own pipeline, research §1) |
| Migration scope | Zero — extend `sprites.rs` | Sim crates untouched (model/engine/social/persist/laya_client are plain Rust); new Bevy front-end replaces `gui.rs`/`sprites.rs` rendering; ratatui TUI and shared-UI text content stay | Same as B, plus a Z-dimension decision per map (flat + props is enough) |
| Cost | S–M | L — engine spike, asset pipeline, HUD rebuild | XL — everything in B plus 3D content production |
| **Recommendation** | Fallback if B's spike fails | **Adopt (D17) — staged, with an exit ramp** | Defer; a cheap upgrade path exists *from* B (same Bevy app, camera swap) |

§7.2–§7.5 assume B is adopted. The terminal view A is unaffected by any path and stays the default `--view tui`.

**Alternatives comparison** (*research/engines.md*, 2026-09-23): Bevy, Fyrox, Godot 4+gdext, raw wgpu/winit, ggez, macroquad, and raylib-rs compared across lighting/normals/post/UI/editor/FFI/cadence — **Bevy keeps the top slot** (8/10): the only engine that beats its 2D lighting (Godot, first-party PointLight2D) imposes a permanent FFI + dual-language architecture that fights a generated, sim-first, Rust-everywhere project, and its editor's value is authored scenes a procedural roguelike doesn't have; Fyrox is the Rust-native runner-up but loses on the decisive criterion (weaker lit-2D story, far smaller ecosystem). The comparison changes confidence, not the recommendation.

**Platform facts** (*research/bevy-engine.md*, verified 2026-09-23):

- **Version & cadence** — target **Bevy 0.19.x** (stable 2026-08-13; 0.20-rc cut 2026-09-15). Every `0.x` is breaking on a 3–4-month cadence and ecosystem plugins (light_2d, hanabi, egui) trail each release: pin one version per wave, migrate deliberately between waves, never mid-wave.
- **Post stack is built-in at 0.19** — HDR Bloom + per-camera Tonemapping + Vignette/lens distortion. Target C deliverable 2 is free engine-side; only grading LUTs are ours to author.
- **2D point lights exist via ecosystem** — `bevy_light_2d` 0.10 (targets 0.19): `PointLight2d` with hard radius cutoff (exactly the D2/DD torch read), `LightOccluder2d` dynamic shadows, camera-scoped ambient. Plugin-lag risk applies.
- **No first-party normal-mapped 2D sprites** — that capability is custom `Material2d`+WGSL (the 0.20 `SpriteMaterial` direction future-proofs this), or comes free via `StandardMaterial` on low-3D geometry under an ortho camera (glTF normal maps native). Staged decision, evidence from E0 (§7.4 item 6).
- **Sim bridge is thin** — `Time::<Fixed>::from_hz(4.0)` + `FixedUpdate` drives the existing sim as a plain function call, or Bevy reads snapshots; no ECS rewrite of model/engine/social.
- **Iteration hygiene is mandatory** — multi-minute clean builds and ~600 transitive deps are the norm: dev-only `dynamic_linking`, `rust-lld` on Windows, and a leaf-crate layout (sim crates never depend on Bevy) from day one. UI route (egui vs bevy_ui) per research §4, decided at E6.

### 7.2 What survives the move (pillar check)

The simulation never sees Bevy. `Game`, saves, the Laya pipeline, and every regression test keep running headless; the Bevy app is a renderer/input layer over the same `Action` verb channel the terminal uses today (model.rs:526). The macroquad view C is frozen in maintenance (kept compiling, no new work) until B reaches visual parity — D18 decides its long-term fate. The visual rule set from VISUAL_ROADMAP changes explicitly: "no new dependencies" and "no shaders" are lifted for the Bevy front-end (owner authorization), and *presentation-only, no sim RNG, no schema change* still binds everything.

### 7.3 Visual target — the competition (D20)

Three candidates distilled in *research/visual-goals.md* §3 (per-game blocks + comparison matrix there):

| Candidate | Recipe | Verdict |
| --- | --- | --- |
| **A — Grim-lite 3D ortho** (Grim Dawn / PoE-1) | Low-poly 3D world + skinned actors, fixed camera, real shadows | Rejected: art-department economics — months of Blender per actor set, discards the existing procedural art, reads "asset-flip indie" without an environment artist |
| **B — Painted iso** (Tyranny / Hades / Disco Elysium) | Hand-painted environments + real-time actors | Rejected: bounded by painting talent (not hireable here) and structurally fights runtime-generated maps (authored scenes vs a procedural roguelike) |
| **C — HD-2D pixel + lights** (Songs of Conquest recipe + Octopath post stack) ★ | Keep the authored procedural pixel art; buy depth with a 3D lighting + post-processing wrapper **driven by sim state**; layer Darkest Dungeon thematic transfers | **Recommended:** the only premium look in the set built by a comparably small team on procedural maps; engineering effort converts one-time into permanent visual ceiling; zero new art assets at entry |

The crucial property of C (visual-goals.md §3–4): the projected light map is sourced *from the game's own data* — torch radius, night cycle, fog-of-war, revealed-but-unseen memory — which the sim already computes. Turn-based cadence means the light layer re-renders on sim ticks only, effectively free even before GPU headroom. Fog and darkness stop being cosmetic and become the identity.

**Sprite fidelity is orthogonal to C (v0.6 clarification, resolves the tension flagged in v0.3).** The light/post wrapper operates on whatever sprites feed it — pixel rectangles today, high-fidelity baked art later. The owner's intent (replace sprites with D2/Titan-Quest-grade fidelity) therefore does *not* re-open D20: it re-scopes the art pipeline (D19, §7.4 item 6). One caveat from the Octopath analysis (visual-goals.md §1.6): fidelity budgets explode through *frame counts*, so baked sprite sheets stay near CT-trigger minimal — idle/walk/attack/hit, 1–2 facings, exactly what a turn-based tactical game tolerates.

### 7.4 Deliverables under Target C

Order = flavor-per-effort; items 1–3 are the new core:

1. **Projected light map from sim state** *(the core deliverable)* — render-to-texture light at tile resolution fed by torch radius, night cycle, vision/fog rules: unexplored is black, revealed-but-unseen is a desaturated "memory" tint, torch decay visibly drains warmth from the frame edge-in. Monsters emerge from black at the light's edge (the D2 read, achieved without D2's pipeline). Implementation routes (bevy-engine.md §2): **(a) custom render-to-texture pass** from the sim's per-tile light/vision bitboard — our data, immune to plugin lag, preferred base layer; **(b) `bevy_light_2d` point lights** at torches/braziers for hard-cutoff pools with occluder-cast shadows — E0 validates whether its shadows read correctly on our tile walls before adoption.
2. **Post stack** — bloom (fed by emissive fire/magic pixels), depth of field, vignette, and per-biome color grading LUTs for the photographic "diorama" frame.
3. **Darkest Dungeon thematic transfers** — torch level drives global exposure/saturation decay (our torch timer is already sim state); morale/stress/boss phases as pose-tint overlays per actor (the pack-morale channel already exists sim-side); staged camera pulse on boss turns; map-in-a-void edge dressing (fog-sliced depth planes at map borders); parchment/black/red UI framing — harmonizes with the existing amber-on-charcoal language.
4. **Rarity color language** — engine-independent; may land *pre-migration* as an early win: worn = grey-white, standard = blue, fine = yellow, masterwork = dark gold, **relics = set-green**, quest items = magenta across all item surfaces, mirrored in the terminal's true-color output.
5. **Orb gauges + bottom frame** — red life orb left, azure mana orb right, thin green stamina between; §4.3 class meter docked alongside in its class hue. Numbers/thresholds identical to the terminal.
6. **Sprite upgrades — AI-authored atlas, style by bake-off (v0.7 doctrine).** All sprite content will be authored and iterated by AI models (per owner): procedural-atlas code (like the gfxlab sketches) and/or image-model source plates batch-baked offline into sheets — idle/walk/attack/hit at 1–2 facings (Octopath scope-cut), one light rig, consistency enforced at bake time. **Style is a three-arm bake-off (D28), decided by screenshot, not preference:**
   - **A — grim painterly** (the D2 realism read): baked painterly albedo, natural proportions, grim values.
   - **B — chunky cartoon** (the Rogue Legacy read): exaggerated silhouettes, 1–2px outlines, 2–3 flat tones per material, squash-and-stretch rigs.
   - **C — hybrid "grim-lit cartoon"** *(recommended default)*: environment leans A (grim, painterly-baked light), actors lean B (chunky, outlined, brightness-lifted — Blizzard's own readability rule from the D2R research: units must out-read the ground). Dead Cells is the standing existence proof that this mix holds together.

   Trial rendered (same staged 20×13 scene, iso projection, identical cast): [A painterly](gfx/proto/artstyle-a-painterly.png) · [B chunky](gfx/proto/artstyle-b-chunky.png) · [C hybrid](gfx/proto/artstyle-c-hybrid.png). Findings from the sheets: A is the moodiest but actors sink into the ground at tactical zoom (exactly the failure Blizzard's readability rule warns about); B reads instantly but its inked diamond grid fights the grim tone the DD thematics need; C shows the intended split — grim muted environment with actors visibly lifted and readable. Arm A's caption caveat stands: painterly fidelity in hand-written code has a low ceiling; real baked plates would take A further than this sheet shows. A fourth sheet, [C+ de-blocked](gfx/proto/artstyle-c-plus-deblocked.png), answers the "still looks blocky" follow-up: organic ridge/canopy silhouettes, rubble-stepped walls, scatter tufts and edge-blend speckle — with zero baked art — showing the blockiness was painter-thin, not structural (the iso lattice stays; it stops *reading* blocky once silhouettes are organic and edges blend). Decision closes D28 on the owner's pick; until then the six class cards and eleven boss cards stay design-language, not final art.
7. **Camera at tactical zoom** — ~18–24 tiles across (D2's density lesson: more world, not bigger pixels); a Songs-of-Conquest-style *tilt* variant is an E0 screenshot experiment, not a gate (closes D14 — iso geometry itself is not required by C).
8. **Gothic display font + boss nameplates** — free incised-serif display face for titles/modals/nameplates; gold serif boss name + HP bar on aggro.

### 7.5 Migration shape

Three gates, each independently shippable:

- **E0 — Engine spike (timeboxed, throwaway):** Bevy 0.19 window + world view from a live `Game` snapshot at tactical zoom + the projected light map fed by sim torch/night/vision (route a) + one `bevy_light_2d` torch with an occluder-cast shadow (route b validation) + one actor walking through a lit/unlit boundary + built-in bloom/vignette; optional tilt-camera screenshot variant. Judges: does it read *premium* with zero new art, and do occluder shadows behave on tile walls? If no → path A fallback, revisit C later. If yes → E4.
- **★ E0 PASSED (2026-09-23, `spikes/e0`, throwaway):** both gates YES. [Square frame](gfx/proto/e0-a-lgttorch.png): palette quads + sim-exact light math + route-b torch pool + bloom 0.08/vignette 0.35/exposure ×2.2 reads photographic with zero new art — the warm band/rim/memory tiers carry the frame. [Diorama frame](gfx/proto/e0-b-diorama.png) (shear 0.34, squash 0.58, z-sorted): real depth on the staircase rows. **Route b ADOPTED:** bevy_light_2d 0.10.0 ↔ Bevy 0.19.1 zero-friction; occluder shadows are stable/hard-edged on tile-wall quads. E4 guidance from the run: route a stays the base layer (our data, plugin-lag-immune), route b serves torch/brazier pools (falloff 8–10, intensity 1.2–1.5, ambient 0.08–0.35 — 0.65 washes shadows out); it took six tuning iterations, so budget tuning time in E4. Build fact for the research doc: bevy's `tonemapping_luts` needs an explicit `zstd_rust` (or `_c`) feature. Environment: rustc 1.98.1, cold dep build ~7–8 min, Vulkan on the RTX 4090.
- **E4 — Bevy world parity:** map rendering (fog via the light map, day/night, telegraphs, combat bubble), actors (17 archetypes + gear/phase states), camera (follow/zoom), orbs HUD + class meter, floating text, post stack, DD thematic transfers. Gameplay never blocks on menus — terminal stays fully playable throughout.
- **E6 — Bevy UI parity:** modals (inventory/trade/talk/atlas/cast/forge/talents via egui or bevy_ui — options compared in research/bevy-engine.md §4), atlas view, save/title/creation flow in-window, then `--view bevy` graduates from experimental to the default graphical mode.

**D14 revised:** iso geometry is not required by Target C; a Songs-of-Conquest-style camera *tilt* is an E0 screenshot experiment, decided from evidence, not preference.

### 7.6 Dimensionality — rendered evidence (D25)

Evidence was rendered from the real art plus staged projections of one 20×13 map (prototype `src/gfxlab.rs`, throwaway tooling — §Concept gallery for the full card set):

| Variant | Evidence | Read |
| --- | --- | --- |
| A — square top-down (today) | [proj-a-square.png](gfx/proto/proj-a-square.png) | Honest baseline: readable, flat, the "1990s" the owner wants gone |
| B — isometric 2:1 (the D2 read) | [proj-b-iso.png](gfx/proto/proj-b-iso.png) | Most premium per pixel: real depth from geometry alone; seamless ground; blocks/walls must be re-authored per face |
| C — compressed-tilt 2.5D | [proj-c-tilt.png](gfx/proto/proj-c-tilt.png) | Muddy middle: dimmer far rows but no meaningful depth win over A |
| D — extruded pseudo-3D diorama | [proj-d-extruded.png](gfx/proto/proj-d-extruded.png) | Strong diorama read on trivial square logic; z-order fussy at wall edges |

**Recommendation (D25): B is the projection target, D is the bridge.** The D2 read is projection-led, and B delivers it *without* touching the sim grid: iso is a render-time transform (`screen_x = (x−y)·s`, `screen_y = (x+y)·s/2`), input pick-mapping mirrors it (the `map_view` seam lesson already teaches this), and Bevy gives it via `bevy_ecs_tilemap`'s diamond iso or a small custom transform (bevy-engine.md §2e). The real cost is painter re-authoring for 13 tile types (blocks/forests/mountains/walls per face — the mock shows exactly which), scheduled inside E4 with the art-pipeline doc (D19). C is rejected. **D ships as the E0 comparison screenshot and the honest fallback**: if bespoke iso art fights the pipeline mid-E4, the diorama keeps ~80% of the depth feeling with none of the painter rewrite.

This closes the D14 sub-question with evidence: tilt is not enough (C), iso is worth its painter cost (B), and the no-regret first step (D) needs no new art at all.

## 8. Breadth expansions (v0.8 research round)

Owner's expansion list investigated (research: [itemization.md](research/itemization.md), [professions.md](research/professions.md), [class-structure.md](research/class-structure.md); map-scale and UI-chrome analyzed in-house). Verdicts below as new decisions, each with guardrails. Waves stay untouched until adjudicated (§9).

### 8.1 Itemization — seal-glyphs and words (D31)

Adopt the research's recommendation: **socket-and-word crafting, Diablo II's runewords re-scaled to 12 levels**, retextured onto our own lore to avoid the relic collision and the rune/sigil naming confusion:

- **Seal-glyphs** — glyph drops from elites/bosses/shrine finds (sigils stay untouched; oracle runes stay spells). Each glyph is a named fragment of seal-script (fits §4.1 canon: the orders read fragments of the same script).
- **Sockets** on gear: 1 socket on *standard*+, 2 on *fine*+ — **never on boss relics**: words are the plain-gear chase, relics stay supreme (D2's own rule — words on non-magic bases only — re-proved by the research as the anti-collide guardrail).
- **Words** — exact ordered glyph sequences socketed in order (D2 runeword rules: order + exact socket count). ~6 words at campaign scale, each discovered by drop/quest scroll (deterministic chase, no affix RNG — our loot volume is too small for random-affix pools): e.g. **Vigil** (seal·gate → +1 DEF, once/combat brace ignores first hit), **Ember** (ash·fen → attacks scorch), three more per class flavor + one utility.
- **Socket service** — a smith-side "punch" (75g) so socket count isn't pure loot luck.

Rejected with reasons in the research file: random affix pools (need loot volume a 12-level campaign doesn't produce), +1…+N linear ladders (replaces rather than extends the 4-tier forge), D3/D4 re-roll enchanting (gambling UI doesn't fit the inspectable pillar: every number must be explainable).

### 8.2 Professions — NPC-service only (D32)

Adopt the research's recommendation; the social sim is the natural home:

- **Extend the smith** (existing forge tiers) and **add an Alchemist NPC** per big city: brews the counter-potion prep class the *new* boss designs assume — **anti-toxin** (fester cure/prep, Gnaw-Thane's Plague Tide) and a **mana tonic** (+4 mana — the Sigil-Sworn's mid-fight runway). *(v0.9/D36: the warming draught is cut — no chill mechanic ships or is scheduled; Cragmother's rubble is slow terrain, not cold. professions.md's "we have poison and chill threats" was wrong — the Matriarch fights by pack tactics and the Lich's curse is −2 DEF; the first poison-class threat is Gnaw-Thane himself, so the alchemist ships with him in E5. Fester stacks are a new Player status field — `#[serde(default)]`, one more save-schema line.)*
- **One quest-gated masterwork forge beat** (Charsi-imbue/ADOM-anvil pattern): a one-per-run service that upgrades one piece by hand — teaches "crafting exists" without a tree.
- **Discoverable transmute set** — ~6–8 cube-style recipes (3×potion→greater, rations+ration→traveler's, gem dust→glyph shard), learned permanently when first performed (Qud recipe memory); a codex tab lists learned recipes.
- **No player professions, no gathering nodes.** Where gathering would live, use **one-shot seeded scrounge pickups** (herb clusters in forest tiles, ore flakes on ridge/mountain) that feed the alchemist — zero grind loop, fully explainable spawns.

### 8.3 Class architecture — keep six, no attribute screen (D33)

Research verdict **A**: the 12-point talent economy (§5) **cannot afford branches-as-specializations** — specialization must equal *choosing* a branch, which the first 5-point branch already delivers at the right pacing (the BG3-L3 beat falls naturally inside it). Boss fights and lore class quests are already authored per-class; a 3×2 fold would rebalance every fight for zero player-visible gain. Optional pure-flavor "calling" groupings (Ward: Keepwarden/Gravebound · Hunt: Redwake/Waysworn · Speaker: Sigil-Sworn/Fensworn) may present the roster in-universe as three callings of two orders each — zero mechanics, just creation-screen texture. **No allocatable attribute screen**: ATK/DEF/SPD growth, talent branches, and gear tiers are already three build channels; a fourth (D2's 5-points-per-level) drowns a 12-level UI for no new decisions.

### 8.4 World scale — pockets, not inflation (D29)

In-house analysis rather than research: the overworld's 32,000 tiles (~125 KB, memory irrelevant) serve a ~2,600-action campaign — the world is already *sparse per action*. Area is not a constraint; **content density is**. Perf scales with population (8 µs/tick at 258 NPCs), not area. Crossing on roads already costs 17–25 s held movement, and caravan travel exists for hauls. Verdict: **no overworld inflation**; the world grows *downward and inward* — the act-driven pocket maps already designed (fen barrow, ridge den, docks arena, two Underkeep depth floors) plus optional post-campaign "Watch bounties" zones if E5 runs late. Any future size increase rides with named content, never ahead of it.

### 8.5 UI chrome — the crafted HUD (D30)

The "less pixely gauges" ask folds into §7.4 items 3/5 and is adopted as doctrine: the windowed HUD graduates from shared text-bars to **baked chrome**: the carved bottom frame, life/mana orbs (their fills use the exact gauge ratios), thin stamina ribbon, and the §4.3 class-meter chip, all drawn as assets from the same bake pipeline (D19) so frame style matches chosen art arm (D28). Boss nameplates in the display serif (§7.4-8). **Terminal view keeps its text gauges** — same numbers, one source, per the shared-UI rule (VISUAL_ROADMAP rule 2); chrome is presentation.

## 9. Sequencing proposal

| Wave | Contents | Size | Depends on |
| --- | --- | --- | --- |
| **E1 — Creation & classes** *(DELIVERED 2026-09-23, ROADMAP M14)* | `Modal::Create`, all six class sheets and signatures (§4.2, D27 — the bond-wolf summon alone defers to E5, D5), stat application, save field, origin boon (D2), callings presentation (D33-flavor), §4.3 meters in the existing HUD | M | — |
| **E2 — Talent trees** *(DELIVERED 2026-09-23, ROADMAP M15)* | 12-node trees ×6 classes, talent UI (`T`), point economy (9 level points + 3 sigil; D34 mastery levels keep it at 12), Oracle respec, capstones, mini-synergy | M–L | E1 |
| **E3 — Bosses I** *(DELIVERED 2026-09-23, ROADMAP M16)* | Tidemother + docks arena + Saltcrown + 2 Laya questions; Cragmother + ridge den + Stoneheart + 2 questions | M–L | — (parallel with E2 possible) |
| **E4 — Bevy world parity** (D17 adjudicated) *(DELIVERED 2026-09-23, ROADMAP M17: atlas-textured world + route-a/b lighting + post stack, live actors with gear/phase flash, orbs HUD + class meter chip + boss nameplates, `--view bevy` flagship behind opt-in `bevy-view` feature; nameplate bloom-bleed & mandatory single-camera HUD noted for D30 baked chrome; Cinzel font blocked on network — slot accepts drop-in)* | §7.4 deliverables 1–3, 5, 7–8: light map from sim state, post stack, DD transfers, orbs+class meter, tactical camera, font+nameplates; item 6 sprite upgrades via the D19 pipeline; D30 HUD chrome (baked orbs/frame) from the same pipeline. Macroquad C frozen in maintenance | L | E0 |
| **E5 — Bosses II + side bosses + signatures** | Pale Stag + bounty escalation; side bosses Gnaw-Thane (+ fen barrow), Tollmaster, Mirelight; alchemist NPC + anti-toxin with the fester status field (D36); Fensworn bond-wolf; Waysworn bounty-economy pass; Redwake momentum polish; essence respec items; Adjudicator no-retreat hard mode (D13); playtest rebalance incl. re-running §4.5 with A1–A3 applied | L | E1 (wolf), E2 (balance) |
| **E6 — Bevy UI parity** | All modals + atlas + title/creation/save flow in-window (egui or bevy_ui, research/bevy-engine.md §4); `--view bevy` becomes the default graphical mode | L | E4 |
| **E7 — The Second Watch** (campaign completion) | Six-act resequencing (§6.5), Vael/Lich rework + two Underkeep depth floors, the Oathless Curate (Unwrit, First Writ), class quests + trial arenas, epilogue stance, seal-glyphs and words (D31), transmute set + recipe codex (D32), scrounge pickups | L | E5 (content), E2 (talent trials) |

Any wave is veto-able without breaking later ones except E0→E4→E6 and E1→E2→E5→E7. **E0 passed its look test 2026-09-23 (§7.5★) — the path-A fallback is archived, not taken.** If E0 had failed, E4/E6 would have been replaced by the v0.1 macroquad visual pass (kept in git history of this file).

## 10. Decision register (iterate here)

| # | Question | Proposal |
| --- | --- | --- |
| D1 | 4 classes or 3 (cut Fensworn)? | 4 |
| D2 | Origin boon at creation? | **Ratified — yes, minor only** (owner, 2026-09-23): one of two skippable bonuses (+20 gold / +1 max stamina) |
| D3 | Name input? | No |
| D4 | "No class" classic mode selectable? | **Ratified — no, hidden** (owner, 2026-09-23): creation shows the six orders only; old saves still load as `Class::None` (schema default), but new runs pick a class |
| D5 | Fensworn wolf summon: ship-or-cut in v3? | Ship, but isolated in E5 |
| D6 | Runes class-gated? | No (universal, Sigil-Sworn ahead) |
| D7 | Class stat floors? | Yes, listed minimums |
| D8 | Respec sources? | 1 free Oracle reset + boss-essence craft |
| D9 | Graphical tree layout in v3? | No, shared menu first |
| D10 | Tidemother crew respawns? | Yes, capped |
| D11 | New bosses drop sigils? | No — relics/essences only |
| D12 | Boss gating (quest/open/bounty)? | As tabled §6 |
| D13 | Adjudicator no-retreat hard mode? | **Ratified — include in E5** (owner, 2026-09-23): ships with the boss's second pass; tuning + test surface added to the E5 wave |
| D14 | Camera orthogonality (iso/tilt)? | Not required by Target C; SoC-style tilt is an E0 screenshot experiment |
| D15 | Class signature meters (§4.3), counters only, no new spending economy? | Yes, four class hues |
| D16 | "The Vigil" canon skeleton (§4.1) ratified? | Yes (v0.2 proposal; names/suggestions welcome) |
| D17 | Engine path (§7.1)? | B — Bevy 2.5D, staged with path-A fallback if E0 fails (confirmed independently by engines.md alternatives scoring) |
| D18 | Macroquad view C fate after B reaches parity? | Keep frozen in maintenance (revisit at E6) |
| D19 | Art pipeline for E4 assets? | **AI-authored offline bake** (procedural-atlas code and/or image-model plates → sheets; style itself is D28's bake-off). Pipeline doc before E4 fixes the plate source per arm |
| D20 | Visual target (§7.3)? | C — HD-2D pixel + lights + DD thematics (research recommends; A needs a 3D art dept, B needs a painter) |
| D21 | Validation A1: Keepwarden drops +1 DEF; recoil capped to first attacker/round? | Yes (F2 — class strictly dominated otherwise) |
| D22 | Validation A2: sellsword hire level scales to player's level at hire? | Yes (F3 — unscaled level-3 sellsword breaks early-game math) |
| D23 | Validation A3: 3 momentum buys a guaranteed flee? | Yes (F4 — closes Redwake's loop inside boss arenas) |
| D24 | Validation F5: Wait/item/defend preserve channel; only move/melee break it? | Yes |
| D25 | Dimensionality (§7.6 rendered evidence)? | B iso target · D diorama bridge/fallback at E0 · C out |
| D26 | Level cap 10 → 12 for the six-act campaign (§6.5)? | Yes; XP + talent tiers rebalanced in E2/E5 (capstone stays L9) |
| D27 | Class roster 4 → 6 (Gravebound, Waysworn, §4.2)? | Yes (model-checked §4.5) |
| D28 | Art style (§7.4-6): A grim painterly vs B RL chunky vs C hybrid? | **Ratified — C/C+ hybrid** (owner, 2026-09-23): grim painterly environment + chunky outlined actors with brightness lift; the C+ de-blocked sheet is the target read. D19 pipeline doc fixes plate sources for C/C+ only |
| D29 | World scale (§8.4)? | **Ratified** (owner go-ahead, 2026-09-23): pockets/depths only; no overworld inflation |
| D30 | Crafted HUD chrome (§8.5)? | **Ratified** (owner go-ahead, 2026-09-23): baked chrome frame + orbs in GUI; TUI keeps text gauges |
| D31 | Itemization path (§8.1)? | **Ratified** (owner go-ahead, 2026-09-23): seal-glyphs + words; sockets never on relics; ships E7 |
| D32 | Professions (§8.2)? | **Ratified** (owner go-ahead, 2026-09-23; as amended by D36): NPC-service only, ships E5/E7 |
| D33 | Class architecture (§8.3)? | A — keep 6 flat classes; no attribute screen; optional flavor "callings" groupings — **owner ratified the callings garnish for the creation screen** (2026-09-23) |
| D34 | Talent-point income under the cap-12 campaign (D26 vs §5 arithmetic: 11 level-ups + 3 sigil = 14)? | **Ratified — mastery levels** (owner, 2026-09-23): L11/12 grant stat growth only, economy stays 12 = 9+3; keeps the 5+5+2 promise and the class-structure fit analysis valid. (Alternative: 14 = 5+5+4, rejected — the 4th spare point dead-ends half-paid capstones.) |
| D35 | Sellsword scaling cadence (amends D22)? | Re-derive companion stats at every player level-up (and at hire), not at hire only — removes the wait-to-hire meta, keeps Fensworn's early-hire identity |
| D36 | Alchemist launch set (amends D32)? | Anti-toxin + mana tonic; warming draught cut (no chill mechanic ships or is scheduled); fester = new stacking-status field with save note; ships with Gnaw-Thane in E5 |
| D37 | Default presentation on evidence (docs/VISUAL_OPTIONS.md)? | **Ratified — iso default now** (owner, 2026-09-23): `crates/view` ships `Proj::Iso` as the default; square survives as the low-DPI fallback; pick-mapping inversion pre-tested for E6 |
| D38 | Art-source ladder per evidence (docs/VISUAL_OPTIONS.md §C)? | **Ratified — pre-render 3D → plates, terrain first** (owner, 2026-09-24): Mountain/DeepForest batch in `tools/atlas` headless bake (no external assets), then Forest/Wall, then bosses + actor frames; manifest consumers untouched |
| D39 | Full real-time 3D view's fate (docs/VISUAL_OPTIONS.md §D)? | **Parked, NOT discarded** (owner, 2026-09-24): weak at tactical zoom on blockout evidence (`r3d-fullview`); reopen after E7 content with textured mesh candidates — `spikes/r3d` stays buildable as the restart point; set-piece 3D cameras are the plausible re-entry niche |
| D40 | Display-options surface in the Bevy view (E12)? | Runtime toggles B (bloom) / V (vignette) / L (light flicker — F was taken by Defend) + persisted zoom, saved to `saves/view-options.json` (tolerant load, defaults = the E0/E4-ratified frame); gates live in crates/view `options.rs` + `lights.rs` |

## 11. Doc edits required on approval

- **GAME_DESIGN.md §11** — move "spell trees" from *still out* to v3 (class talent trees); add the Vigil canon (§4.1), six classes/creation/class quests (§4, §6.5), the 8+3 boss roster (§6), the six-act campaign and level cap 12 (§6.5), and the v3 visual pass to the delivered-design narrative; keep crafting trees, multiplayer, mouse, multi-slot parties out.
- **ROADMAP.md Non-goals** — same spell-tree move; add milestones M14 (classes/creation, E1), M15 (talents, E2), M16 (bosses I, E3), M17 (engine spike + Bevy world parity + HUD chrome, E0+E4), M18 (bosses II + side bosses + signatures + alchemist, E5), M19 (Bevy UI parity, E6), M20 (the Second Watch: six-act campaign, Vael/Curate, class quests, words, transmutes, E7).
- **VISUAL_ROADMAP.md** — append the §7 engine decision, the D20 target choice, and the §7.4 deliverables as a new chapter; note the "no new dependencies / no shaders" rules are lifted for the Bevy front-end only (macroquad/terminal views keep them), and macroquad waves 1–3 remain valid until E4 supersedes them.
- **AI_ARCHITECTURE.md** — add the 14 new narrow questions (2 × each of the seven new bosses: Tidemother, Cragmother, Pale Stag, Oathless Curate, Gnaw-Thane, Tollmaster, Mirelight — per the §6 tables) to the question-bank table.

---

## Concept gallery (v0.5 — prototype renderer `src/gfxlab.rs`, throwaway tooling)

Cards are **design-language** artifacts: palette, silhouette, hue, and layout are the decisions; the pixel art itself is placeholder until E4 and the D19 art pipeline. Rendered from the real engine and the real seed-42 world. **Engine evidence:** the E0 spike frames live beside these protos — [e0-a-lgttorch](gfx/proto/e0-a-lgttorch.png) (routes a+b + post, square) and [e0-b-diorama](gfx/proto/e0-b-diorama.png) (D25 diorama variant).

**Projections (§7.6):** [A square](gfx/proto/proj-a-square.png) · [B isometric](gfx/proto/proj-b-iso.png) · [C tilt](gfx/proto/proj-c-tilt.png) · [D extruded](gfx/proto/proj-d-extruded.png)

**Art-style bake-off (D28):** [A grim painterly](gfx/proto/artstyle-a-painterly.png) · [B chunky cartoon](gfx/proto/artstyle-b-chunky.png) · [C hybrid](gfx/proto/artstyle-c-hybrid.png) · [C+ de-blocked (target read)](gfx/proto/artstyle-c-plus-deblocked.png)

**Classes (§4.2):**
[Keepwarden](gfx/proto/classes/1-keepwarden.png) · [Sigil-Sworn](gfx/proto/classes/2-sigil-sworn.png) · [Fensworn](gfx/proto/classes/3-fensworn.png) · [Redwake](gfx/proto/classes/4-redwake.png) · [Gravebound](gfx/proto/classes/5-gravebound.png) · [Waysworn](gfx/proto/classes/6-waysworn.png)

**Bosses (§6):**
[Red Jack, the Chief](gfx/proto/bosses/01-red-jack-the-chief.png) · [Gnaw-Thane](gfx/proto/bosses/02-gnaw-thane.png) · [Ashfang, the Matriarch](gfx/proto/bosses/03-ashfang-the-matriarch.png) · [Tollmaster Grudge](gfx/proto/bosses/04-tollmaster-grudge.png) · [Cragmother](gfx/proto/bosses/05-cragmother.png) · [Mirelight](gfx/proto/bosses/06-mirelight.png) · [The Pale Stag](gfx/proto/bosses/07-the-pale-stag.png) · [The Tidemother](gfx/proto/bosses/08-the-tidemother.png) · [Vael, the Last Castellan](gfx/proto/bosses/09-vael-the-last-castellan.png) · [The Oathless Curate](gfx/proto/bosses/10-the-oathless-curate.png) · [The Adjudicator](gfx/proto/bosses/11-the-adjudicator.png)

**Areas (real generated maps, seed 42):**
[The Three Realms (overworld)](gfx/proto/areas/01-the-three-realms.png) · [Millbrook](gfx/proto/areas/02-millbrook.png) · [Highgate](gfx/proto/areas/03-highgate.png) · [Saltmarsh](gfx/proto/areas/04-saltmarsh.png) · [Provisioner's Cellar](gfx/proto/areas/05-millbrook--provisioner's-cellar.png) · [Burrow F1](gfx/proto/areas/06-the-burrow--floor-1.png) · [Burrow F2](gfx/proto/areas/07-the-burrow--floor-2.png) · [Crimson Hollow F1](gfx/proto/areas/08-crimson-hollow--floor-1.png) · [Crimson Hollow F2](gfx/proto/areas/09-crimson-hollow--floor-2.png) · [Underkeep F1](gfx/proto/areas/10-the-underkeep--floor-1.png) · [Underkeep F2](gfx/proto/areas/11-the-underkeep--floor-2.png) · [Underkeep F3](gfx/proto/areas/12-the-underkeep--floor-3.png) · [Hall of Verdicts](gfx/proto/areas/13-final-trial--the-hall-of-verdicts.png)

*Regenerate any card with `cargo run --example gfxlab` (opens a window briefly; exports to docs/gfx/proto/).*

---

*Drafted from research/{d2-visuals, d2-bosses, d2-classes-talents, bevy-engine, visual-goals, engines, itemization, professions, class-structure}.md and the live codebase map (Player struct model.rs:398, level math engine.rs:1338, boss table world.rs:344, verb enum model.rs:526, title hook input.rs:57, sprite seams sprites.rs:12/91/527/1023). Challenge anything — that's what D-numbers are for.*
