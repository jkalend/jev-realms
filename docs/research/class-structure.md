# Class structure & attribute systems — research

Purpose: adjudicate the owner's "fewer bases + specializations (e.g. warden as a base)" float against how comparable games structure base-class → specialization choice, and whether an attribute/stat system earns its place at Laya Realms scale (cap 12 per D26, 4 flat stats ATK/DEF/SPD + HP/stamina/mana pools — model.rs:401-425). Design targets read: `docs/D2_EVOLUTION.md` §4 (six classes), §5 (3 branches × 4 nodes, 12 points), §6.5 (six acts, class quests with trial fights). All sources accessed **2026-09-23**.

## 1. Base-class + specialization models

### 1.1 The six reference structures

**Grim Dawn — dual mastery** ([wiki](https://grimdawn.fandom.com/wiki/Masteries)): mastery 1 at **level 2**, mastery 2 at **level 10**, both permanent picks (skill-point refunds exist, but the mastery pair itself is not re-choosable — [UNVERIFIED], established game knowledge; the wiki page documents the picks, not the refund rule). Base game ships **6 masteries** (Soldier, Demolitionist, Occultist, Nightblade, Arcanist, Shaman); Ashes of Malmouth adds Necromancer + Inquisitor, Forgotten Gods adds Oathkeeper (→9); a tenth (Berserker) is listed for the unreleased Fangs of Asterkarn expansion. *(Correction to the brief's "8 base": base=6, current expansions=9.)* **Skill points are split** between both masters' bars: 3/level up to 50, 2/level to 90, 1/level to 100 + quest points ([wiki](https://grimdawn.fandom.com/wiki/Masteries)). Each mastery bar maxes at 50 points in 9 tiers that gate the skill rows — the bar *is* the attribute ladder (each bar point grants stats). Every pairing gets a unique class name (Soldier+Oathkeeper = Warlord, etc. — 45 named combos). Point sink per mastery is deep enough that dual-classing means *thinning two trees*, not full+full.

**BG3 / D&D 5e — subclass at level 3** (Fighter exemplar, [bg3.wiki](https://bg3.wiki/wiki/Fighter)): Fighter picks its subclass at **class level 3** (Battle Master, Champion, Eldritch Knight, Arcane Archer), with subclass features landing at 3/7/10 of the 12-level cap — i.e., the commitment lands at 25% of max level, after the player has actually felt the base class. Caster-adjacent classes pick earlier (domain/oath/patron at 1–2). **Swap cost ≈ zero**: Withers offers full class+subclass+ability respec for 100 gold at camp, reinstatable any time except the finale ([bg3.wiki](https://bg3.wiki/wiki/Withers#Services)).

**Path of Exile — Ascendancy via trial** ([poewiki](https://www.poewiki.net/wiki/Ascendancy_class)): 7 base classes × 3 Ascendancies = 21 (**1 of 3 per class**; Scion's Ascendant is special). Chosen only after completing **the Labyrinth** — a dedicated trial dungeon introduced ~act 3, re-run at higher difficulties for **2 Ascendancy points each, 8 total**. Each Ascendancy is a small bespoke tree (12–16 passives, notables cost 2 pts each). Swap: allowed, but requires re-running the Labyrinth to reach the Altar of Ascendancy with all points unallocated, at 5× refund-point cost per point ([poewiki](https://www.poewiki.net/wiki/Ascendancy_class)). Base-class choice itself is permanent and is really a **starting position + starting attributes** on the shared tree ([poewiki](https://www.poewiki.net/wiki/Attribute)).

**Dragon Age: Origins — 2 specialization slots** ([wiki](https://dragonage.fandom.com/wiki/Classes_and_specializations_(Origins))): 3 base classes (warrior/rogue/mage) × 4 specializations each (+2 per class in Awakening). Slots: **2 in Origins** (one point at **level 7**, one at **level 14**; Awakening adds a third at 22). Two-step model: (1) **unlock** via trainer NPC, purchasable **manual/tome**, or plot event — unlocks persist across *all* characters/saves; (2) **spend** the slot to take it. Taking a spec grants a one-time attribute bonus + a small exclusive talent chain. Choice is mid-campaign by construction (slot at 7) and lore-diegetic (Reaver via a forbidden ritual, Blood Mage via a demon bargain).

**Diablo II — single lineup, lock-in via respec scarcity** ([skill points](https://diablo.fandom.com/wiki/Skill_points), [attributes](https://diablo.fandom.com/wiki/Character_Attributes), [Akara](https://diablo.fandom.com/wiki/Akara)): 7 classes (LoD) picked once at creation, no subclass layer at all — 3 skill trees × ~10 skills each, 1 point/level + 4 quest points per difficulty (~110 by 99), max 20/skill. Identity lock-in came from scarcity: pre-1.13 respec was *impossible*; post-1.13 you get **one free reset per difficulty** from Akara (after Den of Evil), and further resets require a **Token of Absolution** — cubed from Hell-difficulty act-boss essences ([Akara](https://diablo.fandom.com/wiki/Akara); token recipe page fetch-blocked (HTTP 403), composition cited from game knowledge — **[UNVERIFIED]**). Our §5 draft already copies this exact economy (Oracle free reset + Essence token, D8).

**Fire Emblem: Three Houses — class-change lines as contrast** ([fireemblemwiki](https://fireemblemwiki.org/wiki/Class_change)): **no lock-in at all.** Four tiers keyed to unit level (beginner 5 / intermediate 10 / advanced 20 / master 30); a unit takes a **certification exam** (needs the right seal + weapon-skill levels; passable down to 30% odds), and once *any* class is certified the unit can swap among certified classes freely — level is never reset. Identity lives in the *character* (personal skills, growths, lords' event classes), not the class. Contrast point: a system whose classes are costumes, and whose tutorial is the exam menu itself.

### 1.2 Comparison table

| Game | When chosen | Swap cost | Identity payoff | Tutorial friendliness |
| --- | --- | --- | --- | --- |
| Grim Dawn dual-mastery | L2 + L10 (both mid-early) | High — mastery picks permanent, only points refundable | Combo *naming* (45 dual-class names); free-form hybridization | Hard — 6→9 options × pairing grid at L2 before the player knows anything |
| BG3/5e subclass | Class picks L1; subclass at L3 (~25% of cap) | ~None — 100g full respec any time | One verb-defining feature block (maneuvers / spells / crit) | Best-in-class — taste the base 3 levels, choose informed, regret is cheap |
| PoE Ascendancy | Creation (base) + ~act 3 trial (ascend) | Moderate — trial re-run + 5× refund cost; base permanent | 8 pts in a bespoke mini-tree; 3 authored identities per base | Worst — base choice is a tree-position bet new players can't read; trial-gated |
| DA:O specializations | Unlock anytime (trainer/tome/plot); slots at L7/L14 | Permanent per character | Lore-diegetic titles + small talent chain; 2 slots stack | Good — unlock is itself a quest/tutorial; choice deferred to L7 |
| D2 single lineup | Creation, irreversible | Scarce — 1 free/difficulty + craftable token (1.13+) | Tree-shape IS identity (30 skills, 20-max caps, synergies) | Brutal in 2000, softened by Akara+Token; no subclass layer to manage |
| FE Three Houses | Levels 5/10/20/30, exam-gated | Zero — free swap among certified classes | None per class; identity = character, class = costume | Menu teaches itself; failure mode is "wrong seal", not "wrong life" |

Pattern across the six: **every well-regarded system defers or cheapens the second choice.** The commitment lands after the player has 3–7 levels of feel for the base (BG3 L3, DA:O L7, GD L10, PoE ~act 3), swap cost is inversely correlated with how much noise preceded the choice, and the *naming* payoff (Warlord, Battle Master, Assassin, Berserker) does most of the identity work per unit of mechanism. Only PoE and Grim Dawn defer-and-permanently-lock the second pick — both for 100-level games with hundreds of points, not us.

## 2. Attribute/stat systems

| System | Stats | Income | Role of stats | Verdict at scale |
| --- | --- | --- | --- | --- |
| **D2** ([wiki](https://diablo.fandom.com/wiki/Character_Attributes)) | str/dex/vit/ene | 5 pts/level (+5 via Lam Esen's Tome quest) | str+dex primarily **gate gear** ("invested just to reach equipment requirements"); vit = HP/stamina; ene = mana | Community-solved: "vit dump" — the wiki itself says str/dex are spent to requirements and vit takes the rest. 4 stats, 1 real decision. |
| **Grim Dawn** ([wiki](https://grimdawn.fandom.com/wiki/Attributes)) | physique/cunning/spirit | 1 attribute pt/level (each = +8 raw) + quest pts | physique gates armor/weapons + health/DA; cunning = physical dmg/OA; spirit = magical dmg/energy — stats gate gear AND steer damage type | Works because battle points also come from the same economy; still a "physique-to-wear, damage-stat rest" routine for most builds. |
| **PoE** ([attribute](https://www.poewiki.net/wiki/Attribute), [passives](https://www.poewiki.net/wiki/Passive_skill)) | str/dex/int | **None allocated directly** — attributes come from the shared tree (+10 on travel nodes), starting 60 split by class position | Dual duty: small inherent bonuses (life/melee per 10 str, etc.) AND gear/gem attribute requirements | Attribute system and skill system are the *same screen*: "roads" (+10 attribute nodes) vs "suburbs" (effect clusters). The hybrid eliminates a UI, not adds one. |
| **BG3/5e** ([fighter](https://bg3.wiki/wiki/Fighter), [respec](https://bg3.wiki/wiki/Withers)) | str/dex/con/int/wis/cha | Fixed at creation + feats (~4 chances over 12 levels) | Modifiers feed dice math everywhere; class picks the 1–2 that matter | 6 stats with point-buy is the heaviest here — and BG3 hides the math, autofixes racial spread, and charges 100g to undo mistakes. |

**Judgment for Laya Realms (cap 12, ATK/DEF/SPD + HP/stamina/mana):** an allocatable attribute screen would **drown the UI, not deepen it.**
- Scale math: D2 spends 5 pts/level × 98 levels ≈ 495 allocations to express one real choice (vit). At 11 level-ups (1→12) a 5-pt/level screen hands out ~55 clicks in a terminal/menu UI for a choice meta-configured games collapse to one.
- Redundancy: we already have **three** growth channels — flat level growth (+6HP/+1ST/+1ATK@even/+1DEF@3/6/9 per §4.5), class starting bias (+4HP/−2mana etc.), and the 12-point talent economy. An attribute screen would be a fourth channel spending the same strategic atoms (durability vs damage vs tempo) the first three already partition.
- Gear gating is the one attribute *function* worth stealing: D2/PoE both use stats to gate gear, but at 12 levels and one equipment tier ladder (worn→masterwork) the same pressure is already carried by **gold + forge cadence** — like-for-like, no new stat needed.
- What the talent tree (§5) does — 1 pt/level, 3 branches, capstones at 2 pts — is precisely PoE's tree-as-attribute move at our scale: growth lives in the tree, stats stay flat and legible. What drowns: a second point currency beside talent points competing for the same 12-key menu.

**Recommendation: no free attribute allocation.** Keep stats flat/readable (they feed `max(1, atk−def)` combat math tuned in §4.5); if identity-side stat *flavor* is wanted later, borrow BG3's pattern — one feat-like choice mid-campaign, not a per-level screen.

## 3. Fit analysis for Laya Realms

Current design (§4–5, §6.5): 6 lore classes, each = signature counter + HUD meter + class quest/trial; 12-node trees (3 branches × 4, tiers L1/3/6/9, capstones 2 pts); point income 9 levels + 3 sigil bosses = **12 points ≈ two full branches (5+5) + 2 spare**; six acts with class-quest trials in acts II–V; cap 10→12 per D26.

### Option A — keep 6 flat classes
Each class is base *and* identity; the 3-branch tree already delivers the mid-game specialization beat: the first full 5-point branch (affordable by L6–7 incl. 1–2 boss points) is the BG3-L3 / DA:O-L7 specialization moment, *already gated exactly where the research says commitment should land*. Zero new structure; class quests (§6.5) keep the diegetic teaching.
- *Talent economy:* unchanged. "Specialization" = your primary branch; spare 2 pts = support dabble. Matches D2's one-maxed-pair + partial-second meta by construction.

### Option B — fold to 3 bases × 2 specializations
Natural cut along the existing mechanic/lore seams:

| Base (working name) | Spec 1 | Spec 2 | Shared base kit |
| --- | --- | --- | --- |
| **Ward** (the garrison) | Keepwarden — Bulwark oaths | Gravebound — Last Vigil | martial/HP bias, stamina loop |
| **Hunt** (the road) | Redwake — Momentum | Waysworn — Open Road | speed/tempo loop, bounty economy |
| **Speaker** (the word) | Sigil-Sworn — Channel | Fensworn — Bond | caster/companion loop |

- *Costs:* two signature counters share one HUD meter per base (§4.3 needs re-spec); the 6 exclusive signatures degrade to "spec features" (violates §4.2's one-class-one-signature exclusivity rule); 6 class quests fold to 3 wasted or 6 same-base look-alikes; §4.5's per-class balance model reopens (B1 dominates like Keepwarden did pre-A1).
- *Talent economy:* each base needs 6 branches (spec-paired) or 3 generic branches — either 2× tree authoring, or the spec = branch-pick (in which case B is just A with a creation-screen rename).
- *Verdict:* Buys novelty by destroying authored identity; the 6-class model was already chosen over this (D27).

### Option C — hybrid: base at creation, specialization unlocks at mid-campaign trial
PoE's ascendancy/via-trial beat applied to six acts. The §6.5 class-quest **trial fights are literally the labyrinth**: acts II–V, custom arenas teaching the signature. Two sub-forms:
- **C-thick:** specialization adds a **4th branch per class**, unlocked at the trial. With 12 points total (2 capstones + 3 + 3 = 10 for two branches) a third spendable branch *dilutes* the named-build promise ("one named identity + support" §5) into "two halves + a third of something" — and doubles tree authoring (6→24 branches). It also conflicts with D8-first drafts. **Not viable in this economy without +points.**
- **C-thin:** specialization = **branch choice 1, surfaced diegetically.** At tier-gate L3 (act I→II, matching BG3's 25%-of-cap timing) the trial grants a boss-point-style unlock of your chosen branch's tier-3/6 access or a chartered title ("Gatekeeper", "Wayfarer") — no new points, no new branches, the UI names what the economy already contains.
- *Talent economy (rule the owner asked for):* **specialization must be choosing a branch, not a fourth branch.** 12 points = 5+5+2 is tuned to two-branch builds; a 16-node (4-terminal) surface only fits if income rises to ~15 points (e.g. +1/act-boss), which reopens §5's scale argument. Choosing-a-branch costs nothing and keeps capstones as the identity anchor.

### Interactions matrix vs §5 economy

| Option | Specialization = branch pick? | Fits 12 = 5+5+2? | New authoring load | Trial beat (§6.5) used? |
| --- | --- | --- | --- | --- |
| A: 6 flat | De facto yes (primary branch) | Yes, as designed | None | Class quests already teach signature |
| B: 3×2 | Spec = tree half, branches generic | Strained (6 branches/base or rename) | High — HUD, quests, rebalance | Would need rewriting |
| C-thick: +4th branch | No | **No** — dilutes identity math | 6 extra branches + point rebalance | Strong |
| C-thin: name the branch | Yes, explicitly | Yes | One UI/string pass | Strong |

## Recommendation

**A — keep 6 flat classes, with C-thin as garnish if the owner wants the mid-campaign identity beat.** The game already contains the BG3/DA:O specialization moment *for free*: the first 5-point branch lands at L5–7 (≈25–50% of the 12-cap campaign), gated by the same tier ladder the references use; respec at Oracles (D8) matches the "cheap second choice" rule. Of the six systems studied, the four with good tutorial outcomes (BG3, DA:O, D2-post-1.13, FE:3H) all win by *deferring-or-cheapening one meaningful pick* — never by adding a second structural layer. The 12-point economy cannot afford a fourth branch (dilution math above), and folding to 3 bases spends the six signatures' authored distinctness to buy a rename. If "fewer bases" is about creation-screen overwhelm, the fix present in §6.5 already: class quests do the tutorializing after the pick, which is how DA:O and PoE both survive their grids.

---
*Sources (all accessed 2026-09-23):*
- Grim Dawn masteries: https://grimdawn.fandom.com/wiki/Masteries
- Grim Dawn attributes: https://grimdawn.fandom.com/wiki/Attributes
- BG3 Fighter/subclass: https://bg3.wiki/wiki/Fighter
- BG3 respec (Withers): https://bg3.wiki/wiki/Withers#Services
- PoE Ascendancy: https://www.poewiki.net/wiki/Ascendancy_class
- PoE attributes: https://www.poewiki.net/wiki/Attribute
- PoE passive tree: https://www.poewiki.net/wiki/Passive_skill
- DA:O specializations: https://dragonage.fandom.com/wiki/Classes_and_specializations_(Origins)
- D2 skill points/reset: https://diablo.fandom.com/wiki/Skill_points
- D2 attributes: https://diablo.fandom.com/wiki/Character_Attributes
- D2 Akara respec + Token requirement: https://diablo.fandom.com/wiki/Akara
- Fire Emblem class change (Three Houses): https://fireemblemwiki.org/wiki/Class_change
- **[UNVERIFIED]** D2 Token of Absolution recipe (4 hell-boss essences cubed): https://diablo.fandom.com/wiki/Token_of_Absolution — page fetch blocked (HTTP 403/404 via reader); mechanism cross-confirmed against the Akara citation, recipe detail from established game knowledge.
