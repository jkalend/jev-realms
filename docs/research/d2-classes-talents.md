# D2:LoD (patch 1.10+) — Class & Talent-Tree Systems Research

Scope: Diablo II: Lord of Destruction class design for adaptation into Laya Realms. Sources: Arreat Summit (official Blizzard classic.battle.net, still live), Diablo Wiki (diablo2.diablowiki.net via archive), Diablo Wiki (Fandom) as fallback. Synergy-era (1.10+) rules; respec data is 1.13-era as annotated. Data-dense notes, not prose.

## 1. Classes

### 1.0 Overview table

| Class | Lore identity | Primary stat investment | Resource loop | 3 skill trees (tab order) |
|---|---|---|---|---|
| **Amazon** | Fierce nomadic warrior-women of the Askari civilization, island of Skovos; masters of bow/javelin/spear [1] | Dexterity (bows/crossbows derive damage from Dex), some Vitality; Str only to gear breakpoints [2] | Mana (baseline full refill takes 120 s without regen items) + potions (misc resources: arrows/bolts must be re-bought); weapon-swap on W key [2][3] | Bow & Crossbow · Passive & Magic · Javelin & Spear [4] |
| **Necromancer** | Priest of Rathma, keeping the Balance; re-animates the dead [1] | Energy for summons mana; Vitality; minimal Str/Dex [2] | Mana + **corpse economy** (summons, Corpse Explosion, Revive all consume fresh corpses) [5] | Summoning · Poison & Bone · Curses [4] |
| **Barbarian** | Nomadic war clan guarding Mt Arreat; brute shaman-warrior [1] | Strength + Vitality; Dexterity only for gear [2] | Cheap mana costs; shouts feed the whole party [6] | Warcries · Combat Masteries · Combat Skills [4] |
| **Sorceress** | Rebellious Zann Esu clan witch; pure elemental caster [1] | Energy/Warmth mana engine; Vitality; almost no Str/Dex [2] | Mana-hungry caster; regen via Warmth passive + potions [7] | Cold · Lightning · Fire [4] |
| **Paladin** | Zakarumite holy knight defending faith; aura-warrior-cleric [1] | Strength + Vitality; Dex to 75% block breakpoint [2] | Moderate mana; auras cost no mana while active [8] | Defensive Auras · Combat Skills · Offensive Auras [4] |
| **Assassin** | Viz-Jaq'taar "mage slayer" order hunting rogue sorcerers; psychic-martial hybrid [1] | Strength + Dexterity to gear breakpts., Vitality [9] | Mana; **charge-up meter** built by hits then cashed out in one finisher [9] | Martial Arts · Shadow Disciplines · Traps [4] |
| **Druid** | Shamanic tribe of Scosglen; shuns traditional magic, bonds with nature/spirits [1] | Strength + Vitality (shifter); Energy (elemental) [10] | Mana (elemental/hurricane builds) or melee cost-per-form (shifter) [10] | Elemental · Shape Shifting · Summoning [4] |

### 1.1 Amazon — weapon-swap identity + minion tank [4][11]

| Signature mechanic | Why it plays differently |
|---|---|
| **Two mutually exclusive weapon builds** | Bowazon (Strafe/Multiple Shot tearing packs from range) vs Javazon/Spearazon (Charged Strike/Lightning Fury vs object density, Fend in melee) — the same 30-point class budget forces a weapon choice twice: via skill trees AND via item slots [4][11] |
| **Passive & Magic tree** | Dodge/Avoid/Evade soft-capped chance-to-dodge replace defense: the Amazon survives not by mitigation but by RNG mobility; Penetrate/Critical Strike convert Dex into raw offense [4][11] |
| **Valkyrie + Decoy** | One summon tank (Valkyrie, level 30) backed by a repeating aggro handoff to Decoy = mini-army of exactly ONE body, in deliberate contrast to Necromancer skeleton mass [4] |

### 1.2 Necromancer — army management + corpse economy [4][5]

