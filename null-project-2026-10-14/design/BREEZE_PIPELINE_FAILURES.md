# How the breeze pipeline fails

> **Status (2026-10-14): reference.** Failure modes of the breeze pipeline. Its fixes landed in ARCHITECTURE v0.7 (§3.14) and are specified in `specs/C9-entity-logistics.md` and `specs/B7-mob-handling.md`. References to v0.6 behaviour describe the design before those fixes.

Companion to `REVIEW_OSC_CONTROL_METHOD.md`. Design only; neither architecture file was changed.

**Assumptions (from Allen, 2026-10-13).** The Orbital Skybender is a proven design: 256 breezes in a 1 × 2 cell, whose thrown wind charges deliver the payload, with TNT dupers for the payload and **dual chunk loaders** that keep the machine loaded. Its mechanics (persistence, cramming, how the breezes are made to fire) are taken as working. This document is about how **Null running the pipeline** fails: the bots, the control layers, the ledgers, the server and other players.

The pipeline, as stages:

```
 P1 capture at the trial chamber ─ P2 stage at the chamber ─ P3 convoy (overworld → portal → nether → portal → overworld)
   ─ P4 hand over at the site ─ P5 install into the cell ─ P6 hold in the cannon (chunk loaders) ─ P7 top up after losses
```

The headline: the pipeline is most likely to fail **at the seams**. That means handovers between operations, server restarts, preemption, and the moment the breezes first see a player inside the cannon. Breezes dying in the open is a smaller risk. Most mitigations are generic framework features, not cannon features.

---

## 1. Failures by stage

