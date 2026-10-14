# Spec error audit

> **Status (2026-10-14): closed.** Every finding is resolved: #1 became AD-F5.2 (decided 2026-10-14; spike and live join check passed); #4 became AD-K1.1 (decided); #6 is noted in C1 §9 for Allen's builder agent; the rest are fixed in the specs, marked "(spec audit #n)", and #2, #5 and #8 are also in ARCHITECTURE v0.8. Section references are to v0.7 and the specs as of 2026-10-13.

2026-10-13. Read: `specs/` (all 31 component specs, CONVENTIONS.md, DECISIONS.md, INDEX.md as of 15:04 UTC, after the specs thread's last round of edits) against `ARCHITECTURE.md` v0.7 and project memory. Nothing under `specs/` or `design/` was edited; every fix below is a proposal for the thread that owns the file.

**What this looks for.** Only serious problems: contradictions between specs or with the architecture, impossible or unfounded assumptions, and breaks of the hard constraints (survival only on pure vanilla, no op or creative or flying, ~2,000 bots over several machines, a bounded LLM context, switchable reflexes, a generic framework). Style and anything buildable as written are skipped.

**Settled points respected.** Which server the swarm plays on doesn't matter. The client is a custom Azalea build with the latest features speaking 1.21.11 natively. Local testing is per component. Builder internals belong to Allen's builder agent, so C1 is judged only on its interface. Proven community builds (the Skybender, dupers, chunk loaders) are trusted.

**Ranking.** By how costly each would be if found late: how much built code or in-world infrastructure would have to change. Estimates are marked as estimates.

## Summary

| # | finding | where | found late, it costs |
|---|---|---|---|
| 1 | The version boundary can't hold if skills are Bevy plugins | F5 §4.1, F1 §4.1, ARCH §9 | rewriting every skill crate on each Azalea upgrade |
| 2 | Bot-to-bot traffic grows with the square of local density, and the bandwidth model leaves it out | F1 §4.11, ARCH §3.8, K5 §6, AD-C7.1 | the capacity plan, node placement and operation sizes |
| 3 | Observations can't reach a shard in the operation service within the bus budget | ARCH §3.7–3.8, F2 §4.1, §4.10 | the belief data path, which INDEX says must be settled before code |
| 4 | Specs disagree on whether one HQ spans one world or several | F1 AD-F1.1, K1 header, F6 §4.2 | every region, ledger and map key |
| 5 | A network partition makes two nodes fight over the same accounts | F1 §4.2, §4.7, §4.10, §5, F6 §4.7 | kick storms that look like anti-cheat and can halt all joins |
| 6 | The builder contract has no inventory, hunger or durability precondition for open-loop replay | C1 §3.2, §4.2, B6 §4.7 | a contract change at the creative-to-survival switch |
| 7 | Cargo exclusion stops targeting, not collateral damage | B5 §4.6, C7 §4.1, §4.3, C9 §4.2 | breeze losses from the swarm's own escorts |
| 8 | "Break blocks outside claimed territory needs approval" blocks ordinary gathering | ARCH §4.2, K2 §4.4, B4 §4.8, C8 §4.1 | the first milestone stalls, or tickets flood the commander |

Seven lower-priority items follow the main eight.

---

## 1. The version boundary can't hold if skills are Bevy plugins

**What's wrong.** F5 §4.1 says only `mc-adapter` imports Azalea, enforced by a build check, so that "porting means changing Azalea's version and fixing the adapter, not everything above it" (ARCH §9.1). But ARCH §9 and F1 §4.1 make skills and reflexes Bevy plugins and systems in the node's ECS, with reflexes checking "a per-bot enable flag component". Systems that run per tick on a bot have to query that bot's state (position, inventory, world, physics), and in Azalea those are Azalea's own ECS components. On top of that, Azalea's ECS is `bevy_ecs` at whatever version Azalea pins, and Azalea follows Bevy releases, which change APIs often. A skill crate that is a Bevy plugin depends on that Bevy version even if it never names an Azalea type. Allen's stated preference for Azalea's built-in features (pathfinder, physics, inventory) also pulls Azalea's types upward, because those features are driven through Azalea's components and goals.

**What it breaks.** The portability requirement. As written, an Azalea upgrade (including tracking its latest features in the specialized build) means touching every skill and reflex crate, not just the adapter. The build check in F5 §8 would either fail on day one or push every skill into the adapter.

**Smallest fix (Allen's call, before §7 step 2).** Pick one and restate F5 §4.1 and ARCH §9 to match:
- (a) Skills are plain Rust behind narrow traits (`BotView` to read, `BotCommands` to act), and the adapter hosts them inside Azalea's ECS, wrapping built-ins like the pathfinder. Portable, but the adapter is thicker than "thin", and it has to run the mirrored state in the same tick so reflexes don't lag by one.
- (b) Accept Bevy and Azalea components above the adapter, and narrow the boundary to protocol, block-state and game-data types. Simpler, but porting cost then includes skills, and the requirement should say so.

The Azalea reality-check thread is the right place to measure how much of each skill touches Azalea's components.

## 2. Bot-to-bot traffic grows with the square of local density

**What's wrong.** F1 §4.11 models bandwidth as chunk data per bot, cut by asking for a small view distance. It leaves out the term that dominates when bots work close together. Every bot is a player, and the server sends each bot the movement, rotation, arm swings, equipment changes, block-break progress and block changes of every other player in its tracked chunks. Lowering the view distance doesn't remove this, because the bots in one operation are inside each other's view by design. So per-node inbound traffic is roughly N² for N bots in one area, and the server sends the same stream.

Rough estimate, for scale only: ~15 packets/s at ~20 bytes from each active bot to each bot near it, about 300 B/s per pair.

| bots in one area | per bot | per node |
|---|---|---|
| 100 | ~30 KB/s | ~3 MB/s (~24 Mbit/s) |
| 300 | ~90 KB/s | ~27 MB/s (~216 Mbit/s) |
| 1,000 | ~300 KB/s | ~300 MB/s (~2.4 Gbit/s) |

The vanilla server also does this tracking work on its main thread.

**What it breaks.** Several rules push density up: ARCH §3.8's "co-locate by region", 50–300 bots per operation in one area (K5 §6), war squads of ~50 (AD-C7.1's lean), and ~1,000 bots per laptop (F1 §4.1). The ~1,000-per-laptop figure can hold for bots spread out and fail badly for the same count on one build site or mining district. Per-component tests won't show it unless a load test packs bots together.

**Smallest fix.** Add a density term to F1 §4.11 and make "bots within mutual tracking range" a placement constraint alongside CPU and bandwidth. Make F6 §4.9's first load test compare N bots in one 64×64 area against N bots spread out, and measure bytes per bot as N grows. Design levers once that's measured: suppliers wait outside the site, idle bots stand still (stationary players send almost nothing), big sites are worked in separate sub-areas, and the operation size is capped by density rather than headcount.

## 3. Observations can't reach a shard in the operation service within the bus budget

**What's wrong.** ARCH §3.7 and F2 §4.10 put each operation's world shard in the operation service ("SQLite, local to the operation service"). ARCH §3.8 runs the operation service as its own process, "preferably" on the host with most of its bots, while an operation's 50–300 bots may sit on several nodes. The same section says raw observations never go on the bus and a node's upward traffic stays in the low KB/s. Those can't all be true. Observations have to cross from the node process to the operation service, and F2 says belief keeps "latest state per cell" for terrain, which means chunk contents. A bot walking into new terrain sees about one new chunk a second, at a few to tens of KB each (estimate). Comparing a re-seen chunk with belief to send only changes needs the old belief next to the chunk data, which is in the node.

**What it breaks.** The belief data path, which INDEX lists as needed before any code (AD-F2.1 and AD-F2.2). Neither decision says where belief physically lives relative to the node, and that is what sets the traffic.

**Smallest fix.** Settle it together with AD-F2.1 and AD-F2.2. Either (a) belief (terrain, containers, entities) lives in the node, per tile, and the operation service holds claims and the ledger and queries the nodes, or (b) an operation's service runs inside the node process. Allen's ~1,000 bots per host makes (b) cheap more often than v0.7 assumed, since a whole operation now fits on one node. Either way, state that only diffs and summaries leave the node.

## 4. Specs disagree on whether one HQ spans one world or several

**What's wrong.**
- F1 AD-F1.1 says "HQ is scoped to one server world."
- K1 says the opposite in its header: AD-F1.1 is decided, "so HQ shouldn't assume one world."
- F6 §4.2 says "one HQ can talk to swarm nodes on different versions". On pure vanilla, one server runs one version, so that means one HQ across several servers.
- ARCH §8 says a vanilla server won't hold 2,000 players in one world, so at full scale on pure vanilla the swarm spans several servers.

This doesn't reopen which server to use. It asks how many worlds one HQ covers.

**What it breaks.** If one HQ spans several servers, every coordinate, region, tile, ledger entry, transfer, convoy and coarse-map row needs a server key. Items and entities can never move between servers, so transfers and handovers across servers have to be refused. Moving bots between operations on different servers means relogging. If HQ is per server, the 2,000-bot swarm is several HQs, and the "one commander, one bounded digest" design needs a rule for combining them. Either way the keys go into F2, C3, C9 and K1's schemas, which are expensive to re-key later.

**Smallest fix (Allen's call).** One line in F1 and K1 that agrees: one HQ per server world, or one HQ across servers with a server key everywhere. Also say what happens at 2,000 bots if one vanilla world can't hold them.

## 5. A network partition makes two nodes fight over the same accounts

**What's wrong.** F1 §4.2 and §4.7: an account's lease to a node expires when the node's heartbeat is missed, and the account is then logged in on another node. F6 §4.7 and its partition test expect a home node that loses its link to keep running and "rejoin cleanly". But a node cut off from the bus is often still connected to the Minecraft server. When the new node logs the same account in, the server kicks the old session for a duplicate login. F1's kick classes don't include that case, and the lifecycle sends kicked bots back to `connecting`. So the old node logs in again and kicks the new one, and the two nodes ping-pong. With hundreds of bots on the partitioned node, this is a mass kick, which F1 §5 answers by stopping all joins to the server until a person intervenes. A home network blip can halt the whole swarm. The same gap applies to claims: when a lease expires on the bus side, nothing stops the cut-off holder from acting on it.

**What it breaks.** "Leases expire, so nothing stays locked" (ARCH §3.8) assumes an expired holder stops acting. Nothing in the specs makes it stop.

**Smallest fix.**
- A node that can't renew its leases fences itself before they expire, by holding or disconnecting by its own clock.
- Reassignment waits the lease time plus a grace period.
- A duplicate-login kick is its own class, "superseded: never reconnect", and is excluded from the mass-kick halt.
- Claims carry a fencing token that the shard checks.

Cargo keepers are the hard case: fencing them drops presence. So Allen should decide whether cargo accounts are ever reassigned during a partition, or held until a person decides.

## 6. The builder contract has no inventory, hunger or durability precondition

**What's wrong.** C1's plan (§3.2) gives each step `items_needed` counts and a `packet_stream_ref` that B6 replays open-loop (§4.7). In survival, the effect of a fixed packet stream depends on bot state the contract doesn't carry:
- **Which slot holds which item.** Held-item packets name slot numbers, so the same stream with the items in different slots places the wrong blocks.
- **Tool durability.** A tool breaking mid-stream changes every break time after it.
- **Hunger.** At 6 hunger or less a player can't sprint, so a stream with sprint moves ends somewhere else and gets corrected.
- **Status effects.** Haste and Mining Fatigue change break times.

ONLINE_BUILDER.md and builder-hard-cases.md don't mention any of these, because creative hides all of them. `upkeep.eat` and `upkeep.tool_swap` are off during replay (B2 §4.9), so nothing fixes them mid-step either.

**What it breaks.** ARCH §3.9's goal that the creative-to-survival switch shouldn't change the contract. This is the same class of surprise as AD-C1.2, but a separate one: it happens even when the items do arrive.

**Smallest fix (for Allen and the builder agent).** Add a per-step precondition to the plan: slot-to-item layout, a minimum durability per tool, a hunger and saturation floor, and no effects that change break times. Add a "loadout" action at each step boundary that arranges them, which fits naturally into the restock of AD-C1.2's recommendation.

## 7. Cargo exclusion stops targeting, not collateral damage

**What's wrong.** B5 §4.6, B7 §4.9, C9 §4.2 and CONVENTIONS §4.5 protect cargo by never choosing it as a target. Most ways an escort hurts cargo don't involve targeting it:
- a sword's sweep attack damages every living entity next to the mob being hit;
- arrows and tridents miss into the train;
- the mace's smash and Wind Burst knock nearby entities around;
- a bot's wind charges and explosives hit the cargo;
- a dodging or retreating bot can push a boat.

C7 §4.1 spaces bots out of each other's sweeps but not the cargo's. C7 §4.3 escorts fight from a ring around the cargo, which is where these hits land.

**What it breaks.** The breeze pipeline's loss model (C9 §4.11 sizes reserves from measured losses), and escorting cargo through hostile mobs in general. The losses would be self-inflicted, and the harness would show them as "expected losses".

**Smallest fix.** In B5 and C7, add a cargo-proximity weapon rule: within a set radius of cargo, no sweeping attacks (axe or sprint hits only), no ranged attacks with cargo in or near the line of fire, and no explosives, wind charges or mace smash. Add one B5 harness test of a fight next to tagged cargo.

## 8. "Break blocks outside claimed territory needs approval" blocks ordinary gathering

**What's wrong.** ARCH §4.2 and K2 §4.4 put "break blocks outside claimed territory" in the needs-approval class, and B4 §4.8 applies it before every break outside territory. C8 §4.1 defines territory as operator annotations plus HQ's region claims, which leaves it unclear whether an operation's own region claim or a convoy's moving claim counts. On a fresh server almost everything is outside territory: tree felling for §7 step 1 (`Obtain(oak_log, 64)`), mining, the nether expedition, digging to a stronghold, B1's unstuck ladder (§4.5 rung 3), C9's rail-laying through the nether.

**What it breaks.** Read literally, the first milestone needs a ticket per job, and at scale that floods the commander's bounded digest. The alternative is that everyone quietly sets a blanket policy, which defeats the rule.

**Smallest fix (cheap at any time, but it should land before the skeleton).** Count active operation region claims and moving claims as claimed territory for natural blocks. Apply the approval class only to blocks that B4 §4.8's ownership rules mark as player-made.

---

## Lower priority

9. **Observation ordering isn't tick-accurate across bots** (F2 §4.2, §5). Belief merges by "newest `at_gt` wins", but a client only learns game time from a time packet about once a second and estimates in between. Two bots on different nodes can disagree by their latency difference, so a fresh observation can lose to a stale one at busy cells (build sites, contested chests). Fix: order by receive sequence within a node, and treat cross-node differences under about a second as a conflict to re-observe.
10. **Per-node budgets are still sized for 300 bots** (F1 §6 tick budget, B1 §6 path budget, AD-B1.1's spike at 300 bots per node), while F1 §4.1 now sizes nodes at ~1,000. Restate the budgets for the node size actually planned.
11. **Account authentication limits aren't modelled** (F1 §4.2, §4.3, §4.8). This is unverified: Microsoft/Xbox token refresh and the session server's per-IP limits apply to every login. A server restart reconnecting hundreds of accounts, and account migration, may hit them. Add them to what F1 §4.3 measures.
12. **Azalea's own built-in behaviours versus "every reflex can be turned off"** (CONVENTIONS §4). If the build keeps Azalea's default plugins, anything autonomous among them (auto-respawn, auto-reconnect, and a pathfinder that mines or places on its own) must either be registered as reflexes or listed as non-reflexes in §4.5. Worth confirming in the Azalea reality-check thread.
13. **Server-software profiles go beyond pure vanilla** (F5 §4.6 Paper and Folia overrides, C4 §8's Paper test). This is extra scope, not an error, but it contradicts the "pure vanilla only" assumption in memory.
14. **The digest budget is stated three ways**: ARCH §4.3 says 4–8k tokens, K1 §6 says 3–8k, and K3 §4.1 says ~8.5k before reads. Pick one number for the bake-off.
15. **Boat capacity** (ARCH §3.10, B7 §4.4, C9 §4.6). A boat holds 2 passengers, so it carries 2 mobs only when no bot rides it. B7 says bots row boats with mobs aboard, which leaves 1 mob per boat. Convoy sizes in quotes can be off by up to 2× depending on which is meant.

## Checked and consistent

- **Reflex contract.** Every reflex table (B1, B2, B5, B7) is switchable at every scope. F1's arbitration emits `ReflexSuppressed` when a reflex is off, and K6 restores settings when a job or play ends. Cargo filtering, replay mode, halt and recall, and fallback are correctly kept outside it.
- **Survival only.** Op appears only on the harness's own channel (F3 §4.2), and `/forceload` and creative appear only as prototype notes.
- **Bounded LLM context.** One row per program or operation, merged exceptions, paged inspection, and expected losses don't escalate.
- **Generic framework.** No machine-specific calls. Errands and annotation effects are generic and closed.
- **Time units.** Game ticks versus wall time is applied consistently, and the server's tick rate can be read from the vanilla protocol.