| Signature mechanic | Why it plays differently |
|---|---|
| **Skeleton army** | Raise Skeleton + Skeleton Mastery stack point-for-point: Necro is the only class whose DPS is a *team*: positioning (Iron Golem, Clay Golem wall), aggro split, telestomp via revive. Corpse Explosion scales with monster corpse HP — its damage comes from what you already killed, so efficiency snowballs mid-fight [4][5] |
| **Curse tree as whole-tree utility** | Amplify Damage (-100 phys resist in Hell) ≠ damage: a single global debuff click replaces a whole skill investment; Iron Maiden; Life Tap heal; Attract/Confuse crowd control [4] |
| **Golem/eco-support** | Clay Golem slows bosses (keeper of Duriel fights); Blood/Iron/Fire golems as specialty counters [4] |

### 1.3 Barbarian — shouts as persistent buff stack + mastery lock-in [4][6]

| Signature mechanic | Why it plays differently |
|---|---|
| **Warcries = party-wide buff tree** | Battle Orders (+life/mana/stamina, party-wide incl. hirelings), Shout (+defense), Battle Command (+1 all skills): whole tree is pre-combat "chanting" with long durations — gameplay loop is *re-shout timers*, not mana-fueled DPS [4][6] |
| **Weapon Masteries force early lock-in** | Each weapon class is its own passive (Sword/Axe/Mace/Polearm/Spear/Throwing): picking Sword Mastery at 6 makes a whole loot category dead forever — the tree *commits your loot filter* [4] |
| **Whirlwind channel** | Control inversion: you steer an un-interruptible spinning hitbox through packs; leech sustains during the channel — costs mana per spin [4] |

### 1.4 Sorceress — element specialization + Teleport privilege [4][7]

| Signature mechanic | Why it plays differently |
|---|---|
| **One element per build** | Three mono-element trees; in Hell almost all monsters are immune to something (immunity = resist ≥100%), so a mono-element Sorceress literally cannot kill some packs solo — the class *is* a resistance-check puzzle [12][13] |
| **Teleport** | Req level 18; lines-of-sight-ignoring positional reset for 24→mana. Only the Sorceress gets it natively; the rest of the player base must craft it via Enigma runeword. Mobility as class identity; every additional point only reduces mana cost [7] |
| **Skill class variety in one tree** | Static Field (% current HP, floor at 33% NM / 50% Hell) skips scaling; masteries are +% passives to their element's damage [7] |

### 1.5 Paladin — right-click aura swapping [4][8]

| Signature mechanic | Why it plays differently |
|---|---|
| **Auras: persistent radius buffs you swap on right-click** | Design: left-click is the attack, right-click is the *stance*: Might/Fanaticism/Conviction (offense) vs Vigor/Redemption/Salvation (defense) — no mana cost once active, applies to party *and* hirelings within radius. One aura at a time = every click is a stance choice; auras debuff enemies (Conviction, Holy Freeze) as well [4][8] |
| **Zeal / Hammer / Smite melee variants** | Zeal = uninterruptible fast multi-hit; Blessed Hammer = magic-damage spiral projectiles (no phys immune can block them); Smite always hits + stuns (boss takedown kit) [4] |
| **Per-level aura radius growth** | Offensive auras grow radius per point invested: investing = social value (whole party) not just self-buff [8] |

### 1.6 Assassin — charge-up + finisher loop [4][9]

| Signature mechanic | Why it plays differently |
|---|---|
| **Charge-up → finisher economy** | Martial Arts charge-up skills (Tiger Strike, Cobra Strike, Phoenix Strike…) add orbs per hit (max 3); a single finisher (Dragon Talon kick, Dragon Claw, Dragon Tail) cashes in all accumulated charges — gameplay loop = build to 3 → dump. Build meter visible under character portrait [4][9] |
| **Traps as persistent deployables** | Wake of Fire/Lightning Sentry/Death Sentry are turret-like self-casting summons (max 5 traps up at once); the class shadows the Necro's army pattern with *stations* instead of *units* [4] |
| **Shadow Disciplines as identity switching** | Burst of Speed vs Fade (survivor stances), Mind Blast/Shadow Warrior clones, Weapon Block passive (dual-claw unique defense) — a secondary stance-doubling layer on top of the charge loop [4] |

### 1.7 Druid — shapeshift forms + spirit allies [4][10]

