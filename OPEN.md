# Open for Allen: start here next thread

> State on `main`, 2026-10-19 (night). SA, the ability check, is closed: every row passes on vanilla and Paper + Grim. S5 is under way: the action mailbox (AD-F5.4) and `null-belief` are built, verified and committed (`ec8f492`). Fork patch 25 (no action result lost to a newer one) is pushed and pinned (`70a1295`; full suite 362/362 on vanilla and on Paper + Grim, zero Grim alerts). The `gather` skill with the tree policy is written and unit-tested but uncommitted and not yet live. Standing rules for Claude: [CLAUDE.md](CLAUDE.md). Milestone details: [HANDOFF.md](HANDOFF.md) and `specs/plans/BUILD_PLAN.md` S5. Past decisions by date: `specs/history/DECISIONS_LOG.md`. Don't load `specs/logs/` (about 600k tokens of old builder logs).

## 1. Next actions, in order

1. ~~**Fork patch 25**~~ done 2026-10-20: every in-flight break, place and block open settled by the server's cumulative ack; pushed and pinned; full suite 362/362 on each server; verified by an independent verifier with a negative control (old one-record behaviour: the instamine row got 4 of 16 Done) (HANDOFF).
2. **`gather` live phase (S5 done-when):** verify and commit the pure `null-skills` code; the adapter's `block_property`; live rows (trees, a hut, the claim edge, the quota, a tall trunk); the step time measured live; the full suite on both servers. The list is in BUILD_PLAN S5.
3. **`mine` (after S5):** methods as presets (fishbone, quarry, tunnel; B4 §4.12), sight mode for anti-x-ray servers (B4 §4.11; belief needs `how = seen_sight`), and underground hazards handled by the miner, placing to seal fluids (B4 §4.13).
4. **S2 and S3** (message types, transport, the control spine on simulated bots), then S6 (the skeleton live: `Obtain(oak_log, 64)` split between two bots).
5. **Bench, remote server.** Borrow the M5 MacBook Airs (family OK pending; backup: hourly VPSes). Add `host=` to `azbench` and a remote mode to `run_one.py`. Ladder 100–500 bots with 3 and 5 threads. Watch the Airs for throttling.
6. **Shared section cache** in the Azalea fork (99% of section building was repeated), then the same ladder again.
7. **Bench gaps:** lock-wait and per-thread timing (why the ECS tick waits), the level-off at 22–28 searches per second, re-run 5 threads at 50 bots (suspect value), why 3% of repeated chunks have different bytes.
8. **Probe `[c]` facts** before anything depends on them: scaffolding (climbable; breaking the base drops the column), placed leaves get `persistent=true` (the tree policy depends on this), which entity attributes the server sends to clients.
9. **World transformation tests** T-ACC, T-WE, T-OSC, T-COB (§8).
10. Optional: a Docker topology in F3 for distribution logic (HQ, NATS, 2–3 nodes, injected latency and partitions), once F1 and F6 exist.

## 2. Nothing blocks building

All 22 architecture decisions are decided (16) or deferred (6). Nothing is marked "before any code". Unflagged §9 questions in the component specs are measurements or implementation details, deferred to the build by rule (CONVENTIONS, Flags).

## 3. Waiting on your call, when it comes up

None of these blocks current work. Each has a default.

| item | when it matters | default now | where |
|---|---|---|---|
| AD-K2.1 commander API: imperative commands or a desired-state manifest. Bake-off ran 2026-10-14: no decisive difference | before the LLM commander | manifest hybrid | K2, DECISIONS, `specs/reference/BAKEOFF_K21_2026-10-14.md` |
| AD-C3.1 central depots or storage per operation | large logistics | per operation, depots when shared | C3 |
| AD-C7.1 fights bigger than one squad | large fights | bigger squads first | C7 |
| AD-C8.1 bots talk to people in v1 | first public server | silent | C8 |
| AD-C9.1 one convoy's escort across two nodes | multi-node convoys | never; split trains | C9 |
| AD-F6.1 third bus quorum member | several always-on hosts | defer | F6 |
| World transformation §7 items 1–4: which jobs merge into one service; the isolation shell as the basis of the guarantee; trust pack claims or check once per version; refuse unorderable packs or allow some search | builder build-out | proposals as written | proposals/WORLD_TRANSFORMATION.md §7 |
| World transformation §7 item 6: access planning (placement options, strict layers, scaffolding temps) for large technical builds | after test T-ACC | adopt; devise something else if unsafe | same, §5.3, §8 |
| Builder execution model: packet replay, skills or a hybrid | builder build-out | deferred | same, §7 |
| A rule for conditional mob states (wither, shulker) | when a third case appears | none | GENERALIZATION §0.5 |
| AD-B2.1 after a death: recovery against quarantine, and whether a death is a hard interrupt | step 2 design | working rule: go back to work, or escalate | DECISIONS |
| When the commander's load justifies LLM officers | when the commander's context or wake budget overflows | no officers (AD-K5.1) | K5, GENERALIZATION §0.6 |

## 4. Where things are

| what | path |
|---|---|
| spec index and counts | `specs/INDEX.md` |
| both sides of every decision | `specs/DECISIONS.md` |
| generalization shape | `specs/proposals/GENERALIZATION.md` §0 |
| builder redesign | `specs/proposals/WORLD_TRANSFORMATION.md` |
| benchmark report | `specs/reference/BENCH_AZALEA_2026-10-15.md` |
| commander bake-off report | `specs/reference/BAKEOFF_K21_2026-10-14.md` (code and logs: `~/Desktop/null-spikes/k21-bakeoff`) |
| bench code and raw data | `~/Desktop/null-spikes/azalea-bench` (`data/`, `scripts/queue.py`) |
| build plans and milestone status | `specs/plans/BUILD_PLAN.md`, `specs/plans/COMBAT_SCAFFOLD.md` |
| swarm code | `crates/` (Cargo workspace at repo root); release build `cargo build --release -p null-node` |
| skeleton test server | `harness/skeleton-server` in this repo (scripts and README; the server world and match runs stay local and untracked); `~/Desktop/null-spikes/skeleton-server` links to it |
| current milestone hand-off, how to run the ability check, open items | `HANDOFF.md` |
| decisions log by date | `specs/history/DECISIONS_LOG.md` |
| belief store, skills (`gather`, tree policy) | `crates/null-belief`, `crates/null-skills` |
| action mailbox | `crates/null-bot-api/src/action.rs`, `crates/mc-adapter/src/actions.rs`, spec B6 §4.6 |
| ability check (runner and scenarios; closed 2026-10-19, still the regression suite) | `crates/null-abilitycheck`; plan `specs/plans/ABILITY_CHECK.md` |
| Paper + Grim test server (reference) | `harness/grim-server` (README; world untracked) |
| Azalea fork | public `allen546/azalea`, branch `null-main` (1.21.11 + 38 cherry-picks + bench timers + our patches, nightly-2026-10-12); local clone `~/Desktop/null-spikes/azalea`; Null pins a commit in `Cargo.toml` |

## 5. Size of the spec base (estimate: characters ÷ 3.3 to ÷ 4)

| part | files | about tokens |
|---|---|---|
| core specs (`specs/*.md`, the diagram) | 39 | 135–165k |
| proposals | 5 | 37–45k |
| reference | 19 | 140–170k |
| history | 9 | 41–50k |
| logs (old builder runs) | 19 | 540–650k |
| **spec base proper (core + proposals)** | 44 | **170–210k** |
| everything under `specs/` | 91 | 890k–1.08M |
