# S3 brief: control spine on simulated bots

Implementer briefs for two Sonnet agents, run one after the other (written 2026-10-24; S2 notes added; S3a done 2026-10-24, its notes added; S3b not started). Milestone: `BUILD_PLAN.md` S3. It needs S2 done first (`specs/plans/S2_BRIEF.md`: `null-proto`, `null-bus`). Next after it: S6 (the skeleton live).

S3 has five crates, more than one task of 500–800 lines. So it is two tasks, each with one done-when:

- **S3a:** `null-simbot` (the toy world and fake bots) and `null-op` (the operation). Done when an operation and fake bots, wired directly with a minimal stand-in squad, collect logs with no double claims, and the operation restarts from its log.
- **S3b:** `null-squad`, `null-hq` and `null-cli`, then the S3 done-when runs end to end.

Before either task starts, the lead checks S2's report for the open details it decided and for gaps it named for S3, and adds them to the brief.

## What S2 gives and leaves (lead's check, 2026-10-24; paste into both briefs)

S2's open-detail choices are listed in `BUILD_PLAN.md` S2 "Done 2026-10-24". The ones that matter for S3:

- `null-proto` has its own `Pos`, `ReflexTarget` and `ReflexGroup` (copies of the `null-bot-api` ones, with serde). Use the `null-proto` ones on the bus; convert only at a real-bot boundary (S6).
- Claims: requests carry `ttl: Gt`; a grant carries `expires_at: Gt` and a fencing `token: u64` (F6 §4.7). The registry issues tokens and checks them on renew and release. `Preempt` is a reply the registry sends to the old owner. Denials are `Held`, `NotHeld`, `NotOwner`.
- `OpSummary.exceptions` is merged by `merge_exception()` and capped at `MAX_EXCEPTIONS` (8).
- Only halt and recall use the command class (`Bus::send_command`; others return `NotUrgent`). Status, inspect, set_goal, cancel_goal and `SetReflexes` travel as `IntentBody::Command`. A command sent before anyone subscribes is held for the first subscriber, so a component may subscribe late, but subscribe at startup anyway.
- Intent, escalation and claim are one-consumer queues per subject; a second subscribe returns `AlreadySubscribed`. Duplicates are dropped on receipt by key, so every intent and escalation needs a stable key (a replayed or resent message reuses its key).
- Subjects match exactly, no wildcards; the id part may be empty (`null.smp1.hq.digest`). `Bus` is generic-only (`impl Future` returns), not `dyn`.
- The event log stores `LogEvent { at: WallMs, kind, data }` with `data` as JSON text: S3a defines its own event payload types in `null-op` and serializes them into `data`. Don't change `LogEvent`.
- `fell` parameters are a free map (`BTreeMap<String, ParamValue>`): S3a picks the key names (for example `"trees": Locs`) and lists them in its report.

Gaps S2 left, and who fills them:

