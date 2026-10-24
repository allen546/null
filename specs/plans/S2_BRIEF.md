# S2 brief: message types and transport

Implementer brief for one Sonnet agent (written 2026-10-24, not started; Allen held the agent). Paste it as the agent's prompt. Milestone: `BUILD_PLAN.md` S2. Next after it: S3 (control spine on simulated bots), then S6.

---

You work in /Users/allen/Desktop/null (Null: a swarm of Minecraft bots in Rust with an LLM commander above it). Read CLAUDE.md first. Your task is milestone **S2: message types and transport**, two new decision crates: `crates/null-proto` and `crates/null-bus`.

## Read first

- `specs/plans/BUILD_PLAN.md`: the goal paragraph at the top, the milestone table, "### S2: message types and transport" (your task, word for word) and "### S3: control spine on simulated bots" (the next task, which will use your types; make sure S3 can be built on them without changes).
- `specs/CONVENTIONS.md` §3.1 Job contracts, §3.2 Time units, §3.3 Claims, §3.3a, §4.4 Jobs and reflexes.
- `specs/F6-bus-deployment.md` §3 Interfaces, §4.2 Subjects and message schemas, §4.3 Delivery semantics.
- `specs/K2-commander-api.md` §3 Interfaces (the commands).
- `specs/ARCHITECTURE.md` §3.6 Goals and planning; `specs/DECISIONS.md` AD-K1.1 (one HQ spans servers, so every location key carries `server`).
- Skim `specs/K1-hq.md`, `K5-operation-service.md`, `K6-squad-leader.md` for the summary, digest, exception and escalation fields.
- Look at an existing decision crate (`crates/null-bot-api`, `crates/null-belief`) for style, and at `crates/null-bot-api` for types you should reuse instead of redefining (block positions, item ids, ticks). If a type there fits, use it; if `null-proto` would need to depend on it, check that keeps both crates decision crates.

## What to build

Follow the S2 section exactly: the ids, `Loc`/`Region` with `server`, typed time `Gt`/`WallMs` (no bare integers for durations), goals (`Obtain { item, n }` plus the other ARCHITECTURE §3.6 kinds as declared variants), jobs, outcomes with structured reports, claims (kinds, owner, priority, expiry; take/renew/release requests; grant/deny/preempt replies), upward messages (operation summary with fixed fields, merged exceptions with a count, escalations with remedies tried), the digest (one row per operation), bot messages, commands (status, inspect, set_goal, cancel_goal, halt, recall, `SetReflexes`), and `proto_version`.

`null-bus`: a `Bus` trait with the subject scheme `null.<server>.<level>.<id>.<kind>` and the six message classes of F6 §3 with their guarantees. The skeleton implementation is in-process on tokio channels. Keep the trait shaped so a NATS implementation can sit behind it later (don't build NATS).

Where a spec leaves a detail open, pick the simplest choice that S3 needs, and list each such choice in your report. Don't build anything S3 owns (HQ, operation, squad, CLI, simbot logic).

## Rules (from CLAUDE.md; follow exactly)

- Both crates are decision crates: they must build on stable (`cargo +stable build -p null-proto -p null-bus`) and must not reach azalea or bevy. `scripts/check_boundary.sh` must pass. `serde` is allowed in `null-proto`; tokio is allowed in `null-bus`.
- Every major function and type (each message family, the `Bus` trait, each class's delivery logic, the in-process bus) has a one-line link to its spec, e.g. `// Spec: specs/F6-bus-deployment.md §4.3`, pointing at a section that exists. The verifier fails on a missing or dead link.
- Comments state the why in a line or two and point to a doc for more. No long design essays in code. Code reads like the surrounding code.
- No live servers are needed for S2: do not start or use any Minecraft server, and do not touch the Pi. Never read or change network configuration on any device. Python, if you need it: uv only (`uv run --with ...`), never pip.
- Don't edit the Azalea fork. Don't commit or push.
- Keep the size near 600–800 lines of code plus tests. If you are stuck after about two attempts on one problem, stop and report.

## Done when

Every message type round-trips through JSON (unit tests); the in-process bus honours each class's guarantee in tests (at least: a duplicate intent is applied once, summaries keep only the latest, halt overtakes a full queue, a claim request times out with no reply, escalations with the same key are deduplicated, the event log is append-only and readable in order); `cargo +stable build` of both crates, `cargo test -p null-proto -p null-bus`, `cargo test` for the workspace (or at least that nothing else broke) and `scripts/check_boundary.sh` pass.

Report briefly: files and line counts, the test list with results, every open detail you decided and why, and anything S3 will need that S2 does not yet give.