| Signature mechanic | Why it plays differently |
|---|---|
| **Shape Shifting tree** | Werewolf (fast, Feral Rage charges up lifesteal/speed per hit) vs Werebear (tanky, shock-wave stun) — replacing your skill *with a body*: you lose access to all non-form skills while shifted; run-time limb/weapon behavior replaced (weapon speed is recalculated per form) [4][10] |
| **Spirit summons (Oak Sage & co.)** | Only ONE spirit active at a time: Oak Sage (+life aura), Heart of Wolverine (+dmg), Spirit of Barbs (thorns) — Druid summons function as *auras* you pick from a menu, binding the Necro summon pattern to the Paladin aura pattern [4] |
| **Elemental caster side** | Fire (Fissure, Volcano, Armageddon — persistent ground zones) vs Wind (Hurricane — cold armor + Tornado pierce): the Druid's third mini-class, competing for the same point budget as the shift forms [4][10] |

## 2. Skill Tree Structure (exact numbers)

### 2.1 Layout / budget

| Rule | Value | Source |
|---|---|---|
| Skills per class | **30 unique skills**, split **3 tabs × 10** | [6][14] |
| Tree | Pre-req arrows drawn between skills inside a tab; items can grant a skill but NOT skip the prerequisite arrow — "receiving the skill via item does **not** allow you to advance further down the skill tree" | [14] |
| Level tiers (required character level) | **1 · 6 · 12 · 18 · 24 · 30** — e.g. Sorceress Lightning: Charged Bolt 1 → Telekinesis/Static 6 → Lightning/Nova 12 → Chain Lightning/Teleport 18 → Thunder Storm/Energy Shield 24 → Lightning Mastery 30 | [14][3] |
| Investment pacing | After first point, additional points in that skill need **+1 character level per point** past the skill's required level (you may also hoard points to buy the whole unlocked tier at once) | [14] |
| Hard point cap | **Max 20 hard points per skill** (exceedable only via items, Battle Command +1, Skill Shrine +2) — so a level-99 build at most 110 hard pts (95–110 ≈ 5 full 20s) | [14] |
| All classes start with | Attack + Throw (+ Unsummon for minion classes) | [14] |

### 2.2 Point income

| Source | Points | Frequency | Source |
|---|---|---|---|
| Per level | **+1 skill, +5 attributes** | every level (capped at 99 → 98 earnable from leveling) | [2][14] |
| Den of Evil (Act I Q1) | +1 skill (+1 free respec, 1.13+) | once per difficulty | [15][14] |
| Radament's Lair (Act II Q1) | +1 skill | once per difficulty | [15] |
| The Fallen Angel / Izual (Act IV Q1) | +2 skill | once per difficulty | [15] |
| Lam Esen's Tome (Act III Q4) | +5 attribute | once per difficulty | [15][2] |
| **Total quests across 3 difficulties** | **+12 skill points, +15 attribute points** | | computed from [15] |

### 2.3 Synergy system (1.10+)

| Rule | Value | Source |
|---|---|---|
| Mechanic | Each listed skill gives **+X% per HARD point** in a listed other skill — official example: Lightning receives **+8% Lightning Damage per level** of Charged Bolt, Nova, *and* Chain Lightning; Chain Lightning receives +4%/pt from each; Energy Shield → Telekinesis ratio; class mastery tables list the whole synergy footer on each official skill page | [3][14] |
| Hard points only | "+skill bonuses from equipment do not count towards the synergy bonuses" — +3 Fire Ball orb does **not** synergize Fire Bolt | [16][14] |
| Official rationale | "**Synergy bonuses are designed to boost the effectiveness of the higher-level skills based upon the number of points allocated to the lower-level (synergizing) skills. Players are rewarded for using skill points earlier rather than hoarding them** all for later 'cookie-cutter' distribution to high-level skills" | [14] |
| History | Synergies usable since release; made explicit/standardized in **1.10** (in-v1.10-and-later rule: hard points only); skill re-balance happened with same patch | [16] |

## 3. Attribute & Progression Numbers

### 3.1 Attributes & level flow

