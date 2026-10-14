# Spike: the AD-F5.2 version boundary on the 1.21.11 fork

**2026-10-14.** Tests the hybrid that the AD-F5.2 debate converged on ([specs/debates/AD-F5.2-bevy-boundary.md](../specs/debates/AD-F5.2-bevy-boundary.md)): decision code is plain Rust against Null-owned `BotView`/`BotCommands`, run by one host system inside Azalea's GameTick; only named mechanism crates import Azalea or Bevy. It ran the debate's four experiments on the custom Azalea build from [AZALEA_REALITY_CHECK.md](AZALEA_REALITY_CHECK.md) §2. The spike code was throwaway and is not kept in the project folder.

## 1. Bottom line

1. **The split holds.** One unmodified decision crate (API, three reflexes, two skills) built and passed the same five simulator tests against **three** Azalea builds: the 1.21.11 fork (Bevy 0.18), upstream `main` for 26.2 (Bevy 0.19), and the older 1.21.10 branch (Azalea 0.14, Bevy 0.17). The decision crates have **no dependencies and build on stable Rust**; only mechanism crates need nightly.
2. **Upgrade cost landed only in the mechanism crate.** Fork → upstream `main`: zero source lines changed in the adapter (only `Cargo.toml` paths and the Bevy version). Fork → 0.14: about 15 changed lines plus a 16-line helper, all renames and moves, which is the churn pattern the reality check predicted (§4).
3. **Reflexes act in the same tick.** The fall reflex's packets left in the tick its trigger fired, and it placed water 1.49 blocks above the floor during a 30-block fall. The debate's caution is real and measured: input sent as an Azalea message from GameTick lands **one tick late**; direct component writes land in the same tick.
4. **No measurable CPU cost.** With 1,000 and 2,000 bots in one ECS, a full GameTick with the host was within run-to-run noise (about ±5%) of the same reflexes written as native Bevy systems. The decision layer alone cost about 0.1 µs per bot per tick when nothing changed.
5. **The API stayed small.** 31 members (17 view reads, 14 commands), under the debate's threshold of about 40. 8 of the 17 reads are plain renames of an Azalea field; every command is intent-shaped.
6. **The simulator caught a real Azalea bug** that fake-view tests could not: the number-key swap's client-side prediction (B3's chest-to-hotbar) used the wrong slot and copied instead of moving. Fixed with a fork patch (§4.3).
7. **Live vanilla server: passed (second run, §6).** The first run could not fetch a server jar, so every test above ran in Azalea's scripted-packet simulator. A later run on an environment that reaches Mojang's hosts joined the fork to a real vanilla 1.21.11 server and ran the same kinds of scenarios there: the fall reflex (on and off), chest to hotbar, and goto then bridge all behaved as in the simulator. It also found one real gap the simulator could not (§6.3): Azalea never sends `ServerboundUseItem` when the crosshair is on a block, so a water bucket does nothing. Mining and any multi-bot load are still untested live.

**Recommendation:** adopt the hybrid as the AD-F5.2 answer. It is Allen's lean, the debate's result, and has simulator and live-server evidence behind it. Allen approved it on 2026-10-14 (below).

**Decided (Allen, 2026-10-14):** AD-F5.2 is the hybrid. He first chose to wait for the live server join test, then approved the split while the live run was in progress. The live join check (§6) passed.

## 2. What was built

| Crate | Kind | Depends on | Lines |
|---|---|---|---|
| `null-bot-api` | decision | nothing | 452 (API 249, arbitration 203) |
| `null-skills` | decision | `null-bot-api` | 368, plus 126 of fake-view tests |
| `mc-adapter` | mechanism | Azalea, Bevy, `null-bot-api` | 378, plus 292 of simulator tests |

