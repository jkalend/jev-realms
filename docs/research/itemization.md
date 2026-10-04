# Gear Depth Systems — Enchanting, Upgrade-Ladders, Sockets & Runewords

Scope: itemization-expansion candidates for Laya Realms (current gear = 4 discrete tiers worn/standard/fine/masterwork + forge 2×+gold → next tier + 3 named boss relics + one equipped relic slot; GAME_DESIGN.md §4/§11). Sources: Arreat Summit (official, classic.battle.net — still live), Blizzard news/patch posts, community wikis (poewiki.net official community wiki, Fandom game wikis), Wowhead guides. All pages accessed 2026-09-23. Table-dense notes, not prose.

## 1. Enchanting / affix systems

| System | Mechanic | Cost/loop | Failure / gambling profile | Power-ceiling control |
|---|---|---|---|---|
| **D2 magic items** | Blue items roll 1–2 affixes: prefix and/or suffix (25% both / 25% prefix-only / 50% suffix-only) from per-item-type affix tables [1][2] | None — pure loot drop; can shop white bases at vendors and wait for blue equivalents; Imbue quest reward (1/difficulty) creates a rare from a white base [1] | Full RNG: affix pick × roll range; two identical items "not impossible" but rare [1] | **ilvl-tiering**: affixes only roll if affix `qlvl ≤ ilvl + 2`, ilvl = monster level; gambling window is clvl −3/+6; top affixes unreachable until late content [1] |
| **D2 rare items** | Yellow items roll 2–6 properties (max 3 prefix + 3 suffix; one pick per affix group; random name) [1] | Loot only; cube re-roll recipe: 6 Perfect Skulls + Rare → new low-quality rare of same type [3] | Higher-dimensional RNG than magic (count, picks, ranges, name); near-perfect rares are lottery-tier [1] | Same `qlvl ≤ ilvl+2` gate; group exclusion prevents duplicate-stat stacking [1] |
| **D3 Mystic enchanting** (Reaper of Souls, 2014; the artisan is **Myriam the Mystic** — the "Enchantress" is the follower Eirena, not the crafter) | Replace exactly **one** affix on a rare/legendary: pick the slot once, it locks forever (all other affixes become unmodifiable); primary↔primary, secondary↔secondary; legendary powers excluded [4] | Gold + crafting mats per attempt; **gold cost escalates with every attempt on that item**; keeping the original roll still charges and counts [4] | Bounded gamble: options list is previewed per slot, but roll range is random; sunk-cost trap is the failure mode, never item loss [4] | Hard design ceiling: only one slot may ever change; "improves a drop without turning any item into a completely custom perfect item" [4] |
| **D4 Occultist enchant** | Replace one affix on rare/legendary; chosen slot locks permanently; Uniques' preset affixes excluded; preview list shows possible rolls [5][6] | Gold + Veiled Crystals + Fiend Roses; **gold escalates per attempt on the item** ("No Change" also counts); launch-era formula scaled off vendor sell value × escalating multiplier (3×→30×), later rebased to item-power coefficient [6][7] | Same bounded-gamble as D3; escalating cost makes perfectionist re-rolling the gold sink [6][7] | Slot-lock + Greater Affixes (1.5× versions) can never be created by enchanting — ceiling is reserved for raw drops [8] |
| **D4 Tempering** (S4 Loot Reborn) | Find/learn a **Tempering Manual** (loot), then at the Blacksmith add 1 affix from that manual's category (random pick from 2–4 options, random range); Ancestral items may take 2 tempers from different categories [9] | Manual is reusable once learned; each attempt spends **Tempering Durability charges** (≈5 per item) — charges do not refill [9] | Semi-bounded gamble: you choose the category, RNG picks option + roll; exhaust durability on a bad result = item "bricked" as a temper canvas [9] | Durability charges cap attempts at ~5; temper pools are narrow, separated from natural affix pools [9] |
| **D4 Masterworking** (S4 Loot Reborn) | Blacksmith upgrade, **12 ranks**; each rank small % boost to *all* affixes; ranks 4/8/12 grant a **+25% boost to one random affix** [9][10] | Materials from The Pit endgame activity (Obducite et al.) + gold per rank [9][10] | Deterministic except the 3 milestone affix picks; no downgrade/destruction [9] | Finite 12-rank cap; random milestone picks dilute the ceiling (perfect = triple-hit on one affix, pure luck) [9] |
| **D4 item-power steps** | Launch system: six affix-range bands keyed to item power (breakpoints 150/340/460/625/725); crossing a breakpoint re-rolls affixes onto the higher range [11]; 2.0 simplification: non-Ancestral caps at 750, Ancestral always 800 [12] | Loot depth/spend ladder | Not a gamble itself — it *gates* the gambles above | Step bands = coarse ceiling ladder; later replaced by a 2-tier 750/800 cliff [11][12] |
| **PoE crafting bench** | Unlock crafting recipes by discovery (fixed map locations); apply one **crafted mod** at fixed mid-tier value to an open prefix/suffix slot ("of Crafting" meta-mod allows up to 3); also socket count/link/colour recipes [13] | Currency per craft (Alteration…Exalted); **deterministic**: known input → known output [13] | Zero gamble — that is the point: a floor under RNG systems [13] | Crafted values deliberately below natural rolls; one-slot limit; can't fix a bad base [13] |
| **PoE orb chain** | Orb of Transmutation (white→magic), Alteration (re-roll magic), Alchemy (white→rare), Chaos (re-roll all rare mods), Exalted (add one mod), Divine (re-roll numeric ranges), Scouring (strip to white) [14] | Orbs are consumable loot + the de-facto trade currency [14] | Full gamble ladder: Chaos spam = casino crafting; mitigation via meta-mods and bench [13][14] | No ceiling mechanic needed: economy (orb scarcity) is the brake; mod tiers still ilvl-gated [14] |

