# Open for Allen: start here next thread

> State on `main`, 2026-10-17. Everything is committed and pushed (`allen546/null`). Read this, then `specs/INDEX.md` if more context is needed. Don't load `specs/logs/` (about 600k tokens of old builder logs).

## 1. Nothing blocks building

All 22 architecture decisions are decided (16) or deferred (6). Nothing is marked "before any code". Unflagged §9 questions in the component specs are measurements or implementation details, deferred to the build by rule (CONVENTIONS, Flags).

## 2. Waiting on your call, when it comes up

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
| When the commander's load justifies LLM officers | when the commander's context or wake budget overflows | no officers (AD-K5.1) | K5, GENERALIZATION §0.6 |

## 3. Decided 2026-10-16

- AD-B5.1: written doctrine first; then black-box tuning of each technique's named parameters; train from scratch only as a last resort. Every technique is a duel controller behind one observation→action interface. Swarm scaffold started: `specs/plans/BUILD_PLAN.md` (step 1 skeleton as S1–S6; combat is a track, `specs/plans/COMBAT_SCAFFOLD.md`). Code is in the repo's Cargo workspace, uncommitted. Sonnet subagents implement; Haiku (or Sonnet for live checks) verifies. Tasks are sized by importance and lines of code, 500–800 lines each, so none runs an hour or more.
- **Built and verified 2026-10-16:**
  - S1: the workspace, `null-bot-api`, `null-mechanics` and the boundary check.
  - Combat M2: `null-combat` with the `melee_basic` doctrine.
  - S4: `mc-adapter`, `null-node`, `null-skills-demo` and `~/Desktop/null-spikes/skeleton-server` (port 25567). Live, 20 bots logged in and patrolled out and back twice, with no kicks or movement warnings.
- **Swarm duel test (Allen watching, 2026-10-16):**
  - Two 8-bot nodes fight last team standing, with telemetry and analysis: `skeleton-server/scripts/duel_match.py`.
  - The tests found 4 Azalea bugs, now patched in our own public fork `allen546/azalea` (`null-main`; one commit per patch; `specs/reference/AZALEA_PATCHES.md`). No upstream PRs: Azalea's contributing guide asks for no AI-written code (Allen, 2026-10-16).
  - Match 2 stalled in 3 ways. **All three are fixed (evening debug session)**: the fight applier now resends its inputs every tick, recovery has a time limit, and seek re-plans when a path never comes. Four matches since ran to a winner. See "Debug session 2" in `specs/plans/COMBAT_SCAFFOLD.md` for details and open leads. **Then (night): `DuelLoop` was replaced by the per-tick `Engage` skill and the `combat.flee_low_hp` reflex (nothing latches); see "Debug session 3".**
  - 20 v 20 runs to a winner; retreat behaviour is rough and left for later (Allen, 2026-10-17). Committed and pushed.
- **Next build step (Allen, 2026-10-17): SA, the ability check** (`specs/plans/ABILITY_CHECK.md`): slots, breaking, pickup, placing, eating, shield, attack, attackers, inventory, chests, crafting and respawn, each proven by server readback, with fork patches where Azalea falls short. Then S2 and S5 (S5 needs S4 and SA, not S2).

## 3a. Decided 2026-10-15, in one line each

- AD-K5.2: facts as data from the game jar, observation and the LLM's prior; skill code only for cost-ranked important parts; goals to state and check tasks; the LLM plans steps with no model, in validated and quoted stretches (`submit_plan`). GENERALIZATION §0.
- AD-K5.1: no officers in v1. AD-K2.2: the LLM in full control; you hold halt and recall; permission checks on skills, set once (no attacking a player first; no breaking someone else's build, judged by "break only what is clearly natural").
- AD-F2.1: fixed tiles, operation service inside the node. AD-F2.2: separate belief store fed by a `chan` packet observer. AD-B1.1: reuse Azalea's pathfinder, default pool, shared section cache first.
- Builder: AD-C1.1 parallel across inert boundaries; AD-C1.2 restock on need; AD-B6.1 placement table plus probing; AD-F3.1 one server pool. Arming is not a separate action.
- Smaller: counter rules in F5; `carry` takes entity cargo; `quiet(zone)` job constraint; saved routines need no review; convoy right of way through quarantines; rescue a trapped bot only when K7 prices it above the expected losses.

## 4. Next actions (not decisions)

0. **SA, the ability check** (above). It comes before the items below.
1. **Bench, remote server.** Borrow the M5 MacBook Airs (family OK pending; backup: hourly VPSes). Add `host=` to `azbench` and a remote mode to `run_one.py`. Ladder 100–500 bots with 3 and 5 threads. Watch the Airs for throttling.
2. **Shared section cache** in the Azalea fork (99% of section building was repeated), then the same ladder again.
3. **Bench gaps:** lock-wait and per-thread timing (why the ECS tick waits), the level-off at 22–28 searches per second, re-run 5 threads at 50 bots (suspect value), why 3% of repeated chunks have different bytes.
4. **Probe `[c]` facts** before anything depends on them: scaffolding (climbable; breaking the base drops the column), placed leaves get `persistent=true`, which entity attributes the server sends to clients.
5. **World transformation tests** T-ACC, T-WE, T-OSC, T-COB (§8).
6. Optional: a Docker topology in F3 for distribution logic (HQ, NATS, 2–3 nodes, injected latency and partitions), once F1 and F6 exist.

## 5. Where things are

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
| skeleton test server | `~/Desktop/null-spikes/skeleton-server` (README has seed, spawn and oak positions) |
| Azalea fork | public `allen546/azalea`, branch `null-main` (1.21.11 + 38 cherry-picks + bench timers + our patches, nightly-2026-10-12); local clone `~/Desktop/null-spikes/azalea`; Null pins a commit in `Cargo.toml` |

## 6. Size of the spec base (estimate: characters ÷ 3.3 to ÷ 4)

| part | files | about tokens |
|---|---|---|
| core specs (`specs/*.md`, the diagram) | 39 | 135–165k |
| proposals | 5 | 37–45k |
| reference | 19 | 140–170k |
| history | 9 | 41–50k |
| logs (old builder runs) | 19 | 540–650k |
| **spec base proper (core + proposals)** | 44 | **170–210k** |
| everything under `specs/` | 91 | 890k–1.08M |