- **Decision API.** `BotView` (a trait, so tests use a fake), `BotCommands` (a list of `Command`), `Skill` and `Reflex` traits, `ReflexSwitches` (a bitset of the already-resolved CONVENTIONS §4.2 scopes), `BotEvent`, and `Brain`, which runs F1 §4.5 steps 2–5 per bot per tick. Reflexes declare a watch mask; a trigger whose watched inputs didn't change reuses its last result.
- **Sample decision code.** Reflexes `upkeep.eat`, `move.mlg_bucket` (fall escape) and `move.lava_escape`. Skills `move.goto_bridge` (Azalea's pathfinder to an edge, then sneak-bridge) and `inv.chest_to_hotbar` (open, number-key swap, close). This is the debate's step 1 set.
- **Host.** Two chained systems in GameTick, after packet handling and before `PhysicsSystems`, `MiningSystems` and use-item handling. `decide` borrows Azalea's components as a `BotView` (no copy) and runs every bot's `Brain`; `apply` turns commands into direct component writes or Azalea events. Change flags come from Bevy's `Ref::is_changed`.
- **Fork.** Azalea `1.21.11` tip `41a9ae6a` plus the 38 cherry-picks from the reality check (all applied again, same 5 trivial resolutions), plus a `rust-toolchain.toml` pinned to `nightly-2026-10-12`, plus the swap fix. It builds and the upstream tests pass as before.

## 3. Results by experiment

### 3.1 Step 1: API size and where logic landed

| | Members | Plain renames of an Azalea field | Other |
|---|---|---|---|
| `BotView` reads | 17 | 8: position, velocity, on_ground, fall_distance, health, look, selected hotbar slot, using_item | 9 derived: tick, food, fluid (from two fluid heights), inventory slot (menu index mapping), container, container slot, block (state → name, solid, fluid), nav status (two components), change flags |
| `Command` | 14 | 0 | 7 direct component writes (walk, sprint, jump, sneak, look, look-at, select slot); 5 Azalea events or queued components (use item, goto, stop nav, close container, swap to hotbar); 2 raw packets (use on a block face, release use item) |

- Mechanism code was about a third of the non-test bot-layer code (378 of 1,198 lines), at the low end of the judge's 30–50% estimate. Real `mc-nav` move families and `mc-act` packet code will push it up.
- Reads are where renames concentrate; that is expected for primitive state and isn't a mirror of Azalea's components. Watch whether this ratio grows as more skills are written (the debate's drift risk).

### 3.2 Step 2: same-tick reflexes (simulator)

| Test | Result |
|---|---|
| Fall from 30 blocks with a water bucket in hotbar slot 3 | `SetCarriedItem{3}` and a look-down rotation sent before `ClientTickEnd` in the tick the reflex fired; `UseItemOn` against the floor sent 1.49 blocks above it |
| Walk input, direct write vs `StartWalkEvent` | Direct: `PlayerInput` in the same tick. Message: one tick later |
| `move.mlg_bucket` switched off | No action packets; `ReflexSuppressed` emitted every tick the trigger held |
| Chest to hotbar | `UseItemOn` on the chest, then a `Swap` click (slot 5, button 2), then `ContainerClose` once the bread showed in the hotbar |
| Goto then bridge | Azalea's pathfinder walked the 1-wide strip to the edge; two `UseItemOn` against the edge block's east face |

Why messages are late: Azalea reads its input messages in the 60 Hz `Update` schedule, and the real client runs `Update` before `GameTick` in each loop, so a message written in GameTick waits for the next loop's physics. **Rule for `mc-adapter`: commands become component writes or queued components, never `Update`-schedule messages.** Azalea's own pathfinder executor steers with messages, so it is one tick behind by design; a reflex that takes over from navigation overrides it with a direct write in the same tick, and must also stop navigation so the next message doesn't undo it.

### 3.3 Step 3: one skills crate, three adapters

| Azalea build | Bevy | Toolchain | Adapter source change | Simulator tests |
|---|---|---|---|---|
| 1.21.11 fork (0.15 + 38 picks) | 0.18.1 | nightly-2026-10-12 | (baseline) | 5/5 |
| upstream `main` 0.16.0+mc26.2 | 0.19.1 | nightly-2026-10-12 | none | 5/5 |
| `1.21.10` branch 0.14.0 | 0.17.2 | nightly-2025-12-09 (current nightly crashes the compiler) | ~15 lines + 16-line helper | 5/5 |

The 0.14 changes, all in the adapter: `PhysicsState` → `ClientMovementState`; `InstanceHolder.instance` → `WorldHolder.shared`; `Inventory` moved from `azalea-client` to `azalea-entity`; `azalea_registry::{Block, Item}` → `azalea_registry::builtin::{BlockKind, ItemKind}`; 0.14 registries have no `&'static str` name (and their `Display` already includes `minecraft:`), so the adapter interns names; one ordering anchor (`increment_counter`) was private. The simulator tests needed 12 similar lines. **The decision crates' hash was identical across all three builds.**

The fork → `main` result is weaker evidence than it looks: the fork already carries `main`'s API through the cherry-picks, and the Bevy 0.18 → 0.19 bump happened not to touch this surface. The 0.14 build is the one that shows real churn being absorbed.

### 3.4 Step 4: CPU

Release build, 4-core container, no network: Azalea's own simulation app (physics, movement, pathfinder, inventory, mining, interact plugins) with N bots wandering on a flat floor, one shared world. GameTick wall time, 300–400 ticks after warm-up.

| Bots | Baseline (wander only) | Native Bevy reflexes | Null host | Decision layer alone |
|---|---|---|---|---|
| 100 | 1.5 ms | 1.8 ms | 1.6 ms | |
| 500 | 5.7 ms | 5.6 ms | 5.6 ms | |
| 1,000 (3 runs) | 11.3–11.8 ms | 10.9–11.3 ms | 10.9–12.0 ms | host 99 µs, native 47 µs |
| 2,000 | | 24.1 ms | 22.5 ms | |

- Host and native are indistinguishable at this noise level; the debate's "more than about 2% slower means revisit" can't be resolved by this bench, but no penalty showed.
- About 11 µs of GameTick per bot, so 1,000 bots use roughly a quarter of the 50 ms tick on 4 cores before networking, packet handling and pathfinding. That is a useful floor for F1 §4.11, not a capacity number.
- Not measured: bots falling or in fluids (triggers re-evaluated every tick with block lookups), pathfinding load, packet I/O, bandwidth.

## 4. Things the specs should absorb

### 4.1 Confirmed design points

- Decision crates have no dependencies and build on stable Rust. The `cargo-deny` allowlist can be as strict as "decision crates may depend only on `null-*` crates".
- `Skill` and `Reflex` must be `Send + Sync` (they live in a Bevy component).
- The host owns the tick number: `TicksConnected` is missing in some Azalea setups, so the view's clock should come from the host.
- Fake-view tests stay green through Azalea semantic drift; simulator tests per `Command` variant are a hard requirement for mechanism crates (§4.3 is the proof).

### 4.2 Mechanism gotchas found

- Azalea's pathfinder registers its executor in `Plugin::finish`. Adding the plugin after the app is built leaves a bot that finds a path and never moves, with no error.
- Each Azalea version needs its own pinned nightly; the 1.21.10 branch crashes current nightly, as the reality check found for the unpatched 1.21.11 branch.
- Placing against a specific face needs a raw `UseItemOn` (Azalea's `force_block` always reports the top face). Releasing a held item has no Azalea event either. Both are `mc-act` packet code.

### 4.3 Fork patch added

`azalea-entity` `simulate_click` for `ClickOperation::Swap`: the button (0–8 hotbar, 40 offhand) was used as a menu slot index, and moving into an empty hotbar slot copied the stack instead of moving it. Upstream `main` and 0.14 have the same bug. The server corrects the client within a round trip, but skills would see wrong inventory state meanwhile. Worth sending upstream. Add it to the fork patch list (reality check §9 step 3).

## 5. Still open

- **Live server join** is done except mining (§6). Login, chunk load, walk, chest, place and falling all ran against vanilla 1.21.11.
- Anti-cheat ordering of the host's packets (use-item packets before the tick's movement packet) on a server that runs an anti-cheat. Vanilla itself raised no movement warnings and kicked no bot (§6.2).
- A load benchmark with bots falling, swimming and pathfinding, and bytes per bot (reality check §9 step 2); live, only single bots were run.
- The debate's other open points are unchanged: whether `mc-nav` uses an alias module, whether K6 squad leaders are decision code through a `SquadView`, and how the view's rename ratio evolves.

## 6. Live vanilla server run (2026-10-14, second run)

Run by a separate thread on an environment that reaches Mojang's hosts. Fork and test code are not kept in the project folder.

### 6.1 Setup

- **Server.** Vanilla 1.21.11 `server.jar` from `piston-data.mojang.com` (SHA-1 `64bb6d76…43ed`, matches the manifest), Java 21, offline mode, flat world, peaceful, view distance 6, console commands through a pipe. Everything ran on localhost, so there was no network latency.
- **Fork.** Rebuilt from the reality check's recipe: Azalea `1.21.11` tip `41a9ae6a`, the same 38 cherry-picks (same 5 conflicts, all in `Cargo.lock`, `CHANGELOG.md` twice, a codegen script and an example program), `rust-toolchain.toml` pinned to `nightly-2026-10-12`, plus the Swap fix from §4.3. It type-checks (`cargo check -p azalea`, 1 min 24 s).
- **Test code.** The spike code was not kept, so this run rewrote a smaller version of the same design: `null-bot-api` (137 lines), `null-skills` (105 lines: one reflex, two skills) and the adapter (174 lines). The decision crates have no dependencies and build on stable Rust. Differences from §2: one host system does decide and apply together (the spike chained two); one reflex (`move.mlg_bucket`) instead of three; the bridge skill places by crosshair raycast, not a raw face packet. So this checks the design on a live server, not the spike's exact code.

### 6.2 Results

| Scenario | Result on the live server |
|---|---|
| Join | Login, spawn and chunk load worked. `BotView::block` read real blocks, so the reflex's floor scan and the pathfinder both worked. Twelve joins over the run; no kicks, and no movement warnings in the server log (all disconnects were the harness closing the bot). |
| Fall reflex on | 30-block fall with a water bucket in hotbar slot 3: the bot survived at 20 health, the server shows a water block at the landing spot, and the bucket in slot 3 had become an empty bucket. |
| Fall reflex off | Same fall with `move.mlg_bucket` switched off: `ReflexSuppressed` every tick the trigger held (20 ticks), no action commands, and the server logged "fell from a high place". |
| Chest to hotbar | The bot opened the chest (container id 1, contents visible two ticks later), number-key swapped 3 cooked beef from chest slot 12 into the empty hotbar slot 2, and closed it. The server's inventory has the beef in slot 2 and the chest no longer has it. Same result on the fork with and without the Swap fix. |
| Goto then bridge | Azalea's pathfinder walked to the end of a 1-wide stone strip, the bot sneaked to the edge and placed two cobblestone blocks east. The server confirms both blocks, and the bot stood on the new bridge. |

- **The fall result matches the simulator, with detail it could not give.** The first successful place came at 1.49 blocks above the floor, the same figure the simulator gave. An earlier place at 3.22 blocks above the floor did nothing (see §6.3).
- **Same tick.** The reflex fired and its commands were issued in the host's tick at which the trigger first held (`SelectSlot` and look-down at once, `UseItem` as soon as the floor was in reach). Packet send times were not measured live; the same-tick claim still rests on §3.2.
- **Swap fix not separable live.** On localhost the server's correction arrives within a tick, so the live run cannot tell the client's prediction from the server's reply. The prediction bug is still shown only by the simulator test. An early run that used the same index for chest slot and hotbar slot (5 and 5) would have hidden it either way.

### 6.3 Findings the specs should absorb

1. **A block-targeted right click sends only `UseItemOn`.** In Azalea's `handle_start_use_item_queued`, a block under the crosshair means `ServerboundUseItemOn` and nothing else; `ServerboundUseItem` is sent only when the crosshair misses. Vanilla also sends `UseItem` when the block use does not consume the click, which is how a bucket, ender pearl or food works while looking at a block. The first live fall died because of this: the water bucket was "used" and nothing happened. The simulator could not catch it, since it only checks which packets left, not what a server does with them. Fix in `mc-act`: two separate commands, `UseItem` (a raw `ServerboundUseItem`, with a prediction sequence number) and `UseOnCrosshair` (Azalea's block use). B6 and any bucket, pearl or food skill must pick the right one. Worth an upstream issue or patch.
2. **Reach is measured from the eyes.** The first `UseItem` came at 3.22 blocks of feet-to-floor, which is 4.84 from the eyes against the 4.5 block interaction range, so the server ignored it. A reflex must test reach from eye height, not feet. In this fall (30 blocks, about 1.75 blocks per tick near the floor) two ticks were inside reach. From the arithmetic, not measured: at terminal speed (about 3.9 blocks per tick) the in-reach window is about 2.9 blocks, so a fall from high up can skip the window entirely on roughly a quarter of falls (2.9 of 3.9 blocks). A reflex cannot fix that by timing alone; the bot needs another escape for long falls.
3. **Setup order matters for tests.** `setblock` and `item replace block` fail with "That position is not loaded" for chunks nobody has loaded, so a test must put the bot in place first. Not an Azalea issue, but the F3 harness should do the same.
4. **Azalea client handlers do not keep per-bot state across events** unless it lives in a component or a shared static; a handler that mutates its `State` argument loses it. This was a harness mistake here, not a bug.

### 6.4 Not covered

- **Mining** a block (the reality check's list had it).
- **Many bots on a real server:** every scenario was one bot, so live CPU, bandwidth and the host's cost at 1,000 bots are unchecked beyond §3.4.
- **Anti-cheat:** vanilla only. The host's packet order against a server running anti-cheat plugins is untested.
- **Latency:** localhost only, so prediction-versus-correction behaviour under real latency is unseen.
