## Milestone verdict

**Pass: the machine is fully reproduced.** All 1882 blocks (1874 sections: 1850 single, 8 two-block, 16 bulk) pass
every checkpoint in one from-scratch, quasi-static linear run on the simulator. The final whole-region check
(all properties, including connection shape) differs at exactly one position, and that difference is cosmetic:

- `(6, 13, 9)` `redstone_wire[east=none,north=side,south=none,west=side]`: the build produces `east=side`, i.e. the
  dust is drawn connected to `observer[facing=east]` at `(7,13,9)`, whose output face points at it. Dust power is 0
  and an observer takes no input from dust, so it has no functional effect. The user has built this farm in vanilla
  many times. On the simulator, every sequence tried gives `east=side`: player placement of the dust before the
  observer, after it, and the observer pushed into place by a piston (as the duper piston at `(7,13,10)` does).
  The file's `east=none` is most likely an artifact of how the schematic was saved. Litematica's verifier would show
  this one block as a wrong state in a hand-built copy too. It is not an unbuildable state.

Simulator: Leaves 1.21.11 + the plansim plugin (user decision: counts as vanilla). Placement is the plugin's
`place` op (player-placement emulation). The lever is inverted (`powered=false` = TNT duper on, user decision).

## Spec assumptions

| Assumption | Verdict | Evidence |
|---|---|---|
| The real game is the only oracle (Principle 1) | **held** | Every edge comes from a sim experiment. The only offline knowledge is the support/attach seed and property classes. Analog edges (copies of a learned edge where the surrounding block ids match under a horizontal isometry) are hints that the linear run verified. |
| Knockouts give the steady-state graph; transient hazards come from corrections (2b/2c) | **held, but corrections dominate** | 43 knockout edges vs 175 correction edges (113 learned + 62 analog). Every real failure was transient: an observer firing, or a flow level that depends on order. |
| Hazards are local, failures shrink to a few nearby blocks (open question 1) | **partly broke** | Every failure shrank to a 1–4 block fix, but not always nearby. Placing a dropper or trapdoor watched by an observer fired the cobble pistons 6–9 blocks below, and the fix block was 4 away (the snow block at the bottom of the observer chain). One fix (`(2,11,1)` before observer `(1,11,1)`) needed the plan replayed from ~340 sections earlier. |
| Sections = SCCs of the dependency graph; feedback gives multi-block sections | **held, but SCCs were not the source of multi-block sections** | The knockout graph has no cycles once shape-only changes are ignored. All 8 two-block sections come from "group" corrections: b must come first, yet b only reaches its target once x is placed (a quasi-connectivity-powered `dropper[triggered=true]` updates only on a neighbour change; `note_block` / `iron_trapdoor` `powered=true`, `open=true`). |
| Upstream first, payload last (open question 6) | **broke for observers** | 13 irreversible failures and 1 replay fix: a block watched by an armed observer must be placed **before** the observer chain is complete (trapdoor/dropper before the snow block or observer that completes the chain). That is "watched before watcher", not upstream-first. |
| Static often means armed (Principle 3) | **held** | The observer chain at `(0..2, 9..11, 1)` is fully armed: removing any of its blocks fires the pistons, so the swap trial (remove x, place b, put x back) cannot test it. That needed an exact replay of the reordered plan. |
| Derived props land in their target state at placement | **broke (4 positions)** | The dropper `triggered=true` cases above; resolved with two-block sections, without arming actions. |
| History-dependent blocks need arming (knockout put-back test) | **over-reported** | Knockouts flagged 58 blocks as history-dependent (52 observers). The plan needed no arming or commissioning step: putting back an observer fires a pulse, which is not stored history. This bears on open question 3. |
| Nothing irreversible, quasi-static, one block per step | **held, with one semantics call** | Flowing lava between four lava sources (`(2,3,2)` among `(2,3,1)`, `(1,3,2)`, `(2,3,3)`, `(3,3,2)`) cannot be placed one source at a time without lava flowing into the other target cells. The checkpoint now treats the same liquid in an unplaced cell whose target is that liquid as allowed. This matches the existing flow-event allowance: the source poured there later overwrites it. **This is a deliberate relaxation of the invariant (the alternative is a temporary blocker).** |
| Connection shape is excluded from checkpoints and checked at the end | **held** | The final check caught the placement-fidelity gap (24 dust cross/dot and 6 activator rail shapes, fixed by treating them as player choices in `place`) and the one cosmetic dust shape above. |
| Simulated placement = `setBlock` (PLAN decision 4) | **broke as expected** | The default-state placement gave dot dust and north–south rails. Fixed: unconnected dust cross/dot and straight rail shape are now player choices (the target's shape is the starting state, then the game applies neighbour shapes). |
| Settle detection (quiet 20, cap 600) | **held** | No `no_settle` failures in the final run. |
| Snapshot/restore is the main engineering cost (open question 5) | **cheap in practice** | Region snapshots (19³) are fast. The loop keeps one every 50 sections, and copies settled states to 3 more servers by read → paste (verified block for block) to run experiments in parallel. A typical correction iteration went from ~6 min to 20–60 s. |

## How the plan was found

68 correction iterations over several loop runs (out/correct_log.jsonl). Learned edges by method: ddmin 83, swap 18,
single 4, replay 1, plus 62 analogs and 7 from the first agent's runs. Learned edges are in
out/edges_correction.json, and the plan is out/plan.json.

Reproduce: start the four servers (`./mc_serverN/start.sh`), then in `planner/` run
`uv run python correct.py cobble.litematic --workers 25581 25582 25583`
(it re-runs the plan from scratch and stops at the cosmetic final-check difference above).
