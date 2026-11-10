# Decisions log: what Allen decided, by date

**Historical.** Moved from `OPEN.md` on 2026-10-19 so that OPEN stays a short start page. Each entry is a one-line record; the full reasoning is in `DECISIONS.md` and the specs each entry names. Paths are relative to the repo root unless they start with `specs/`.

## Decided 2026-11-10

- Client level model: with own worlds, a bot's prediction is written into its own `World`, as vanilla's `ClientLevel` does; the patch 53 overlay and `BotWorld` go. It lands as one fork patch directly after the own-worlds patch, before the next pair. `specs/proposals/CLIENT_LEVEL_MODEL.md`.
- Block-entity state: the audit and the storage design now, the shulker box as the first kind; the piston stays refused until `mine` or building needs it. `specs/proposals/BLOCK_ENTITY_STATE.md`.
- Unit of execution (adds to AD-F5.6): a process runs its bots in shards, each one thread with its own app, loop and clock, over one section store, to spread the bots over the cores. Several processes per machine are the fallback. The common phase inside a shard is an accepted v1 limit. The spike (two shards of 25 against one swarm of 50, Mac) runs after the next pair. `specs/proposals/UNIT_OF_EXECUTION.md`.
- Test gate: a recorded vanilla session (inputs, server packets by exact client tick, client packets) becomes an offline test of the fork, as the everyday tier. The spike is built now; the exact code is decided then. VD3 T3 to T6 in full stays required until the spike passes, then the scenes are finished only as far as fixtures need. The VD3 side owns the fixture format. The pair stays the gate until the proof in LOAD_CONTRACT is done. `specs/proposals/TEST_GATE.md`.
- Control spine on real bots: `resume(scope)` and holds with owners (the person clears any hold, the commander only its own; K2 command set changes); retire waits for the outcome or the lease end, not 40 ticks; a start is a renew, and a bot may travel before its lease arrives; the lease is enforced at the adapter, where the packet is made (one new refusal reason); the gather core checks before a break that the cell supports no player it can see. `specs/proposals/CONTROL_SPINE_LIVE.md`. Also adopted: a temporary change in transit (pillar, bridge, door) uses a transit lease and carries a duty to restore; built with the survival core, not before `mine` (§5a).
- Sight mode and hidden state: sight mode is a view for every decision above the client (targets, hazards, paths, belief), where an unseen cell is unknown; hazards in sight mode have no look-ahead (worst-case posture, then react), which changes B4 §4.13 for sight mode; the pathfinder plans on the seen view, by a small fork patch; anti-x-ray is believed on by default, with evidence gathered at each exposure; a sighting needs a clear ray, not only exposure (the 2026-10-19 rule, kept). `specs/proposals/SIGHT_AND_BELIEF.md`.
- After a death (settles AD-B2.1): a death is always reported and marked by cause class, whatever the skill; a job resumes only when the skill says `Keep`, the mark does not cover the work, and it is the first death from that cause; recovery of drops passes four checks and never enters an active player quarantine; an unknown cause is treated as a player kill until someone looks; losses have a budget in the quote. `specs/proposals/AFTER_DEATH.md`.
- Commander harness (extends K3): the harness owns the loop and the model is a setting per kind of session; three kinds of session (wake, plan session as the commander in its own context, review off line); **AD-K2.1 closed:** the digest shows the standing state and writes are K2's verbs with a snapshot condition, no manifest; one wake at a time, and an empty heartbeat does not call the model; every write states an expectation that the gateway checks; the real spine on simulated bots is the test bed and the fake HQ is retired; the first build steps start after the swarm stack is pushed, beside `mine`. `specs/proposals/COMMANDER_HARNESS.md`.

## Decided 2026-11-04

- Swarm world (AD-F5.6): one `World` per bot, one process per machine, sections stored once by content, as git stores objects. The full comparison on a hash match stays until it really becomes the limiting factor. Second leak closed by this once built; `plans/SWARM_WORLD_PROBLEM.md`, `plans/LOAD_CONTRACT.md`.

## Decided 2026-10-25

- Shared-world bugs get a class test, not one-off audits: companion mode in the ability check (a second bot of the same swarm near the main bot in every row, both join orders) after fork patch 40. `specs/plans/COMPANION_BRIEF.md`.
- Vanilla diff (the same input into Null and a real vanilla client, packets compared per client tick) is planned on Allen's HeadlessMC + Fabric client, cleaned of mods that change behaviour; code in `harness/vanilla-diff/`, the tap may import `azalea-protocol` as a listed harness tool, Mac first, after S6 and before `mine`; the real-server login plugin is not needed. `specs/plans/VANILLA_DIFF_PLAN.md`.

## Decided 2026-10-24

