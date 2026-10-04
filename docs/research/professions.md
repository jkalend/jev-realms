# Crafting Professions, Consumables & Gathering — Research for Laya Realms

Scope: evaluate breadth expansions (professions, consumables, gathering) against Laya's existing systems — NPC-driven sim (vendors/smiths forge gear tiers for gold, oracles as service NPCs), seeded 200×160 overworld + 13 maps, 4 Hz ticks, 10–12 level campaign, "everything inspectable" pillar. Sources: official-class wikis and dev docs. Access date 2026-09-23. Dense notes, not prose.

Companion files: `d2-bosses.md` §3.3 already covers the *fight-role* of D2 potions (heal-over-time vs instant rejuv, merc feeding, TP-restock loop) — this file covers the *catalog and economy* and does not repeat that analysis.

## 1. Crafting profession models compared

| Model | Gating | RNG | Sink role | UI surface | Verdict shared traits |
|---|---|---|---|---|---|
| **(a) WoW player professions** [1] | Recipes by skill threshold; trainer-bought or world-drop recipes; 2 primary professions per toon (unlearning destroys all skill/recipes) [1] | Skill-up roll per craft: grey = 0, green = rare, yellow ≈ 60%, orange = guaranteed skill point [1]; output item itself is fixed per recipe (classic era) | Gold sink via trainer fees/regent vendors; *time* is the real currency; AH converts grind→gold [1] | Professions tab: recipe list colored by skill-up odds [1] | Persistent toon + market economy; nothing for Laya (single run, one PC, no AH) |
| **(b) Grim Dawn NPC crafts** [3][4] | Blacksmith unlocked by Act-1 quest (choose **Duncan or Angrim**, mutually exclusive per difficulty); further smiths per act; recipes = blueprint items consumed on use [3] | Smith crafts "at least Magical quality, with **random affixes plus a bonus property**" tied to the smith (Angrim: pierce res/armor/physique; Duncan: energy regen/DA) [3] | Massive iron-bits + component sink; crafting *materials* converted 3 cheap → 1 rare by cursed smiths; legendary RNG crafting endgame [3] | Crafting window at NPC: recipe list + ingredient check; result unknown until crafted [3] | Crafting as a *person* in town, not a player skill; choice-of-smith is a build question |
| **(c) D2 Horadric Cube** [7] | One quest item held in inventory; zero skill, zero level gate; recipes are fixed formulae, discovered (or memorized) | Output item reroll RNG on some recipes (e.g. "3 PGems + magic item = new random magic item, ilvl = ilvl" [7]); upgrade recipes deterministic | Item sink: turns junk (3 gems, 3 rings, potions) into usable goods; gems/runes double as portable currency [7] | 2×4 grid + Transmute button; recipe list NOT in game — external knowledge is the gate [7] | "Everything in the box, press button, something comes out" — maximal flavor-to-UI ratio |
| **(d) Roguelike-native** | varies below | varies below | varies below | varies below | See subsections |

### 1.c D2 Cube recipe examples (per [7])

1. **3 gems of same type+grade (< Perfect) → 1 higher-grade gem** (e.g. 3 Flawed Rubies → 1 Ruby). Pure consolidation; turns drop density into a progression dial.
2. **3 runes of same type → next rune** (El→Thul chain: 3 Thul → 1 Amn; ladder/SP 1.10 added gem-keyed upgrades: 3 Thul + Chipped Topaz → Amn … 2 Ber + Flawless Sapphire → Jah). Deterministic; makes low runes never worthless.
3. **3 Small Rejuvenation Potions → 1 Full Rejuvenation Potion**; also 3 health + 3 mana + chipped gem → Rejuv; with normal gem → Full Rejuv. Consumable tier-merging.
4. **3 magic rings → 1 random magic amulet** (ilvl = int(.75 × clvl)) and vice-versa — junk jewelry becomes a build-relevant slot.
5. **Ral + Thul + Perfect Sapphire + normal unsocketed helm → socketed helm** (1–3 sockets, capped by item max) — runes as *crafting reagents*, not just socket fillers.
6. **Hel + Scroll of Town Portal + socketed item → clear sockets** (destroys contents) — an "undo" recipe that eats a utility consumable.
7. **Wirt's Leg + Tome of Town Portal → Secret Cow Level** (in town, post-Baal) — jokes hidden in the same grammar as utility.

