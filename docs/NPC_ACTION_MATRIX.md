# NPC action profile matrix

Status: production action contract for offline 3D → sprite bakes.

`idle`, `walk`, `attack` and `hit` are **render lanes**, not a complete animation
library. Every new actor, player and boss asset carries an `action_profile` in
its manifest. The profile identifies the archetype + weapon + role and names the
actual clips that must exist in the GLB.

## Sharing rules

- Share locomotion only when body proportions, silhouette and equipment do not
  change its read.
- Never share attack clips across different weapon families unless the game
  explicitly defines a common archetype variation.
- Weapon-specific attacks, hit reactions, guards, casts and death/recovery
  actions remain unique to the profile.
- Bosses require a `boss.signature` and `phase_actions`; a generic attack row is
  not a boss animation set.
- Direction is a render axis. It does not replace the semantic clip names.

## Roster contract

| Archetype | Weapon / role family | Profile id | Required identity clips |
|---|---|---|---|
| Commoner | Unarmed / civilian | `npc_commoner_unarmed` | idle, walk, shove, hit |
| Guard | Spear + shield | `npc_guard_spear` | guard, thrust, bash, hit |
| Bandit | Knife / skirmisher | `npc_bandit_knife` | flank, slash, dodge, hit |
| Skeleton | Rusted sword / undead | `npc_skeleton_sword` | raised guard, chop, stagger, hit |
| Chief | Greataxe / heavy boss | `boss_chief_greataxe` | axe windup, overhead slam, recover, hit |
| Matriarch | Claws / brood boss | `boss_matriarch_claws` | claw combo, brood cast, recoil, summon |
| Lich | Staff / caster boss | `boss_lich_staff` | guard idle, walk, cast windup/release/recover, hit, phase actions |
| Adjudicator | Gavel / judge boss | `boss_adjudicator_gavel` | idle, walk, raise, slam, recover, judgement |
| Oracle | Ring staff + scroll | `npc_oracle_scroll` | channel, read, release, recoil |
| Companion | Sword + buckler | `npc_companion_swordshield` | guard, slash, bash, protect, hit |
| Tidemother | Kelp / water boss | `boss_tidemother_kelp` | gown idle, sweep, pull, heal, phase summon |
| Cragmother | Stone ridge / earth boss | `boss_cragmother_ridge` | brace, slam, fissure, recover |
| GnawThane | Claws / shard swarm | `boss_gnawthane_claws` | swipe, pounce, shard cast, recoil |
| Tollmaster | Chain flail / toll boss | `boss_tollmaster_flail` | windup, chain sweep, hook, recover |
| Mirelight | Phase orb / spectral boss | `boss_mirelight_phase` | drift, orb pulse, phase shift, fade |
| PaleStag | Antlers / charging boss | `boss_palestag_antlers` | idle, charge, antler thrust, recover |

Player classes are separate action profiles. A player profile may share a
locomotion base with an NPC only when weapon, armor and silhouette do not change
the attack/read; each class still owns its weapon and reaction clips.

## Manifest shape

```json
{
  "id": "boss_lich_staff",
  "default_action": "lich_staff_idle_guard",
  "weapon": "staff",
  "clips": {
    "idle": ["lich_staff_idle_guard"],
    "walk": ["lich_staff_walk_a", "lich_staff_walk_b"],
    "attack": [
      "lich_staff_cast_windup",
      "lich_staff_cast_release",
      "lich_staff_cast_recover"
    ],
    "hit": ["lich_staff_hit_recoil"]
  },
  "boss": {
    "signature": "summon_and_curse",
    "phase_actions": [
      "lich_phase_summon",
      "lich_phase_curse",
      "lich_phase_reinforce"
    ]
  }
}
```

The render manifest still declares frame counts per lane. The profile names the
semantic clips; the GLB must contain those names. The Lich v6 boss profile, the
Guard v1 spear/shield profile and the 24-entry roster under
`tools/asset3d/examples/npc_roster_profiles.json` are wired through that check.

A clip and its rendered frame must show the same pose. Both are cut from the
same pose track, and the render pass detaches the rig action first - an active
action re-evaluates the pose bones on every depsgraph update and silently
overrides the hand-applied pose, which freezes every frame to the default idle.

The runtime binds `default_action` before the first in-game frame. A T-pose or
unbound skeleton is a missing-binding bug, not an idle animation.

## Review gate

An action set is not approved until:

1. front, side and back silhouettes remain readable;
2. idle, walk, attack and hit are visibly different in a contact sheet;
3. the weapon stays attached to the correct hand/bone during attack frames;
4. boss signature and phase actions are present;
5. every clip named by the profile exists in the exported GLB;
6. the baked fragment preserves the declared lane/frame mapping.
