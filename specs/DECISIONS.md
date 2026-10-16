# Open architecture decisions: both sides

Spec set v1.2, 2026-10-14. Allen asked for both sides of every open architecture decision except the builder ones (AD-C1.1, AD-C1.2, AD-B6.1 and AD-F3.1). Those were with Allen's builder agent; since 2026-10-14 they are still open, to be answered in the world-transformation design ([proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md) §7 item 5). That leaves 16: the original 12, plus AD-F5.2 and AD-K1.1, added from the spec audit, and AD-K5.2 and AD-K5.3, proposed 2026-10-14. AD-K1.1, AD-F5.2, AD-K5.3, AD-K5.2, AD-F2.1, AD-K2.2 and AD-K5.1 are now decided, so 9 were open; AD-F2.2 and AD-B1.1 were then decided from the Azalea benchmark, so 7 are open. Of those, AD-C3.1, AD-C7.1, AD-C8.1, AD-C9.1 and AD-F6.1 are deferred (Allen, 2026-10-15), AD-K2.1 was deferred until its bake-off (Allen, 2026-10-15), which ran 2026-10-14 without separating the options, and AD-B5.1 was decided 2026-10-16, so 6 are open. Each entry gives the strongest case for each option, what evidence would tip it, and where the specs lean now. The lean is a default, not a verdict.

Context used throughout (Allen, 2026-10-13): local testing is per component, not a full 2,000-bot run. Allen estimates one laptop can carry about 1,000 bots at roughly 1 MB of extra memory each, so the limits are CPU and above all bandwidth, not memory. The server is not an architecture question, and Null runs on a specialized Azalea build that speaks 1.21.11 natively.

**Updated 2026-10-13 with evidence** from the Azalea reality check (`reference/AZALEA_REALITY_CHECK.md`) and the spec audit (`history/SPEC_AUDIT.md`). Three facts changed the picture. Azalea's own guide says a swarm handles a few hundred bots by default, and a swarm is one ECS under one lock that has to finish each 50 ms tick, so 1,000 per laptop is a target to benchmark, not a given. Bot-to-bot traffic grows with the square of how many bots work in one area, so density matters as much as headcount. And where belief physically lives relative to the node sets most of the bus traffic. The leans that moved are marked.

