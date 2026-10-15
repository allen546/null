# F3 Test harness

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §7, §9.1, §10; WORKED_EXAMPLE_OSC.md §5; COMMANDER_INTERFACE.md §9 (bake-off); reference/builder-hard-cases.md §3.7 (corpus). Conventions: `CONVENTIONS.md`.
**Depends on:** F5 (versions). **Used by:** every component, which is tested here before it touches a live server.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Server profiles | Deterministic | Copy the target server's settings into a private server |
| 4.2 | Harness control channel | Deterministic | Op-only setup, observe and simulate commands, isolated from bots |
| 4.3 | Scenario format and runner | Deterministic | A typed scenario file and a runner that scores fixed checks |
| 4.4 | Adversary models | Interface · authored input · person | There is no ground truth for how hostile humans play; someone must author representative opponents |
| 4.5 | Replay and determinism | Deterministic | Record inputs, seeds and server state; replay them |
| 4.6 | Metrics and checks | Deterministic | Fixed counters and pass/fail predicates per scenario |
| 4.7 | Commander evaluation rubric | Interface · authored input · person | What counts as a good strategic decision is a value judgment |
| 4.8 | Version matrix | Deterministic | Same scenarios on every supported version |
| 4.9 | Live gate | Interface · runtime decision · person | Going from harness to a live server is a risk call with outward consequences |
| 4.10 | Measurement write-back | Deterministic | Measured values replace claimed values in the mechanics table and packs |

## Architecture decisions

### AD-F3.1 One server pool for the test harness and the builder's simulator? (decided)

**Decided by Allen, 2026-10-15: (a), one pool with two job types.** Both job types need the exact live jar and settings per version. The world-transformation redesign uses a simulator at most once per pack per version, and only if Null checks pack claims ([proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md) §5.4), so the simulator job type stays small. No ⚑. The options were (a) one pool and (b) separate fleets.

---

## 1. Purpose

F3 is a private vanilla server plus scripted scenarios, replayable runs and metrics. Every other component is tested here before it touches a live server. It is also where claimed numbers become measured ones (`[c]` → `[m]`), where versions are certified, and where the LLM commander is evaluated against a fake or real swarm.

## 2. Scope

**Owns:** private server provisioning to match target profiles; the harness's own op channel; the scenario format, runner and library; adversary and fake-player models; recording and replay; metrics; the commander bake-off; the version test matrix; the go-live checklist; writing measurements back.

**Does not own:** the builder's simulator S1/S2 (C1's, though it can share infrastructure); production telemetry (F4); what the swarm does (everything else).

## 3. Interfaces

- `Scenario` files (§4.3) in a library, versioned with the code.
- `RunRequest{scenario, version, profile, seed, variant}` → `RunReport{checks[], timeline, measurements, bot_hours, artifacts}`.
- `Measurement{key, value, unit, conditions, run_id}` → F5 mechanics table, C11 pack rates, K7 rate store.
- Fake HQ and fake swarm endpoints for K3 to run against before the real swarm exists.

## 4. Requirements

### 4.1 Server profiles

- A profile copies the target server's observable settings: version, server software, view and simulation distance, difficulty, gamerules as far as the operator knows them, idle timeout, player cap, anti-cheat plugin if any.
- Vanilla defaults are assumed for anything the operator can't read without op, and the report says so.
- Fixed seeds with known structures (trial chambers, fortresses, strongholds) for repeatable scenarios.

### 4.2 Harness control channel

- **The harness has op; bots never do** (WORKED_EXAMPLE_OSC §5.2). The harness uses op only to set up worlds, observe (`/data`, entity counts, chunk state, MSPT) and simulate conditions (`/tick rate`, restarts, fake players, block removal).
- The control channel is a separate connection with its own credentials. No bot process can reach it, and a bot that sends a command is a test failure.

### 4.3 Scenario format and runner

```
scenario: chest_emptied_mid_job
setup:     world seed, region paste, inventories, bot positions, belief preload
script:    timed harness actions (at 1200 gt: empty chest at (x,y,z))
goal:      commander or job input (Obtain(iron_ingot, 32))
checks:    predicates over the run (job outcome == blocked(container_changed) within 200 gt)
variants:  named perturbations (lag, restart, node kill)
budget:    game ticks and wall seconds
```