**Pattern summary:** three control archetypes — (a) *gating* (D2 ilvl, D4 power bands), (b) *budgeting* (D3/D4 slot-lock + escalating cost, D4 durability charges), (c) *scarcity* (PoE orb economy). All three decouple "item found" from "item perfect" so loot stays exciting after the drop.

## 2. Linear "+1/+2…" upgrade ladders — fit to a discrete-tier game

| System | Ladder shape | Failure / sink profile | Verdict vs our 4 tiers + forge |
|---|---|---|---|
| **D4 Masterworking, 12 ranks** [9][10] | Long uniform ladder on top of fixed base quality; milestones at 4/8/12 | Material-gated, no loss; assumes an endgame activity (The Pit) feeding hundreds of runs | **Poor fit.** Replaces tier identity with a grind ramp calibrated for a live-service endgame. If borrowed, compress to 3–4 ranks — i.e. it *is* our tier count. |
| **Grim Dawn components + augments** [15][16] | Not linear: one component per item slot (complete 3 partial drops → attach, grants stats + an item-granted skill; removable at Inventor) + one augment (faction-vendor purchase, applied consumably) | Zero failure; cost = partial drops, blueprints, faction rep grind | **Best fit.** Orthogonal *depth layer* sitting on top of discrete tiers: base game tiers unchanged, components add horizontal choice (which component), augments add a rep sink. No power-ceilings rewritten. |
| **Monster Hunter: World augmentation** [17][18] | Endgame unlock (first Warrior's/Hero's Streamstone from Tempered monsters): weapon gets 1–3 augment *choices* (attack/affinity/slot/health regen), armor gets an upgrade-limit removal + more Armor Sphere levels | Streamstones RNG-gated; augments themselves choice-based, no destruction | **Partial fit.** "Augment = small menu of perks on a finished item" works at 12-item scale; the Streamstone grind it rides on does not exist in a 12-level campaign. |
| **Knight Online upgrade-scroll model** (+1…+N) [19] | Per-item probabilistic +1 steps; each attempt a scroll | **Failure can destroy ("burn") the item**; success odds fall as +N rises; burn-fodder ritual is pure player superstition [19] | **Worst fit.** Item deletion + unbounded ceiling directly retcons the finite tier system and the forge; it is a casino replacement, not an extension. |
| **Lineage II over-enchant model** (+4…+) [20][21] | Safe to +3 (weapons; +4 on some rulesets), then "over-enchant" attempts at rising risk | Failed scroll above the safe point **crystallizes (destroys) the weapon**; blessed scrolls soften, not remove, the risk [20][21] | **Worst fit**, same reason: the run-destroying cliff *is* the game's dopamine; a 12-level authored campaign cannot amortize it. |

**Read:** linear +N ladders and discrete tiers solve the same slot. You either keep tiers (worn→masterwork) and add a *horizontal* layer (components/glyphs), or you delete tiers and go full ladder. Hybrid "+N on top of tier" (Lineage-style) both negates the tier names ("what is a masterwork sword +7?") and explodes the power curve inside a 4-tier vertical band.