| decision | lean | what would change it |
|---|---|---|
| [AD-F2.1](#ad-f21-shard-belief-by-operation-or-by-fixed-map-tiles) shard key | **decided:** fixed tiles; operation service inside the node | (Allen, 2026-10-15) |
| [AD-F2.2](#ad-f22-extend-azaleas-world-storage-or-keep-a-separate-store) world storage | **decided:** separate store fed by a `chan` packet observer, plus a chunk-storage plug-in | (Allen, 2026-10-15; Azalea benchmark) |
| [AD-F6.1](#ad-f61-where-does-the-buss-third-quorum-member-live) third quorum member | **deferred** (Allen, 2026-10-15); lean: defer until several VPSes | an early need for multi-host durability |
| [AD-B1.1](#ad-b11-reuse-azaleas-pathfinder-or-write-our-own) pathfinder | **decided:** reuse Azalea's; default pool; shared section cache first | (Allen, 2026-10-15; Azalea benchmark) |
| [AD-B5.1](#ad-b51-hand-written-combat-doctrine-or-learned-policies) combat | **decided:** written doctrine first; then tune its parameters; train from scratch only as a last resort | (Allen, 2026-10-16) |
| [AD-C3.1](#ad-c31-central-depots-or-storage-per-operation) storage | **deferred** (Allen, 2026-10-15); lean: hybrid | carrier bot-hours measured in the harness |
| [AD-C7.1](#ad-c71-what-coordinates-fights-bigger-than-one-squad) big fights | **deferred** (Allen, 2026-10-15); lean: bigger squads first, spread out (updated) | measured losses from slow cross-squad reaction |
| [AD-C8.1](#ad-c81-should-bots-talk-to-people-in-v1) bot chat | **deferred** (Allen, 2026-10-15); lean: silent in v1 | a target server where silence causes trouble |
| [AD-C9.1](#ad-c91-can-one-convoys-escort-span-two-nodes) convoy escort | **deferred** (Allen, 2026-10-15); lean: one node only | node failures common enough to lose convoys |
| [AD-K2.1](#ad-k21-imperative-commands-or-a-desired-state-manifest) commander API | **open; bake-off run 2026-10-14, no decisive difference**; lean: manifest hybrid | a longer test at spec scale with a concurrent person, or Allen's call |
| [AD-K2.2](#ad-k22-share-the-commander-seat-and-on-what-terms) commander seat | **decided:** the LLM in full control; the person holds halt and recall; boundaries are permission checks set once (no attacking a player first, no breaking others' builds) | (Allen, 2026-10-15) |
| [AD-K5.1](#ad-k51-llm-officers-at-the-operation-level-in-v1) LLM officers | **decided:** none in v1; the commander alone | (Allen, 2026-10-15) |
| [AD-K5.2](#ad-k52-scenario-knowledge-as-data-or-as-method-code) scenario knowledge | **decided:** facts as data from the jar, observation and the LLM prior; code only for cost-ranked important skills; goals to state and check tasks; the LLM plans in stretches behind a validator and quotes ([GENERALIZATION.md](proposals/GENERALIZATION.md) §0) | (Allen, 2026-10-15) |
| [AD-F5.2](#ad-f52-how-does-the-version-boundary-work-when-skills-are-bevy-plugins) version boundary | **decided:** no Bevy or Azalea in decision code; named mechanism crates import them directly; cargo-deny allowlist | (Allen, 2026-10-14; simulator spike and live join check passed) |
| [AD-K1.1](#ad-k11-does-one-hq-cover-one-server-world-or-several) HQ scope | **decided:** one HQ per deployment, spanning servers | (Allen, 2026-10-13) |
| [AD-K5.3](#ad-k53-interrupts-contain-and-price-or-classify-and-resolve) interrupts | **decided:** contain and price; escalate on cost; fix in cost-ranked batches ([INTERRUPTS.md](INTERRUPTS.md)) | (Allen, 2026-10-14) |

---

## AD-F2.1 Shard belief by operation, or by fixed map tiles?

**Decided by Allen, 2026-10-15: fixed tiles**, with the operation service inside the node process (the linked question). The analysis below is kept for the record.

Spec: [F2](F2-world-model.md). ARCHITECTURE v0.7 shards by operation.

**For sharding by operation (v0.7).**
- One writer per region already exists: the operation that holds the claims. Its log, claims and belief live and die together, which makes restart from the event log simple (K5 §4.9).
- Operations are already cut by place (§3.3), so a second partition by tiles adds a mapping layer. An operation spanning parts of four tiles writes four stores and has to coordinate them.
- Retiring an operation is rare. Exporting its belief to a coarse map once at retirement is cheap and keeps the common path simple.

**For fixed tiles.**
- Knowledge is about places and operations are about work, and their lifetimes differ. A mine district's map stays useful long after the operation that surveyed it is gone, and a new operation in the same place should start from it without an import step.
- Convoys, scouts and transit read across many regions as a matter of routine. With tiles, the reader knows which shard to ask from coordinates alone. With operation shards, it first asks who owns this place, and the answer changes over time.
- The shard key is the one thing that can't change later without migrating everything. Tiles never need re-keying; operation boundaries move whenever programs are re-cut.
- 512×512 tiles line up with Minecraft's own region files, which keeps chunk-aligned bookkeeping trivial.

**What would tip it.** How often operations get re-cut, merged or retired in practice, and how much of the read load crosses operation boundaries. Both can be counted in the harness from a recorded program run.

**Lean (before the decision).** Fixed tiles for belief, with the claims registry and the event log staying per operation as v0.7 has them. This keeps v0.7's single-writer rule (the operation holding the claims writes the tile) while making the data outlive the operation.

**Update: where belief lives (spec audit #3).** The shard key alone doesn't settle the traffic. v0.7 puts each shard in the operation service, a separate process, while raw observations must stay off the bus. Chunk contents can't cross from node to service within the low-KB/s budget, and diffing a re-seen chunk against belief needs belief next to the chunk, which is in the node. Two ways out:
- (a) Belief lives in the node, per tile; the operation service keeps claims and the ledger and queries nodes.
- (b) The operation service runs inside the node process.

(b) is simpler and works whenever an operation fits on one node, which larger nodes make common, but it ties an operation's size to one node. (a) works for operations that span nodes, at the cost of queries across processes. Either way only diffs and summaries leave the node. **Updated lean:** (b). An operation of 50–300 bots fits inside one swarm process even at Azalea's default of a few hundred, so running its service in the node keeps one writer per tile (the operation holding the claims) with belief next to the chunk data. Bots from other nodes passing through send diffs only. The cost is that an operation can't outgrow one node; a bigger job becomes two operations, which §3.3's cut-by-place rule already allows. (a) stays the fallback if operations routinely need to span nodes. This has to be settled together with the shard key, before code.

## AD-F2.2 Extend Azalea's world storage, or keep a separate store?

**Decided by Allen, 2026-10-15: a separate store, fed by a packet observer in the `chan` form** (the ECS forwards each packet's `Arc` to a worker thread). The benchmark measured the observer at 1.2 ms per second of ECS time at 100 bots with no change to the tick; 96% of chunk packets are repeats, so the store deduplicates ([reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md) §5). The analysis below is kept for the record.

Spec: [F2](F2-world-model.md).

**For extending Azalea's storage.**
- One source of truth for blocks means no sync bugs between what the bot sees and what the swarm believes.
- Allen already runs a specialized Azalea build, so its internals are not foreign code; changes there are ours to make.
- Hot-path reads (pathing, placement checks) stay in the same structure the client already updates, with no copy.

**For a separate store.**
- Azalea's world is a cache of loaded chunks. It drops chunks when no bot has them loaded, and belief has to outlive that: the map of a district nobody is standing in still matters. So a persistent store is needed either way, and the question is only whether it also holds the loaded part.
- Keeping our changes out of Azalea's world code keeps merging upstream features into the specialized build cheap. Storage is where upstream changes most often.
- Belief carries things Azalea doesn't model: staleness, source, confidence, claims and annotations (F2 §4). Bolting those onto Azalea's chunk storage spreads them through code that isn't ours.
- Duplication costs little. At about 1 MB per extra bot, memory is not the constraint; CPU and bandwidth are.

**What would tip it.** How invasive belief metadata would be inside Azalea's chunk storage, and how often upstream touches that code. A short spike on the specialized build answers both.

**Update: evidence from the Azalea reality check.**
- Azalea's chunks hold only block states, biomes and heightmaps. It throws away server game time, light, block entities (sign text, spawner and vault data) and container progress. None of what belief attaches has a home in Azalea's `Chunk`, and adding per-cell metadata would spread through palette code upstream is still optimising.
- The extend side gained one real point: Azalea now has a pluggable `ChunkStorageTrait` (upstream branch tip, not in the 0.15.1 release), so a persistent chunk store is a plug-in, not a patch.
- Game time, light and block entities can be captured from packet observers with no fork patch. The cost to measure is that packets are cloned on the hot path.

**Updated lean (before the decision).** A separate store for belief, plus a small `ChunkStorageTrait` plug-in so chunk block data outlives every bot's view. Hot-path reads still go straight to Azalea's loaded chunks. This is now close to settled; the remaining spike only measures the observer cost.

## AD-F6.1 Where does the bus's third quorum member live?

Spec: [F6](F6-bus-deployment.md).

**For a third small VPS now.**
- A real three-member quorum means losing any one machine never stalls the bus or loses acknowledged messages.
- A small VPS is cheap and needs no maintenance beyond the other two.
- Setting it up early means multi-host behaviour is tested from the start instead of retrofitted.

**For deferring it.**
- The near term is Allen's laptop and at most one VPS, running component tests. A quorum across one or two machines buys nothing.
- JetStream with one replica is fine for that phase. Each operation's log is local SQLite first and shipped later, so a bus outage delays shipping but loses no history.
- It's a deployment change, not a code change: going from one replica to three is configuration, so nothing built now depends on the choice.

**What would tip it.** Running across two or more always-on hosts, or any test where losing the bus host would lose work.

**Lean.** Defer. Add the third member when the swarm first runs across several always-on hosts.

## AD-B1.1 Reuse Azalea's pathfinder, or write our own?

**Decided by Allen, 2026-10-15: reuse Azalea's pathfinder, keep the default pool, and add a shared section cache first.** The benchmark showed completed searches level off at 22–28 per second whatever the pool size, while more threads slow the tick and drop bots; 99% of section building was repeated work ([reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md) §4). The analysis below is kept for the record.

Spec: [B1](B1-movement.md).

**For reusing Azalea's pathfinder.**
- It exists, it already matches the client's physics, and it's maintained upstream.
- The specialized build can extend it where needed, so "reuse" doesn't mean "accept as is".
- B1's interface hides the pathfinder, so a replacement later touches one module.
- It gets bots moving early, which unblocks every other skill's tests.

**For writing our own.**
- A swarm needs things a single-bot pathfinder doesn't have: cost layers for danger and claims, reservations so bots don't path through each other or through a build site, and paths that break or place blocks.
- CPU is the real limit at about 1,000 bots per host. Bots in one process often path between the same places, and a shared path cache or flow field across them could cut the cost a lot. That's hard to retrofit onto a per-bot search.
- Owning it means its behaviour is tested and understood the same way as the rest of the swarm.

**What would tip it.** A spike: can Azalea's pathfinder take pluggable cost functions and share computation across bots in one process? If yes, extend it. If not, the swarm-aware version is ours.

**Lean (before the decision).** Start with Azalea's behind B1's interface, and plan a swarm-aware shared cache as the first likely replacement once CPU is measured.

**Update: the spike question is answered by the Azalea reality check.**
- Pluggable costs: yes, mostly. A custom move function (`successors_fn`) can drop or re-price edges for claims, keep-out, hazards and cargo lines of sight, and per-bot state can carry a shared claims map. The base cost constants and the passability test aren't pluggable.
- Shared computation across bots: no. Each search builds its own cache, and searches run on a pool that by default gets 25% of cores, at most 4 threads, so hundreds of bots pathing at once would queue. The pool size is a setting in the fork, and a process-wide section cache is a natural fork addition.
- Missing moves: ladders and vines, doors and gates, bridging and pillaring, portals and boats. Moves are an open table, so these are additions, not a rewrite. With mining on (its default) it digs through doors and player builds, so B1 turns mining off outside digging jobs.

**Updated lean (before the decision).** Reuse, now with evidence, matching your preference for Azalea's built-ins. First fork changes: the compute-pool size, then a shared section cache once CPU is measured. Writing our own is only back on the table if the benchmark shows pathing dominating the tick even with those.

**Raised and deferred by Allen, 2026-10-16: replace Azalea's stateful client glue with our own.** The swarm duel found its stall bugs in Azalea's state handling, not in A* (every search under 4 ms): the shared entity tracker (patches 1, 1b, 2), item use (patch 3) and the stop handler that left a bot "calculating" with nothing running (patch 4, [reference/AZALEA_PATCHES.md](reference/AZALEA_PATCHES.md)). Options: (a) a whole client of our own; (b) keep Azalea's protocol, chunks, physics and A*, and own the entity tracker, the nav executor and item use. Lean: (b), when the patch count or a bug that can't be patched makes the case.

## AD-B5.1 Hand-written combat doctrine, or learned policies?

**Decided by Allen, 2026-10-16: written doctrine first; then tune each technique's named parameters by black-box search against the harness adversaries; train a policy from scratch only if tuned doctrine still loses consistently to strong players.** Every technique sits behind the duel-controller interface from day one, so the last step needs no rewrite (B5, AD-B5.1). The analysis below is kept for the record.

Spec: [B5](B5-combat.md).

**For hand-written doctrine.**
- Every decision is readable and auditable, which matters when bots fight real players and someone asks why a bot did something.
- Doctrine is testable in the harness with fixed scenarios and patchable the day a server changes a mechanic.
- It matches ARCHITECTURE §1.6: deterministic code below the commander, judgment above.

**For learned policies.**
- PvP micro (spacing, crit timing, shield baiting, strafing) plateaus with rules. Good players beat rule-based bots once they learn the pattern.
- Self-play is unusually cheap here: hundreds of bots on one laptop against a private server is a training rig most projects never have.
- Inference for a small policy in Rust is cheap per tick.
- Deciding late means rewriting combat, because the skill's structure differs: doctrine is a tree of rules, a learned policy is an observation-to-action loop.

**Middle ground.** Learned micro inside one skill (a duel controller), with doctrine for everything above it: when to fight, target choice, retreat, formation. The seam is the duel controller's interface.

**What would tip it.** Doctrine losing consistently to strong players in harness matches against recorded human play.

**Lean.** Doctrine first with the duel-controller seam built in from day one, so the self-play option stays open without a rewrite. (Unchanged by the reality check, which found Azalea has attack and cooldown only; weapon logic is ours either way.)

## AD-C3.1 Central depots, or storage per operation?

Spec: [C3](C3-logistics.md).

**For storage per operation.**
- Items live where they're used, so carriers travel less.
- No single vault is a raid target that loses everything at once.
- Each ledger lines up with its operation's shard and event log, so restart and audit stay local.

**For central depots.**
- Less fragmentation: one stock of iron instead of five partial ones, and fewer transfers between operations to cover shortfalls.
- One sorting system to build and maintain instead of one per operation.
- One vault to defend well instead of several to defend poorly.
- Audits and stock counts for the commander are one query.

**What would tip it.** Carrier bot-hours under each layout for a realistic program, measured in the harness. That's the real cost, and it's measurable.

**Lean.** Hybrid: per operation for small swarms, a shared depot once several operations share a base.

## AD-C7.1 What coordinates fights bigger than one squad?

Spec: [C7](C7-squads-tactics.md).

**For a battle group leader.**
- A fight between several squads needs a fast loop. The operation decides every 1–2 s and may sit on another host, which is too slow for flanking or focus fire.
- A battle group leader on one node can reuse the squad leader's code, one level up.

**For no new level (bigger squads first).**
- Every level adds message types, failure modes and restart cases. Fights of 50 against 50 are rare.
- Squads can grow to about 50 bots on one node, since a node now holds hundreds or more. One big squad covers most large fights with no new level.
- For truly large battles, the operation's 1–2 s loop is fine for macro decisions (which squad goes where), and each squad handles its own micro.

**What would tip it.** Harness or live fights where losses come from slow reaction between squads, not from squad-level play.

**Lean.** Bigger squads first; add battle groups only if measured fights show the need. (Changed from v1.1's battle-group recommendation after Allen's capacity numbers.)

**Update (spec audit #2, Azalea reality check).** Bot-to-bot traffic grows with the square of how many bots are near each other, roughly 300 B/s per nearby pair by the audit's estimate, so 50 bots packed together cost the node about 0.75 MB/s. That is a cost of the fight's density, not of who coordinates it: a battle group puts the same bots on the same node at the same density. So it doesn't favour either option, but it does mean big squads should fight spread out rather than packed, and density, not headcount, caps a war squad. A 50-bot squad still fits comfortably inside Azalea's default of a few hundred bots per swarm. **Lean unchanged**, with a density cap added.

## AD-C8.1 Should bots talk to people in v1?

Spec: [C8](C8-faction.md).

**For silent bots.**
- No risk of a bot saying something wrong, offensive or exploitable in public chat.
- Reveals less about the swarm: chat is an easy way to fingerprint bots and learn their plans.
- No injection path from other players' chat into an LLM's context.

**For bots that talk.**
- Faction play is social. A silent faction of hundreds of accounts reads as a bot farm and invites admin bans and player backlash faster than one that answers.
- Diplomacy is a lever: truces, trades and warnings are cheaper than fights.
- The risk can be contained: one officer account speaks, through an LLM with a person approving outgoing messages, and the rest stay silent.

**What would tip it.** The target server's culture. Some servers punish silent alts, others ban bots that chat.

**Lean.** Silent in v1, keeping the policy gate so a speaking officer can be enabled per server when faction play starts.

## AD-C9.1 Can one convoy's escort span two nodes?

Spec: [C9](C9-entity-logistics.md).

**For one node only.**
- The formation check runs every tick in one process, with no network between escort and cargo.
- Nodes are large (a few hundred bots per swarm by Azalea's guidance, possibly ~1,000 per host), so any realistic convoy and its escort fit on one.
- Node failure is already handled: park points and checkpoints (§3.8) let a convoy stop safely and resume.

**For allowing two nodes.**
- If the node dies mid-convoy, an escort on a second node can keep guarding the parked cargo instead of everything going dark at once.
- Fewer forced splits for long trains, and fewer awkward hand-offs at portal crossings.

**What would tip it.** Node failure rates in practice. If convoys are being lost to node crashes despite park points, the fault-tolerance case gets stronger.

**Lean.** One node only, with node failure handled by park points and checkpoints.

## AD-K2.1 Imperative commands, or a desired-state manifest?

**Deferred by Allen, 2026-10-15, until the bake-off**, which runs before the LLM commander is built. **The bake-off ran on 2026-10-14 and did not separate the options** ([reference/BAKEOFF_K21_2026-10-14.md](reference/BAKEOFF_K21_2026-10-14.md)): no injection followed, no rejected writes and almost no thrash on either interface; the hybrid's input cost per wake was 20% lower on haiku, about equal on sonnet and 33% higher on opus. Standing orders and damping in both digests did most of what the manifest was meant to do. Its claimed strengths (memory over long runs, safe concurrent edits, caching) were not tested.

Spec: [K2](K2-commander-api.md). The full comparison is in COMMANDER_INTERFACE.md.

**For imperative commands.**
- Every model handles "do X" well; there's nothing to learn.
- Simple to implement and debug, maps 1:1 onto a CLI, and events fit naturally ("X done").
- No reconciler, so no reconciler bugs.

**For the manifest hybrid.**
- An LLM forgets what it asked for between turns. A manifest is the standing record, so it doesn't re-issue or contradict itself.
- It stops thrash: the commander changes the desired state, and HQ reconciles, instead of a stream of start and cancel commands.
- Conditional patches make the shared seat safe: a change applies only if the state is still what the writer saw.
- A stable manifest caches well in the LLM's prompt.

**What would tip it.** The first bake-off didn't. A longer one could: spec scale (about 30 operations, a 6k-token digest), many wakes with context cuts, a concurrent person editing the same goals, harder scenarios, and Allen's rubric (F3 §4.7).

**Lean.** The manifest hybrid, weakly: the bake-off found no cost to it on haiku or sonnet and no benefit at this scale.

## AD-K2.2 Share the commander seat, and on what terms?

**Decided by Allen, 2026-10-15: the LLM is in full control, and the person holds the kill switch** (halt and recall, K2 §4.9). There is no shared seat. Boundaries are permission checks on low-level skills that a person sets once and may disable; the defaults are no attacking a player first and no breaking someone else's build, judged by comparing Null's belief with the current world (K2 §4.4, B4 §4.8, B5 §4.7). The analysis below is kept for the record.

Spec: [K2](K2-commander-api.md).

**For one holder at a time.**
- Clear accountability: whoever holds the seat owns every decision.
- No conflicts between a person and the LLM issuing opposing commands.

**For shared, with each goal owned by one party.**
- The LLM can run the swarm overnight while Allen keeps the goals he cares about (a build, a war) under his own hand.
- Conflicts are prevented by ownership instead of by locking the whole seat.

**For the LLM commanding with a person approving classes of action.**
- The LLM does the work; the person only signs off on risky classes (arming, war, demolishing unknown structures).
- Simplest for the person, but every approval waits on them.

**What would tip it.** How Allen actually wants to work alongside the LLM. This one is preference more than evidence.

**Lean (before the decision).** Shared with ownership per goal, plus approval classes for risky actions whoever owns the goal.

## AD-K5.1 LLM officers at the operation level in v1?

**Decided by Allen, 2026-10-15: no officers in v1.** The commander alone plans the steps that have no model (AD-K5.2), and the officer interface stays identical so officers can be added later. The analysis below is kept for the record.

Spec: [K5](K5-operation-service.md).

**Against officers in v1.**
- Cost: an LLM call per decision for each officer adds up across many operations.
- Each officer is a new path for injected text from the game into a model that can act.
- Nondeterminism lands exactly where hundreds of bots are scheduled, and tests stop being repeatable.

**For optional officers.**
- Some operations (raids, diplomacy, unusual builds) have no good HTN method, and writing one is brittle.
- Officers keep the commander's context bounded: it delegates the fuzzy operation instead of micromanaging it.
- The interface is the same as the commander's, so adding an officer to one operation is cheap, and using it early shows where the interface has gaps.

**What would tip it.** The commander's context filling up with one kind of operation it keeps having to steer by hand.

**Lean (before the decision).** No officers in v1, with the officer interface kept identical to the commander's so they can be added per operation.


## AD-K5.2 Scenario knowledge as data, or as method code?

**Decided by Allen, 2026-10-15: neither as first framed.** Minecraft's scenarios can't be listed, so neither code nor handwritten data per scenario scales. High-level tasks can't be listed either, so HTN methods per goal kind break, as code or as data. The decided shape is in [GENERALIZATION.md](proposals/GENERALIZATION.md) §0:
- Facts are data whose values come from the game jar, from observation, or from the LLM's prior (unconfirmed). A field that needs handwritten values per row is not allowed.
- Skills are code only where the interrupt contract's cost-ranked review shows a need, plus the named boss exceptions. The generic default is the main path.
- Goals (end condition, constraints, milestones) are the language to state and check tasks. There is no general goal-state planner.
- The LLM is the main planner for steps with no model. It plans in stretches. Each stretch is validated and quoted, runs with the existing searches as subroutines, and wakes the LLM on K3 §4.2's triggers. Plans that work are saved as routines with parameters.

This reopens AD-K5.1: whether the planner is the commander alone in v1, or officers per operation. The analysis below is kept for the record.

Spec: [K5](K5-operation-service.md). Proposal: [proposals/GENERALIZATION.md](proposals/GENERALIZATION.md). Raised by Allen, 2026-10-14: there isn't a budget to write skill code for every possible scenario.

**For method and module code (v1.2 as written).**
- Behaviour that matters (a blaze fight, a convoy) is hard physics and tactics, and a dedicated module can be tuned and tested in isolation.
- Deterministic and reviewable: no LLM in the loop for known goal kinds, and tests stay repeatable.
- Nothing new to build: it is how K5 §4.1 and B5 §4.2 are already written.

**For data and a closed verb set.**
- Scenarios are unbounded and the code budget is not. A new mob or structure should cost a row, and the first encounter should be handled by a bounded generic default.
- Most of what makes a fight different is a few facts (attack kind, damage types, counters), which are data already available from game data plus a few fields.
- K7 can only quote what it can expand. Operator data with pre/effects/cost is quotable. Free-form method code is not.
- Measured outcomes can ratchet rows, so repeat tasks get cheaper without code changes.

**What would tip it.**
- The paper test (GENERALIZATION §7): five unseen tasks, pass is at most one new verb across them and no new method.
- Whether the generic `engage` loses too many bots on first encounters compared with a tuned module, measured in F3 on fixed-seed fortresses.
- Whether LLM-composed plans stay inside their budgets (needs the AD-K5.1 officer interface).

**Paper test (2026-10-14, [proposals/GENERALIZATION_PAPER_TEST.md](proposals/GENERALIZATION_PAPER_TEST.md)).** Passed: across the five tasks, 0 new verbs and 0 methods. The cost moved into 9 new interpreted fields (3, 3, 0, 2, 1 per task), 5 planner features and 1 plan construct, all shared framework code. Every mob in B5's module list mapped onto those fields except the dragon and the wither, which stay modules. GENERALIZATION.md was revised: fields count against the budget, the grammar is a closed list, and outcomes go to K7's rate store instead of F5.

**Paper test 2 (2026-10-14, [proposals/GENERALIZATION_PAPER_TEST_2.md](proposals/GENERALIZATION_PAPER_TEST_2.md)).** Five new tasks: 0 verbs and 0 methods. New fields fell from 9 to 4, but total framework items rose from 18 to 24, and each new domain still cost 1 to 2 fields. The number of domains has no limit.

**Paper test 3 (2026-10-15, [proposals/GENERALIZATION_PAPER_TEST_3.md](proposals/GENERALIZATION_PAPER_TEST_3.md)).** The ten tasks written as goals and broken into 74 steps. Existing searches solve 54 steps (42 in a pessimistic recount). The LLM or a coded skill must carry 27% to 43% of all steps, or 39% to 55% without travel, gear and the final craft. The share by bot-hours and by risk is probably higher. Most of that share has no model to search, so the LLM writes the plan and does not take over a search. No validator could confirm any hard step before it ran. The test had no construction task.

**Lean (before the decision).** Data, with per-mob and per-method code allowed only as tuning that a profile row points to, plus the two named boss exceptions. This also reopens AD-K5.1: composition for non-recipe tasks is the job an officer would do. Still open to evidence: a second paper run (do new fields per task keep falling?) and F3's first-encounter losses.

## AD-K5.3 Interrupts: contain and price, or classify and resolve?

**Decided by Allen, 2026-10-14: contain and price.** Adopted as the interrupt contract, [INTERRUPTS.md](INTERRUPTS.md). It was proposed the same day as "the interrupt contract", which is why the analysis below was first written with that word. It is kept for the record.

Spec: [K5](K5-operation-service.md), with F4 §4.7–4.8 and K1 §4.10. Design: [INTERRUPTS.md](INTERRUPTS.md). Raised by Allen, 2026-10-14: there aren't the tokens to resolve surprises by trial and error.

**For classifying and resolving (v1.2 as written).**
- A diagnosed interrupt gets the right response: a trap is disarmed or mapped, a player is answered, a mechanic is fixed. Containment only avoids it.
- Avoiding places has a cumulative cost. On a busy server or in a contested region, quarantines pile up and the swarm loses ground it needs.
- Containment can be steered: an opponent who sets off quarantines on purpose can push the swarm out of an area.
- It is how F4 §4.7–4.8 and K5 §4.6 are already written.

**For containing and pricing.**
- Diagnosis costs tokens or a person's time for every kind of interrupt, and an open world has no end of kinds. The builder's F7 runs show the rate: one new kind per run, one session each.
- Minecraft's world is large and its resources repeat, so moving away is usually the cheapest response, and it needs no understanding.
- K7 can price a quarantine as a re-plan. So the escalation rule ("avoiding this costs more than the approved quote allows") is computable, and the tokens spent follow what is at stake.
- Batching by cost turns the long tail into a ranked list. Only the top clusters get a fix, each with a harness scenario rebuilt from recorded evidence, so trial and error moves into F3.

**What would tip it.**
- F3 runs with injected interrupts (INTERRUPTS §9): commander wakes and tokens per run-hour, and containment cost, against the same runs handled by per-kind triage.
- How fast quarantines accumulate on a busy server, measured on a live profile after the live gate.

**Lean (before the decision).** Contain and price, with known failure classes kept and grown only from the top of the batch review.

## AD-F5.2 How does the version boundary work when skills are Bevy plugins?

Spec: [F5](F5-version-layer.md). Raised by the spec audit (#1); evidence from the Azalea reality check (§8).

**For keeping the rule literally (skills see only Null's types).**
- Porting really is "fix the adapter": an Azalea upgrade never touches a skill crate.
- Azalea's own churn is heavy: each release has had 3–9 ECS-level breaking changes (`PhysicsState` → `ClientMovementState`, `Instance` → `World`, `Block` → `BlockKind`), and upstream doesn't follow semver.
- Zero-copy wrapper `SystemParam`s keep the runtime cost near nothing; the cost is upkeep, not CPU.
- It forces a clean statement of what each skill needs from the game, which helps tests and fakes.

**For loosening it (curated re-exports, or direct imports).**
- Most churn is renames and moves. One re-export file absorbs those, which gets most of the literal rule's benefit for a fraction of the upkeep.
- No adapter hides Bevy itself. Skills that are Bevy systems depend on Bevy's API (four majors in fourteen months) whatever the rule says, so "only the adapter changes" is never fully true.
- A full mirror is a second component model to maintain and keep in the same tick, or reflexes lag a tick behind.
- We carry a fork, so upgrades arrive when we choose. Other Azalea projects import it directly and pin a revision.

**What would tip it.** How much Azalea state a typical skill touches. Prototyping one movement skill and one inventory skill both ways shows the size of the mirror.

**Lean.** Curated re-exports from the adapter, with full translation only where behaviour may change between versions: movement, placement and item use.

**Update: debate, 2026-10-13** ([full write-up](history/AD-F5.2-bevy-boundary.md)). Allen leans toward no skills seeing Bevy. Three advocates and a judge converged on a hybrid that keeps his lean for decision code:
- A fourth option was missing: skills can run inside the ECS tick without being systems, called by one adapter host that borrows Azalea's components. No copy, no extra lock, no extra tick, and dispatch costs microseconds at 1,000 bots.
- One HQ spanning servers (AD-K1.1) makes several Minecraft versions at once plausible. Only a Bevy-free decision layer compiles unchanged against two forks pinning different Bevy majors.
- But pathfinder move families, the replay patch, raw packets and menu mechanics extend Azalea and can't sit behind a view. Those go in named mechanism crates that import Azalea and Bevy directly and still churn on upgrades.

**Updated lean.** Decision crates (skills, reflexes, arbitration, cargo filter, squad tactics) have no Bevy or Azalea dependency. Named mechanism crates import both directly, with generated surface lists and an upgrade-impact diff. Final after a spike that builds one skills crate against two adapters and checks reflexes act in the same tick. Version policy (Allen, 2026-10-14): the deployment ports together, with two versions at once only when a server forces it. That weakens the run-two-forks-at-once argument but not the lean, since every port is still a Bevy and Azalea upgrade.

**Update: spike, 2026-10-14** ([reference/SPIKE_F5_BOUNDARY.md](reference/SPIKE_F5_BOUNDARY.md)). The spike passed every test the debate set:
- One unmodified decision crate built and passed five simulator tests against three Azalea builds (Bevy 0.17, 0.18 and 0.19). The adapter needed no changes for upstream `main` and about 30 lines of renames for the older 0.14 release.
- Reflex packets left in the tick the trigger fired. Input sent as an Azalea message lands a tick late, so commands must be direct component writes.
- The host matched native Bevy systems within noise at 1,000 and 2,000 bots.
- The API had 31 members, 8 of them plain renames, under the debate's threshold of about 40.

What would still argue for the other side: the read side is where renames concentrate, and the spike's first run was simulator-only. Neither changes the lean. **Decided by Allen, 2026-10-14: the hybrid**, with a cargo-deny allowlist enforcing it. The simulator spike and the live server join check both passed. The live check ([spike §6](reference/SPIKE_F5_BOUNDARY.md)) covered the fall reflex on and off, chest to hotbar, and goto then bridge, and found one Azalea gap: `UseItem` is not sent for a block-targeted click. The `mc-nav` alias module and whether K6 squad leaders are decision code are implementation choices. Mining and many-bot live load are untested.

## AD-K1.1 Does one HQ cover one server world, or several?

Spec: [K1](K1-hq.md). Raised by the spec audit (#4). Which server the swarm plays on is decided; this is how many worlds one HQ spans.

**Decided by Allen, 2026-10-13: one HQ per deployment, able to span several servers.** The cases below are kept for the record; the consequences are in K1.

**For one HQ per server world.**
- Inside one world, everything an HQ allocates can actually move: items, entities, bots. Across servers, nothing can, so a cross-server HQ would mostly be refusing transfers.
- The commander's bounded digest stays per world, which matches how decisions actually split (a war on one server doesn't share stock with a mine on another).
- Each HQ stays simple, and a failure on one server doesn't touch the others.

**For one HQ across servers.**
- One commander, one digest, one allocation of accounts: the swarm's whole picture in one place, which is the architecture's premise.
- Accounts are the scarce shared resource. Moving bots between servers (by relogging) is a real allocation decision that sits naturally in one HQ.
- At 2,000 bots on pure vanilla the swarm spans servers anyway, so the multi-server case is the full-scale case, not an edge.

**What would tip it.** Whether full-scale play means one faction on one SMP (with fewer bots there) or presence on several servers at once.

**Lean.** One HQ per world, with a `server` field in every location key from day one so a cross-server HQ stays possible without re-keying, and the commander API fronting several HQs with one combined digest line per HQ.
