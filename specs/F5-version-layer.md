# F5 Version layer

**Spec v1.2 · 2026-10-14.** Sources: ARCHITECTURE.md v0.8 §9, §9.1, §10; reference/AZALEA_REALITY_CHECK.md; reference/SPIKE_F5_BOUNDARY.md; reference/builder-hard-cases.md §5.3; reference/minecraft-tech-inventory.md A.9, A.10. Conventions: `CONVENTIONS.md`.
**Depends on:** Null's Azalea fork (one Minecraft version per build). **Used by:** everything; its mechanism crates are the only code that imports Azalea or Bevy.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Decision and mechanism crates | Deterministic | A fixed rule enforced by the build: decision crates never import Azalea or Bevy |
| 4.2 | Generated game data | Deterministic | Produced by the vanilla data generator plus shape extraction |
| 4.3 | Mechanics table: schema and lookup | Deterministic | A keyed table queried by feature |
| 4.4 | Mechanics table: values | Measured | Each value is probed in the harness or sourced, with a confidence mark |
| 4.5 | Feature queries | Deterministic | `has_feature("crafter")`, never version numbers |
| 4.6 | Server settings | Measured | Vanilla settings change mechanics in known, testable ways; other server software is not a target, but mechanism code must also run cleanly on Paper with an anticheat |
| 4.7 | Porting procedure | Deterministic | Generate, fill, fix, test, in that order |
| 4.8 | Content compatibility across versions | Interface · runtime decision · person | When a community machine breaks on a new version, only a redesign fixes it |

## Architecture decisions

### AD-F5.1 Which Azalea build and Minecraft version? (decided)

Decided by Allen, 2026-10-13: a specialized Azalea build that carries the latest Azalea features but speaks the 1.21.11 protocol natively (not a ViaVersion-style translation layer). The adapter boundary (§4.1) still applies, so a later protocol change stays inside `mc-adapter`. No ⚑.

### AD-F5.2 How does the version boundary work when skills are Bevy plugins? (decided)

**Decided by Allen, 2026-10-14: the hybrid.** Decision crates have no `bevy*` or `azalea*` dependency; named mechanism crates import them directly; a `cargo-deny` allowlist enforces it (§4.1). The live vanilla 1.21.11 server join check passed (spike §6). Whether `mc-nav` uses an alias module, and whether K6 squad leaders are decision code over a `SquadView`, are implementation choices, not architecture. No ⚑. The analysis below is kept for the record.

