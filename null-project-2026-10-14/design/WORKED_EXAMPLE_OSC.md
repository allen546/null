# Worked example: the Orbital Skybender

Companion to `ARCHITECTURE.md` (written against v0.7; still current for v0.8, whose section numbers are unchanged). Design only.

**Revision 2 (2026-10-13).** Rewritten after the independent review (`REVIEW_OSC_CONTROL_METHOD.md`, rev 2) and `BREEZE_PIPELINE_FAILURES.md`. The first version modelled the wrong machine: 12 breezes feeding a wind charge item farm, and crafted TNT loaded per shot. The real design, the **Orbital Skybender**, holds **256 live breezes in a 1 × 2 cell** whose wind charges deliver the payload, gets its TNT from **dupers**, and keeps itself loaded with **two built-in chunk loaders**. Its mechanics (how the breezes stay persistent, avoid cramming and are made to fire) are taken as proven, per Allen. Operations are now cut by place, and the firing test uses an errand.

This follows one Skybender from the operator's request to a fired test shot. It is a stress test of the generic framework, not a feature:

- **Null** knows items, entities, facilities, sites, presence, jobs, claims, operations and the map.
- **The Skybender and its farms are content:** schematics plus descriptors in the typed schema (C11).
- **After the build,** the operator marks the cannon on the map and operates it. Null only lends bots for generic errands.

**Numbers marked `claimed` are placeholders** until measured in the harness (stage S0, §5.3) or live. The quote shows which numbers are which.

---

## 1. Inputs the operator provides

### 1.1 Content pack: `skybender-pack`

Descriptors sketched as YAML. The real schema is component C11's job.

```yaml
facility: orbital_skybender
schematic: skybender.litematic
footprint: buried; dimensions from the schematic
site:
  within_range_of: $target_region
  max_range: from the pack (claimed)          # useless beyond this
  concealment:                                 # the pack's own budget, not a Null default
    note: "256 breezes sit in one chunk; rely on depth and distance, not entity counts"
    max_bot_visits_per_day: 2                  # traffic counts too
    min_distance_from_known_routes: 200
  spawn_proofing: required for the machine's whole life   # bots are players; dark cavities spawn mobs
inputs:
  - item: slime_block        count: from schematic   use: consumed
  - item: sticky_piston      count: from schematic   use: consumed
  - item: observer           count: from schematic   use: consumed
  - item: tnt                count: seed only (claimed)  use: installed   # dupers make the payload
  - container_contents:                       # exact contents per container, not item totals
      - at: schematic pos (…)  item: music_disc_13  count: 1
      - …
  - entity: breeze           count: 256      use: installed   min_working: (claimed)
facilities_inside:
  - tnt_duper ×N             # part of the build; a rate only matters if the pack says so
  - chunk_loader ×2          # keeps the machine loaded without a bot
presence:
  - at: anchor  band: loaded  count: 1  phase: during_build
  - at: anchor  band: within_32  count: 1  phase: during_arming
  - at: anchor  band: loaded  count: 1  phase: visit_after_restart   # check both loaders came back
  - at: $target_region  band: loaded  count: 1  phase: during_strike  outside: keep_out
arming:                                       # policy class: needs approval, done last
  - circuit_break: at schematic pos (…)       # left open during build and install
  - step: install breezes (pack's safe-insertion procedure, a builder macro)
  - step: place seed TNT
  - step: close circuit_break
operation:                                    # read by the operator's tooling only
  fire: button marker "fire"
  charge_time: "> 30 min game time" (claimed)
precursors: [disc_farm]                       # breezes come from trial chambers, not a farm
```

```yaml
facility: disc_farm
schematic: disc_farm_v1.litematic
site: { spawn_conditions: creeper spawning, dark, space 30x30 }
inputs:
  - entity: skeleton  count: 1  use: installed  method: lure_by_aggro
output: { item: music_disc_*, rate: 40/hour (claimed) }
presence:
  - at: afk_spot  band: 24_to_128  count: 1  phase: while_running
```

### 1.2 The command

```
quote(Build(pack=skybender-pack, facility=orbital_skybender, bind: target_region = region "Front A"))
```

"Front A" is an operator annotation (a region). Null only uses it to bind `$target_region`.

---

## 2. The quote

The program service expands the descriptors into a full production graph, checks it against stock, known sites, known trial chambers, headcount and the server's simulation distance, and returns:

```
QUOTE  Build orbital_skybender near "Front A"          [m] measured  [c] claimed
  Operations (by place): Site, Chamber A, Chamber B, Convoy, Disc farm, + supply quotas
  Critical path: breeze capture → convoy → arming
  Breezes:   256 installed (+ reserve 24 for losses [c])
             capture ~10/h per spawner [c: 5 per wave, 30 min cooldown]
             4 spawners in 2 chambers → ~7 h capture, convoy ~260 blocks via nether
             transport: 2 per boat; batches of 32
  Lead time: ~40 h [c]          Peak bots: 190          Bot-hours: ~5,800 [c]
  Items to make: slime blocks → ~8,400 slimeballs (slime chunks unknown: survey needed)
                 observers → ~250 quartz (nether trip)
                 sticky pistons, iron, …; ~14 diamond pickaxes of tool wear (excavation)
  Spoil: ~92k blocks, ~1,450 stacks to haul ≥200 blocks from the site
  Presence: during arming 1 bot within 32; after restarts a visit; during a strike
            a spotter loading the target outside its keep-out
  Feasibility: spotter ring exists at this server's simulation distance (8) ✓
  Risks: Chamber B near an enemy route (seen 2× this week); capture rate unmeasured
  Needs approval: arming (at the time); any player-made blocks met outside a permit
                  (natural excavation needs none, ARCHITECTURE §4.2)
```

About 300 tokens whatever the swarm size. The `[c]` marks tell the commander which numbers are guesses. Here, the capture rate drives the lead time and is unmeasured, which argues for running harness stage S0 before approving.

---

## 3. The program

Operations are cut by place. Inside each, the steps are just the order its HTN runs jobs in.

```
 approve ─┬─ Chamber A ─┐
          ├─ Chamber B ─┼─ Convoy (moving claim) ─┐
          │             │                         │
          ├─ Site: survey → dig → stock → build ──┼─ gate ─ Site: arm (install 256, seed TNT, close break) ─ done
          ├─ Disc farm ───────────────────────────┤
          └─ Supply quotas (slime survey + farm, quartz, iron, tools) ┘
```

### Site (one operation, one region)

- **Survey:** candidates within range of Front A, scored on depth, distance from routes and enemy sightings, and the pack's concealment budget, including how much bot traffic the site will need. Scouts verify the best ones in person, then the site is claimed.
- **Dig:** about 92k blocks. Spoil goes into the ledger, and much of it becomes casing. The rest is hauled at least 200 blocks away so it doesn't mark the site. Lava and aquifers are handled by the site-prep skills. **Spawn-proofing goes in as the cavity is dug,** because the bots digging it are players and make mobs spawn in the dark.
- **Stock:** the bill of materials, including exact per-container contents, is staged near the site. Carriers stay outside the concealment radius until the last leg.
- **Build:** the builder works step by step with real items. The redstone core uses certified mode, and one builder squad works in it at a time while others do the casing. The **circuit break** named in the descriptor stays open, and no TNT is placed.
- **Holding cell:** a cell at the site, behind the chunk loaders once they're built, receives breezes from the convoy. That way convoy timing doesn't depend on build timing.

### Chamber A and Chamber B (one operation each)

- **Survey:** map every spawner in the chamber and its range. Capture stands are placed outside the ranges of the non-breeze spawners, or those spawners are walled off first.
- **Capture:** bots activate the breeze spawner by standing in range (more bots, bigger waves), and catch each wave into the pack's capture containment, which has no wind-toggleable blocks on the cargo side. Captured breezes go into boats, two per boat, and are logged by UUID as **cargo**. Reflexes never fight back; bots shield and back off.
- **Spawner tracking:** if the mechanics table says the spawner waits for its tracked breezes before cooling down, each batch is moved out of tracking range at once. If not, they're staged at the chamber. Stage S0 measures which.
- **Staging:** small batches leave often (32 per convoy run), because every hour at the chamber is exposure. Staged breezes have a keeper bot within 32 blocks at all times.
- **Hygiene:** no bot with Bad Omen takes a chamber job.
- **Contested:** if enemies arrive, policy decides whether to fight or withdraw. Withdrawing means parking the cargo, not abandoning it.

### Convoy (one operation, a moving region)

- **Moving claim** covering the whole train plus a margin, spanning two dimensions while crossing portals. It's renewed by the escort's squad leader, not through HQ, and registers right of way with each shard it passes through. The nether leg runs through the standing transit operation's route.
- **Formation:** every boat is within range of an escort bot, checked every tick by the squad leader. The train stops when the check fails.
- **Portals:** a bot crosses first and confirms where the link comes out (guarding against a hijacked portal). Cargo then crosses in groups that arrive with a bot already waiting.
- **Losses:** ghasts, lava, piglins and broken routes. Losses within the measured rate only move the ETA; the reserve covers them.
- **Opsec:** if a player is seen following the train, the convoy halts at a park point and only continues when clear.
- **Preemption:** the escort has a floor that can't be preempted. Anything above that is normal; preempting the convoy itself means moving to the next park point and handing the cargo to a keeper.
- **Restart:** convoy state is checkpointed. After a server restart, escorts reconnect first and the convoy resumes from its park state.
- **Handover to the Site:** one step by UUID. Both sides see the same UUIDs, and the Site's claim on the holding cell exists before the convoy releases. A count mismatch is resolved by UUID, not by guessing.