**Why the cube's flavor works:** (i) gate = *knowledge*, grinded never — no skill bar to fill [7]; (ii) inputs are *junk adjacent to progress* (low gems/runes/potions) so the sink recycles the floor instead of adding a farm; (iii) the same verb (Transmute) spans consolidation, upgrade, reroll, and secrets — one UI, `O(recipes)` content; (iv) inventory-slot tax (cube is 2×2 holding 3×4) makes carrying it a decision, not a given [7].

### 1.d.i Caves of Qud — cooking & psychometry [9][10][11]

- **Cooking**: works at lit campfire/oven; requires *Cooking and Gathering* skill tree purchases (50 sp + INT>15 each): Meal Preparation (2 ingredients), Spicer (3), Carbide Chef (choose 1-of-3 random recipe outcomes when inspired and *memorize it*). Meal grants a **metabolizing buff**; 10% "tastier than usual" roll (+1 stat, +10% HP, etc.). Hunger gates re-cooking; ingredients come from **Butchery** (auto-butcher valid corpses, one part per corpse, meat → 3–10× jerky preserves) [11], harvesting, merchants [9].
- Design read: cooking is a **buff production line with a recipe-discovery minigame**, not a gear path — Carbide Chef's "remember what worked" is Qud's answer to D2's external-wiki problem [9].
- **Psychometry** (mental mutation): touch artifact → identify it and, with Tinkering skill, *learn its build recipe* (complexity capped by mutation level: 4 + L/2 identify, 2 + (L-1)/2 learn) [10]. Recipe acquisition as *detective verb on found objects* — the reverse of vendor recipe lists.

### 1.d.ii DCSS — the "no crafting" lesson [12][13]

Premise check: no general crafting system ever shipped in DCSS (0.6–0.7 changelogs contain no crafting addition or removal [13]). The real lesson is stronger: an explicit, maintained **developer won't-do list** [12]:

- "**Cooking, crafting** … a passable design space *only* if abstracted away from the inventory … regulated with heavy piety costs (and thus only renewed by clearing new floors), preventing the bulk of grinding such spaces inherently suggest." Always-available crafting for every character is rejected because it "would heavily flatten runs" (adaptation to drops is the skill being tested) [12].
- "Enchant weapon / armour and brand weapon scrolls **cover about the maximum Crawl is interested in approaching this space**" — single-use, inventory-light transmutes only [12].
- Package-deal removals: **hunger** (food juggling "didn't make up for the inherent busywork") [12]; **ranged ammo**, Fulsome Distillation/Evaporate potion-bombs, god Pakellas (consumable-doubling god removed as "narrow and polarizing") [12]; **selling loot** rejected ("tedious drudgery") [12].
- Translated constraint set: crafting that needs *ingredient inventory + backtracking + repetition* is out-of-bounds; crafting that is *bounded, one-step, drop-adjacent* (enchant scrolls) is fine.

### 1.d.iii ADOM — smithing as gated scarcity [14]