- **The problem (spec audit #1):** §4.1 says only `mc-adapter` imports Azalea, but skills and reflexes are Bevy plugins in the node's ECS (F1 §4.1), and per-tick systems must query bot state, which lives in Azalea's own components. Skill crates also depend on whatever Bevy version Azalea pins, and Bevy has had four major versions in fourteen months. As written, the build check would fail on day one or push every skill into the adapter.
- **Options** (Azalea reality check §8):
  - (a) Keep the rule literally: the adapter is a Bevy plugin translating between Azalea's components and Null-owned ones (or zero-copy wrapper `SystemParam`s); skills never name an Azalea type. Most portable, most upkeep; Bevy types still appear in skills.
  - (b) Loosen it: one adapter module re-exports a curated Azalea ECS surface under Null's names (`BotPosition`, `WalkIntent`). Renames and moves, which is how most churn shows up, are absorbed in one file; semantic changes still leak.
  - (c) Allow direct Azalea imports in skill crates, with a build check that lists which Azalea types each skill uses. Cheapest now, most expensive at each upgrade; what other Azalea projects do.
  - Running skills outside the ECS through Azalea's `Client` handle is ruled out at this scale: Azalea's guide says it takes locks and loses most of the performance benefit.
  - (d) Found in the debate: skills run inside the ECS tick without being systems. Decision code is plain Rust against Null-owned `BotView`/`BotCommands`, called by one adapter host system in GameTick that borrows Azalea's components (no copy, no mirror, no extra tick).
- **Recommendation (updated after the debate, 2026-10-13):** a split by kind of code. **Decision crates** (skills, reflex triggers and actions, F1 §4.5 arbitration, the cargo filter, squad tactics) follow (d): no `bevy*` or `azalea*` dependency. **Mechanism crates**, named on an allowlist (`mc-adapter` host and views, `mc-nav` move families, `mc-act` packets, menus and the replay patch), import Azalea and Bevy directly, with a generated per-crate Azalea surface list and an upgrade-impact diff before each fork bump. Enforced by a cargo-deny allowlist. Matches Allen's lean for decision code. Final after a short spike: one skills crate builds unchanged against two adapters (the 1.21.11 fork and upstream with its Bevy 0.19 bump), and reflexes act in the same tick. Debate: [debates/AD-F5.2-bevy-boundary.md](history/AD-F5.2-bevy-boundary.md).
- **Why it needs Allen:** It sets how every skill crate is written and what a version port costs, and it restates the portability requirement.
- **Spike result (2026-10-14, [reference/SPIKE_F5_BOUNDARY.md](reference/SPIKE_F5_BOUNDARY.md)):** passed. One unmodified decision crate (API, three reflexes, two skills) built and passed five simulator tests against three Azalea builds: the 1.21.11 fork (Bevy 0.18), upstream `main` (Bevy 0.19) and the 1.21.10 branch (Bevy 0.17). Only the adapter changed: no lines for `main`, about 30 for 0.14, all renames and moves. Reflex packets left in the tick the trigger fired. The host matched native Bevy systems within noise at 1,000 and 2,000 bots. The API stayed at 31 members, 8 of them plain renames. A second run on a live vanilla 1.21.11 server (2026-10-14, spike §6) ran the fall reflex on and off, chest to hotbar, and goto then bridge, with matching results. It found that Azalea sends no `ServerboundUseItem` for a block-targeted right click, so `mc-act` needs separate `UseItem` and `UseOnCrosshair` commands. Not live-tested: mining, many bots, anti-cheat.
- **Settled by:** Allen approved the split (2026-10-14). The simulator spike and the live vanilla 1.21.11 server join test he had asked for (spike §6) both passed. Before skills are written (§7 step 2). Version policy, decided (Allen, 2026-10-14): the whole deployment moves to a new version together; two versions run at once only when a target server forces it. So the multi-version argument weighs less, but the lean stands: each port is still a Bevy and Azalea upgrade that should touch only mechanism crates, which is what the two-adapter spike tests.
- **If decided late:** Every skill crate is rewritten at the first Azalea or Bevy upgrade.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-f52-how-does-the-version-boundary-work-when-skills-are-bevy-plugins).

---

## 1. Purpose

The target is Java 1.21.11, and the code must port to other versions without a rewrite (§9.1). F5 isolates everything that changes between versions: the protocol (Azalea), game data, mechanics and server behaviour. Above it, skills and planners talk about features and named blocks.

## 2. Scope

**Owns:** the `mc-adapter` crate; per-version generated data (blocks, states, shapes, items, recipes, tool speeds, loot, mob stats); the versioned mechanics table; feature queries; server-software profiles; the porting procedure; compatibility marking for content packs.

**Does not own:** Azalea itself; the test scenarios (F3 runs them); content packs (C11), though F5 records which versions each pack passed on.

## 3. Interfaces