### Disc farm (one operation)

- Site search for creeper spawning, then build. A bot lures a skeleton by aggro, kites it into the trap and makes it persistent, retrying if it's lost. Presence is one bot at the AFK spot, in the 24–128 block band.
- Output goes to the Site's stock as **specific discs per container**, not "any disc".

### Supply quotas

- Slime: no known slime chunks on a server with a hidden seed, so a survey watches where slimes spawn below y=40, then a slime farm is built. This is likely the second-longest chain.
- Quartz from a nether trip, iron from an existing farm, tools for the excavation.

### Gate before arming

Before arming starts, the Site operation re-checks against current belief: the site claim is held, no intruder sightings near it since the survey, concealment is still within budget, the holding cell's breeze count is fresh and at least 256, and the builder's final verification passed. Any failure escalates.

### Arming (needs approval)

- The commander (or a policy the commander set) approves arming.
- With the circuit break still open, bots run the pack's safe-insertion procedure to move breezes from the holding cell into the 1 × 2 cell, counting by UUID at each insertion. One bot stays within 32 blocks throughout.
- Seed TNT is placed. The circuit break is closed last.
- The machine's chunk loaders now keep it loaded. **Null's job ends here.**

### After the build

The operator adds annotations:

```
annotate(marker "OSC-1" at <pos>, notes: "Skybender, >30 min charge")
annotate(marker "OSC-1 fire" at <pos>)
annotate(region "Front A impact", effect: keep_out)
annotate(region "Front A", effect: presence_target)
```

The descriptor's `visit_after_restart` presence means a bot visits OSC-1 after every server restart, to check that both chunk loaders came back. Null does this without understanding what OSC-1 is.

---

## 4. What the commander sees along the way

```
PROGRAM skybender@Front A   ETA 22 h [c]   critical: Convoy (run 3/8, 96/280 breezes at site)
  issue: Chamber B wave sizes below quote (avg 3.1 vs 5 [c]); ETA +4 h; rate now [m]
```

`inspect(program)` gives one row per operation. `inspect(Convoy)` shows the UUID ledger summary (counts, ages, positions). Losses within the measured rate never show as issues, only as ETA.

---

## 5. Firing test

### 5.1 What is being tested

Firing is the operator's job, so the test checks two things separately:

1. **Null's part:** the build matches the schematic, arming happened as specified, the errand does what its contract says, presence and keep-out were honoured, and counts were fresh.
2. **The pack's part:** the Skybender as built and armed by Null actually works. This is the pack's own acceptance test, run in Null's harness.

### 5.2 Where it runs

The test harness (F3): a private vanilla 1.21.11 server with the target server's settings (view and simulation distance, gamerules, difficulty, idle timeout, anti-cheat plugin if any), on a fixed seed with known chambers, site and target range. **The harness has op; the bots never do.** The harness uses op only to set up, observe (`/data`, entity counts, chunk state, MSPT) and simulate conditions (`/tick rate`, restarts, fake players).

### 5.3 Stages

| id | stage | starts from | checks |
|---|---|---|---|
| **S0** | **Small breeze pipeline (run first)** | one chamber, ~10 bots | capture a few dozen breezes, convoy them through the nether, install into a test cell. Measures wave size, cooldown and spawner tracking, persistence in vehicles and in the cell, losses per km, and portal handling. Turns the quote's biggest `[c]` numbers into `[m]` |
| S1 | Site | empty world | concealment budget met, including traffic; cavity matches the footprint; spawn-proofed; nothing outside the claim changed |
| S2 | Breeze pipeline at scale | chambers known, holding cell built | 280 breezes delivered by UUID; no despawns while held; a bot always in range of every vehicle; handovers matched by UUID |
| S3 | Build | site dug, stock seeded by harness | matches the schematic block for block, including per-container contents; circuit break open; no TNT placed |
| S4 | Arming | S3 plus the holding cell | 256 breezes in the cell by UUID, no unplanned block changes or TNT during installation, break closed last, approval logged |
| **S5** | **Firing** | **S4** (and a variant with a harness-pasted, pre-armed cannon to isolate the firing path) | §5.4 |
| S6 | Soak (rare) | empty world, bots only | S1–S5 end to end; real lead time and bot-hours compared with the quote |

