# B5 Combat

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.4, §1.8, §3.10 (cargo), §4.2, §8, §10; reference/minecraft-tech-inventory.md C.3, C.5, C.9. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** B1, B2, B3, F2, F5. **Used by:** B4 (mob drops), C6 (boss fights), C7 (squad tactics), C8, C9 (escort).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Weapon mechanics | Measured | Cooldowns, crits, sweeps, mace and spear behaviour are game rules, confirmed in the harness |
| 4.2 | PvE per mob type | Deterministic | Each mob's behaviour is fixed code; counter-play is known community knowledge |
| 4.3 | Boss fights (blaze, wither, dragon) | Deterministic | Known patterns with known solutions, timed by measurement |
| 4.4 | Ranged combat | Deterministic | Projectile ballistics are game physics |
| 4.5 | Shields, totems and retreat | Deterministic | Thresholds are policy values; the mechanics are fixed |
| 4.6 | Target selection and cargo exclusion | Deterministic | A filter and a priority function |
| 4.7 | Rules of engagement and attribution | Interface · runtime decision · either | Whether a player's hit was hostile, accidental or an ally's mistake is intent, not mechanics |
| 4.8 | PvP against real players | Interface · authored input · person | Humans adapt; there's no optimal policy to derive, only doctrine iterated from fights |
| 4.9 | Combat reflexes | Deterministic | Registered per the reflex contract, each switchable |

## Architecture decisions

### ⚑ AD-B5.1 Hand-written combat doctrine, or learned policies?

- **Options:**
  - (a) Hand-written doctrine files run deterministically (B5 §4.8, C7 §4.5).
  - (b) Trained policies (reinforcement or imitation learning on recorded fights) for PvP.
  - (c) Hand-written doctrine, with a seam where a learned policy can replace one technique later.
- **Recommendation:** (c). Doctrine is auditable and testable against the harness's adversaries. Learned policies need a training pipeline, many fights and their own evaluation, which is a project of its own.
- **Why it needs Allen:** It decides whether the project builds a machine-learning pipeline: a big investment and a different kind of work.
- **Settled by:** Allen's call.
- **If decided late:** The seam in (c) keeps the option open.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-b51-hand-written-combat-doctrine-or-learned-policies).


---

## 1. Purpose

B5 is one bot fighting: against every mob type, against the bosses on the way to beating the game, and against players when policy allows. It also defines the combat reflexes (shield, dodge, defend self) that keep bots alive in a fight and that cause the most trouble near cargo and allies. Group tactics are C7's.

## 2. Scope

**Owns:** weapon mechanics; per-mob PvE behaviour; boss fight skills for a single bot's part; ranged combat; pearls; shield, totem and retreat behaviour; target selection with cargo exclusion; rules of engagement as applied per bot; PvP technique; combat reflexes.

**Does not own:** formation and focus fire (C7, K6); deciding to fight a player (policy and approval, K2); threat assessment (C8).

## 3. Interfaces

Skills:
- `fight(target_selector, rules)` until the target is dead, fled or the rules say stop.
- `kill(entity_uuid)`, `defend(position | bot, radius)`, `escort_guard(entity_or_bot)`.
- `shoot(target, weapon)`, `pearl_to(position)`.
- `disengage(to: position)`.

Rules passed in: allowed targets, never-targets (cargo, allies), retreat thresholds, PvP allowed or not, explosive weapons allowed or not.

## 4. Requirements

### 4.1 Weapon mechanics

Attack cooldown per weapon, critical hits (falling, not sprinting), sweep attacks, axes disabling shields, the mace's fall-damage smash and Wind Burst, spears (1.21.11) and lunge, bows and crossbows, tridents. All values from the mechanics table (F5), confirmed in the harness.

### 4.2 PvE per mob type