## 3. Socketing & runes (D2)

### 3.1 Sockets

| Rule | Value | Source |
|---|---|---|
| Socket sources | Items drop pre-socketed (grey name text); **Larzuk** (Act V Q1 *Siege on Harrogath*, 1/difficulty) adds sockets: **max allowed** for white/grey items, **exactly 1** for magic/rare/set/unique; **Horadric Cube** recipes add a *random* socket count to normal items (white only) | [22][23][24] |
| Cube socket recipes | Tal+Thul+P.Topaz+body → socketed armor (rnd count); Ral+Amn+P.Amethyst+weapon; Ral+Thul+P.Sapphire+helm; Tal+Amn+P.Ruby+shield | [24] |
| Socket caps | Per base item type and ilvl bracket (e.g. 6-socket weapons are high-ilvl only) | [23] |
| Socket fillers | Gems (5 quality tiers), Jewels (magic/rare affix pools), **Runes** (33, individually useful: El…Zod, fixed static bonus per rune per slot type) | [25][26] |

### 3.2 Rune word rules (exact)

From the official rules page [22] — every clause load-bearing:

1. **Non-magical socketed items only.** Magic, Rare, Set, and Unique items can *never* become rune words, even with correct sockets/type/runes. (Arreat Summit's own example: a Mechanic's 2-socket armor cannot make Stealth.)
2. **Exact socket count.** A 3-rune word needs exactly 3 sockets; 4 sockets fails.
3. **Correct item type** (Body Armor / Maces / Staves etc. have careful inclusions-exclusions — Hammers are not Maces; Wands/Orbs are not Staves).
4. **Order matters.** Tal+Eth ≠ Eth+Tal. Wrong spelling = you keep the individual rune bonuses but get no word; Blizzard does not restore consumed runes.
5. Runes are **consumed** on insertion; you keep each rune's own bonus *plus* the word's bonuses (so the words stack on top of the parts).
6. Superior socketed bases work and their superior stats carry through (better base, better word).
7. The base is re-usable conceptually: find a better socketed base later, make the same word again at a higher base quality — words are *personal*, uniques are *fixed*.
8. Patch-gating: some words existed only on later patches/ladder (1.10, 1.11 lists) [27][28].

### 3.3 Iconic rune words and what each one did to the meta

| Word | Formula (order) | Base | Req lvl | Headline stats | Why it defined a build |
|---|---|---|---|---|---|
| **Leaf** | Tir + Ral | 2-socket **Staves** | 19 | +3 Fire Skills; +3 Fire Bolt; +2 mana after each kill | Act 1 shop sells white 2-socket staves → every fire Sorc *walks in with her endgame-shaped weapon at lvl 19*, crafted from vendor trash + farmed runes. The archetype "leveling word": deterministic, early, build-shaping [27][22] |
| **Stealth** | Tal + Eth | 2-socket **body armor** | 17 | 25% faster cast/hit recovery/run; +15 mana; +30% poison resist | Universal leveling chest for *all seven classes*; runes farmable from the Normal Countess (her special rune drop table) in an evening. Set the power baseline every guide assumes [27][29] |
| **Spirit** | Tal + Thul + Ort + Amn | 4-socket **swords or shields** | 25 | +2 all skills; 25–35% FCR; 55% FHR; +vitality/mana | Cheap-rune +skills shield: moved "+skills" from rare-found to *makeable at will*, inflating the caster baseline permanently [27] |
| **Insight** | Ral + Tir + Tal + Sol | 4-socket **polearms/staves** | 27 | Level 12–17 **Meditation aura when equipped**; crit strike; +5 attributes | Given to the Act 2 mercenary → infinite mana for the whole party → made the Energy attribute obsolete overnight; the "one word rewrites a stat" lesson [27] |
| **Enigma** | Jah + Ith + Ber | 3-socket **body armor** | 65 | **+1 Teleport**; +2 all skills; %MF scaling with level | Handed the Sorceress's class-defining skill to everyone; the most economically dominant word in the game, and the canonical example of a word that *compressed class identity* [27] |
| **Heart of the Oak** | Ko + Vex + Pul + Thul | 4-socket **staves/maces** | 55 | +3 all skills; 40% FCR; +30–40 all resists; mana | The caster weapon standard: provenance chase (4 non-trivial runes) put a clean ceiling on "done" caster gear [27] |
| **Call to Arms** | Amn + Ral + Mal + Ist + Ohm | 5-socket **weapons** | 57 | Battle Orders warcry (+life/mana/stamina party-wide) on swap | Gave every class the Barbarian's signature buff on weapon-swap — another identity-compression word, kept on the off-hand forever [27] |
| **Grief** | Eth + Tir + Lo + Mal + Ral | 5-socket **swords/axes** | 59 | +340–400 flat damage (not shown as ED%); -20–25% enemy poison res | The melee damage ceiling; so strong it warped weapon-base choice (Phase Blade only) [27] |

