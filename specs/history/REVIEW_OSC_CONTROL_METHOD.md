# Review: the control method in the OSC worked example

> **Status (2026-10-14): historical.** A review of the worked example against ARCHITECTURE v0.6. Its findings were answered in `REVIEW_RESPONSE.md` and landed in ARCHITECTURE v0.7 and WORKED_EXAMPLE_OSC rev 2. Kept for the reasoning; section references are to v0.6.

Independent review of `WORKED_EXAMPLE_OSC.md` against `ARCHITECTURE.md` v0.6. Written by a reviewer who did not author either file; neither file was changed.

Minecraft facts below are from my knowledge of 1.21.x vanilla. The ones marked **(verify)** are ones I'm less sure of and should be checked in the F3 harness before anyone designs around them.

**Revision 2 (2026-10-13), after Allen's correction.** The first version said the breeze capture and convoy wasn't needed, because live breezes don't produce wind charge *items*. That was wrong for this design. The current cannon, the **Orbital Skybender**, packs **256 live breezes into a 1 × 2 area** and uses the wind charges they throw to deliver the payload, and the payload TNT comes from **TNT dupers**, not crafting. So breeze capture and transport is the real critical path, at about 20 times the scale the example assumed, and the gunpowder and sand points no longer apply. §1, §3.2 and §4 are rewritten to match; §2, §3.3–3.4 and §5 mostly stand.

**Note 3 (2026-10-13).** Allen confirmed the Skybender is a proven design with **dual chunk loaders**, so the mechanics questions in §1.1 (B2, B4, B5, B7) are taken as solved by the design, and bot presence at the cannon only matters for visits, not to keep it loaded. How the breeze pipeline fails *under Null's control* is in `BREEZE_PIPELINE_FAILURES.md`.

---

## Verdict

The shape is right: commander → quote → approve → program of operations → generic jobs, with the cannon kept entirely in content and annotations. The framework/content split holds up, and the bounded quote and digest are the best parts of the design.

But the example is less convincing than it looks, for three reasons:

1. **It models the cannon's real inputs at the wrong scale and in the wrong shape.** The Skybender's key input is 256 live, persistent breezes delivered *into the finished machine*, which is a far bigger and stranger job than the example's 12 breezes feeding a wind charge item farm. Its TNT comes from dupers, so per-shot ammunition is a facility rate rather than a stack of items to load. The descriptor format has no way to say any of this, and the planner trusts whatever the descriptor claims.
2. **The control layering is heavier than the problem.** Making every program step a full operation (own planner, scheduler, ledger, shard, claims, event log) and putting a second planner at HQ adds a lot of machinery and contradicts several things v0.6 says about itself. The same program fits in five operations cut by place, with one planner.
3. **The generic primitives have gaps the example hides.** The operator's firing script can't be expressed in the §4.1 command API, the keep-out region needs Null to interpret an annotation that §3.12 says it never interprets, and "presence" is free text the planner can't plan against.

The essential complexity (entities, presence, logistics at scale) is real, and with 256 breezes it's even more dominant than the example says. Most of the accidental complexity is in the control layers, and it's cheap to remove now, while this is still a design.

---

## 1. What the example gets wrong about the Skybender

### 1.1 The breeze job is about 20 times bigger and ends inside the cannon

The example captures 12 breezes and convoys them to a separate wind charge farm. In the Skybender, 256 breezes are part of the machine. That changes the job:

| # | problem | consequence |
|---|---|---|
| B1 | **Capture throughput is capped by trial spawners.** A breeze trial spawner spawns a few breezes per wave (more with more players in its detection range) and then goes into a cooldown of about 30 minutes **(verify the exact wave sizes and cooldown)**. Even with around 10 bots in range, 256 breezes is on the order of 20+ waves, so roughly half a day per spawner. | Capture, not the build, sets the lead time. The planner should run several spawners or chambers in parallel, and the quote needs spawner count × wave size × cooldown, not a flat rate. |
| B2 | **The spawner may not release captured breezes.** A trial spawner tracks the mobs it spawned. If it waits for live, captured breezes to die or leave before it finishes the wave and starts its cooldown, capturing stalls it **(verify: how far a tracked mob must be before the spawner forgets it)**. | This decides whether capture is "cage at the spawner" or "move each batch away at once", and it belongs in the mechanics table. |
| B3 | **Transport capacity.** A boat holds 2 mobs and a minecart holds 1. 256 breezes is 128 boats or 256 minecarts, or many trips. | The convoy (O4) is a large entity-logistics operation, with an escort and a moving claim long enough to cover the whole train. |
| B4 | **Persistence for 256 hostile mobs.** Hostile mobs despawn at once beyond 128 blocks from every player and can randomly despawn beyond 32. Riding a vehicle protects them, but packed into the 1 × 2 cell they're no longer passengers. | Either 256 name tags (each needs a name tag item, an anvil and XP), or a bot within 32 blocks of every breeze at all times, including the long wait between capture and firing. Name tags aren't in the bill of materials. |
| B5 | **Entity cramming.** Vanilla's `maxEntityCramming` is 24: a mob sharing space with more than 24 pushable entities takes damage, and bots can't change gamerules without op. | The design must already avoid this somehow (for example, entities that don't count as pushable). The descriptor should say how, because the builder and the convoy have to preserve it while loading the cell. |
| B6 | **Loading 256 mobs into a redstone machine.** The builder research says mobs go in after the blocks are inert, insertion is trialled, and results are stochastic. 256 insertions into a 1 × 2 cell is the extreme case. | "Deliver breezes into the built cannon" is its own step after O9, probably with a capture-cell macro, not something O5 or O9 covers today. |
| B7 | **The concealment budget contradicts the design.** The descriptor caps entities at 20 per chunk, and the machine holds 256 breezes in one chunk. | Either the cap is wrong for this cannon, or concealment has to rely on depth and distance instead. A pie chart near the site will show the breezes. |
| B8 | **Server load.** 256 ticking breeze AIs in entity-ticking chunks for the whole time a bot is near is a real MSPT cost, and P5 should measure it during the idle wait, not only during the charge and impact. | — |

### 1.2 The payload comes from TNT dupers

- **Gunpowder and sand drop out.** My first version counted 8,000 gunpowder and 6,400 sand for 1,600 crafted TNT. With dupers, the per-shot payload needs a few seed TNT and the duper, not a crafted stack.
- **The duper is a facility,** with a rate (TNT per tick or per cycle), presence needs and a failure mode, so it fits the facility model well. Its output is **primed TNT entities** in place, not items, so `per_shot_inputs: tnt 400` and the `FillContainer(tnt hopper, 400)` step are probably the wrong shape. The descriptor should say what the operator loads per shot, if anything.
- **The build's 1,200 TNT may be wrong too.** If most of that TNT is duper output during a shot, the bill of materials shouldn't ask logistics to craft it. If real TNT blocks are placed in the build, see the TNT point in §4.

### 1.3 Smaller errors that still stand

| # | where | problem | consequence |
|---|---|---|---|
| S1 | §5.4 N1 | If any load step does go through a hopper: a hopper has 5 slots, so it holds at most 320 of a 64-stack item, and it pushes items onward every 8 ticks. "Holds exactly N" can't be tested on it. | `FillContainer` should promise "N items delivered into the container's input", not "container holds N". |
| S2 | `music_disc_*  any disc` | A jukebox's comparator output depends on which disc is in it. If the cannon uses discs for signal strength, "any disc" builds a cannon that doesn't work. | The bill of materials needs **per-container contents**, not just item counts. This is a builder input too. |
| S3 | O2 "deliberately dark" | Since 1.18, hostile mobs spawn at block light 0. The bot keeping the cannon loaded is a player, so it makes mobs spawn in the dark cavity. A creeper inside the machine, next to TNT and 256 breezes, ends the program. | Spawn-proofing (slabs, light ≥ 1, or non-spawnable floors) must be part of the site spec for the whole life of the cannon, not just the build. |

**Process finding.** The example didn't model the Skybender's real inputs, and nothing in the method would notice. The planner reads descriptor rates and entity inputs as facts, and a first-time reviewer (me) also guessed wrong about what the breezes are for. A descriptor should say *how* each input is used (items consumed, entities installed in the machine, entities feeding a farm) and carry each rate as `claimed` until it's measured once in the harness. The quote should say which numbers are which.

---

## 2. Where the example contradicts ARCHITECTURE v0.6

| example does | v0.6 says | suggested fix |
|---|---|---|
| HQ expands descriptors into a production graph, prices it, tracks the critical path and coordinates nine operations | §3.2: "HQ is deliberately thin" | Either accept that HQ hosts the planner and say so, or move program planning into a **program service** (a capability service HQ calls, like the builder). I'd do the second. |
| A convoy's moving claim crosses overworld, nether and overworld, through regions owned by other operations or by nobody | §3.7: cross-shard claims "are rare" | Convoys and long travel make them routine, and a 256-breeze convoy makes them large. The nether needs an owner (a standing "transit" operation, or HQ owning nether claims). |
| The operator's script calls `FillContainer`, `HoldPresence`, `Press` | §4.1 has no command that runs a job; commands target goals, operations and selectors | Add one generic goal, e.g. `Errand(steps, at markers, priority)`, that HQ hands to the nearest operation. Then firing is an ordinary goal with an ordinary audit trail. |
| Bots must stay out of `OSC-1 impact keep-out` | §3.12: Null stores annotations and "doesn't interpret them" | Give annotations a small closed set of typed effects (`keep_out`, `defend`, `rally`, `presence_target`) that pathing and policy honour. Everything else stays free-text notes. |
| The quote reports "+2 bots during each charge" | The descriptor's `operation:` block is "read by the operator's tooling, not by Null" | Pick one. I'd let Null read presence needs (it must, to quote them) and leave only firing logic outside. |
| `Build(pack=…, facility=…, bind: target_region = …)` | §3.6: `Build(schematic, site)` | Fine, but variable binding plus enum methods like `capture_and_convoy_or_build_in_place` is the start of a DSL. Freeze a small typed schema for C11 now. |

---

## 3. Complexity

### 3.1 How much machinery one cannon touches

To fire one test shot, the example uses: commander, HQ, program, 9 operations, squads, bots; jobs; claims (region, cross-operation, moving); item ledger and entity ledger; facilities with 5 states; sites; presence; content packs with descriptors and variable binding; annotations; quote, production graph and critical path; HTN planners per operation; world shards; coarse map; digest and `inspect`; policies and approvals; and a harness with 5 stages and 6 variants. It touches about a dozen components (C1, C2, C3, C4, C5, C9, C10, C11, B7, K1, K2, F3).

Some of that is essential. Entities, presence and multi-facility logistics really are the hard part, and the Skybender makes the entity part dominant. The rest comes from how the control layers are cut.

### 3.2 One operation per step is the main source of accidental complexity

§3.3 defines an operation as "one goal in one area", a service running 50–300 bots with its own planner, scheduler, ledger, shard, claims and event log. The example makes each program step an operation, so a 6-bot scout survey gets the same machinery as a 150-bot build, and every handover between steps (site, spoil, stockpile) becomes a cross-operation transfer.

Cut by place instead, the program is five operations:

| operation | covers | why one |
|---|---|---|
| **OSC site** | survey, excavation, stockpile, build, duper, breeze installation (O1, O2, O8, O9, plus the B6 insertion step) | same region, same claims; the spoil feeds the casing directly |
| **Chamber(s)** | survey and breeze capture (O3 and the capture half of O4) | one place, one danger profile, one presence plan; one per chamber if several run in parallel (B1) |
| **Breeze convoy** | the transport half of O4 | the only operation whose region moves, so it needs the moving claim and its own escort |
| **Disc farm** | O6 | its own site |
| **Existing farms** | whatever O7 still needs as a quota on operations that already run them | they already exist |

Inside each operation the steps are just the HTN's ordering. The program DAG at HQ shrinks to five nodes and their handovers, which is what HQ should be reasoning about.

### 3.3 Two planners that expand the same graph

HQ builds a production graph for the quote, and each operation runs an HTN over the same recipe graph. Who expands `sticky_piston ×300` into slimeballs and pistons and iron? If both do, they'll disagree. I'd draw the line like this:

- **One production-graph planner** (the program service) decides *which facilities and sources* and *how much of each, at what rate*. It owns the quote.
- **Operation HTNs** only decide *how to do the jobs* for the quotas they were given, and never re-derive sources.

### 3.4 Presence is a constraint on where bots are, not a bot-hours line item

The quote counts presence as bots held permanently. In practice a bot holding presence can still craft, sort or guard nearby, since being there is the whole requirement. And presence isn't fungible:

- capture rates depend on **how many** bots stand near a trial spawner (B1), and farm rates depend on **where** an AFK bot stands (spawn bands of 24–128 blocks);
- unnamed breezes need a bot within 32 blocks the whole time (B4), which is a different band from "within simulation distance";
- every bot is a player, so it makes hostile mobs spawn near itself (S3), and bots close together share mob-cap and spawning;
- vanilla's `player-idle-timeout` can kick idle players if the server sets it;
- server restarts drop every holder at once.

So presence should go into the scheduler as **location constraints over time**, which other jobs can be packed around, and the descriptor needs a structured form for it instead of free text:

```
presence:
  - at: marker "OSC-1" | facility_anchor | region "target"
    band: entity_ticking | loaded | within_32 | within_128 | 24_to_128
    count: 1          # or a function: rate depends on count
    phase: always | while_running | during_charge
    standoff_outside: region "impact keep-out"   # optional
```

With that, the planner can check feasibility before quoting. Example: the target spotter must be within the server's simulation distance of the target chunks **and** outside the keep-out. On a server with simulation distance 4 (64 blocks), that ring may not exist. The server sends its simulation distance at login, so Null can know it.

---

## 4. Risks the example underweights

- **The "..." in the bill of materials still hides work.** 900 slime blocks plus 300 sticky pistons is about 8,400 slimeballs, and slime chunks are seed-derived, so on an SMP with a hidden seed you find them by watching where slimes spawn below y=40, which is itself a presence-heavy survey. 250 observers need 250 nether quartz, so a nether trip. 256 name tags (B4) need fishing, loot or librarian trades. None of this is in the program DAG. Breeze capture is the likeliest critical path, with slime next.
- **Concealment fights presence.** A bot that sits at a "hidden" buried site for every charge, plus carrier traffic, a 256-breeze convoy arriving, and 92k blocks of spoil, is a bigger tell than any per-chunk count. The concealment model should count bot traffic and dwell time, not just block and entity counts.
- **TNT near redstone is the worst mid-build case.** The builder research lists TNT among actuators that fire mid-build. Whatever real TNT blocks the duper and payload path need (seed TNT at least) should go in last, behind a physical break in the circuit that's closed as the final step, and the descriptor should say where that break is. A duper misfiring during the build is the same risk.
- **31 hours (likely more, given B1) of stale belief.** The site was verified at hour 0. There's no gate that re-checks it (claims, intruder sightings, concealment) before the build, before the breezes go in, and before each shot. Handover edges in the program should re-verify their preconditions.
- **Cancel and partial progress.** If the target region moves, or the site is found halfway through, what's unwound and what's kept? Captured breezes are expensive to hold and pointless without the cannon. The program needs a defined cancel path that releases claims and returns stock to HQ.
- **Tool wear and spoil.** 92k deepslate blocks wear out on the order of a dozen or more diamond pickaxes, and moving 1,400+ stacks of spoil 200 blocks away is a carrier operation in itself. Neither is in the quote.

---

## 5. The firing test

The split between Null's checks (N1–N7) and the pack's acceptance (P1–P5) is good, and so is "harness has op, bots never do". Comments:

- **N1 should follow the real load path.** With dupers, the per-shot load may be a few seed TNT or nothing. If a hopper is involved, "holds exactly N" can't pass (S1).
- **Add a check that the breezes survive.** Between installation and firing, and after each shot: 256 breezes still present, none despawned or crammed (B4, B5). A shot that loses breezes isn't reusable, so P4 depends on it.
- **P3 (±2 s from `Press`) actually tests Null, not the pack.** It measures the latency from the operator's script through gateway, HQ, operation and squad to the bot. With HQ deciding every ~10 s (§2) and an `Errand` that has to be allocated, ±2 s is optimistic. Either give errands a fast path or measure the press against the moment the bot clicks.
- **Steps 1–7 aren't atomic.** Between the load and the press, preemption or another operation's claim can take the bot or empty the stock chest. Firing should be one errand that holds its claims (bots, containers, keep-out) from load to release.
- **V2 (`/tick rate 14`) tests something more important than the cannon:** whether leases, job budgets and waits are counted in game ticks or wall time. The builder already counts in game ticks (`ONLINE_BUILDER.md`); claims and job budgets in v0.6 don't say. Game processes (charge time, spawner cooldowns, farm rates, settle times) should be in ticks, network timeouts in wall time, and the contract should state which is which. Also check that Azalea handles the server's tick-rate packet.
- **S2 needs to be rescaled** from 12 breezes to 256 delivered *into the built cannon*, and is now the stage most worth running early.
- **S5 (soak) is days long with about 150 bots.** It's worth running, but rarely. A cheaper slice gives the same design signal sooner (see below).
- **There's no check for the risks in §4**, for example a creeper spawning in the cavity during the charge, or a stale site at build time. A variant V7 "hostile spawn in the cannon during charge" is cheap and would catch S3.

---

## 6. Recommendations, in order

1. **Fix the example's inputs to match the Skybender:** 256 breezes installed into the machine, persistence and cramming handled, TNT from dupers modelled as a facility, and the wind charge item farm removed if the design doesn't need wind charge items.
2. **Extend the descriptor** so each input says how it's used (consumed, installed, feeds a farm), and mark each rate `claimed` until it's measured once in the harness. The quote shows which numbers are measured.
3. **Cut operations by place, not by step,** and keep one production-graph planner (a program service) above per-operation HTNs that don't re-derive sources.
4. **Make presence structured** (location, band, count, phase, standoff) and schedule it as location constraints. Check feasibility against the server's simulation distance at quote time.
5. **Close the API gaps:** an `Errand` goal for the operator's tooling, with held claims and a fast path, and a closed set of typed annotation effects such as `keep_out`.
6. **Decide game-time versus wall-time** for every lease, budget and wait.
7. **De-risk the breeze pipeline first, at small scale:** capture a few dozen breezes at one chamber with about 10 bots, convoy them through the nether, and install them into a test cell, while measuring wave sizes, spawner cooldown and tracking (B1, B2), persistence (B4) and cramming (B5). This is now the riskiest part of the cannon, and it exercises entities, presence, moving claims and descriptor validation at a small fraction of the cost.
