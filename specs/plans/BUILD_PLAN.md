# Build plan: the Null swarm

**Plan, 2026-10-16.** Scaffolds the whole swarm in the order of ARCHITECTURE §7. Step 1 (the skeleton) is broken into milestones S1–S6; later steps are outlined and get milestones when they come up. Implementation is done by Sonnet subagents. Each milestone is cut into tasks that each finish well under an hour (for example: test server, one crate and its tests, the binary, the live run); no single subagent runs for an hour or more (Allen, 2026-10-16). Tasks are sized by importance and estimated lines of code, aiming for 500–800 lines each. S4 was one task: about 2,000 lines plus a cold Azalea build took 67 minutes. Each milestone is checked by a separate verifier subagent: Haiku for builds and unit tests, Sonnet for anything live. The bar is that bots act correctly in-game (Allen, 2026-10-16). If the current docs disagree with this plan, the docs win.

## Layout

- Code lives in this repo: a Cargo workspace at the root, crates under `crates/`, tools under `tools/`, scripts under `scripts/`. `specs/` stays the single source of truth for documents.
- The root `rust-toolchain.toml` pins `nightly-2026-10-12`, the Azalea fork's toolchain. Decision crates must also build with `cargo +stable`. `scripts/check_boundary.sh` checks that, and checks that no crate outside the allowlist (`mc-adapter`, `mc-nav`, `mc-act`, and binaries that link them) depends on `bevy*` or `azalea*` (F5 §4.1).
- The Azalea fork is a path dependency: `~/Desktop/null-spikes/azalea`.
- Test servers are outside the repo, under `~/Desktop/null-spikes/`: `combat-server/` (flat, PvP; port 25566, RCON 25576) and `skeleton-server/` (S4; port 25567, RCON 25577). The bench server keeps 25565.
- Nothing is committed without Allen asking.

## Step 1: skeleton (ARCHITECTURE §7.1)

The goal: all four levels (HQ, one operation, one squad, two bots) run in one process and talk through the real message types. A CLI commander sets `Obtain(oak_log, 64)`, and both bots split the work with no collisions.

| id | milestone | crates | depends on | status |
|---|---|---|---|---|
| S1 | Workspace and decision API | `null-bot-api`, `null-mechanics`, `scripts/check_boundary.sh` | | done 2026-10-16 (combat M1) |
| S2 | Message types and transport | `null-proto`, `null-bus` | S1 | not started |
| S3 | Control spine on simulated bots | `null-hq`, `null-op`, `null-squad`, `null-cli`, `null-simbot` | S2 | not started |
| S4 | Adapter, host and login | `mc-adapter`, `null-node`, `skeleton-server/` | S1 | done 2026-10-16, verified live |
| SA | Ability check: basic abilities proven live, fixed in the fork where needed ([ABILITY_CHECK.md](ABILITY_CHECK.md)) | `null-abilitycheck`, `mc-adapter`, fork | S4 | in progress: A1–A8 and the block shape families pass on Paper + Grim (2026-10-17); A9–A12 open ([HANDOFF.md](../../HANDOFF.md)) |
| S5 | Skills and belief for the goal | `null-skills`, `null-belief` | S4, SA | not started |
| S6 | Skeleton live | wiring only | S3, S5 | not started |

S3 and S4 can run in parallel. **First target (Allen, 2026-10-16): S4, a swarm that logs in and moves around.** It is done, and so is the combat track's M2 with the swarm duel test. **Next (Allen, 2026-10-17): SA, the ability check, before S5 and S2.** S5 needs S4 and SA, not S2: its claims can be held locally until S3's squad hands them out in S6. S2 and S3 feed S6 only.

### S2: message types and transport

- **`null-proto`** (decision crate; `serde` allowed). These are the types every level uses, versioned with a `proto_version`:
  - Ids: `ServerId`, `OperationId`, `SquadId`, `BotId` (account name), `JobId`, `ClaimId`, `GoalId`.
  - Location keys carry `server` (AD-K1.1): `Loc { server, dim, pos }`, `Region { server, dim, min, max }`.
  - Typed time: `Gt` (game ticks) and `WallMs` (CONVENTIONS §3.2). No bare integers for durations.
  - **Goals:** `Goal::Obtain { item, n }`, with the other ARCHITECTURE §3.6 kinds as declared variants that the skeleton rejects as unsupported.
  - **Jobs** (CONVENTIONS §3.1): id, kind and parameters, required items and claims, preconditions, success test, budget (with units), `reflexes`, `checkpoint`, `preemption_floor`.
  - **Outcomes:** `Succeeded | Failed(reason) | Blocked(on) | Preempted | Interrupted(incident)`, each with a structured report.
  - **Claims** (CONVENTIONS §3.3): kind (region, block set, container, items, entity, bot), owner, priority and expiry. Requests are take, renew and release, with grant, deny and preempt replies.
  - **Upward messages:**
    - the operation summary, with fixed fields (state, headcount, progress, stock, health, top issue);
    - exceptions, merged with a count;
    - escalations, carrying the remedies already tried.
  - **The digest**, one row per operation.
  - **Bot messages:** `BotState`, `JobOutcome`, `Observation` (a block or entity seen, with observer and tick).
  - **Commands** (K2 §3): status, inspect, set_goal, cancel_goal, halt and recall. Plus `SetReflexes`.
