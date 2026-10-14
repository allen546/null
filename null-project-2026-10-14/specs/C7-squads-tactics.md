# C7 Squads and tactics

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.4, §3.4, §6, §8, §10. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** B5, B1, K6 (runs the tactics loop), F2, C8 (relations). **Used by:** K5 (war fronts, escorts, defence), C9 (convoy escort), C6 (boss fights).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Formations | Deterministic | Geometry with spacing and roles |
| 4.2 | Target allocation against mobs | Deterministic | Assignment by threat, distance and damage, a standard problem |
| 4.3 | Escort | Deterministic | Keep a formation around a moving object, with checks per tick |
| 4.4 | Retreat | Deterministic | Thresholds from policy, to rally points |
| 4.5 | Plays against players | Interface · authored input · person | Raid and defence against adapting humans has no optimal play; it's doctrine |
| 4.6 | Play selection during a fight | Deterministic | Pick among authored plays by fixed conditions at 4–10 Hz |
| 4.7 | Squad-level reflex settings | Deterministic | Squads switch reflexes per play under the reflex contract |
| 4.8 | After-action review | Interface · runtime decision · either | Learning why a fight against humans was lost is open-ended analysis |

## Architecture decisions

### ⚑ AD-C7.1 What coordinates fights bigger than one squad?

- **Options:**
  - The gap: squads are 5–20 bots running at 4–10 Hz on one node, and the next level up, the operation, decides every 1–2 s. A battle with 50 or more of our bots needs coordination bigger than a squad and faster than an operation.
  - (a) A battle group leader: several squads placed on one node with a leader running at squad speed, formed only for big engagements.
  - (b) Bigger squads in war (up to about 50).
  - (c) Accept the operation's 1–2 s for coordination between squads.
- **Recommendation:** (b) first, with (c) for macro between squads. A node now holds hundreds of bots, so a squad of about 50 fits on one, and each extra level adds message types and failure modes. Add (a) only if measured fights lose to slow reaction between squads. (Changed 2026-10-13 after Allen's capacity numbers.) Density, not headcount, caps a war squad: bot-to-bot traffic grows with the square of how many bots are near each other (spec audit #2), so big squads fight spread out.
- **Why it needs Allen:** It adds a level to the hierarchy for war only, and how big fights get depends on the servers and factions you'll face.
- **Settled by:** Allen's call, once the scale of faction fights is clearer.
- **If decided late:** Node placement (§3.8) has to co-locate war squads, which is easier to plan in from the start.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-c71-what-coordinates-fights-bigger-than-one-squad).


---

## 1. Purpose

C7 is group combat: formations, target choice, focus fire, escort, raid and defence plays, and retreat, run by the squad leader's fast loop (4–10 Hz). It's what lets 5–20 bots fight as one unit without the LLM in the loop (ARCHITECTURE §1.4: tactics never wait on the LLM).

## 2. Scope

**Owns:** formation definitions; target allocation; escort behaviour; retreat; the play library and its selection rules; squad-level reflex settings per play; after-action records.

**Does not own:** individual fighting (B5); deciding to fight (policy, K2; the commander); the loop's runtime (K6).

## 3. Interfaces

- `Play{name, preconditions, roles, formation, targeting, reflex_settings, exit_conditions}`, loaded from versioned doctrine files.
- K6 calls `select_play(situation)` each tactics tick and executes the chosen play's role assignments through B5/B1 skills.
- Out: engagement records (positions, damage, deaths, plays used) to F4 and the after-action queue.

## 4. Requirements

### 4.1 Formations

Line, wedge, ring (escort), column (travel), hold (choke point). Spacing keeps bots out of each other's sweep and arrow paths, and out of cargo's: escorts in a ring stand where their sweeps, shots and knockback can't reach the cargo, and fight under B5 §4.6's cargo-proximity weapon rule (spec audit #7, 2026-10-13). Roles: melee, ranged, shield, medic (potions), runner.

### 4.2 Target allocation against mobs

Assign bots to targets to maximise damage on the most dangerous first, limit overkill, and keep ranged bots covered. A standard assignment heuristic each tick.

### 4.3 Escort

Ring formation around a moving object (a convoy's vehicles, a builder, a VIP bot), keeping every protected entity within range of an escort, checked every tick; the column stops when the check fails (ARCHITECTURE §3.10). Escorts never target cargo, and engage threats outward from the ring so stray hits fall away from it (B5 §4.6).

### 4.4 Retreat

When squad strength (health, gear, numbers) drops below policy thresholds, or the play's exit condition is met, retreat together to a rally point (an annotation with the `rally` effect, or the squad's last safe position), covering each other.

### 4.5 Plays against players

> **Interface input (authored).** Who: person (an LLM can analyse engagement records and draft play changes).
> Why: raids and defence against human factions are adversarial and adaptive; what works depends on the opponents, the terrain and the server's mechanics, and any fixed play can be learned and countered. There's no optimal policy to derive, only doctrine written and revised by someone who understands PvP.
> Enters the system as: versioned play files (`Play` in §3) in a doctrine library, each tested against the harness's adversary models (F3 §4.4) before it's allowed on a live profile.
> Null's side: plays run deterministically; a squad with no applicable play holds or retreats by default; engaging players at all still needs approval or a standing policy (ARCHITECTURE §4.2).

### 4.6 Play selection during a fight

Each tactics tick, K6 checks the current play's exit conditions and, if needed, picks the first play whose preconditions match (enemy count, gear estimate, terrain, our strength). Selection is a fixed rule, so it's fast and auditable. No LLM is ever consulted inside a fight.

### 4.7 Squad-level reflex settings

Each play can set reflexes at squad scope for its duration (CONVENTIONS §4.2): a formation play switches off `combat.defend_self` and `move.repath` so bots hold position; an escort play switches off `combat.defend_self` near cargo; a skirmish play leaves everything on. Leaving the play restores the inherited settings.

### 4.8 After-action review

> **Interface input (runtime decision).** Who: either (an LLM reviews records and proposes doctrine or policy changes; a person approves changes to doctrine).
> Why: when a squad loses to humans, the reason (a tactic we haven't seen, bad terrain, wrong play choice, a gear gap) has to be worked out from the record, and the right change isn't derivable.
> Enters the system as: proposed changes to play files (reviewed like code) or to policies and gear targets.
> Null's side: records every engagement in full (positions, inputs, damage, play transitions) and files one review item per lost engagement, merged by opponent.

## 5. Failure handling

| failure | response |
|---|---|
| Squad loses cohesion (bots scattered) | Regroup at the rally point |
| Leader process lagging | Bots fall back to holding defensively (F1 local fallback) |
| No play matches | Hold or retreat by default |

## 6. Scale and performance

The tactics loop runs at 4–10 Hz per squad inside the node (K6). War fronts run on VPS nodes (ARCHITECTURE §3.8).

## 7. Version-sensitive parts

Weapon mechanics that plays depend on (mace, spear, wind charges), via F5.

## 8. Tests

- Squad of 10 against waves of mobs: losses and kill time measured.
- Escort ring around a boat convoy: formation check stops the column when one boat drifts out of range.
- Each play against its adversary models before live sign-off.

## 9. Open questions

- How many plays are needed before the first faction server? Start with defend-a-choke, escort and retreat, and add raids later.