| Rule | Value | Source |
|---|---|---|
| Attribute points | **5 per level** (+ Lam Esen's +15 total from quests = 505 max) | [2][15] |
| Skill points | **1 per level** (+12 from quests = 110 max at level 98–99) | [2][14][15] |
| Game attributes | **Strength** (damage & gear reqs; hammer = 1.10·str/100, spear = mixed), **Dexterity** (AR, defense, block chance: (Block·(Dex-15))/(Lvl·2), capped 75%), **Vitality** (life, stamina, double-heal), **Energy** (mana; full mana-pool refill in 120 s by default) | [2] |
| Level cap | **99** | [17] |

### 3.2 XP curve shape (per-level XP table)

In-level XP required to reach each named level, official table [17]:

| Level | Total XP | Level | Total XP |
|---|---|---|---|
| 2 | 500 | 30 | 4,663,553 |
| 10 | 57,715 | 50 | 47,116,709 |
| 20 | 537,513 | 70 | 285,041,630 |
| 25 | 1,640,359 | 90 | 1,618,470,619 |
| 28 | 3,203,826 | 99 | **3,520,485,254** |

Curve properties (all from [17]):

- Roughly **×8–×10 per 10 levels**; 90→99 alone costs 1,902,014,635 = **54% of the entire character's lifetime XP**.
- Level 98→99 alone is 291,058,498 XP ≈ the jump 1→94.
- On top of the raw table, **post-70 kill-XP penalty** multiplies every kill: 95.31% at 70 → 48.44% at 80 → 5.96% at 90 → **0.59% at 98** [18].
- Party member in same named area boosts total kill XP by 35%; split is level-weighted [18].

### 3.3 Respec (scarcity is the rule)

| Mechanism | Rule | Source |
|---|---|---|
| Akara respec | **1 per difficulty** (max **3 per character**, carried across N/NM/H); added with the 2010 (1.13) client, reward of Den of Evil in addition to the +1 skill point | [14][15] |
| Token of Absolution | Unlimited repeats: combine **all 4 hell-act-boss essences** in the Horadric Cube: **Twisted Essence of Suffering** (Andariel OR Duriel) + **Charged Essence of Hatred** (Mephisto) + **Burning Essence of Terror** (Diablo) + **Festering Essence of Destruction** (Baal); each is a low drop rate in Hell only | [19][14] |

## 4. Design Analysis — why the trees create build identity

| Mechanic | Effect on build identity | Consequence / meta |
|---|---|---|
| **Exclusive investment** (110 hard pts across 30 skills, 20 max per skill) | One build can max ~5 skills + fillers; the other 25 skills are permanently dead weight for that character | Sorc "Blizzard" vs "Fire Wall" vs "Lightning" are effectively three classes; build guides (= identity) get named per skill max combination [14] |
| **Tier gating forces a leveling arc** | The level-30 capstone is unreachable at low level; players must pick a tier-1/6 skill as a vehicle for the early game, then pivot | Early game uses different gameplay than endgame; "leveling skills" are a real genre meta-concept (e.g. Fireball/Static Sorc 1–29, Blizzard at 30+); arc = narrative structure for free [3][14] |
| **Prerequisite arrows tax access** | End-game skill costs `(points in prereqs +1 per mid skill)` on top of tier gating — the "trash" skills define the depth of investment required | Prereqs serve a second duty as delays: you must commit before power spikes [14] |
| **1.10 synergies create related-pair metas** | Every tier-1 skill's 20 points re-enter the value equation: spend points on Fire Bolt (tier 1) *because* it makes Fire Ball (tier 12) +8%/pt stronger | Fixes hoarding (Blizzard's stated rationale) AND removes the "cookie-cutter max-only" meta by re-weighting low tiers; build math becomes a constrained optimization puzzle [14][16] |
| **Hard-points-only synergy rule** | Items granting +skills do **not** synergize: power budget separated from gear | Character build ≠ item build; loot chase stays orthogonal [16] |
| **20-point cap + level-pace 1-pt-per-your-level rule** | "Maxing" is a ~25-level arc per skill — you finish exactly one skill per ~25 character levels | Two fully synergized skill pairs ≈ full character level budget; meta "hybrid" builds = one pair + partial second [14] |
| **Scarce respec (3 Akara + farmable Tokens)** | Mistakes have a floor below which you cannot repair; permanent until you re-do all 3 difficulties | Early game is experimentation, endgame is commitment; respec is itself a reward economy tied to boss grind (Tokens) [14][19] |
| **Aura/stance slots (Paladin) and form-swap (Druid/Assassin charge)** | Class kit adds a *second axis of choices* orthogonal to the skill tree | Character sheet = build; right-click button = stance. Quadratic design surface per class [4][8] |