### 3.4 Grim Dawn components — the "socketed gear augment" analog

Grim Dawn ships the depth without the casino [15][16]:

- **Components** drop as partials (⅓, ½…), complete into one attachable item per gear slot (weapon/armor/accessory families), granting stats **plus a granted skill** (e.g. Chilled Steel → Ice Spike) — socket-fillers that are themselves progression items.
- One component per item; **removable at the Inventor** (salvage choice: keep item or component) — sockets are *not* permanent decisions.
- Rarity tiering mirrors D2 runes: common components everywhere, rare ones craftable/faction-gated.
- **Augments** (expansions): consumable purchases from faction quartermasters → the deterministic, rep-grind counterpart to rune-hunting.

### 3.5 Why rune words are beloved (design causes, not vibes)

| Cause | Mechanism |
|---|---|
| **Deterministic chase** | The recipe is fixed and public; the bottleneck is concrete and farmable (Countess has a *special rune drop table* — there is literally a boss whose loot is "rune knowledge made flesh" [29]). A unique drop is luck; a word is a plan you execute. |
| **Lore discovery** | Runes are a real alphabet; formulas *spell* (TirRal, TalEth); the order-matters rule means finding the word is structured like learning a language — the Arreat Summit calls runes "an ancient alphabet" inscribed on items [25]. |
| **Vendor-trash alchemy** | The best early game gear is a *white* item you bought at a shop plus farmed runes; the loot game's lowest tier (grey/white socketed) stays permanently relevant because only non-magical items can hold words [22]. |
| **Personal ceiling** | Re-make the same word in a better base as you find one: the word is *yours* and portable, vs. uniques being everyone's identical drop [22]. |
| **Knowledge = power, mechanically** | Wrong order fails and *burns your runes* [22]; mastery of the system is rewarded with raw power, which is what "experienced users" — the Summit's own framing — means. |

## 4. Fit analysis for Laya Realms

### 4.1 Current state (anchor points)

- Gear: weapon/armour in 4 discrete tiers (worn/standard/fine/masterwork); forge trades 2 matching pieces + 25/70/150 gold → next tier [30]; 3 named boss relics (Rallybreaker, Fangmantle, Graveglass) in 1 equipped relic slot; rarity colour language already reserves green for relics [31].
- Campaign L1–12 (D26), talent income 12 points, three shrine-taught spell **runes** (Spark/Mend/Iron ward — a name collision flagged below), gold sinks: forge, companion hire, rune tuition, keys [30].
- Artefacts/plan docs treat relics as the "one named idol per build" slot; anything else must live *below* relic privilege.

### 4.2 Candidate paths, scored against this anchor