- S3a: the claims registry with expiry, preempt and tokens; `JobId`/`ClaimId` generation (deterministic from the seed or a counter that the log replays, so the same seed gives the same log); the `Obtain` to `fell` expansion.
- S3b: reply types for status and inspect (or read them from the digest; pick the simpler and say why); halt to all operations (HQ sends to each operation's command subject, or one agreed swarm-wide subject; pick one and say why).

---

## Task S3a: simulated world and the operation

You work in /Users/allen/Desktop/null (Null: a swarm of Minecraft bots in Rust with an LLM commander above it). Read CLAUDE.md first. Your task is the first half of milestone **S3: control spine on simulated bots**: two new decision crates, `crates/null-simbot` and `crates/null-op`.

### Read first

- `specs/plans/BUILD_PLAN.md`: the goal paragraph at the top, the milestone table, "### S3: control spine on simulated bots" (the whole milestone; you build the simbot and operation parts) and "### S6: skeleton live" (where this goes next: the same operation must later drive real bots, so keep the bot side behind the S2 message types, not simbot calls).
- `crates/null-proto` and `crates/null-bus` (S2, already built): use their types and the `Bus` trait. Do not redefine a message type; if one is missing or wrong for S3, add the smallest change to `null-proto` and list it in your report.
- `specs/K5-operation-service.md` §3 Interfaces, §4.2 Scheduler, §4.4 Ledger and world shard, §4.5 Claims registry, §4.6 Recovery rules, §4.7 Escalation, §4.9 Restart from the event log, §5 Failure handling, §8 Tests. Skip §4.1 (HTN planner), §4.3, §4.8 and §4.10 except where S3 needs a stub: the skeleton expands only `Obtain(oak_log, n)`.
- `specs/CONVENTIONS.md` §3.1 Job contracts, §3.2 Time units, §3.3 Claims, §3.3a.
- `specs/F3-test-harness.md` §4.3 and §4.5 (scenario runner, replay and determinism) for how the simulation should be seeded and replayed.
- Look at `crates/null-skills` for how a real `fell` job reports its result (yield, left trees, reasons), so the fake bots report the same shape.

### What to build

- **`null-simbot`:** a toy world (a grid with trees at known positions, each with a log count) and fake bots that run `fell` jobs in simulated ticks: walk, fell, report a `JobOutcome` with the yield. A seeded RNG drives every choice, so a run replays exactly. Fake bots can die and disconnect on command (tests trigger these at a chosen tick). Fake bots send and receive only S2 messages over the bus.
- **`null-op`** (K5):
  - **Claims registry:** every claim has an expiry and a priority; a higher-priority take preempts and sends `Preempted`; an expired claim frees itself; renew and release work. One block-set claim per tree.
  - **Expansion:** `Obtain(oak_log, n)` becomes `fell` jobs, one per tree, enough trees for `n` with a margin, each with its claim.
  - **Scheduler:** hands job batches to its squad over the bus; requeues a job whose claim expired or whose bot was lost.
  - **Ledger:** counts logs from outcomes, never from intents. The goal is met when the ledger reaches `n`.
  - **Summaries:** the operation summary with the fixed fields (state, headcount, progress, stock, health, top issue).
  - **Event log:** append-only JSONL. A restarted operation rebuilds its claims and ledger from it, with no double count.
- For your tests, a minimal stand-in squad (a few lines in the test module, not a crate) passes jobs straight to idle fake bots. S3b builds the real squad.

### Done when

In simulation, with the stand-in squad: `Obtain(oak_log, 64)` completes with 2 fake bots and the claims log shows no tree claimed by two bots at once; an expired claim frees itself and its job is requeued; a preempt sends `Preempted`; an operation killed mid-run and restarted from its log ends with the ledger intact (no double count, no lost logs); the same seed gives the same event log twice. `cargo +stable build` of both crates, `cargo test -p null-simbot -p null-op`, the workspace tests and `scripts/check_boundary.sh` pass.

Report briefly: files and line counts, the test list with results, every open detail you decided and why, any change to `null-proto`, and what S3b needs from you.

---

## What S3a gives and leaves (lead's check, 2026-10-24; paste into the S3b brief)

S3a is done (`null-simbot`, `null-op`; 18 `null-op` and 5 `null-simbot` tests; verified twice, safety bugs fixed). How it works, as far as S3b depends on it:

- Each dispatch is an attempt `n` of a job. Its claim id is `claim_id(job, n)` = `claim-<job>-a<n>`, owner `Job(id)`; the dispatched `Job` carries `claims = [that id]` and `attempt = n`. A stale bot's renew of an old attempt's id gets `NotHeld`. The registry denies any take that overlaps a live claim, the same owner's included; a same-owner take of the same live id renews it.
- Only an outcome for the current attempt of an in-flight job changes job state or releases a claim. An older attempt's yield still counts (the logs are real), once per (job, attempt) under key `out:<job>:a<n>`. Outcomes with attempt 0, an unknown job or attempt, or a bot outside the allocation are ignored and logged (`OpEvent::Ignored`).
- `fell` parameters: `"trees"` (Locs, one tree), `"quota"` (Int), `"tree_type"` (Text). Job ids `<op>-j0001` from a counter the log replays. Up to 3 dispatches per job, then `job_dead:<job>` escalates. Claim TTL is `Gt(100)`.
- `Operation::new/restore/kill/tick(now)`, `summary()`, `state()`, `ledger()`. Call `tick(now)` every tick; it never blocks. `kill()` returns the bus ports and `restore` takes them back (the in-process bus can't resubscribe a one-consumer queue).
- Subjects: `null-op` has `op_intents`, `op_cmd`, `op_claims`, `op_events`, `op_summary`, `squad_jobs`, `hq_escalate`; `null-simbot` has `bot_jobs`, `bot_up`, `bot_summary`, `bot_cmd`.
- Halt and recall both stop new dispatch; bots are not recalled.

The squad must (safety rules, from the S3a checks):

- Renew each running job's claim with `job.claims[0]`, never a rebuilt id, well before expiry; key running jobs by (job, attempt).
- Echo `job.attempt` in `JobOutcome.attempt`.
- Halt the bot at once on any failed renew: denied, `Preempt`, error or no reply.
- **Lease rule:** a bot must also stop by itself when its claim's `expires_at` passes without a successful renew (with a margin), so a bot that stalls and wakes never works a tree whose claim went to a new attempt. Add a test with a fake bot that stalls past its TTL and wakes: it must not take a log from a tree claimed by the next attempt. The sim fake bot does not check claims today; add the check where the squad or bot holds the lease.
- Stop renewing for a lost or silent bot, and report lost bots up, so `OpSummary.headcount.alive` counts live bots (today it is the allocated count).
- Resend outcomes after an op restart (the op dedupes them).

Known and accepted: `is_current_claim` (op.rs) is a defensive guard that no test reaches, because an old attempt's claim is released before the next is taken. The event log records the attempt, not the bot; the op doesn't choose bots (K6 §4.1).

## Task S3b: squad, HQ, CLI and the end-to-end runs

You work in /Users/allen/Desktop/null (Null: a swarm of Minecraft bots in Rust with an LLM commander above it). Read CLAUDE.md first. Your task is the second half of milestone **S3: control spine on simulated bots**: three new decision crates, `crates/null-squad`, `crates/null-hq` and `crates/null-cli`, and the S3 done-when runs.

### Read first

- `specs/plans/BUILD_PLAN.md`: the goal paragraph, "### S3: control spine on simulated bots" (your done-when is its done-when) and "### S6: skeleton live".
- `crates/null-proto`, `crates/null-bus` (S2) and `crates/null-simbot`, `crates/null-op` (S3a, already built). Replace S3a's stand-in squad in its tests with the real one where that is simple.
- `specs/K6-squad-leader.md` §3 Interfaces, §4.1 Job batch assignment, §4.2 Local retries, §4.4 Observation merging, §4.7 Reflex application, §4.8 Moving claim renewal, §5, §8. Skip §4.3, §4.5, §4.6 (tactics, formations, errands).
- `specs/K1-hq.md` §3 Interfaces, §4.1 Goal tree and programs, §4.2 Allocation between operations, §4.8 Digest, §4.10 Escalations to the commander, §5, §8. Skip the rest.
- `specs/K2-commander-api.md` §3, §4.1 Command set, §4.3 Audit log, §4.9 Halt and recall. `specs/K4-human-interface.md` §4.1 CLI.
- `specs/CONVENTIONS.md` §4 Reflex control (what `SetReflexes` changes).

### What to build

- **`null-squad`** (K6): assigns job batches to idle bots; retries a failed job locally once before it reports the failure up; applies each job's reflex settings; renews claims while a job runs; merges bot observations and passes them up.
- **`null-hq`** (K1, thin): a goal tree from commands; one operation per goal; headcount allocation; the digest (one row per operation); halt and recall, never gated or queued behind other traffic.
- **`null-cli`** (K2 subset plus K4): `set-goal obtain oak_log 64`, `status`, `inspect <op>`, `halt [scope]`, `recall [scope]`. It runs the whole spine in one process against `null-simbot` (a flag picks the seed and the bot count), and writes the audit log.

### Done when

In simulation, `Obtain(oak_log, 64)` completes with 2 bots, driven through `null-cli`, and the claims log shows no tree claimed by two bots at once. In further runs (tests):

- a bot killed mid-job has its claim expire and its job requeued;
- halt stops new work within one squad tick;
- an operation killed and restarted resumes from its log with the ledger intact;
- the digest size doesn't grow with bot count (2 bots against 50).

`cargo +stable build` of the three crates, the workspace tests and `scripts/check_boundary.sh` pass.

Report briefly: files and line counts, the test list with results, every open detail you decided and why, and what S6 will need to wire real bots in.

---

## Rules for both tasks (from CLAUDE.md; follow exactly)

- All crates are decision crates: they must build on stable (`cargo +stable build -p <crate>`) and must not reach azalea or bevy. `scripts/check_boundary.sh` must pass.
- Every major function and type (each component a spec defines: the claims registry, the scheduler, the ledger, the event log replay, squad assignment, HQ allocation, the digest, each CLI command, the simbot world) has a one-line link to its spec, e.g. `// Spec: specs/K5-operation-service.md §4.5`, pointing at a section that exists. The verifier fails on a missing or dead link.
- Comments state the why in a line or two and point to a doc for more. No long design essays in code. Code reads like the surrounding code.
- Where a spec leaves a detail open, pick the simplest choice the done-when needs, and list it in the report. Don't build what the done-when doesn't need (HTN planning, tactics, errands, the LLM commander, NATS).
- No live servers are needed for S3: do not start or use any Minecraft server, and do not touch the Pi. Never read or change network configuration on any device. Python, if you need it: uv only (`uv run --with ...`), never pip.
- Don't edit the Azalea fork. Don't commit or push.
- Keep each task near 600–800 lines of code plus tests. If you are stuck after about two attempts on one problem, stop and report.
