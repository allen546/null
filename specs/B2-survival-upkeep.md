# B2 Survival upkeep

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.4, §1.7, §3.14, §4.2, §5, §10. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** F1, F2, F5, B3. **Used by:** every job (bots that stay alive), C3 (death drop recovery).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Food and eating | Deterministic | Hunger, saturation and food values are game data |
| 4.2 | Health and potions | Deterministic | Regeneration and potion effects follow known rules |
| 4.3 | Sleeping | Deterministic | Bed rules and the server's sleep setting decide it |
| 4.4 | Armour and tool swapping | Deterministic | Durability thresholds and a keep-list |
| 4.5 | Repair | Deterministic | Mending and anvil costs are computable; the cheapest order is a known optimisation |
| 4.6 | Hazard escape | Deterministic | Fire, lava, drowning and suffocation each have a known escape |
| 4.7 | Death and item recovery | Deterministic | Drop position, despawn timer and a recovery job |
| 4.8 | Status-effect hygiene | Deterministic | Policies list effects that bar jobs (Bad Omen and trial chambers) |
| 4.9 | Survival reflexes | Deterministic | Registered per the reflex contract, each switchable |

## Architecture decisions

None open. Staying alive is fully specified by game rules; what to risk is policy (K2).


---

## 1. Purpose

B2 keeps each bot alive and equipped: fed, healed, armoured, with working tools, out of fire and water, and able to recover its items after a death. Most of it runs as reflexes, which is exactly why every one of them must be switchable.

## 2. Scope

**Owns:** eating; health and potion use; sleeping; armour and tool swapping; repair; escaping fire, lava, drowning and suffocation; death handling and item recovery; status-effect hygiene; survival reflexes.

**Does not own:** fighting (B5); getting food or gear in the first place (B4, C3, C4); what risks a bot may take (policy, K2).

## 3. Interfaces

- **Keep-list** per bot role: food, spare tools, armour, a water bucket, blocks for pillaring, totems. B3 keeps these in slots; B2 consumes them.
- **Events out:** `low_food_reserve`, `tool_worn(item)`, `armour_broken`, `died(cause, position, inventory_snapshot)`, `effect_gained(effect)`.
- **Jobs:** `recover_drops(position, deadline_gt)`, `repair(items)`, `sleep()`.

## 4. Requirements

### 4.1 Food and eating

- Eat when hunger falls below a threshold, choosing food by saturation and the keep-list (keep golden apples for combat). Never eat in a tick where the active skill has declared a no-eat window (for example the last ticks of a timed click).
- Report `low_food_reserve` when food in inventory drops below a role threshold, so logistics restocks.
- Azalea doesn't apply the using-item slowdown, so a bot eating while moving moves at full speed. Until the fork patches it, eat only while standing still (Azalea reality check, 2026-10-13).

### 4.2 Health and potions

- Natural regeneration needs hunger ≥ 18; B2 keeps it there when health is below max.
- Potions (healing, regeneration, fire resistance, water breathing) are used when the job or a reflex calls for them and they are in the keep-list.

### 4.3 Sleeping

- Sleep at night when the job allows and a bed is claimed. On servers where a percentage of players must sleep, bots sleeping affects everyone; the operation's policy says whether bots sleep at all.
- Never place or sleep in beds in the nether or End (they explode).

### 4.4 Armour and tool swapping

- Swap to the right tool for each break (B6 asks). Swap out a tool before its last few uses if it has enchantments worth repairing; let plain tools break.
- Replace broken armour from the keep-list immediately.

### 4.5 Repair

- Mending: hold or wear mending items while collecting XP.
- Anvils: combine and repair in the cheapest order, respecting the "too expensive" cap; this is computable.
- Report `tool_worn` for items that can't be repaired here.

### 4.6 Hazard escape

- On fire: water bucket, nearby water, or keep moving out of fire.
- In lava: move out by the shortest path; fire resistance if carried.
- Drowning: surface; place a block to get air if surfacing is blocked.
- Suffocation: move out; break the block if allowed.
- Void and big falls: handled by B1.

### 4.7 Death and item recovery

- On death, F1 reports the position and an inventory snapshot. Items despawn after 5 minutes (6,000 gt) if their chunk stays loaded; the recovery job races that timer.
- Recovery job: travel to the drop, collect, return; or hand off to C3 to dispatch a nearer bot. The death site is marked dangerous in F2 with the cause.
- Repeated deaths with the same cause in one region go to K5 as an exception (merged), not per bot.

### 4.8 Status-effect hygiene

- Effects that bar jobs are policies (ARCHITECTURE §3.14): no bot with Bad Omen or Trial Omen takes trial chamber jobs; B2 never drinks an ominous bottle unless a job explicitly requires it.
- B2 reports effects gained so the scheduler can apply these policies.

### 4.9 Survival reflexes

| id | trigger | action | default | turn it off when | if off |
|---|---|---|---|---|---|
| `upkeep.eat` | hunger ≤ threshold | eat best food from the keep-list | on | timed actions where a lost tick breaks the step (builder replay, errand triggers) | `ReflexSuppressed`; the skill or squad schedules eating |
| `upkeep.extinguish` | bot on fire | water bucket or move to water | on | near redstone or cargo cells where water would do damage | bot burns; event emitted |
| `upkeep.flee_lava` | bot in lava | move out by the shortest path | on | almost never; available for replay, where the replay's own halt handles it | bot stays; the job's own safety logic must act |
| `upkeep.surface_air` | air below threshold | swim up or place a block for air | on | underwater builds following a certified plan | bot takes drowning damage |
| `upkeep.armour_swap` | armour piece broken | equip a spare | on | when slot changes would disturb a timed sequence | bot fights on without the piece |
| `upkeep.tool_swap` | tool near breaking | swap to a spare | on | when slot changes would disturb a timed sequence | the tool may break |

All are switchable at every scope (CONVENTIONS §4.2). Turning survival reflexes off swarm-wide goes through echo-before-effect and shows in the digest (CONVENTIONS §4.3). Long jobs that assume eating declare `reflexes_assumed_on: [upkeep.eat]`.

## 5. Failure handling

| failure | response |
|---|---|
| No food in inventory | `blocked(no_food)` after the reserve warning went unanswered |
| Can't reach drops before despawn | Report items lost to C3's ledger |
| Repeated deaths, same cause | Merged exception to K5 |

## 6. Scale and performance

Reflex triggers are cheap per-tick checks. Death recovery is dispatched by the operation so the nearest free bot goes, not necessarily the dead one.

## 7. Version-sensitive parts

Food values, effect behaviour, item despawn timing, bed rules per dimension, and the 1.21.11 `bed_rule` environment attribute, via F5.

## 8. Tests

- Bot in a cave runs out of food: reserve warning, then `blocked(no_food)`.
- Bot falls into lava with fire resistance in inventory: escapes.
- Death in a creeper ambush: recovery job collects drops before despawn.
- `upkeep.eat` off at job scope during a replay: no eating during the job; eating resumes after.
- Bad Omen bot is never assigned a chamber job.

## 9. Open questions

- Does the server's idle timeout or sleep setting interact with bots sleeping in large numbers? Measure on each server profile.