- **`null-bus`** (decision crate): a `Bus` trait using the subject scheme `null.<server>.<level>.<id>.<kind>` and the message classes in F6 §3:
  - intent: at least once, with an idempotency key;
  - summary: latest wins;
  - escalation: deduplicated by key;
  - claim: request and reply with a timeout;
  - event log: durable, append-only;
  - command: highest priority, never queued behind other traffic.

  The skeleton implementation runs in-process on tokio channels. NATS comes in step 4, behind the same trait.
- Done when: every message type round-trips through JSON (unit tests); the in-process bus honours each class's guarantee in tests (a duplicate intent is applied once, summaries keep only the latest, halt overtakes a full queue); the stable build and the boundary check pass.

### S3: control spine on simulated bots

- **`null-simbot`**: a toy world (a grid with trees at known positions) and fake bots that run `fell` jobs in simulated ticks. Fake bots can die and disconnect.
- **`null-hq`** (K1, thin):
  - a goal tree from commands;
  - creating one operation per goal;
  - headcount allocation;
  - the digest;
  - halt and recall, never gated.
- **`null-op`** (K5):
  - **Claims registry:** every claim has an expiry and a priority, preemption sends `Preempted`, and an expired claim frees itself.
  - **Expansion:** `Obtain(oak_log, n)` becomes `fell` jobs, one claim per tree (a block-set claim).
  - **Scheduler, ledger, summaries:** a scheduler that hands job batches to its squad; a ledger counting logs from outcomes; summaries with fixed fields.
  - **Event log:** an append-only JSONL log. A restarted operation rebuilds its claims and ledger from it.
- **`null-squad`** (K6): assigns jobs to idle bots, retries a failed job locally once before reporting, applies job reflex settings, and passes merged observations up.
- **`null-cli`** (K2 subset plus K4): `set-goal obtain oak_log 64`, `status`, `inspect <op>`, `halt [scope]`, `recall [scope]`.
- Done when: in simulation, `Obtain(oak_log, 64)` completes with 2 bots, and the claims log shows no tree claimed by two bots at once. In further runs:
  - a bot killed mid-job has its claim expire and its job requeued;
  - halt stops new work within one squad tick;
  - an operation killed and restarted resumes from its log with the ledger intact;
  - the digest size doesn't grow with bot count (2 bots against 50).

### S4: adapter, host and login

- **`skeleton-server/`**: vanilla 1.21.11 copied from the combat server. Survival, peaceful, offline mode, port 25567 and RCON 25577. A fixed seed with oak trees near spawn (record the seed and tree positions). Gamerules use the 1.21.11 snake_case names in `combat-server/README.md`.
- **`mc-adapter`** (mechanism):
  - **`BotView`:** implemented by borrowing Azalea components inside `GameTick`.
  - **`BotCommands`:** implemented as direct component writes, never `Update`-schedule messages (spike §3.2).
  - **Host system:** one system runs each bot's `Brain` (null-bot-api arbitration) every tick.
  - **Setup rules:** add plugins before the app is built (spike §4.2). Disable or register Azalea's own autonomous behaviours (CONVENTIONS §4.5).
  - **`goto`:** Azalea's pathfinder, behind a command (AD-B1.1).
- **`null-node`** (F1 shell, binary): brings up a list of accounts on one server, with reconnect and backoff. It emits `BotState` and runs per-bot skills handed to it in-process.
- Done when:
  - simulator tests cover each `BotCommands` variant;
  - live, two bots log in to `skeleton-server` and walk to a given point using the pathfinder;
  - live, a swarm of 20 bots in one node logs in and each walks to its own assigned point and back, twice;
  - the server log shows no kicks and no movement warnings.