Per mob, a behaviour module: creepers (keep distance, strike and back off, shield the blast), skeletons and strays (close distance using cover or shield), endermen (don't look, fight under a 2-high roof or in water), spiders, witches, drowned, piglins (gold armour), hoglins, guardians, phantoms, breezes (deflected wind charges; never fight breezes tagged as cargo), and the rest. Unknown mob types fall back to generic melee with a retreat threshold. *Proposed (AD-K5.2): this fallback becomes the main path. A mob's behaviour is a threat-profile row in F5 (attack kind, damage types, counters, habits) read by one parameterised `engage`, with per-mob code kept only as optional tuning (proposals/GENERALIZATION.md §3.2, §3.3).*

### 4.3 Boss fights (blaze, wither, dragon)

- Blaze: fire resistance, cover from fireballs, snowballs or melee.
- Wither: summoning location policy, phases, ranged and melee split.
- Ender dragon: crystal destruction (with caged crystals), perch timing, beds are a known strategy but explosive and need policy. C6 owns the swarm-level plan; B5 owns each bot's part. *Proposed (AD-K5.2): these are profile rows and compositions of the generic verbs, not separate modules, except where a fight has phases that no profile field can express (the dragon).*

### 4.4 Ranged combat

Ballistic aim with gravity and drag from game data, leading moving targets, crossbow loading, arrow supply from the keep-list.

### 4.5 Shields, totems and retreat

- Shield up against incoming melee and projectiles when the reflex is on.
- Totem in the offhand when health drops below a threshold (or always, per role).
- Retreat when health or gear falls below policy thresholds, to a rally point (an annotation or the squad's), not just away.

### 4.6 Target selection and cargo exclusion

- Targets come from the job's selector and rules, ranked by threat and distance.
- **Cargo exclusion:** entities tagged as cargo in C9's ledger are never targets for skills or reflexes, even when they attack. The bot shields and backs off instead, and keeps out of the cargo's line of sight when it can (ARCHITECTURE §3.10). This is a filter, not a reflex (CONVENTIONS §4.5).
- Allies (from C8's relations) are never targets; friendly fire is avoided in aiming.
- **Cargo-proximity weapon rule.** Exclusion stops targeting, not collateral damage. Within a set radius of tagged cargo: no sweeping attacks (axe hits or sprint hits only, which don't sweep), no bow, crossbow or trident shots with cargo in or near the line of fire, no explosives or wind charges, and no mace smash. Bots don't dodge or retreat into cargo or its vehicles. Like exclusion, this is a filter, not a reflex (spec audit #7, 2026-10-13).

### 4.7 Rules of engagement and attribution

> **Interface input (runtime decision).** Who: either (an LLM officer or the commander for routine cases, a person for anything that would start a war).
> Why: a player hitting a bot might be attacking, testing, griefing, or an ally's stray arrow; whether to treat it as hostile is a question of intent and politics that no rule settles in every case.
> Enters the system as: a merged escalation ticket from C8 with the sighting, damage and history, answered by setting the player's relation (C8) or a policy; the default answer is in policy.
> Null's side: by default (ARCHITECTURE §4.2) bots defend themselves only, against the attacker only, only while attacked, and disengage when it stops; the incident goes to C8 for attribution; nothing escalates to attacking a player without approval.

### 4.8 PvP against real players

> **Interface input (authored).** Who: person (an LLM can analyse replays and suggest changes).
> Why: human opponents adapt, combine techniques, and change between fights; a fixed algorithm can be beaten once learned, and there's no optimal policy to derive. Good PvP has to be written as doctrine and revised from real fights.
> Enters the system as: a versioned doctrine file per technique (spacing, crit timing patterns, shield and axe use, pearl escapes, crystal and anchor use where the server allows it), selected by the squad's plays (C7), each tested against the harness's adversary models (F3 §4.4).
> Null's side: executes doctrine deterministically within vanilla limits (reach, click rate, rotation), records every engagement for review, and falls back to retreat when doctrine has no answer.

### 4.9 Combat reflexes

| id | trigger | action | default | turn it off when | if off |
|---|---|---|---|---|---|
| `combat.defend_self` | bot takes damage from a non-cargo, non-ally entity | fight back the attacker only, while attacked | on | loading or escorting cargo; in formation; on errands; in opsec situations where fighting reveals the swarm | `ReflexSuppressed`; squad or skill decides (often: retreat) |
| `combat.shield` | incoming melee or projectile | raise shield | on | timed click sequences; builder replay | bot takes the hit |
| `combat.dodge` | creeper fuse nearby, arrow in flight towards bot | step out of the blast or path | on | narrow ledges or redstone areas where a step does damage | bot holds position |
| `combat.crit` | in a fight skill, attack ready | time the attack for a critical hit | on | when jumping is unsafe (low ceilings, near edges) | normal attacks |
| `combat.totem` | health below threshold | move a totem to the offhand | on | when the offhand holds something a job needs (a shield in a shield-only drill) | no totem swap |
| `combat.flee_low_hp` | health below policy threshold | disengage to the rally point | on | last-stand defence ordered by the commander | bot fights on |

All switchable at every scope (CONVENTIONS §4.2). Squads often switch off `combat.defend_self` in formation so bots don't break ranks, and cargo jobs switch it off with `reflexes_off`.

## 5. Failure handling

| failure | response |
|---|---|
| Losing a fight | Retreat at threshold; report to the squad |
| Out of arrows or durability | Switch weapon; retreat if none |
| Killed | B2 death handling; the death goes into the squad's after-action record |

## 6. Scale and performance

Per-bot combat is per-tick on the node. Group coordination at 4–10 Hz is K6/C7. Click rates and rotations stay within what a vanilla client produces.

## 7. Version-sensitive parts

Weapons (mace 1.21, spear 1.21.11), cooldowns, mob behaviours, totems, crystal and anchor mechanics, via F5.

## 8. Tests

- One bot against each hostile mob type in the harness: survival rate measured.
- Breeze tagged as cargo attacks a bot: the bot never hits it, shields and backs off.
- `combat.defend_self` off at job scope: an attacked bot emits `ReflexSuppressed` and does not fight back.
- PvP doctrine versions against each adversary model (F3 §4.4), scored and replayable.
- A harness "ally" shoots a bot by accident: no retaliation beyond policy; an attribution ticket goes to C8.

## 9. Open questions

- Which servers allow crystal and anchor PvP, and what their anti-cheat accepts for click rates, needs a per-server profile (F1 §4.9).