- `null-bot-api` (a decision crate) defines `BotView` (per-tick reads: position, motion, fluid, health, food, look, inventory and container slots, blocks, navigation status, change flags), `BotCommands` (intent-shaped: walk, sprint, jump, sneak, look, select slot, use item (a raw `ServerboundUseItem`), use on the crosshair (Azalea's block use), use on a given block face, release, goto, stop, close container, swap to hotbar), the `Skill` and `Reflex` traits and the per-bot arbitration. The spike's version had 17 reads and 14 commands.
- `mc-adapter` implements `BotView` by borrowing Azalea's components inside GameTick and applies `BotCommands`; it also exposes project types (`BlockState`, `ItemStack`, `EntitySnapshot`) with conversion from Azalea's.
- `GameData` API: `block("minecraft:observer")`, `shape(state)`, `recipes_for(item)`, `break_time(state, tool, effects)`, `loot(source)`, `mob(type)`.
- `Mechanics` API: `mechanic(key)` → `{value, unit, confidence: sourced|measured|claimed, source}`.
- `has_feature(name)` → bool.

## 4. Requirements

### 4.1 Decision and mechanism crates

- **Decision crates** (skills, reflex triggers and actions, F1 §4.5 arbitration and switch resolution, the cargo filter, squad tactics, everything above the node) depend on no `bevy*` or `azalea*` crate and build on stable Rust. They read through `BotView` and act through `BotCommands` (§3).
- **Mechanism crates** are named on an allowlist and import Azalea and Bevy directly: `mc-adapter` (the host, view construction, command application) and the binaries that link it. A `cargo-deny` allowlist fails any other crate that depends on `bevy*` or `azalea*`.
- **Raw capabilities live in the fork; `mc-adapter` only translates** (Allen, 2026-10-17). A game ability that Azalea lacks or gets wrong (placing on a face with client-side prediction and the server's result, holding and releasing an item, attacker tracking, menu and crafting helpers, new pathfinder moves, the replay switch) is a fork patch written as a normal Azalea client API. `mc-adapter` maps those Azalea components, whose shape may change between versions, onto Null's version-stable `BotView` and `BotCommands`, with no game logic of its own. The earlier plan for separate `mc-nav` (move families) and `mc-act` (raw packets, menus, replay) crates is dropped: their work goes into the fork. Capability patches go in new modules where possible and touch few existing lines, so a version rebase stays cheap ([reference/AZALEA_PATCHES.md](reference/AZALEA_PATCHES.md)).
- Each mechanism crate has a generated list of the Azalea types it uses; every fork bump starts with an upgrade-impact diff of those lists.
- Commands are applied as direct component writes or queued components, never as Azalea `Update`-schedule messages, which reach the server a tick late (spike §3.2).
- This is AD-F5.2, decided by Allen (2026-10-14); the simulator spike and the live server join check both passed.
- `UseItem` and `UseOnCrosshair` are separate commands. With a block under the crosshair, Azalea sends only `ServerboundUseItemOn`, never `ServerboundUseItem`, so a bucket, ender pearl or food used while looking at a block does nothing unless the fork's item-use API sends `UseItem` itself, with a prediction sequence number (spike §6.3, live run 2026-10-14). Worth an upstream issue.
- One binary per version: two Azalea versions can't share a process (§9.1). The commander API, job and event types are identical across versions.

### 4.2 Generated game data

- Source: Azalea's generated data for block states, collision and outline shapes and block behaviour constants; the vanilla server's data generator (`--reports`) only for what Azalea lacks (recipes, loot, support rules, mob data), stored per version (Azalea reality check, 2026-10-13).
- Code refers to blocks and items by namespaced name, never by numeric id.
- Includes placement-relevant facts the builder's pre-pass needs (support rules, attachment, gravity, waterloggable, connectable) as data.

### 4.3 Mechanics table: schema and lookup

Each entry: key, version, value, unit (game ticks where it's a duration), confidence, source (wiki URL, harness run id, or "claimed by pack X"). Lookup by key and the running version.

Initial keys from the docs: attack cooldowns per weapon, mace and spear behaviour, trial spawner wave size and cooldown and whether it tracks spawned mobs, despawn and persistence rules, entity cramming default, breeze attack range and rate, observer pulse on placement (by placement method), quasi-connectivity, which blocks are memory cells for the builder, portal cooldowns for vehicles (1.21.2 change), ender pearl chunk loading (1.21.2, 1.21.5), duper viability, villager leashability. *Decided (AD-K5.2, 2026-10-15): key families for threat profiles (per mob), place profiles (per structure, biome or height band), counter rules (hazard tag to capability alternatives) and exchange rows (bartering, trades), each with the same confidence and source fields. Game facts only: outcome measurements such as losses per kill go to K7's rate store. A field's values must come from the game jar, from observation or from the LLM's prior (marked unconfirmed), never written per row; proposals/GENERALIZATION.md §0, §3.2, §3.4.*

### 4.4 Mechanics table: values

- Every value has a confidence. `claimed` values (from community sources or packs) are probed in the harness before anything critical depends on them, and become `measured`.
- Where sources disagree (the tech inventory flags several, for example observer facing conventions and wind charge knockback multipliers), the harness measurement wins.

### 4.5 Feature queries

Skills ask `has_feature("crafter")`, `has_feature("copper_bulb")`, `has_feature("spear")`. A skill that needs an absent feature declares it, and the scheduler won't offer it.

### 4.6 Server settings

Null targets vanilla servers only (Allen, 2026-10-13, confirmed 2026-10-17). Profiles cover vanilla server settings that change mechanics: gamerules, view and simulation distance, spawn limits, difficulty. Other server software (Paper's duper-breaking defaults, Folia's regionised ticking, plugin settings) is not a target: plans and skills don't adapt to it. If one is ever targeted, its differences become mechanics overrides keyed by server software, measured in the harness (spec audit #13, 2026-10-13).

**Mechanism code must also run cleanly on Paper with an anticheat (Allen, 2026-10-17).** The Azalea fork and `mc-adapter` must give the same results, at vanilla timing, with zero anticheat alerts on Paper + GrimAC, because those mechanics are well documented. In practice this means:
- a bot sends only what a vanilla client could send; a request a vanilla client could not make is refused on the client, with a reason, and nothing is sent;
- the ability check (`specs/plans/ABILITY_CHECK.md`) passes on both test servers: vanilla `harness/skeleton-server` (the target) and Paper + Grim `harness/grim-server` (the strict check, timing included);
- a timing that an item or a vanilla setting can change is a belief, not a fact: a prior from the item's components or the mechanics table, refined by what the bot observes (example: the shield's raise-to-block delay, the held item's `blocks_attacks.block_delay_seconds`).

### 4.7 Porting procedure

The whole deployment moves to a new version together; two versions run at once only when a target server forces it, and then each version has its own fork, game data, mechanics table and harness run (Allen, 2026-10-14).


1. Merge upstream Azalea's version-bump commit into our fork and run its `migrate.py`. Null carries a fork (AD-F5.1), not a pinned release. Pin a Rust nightly that builds that version (each Azalea version has needed its own: the 1.21.10 branch crashes current nightly) and re-apply the fork's patches (byte counters, game time and block entities, per-bot replay switch, the swap-click prediction fix from the spike).
2. Generate the game data Azalea doesn't ship, with a generator version that still supports that Minecraft version's mappings (upstream's 26.1 generator can't regenerate 1.21.11 data) (Azalea reality check, 2026-10-13).
3. Fill the mechanics table: copy the previous version's values marked `claimed`, then probe each in the harness.
4. Fix the mechanism crates until they compile and their simulator tests pass. Decision crates are not touched; if one needs a change, the boundary leaked and the view or command set is fixed instead.
5. Run the full F3 scenario set, the builder corpus and pack acceptance tests.
6. Mark what passed.

### 4.8 Content compatibility across versions

> **Interface input (runtime decision).** Who: person.
> Why: community machines often depend on version quirks (the tech inventory notes an OSC generation scrapped because 1.21.4 broke entity alignment); when a pack fails its acceptance test on a new version, there's no derivable fix, only a redesign by someone who understands the machine.
> Enters the system as: an updated pack (new schematic or descriptor) from the pack's author or the operator, or a decision to drop the pack on that version.
> Null's side: runs every pack's acceptance test on every supported version, marks each pack `passes | fails | untested` per version, and refuses to quote a facility whose pack fails on the running version.

## 5. Failure handling

- An unknown block or item from the server (a newer server than the adapter) is a hard error for skills that touch it, logged, and reported; the bot avoids interacting with it.
- A mechanics lookup with no entry for the running version fails loudly rather than falling back to another version's value.

## 6. Scale and performance

Data lookups are in-process tables; mechanics lookups are cached at startup.

## 7. Version-sensitive parts

All of it, by definition.

## 8. Tests

- Adapter round trip: every block state in the generated data converts to Azalea and back.
- A crate outside the mechanism allowlist depending on `bevy*` or `azalea*` fails the build.
- Every `BotCommands` variant has a simulator test in Azalea's packet simulator (fake-view tests stay green through Azalea semantic drift; the spike's simulator test caught a swap-click bug no fake view would).
- Port drill: bring up a second version end to end through §4.7 with the test scenarios.

## 9. Open questions

AD-F5.2 is decided (Allen, 2026-10-14) and its live server join check passed. Implementation choices left inside it: whether K6 squad tactics are decision code over a `SquadView`.