**Status (2026-10-16): done, verified live by a separate Sonnet agent through RCON and the server log.**
- Two bots reached the `goto` point. 20 bots made two out-and-back cycles each, with 20 online at once.
- The server logged 0 kicks, 0 movement warnings, 0 WARN or ERROR lines, and 22 joins matched by 22 leaves.
- A bot kicked mid-patrol reconnected with backoff and finished its patrol.
- Simulator tests: 13 in `mc-adapter`, one or more for each `BotCommands` variant.

What it added:
- `null-bot-api` gained `goto`, `stop_nav` and `nav_status`.
- New crate `null-skills-demo` holds the `patrol` and `goto_point` skills.
- `null-node` flags: `--goto`, `--center`, `--state-log`, `--exit-after-skills`, `--join-interval-ms` (default 300).
- Use release builds: debug builds of the pathfinder are slow.

Server: seed 3, world spawn set to 416,70,352, about 38 oaks within 40 blocks, listed in the server README. Gaps:
- `goto` starts one tick late, because Azalea's pathfinder only takes `Update` messages. Every other command applies in the same tick, and the view reports `Calculating` meanwhile.
- `recent_attackers` is always empty, because Azalea doesn't track attackers. It must be filled before combat M3, because the no-attacking-a-player-first rule (AD-K2.2) depends on it.
- `using_item` always reports the main hand and 0 ticks.
- Use on crosshair (`UseItem` with a block in view) is not built; it goes in the fork's item-use API (F5 §4.1). Use on a face comes with SA task 1.
- A patrol point that lands on a tree top or a blocked spot is shifted along the ring, so shifted points may sit near another bot's point.
- S4 ran as one subagent task for 67 minutes. 22 of those went to a seed search for S5's trees. In S5, place test trees with RCON `/place feature` if the natural ones get in the way.

### S5: skills and belief for the goal

- **`null-belief`** (F2, minimal): a store keyed by fixed tiles (AD-F2.1), fed by observations (AD-F2.2's packet observer can come later). Each entry records the block, who saw it and when. Queries: `find_blocks(type, near, radius)` and `block(pos)`, with ages.
- **`null-skills`** (decision crate): `fell(tree_type, quota)` (B4), built on `goto` (B1) and `break_block` with the right tool and break timing (B6). It also counts inventory (B3 minimal).
  - It fells only trees whose claim it holds.
  - It collects drops.
  - It returns yield and changed cells.
  - It never breaks a block that isn't natural (B4 §4.8 ownership rule: break only what is clearly natural).
- Done when:
  - unit tests run on `FakeView`;
  - live, one bot fells a claimed tree completely and picks up the logs;
  - belief shows the tree gone, with the bot as observer.

### S6: skeleton live

- Wire S3's spine to S4/S5's real bots in one process (`null-node` hosting HQ, the operation and the squad).
- Done when, on `skeleton-server`, the person types `set-goal obtain oak_log 64` in `null-cli` and the two bots collect 64 logs between them. Further conditions:
  - the claims log shows no tree claimed or broken by two bots;
  - no block is broken outside a held claim;
  - `status` shows the operation's progress while it runs;
  - `halt` stops both bots within a second;
  - a bot disconnected by RCON `kick` mid-job reconnects, and its job is requeued.

  The verifier checks this in-game through RCON and the server log, not only through Null's own logs.

## Later steps (milestones planned when each starts)

| step | ARCHITECTURE §7 | main components | notes |
|---|---|---|---|
| 2 | Survival core | B2, B3, B4, B6, C3, F1 (death, reconnect), F4 | gather, craft, smelt, eat, sleep, store; logistics ledger; recovery from death and disconnect |
| 3 | Planner | K7, K5 | tech-tree expansion to iron and diamonds unattended; the recipe graph from game data (F5) |
| 4 | Distribution | F6, F1, K5 | NATS behind `null-bus`; levels across processes and hosts; node failure and account reassignment; a load test with a few hundred bots (the bench hardware plan) |
| 5 | LLM commander | K3, K2 | needs AD-K2.1 decided (the bake-off didn't settle it) |
| 6 | Builder integration | C1, C2, B6 | world transformation (`proposals/WORLD_TRANSFORMATION.md`); its tests T-ACC, T-WE, T-OSC, T-COB |
| 7 | Nether and End | B5 (PvE), C5, C6 | the full `KillDragon` goal |
| 8 | Faction | C7, C8, B5 (PvP) | the combat track below joins here |

## Combat track

Combat milestones are in [COMBAT_SCAFFOLD.md](COMBAT_SCAFFOLD.md). Its M1 is S1 here. M2 (the combat decision crate) was started 2026-10-16, before the scope was widened to the whole swarm. M3 (live duel) needs S4. M3 and M4 (parameter tuning) wait until the spine works (ARCHITECTURE §10: combat moves in parallel once the spine works).