| Path | What it adds | Collision surface | Fit |
|---|---|---|---|
| **Affix RNG (D2 magic/rare style)** | Re-rolls of ±ATK/DEF/utility on drops; enchanting one slot | Campaign has ~2–3 gear upgrades per slot *total*; affix variance needs loot volume to be interesting (D2 model assumes thousands of drops + ilvl ladder [1]). Our 12 levels give maybe 30 weapon drops. | ✕ — a slot machine with 30 pulls never pays out. |
| **+N linear upgrade (Lineage/KO; D4 MW)** | Grind ramp past masterwork | Directly retcons the 4-tier identity + forge economy (§2 read); failure/destroy models contradict authored-campaign item permanence. | ✕✕ — replaces the game, diegetically and mathematically. |
| **Sockets + glyphs ("seal-words")** | A horizontal depth layer: grey socketed bases + collectible glyph items + a handful of fixed two/three-glyph words | Contained if sockets/words are restricted to weapon+armour and relics/quest items are ruled socketless (D2's own non-magical-only clause [22] transfers almost verbatim). | ✓ — the only candidate that *adds a loop without moving the existing power budget*. |

### 4.3 Recommendation — **one path: sockets + words of the seal** (D2 runewords at Laya scale, Grim-Dawn-flavoured)

Concrete shape, sized for L1–12:

| Element | Proposal | Provenance |
|---|---|---|
| Socketable objects | **Glyphs** (do *not* call them runes — "rune" already means the 3 shrine spells in GAME_DESIGN §11; call formulas **words** to lean on the seal-script/Sigil-Sworn lore) — ~8 glyphs, flat static bonuses (e.g. +1 ATK on weapon, +1 DEF on armour, stamina/mana utility) | D2 rune static bonuses [26]; naming resolves genuine internal collision |
| Socketed bases | Weapon + armour may drop/buy as grey "socketed" (1–2 slots); the existing four smiths can add 1 socket for gold ≈ a Larzuk service (Larzuk: [23]) | D2 Larzuk/cube services [23][24] |
| Words | 4–6 two-glyph words + 1–2 three-glyph capstone words; exact-order, exact-socket-count, non-named bases only (D2 rules 1–4 verbatim [22]); words taught as discoverable lore (Oracle dialogue, Underkeep inscriptions — Laya-question hooks, pillar 1) | D2 iconic-word roles [27]: a *Leaf*-style early word (cheap, class-shaping ~L5), a *Stealth*-style universal word, an *Insight*-style utility word (e.g. companion-focused), one expensive capstone word |
| Power budget | Word bonuses ≈ **half a forge tier total** (+1–2 ATK / +1–2 DEF equivalents); tier stays the dominant axis; glyph drops begin in the Crimson Hollow (post-Chief, ~L5–7) so the early game is untouched | Guardrail derived from D2 lesson: bounded, recipe-known power at authored scale |

### 4.4 Interaction risks (called out)

| Risk | Mechanism | Mitigation |
|---|---|---|
| **Relic collision** | Relics are the "set-green named idol" system; a word producing relic-class power (esp. anything resembling Graveglass's rune-cost discount or Fangmantle's adjacency guard) devalues the 3-drop relic chase and the relic slot's one-equipped tension [30][31] | Rule mirror of D2's: words on **non-named, socketed weapon/armour only**; relics, sigils, quest items permanently socketless-by-rule [22]. No word may duplicate a relic effect; words give stats, relics give rules. |
| **Forge double-dip** | If words survive the 2×+gold tier upgrade, players socket once and ride it to masterwork, collapsing the tier axis against §4.3's "half a forge tier" budget | Words bind to the *item*, and the forge produces a *new* item of the next tier → word lost; sockets re-decide per tier. Keeps grey-base shopping meaningful at every tier (D2's "re-make the word in a better base" loop [22]). |
| **12-level power creep** | Even +4 ATK equivalents ≈ one full forge step on top of masterwork ≈ ±20–30% swing at cap (forge-cadence combat model, D2_EVOLUTION §4) | Cap word stack at +2 ATK / +2 DEF equivalents; glyph drops gated ≥L5; words non-stacking (one word per item obviously, one socketed item per slot structurally). |
| **Name/lore collision (runes, sigils)** | In-game "runes" = 3 shrine spells; "Sigils" = the 3 dungeon-lord macguffins | Call them **glyphs/words of the seal**; ties to the Oathless Curate's unwriting and Sigil-Sworn seal-script lore instead of colliding with it [31]. |
| **Determinism vs. Laya pillar** | Word discovery via fixed lore dumps is zero-Laya | Route word hints through Oracle/NPC dialogue questions (the existing Laya-question surface) — knowledge-as-reward is already the game's first pillar. |

## Sources