- A14: a drop that lands on canopy leaves out of reach is reported as left, not chased (`gather` unchanged). The A14 log-count rows accept it only with server proof (item entity of that log on leaves, counts add up). B4 §4.9.
- Squad claim renewal: keep claims short (TTL 100 ticks, renew every 20); a renew with no reply in 2 s counts as failed and halts the bot, so an operation restart longer than about 2 s pauses its bots (they resume when the restarted op dispatches again). K6 §4.8, S3b `SquadConfig::renew_timeout`.

## Decided 2026-10-20

- Long suites (the full ability check on each server) are run by Allen offline from commands the agents give; no agent runs them. CLAUDE.md "Subagents".
- `gather` sees only its claim: a tree wholly outside is not reported; a tree across the edge is reported and left whole. B4 §4.10.

## Decided 2026-10-19

- AD-F1.2: the skill decides at its bot's death (`on_death`: End or Keep). F1 §4.4.
- AD-F5.3: tick catch-up like the vanilla client: at most 10 due ticks per update, the rest dropped (fork patch 23).
- AD-F5.4: action ids and a mailbox per owner for action results, pruned by owner release and end, not by time; a leak guard. B6 §4.6.
- AD-B2.1 working rule: after a death, go back to work or escalate. Open: recovery against quarantine, and whether a death is a hard interrupt.
- Live runs strictly once: the implementer runs the final full suite, the verifier checks its logs, the lead reruns nothing (CLAUDE.md).
- `gather` is the generic skill; a policy only filters blocks (`target`, `may_clear`); `fell` is the tree policy; `mine` digs by a method. B4 §3, §4.10.
- Tree policy: skip and report a log out of reach (no placing to reach it); break only natural leaves that block a log; no replant yet.
- Fork patch 25: settle every in-flight action by sequence, so instamine chains never lose a result.
- Sight mode (no x-ray) for anti-x-ray servers: a block is known only after a clear ray reached it; no light needed (shader packs); per bot by default, swarm-wide as an option; off for surface and bulk work. B4 §4.11, F2 §4.1 `seen_sight`.
- Mining methods as presets (fishbone, quarry, tunnel, later staircase and cave following). B4 §4.12.
- Mining handles underground hazards itself, placing blocks to seal fluids; no escalation. B4 §4.13.
- Commit each step after its verifier passes.

## Decided 2026-10-16

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

## Decided 2026-10-17

- **Fork vs adapter (Allen):** raw game capability (placing with results, item use, attackers, menus, pathfinder moves, the replay switch) goes in the Azalea fork as normal Azalea APIs; `mc-adapter` only translates to `BotView`/`BotCommands`. AD-F5.2 is unchanged (the boundary stands); the planned `mc-act` and `mc-nav` crates are dropped. F5 §4.1, ARCHITECTURE §9.1, CONVENTIONS updated.
- **Paper + GrimAC is the strict test server, timing included** (vanilla stays the target; see the evening entries below) (`harness/grim-server`, port 25568; your jars from `~/Documents/antigravity/focused-bardeen`). A bot never sends a click a vanilla client couldn't make.
- **Server-configurable timings are beliefs, not facts** (example: the shield's raise-to-block delay).
- **SA progress:** A1–A8 and every non-cube block shape family pass on Paper + Grim with zero alerts (about 200 rows), fork patches 5–10 (`specs/reference/AZALEA_PATCHES.md`). A9 done 2026-10-18 (fork patches 16–17); A10–A12 and the open items are in [HANDOFF.md](HANDOFF.md).
- **Reference sources (evening):** Minecraft client and server (Mojang names first, Yarn optional), Paper and Bukkit source, mineflayer, Meteor Client and Baritone (automation code to refer to), GrimAC; plan in `specs/plans/REFERENCE_SOURCES.md`. Fork patches then cite the vanilla method they reproduce.
- **Server targets (evening):** vanilla only (F5 §4.6); the Azalea-side code must also run cleanly on Paper with an anticheat. The ability check passes only on both test servers.
- **Grim server difficulty (evening):** normal.

## Decided 2026-10-15, in one line each

- AD-K5.2: facts as data from the game jar, observation and the LLM's prior; skill code only for cost-ranked important parts; goals to state and check tasks; the LLM plans steps with no model, in validated and quoted stretches (`submit_plan`). GENERALIZATION §0.
- AD-K5.1: no officers in v1. AD-K2.2: the LLM in full control; you hold halt and recall; permission checks on skills, set once (no attacking a player first; no breaking someone else's build, judged by "break only what is clearly natural").
- AD-F2.1: fixed tiles, operation service inside the node. AD-F2.2: separate belief store fed by a `chan` packet observer. AD-B1.1: reuse Azalea's pathfinder, default pool, shared section cache first.
- Builder: AD-C1.1 parallel across inert boundaries; AD-C1.2 restock on need; AD-B6.1 placement table plus probing; AD-F3.1 one server pool. Arming is not a separate action.
- Smaller: counter rules in F5; `carry` takes entity cargo; `quiet(zone)` job constraint; saved routines need no review; convoy right of way through quarantines; rescue a trapped bot only when K7 prices it above the expected losses.