- Skill is race/class-locked (Dwarves, Weaponsmiths, Farmers start with it) or bought from Glod for 5,000 gp — price hiked by his prejudices (orcs, trolls, female PCs) [14].
- Chain of prerequisites: a **forge** (rare dungeon feature; guaranteed forges gated behind Dwarftown's 2,500-gp-per-use smith, hostile Darkforge, or Ice Queen prison) + an **anvil** (rare, heavy) + a hammer + **identified ingots of the item's exact material** (Metallurgy skill to ID; Weaponsmiths melt 200 stones of same-material items → ingots at level 6) [14].
- Smithing costs large clock *and game time + satiation* — a strategic "retreat and forge" excursion, not a verb you spam [14].
- Lesson for a procedural world: make the **craft site itself the loot**. The forge-hunt creates a quest arc (see also D2's The Smith/Malus guarding Charsi's imbue [d2-bosses §2]) without any skill-grind system.

## 2. Consumables economy

### 2.1 D2 catalog — what actually exists (numbers per [8])

| Category | Contents | Mechanic notes |
|---|---|---|
| Healing ×5 | 30 / 60 / 100 / 180 / 320 HP per bottle (Nec/Sorc/Dru) — **Barb gets 2×** (150-class: Ama/Pal/Asn; 200-class: Barb) [8] | heal **over time**, stacks total not rate, wasteable; small chance of double heal per draught [8] |
| Mana ×5 | 20–250 by tier, class-scaled; regenerates faster per second than health [8] | same HoT rules |
| Rejuvenation | 35% / 100% (Full) of life+mana, **instant** [8] | the panic slot (see d2-bosses §3.3) |
| Counter-potions | **Antidote** (cure poison; +50 PR & +10 *max* PR, 30 s), **Thawing** (cure chill; +50 CR & +10 max CR, 30 s), **Stamina** (instant stamina + "super Stamina recovery", 30 s), durations stack on pre-gulping [8] | the only true **pre-fight buff consumables** in D2; fight-role coverage in d2-bosses §3.3 — new point here: *three bottles is the entire pre-buff loadout D2 needs for 5 acts* |
| Throwing potions | Strangling/Choking/Rancid Gas (poison clouds) + Fulminating/Exploding/Oil (fire), stack 25/slot, equip as weapon [8] | early-grenade economy that dies out by Hell |
| **Utility scrolls** | **Scroll of Identify** (identify 1 item), **Scroll of Town Portal** (blue portal to current town); **Tome of Identify / Tome of Town Portal** consolidate 20 scrolls into one slot [18] | the "slot tax" design: carrying a TP tome = insurance, carrying ID tome vs Cain = inventory tension. Vendors restock them; shift-right-click auto-fills the belt [8] |

New (not in d2-bosses): the cube *recycling loop for consumables* — 3 small rejuvs → full [7]; Strangling Gas + healing potion → Antidote [7]; 6 PGems + magic amulet → Prismatic (all-res) — consumables are themselves cube reagents, closing the loop from §1.c [7].

### 2.2 Utility consumables: Identify / Town Portal as a pattern [18]

| Property | Identify | Town Portal |
|---|---|---|
| Question answered | "Is this worth a slot?" | "Do I get out / come back?" |
| Consumes | scroll OR a trip to town (Cain) | scroll = a square of safety you preemptively buy |
| Inventory pressure | tome (20 uses) vs 1-slot singles | same tome pattern |
| Turn-based mapping | ID = *information* consumable; ours could be an Oracle-service or a rare "Lens" item | TP = *retreat* consumable; in our sim, a "Waystone" with a Laya-visible `return_risk`? — Guardrail: one type each max |

### 2.3 Food/buff: Grim Dawn — rations: none [5][19]

Grim Dawn has **no food or hunger system**. Field consumables: auto-keyed potions (heal/energy), one-off elixirs (XP, respec-adjacent: Elixir of the Aether/Ancients/Hunt/Void are smith-crafted from recipes [6]), Dynamite crafted by legion smiths [3], and late-game **Tonics of Clarity/Reshaping** (merchant/smith-sold, dungeon-wide %XP% and reset) [3][19]. Cauldrons exist as world potion-brew sites [19]. Buff *duration* pressure is carried by components/auras instead of food. Lesson: an ARPG campaign does fine with **zero food items** if counter-potions + one XP/utility tonic tier cover the space.

### 2.4 Food/buff: Pathfinder Kingmaker/WotR — rations as camp clock [17]

- Kingmaker: resting in the wild costs **Camping Supplies and Rations** (heavy, purchasable); a companion fills the Cook role; successful cooking (skill check) yields a **meal buff lasting to next rest** (custom recipe-ingredient matching gives stronger buffs); failed hunts extend travel time [17].
- WotR: rations largely dropped; rest menu keeps **cooked-meal buffs** via Knowledge (World) skill checks [17].
- Lesson: rations are a *rest-permission currency*, i.e. a time-pressure dial — they only earn their slot when resting is the scarce action. In Laya (city inns refill mana, short runs), ration-pressure duplicates the death/reset clock we already have.

### 2.5 Which consumable categories a 10–12-level turn-based game needs

| Category | D2/GD/PF evidence | Laya verdict |
|---|---|---|
| Heal-over-time tiers | D2 5 tiers, class-scaled [8] | **Need 2 tiers max**; per-round tick maps to turn tension (drink = forgo attack) |
| Instant full-heal panic | Rejuv/Full Rejuv [8] | **Need 1**; rare drop/expensive, mirrors d2-bosses §3.3 panic-button role |
| Counter-potion pre-buffs (anti-poison / anti-chill) | Antidote/Thawing +50 res 30 s [8]; boss-checks: Andariel poison, Duriel/Izual chill [d2-bosses] | **Need exactly these 2** — we have poison (Matriarch) and chill threats; 30 s → N-round duration |
| Escape fuel (stamina) | Stamina potion [8] | **Skip** — our retreats are positional, no stamina stat |
| Throwables | D2 gas/oil potions die out by Hell [8] | **Skip** (or 1 boss-tuned firepot); danger is a 5th inventory column nobody reads |
| Utility scrolls (ID / TP) | Scroll+tome pattern; slot tax [18] | **Keep 1 each max**: Oracle/lens = identify; waystone-scroll = retreat |
| Food/rations | GD: none, works [5][9 vs 17]; PF: ration = rest-permission currency [17] | **No food system**; if a buff-slot is wanted, 1 "camp canteen" tonic per dungeon from an innkeeper — see §4 |
| XP/utility tonics | GD Elixirs/Tonics [3][19] | **Optional 1** (e.g. skill-point elixir) only if it feeds the NPC economy, never a repeatable grind |

## 3. Gathering in procedural worlds

| Game | Resource placement | Respawn vs one-shot | Notes |
|---|---|---|---|
| **Terraria** [15] | Veins at worldgen by layer/depth; **one variant per ore tier per world** chosen deterministically from the seed (`(s×f + c) mod … > 2^30`); hardmode ores **retro-spawned into an existing world** when the player smashes altars (count-formula halves yield per altar cycle) | **One-shot** blocks (mined = gone); scarcity relieved by huge worlds + altar-regen + boss/fishing/extractinator alternates [15] | Seed-picks-the-variant = replay differentiator; altar formula shows how to budget a *finite* resource across a campaign |
| **Minecraft** [16] | Ore distribution per chunk by Y-range (triangular distributions) | **One-shot**; scarcity solved by effectively unbounded world (default world border 60 M × 60 M blocks) [16] | Infinite map ⇒ no budgeting needed; opposite of our fixed 200×160 |
| **WoW nodes** [2] | Herb/mining nodes as interactable world objects, tracked on minimap per gathering profession [1][2] | **Timer respawn** — nodes despawn on loot and re-spawn; multi-player race for nodes is the scarcity mechanic [2] | MMO-shared-world answer; inapplicable (single player, no racing) |
| **Roguelike scrounging** | Qud: **corpse = node** — Butchery auto-yields one part per valid corpse, jerky-preservation converts meat 1 → 3–10 units [11]; plants harvested as items in generated biomes [11][9] | **One-shot per corpse/biome cell**; biomes re-roll per zone | The natural fit: loot-table dressing, not a click-loop on the map |
| **DCSS** [12] | None — and deliberate: harvesting/ingredients are in the won't-do list as grind-and-inventory bloat [12] | n/a | Cut entirely; the game pays nothing for it |

Interaction with our world: a seeded 200×160 overworld + 13 fixed maps is **Terraria-shaped, not Minecraft-shaped** — every harvestable tile is budget space. Respawning nodes would need 4 Hz-tick bookkeeping + a rule for "when does the bush regrow when you farm at the city gate" (the exact MMO-grind-loop failure). Terraria's answers that transfer: (i) per-seed variant selection for flavor, (ii) *event-triggered* retro-spawn (altar-smashing) ≈ our sigil milestones could "wake" new growth in already-visited forest, (iii) alternates-to-mining (crates/bosses) ≈ keep ingredient drops in the loot table instead of the terrain.

## 4. Fit analysis — Laya Realms

Existing (GAME_DESIGN): smiths Orrin/Rook/Sera/Vey already **forge two matching pieces + 25/70/150 gold → next tier** (duplicate-loot sink, no recipe tree, deliberate). Vendors have schedules, personalities, Laya-inspected `haggle_accept` / `cheat_player` questions. Oracles = service NPCs (rune tuition with visible `study_request` judgment). Pillars: inspectable decisions; one run, session-scoped; death resets to last city.

| Candidate shape | Fits sim? | Grind risk | Inspectability cost | Verdict |
|---|---|---|---|---|
| **WoW-style player professions** | No persistent toon, no market, no raid consumable treadmill to feed [1] | **Fatal** — skill bars exist to burn hours [1] | A hidden 1–300 skill hidden under a pillar that demands visible probabilities — contradiction | **No** |
| **Grim Dawn NPC services** [3] | Perfect: crafting is already a *person* here; smith choice/personality bonus is a Laya surface (`forge_care`, `takes_luck`) | Low: fixed repertoires + quest gates, no per-craft XP [3] | Result previewable in the trade modal (or shown as "smith's luck" roll — visible roll preserves pillar 1) | **Yes — extend** |
| **D2 cube-recipes only** [7] | Works as one shrine/artifact object; recipes = discoverable knowledge, zero skills | Low if bounded (~6–8 recipes) and junk-adjacent [7] | Show the transmutation in the log with odds for reroll recipes; recipe *discovery* is the fun (Qud Carbide Chef memory model [9]) | **Yes — small recipe set** |
| **ADOM gated scarcity** [14] | The anvil/forge hunt = quest beat, great for 13 fixed maps | None (it's a puzzle, not a loop) | Fully deterministic | **Yes — one special craft site** (e.g. Underkeep forge, two uses per run) |
| **Player gathering (mining/herbalism nodes)** | Fits nothing; skills that watch you click bushes | High in fixed 32k-tile world [15] | Respawn rules must be inspectable = more inspector surface for no decision | **No as a system; see below** |
| **Crude scrounging (one-shot pickups)** | Terrain dressing: berry bushes, ore glints, corpse-takes | One-shot = no loop [15] | Trivial (loot roll, logged) | **Yes-lite**: one-shot seeded pickups that feed the *counter-potion and forge ingredient* economy, exactly Qud's corpse-node model [11] |

**Recommended crafting shape (one line):** **NPC-service professions only** — keep smith tier-forging, add 1–2 more named service NPCs (an alchemist selling/crafting the 2 counter-potions + 1 mana tonic; a quest-gated forge beat à la Charsi imbue / ADOM anvil hunt [14]) — **plus a bounded, discoverable transmute set** (a shrine relic, ~6–8 cube-style recipes [7], memorized when discovered like Qud recipes [9]); **no player professions, no skill bars, no mining/herbalism nodes** — gathering limited to one-shot seeded pickups that are dressed loot, per the DCSS won't-do constraint that always-available ingredient economies breed grind and flatten runs [12].

Guardrails enforced by the shape:
- *No MMO grind loops*: every service is gold- or item-priced with fixed repertoire (GD model [3]), every transmute bounded by recipe count and drop-fed inputs (D2 model [7]); nothing has an XP bar.
- *Everything inspectable*: all craft odds either deterministic (shown before commit) or surfaced as an explicit roll in the trade dialog / Laya inspector; recipe knowledge visible in a journal once learned (anti-D2-external-wiki, pro-Qud-recipe-memory [9][7]).
- *Session-scoped*: per-run discovery table is fine — discovery ≠ persistence; no cross-run unlocks needed.

### Sources
- [1] https://warcraft.wiki.gg/wiki/Profession (Profession — types, skill-up odds per color, 2-primary limit, companions)
- [2] https://warcraft.wiki.gg/wiki/Resource_node (node types per profession)
- [3] https://grimdawn.fandom.com/wiki/Blacksmith (Tale of Two Blacksmiths; Duncan/Angrim bonuses; random-affix crafts; dynamite/tonics/legendary services)
- [4] https://grimdawn.fandom.com/wiki/Crafting
- [5] https://grimdawn.fandom.com/wiki/Consumables
- [6] https://grimdawn.fandom.com/wiki/Recipes (smith-crafted elixir recipes)
- [7] https://classic.battle.net/diablo2exp/items/cube.shtml (Arreat Summit — all transmute formulae incl. gem/rune upgrades, ilvl rules, cow level)
- [8] https://classic.battle.net/diablo2exp/items/potions.shtml (potion tiers, class-scaled healing, +50 res 30 s counter-potions, throwing potions, belt/hireling UI)
- [9] https://wiki.cavesofqud.com/wiki/Cooking (campfire/oven meals, ingredient skills, Carbide Chef memory, tasty roll)
- [10] https://wiki.cavesofqud.com/wiki/Psychometry (touch-identify + learn recipe formulas)
- [11] https://wiki.cavesofqud.com/wiki/Butchery (one part per corpse, auto-butcher, preservation yields)
- [12] https://github.com/crawl/crawl/wiki/Won't-Do-(2024) (DCSS dev won't-do: cooking/crafting scope, hunger removal, Pakellas, selling items)
- [13] https://crawl.develz.org/main/0.7.0.txt (0.7 changelog control — no crafting system shipped/removed)
- [14] https://ancardia.fandom.com/wiki/Smithing (ADOM forge/anvil/ingot chain, Glod's tuition, melting power)
- [15] https://terraria.wiki.gg/wiki/Ores (seed ore-variant formula, altar hardmode retro-spawn & halving, one-shot veins)
- [16] https://minecraft.wiki/w/Ore + https://minecraft.wiki/w/World_border (per-chunk ore gen; default 60 M × 60 M border)
- [17] https://pathfinderkingmaker.fandom.com/wiki/Camping + https://pathfinderkingmaker.fandom.com/wiki/Camping_supplies_and_rations
- [18] https://diablo.fandom.com/wiki/Scroll_of_Town_Portal, https://diablo.fandom.com/wiki/Scroll_of_Identify (tome = 20 scrolls; town/merchant restock)
- [19] https://www.grimdawn.com/guide/gameplay/service-npcs/ (official service-NPC guide incl. cauldrons)