[1] https://classic.battle.net/diablo2exp/items/magic.shtml (magic/rare affix counts, `qlvl ≤ ilvl+2`, gambling window clvl −3/+6, Imbue) ·
[2] https://classic.battle.net/diablo2exp/items/magic/ (per-propert prefix/suffix tables) ·
[3] https://classic.battle.net/diablo2exp/items/cube.shtml (Horadric Cube incl. 6 Perfect Skull rare re-roll, socket recipes) ·
[4] https://diablo.fandom.com/wiki/Enchanting (D3 Mystic Myriam: one locked slot, escalating gold per attempt, primary/secondary separation; note: Fandom may require browser for robots-check) ·
[5] https://www.wowhead.com/diablo-4/guide/vendors-and-crafting/occultist-sigils-aspects-enchanting (D4 Occultist slot-lock, escalating cost, "No Change" counts) ·
[6] https://www.wowhead.com/diablo-4/guide/gear/itemization-affixes-upgrading-sockets (D4 affix/enchant overview) ·
[7] https://news.blizzard.com/en-us/article/24092662/diablo-iv-patch-notes-1-0-1-2 (enchant cost rebased off sell value, patch notes) ·
[8] https://www.wowhead.com/diablo-4/guide/gear/greater-affixes (Greater Affix = drop-only, 1.5×, enchanting destroys it) ·
[9] https://news.blizzard.com/en-gb/article/24077223/galvanize-your-legend-in-season-4-loot-reborn (official: Loot Reborn — tempering manuals + durability, masterworking, Pit materials, affix-count reduction) ·
[10] https://www.wowhead.com/diablo-4/news/itemization-update-summary-diablo-4-season-4-338170 (S4 itemization summary: 12 masterwork ranks, milestone boosts) ·
[11] https://www.wowhead.com/news/how-item-power-breakpoints-work-in-diablo-4-333439 (launch-era power bands 150/340/460/625/725, affix re-range at breakpoint) ·
[12] https://news.blizzard.com/en-us/article/24140803/conquer-colossal-foes-in-season-of-hatred-rising (2.0: non-Ancestral caps 750, Ancestral fixed 800) ·
[13] https://www.poewiki.net/wiki/Crafting_Bench (deterministic one-mod crafts, recipe-unlock by location, socket recipes) ·
[14] https://www.poewiki.net/wiki/Currency (orb ladder: Transmutation/Alteration/Alchemy/Chaos/Exalted/Divine/Scouring functions) ·
[15] https://grimdawn.fandom.com/wiki/Components (partial drops → completed component, one per item, granted skills, Inventor removal) ·
[16] https://grimdawn.fandom.com/wiki/Augments (faction-vendor consumable augments) ·
[17] https://monsterhunter.fandom.com/wiki/MHWI%3A_Augments (MHW/Iceborne augment unlock via streamstones; armor upgrade-limit removal; rarity material table) ·
[18] https://mhworld.kiranico.com/en/guide/augmentation (Warrior's vs Hero's Streamstone roles, R6–8 weapon augments) ·
[19] https://www.nttgame.com/knight/en/guide/enjoy (Knight Online official guide: probabilistic +1 upgrades; burn-on-failure community-documented, e.g. https://www.taultunleashed.com/knight-online-submissions/excellent-upgrade-guide-t46795.html) ·
[20] https://en.wikibooks.org/wiki/Lineage_2/Items/Enchantments (Lineage II: safe +3 weapons, over-enchant crystallize on failure, blessed scrolls) ·
[21] https://www.l2.wiki/essence/wiki/items/item-enchanting/en (Essence variant: +4 safe point) ·
[22] https://classic.battle.net/diablo2exp/items/runewords.shtml (official rune-word rules — all rules in §3.2 quoted/summarized from here) ·
[23] https://classic.battle.net/diablo2exp/quests/act5.shtml (Siege on Harrogath → Larzuk socket reward) ·
[24] https://classic.battle.net/diablo2exp/items/socketeditems.shtml (socketed-item behavior, per-type socket caps) ·
[25] https://classic.battle.net/diablo2exp/items/runes.shtml (33 runes El→Zod, static per-slot bonuses, "ancient alphabet") ·
[26] https://classic.battle.net/diablo2exp/items/gems.shtml (gem filler analog; jewels: /items/jewels.shtml) ·
[27] https://classic.battle.net/diablo2exp/items/runewords-original.shtml (Leaf, Stealth; originals list) + https://classic.battle.net/diablo2exp/items/runewords-110.shtml (1.10: Spirit, Insight, Enigma, Heart of the Oak, Call to Arms, Grief) ·
[28] https://classic.battle.net/diablo2exp/items/runewords-111.shtml (1.11 words; patch-gating example) ·
[29] https://diablo.fandom.com/wiki/Countess_(Diablo_II) (Countess special rune drop table — the farmable-rune bottleneck) ·
[30] GAME_DESIGN.md §4/§11 (current gear tiers, forge 2×+gold, relic effects, rune spells) — repo-local ·
[31] docs/D2_EVOLUTION.md (relic colour language §7.4 item rarity; D26 level cap 12; sigil lore)

*(All external URLs accessed 2026-09-23; Arreat Summit pages re-verified live on that date.)*