### P1 Capture at the trial chamber

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| The other spawners in the chamber wake up | Bots positioned for the breeze spawner come in range of zombie, skeleton or spider spawners, so the capture squad ends up fighting | Combat reports from the capture squad; capture rate below plan | Survey (O3) maps every spawner's range; capture stands are chosen outside the others, or those spawners are walled off first |
| A bot triggers an ominous trial | A bot carrying Bad Omen (from a raid captain, an ominous bottle) enters the chamber | Effect visible in the bot's own state | Policy: bots with Bad Omen never get chamber jobs; reflexes don't drink ominous bottles |
| The bots kill the cargo | Breezes attack the nearest player, and each bot's reflex layer ("defend self") fights back | Entity ledger count drops while the capture squad reports kills | **Cargo protection:** ledger entities are tagged so reflexes and combat never target them. Bots shield and retreat instead |
| Wind charges open the containment | A breeze's wind charge toggles doors, trapdoors and gates and presses buttons and levers. A breeze shooting at a bot can open its own cage | Escape is seen as a ledger entity outside its held location | Containment from the pack uses no wind-toggleable blocks on the cargo side, and bots stay out of the cargo's line of sight except while loading |
| Capture rate below the quote | Fewer bots in range than planned, wave timing, losses | Capture rate tracked against plan per wave | The quote uses a measured rate. The operation can ask HQ for more bodies in range, since bots raise the wave size |
| The chamber is contested | The chamber is near an enemy route (the quote's own risk line); enemies arrive mid-capture | Threat sighting in the operation's region | Policy decides fight or flee. Fleeing means parking the cargo safely (see §2.2), not abandoning it |

### P2 Staging at the chamber

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| Batches pile up at the most exposed place | The convoy waits for a full load while capture is slow | Count staged and time staged, against plan | Move small batches often instead of one big convoy. Every hour at the chamber is exposure |
| Staged breezes despawn or are lost | The bot keeping them alive is pulled away, disconnects, or the server restarts | Ledger shows a holder with no keeper | Staging is a held claim with a keeper, the same as the convoy. Losing the keeper is an immediate high-priority reassignment |

### P3 Convoy

This is the longest stage and has the most moving parts.

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| **The train splits across loaded and unloaded chunks** | Escort spacing lets one vehicle drift beyond simulation distance, so it freezes while the rest move | A ledger entity's last-seen position stops updating while the others move | Formation is part of the convoy job: every vehicle within range of a bot, checked every tick by the squad leader, and the train stops when the check fails |
| **The convoy splits across dimensions** | Some vehicles go through the portal and some don't. The portal breaks, a ghast hits it, or an enemy breaks it | Ledger entities in two dimensions under one claim | The moving claim must allow two dimensions. Rule: a bot crosses first, then the cargo in groups that each arrive with a bot already waiting |
| **The portal goes to the wrong place** | The nether portal links to a different portal, possibly one an enemy built near the expected exit (a portal hijack) | The first bot across reports where it came out | Both portals are claimed and guarded. A bot probes the link before any cargo crosses, and again after any gap |
| Arrival chunk unloaded | Cargo arrives with no bot already there, so it sits in a chunk loaded only briefly | Cargo seen leaving, not seen arriving | Covered by "a bot crosses first" |
| Rails and vehicles fail | Rails broken by a creeper, a ghast or an enemy; a derailment; boats stuck; vehicles pushed by mobs | Vehicle stopped or off route | Rail-laying squad ahead, a repair squad behind, and spare vehicles in the convoy's inventory |
| Cargo dies on the way | Ghast fireballs, lava falls, piglins or hoglins in the way, fall damage | Ledger count drops | Route chosen for exposure (enclosed nether tunnel if the route will be used again). Losses are re-ordered, not escalated, unless the critical path slips |
| **Escort reflexes cause losses** | A breeze shoots an escort, the escort's knockback reflex or combat reply hits the cargo or knocks a bot into lava | Same as the capture case | Cargo protection, and no line of sight between cargo and escorts where the route allows |
| The convoy leads enemies to the site | A long breeze train in the nether is very visible. Anyone who follows it finds the cannon | Threat sighting along the route, or a player seen at more than one point of the route | Opsec policy: the convoy halts at a decoy or park point if followed and only continues when clear. The final leg to the site is short and unmarked |
| Moving claim expires | The claim lease isn't renewed during a network partition, so another operation's bots path through or mine the rails | Claim expiry event | Moving claims renew from the squad leader, which is on the same node as the escort, not through HQ |

### P4 Handover at the site

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| The count doesn't match | Convoy says 64 delivered, site operation counts 61, and the ledger is now unclear about who lost three | The two operations' counts disagree | Ledger entries are by entity UUID, not by count. Handover is a joint check: both sides see the same UUIDs before ownership moves |
| Nobody owns the breezes for a moment | Convoy releases its claim before the site operation takes it | A ledger entry with no owner | Handover is one step: the new owner's claim exists before the old one is released |
| Cargo arrives before the cell is ready | The program let the convoy run ahead of the build | Cargo waiting at the site with no target | A holding cell at the site (behind the chunk loaders if they're built first) decouples convoy timing from build timing |

### P5 Installation into the cell

This is the most dangerous step, because the bot doing it is the thing breezes shoot at, and breeze wind charges are the cannon's own actuator.

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| **The cannon fires, or half-fires, during installation** | The installing bot gives the breezes a target, and their charges actuate the machine | Unplanned block changes or TNT at the site | Installation is **arming**, not building. It needs the pack's own safe-insertion procedure (a builder macro), the payload path empty or broken while it runs, and approval under policy, like any step that makes a weapon live |
| Cell disturbed by the installer | Bot pushes entities, steps on blocks that react to players, or opens the wrong gate | The builder's post-step verification differs | Same rules as any builder step: trialled in the simulator, verified after |
| Overfill or underfill | Batch counts are off, or breezes die inside | Count in the cell is not 256 | Count by UUID at each insertion; the operation tops up from the holding cell |

### P6 Holding in the cannon

The dual chunk loaders take over the job bots did before, but they add their own failure modes.

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| **Both loaders stop at once** | A shared cause: a server restart (if the loaders need a player to restart them afterwards), a shared component breaking, a change of server settings | **Null can't see it without a bot nearby.** The ledger only says "256, last seen N hours ago" | A generic **visit-after-restart** job: after every server restart, a bot goes to each marker that needs it, before other work. Plus a periodic check visit, with its interval set by the descriptor |
| Breezes lost while nobody watches | Anything that happens in a loaded chunk with no player: a mob wandering in, a block update | Only found at the next visit | Check visit before every shot, and a count check as the first step of firing |
| Someone finds it | An enemy notices the loaded chunks (lag, mobs moving, the chunk loaders' own signs) | Threat sighting, or damage found at a visit | Out of Null's hands beyond policy (defend, alarm). The operator decides whether to keep the cannon |

### P7 Top-up after losses

| failure | cause | how Null notices | mitigation |
|---|---|---|---|
| A small top-up costs as much as the first fill | Each loss means another chamber trip, a convoy, and an installation (arming) | Losses after each shot | Keep a reserve in the site's holding cell, sized from measured losses per shot, so top-ups don't need a convoy |
| The chamber is gone | Raided, claimed by an enemy, or its spawners destroyed | Survey or threat report | The quote should note how many usable chambers exist, since the cannon depends on one for its whole life |

---

## 2. Failures across stages (the control method)

### 2.1 Server restart

The single most dangerous event. Every bot disconnects at once, and every keeper, escort and holder disappears together. When the server comes back, each bot logs in where it left, but the order and timing are random.

- The order bots reconnect in should be **by what they're protecting**: convoy escorts and keepers first, then visit-after-restart jobs at chunk loaders, then everything else.
- A convoy caught by a restart should **resume from its park state**, not re-plan from scratch.
- This is also when the architecture's "jobs come back as `failed(disconnected)` and are re-planned" (§3.8) is wrong for cargo. A convoy needs to recover its exact state.

### 2.2 Preemption

HQ moves 30 bots to a war front, and some of them are escorts. Under v0.6 the preempted job stops at a safe point. For cargo, "stop" leaves breezes unattended.

- Cargo jobs need **park points** along the route (enclosed holding spots), and preempting a convoy means "move to the next park point, hand to a keeper, then release".
- The cheaper rule: the convoy's minimum escort can't be preempted below a floor; anything above that can.

### 2.3 Handover between operations

The pipeline crosses three operations (chamber, convoy, site). Every crossing is a chance to lose ownership or count. Ownership by UUID, and one-step handover (new owner first), handle most of it.

### 2.4 Several nodes see different worlds

A convoy escort may be bigger than one squad, so it may run on two swarm nodes, and each node has its own view of the world and its own entity IDs. Squads "never span nodes" (§3.8), but the convoy can. The fix is to identify cargo by UUID across nodes, and to keep the escort for one convoy on one node where possible.

### 2.5 Escalation spam

A long convoy produces a steady stream of small losses and stops. If each one is an exception, the commander's digest fills with them. Losses within the expected rate should only show as the program's ETA moving. Only "critical path slipped by more than X" or "cargo below the cell's minimum" should reach the commander.

### 2.6 The ledger outlives what it can see

After installation, nobody watches the breezes until the next visit. The ledger has to say so ("256, last seen 14 h ago") and the firing errand should treat an old count as unverified. Otherwise the operator fires a cannon that lost half its breezes overnight.

---

## 3. What this asks of the framework

All of these are generic. None is cannon-specific.

1. **Cargo entities.** Ledger entities tagged as cargo are never targets for reflexes or combat, and bots avoid their line of sight by default.
2. **UUID-based entity ledger,** consistent across nodes, with one-step handover between owners.
3. **Moving claims** that can span two dimensions and renew locally, not through HQ.
4. **Park points and a preemption floor** for cargo jobs, so preempting a convoy parks it instead of stranding it.
5. **Restart protocol:** reconnect order by priority, a `visit-after-restart` job for markers that need it, and cargo jobs resuming from saved state instead of re-planning.
6. **Arming as a policy class:** any step the descriptor marks as arming (installing the breezes) needs approval, the same as attacking a player.
7. **Staleness on the ledger:** counts carry their age, and a firing errand checks the count first.
8. **Bad Omen hygiene** and other "don't carry this into that place" rules as ordinary policies.

The cheapest test of most of this is the small breeze pipeline from the review: a few dozen breezes, one chamber, one nether convoy and a test cell. Then add harness variants for a restart mid-convoy, preempting the escort, breaking the exit portal, and a fake enemy following the train.
