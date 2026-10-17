# Null: swarm architecture v0.8

**v0.8, 2026-10-14.** Design only. The bot client is **Azalea** (Rust), using its swarm mode, on Null's own Azalea fork (§9). The target is **Java Edition 1.21.11**, and the code must port to other versions (§9.1). It scales from a handful of bots to about 2,000 across several hosts and servers under one HQ (§2). The component breakdown is in §10; each component has a spec in this folder (see `INDEX.md`), and the decisions still open are listed in `INDEX.md` and argued both ways in `DECISIONS.md`. What changed since v0.7 is at the end (§11). Section numbers are unchanged from v0.7, so references to them stay valid.

**Goal.** A swarm of Minecraft bots under one commander (a person or an LLM). The swarm survives and beats the game, builds large decorative and redstone structures, and fights as a faction on SMP servers.

**Hard constraint.** Live survival servers: no op, no creative, no WorldEdit, no schematic pasting, no `/tick freeze`, no `/forceload`. Every block is mined, carried and placed by a bot. Everything below assumes that.

**Pure vanilla servers.** Null assumes the server runs unmodified vanilla: no Carpet, no accurate-placement protocol, no helper or land-claim plugins. Which particular server the swarm plays on is not an architecture question; adapting to one is a settings profile (F5 §4.6). Other server software is out of scope unless a target ever needs it (Allen, 2026-10-13).

**Prototyping exception.** Builder prototypes may run in creative with op on a private test server, to defer resource acquisition and block breaking. Flight is banned even there. This is a test convenience only; nothing in the final system may depend on creative or op.

---

## 1. The concepts that will dominate

These are the ideas that most of the code, and most of the bugs, will be about. Get these right and the features are "a lot of work"; get them wrong and every feature fights every other.

### 1.1 Belief, not world state

No bot sees the world. Each sees a few loaded chunks, through a laggy connection, and what it saw goes stale the moment it walks away. The swarm's knowledge is therefore a **belief**: a merged map of observations, each stamped with *who saw it, when, and how*. Other players change the world between looks.

Consequences that leak everywhere:

- Every query answers "what we last knew, and how old it is", never "what is".
- Plans are made against belief and checked against observation when a bot arrives. Arrival-time mismatch is normal, not an error.
- Chests are the worst case: their contents are only known when someone opens them, and on an SMP someone else may have emptied them.

### 1.2 Claims (reservations and leases)

Many bots acting on one world collide constantly: two bots mine the same ore, two bots stand in the same cell, a gatherer digs out a block a builder is relying on, a bot loots the chest a crafter was saving. The single most important coordination primitive is the **claim**: a time-limited, revocable lease on something.

| claim on | example |
|---|---|
| a region (volume) | a build site; a mining tunnel; a farm |
| a block or small set | "I'm mining this ore vein" |
| a container | "I'm taking 32 iron from this chest" |
| items in a container | soft reservation of stock for a pending craft |
| an entity | "I'm killing this cow"; "I'm fighting that player" |
| a bot | a job owns the bot's body until it releases it |

Claims have an owner, an expiry, and a priority. Higher-priority work can **preempt** a claim, which tells the holder to stop cleanly. Most coordination bugs will turn out to be missing or leaked claims.

**Claims, permits and territory are three different things** (Allen, 2026-10-13; `CONVENTIONS.md` §3.3a):

- a **claim** is Null's internal reservation between its own operations and bots. It grants no permission to change anything;
- a **permit** is the operator's approval for the swarm to modify blocks in an area. Only the operator grants one. It says nothing about who owns the land: on pure vanilla nothing allocates land, and land-claim plugins are out of scope. Natural blocks need no permit (§4.2);
- **territory** is what the faction defends and patrols (C8). It usually lies inside permits, but defending an area and being allowed to modify it are separate settings.

### 1.3 Jobs with contracts

All work, from "chop 16 logs" to "build this castle", is a **job** with an explicit contract:

- **inputs:** parameters, required items, required claims;
- **preconditions:** what must be believed true to start;
- **success test:** an observable check, not "the code returned";
- **outcomes:** `succeeded`, `failed(reason)`, `blocked(on what)`, `preempted` (Null stopped it on purpose), `interrupted(incident)` (the world did something it didn't expect, §1.7), each with a structured report;
- **budget:** time and attempts, after which it gives up and says why.

Bots never improvise new goals. They run jobs and report outcomes. Their squad and operation decide what happens next. This is what keeps thousands of bots legible to one commander.

### 1.4 Layered control at different time scales

Minecraft runs at 20 ticks per second. An LLM answers in seconds. A build takes hours. These cannot be one loop.

| layer | time scale | lives in | examples |
|---|---|---|---|
| **Reflexes** | 1 tick (50 ms) | each bot | dodge, shield, crit, eat when low, MLG bucket, flee lava, re-path |
| **Skills** | seconds to minutes | each bot | mine a vein, craft a recipe, smelt, travel, place one build step, fight one mob |
| **Tactics** | 0.1–1 s, multi-bot | squad leader | focus fire, escort, surround, retreat together |
| **Operations** | seconds to minutes | operation and HQ services | allocate jobs, move materials, run a build, run a raid |
| **Strategy** | minutes to hours | commander | "beat the dragon", "fortify the base", "build this schematic here" |

The rule: **a layer may only be as slow as the layer it waits on is patient.** Reflexes never wait on the network. Tactics never wait on the LLM. The commander is never in a loop that has to close faster than a few seconds.

### 1.5 Logistics: the swarm is an economy

Every survival goal turns into a supply chain. A 20k-block build is 20k items that must be mined or farmed, smelted, crafted, carried, stored and handed to the right bot at the right stand. Diamonds for gear, food for everyone, tools that break. Operations and HQ spend most of their time on:

- **bill of materials** for every goal, expanded through recipes down to raw sources;
- **stock**: what the swarm holds, where (bot inventories, chests, shulker boxes, ender chests);
- **flows**: who fetches what from where, delivered to whom;
- **production**: farms and smelters as long-running jobs that turn time into items.

Inventory slots (36 per bot, plus shulkers) and walking distance are the real constraints, not CPU. Items are only one kind of resource. Entities, facilities, sites and bot presence are the others, and they are harder (§3.10).

### 1.6 Capabilities as a library

Bots are interchangeable bodies running the same **skill library**. A skill is deterministic code with a typed contract (§1.3). The universal builder is one capability among many; so is "beat the ender dragon", which is itself a composition of smaller skills. The commander and the orchestrator compose skills; they never write per-tick behaviour.

*Decided (AD-K5.2, Allen, 2026-10-15): scenarios can't be listed, so skill code is written only for the important parts, chosen by the cost-ranked interrupt review. Mob and place knowledge are F5 rows whose values come from the game jar, from observation or from the LLM's prior, never written per row. Unknown cases use the generic default. See [GENERALIZATION.md](proposals/GENERALIZATION.md) §0.*

### 1.7 Failure is the normal case

Things that will happen every hour: a bot dies, disconnects, gets stuck, is kicked by anti-cheat, runs out of food, finds its chest looted, gets ambushed, finds the server restarted. Each needs a defined recovery: requeue the job, release claims, recover the corpse's items, mark the region dangerous, tell the commander if it repeats. A design that treats these as exceptions will not survive a day on an SMP.

**Anything else is an interrupt** (AD-K5.3, Allen, 2026-10-14; [INTERRUPTS.md](INTERRUPTS.md)). A failure that matches no known class is contained, not diagnosed. The bot backs off, the place is quarantined, and the job ends `interrupted` and goes elsewhere. It escalates only when avoiding the place costs more than the approved quote allows, not because it happened or happened again. Fixes come from a cost-ranked batch review with a token budget.

### 1.8 Adversaries and secrecy (faction play)

On an SMP, other players read chat, watch for bots, raid bases and steal. This adds:

- **threat model:** known players, their relation (ally, neutral, enemy), last seen where, gear;
- **operational security:** no coordinates in public chat, no obvious bot behaviour near others, swarm communication only on the out-of-band bus;
- **territory:** what the faction holds and must defend; patrols, alarms, retreat points.

Also a practical risk: many SMP servers forbid bots, and even pure vanilla kicks a player whose movement is physically slightly wrong. Anti-cheat plugins (Grim, Vulcan) are other server software and out of scope unless a target server needs them (F5 §4.6). Which servers the swarm plays on, and whether it is allowed there, is a decision for the operator, not the software.

---

## 2. Scale and shape

**Scale target:** from a handful of bots up to about **2,000**, spread over a few home machines and a couple of dedicated VPSes. The same software must run both ends; a 20-bot deployment is just the big one with most levels collapsed into one process.

Two facts at 2,000 bots shape everything below:

- **No single process sees everything in detail.** Not the orchestrator, not the commander, and certainly not an LLM. If each bot's job took 1,000 tokens to describe, 2,000 of them would be 2 million tokens, more than most models can read at all. So every level of control sees a **bounded summary** of the level below, never the bots themselves.
- **Nothing central may sit on a hot path.** Per-bot decisions, claims and observations are handled close to the bots. Only aggregates and exceptions travel up.

So control is a **hierarchy with a fixed span at each level** (about 10–50 children per parent), like a military chain of command:

| level | count at 2,000 bots | owns | thinks about | decides every |
|---|---|---|---|---|
| **Commander** (person or LLM) | 1 | intent | goals, priorities, policies | minutes |
| **HQ** | 1 | the whole swarm | which operations exist; bots and resources per operation | ~10 s |
| **Operation** | ~10–30 | one goal in one area (a base, a build, a mining district, a war front) | its squads, its stock, its region | ~1–2 s |
| **Squad** | ~100–200 | 5–20 bots doing one kind of work together | jobs for its bots; tactics in a fight | 0.1–1 s |
| **Bot** | 2,000 | one body | reflexes and the current skill | every tick |

Each level talks only to its parent and its children. Each level reports up a **summary whose size depends on its number of children, not on the number of bots under it**. That is what keeps the commander's context bounded (§4).

**One HQ per deployment, spanning servers** (Allen, 2026-10-13, AD-K1.1). A deployment may play on several servers at once, all under one HQ and one commander. Every location key carries a `server` field, items and entities never cross servers (so cross-server transfers are refused at planning time), the digest groups operations by server, and a swarm node never mixes servers (K1). The whole deployment normally runs one Minecraft version; two run at once only when a wanted server forces it (§9.1).

**Capacity.** One laptop may carry about 1,000 bots, at roughly 1 MB of extra memory per bot in a swarm, so the limits are CPU and above all bandwidth, not memory (Allen, 2026-10-13). A swarm node is one ECS that must finish each 50 ms tick; the AD-F5.2 spike measured about 11 µs of tick per bot for physics and movement alone on 4 cores (`reference/SPIKE_F5_BOUNDARY.md`). Bandwidth grows roughly with the square of bots working in one area, because every bot is sent every nearby player's movement, so density is a scheduling constraint (F1 §4.11). Local testing is per component, not a full 2,000-bot run.

At 20 bots, HQ, one operation and two squads run in one process on one machine. The roles are the same; only the deployment changes.

```
                 Commander (person or LLM)
                          │  goals, policies ▲ fixed-size digest
                         HQ ───────────────── capability services (builder planner, long-range pathing, …)
          ┌───────────────┼────────────────┐
     Operation        Operation         Operation        ~10–30
     "Main base"      "Castle build"    "North front"
      ┌───┴───┐        ┌───┴────┐         ┌──┴──┐
    Squad   Squad    Squad    Squad     Squad  Squad      ~100–200
     │││     │││      │││      │││       │││    │││
     bots    bots     bots     bots      bots   bots      ~2,000
```

---

## 3. How the orchestration hierarchy works

### 3.1 What flows between levels

Every link in the hierarchy carries the same three things, at different grain:

- **Down: intent.** A goal or quota plus constraints ("produce 20k stone by 18:00 into staging area B", "hold this region", "you get 120 bots"). Never a list of per-bot actions.
- **Up: a summary.** Fixed fields per child (state, headcount, progress, stock, health), plus the top few **exceptions** ranked by severity. Similar exceptions are merged: "37 bots in Mining-3 blocked: no pickaxes" is one line, not 37.
- **Up: escalations.** Problems a level can't solve with what it owns. A level must try its own remedies first (its recovery rules, its stock, its own bots), and only then escalate, with the remedies it already tried.

The rule that keeps this stable: **a level only escalates what its parent can act on.** HQ can move bots or resources between operations; an operation can't, so "need more miners" goes to HQ. A squad can re-path a stuck bot; HQ can't, so that never leaves the squad.

### 3.2 HQ

HQ is deliberately thin. It owns:

- the **goal tree** from the commander, and the **program** each goal becomes. The planning itself (production graph, quote) is done by the **program service** (§3.11), a capability service HQ calls, so HQ stays thin;
- the **operator's map annotations** (§3.12), for display and for goals to refer to;
- **resource allocation between operations**: bot headcount (by role and gear), and transfers of items between operations' stores;
- the **swarm-wide policies** and their enforcement;
- **cross-operation claims**: big regions, territory borders;
- the **digest** for the commander (§4).

HQ never assigns a job to a bot. It allocates *bots to operations*, as quotas, and lets operations do the rest. Moving 30 bots from mining to defence is one HQ decision, not 30.

### 3.3 Operations

An operation does most of the scheduling, scoped to one goal and one area.

**Operations are cut by place, not by step.** One area with one set of claims is one operation, even when many steps happen there in sequence (survey, dig, stock, build). Those steps are just the order its HTN runs jobs in. A new operation is only worth it for a different area, a region that moves (a convoy), or a different danger profile. An operation has:

- an **HTN planner** that orders and runs jobs to meet the quotas the program service gave it. It decides *how*, never *where from*: which facility or source supplies an input is the program service's decision (§3.11);
- a **scheduler** that hands jobs to its squads (and through them to bots);
- its own **logistics ledger**: stock in its chests and its bots' inventories, reservations, carrier runs;
- the **world model shard** and **claims registry** for its region (§3.7);
- **recovery rules** for its jobs.

A typical operation runs 50–300 bots. Operations are independent services: one crashing or lagging doesn't touch the others.

### 3.4 Squads

A squad is 5–20 bots with a leader process (not a leader bot) that:

- takes job batches from its operation and assigns them to its bots;
- reruns or reassigns failed jobs locally before reporting anything;
- in combat, runs the fast tactical loop: target choice, formation, focus fire, retreat (4–10 Hz);
- merges its bots' observations before passing them up.

Squads are cheap and can be formed and dissolved by the operation as work changes. Keeping squads small and on the same host as their bots (§3.8) is what makes tactics fast.

### 3.5 Bots

As before (§5): reflexes, one skill at a time, local fallback. A bot only ever talks to its squad leader.

### 3.6 Goals and planning

A **goal** is what the commander asks for: `Obtain(item, n)`, `Build(schematic, site)`, `Defend(region)`, `KillDragon`, `Explore(area)`, `Hold(position)`.

- **HQ**, through the program service, decomposes a goal into **operations** and gives each a sub-goal, quotas, a budget and a region. `KillDragon` might become "Overworld economy", "Nether expedition" and "Stronghold assault", each with its own operation.
- **Operations** decompose their sub-goal into jobs with an HTN planner over the skill catalogue and the recipe graph (the tech-tree DAG: wood → stone → iron → diamonds → nether → blaze rods and pearls → stronghold → fight). They re-plan when jobs fail, rather than planning everything up front, but they never re-derive sources.

Both planners are ordinary deterministic code. The LLM sits above HQ (§4).

*Decided (AD-K5.2, Allen, 2026-10-15):* there are no operation "methods" per goal kind. Steps that an existing search solves (K7's recipe graph, B1, C5, B4) expand deterministically. The LLM commander (alone in v1, AD-K5.1) plans the steps that have no model, in stretches of goals and verb plans that are validated and quoted before they run. It wakes on K3 §4.2's triggers. Plans that work are saved as routines with parameters. Goals are the language to state and check a task. There is no general goal-state planner. See [GENERALIZATION.md](proposals/GENERALIZATION.md) §0.

### 3.7 State is sharded by region

There is no single central world model or claims table at 2,000 bots. Both are **sharded by region**:

- Each operation owns the **world model shard** and **claims registry** for its region. Its bots write observations there; nothing per-block goes to HQ.
- HQ keeps a **coarse map**: regions, owners, territory, threats, known structures, and per-region summaries (stock, headcount, danger).
- Claims come in three kinds:
  - **region claims**, held in the shard that owns the area;
  - **cross-shard claims** (a tunnel between two bases), owned by one side and forwarded by the other, with HQ arbitrating disputes;
  - **moving claims**, for convoys and long travel. These are routine, not rare. The convoy owns its claim, which covers the whole train plus a margin and can span two dimensions while the train crosses a portal. It registers right of way with each shard it passes through, and it is renewed locally by the escort's squad leader, never through HQ. Routes used regularly through unowned space (most of the nether) belong to a standing **transit operation**.
- Items are in **per-operation ledgers**. HQ sees only aggregates ("swarm has 41k cobblestone, 3k iron, 212 diamonds") and moves goods between operations as **transfer orders** carried out by carrier bots.
- Every shard writes an append-only event log, so a restarted operation rebuilds its state from it.
- **Belief is a separate store** on top of Azalea's shared chunk storage, which holds only blocks, biomes and heightmaps with no timestamps (AD-F2.2, decided 2026-10-15: fed by a packet observer that forwards packets to a worker thread; `reference/BENCH_AZALEA_2026-10-15.md`). **Decided (AD-F2.1, Allen, 2026-10-15):** belief is keyed by fixed map tiles, not by operation as above. Operations write the tiles they cover while they hold the claims, and the operation service runs inside the swarm node, so observations never cross the bus (`DECISIONS.md`).

### 3.8 Distributed runtime

Hosts: a few home machines and a couple of VPSes. On them run:

| process | where | how many |
|---|---|---|
| **Swarm node:** an Azalea swarm process hosting bots for one server and the squad leaders for those bots | any host, sized by its CPU and bandwidth (up to about 1,000 bots on a laptop is the target) | a few to ~40 |
| **Operation service** | preferably on the host that runs most of its bots | ~10–30 |
| **HQ** | a VPS (stable, always on) | 1, with a standby |
| **Message bus** (NATS cluster, with leaf nodes on home machines) | VPSes, with leaf nodes elsewhere | 1 cluster |
| **Commander gateway:** the command API, CLI and LLM adapter | a VPS | 1 |

Placement rules:

- **Co-locate by region.** Azalea's swarm mode only shares world memory *within* one process, so bots working in the same area should share a node. An operation's bots should sit on as few nodes as possible.
- **Squads never span nodes.** Squad leader and bots are in one process, so tactics never cross the network.
- **Home machines are unreliable.** Put calm, restartable work there (mining, farming, building). Put HQ, the bus core and war fronts on VPSes.
- **Accounts are portable.** A bot is an account plus its job; if a node dies, the operation reassigns those accounts to another node and they log in again. Their in-flight jobs come back as `failed(disconnected)`. Most are re-planned. Jobs holding state that can't be recreated (convoys, keepers, staged cargo) are **checkpointed** and resume from their last saved state instead (§3.14).

Traffic budget: the bus carries summaries, intents, escalations and job batches, not raw observations. Observations stay on the node and in the operation's shard. A node's upward traffic should be in the low kilobytes per second, whatever its bot count. The Minecraft connections are the expensive traffic: each node counts bytes per bot, and placement treats host bandwidth as a capacity limit (F1 §4.11).

Failure modes:

- **Node down:** its bots disconnect; the operation re-plans and reassigns accounts.
- **Operation down:** its bots finish their current safe step and hold in their fallback mode; the operation restarts from its event log and resumes.
- **HQ down:** operations keep running their current goals; nothing new is allocated. The standby takes over from the shared log.
- **Network partition:** leases (claims, job assignments) expire on their own, so a lost link never leaves anything permanently locked. A node that can't renew its leases **fences itself** before they expire: its bots finish the current safe step and log out, so two nodes never fight over one account. Cargo keepers are the exception: they stay online and hold, and their accounts aren't re-placed while the player list shows them online (F1 §4.2, §4.7).
- **Server restart:** the most dangerous event, because every bot, keeper and escort drops at once. It has its own protocol (§3.14).

### 3.9 Where the builder fits

The builder is a **capability service** that operations call:

1. Commander: `Build(schematic, site)`. HQ creates a build operation for the site, with a budget.
2. The build operation claims the site region and asks the builder to plan.
3. The builder returns a plan: ordered steps, the **bill of materials per step**, the stands and temps it needs, and the claims to hold.
4. The operation turns the bill of materials into supply jobs (and HQ transfer requests), stages materials, and gives build steps to builder squads.
5. Bots report every step; on divergence they halt and the builder repairs or reports stuck.

At this scale a big build is one build operation plus several supply operations feeding it. Most bots are suppliers; a few builder squads place blocks.

Interface questions for the builder (being redesigned as world transformation in `proposals/WORLD_TRANSFORMATION.md`; C1 specifies the interface, and AD-C1.1, AD-C1.2, AD-B6.1 and AD-F3.1 were decided 2026-10-15 in that design's frame; see C1, B6 and F3):

- **Multi-bot building.** Does the builder partition into regions that bots build in parallel, or is the guarantee per-bot, with only one bot in a redstone region at a time and others doing decorative shell or logistics? At hundreds of bots, the decorative shell needs a parallel mode.
- **From prototype to survival.** `ONLINE_BUILDER.md` v4.1 builds in creative with `/forceload` because it is a prototype (see the top of this doc). The final builder has to take items from logistics, spend real time breaking temps, and handle the drops they leave. It also has to keep the site loaded by having a bot nearby instead of using `/forceload`. The builder interface should carry per-step item needs and break-time from the start, so the switch doesn't change the contract.
- **Packet replay vs. client physics.** The builder replays an exact packet stream. The bot client must be able to turn off its own physics and movement during replay (§9).

### 3.10 Resources beyond items: entities, facilities, sites and presence

Until v0.4 the economy only knew items: things that stack, sit in chests and ride in a bot's inventory. Big tech builds like the orbital strike cannon show that items are the easy case. The planner and logistics need five kinds of resource, each with its own rules:

| kind | examples | what makes it different |
|---|---|---|
| **Items** | stone, TNT, slime blocks, music discs | fungible, storable, carried in inventories and shulkers |
| **Entities** | a skeleton for a disc farm, breezes installed in a machine, villagers, livestock | can't go in a chest; have AI and physics; despawn; must be lured, caught, contained and moved, often across a long distance |
| **Facilities** | mob farms, trial spawner setups, portals, storage, the cannon itself | built once and then *produce a rate*; only run while a player is near; have a build cost and precursors |
| **Sites** | a farm location, a hidden cannon site, a stronghold | chosen by search under constraints; reserved with an internal region claim (C10), plus a permit if player-made blocks are in the way; often need digging out first |
| **Presence** | a bot near a farm, beside a convoy, at a trial spawner | a constraint on *where bots are, over time*, not a pool of bot-hours. Without op there's no `/forceload`, so a bot nearby is what keeps chunks, farms and mobs running, unless a machine has its own survival chunk loader |

The planner treats these uniformly: every goal expands into a **production graph** whose nodes are recipes (items in, items out), facilities (build cost, then a rate per bot-hour of presence), entity acquisitions (a source, a capture method, a route) and site searches. For each needed input it chooses between producing it by hand and building a facility for it, by comparing costs over the goal's time horizon.

#### Entities as a resource

Entity resources get their own ledger alongside the item ledger, and their own skills and jobs:

- **Ledger.** Keyed by **entity UUID**, which the server assigns and every node sees the same. Each entry has its type, where it is held (pen, cell, boat, minecart), its owner operation, whether it is safe from despawning, what keeps its chunk loaded, and **when it was last seen**. Counts are derived from entries and carry their age, so "256, last seen 14 h ago" is never shown as "256". Hostile mobs despawn when no player is near (instantly beyond 128 blocks), so every held hostile mob must be persistent (a vehicle, a name tag, or what the machine's own design provides, per the mechanics table, §9.1) or have a bot close by at all times.
- **Cargo.** Ledger entities are tagged as cargo. Reflexes and combat never target cargo, even when it attacks; bots shield and back off instead, and keep out of its line of sight where they can. This matters for mobs like breezes, which attack the bots carrying them.
- **Sources.** Natural spawning (needs a site with the right biome, light and space), spawners, trial spawners, and breeding. Trial spawners cap throughput: a few mobs per wave, then a long cooldown, with wave size rising with players in range. Throughput is quoted from the mechanics table and measured rates, and the planner runs several sources in parallel when one is too slow.
- **Capture methods** (skill B7): luring by aggro (a bot kites a skeleton into a trap, keeping just out of reach), luring with held items (passive mobs), leads where the mob allows them, boats and minecarts, water streams, and portals.
- **Transport** is a **convoy**, an operation of its own: containment, a route, an escort, and a bot in range of every vehicle at all times (checked by the squad leader; the train stops when it fails). A boat carries 2 mobs and a minecart 1, so capacity planning matters. The convoy holds a moving claim (§3.7). Nether portals cut overland distance by 8× and mobs pass through them, so long hauls go through the nether, with a bot crossing first and the portal link probed before cargo crosses.
- **Handover** between owners is one step, by UUID: both sides see the same UUIDs, and the new owner's claim exists before the old one is released.
- **Losses are expected.** A convoy reports survivors; the planner re-orders the shortfall rather than treating a loss as a failure.

Moving hundreds of breezes is therefore not a build step. It is a pipeline of operations: capture at one or more chambers, convoy, then installation at the site (`WORKED_EXAMPLE_OSC.md`).

#### Facilities as capital

A facility is a long-lived object with a state (`planned`, `building`, `running`, `degraded`, `destroyed`), a rate, inputs (including bot presence) and a location. Production (C4) runs facilities; HQ sees their rates in the digest. Choosing to build a farm is a capital decision: it costs materials, entities and bot-hours up front, and pays back over time.

#### Presence

Presence is scheduled as **location constraints over time**. A bot meeting one can do other work nearby (craft, sort, guard), since being there is the whole requirement. Descriptors state presence in a fixed form:

```
presence:
  - at:      marker | facility anchor | region
    band:    entity_ticking | loaded | within_32 | within_128 | 24_to_128
    count:   1                      # or a rate that depends on the count
    phase:   always | while_running | during_<phase> | visit_after_restart
    outside: region                 # optional standoff, e.g. a keep_out
```

Things the scheduler has to respect:

- **Feasibility at quote time.** The server reports its simulation distance at login, so the program service checks that each band can be met at all (for example "load the target, but stay outside its keep-out" may be impossible at a small simulation distance).
- **Bots are players.** They make hostile mobs spawn near themselves and share mob caps, so presence in a dark space needs that space spawn-proofed.
- **Idle timeouts.** If the server kicks idle players, presence jobs must act periodically.
- **Restarts drop all presence at once** (§3.14).

#### Sites

Site selection (C10) is a search over the coarse map and scouting reports, scored on constraints: biome and spawn conditions, depth and space, distance to supply and to targets, **concealment**, claims and territory. Concealment budgets come from the content pack, not from Null, and count **bot traffic and dwell time** as well as block entities and entities: a "hidden" site that bots visit daily is not hidden. Scouts verify the top candidates before a claim. Excavation then runs as a site-prep operation (C2), with the spoil feeding the item ledger.

### 3.11 Programs: operations that depend on each other

Some goals need operations in sequence, not in parallel: a farm must run before its output exists, and a site must be dug before anything is built on it. HQ runs these as a **program**: a DAG of operations where each edge is a resource handover (items, entities, a facility or a site).

- **One production-graph planner.** The **program service** (K7) expands descriptors and recipes into the production graph, decides which facilities and sources supply each input and how much, and cuts the program into operations by place (§3.3). It owns the quote. Operations' HTNs only decide how to meet their quotas.
- HQ tracks the program's **critical path** and moves bots onto it.
- Before starting a large program, the program service produces a **quote**: the full production graph (nothing hidden behind "…": every input expanded to raw sources, including tool wear and spoil haulage), bot-hours, entities, presence, feasibility checks and lead time. **Each number is marked measured or claimed**: descriptor rates are claimed until measured once in the harness or live. The commander approves it (`quote(goal)`, then `approve`). Whether a big machine is worth it on this server is the commander's call, informed by the quote.
- **Handover edges are gates.** Before a handover (site to build, cargo to installation), the receiving side re-checks its preconditions against current belief: claims still held, site not discovered, counts fresh. A failed gate escalates instead of proceeding on stale belief.
- **Cancel is defined.** Cancelling a program unwinds in order: stop new work, park or release cargo, return stock to HQ, release claims. What is kept (facilities, stock) is listed in the cancel report.
- **Expected losses don't escalate.** Losses within the measured rate only move the ETA. Only a critical-path slip beyond a threshold, or a resource below a minimum, reaches the commander.
- The digest shows one row per program (stage, critical-path item, ETA), not one per operation inside it, so the commander's context stays bounded.

### 3.12 Framework, content and operator annotations

Null is a framework. It knows about generic things: items, entities, facilities, sites, presence, jobs, claims and operations. It knows nothing about any particular machine. A cannon, a disc farm or a breeze farm is **content**, and what the operator does with a finished machine is **the operator's business**. Three layers:

| layer | what's in it | who writes it |
|---|---|---|
| **Framework (Null)** | the hierarchy, planner, ledgers, skills, builder, map, command API | the project |
| **Content packs** | schematics plus a data descriptor per facility in a small typed schema (C11): outputs and rates; inputs and **how each is used** (consumed, installed in the machine, feeding a facility); per-container contents; presence (§3.10); site and concealment constraints; **arming steps**; precursors. Rates are claimed until measured | operators and the community; data, not code, no scripting |
| **Operator annotations** | markers and regions the operator puts on the map, with a label, notes and optional fields | the operator, at runtime |

What follows from this:

- **Building is generic.** `Build(schematic, site constraints)` with a content-pack descriptor is enough for the planner to work out the precursor chain (§3.13). Nothing in Null is cannon-specific, and there is no "get or build cannon" call.
- **Finished things are tracked by annotation.** Once something is built, the operator marks it on the map ("cannon, covers this area, ~30 min to charge"). Markers show on the map and in the digest, and goals can refer to them (`Defend(marker)`, `Hold(region)`). Their labels and notes are free text that Null doesn't interpret. An annotation can also carry one effect from a small closed set that Null does honour: `keep_out` (pathing and claims avoid it), `defend`, `rally`, `presence_target`. That set is generic and grows only by changing the framework.
- **Operating a machine is the operator's job.** Aiming and firing a cannon is done by a person, or by the operator's own tooling outside Null. The framework offers generic primitives that tooling can use, such as pressing a button or delivering items into a container at a marked block, bundled as an **errand** (§4.1), never machine-specific commands. Null does read a machine's presence needs, because it must quote and schedule them; the firing logic stays outside.
- **Arming is a policy class.** The descriptor marks steps that make a machine live: installing payload or actuator entities, placing TNT, closing the final break in a circuit. The builder leaves these until last, behind a physical break in the circuit that is closed as the final step, and each one needs approval like an attack on a player.
- **Facilities (§3.10) stay generic.** The framework tracks a facility's state, rate and presence needs because that's what production planning needs. It reads them from the descriptor, whatever the facility is.

### 3.13 Worked example: the Orbital Skybender

The cannon is a stress test of the generic pieces, not a feature. The current design, the **Orbital Skybender**, holds 256 live breezes in a 1 × 2 cell whose wind charges deliver the payload; its TNT comes from dupers, and it has two built-in chunk loaders. The operator supplies the schematic and the descriptors.

```
Commander: Build(skybender pack, site: within range of region A, concealed)
  Program service: quote → approved → program (operations cut by place):
    Site          survey, dig, stock, build, then arm (install 256 breezes)
    Chamber(s)    survey and capture breezes; one per chamber run in parallel
    Convoy        moving claim: chamber → nether → site, hands over by UUID
    Disc farm     its own site; lured skeleton
    Supply        quotas on existing farms and mines (slime, quartz, iron, …)
  Done: the operator marks the cannon on the map and operates it.
```

The builder only sees the Site operation's build step, with a dug site and staged materials. Everything that "doesn't show up in the litematic" is the rest, and the breeze pipeline is the critical path.

The full walkthrough, including the descriptors, the quote, the breeze pipeline and a firing test in the harness, is in `WORKED_EXAMPLE_OSC.md`. Failure modes of the breeze pipeline are in `BREEZE_PIPELINE_FAILURES.md`, and the review that drove v0.7 is in `REVIEW_OSC_CONTROL_METHOD.md`.

### 3.14 Cargo, preemption, restarts and time

Rules that apply to any job holding things that can't be recreated on demand:

- **Preemption floor and park points.** A cargo job (convoy, keeper, staging) has a minimum crew that can't be preempted. Preempting anything above the floor is normal. Preempting the job itself means moving to the next **park point** (an enclosed holding spot on the route), handing the cargo to a keeper, then releasing.
- **Checkpoints.** Cargo jobs save their state (position, UUIDs, crew) and resume from it after a failure instead of re-planning.
- **Restart protocol.** After a server restart, bots reconnect in priority order: cargo keepers and escorts first, then **visit-after-restart** jobs at markers whose descriptors need them (for example to check a machine's own chunk loaders came back), then everything else. Convoys resume from their park state.
- **Staleness.** Anything that depends on a count or state (a firing errand, a handover gate) checks its age first and re-verifies with a visit if it's too old.
- **Game time and wall time.** Durations of things the game does (charge times, spawner cooldowns, farm rates, settle times, job budgets for in-game work) are counted in **game ticks**. Network timeouts, lease heartbeats and node health are in **wall time**. Every contract field says which. The server's tick rate is read from the server, so lag and `/tick rate` changes slow game durations without breaking leases.
- **Hygiene policies.** "Don't bring X into Y" rules are ordinary policies, for example no bot with Bad Omen takes a trial chamber job.

---

## 4. The commander interface

### 4.1 One API for people and LLMs

The commander talks only to HQ, through a small typed command set. A person uses it through a CLI or console; an LLM uses it as tools. Same validation, same permissions, same audit log.

**Read:**
- `status()`: the fixed-size digest (§4.3).
- `inspect(target)`: zoom into one operation, squad, region or bot. Paged, with the same size cap as `status()`.
- `query(question)`: structured lookups (stock of an item, known structures, threats near a region). Results are aggregated and capped.
- `explain(goal_or_operation)`: why it is in its current state, and what it waits on.

**Write:**
- `set_goal(goal, priority, constraints)` / `cancel_goal` / `reprioritise`.
- `quote(goal)`: the production graph, cost and lead time before committing (§3.11).
- `annotate(marker or region, label, notes, effect?)` / `remove_annotation`: the operator's own map marks, optionally with one typed effect (§3.12).
- `errand(steps at markers, claims to hold, priority)`: a bundle of generic jobs (deliver into a container, press, hold presence) for the operator's tooling. It is **prepared** first (bots allocated and in position, claims taken and held until release) and then run step by step with `trigger(errand, step)`, which goes straight to the squad holding it with no planning in between. Everything is audited like any goal.
- `allocate(operation, headcount or share)`: move bots between operations.
- `set_policy(scope, ...)`: standing orders, swarm-wide or per operation.
- `approve(plan)` / `reject(plan, reason)`.
- `halt(scope)` / `recall(scope)`: the big red buttons, for one operation or everything. Never gated.

Commands target **goals, operations, regions and selectors** ("all idle miners in the north"), never lists of bots. Per-bot control exists only as a debugging tool for a person.

### 4.2 Policies and autonomy

Standing orders say what each level may do without asking:

- always allowed: eat, sleep, flee, defend self, recover own items;
- allowed by policy: defend allies, retaliate, move bots within an operation, spend diamonds, enter the nether; hygiene rules (§3.14);
- needs approval: attack a player, **arm a machine** (§3.12), break a **player-made** block outside a permit area, start a build over N blocks, spend scarce items, create an operation.

*Changed by AD-K2.2 (Allen, 2026-10-15): approval classes are replaced by permission checks on low-level skills, set once by a person (K2 §4.4). Arming is not a separate action: it is a sequence of ordinary clicks (buttons, then a lever), so no check can guard it. It stays last in the build order.*

**Breaking blocks** (spec audit #8; Allen, 2026-10-13). Natural blocks can be broken anywhere without a permit, so trees, mining and nether travel never wait on a ticket. Before every break outside a permit area, gathering applies the operator's ownership rules (B4 §4.8: crafted blocks, unnatural combinations, torches, signs, known bases), skips anything they mark as player-made, and reports it; breaking such a block needs approval. Permits are granted only by a person (`grant_permit`, or a map annotation); the LLM commander can request one, which arrives as an approval ticket (K2 §4.4).

Policies are what let the swarm run for hours without the commander, and they are where most "LLM decisions" actually turn into rules.

### 4.3 Keeping the commander's context bounded

The commander's view must fit in a fixed token budget (say 4–8k tokens for the digest) whether there are 20 bots or 2,000. How:

- **It sees operations, not bots.** The digest has one row per operation (goal, headcount, progress, stock, health, top issue), so its size grows with the number of operations (~10–30), not bots.
- **Exceptions are ranked, merged and capped.** At most K (say 10) escalations, each one merged across all the bots it affects, with what was already tried.
- **Aggregates, not inventories.** Swarm-wide stock is the top items and the items goals are short of; threats are per region.
- **Zooming is explicit and paged.** `inspect()` lets the commander drill into one thing at a time, with the same size cap. It never gets a dump of everything.
- **Only deltas between wakes.** Each wake gets what changed since the last one, plus a short standing summary.
- **Memory outside the context.** The LLM's notes, its plans and the decision log are kept by the gateway and fetched when needed, so the conversation can be cut down at any time without losing the plan.

Rough budget at 2,000 bots: 30 operations × ~60 tokens + 10 escalations × ~100 tokens + standing summary ≈ 3–4k tokens per wake. The same at 20 bots, just with fewer rows.

**Optional: LLMs at the operation level.** If an operation's work is fuzzy (diplomacy, an unusual build, a raid), it can have its own LLM "officer" with the same bounded interface, scoped to that operation. Each officer sees ~10 squads, never 2,000 bots. The commander stays above them all. This is optional; the default is deterministic operations.

### 4.4 When the LLM commander runs

The LLM is woken, not polled every tick:

- an **escalation** HQ can't resolve by policy;
- a **milestone** (a goal or operation done);
- a **heartbeat** (every few minutes) to review the strategy;
- a person talking to it.

It answers with tool calls. Latency of seconds is fine at this level. Cost grows with escalations, not with bot count, and policies keep escalations rare.

A person can sit above the LLM: the LLM proposes, the person approves, or the person takes over. Switching is just a matter of who holds the command API.

---

## 5. Bot agent internals

Each bot agent is a small runtime:

- **connection:** the protocol client, physics, inventory, chunk cache;
- **reflex layer:** priority-ordered behaviours that can override a running skill for a tick (survival first). **Reflexes can be turned off** (see below);
- **skill executor:** runs one job at a time as a skill; reports progress; handles preemption by stopping at a safe point;
- **local perception:** turns what the client sees into observation events (throttled and deduplicated);
- **local fallback:** if the squad or operation link drops, finish the current safe step, then hold position defensively.

**Turning reflexes off.** Every reflex can be turned off, with no exceptions (Allen). The detailed rules are in `CONVENTIONS.md` §4, which this summarises:

- **Registry.** Each reflex is registered by the skill that owns it, with an id, a group (`movement`, `survival`, `combat`, `mob`), its trigger, a priority and a default.
- **Scopes.** Settings exist at swarm, operation, squad, job and bot scope (bot scope is for a person debugging). The most specific setting wins, and each scope can switch one reflex, a group, or all of them, optionally with an expiry. The commander sets swarm and operation scope through `set_policy`; operations and squad leaders set them through job contracts and tactics.
- **Jobs.** A job contract lists `reflexes_off` (builder replay: all of them) and `reflexes_assumed_on`. The squad leader applies the first for the job's duration and restores the settings afterwards. If policy has turned off a reflex the job assumes, the job comes back `blocked` rather than running silently without it.
- **Off still notices.** The trigger is still evaluated, and a merged, rate-limited `reflex_suppressed` event goes to the skill and squad leader, which decide what to do. Skills never rely on reflexes to stay correct.
- **Visible and audited.** Every change is logged with who, scope and why. The digest notes any scope with a survival reflex off. Turning survival reflexes off swarm-wide asks for confirmation, but is never refused.
- **Not reflexes:** cargo exclusion (a targeting filter, §3.10), replay mode for client physics, halt and recall, and the local link-loss fallback.

**How reflexes and skills run** (AD-F5.2, §9.1). Per bot, every tick, one host inside the node's game tick runs the pipeline in F1 §4.5: evaluate reflex triggers, resolve switches, pick the winner, apply the cargo filter, otherwise step the skill. Reflexes and skills are plain Rust against Null's own bot view and command types, not Bevy systems. A reflex declares which inputs its trigger watches, so unchanged triggers aren't recomputed. Switch state is a cached per-bot bitset, recomputed only when a setting at an enclosing scope changes. Commands are applied as direct writes to the client's input state, so a reflex acts in the same tick it fires (verified in the spike).

Skills are versioned code shared by all bots. A bot's "role" (miner, builder, guard, carrier) is just which jobs the scheduler tends to give it, not a different program.

---

## 6. The three goals mapped onto the architecture

| goal | dominant concepts | main components |
|---|---|---|
| Survive and beat the game | logistics, tech-tree planning, failure recovery | planner, logistics, skill library (mining, crafting, combat vs. mobs, nether, End fight) |
| Large decorative and redstone builds | claims on regions, logistics at scale, the builder's guarantees | builder service, logistics, scheduler (many bots feeding few precise builders) |
| Faction on SMP | belief under adversaries, secrecy, tactics | world model (threats), squad controller, policies, commander approval |

A useful observation: for big builds, most bots are **suppliers**, not builders. A few bots place blocks precisely; the rest mine, farm, smelt and carry. So logistics throughput, not placement speed, will set the build rate.

---

## 7. Suggested build order

1. **Skeleton:** all four levels (HQ, one operation, one squad, two bots) in one process, but talking through the real message types. A CLI commander. Goal: `Obtain(oak_log, 64)` split across both bots with no collisions.
2. **Survival core:** skill library for gather, craft, smelt, eat, sleep, store; logistics ledger; recovery from death and disconnect.
3. **Planner:** tech-tree expansion to reach iron and diamonds unattended.
4. **Distribution:** split the levels across processes and hosts on the bus; several operations; node failure and account reassignment. Load-test with a few hundred bots on a private server before going further.
5. **LLM commander:** the same command API as tools, a reporter digest, escalation triggers.
6. **Builder integration:** decorative builds first (no redstone), with logistics staging; then redstone via the builder's certified mode.
7. **Nether and End:** the full `KillDragon` goal.
8. **Faction:** threat model, squads, policies, opsec. Last, because it needs everything else to be stable.

---

## 8. Risks worth tracking early

- **Movement checks and server rules.** Bots moving slightly wrong get kicked by vanilla's own movement checks, and flagged by anti-cheat on servers that run it. Azalea's physics is a close port of vanilla; test against a vanilla 1.21.11 server early. The AD-F5.2 spike's live run did (2026-10-14, `reference/SPIKE_F5_BOUNDARY.md` §6): bots logged in, fell, walked, opened a chest and placed blocks with no kicks or movement warnings from vanilla. Servers with anti-cheat plugins are still untested.
- **Account count.** Each bot needs its own licensed account (on online-mode servers). At 2,000 bots this is the biggest cost.
- **Servers at that scale.** Many servers cap players and connections per IP, and one vanilla server won't hold 2,000 players in one world. Big deployments spread over several vanilla servers under one HQ (§2), with bots spread over enough IPs. This decides where the swarm can play at all.
- **Hierarchy tuning.** Span of control, summary size and escalation thresholds need measuring under load. Too many escalations will swamp HQ and the commander.
- **World model staleness.** Plans built on old beliefs fail at arrival. Measure how often.
- **Claim leaks.** A crashed job that never releases its region stalls everything. Every claim must expire.
- **Builder fit.** The builder prototype runs in creative (§3.9). Design its interface around survival needs (items, break time, drops, chunk loading) now.
- **Azalea is a one-maintainer project** with frequent Bevy upgrades. Null carries its own fork (§9) and must assume it may have to keep its own fixes.

---

## 9. Stack

**Bot client: Azalea (Rust).** Chosen for the final version. Its swarm mode runs many bots in one process and shares world storage between them, which cuts memory sharply. It is built on the Bevy ECS: bots are entities.

**Null's Azalea fork** (AD-F5.1, decided by Allen, 2026-10-13). A specialised build with the latest Azalea features that speaks 1.21.11 natively, not through a translation layer like ViaVersion. Recipe: Azalea's `1.21.11` branch plus the version-neutral upstream commits cherry-picked onto it, with the Rust nightly pinned per version (`reference/AZALEA_REALITY_CHECK.md` §2, rebuilt in `reference/SPIKE_F5_BOUNDARY.md`). The fork carries Null's own patches (byte counters, game-time and block-entity capture, per-bot replay switch, a swap-click fix, and so on) and merges upstream continuously except the version-bump commits. Prefer Azalea's built-in features over rewriting them (Allen, 2026-10-13): its pathfinder is reused behind B1's interface, with the default search pool and a shared section cache as the first fork change (AD-B1.1, decided 2026-10-15), and its shared chunk storage sits under Null's belief store (AD-F2.2, decided 2026-10-15).

How that shapes the design:

- **Swarm nodes.** Each node is one Azalea swarm process hosting ~50–300 bots and their squad leaders (§3.8). Shared world memory only works within a process, which is why bots are placed on nodes by region.
- **Squad leaders run inside the node**, so squad-to-bot traffic never crosses the network. Whether squad tactics are decision code over a `SquadView` or mechanism code is an implementation choice, not architecture (AD-F5.2, decided 2026-10-14; F5 §9).
- **Everything above the squad talks over the bus** (NATS), using serialisable message types. In a small deployment HQ and operations can still run in the node process, using the same types.
- **Skills and reflexes are decision code, not Bevy plugins** (AD-F5.2). They are plain Rust against Null's `BotView` (read) and `BotCommands` (act), called once per bot per tick by one host system inside the game tick, which borrows Azalea's components with no copy and no extra tick (§5). Turning a reflex off is a switch the host reads, not a code path.
- **The commander gateway** exposes the command API (HTTP or WebSocket) to the CLI and the LLM adapter.
- **Builder packet replay.** Azalea lets a bot send raw packets and drive its own movement, so the builder can turn off normal physics and pathing during replay. A per-bot replay switch is a fork patch.
- **Cost:** a smaller ecosystem than Mineflayer, so more skills are written in-house (PvP, auto-eat, armour, farming). Each Azalea release supports one Minecraft version, so the version boundary in §9.1 matters. Azalea needs nightly Rust; decision crates don't.

**Prototyping:** Mineflayer or anything quick is fine for throwaway experiments, but skill code should be written against Azalea from the start, since it won't port.

**Storage:** each operation keeps its shard and event log locally (SQLite is fine), with logs shipped to a VPS for recovery. HQ's state and the coarse map live on a VPS (Postgres).

**LLM:** any tool-calling model behind the command API; the orchestrator does not depend on which.

### 9.1 Version abstraction

**Target:** Java Edition 1.21.11. **Requirement:** the code must port to other versions without a rewrite.

What changes between versions, and where each one lives:

| what changes | examples | where it lives |
|---|---|---|
| protocol and client | packet formats, login, chunk format | Azalea itself; one build per version |
| game data | block states, shapes, items, recipes, tool speeds, loot, mob stats | data files generated per version |
| mechanics | combat timings, the mace, redstone quirks, crafter and copper bulb existing, physics constants | a versioned mechanics table |
| server behaviour | movement checks, anti-cheat tolerances | test harness profiles per version |

The rules:

- **Porting touches the named mechanism crates, never the decision crates** (AD-F5.2, decided by Allen on 2026-10-14; the simulator spike and the live join check passed). Two kinds of code:
  - **Decision crates** (skills, reflex triggers and actions, F1 §4.5 arbitration and switch resolution, the cargo filter, squad tactics, and everything above the node) depend on no `bevy*` or `azalea*` crate. They see the game only through Null's `BotView` and `BotCommands`, and build on stable Rust.
  - **Mechanism crates**, named on an allowlist, import Azalea and Bevy directly: `mc-adapter` (the host, view construction, command application) and the binaries that link it. Raw capabilities Azalea lacks (face placement with prediction, item use, attackers, menus, pathfinder moves, the replay switch) are fork patches; `mc-adapter` only maps Azalea's components onto Null's stable interface (Allen, 2026-10-17; F5 §4.1). Each has a generated list of the Azalea types it uses, and every fork bump starts with an upgrade-impact diff of those lists.
  - A `cargo-deny` allowlist enforces the split. The spike built one unchanged decision crate against three Azalea builds (Bevy 0.17, 0.18 and 0.19); only the adapter changed.
  - Commands become direct component writes or queued components, never Azalea's `Update`-schedule messages, which land a tick late.
- **Game data is data, not code.** Block states, collision and outline shapes and block behaviour constants come from Azalea's generated data; the vanilla server's data generator (`--reports`) supplies only what Azalea lacks (recipes, loot, support rules, mob data). Both are stored per version. Code refers to blocks by name (`minecraft:observer`), never by numeric ID.
- **Mechanics are looked up, not hard-coded.** Anything that differs by version goes in a mechanics table keyed by version: the attack cooldown, trial spawner wave sizes, cooldowns and mob tracking, despawn and persistence rules, which blocks are memory cells for the builder (§2 of `ONLINE_BUILDER.md`), and whether a block or item exists. Skills ask about features ("does the crafter exist?"), never about version numbers.
- **One binary per version.** Two Azalea versions can't share a process, so each supported version is its own swarm build with its own fork, pinned nightly, game data, mechanics table and harness run. The commander API and the job and event types are the same across versions, so the orchestrator and commander don't care which one is running.
- **The deployment upgrades together** (Allen, 2026-10-14). Normally every server and node runs one version and the whole deployment ports at once. Two versions run side by side only when a wanted server forces it.
- **Every version runs the same test scenarios** (F3). Adding a version means generating its data, filling in its mechanics table, fixing the adapter, and getting the test scenarios green.
- **The builder's simulator matches the server exactly.** `ONLINE_BUILDER.md` already requires the same server jar and version as live, so its certificates are per version and never carry across.

---

## 10. Components

The project splits into these components. Each has one owner, a contract with the others, and can be built and tested on its own. Each one can be its own thread or work stream.

### Foundation

| # | component | what it owns |
|---|---|---|
| F1 | **Swarm runtime** | swarm nodes, accounts and login, reconnects, per-bot lifecycle, placing bots on nodes, moving accounts when a node dies |
| F6 | **Bus and deployment** | NATS cluster and leaf nodes, service discovery, deploying to home machines and VPSes, load testing |
| F5 | **Version layer** | the Azalea fork and its patches, the mechanism-crate boundary and `mc-adapter`, per-version game data, the mechanics table, server profiles (§9.1) |
| F2 | **World model** | the belief store beside Azalea's shared chunk storage: last-seen times, POIs, chest contents, entity tracking, hazards, persistence |
| F3 | **Test harness** | a private server with scripted scenarios ("bot in a cave with a creeper", "chest emptied mid-job"), replayable runs, and metrics, plus Azalea's packet simulator for per-command tests. Every other component is tested here, and a person signs off before it runs on a live server (the live gate, F3 §4.9) |
| F4 | **Ops and telemetry** | logs, the event log, dashboards, alerts when bots die or get kicked, anti-cheat flag tracking |

### Bot skills (run on each bot)

| # | component | what it owns |
|---|---|---|
| B1 | **Movement** | local pathfinding, long-range travel (roads, nether travel, boats, elytra later), getting unstuck, safe falls, swimming |
| B2 | **Survival upkeep** | eating, health, sleeping, armour and tool swapping, repair (mending, anvils), escaping fire, lava and drowning |
| B3 | **Inventory and crafting** | slot management, crafting, smelting, enchanting, brewing, shulker handling, chest transactions |
| B4 | **Gathering** | mining (strip, branch, ore veins, ancient debris), tree felling, crop and animal farming, fishing, mob drops |
| B5 | **Combat** | PvE (each mob type, including the blaze, wither and dragon fights), PvP (crit timing, shields, totems, crystal and anchor PvP if allowed), bow, pearls, retreat |
| B6 | **Placement and breaking** | the low-level actions the builder and gathering use: reach checks, click choice, break timing with the right tool, collecting drops |
| B7 | **Mob handling** | luring by aggro or held items, kiting into traps, leads, boats and minecarts, name tags, breeding, killing at the right moment |

### Swarm capabilities (multi-bot, run by operations)

| # | component | what it owns |
|---|---|---|
| C1 | **Builder** | schematic in, certified plan out (`ONLINE_BUILDER.md`). Redstone-safe placement, entities, temps |
| C2 | **Site prep and demolition** | clearing and levelling sites, digging out volumes, removing water and lava. On survival this is often as large as the build itself |
| C3 | **Logistics** | stock ledger, reservations, carriers, staging chests, storage layout, sorting, recovering death drops |
| C4 | **Production** | facilities as capital (§3.10): crop and animal farms, smelting arrays, mob farms (iron, gold, XP, discs, wind charges); their rates, the presence they need, and their upkeep |
| C5 | **Exploration** | scouting, finding structures (villages, fortresses, bastions, strongholds via eye triangulation), mapping |
| C6 | **Progression** | the tech-tree plans for beating the game: speedrun-style routes for the nether, stronghold and dragon |
| C7 | **Squads and tactics** | group combat: target choice, formation, escort, raid and defence plays, retreat |
| C8 | **Faction** | territory, defences (walls, traps, alarms), threat intel on other players, opsec, diplomacy and chat with humans |
| C9 | **Entity logistics** | the UUID entity ledger with ages, cargo tagging, containment, persistence, convoys with moving claims, park points and handover, portal routing |
| C10 | **Site selection** | constraint search for sites (spawn conditions, space, range, concealment), scouting checks, handover to excavation |
| C11 | **Content packs** | the typed descriptor schema (outputs, rates measured or claimed, inputs and how each is used, per-container contents, presence, site and concealment constraints, arming steps, precursors), loading and validating packs |

### Control

| # | component | what it owns |
|---|---|---|
| K1 | **HQ** | goal tree, programs and quotes, allocating bots and resources between operations, map annotations, coarse map, digest |
| K5 | **Operation service** | per-operation HTN planner, scheduler, ledger, world model shard, claims, recovery rules |
| K6 | **Squad leader** | job batches for 5–20 bots, local retries, tactics loop |
| K7 | **Program service** | the production graph, choice of sources, quotes with measured or claimed numbers, cutting programs into operations, gates and cancel paths |
| K2 | **Commander API** | the typed command set (§4.1), policies and autonomy levels, permits, audit log, halt and recall |
| K3 | **LLM commander adapter** | digest writer, escalation triggers, tool bindings, prompt and memory for the commander model |
| K4 | **Human interface** | CLI or web console, in-game whisper commands, map view |

### Dependencies and order

Everything depends on F1–F3 and F5; F6 is needed before going past one host. Skills B1–B3 and B6 come next, because every capability uses them. HQ, operations and squads (K1, K5, K6) and logistics (C3) are the spine every capability plugs into. Combat (B5, C7) and faction (C8) can move in parallel with building (C1, C2) once the spine works. The LLM adapter (K3) can start early against a fake orchestrator, since it only touches the command API.

The hardest ones, in my estimate: the builder (C1), logistics at build scale (C3), PvP combat against real players (B5, C7), and staying under anti-cheat while doing all of it.

---

## 11. Changes in v0.8 (2026-10-14)

Folded in from decisions Allen made and from proposals in the component specs, the spec audit, the Azalea reality check and the AD-F5.2 spike. Section numbers are unchanged.

- **Pure vanilla servers** stated as an assumption (header). Server choice is a profile, not architecture (AD-F1.1).
- **Claims, permits and territory** separated (§1.2), and the break rule narrowed: natural blocks need no permit; only player-made blocks outside a permit need approval (§4.2; B4 §4.8, K2 §4.4, spec audit #8).
- **One HQ per deployment, spanning servers**, with a `server` field in every location key (§2; AD-K1.1).
- **Capacity**: about 1,000 bots per laptop as the target, CPU and bandwidth as limits, density as a scheduling constraint (§2, §3.8; F1 §4.1, §4.11; spec audit #2).
- **Self-fencing** on partition, and cargo keepers held rather than re-placed (§3.8; F1 §4.2, §4.7; spec audit #5).
- **Belief store** separate from Azalea's chunk storage; AD-F2.1 (shard key and where belief lives) decided 2026-10-15: fixed tiles, operation service inside the node (§3.7).
- **Reflexes and skills as decision code run by one host** in the game tick, replacing "skills and reflexes are Bevy plugins" (§5, §9; AD-F5.2, spike in `reference/SPIKE_F5_BOUNDARY.md`). The version boundary restated as decision crates versus named mechanism crates (§9.1).
- **Null's Azalea fork** described (AD-F5.1), with Azalea's pathfinder and chunk storage reused (§9).
- **Game data source** corrected: Azalea's generated data first, the vanilla data generator for the rest (§9.1).
- **Version policy**: the deployment upgrades together; two versions only when forced (§9.1).
- **Live gate**: a person signs off before a component runs on a live server (§10 F3; F3 §4.9).
- **Risks** updated: vanilla movement checks, several vanilla servers instead of Folia or Paper, Azalea's bus factor (§8).
- The reflex switch contract (`CONVENTIONS.md` §4) was already summarised in v0.7 §5 and is unchanged.

### Adopted after v0.8

- **Interrupt contract** (AD-K5.3, Allen, 2026-10-14; proposed the same day as "the surprise contract"): detect, contain, price, then escalate or only record; fixes from a cost-ranked batch review with token budgets. Changes §1.3 and §1.7, CONVENTIONS §3.1 (the `interrupted` outcome), F2 §4.7, F3 §4.3, F4 §4.7–4.8, B1 §4.3, B6 §5, C8 §4.4, K1 §4.10, K3 §4.2 and §4.6, K5 §4.6–4.7 and K7 §4.2. See [INTERRUPTS.md](INTERRUPTS.md).
- **Generalization** (AD-K5.2, Allen, 2026-10-15; proposed 2026-10-14 as "the generalization floor"): facts as data from the jar, observation and the LLM prior; skill code only for cost-ranked important parts; goals to state and check tasks; the LLM as main planner for steps with no model, in validated and quoted stretches. Changes §1.6, §3.6, K5 §4.1, B5 §4.2–4.3, F5 §4.3 and C6. See [GENERALIZATION.md](proposals/GENERALIZATION.md) §0.
- **Commander seat** (AD-K2.2, Allen, 2026-10-15): the LLM is in full control and the person holds the kill switch (halt and recall). No shared seat. Boundaries are permission checks on low-level skills, set once by the person or disabled; by default no attacking a player first and no breaking someone else's build (judged by comparing belief with the current world). Changes K2 §4.4 and §4.10, B4 §4.8, B5 §4.7 and INTERRUPTS §5.
- **No officers in v1** (AD-K5.1, Allen, 2026-10-15): the commander is the only LLM (§4.3 allows officers later).
- **Builder decisions** (Allen, 2026-10-15), in the world-transformation frame: a builder bot restocks on need (AD-C1.2); parallel work across inert boundaries and any interleaving in printable regions (AD-C1.1); a placement table plus probing (AD-B6.1); one server pool (AD-F3.1).
- **Pathfinder and belief store from the Azalea benchmark** (AD-B1.1, AD-F2.2, Allen, 2026-10-15; `reference/BENCH_AZALEA_2026-10-15.md`): reuse Azalea's pathfinder with the default pool, a shared section cache first; a separate belief store fed by a packet observer that forwards packets to a worker thread; change detection compares decoded block states, not bytes.
- **Combat doctrine ladder** (AD-B5.1, Allen, 2026-10-16): written doctrine first; then black-box tuning of each technique's named parameters; a policy trained from scratch only as a last resort. Every technique is a duel controller behind one observation→action interface. Changes B5 §4.8.