### 5.4 S5: the firing test

**Setup (harness):**

- A target range inside the cannon's range: a 32 × 32 grid of armour stands and persistent armoured zombies, some under a roof to stand in for "inside a base". A second range beyond max range for a negative test.
- The annotations from §3, including the `keep_out` on the impact region.

**The operator's tooling** uses one errand:

```
errand "OSC-1 test shot" (priority: high, holds: spotter bot, cannon-side bot, keep_out)
  prepare:
    1. Visit(marker "OSC-1"): count breezes in the cell (fresh count, by UUID)
    2. HoldPresence(region "Front A", band: loaded, outside: keep_out)   # spotter
  on trigger:
    3. Press(marker "OSC-1 fire")
  then:
    4. Wait for impact + 60 s (game time), recount breezes, release
```

The tooling prepares the errand, waits until it reports ready (bots in place, claims held), then calls `trigger(errand, 3)`. The trigger goes straight to the squad holding the errand, with no planning in between.

**Checks on Null's side:**

| # | check | pass if |
|---|---|---|
| N1 | errand preparation | ready state reached; claims held from prepare to release; no other operation took the bots |
| N2 | fresh count | step 1 ran within the staleness limit before the trigger, and the count matched the harness |
| N3 | presence at target | target chunks loaded at impact; spotter outside the keep-out |
| N4 | keep-out | no Null bot inside the impact region from trigger to release |
| N5 | trigger latency | `Press` clicked within 1 game tick of the squad receiving the trigger, measured at the bot's click. The gateway-to-squad time is logged separately |
| N6 | no side effects | no blocks changed outside the cannon and the target range; no claims leaked after release |
| N7 | digest | under its token budget; shows the errand and the annotations, nothing cannon-specific |

**Checks on the pack's side (acceptance of the Skybender as built):**

| # | check | pass if (`claimed`) |
|---|---|---|
| P1 | impact location | ≥ 90% of the payload explodes inside the grid |
| P2 | damage | total damage to targets ≥ 1,000 HP; unroofed targets take most of it |
| P3 | breezes survive | the recount after the shot equals the count before it, or the loss is within the pack's stated rate |
| P4 | reusability | the cannon still matches its schematic (except consumed parts), and a second shot passes P1–P3 |
| P5 | server load | MSPT within a limit during idle (256 breezes ticking with the loaders running), charge and impact |

**Variants (each a separate run):**

| variant | change | expected |
|---|---|---|
| V1 out of range | target beyond max range | the shot misses; nothing in Null breaks |
| V2 lag | `/tick rate 14` during the charge | game durations stretch in real time; no lease or job budget fails because of wall time; results the same in game time. Also checks Azalea handles the tick-rate change |
| V3 restart while idle | server restarts between arming and firing | the visit-after-restart job reaches OSC-1 before other work, confirms both loaders came back, and refreshes the count |
| V4 spotter loss | the spotter dies before impact | Null reports the presence failure to the operator; the test records what the strike does in unloaded chunks |
| V5 intruder | a fake enemy player approaches the site during the charge | reflexes and policy react (alarm, defend); Null doesn't touch the cannon |
| V6 version | the next supported Minecraft version | data and mechanics tables load; S3–S5 pass, or failures point at the version layer |
| V7 hostile spawn | the harness turns off one spawn-proofing block during a visit | the test confirms whether a mob can spawn in the cannon while a bot is present; S1's spawn-proofing check must catch it first |
| V8 stale count | the harness removes 10 breezes after the last visit | the errand's fresh count catches it before the trigger; the tooling sees the real number |

Pipeline variants (restart mid-convoy, escort preemption, a broken exit portal, a player following the train) belong to S0 and S2. They're listed in `BREEZE_PIPELINE_FAILURES.md`.

**Output:** a report per run with every check, the timeline, harness measurements and bot-hours. Measured numbers are written back so later quotes show them as `[m]`.

---

## 6. What this example shows about the design

- **The build is the smallest part.** Getting 256 live breezes from trial chambers into a sealed cell is the critical path, and slime is the next longest chain.
- **The hard problems are at the seams:** handovers, restarts, preemption and arming. Most of the fixes are generic framework rules (§3.14 of the architecture), not cannon features.
- **The framework stays generic.** Everything Skybender-specific is in the pack's descriptors and the operator's annotations and tooling.
- **The quote is where the commander decides,** and the `[c]` marks show which numbers to distrust. Running S0 first is cheap and turns the most important guesses into measurements.