- The runner starts the server, brings bots up through the real F1 runtime, runs the script, evaluates checks, and writes a report.
- Scenarios from the architecture and reviews are seeded into the library: bot in a cave with a creeper, chest emptied mid-job, node dies with 200 bots, server restart mid-convoy, escort preempted, exit portal broken, enemy following a convoy, the builder corpus (builder-hard-cases §3.7), and the OSC stages S0–S6 with variants V1–V8.
- Every fix from the interrupt batch review adds a scenario built from a recorded evidence bundle, so the cluster it fixes can't come back unnoticed (INTERRUPTS §4.2).

### 4.4 Adversary models

> **Interface input (authored).** Who: person, with an LLM drafting variants.
> Why: real opponents adapt and there is no ground truth for "how a hostile faction plays"; a scripted enemy that is too weak or too predictable gives false confidence, and only a person who knows the servers can judge if a model is representative.
> Enters the system as: adversary scripts in the scenario library, each with a written description of what real behaviour it stands in for.
> Null's side: runs them, scores the swarm against them, and keeps every run replayable so a person can watch it.

Adversary kinds needed: a griefer who empties chests and breaks builds; a follower who tails convoys and scouts; a PvP player or group using known techniques (crits, shields, pearls, crystals where allowed); a portal hijacker; a chat manipulator who writes injection text on signs and in chat. Fake players run through the harness's own client, not through Null's code.

### 4.5 Replay and determinism

- Every run records: the server's seed and settings, harness actions with game times, every packet each bot sent, bus messages, and periodic world snapshots.
- A run can be replayed against the same server build to reproduce a failure. Full determinism isn't possible on a real server (entity and chunk ordering, RNG), so replays are for diagnosis, not proof.

### 4.6 Metrics and checks

Standard counters on every run: deaths, kicks, anti-cheat setbacks, claims leaked after the run, jobs by outcome, bot-hours, items and entities lost, MSPT, bus traffic per node, digest token count, escalation count. Each scenario adds its own checks.

### 4.7 Commander evaluation rubric

> **Interface input (authored).** Who: person.
> Why: the bake-off (COMMANDER_INTERFACE §9) scores tokens and round trips mechanically, but "was this the right call" in a raid or a shortage is a value judgment that someone accountable must write down.
> Enters the system as: a rubric file per scenario (acceptable decisions, unacceptable ones, and why), used by graders.
> Null's side: computes everything mechanical (tokens per wake, round trips, duplicate or contradictory commands, flip-flops per hour, whether injected text ever led to a tool call).

### 4.8 Version matrix

- Every supported version runs the same scenario set (§9.1). Adding a version: generate its data, fill its mechanics table, fix the adapter, get the scenarios green.
- Builder corpus and pack acceptance tests run per version; certificates never carry across versions.

### 4.9 Live gate

> **Interface input (runtime decision).** Who: person.
> Why: deciding that a component is ready to run on a real server, with real players and accounts at stake, is a judgment about acceptable risk, not a number the harness can produce.
> Enters the system as: a go-live sign-off recorded against a component version and server profile.
> Null's side: produces the evidence (scenario pass rates, anti-cheat setbacks, leaks) and refuses to enable a component on a live profile without a sign-off.

### 4.10 Measurement write-back

- Measurements from runs replace claimed values: mechanics table entries (spawner wave size, cooldowns, spawner tracking), pack rates, capture and loss rates, travel speeds.
- Each measured value carries its conditions (version, profile, bot count) and run id, so K7 quotes show `[m]` with a source.

## 5. Failure handling

A run that crashes the harness itself is a harness bug, reported separately from scenario failures. Flaky checks (pass and fail on the same input) are marked flaky and quarantined until fixed, never retried until green.

## 6. Scale and performance

- Most scenarios: 2–20 bots on one host.
- Local testing is per component, not a full 2,000-bot run (Allen, 2026-10-13). Each component is tested in isolation against its scenarios.
- Load scenarios: up to what one host can run (Allen estimates ~1,000 bots on a laptop), measuring CPU and bandwidth per bot, since those are the limits, not memory.
- Big-scale soak tests are rare and expensive; small pipelines first (WORKED_EXAMPLE_OSC S0).

## 7. Version-sensitive parts

Server jars, data generators and the harness's own command syntax per version.

## 8. Tests

The harness tests itself: a scenario that is known to pass, one known to fail, and one that tries to send an op command from a bot (must fail the run).

## 9. Open questions

- How close can a private server come to the target's anti-cheat behaviour if the target runs a paid plugin?