### Transferable lessons for a 10-level indie roguelike

| D2 lesson | Scale down |
|---|---|
| Tier gating at 1/6/12/18/24/30 creates a character-arc of 6 power jumps | With 10 levels, use 4 tiers (1/4/7/10) and make tier cap compulsory for "the run-defining spell" |
| 20-pt cap with 1 pt per player-level = skill maturation *is* the XP curve | Cap each skill at level 3–4; allow 1 pt/skill/run-level at most; pacing (not raw power) does the heavy lifting |
| Exclusive investment + quest points = identity within scarcity | ~14 pts across 10 levels ≈ 3–4 maxed skills; that's enough for named builds without 110-point spreadsheets |
| Synergies + hard-points-only sideline gear | In a roguelike, *run-locked* synergies ("+8%/point of X spell taken this run") work without inventory systems |
| Akara + Token = bounded regret | Allow exactly 1 respec per run as a boss reward; farmable Token pattern maps to "kill all 4 act bosses to reset once more" |
| Prerequisite chains = leveling arc scaffolding | Don't expose all 10 level-1 pickables; 100% tier-1 + tier-1×2 gating is enough for an arc |
| Early/mid/late reading of one skill (Static Field / Teleport) | Design skills for two different reading points (use-while-leveling vs use-at-cap) — D2 didn't have to design twice |

## Sources

[1] https://classic.battle.net/diablo2exp/classes/ (+ per-class lore pages amazon.shtml, necromancer.shtml, barbarian.shtml, sorceress.shtml, paladin.shtml, assassin.shtml, druid.shtml) ·
[2] https://classic.battle.net/diablo2exp/basics/characters.shtml ·
[3] https://classic.battle.net/diablo2exp/skills/sorceress-lightning.shtml ·
[4] https://classic.battle.net/diablo2exp/skills/ (class skill-tab index; per-tab pages: amazon-{bow,passive,javelin}, necromancer-{summoning,poisonandbone,curses}, barbarian-{warcries,combatmasteries,combatskills}, sorceress-{cold,lightning,fire}, paladin-{defense,combat,offense}, assassin-{martialarts,shadow,traps}, druid-{elemental,shapeshifting,summoning}.shtml) ·
[5] https://classic.battle.net/diablo2exp/skills/necromancer-summoning.shtml ·
[6] https://classic.battle.net/diablo2exp/skills/barbarian-warcries.shtml ·
[7] https://classic.battle.net/diablo2exp/skills/sorceress-lightning.shtml#teleport ·
[8] https://classic.battle.net/diablo2exp/skills/paladin-offense.shtml ·
[9] https://classic.battle.net/diablo2exp/skills/assassin-martialarts.shtml ·
[10] https://classic.battle.net/diablo2exp/skills/druid-shapeshifting.shtml ·
[11] https://classic.battle.net/diablo2exp/skills/amazon-bow.shtml ·
[12] https://classic.battle.net/diablo2exp/monsters/super.shtml ·
[13] https://diablo.fandom.com/wiki/Immune_(Diablo_II) (fallback: immunity = resistance ≥100%; Hell "almost all Monsters are immune to something") ·
[14] https://classic.battle.net/diablo2exp/skills/basics.shtml ·
[15] https://classic.battle.net/diablo2exp/quests/rewards.shtml ·
[16] https://web.archive.org/web/20201112011009/https://diablo2.diablowiki.net/Synergies (v1.10 synergy integration; hard-points-only) ·
[17] https://classic.battle.net/diablo2exp/basics/levels.shtml ·
[18] https://classic.battle.net/diablo2exp/basics/experience.shtml ·
[19] https://diablo.fandom.com/wiki/Token_of_Absolution (Token introduced in patch 1.13c; 4 essences, Hell-only act-boss drops, unlimited respecs)
