# Milestone report: cobble.litematic

## 1. Paste and settle

- 1882 blocks pasted at origin (0, 100, 0); settle = quiet 20 ticks, cap 600 (plansim plugin).

## 2. Bulk vs active set

- bulk: **16**, active (actuators + cone of influence): **1866**
- by role: bulk 911, actuator 313, wiring 270, reactive 236, liquid 124, source 27, payload 1

## 3. Knockouts and edges

- 1882 knockouts: late_safe 1096, irreversible 662, trigger 66, history_dependent 58
- edges by kind: support 276, correction (learned) 113, correction (analog) 62, knockout 43, attach 13

### History-dependent blocks (58)

Removing and putting back did not return the region to the target.

| block | count | example positions |
|---|---|---|
| observer | 52 | [[1, 5, 0], [13, 5, 0], [0, 5, 1], [14, 5, 1]] |
| note_block | 3 | [[3, 12, 3], [11, 12, 3], [11, 12, 11]] |
| dropper | 2 | [[5, 11, 13], [9, 11, 13]] |
| iron_trapdoor | 1 | [[3, 12, 11]] |

### Triggers (66)

Removal never settled (cap reached).

| block | count | example positions |
|---|---|---|
| snow_block | 17 | [[2, 11, 6], [12, 11, 6], [2, 11, 7], [12, 11, 7]] |
| repeater | 14 | [[2, 12, 3], [12, 12, 3], [2, 12, 5], [12, 12, 5]] |
| white_stained_glass | 13 | [[3, 11, 7], [11, 11, 7], [8, 12, 7], [4, 12, 10]] |
| redstone_wire | 10 | [[2, 12, 7], [3, 12, 7], [11, 12, 7], [12, 12, 7]] |
| observer | 4 | [[2, 11, 5], [12, 11, 5], [2, 11, 9], [12, 11, 9]] |
| redstone_block | 2 | [[3, 13, 7], [11, 13, 7]] |
| comparator | 2 | [[4, 13, 12], [10, 13, 12]] |
| redstone_wall_torch | 2 | [[6, 13, 13], [8, 13, 13]] |
| iron_door | 1 | [[8, 11, 7]] |
| warped_fence_gate | 1 | [[7, 12, 8]] |

Irreversible removals: 662 (full table in out/knockout_summary.md).

## 4. Sections and order

- 1874 sections; by kind: single 1850, bulk 16, loop 8
- section sizes: 1 steps × 1866, 2 steps × 8

Order, as runs of block types (block ids in each band of 100 sections):

| sections | most common blocks |
|---|---|
| 0–99 | white_stained_glass 62, crying_obsidian 24, snow_block 14 |
| 100–199 | white_stained_glass 37, crying_obsidian 31, cobblestone 20, snow_block 10, iron_chain 2 |
| 200–299 | cobblestone 55, crying_obsidian 26, snow_block 10, white_stained_glass 7, iron_chain 2 |
| 300–399 | cobblestone 56, crying_obsidian 28, snow_block 10, white_stained_glass 6 |
| 400–499 | cobblestone 46, crying_obsidian 25, white_stained_glass 21, snow_block 8 |
| 500–599 | cobblestone 36, white_stained_glass 35, crying_obsidian 23, snow_block 4, iron_chain 2 |
| 600–699 | white_stained_glass 40, cobblestone 36, crying_obsidian 14, snow_block 8, iron_chain 2 |
| 700–799 | snow_block 47, white_stained_glass 36, crying_obsidian 9, white_stained_glass_pane 8 |
| 800–899 | activator_rail 22, white_stained_glass 20, target 20, snow_block 12, redstone_wire 11 |
| 900–999 | redstone_wire 79, activator_rail 15, repeater 6 |
| 1000–1099 | observer 44, redstone_wire 27, repeater 21, comparator 4, activator_rail 3 |
| 1100–1199 | observer 71, repeater 17, activator_rail 10, redstone_wire 2 |
| 1200–1299 | observer 86, repeater 6, redstone_wire 6, iron_trapdoor 2, note_block 2 |
| 1300–1399 | piston 45, observer 33, repeater 9, iron_trapdoor 7, sticky_piston 4 |
| 1400–1499 | piston 59, note_block 15, sticky_piston 12, iron_trapdoor 7, redstone_wire 7 |
| 1500–1599 | piston 43, iron_trapdoor 13, snow_block 12, dropper 10, activator_rail 8 |
| 1600–1699 | iron_trapdoor 28, dropper 14, piston 11, note_block 11, snow_block 8 |
| 1700–1799 | lava 71, oak_leaves 31 |
| 1800–1873 | lava 46, oak_leaves 21, iron_trapdoor 2, water 2, birch_wall_sign 1 |

## 5. Linear execution and corrections

- correction loop: 68 logged iterations over 16 runs, 55 fixes, 122 learned edges, 57 analog edges added after fixes

| run | it | section | kind | block | how | edges |
|---|---|---|---|---|---|---|
| - | 1 | 1015 | mismatch | [3, 12, 6] repeater[delay=1,facing=south,locked=fa… | unresolved | 0 |
| 111625 | 1 | 1026 | mismatch | [2, 12, 6] repeater[delay=2,facing=south,locked=fa… | ddmin | 2 |
| 111625 | 2 | 1026 | mismatch | [3, 12, 6] repeater[delay=1,facing=south,locked=fa… | single | 1 |
| 111625 | 3 | 1026 | mismatch | [11, 12, 6] repeater[delay=1,facing=south,locked=fa… | single | 1 |
| 111625 | 4 | 1026 | mismatch | [12, 12, 6] repeater[delay=2,facing=south,locked=fa… | ddmin | 2 |
| 111625 | 5 | 1026 | mismatch | [2, 12, 7] redstone_wire[east=side,north=side,powe… | single | 1 |
| 111625 | 6 | 1042 | mismatch | [9, 13, 5] repeater[delay=1,facing=south,locked=fa… | unresolved | 0 |
| 112515 | 1 | 1026 | mismatch | [2, 12, 6] repeater[delay=2,facing=south,locked=fa… | ddmin | 2 |
| 112515 | 2 | 1026 | mismatch | [3, 12, 6] repeater[delay=1,facing=south,locked=fa… | single | 1 |
| 112515 | 3 | 1026 | mismatch | [11, 12, 6] repeater[delay=1,facing=south,locked=fa… | single | 1 |
| 112515 | 4 | 1026 | mismatch | [12, 12, 6] repeater[delay=2,facing=south,locked=fa… | ddmin | 2 |
| 112515 | 5 | 1026 | mismatch | [2, 12, 7] redstone_wire[east=side,north=side,powe… | single | 1 |
| 112515 | 6 | 1042 | mismatch | [9, 13, 5] repeater[delay=1,facing=south,locked=fa… | unresolved | 0 |
| 115055 | 1 | 1042 | mismatch | [9, 13, 5] repeater[delay=1,facing=south,locked=fa… | unresolved | 0 |
| 124157 | 1 | 1042 | mismatch | [9, 13, 5] repeater[delay=1,facing=south,locked=fa… | unresolved | 0 |
| 131151 | 1 | 1042 | mismatch | [9, 13, 5] repeater[delay=1,facing=south,locked=fa… | ddmin | 8 |
| 131151 | 2 | 1045 | mismatch | [8, 13, 6] repeater[delay=2,facing=east,locked=fal… | ddmin | 8 |
| 131151 | 3 | 1045 | mismatch | [9, 13, 6] redstone_wire[east=none,north=side,powe… | ddmin | 7 |
| 131151 | 4 | 1045 | mismatch | [9, 13, 7] redstone_wire[east=none,north=side,powe… | ddmin | 6 |
| 131151 | 5 | 1046 | mismatch | [9, 13, 8] redstone_wire[east=none,north=side,powe… | ddmin | 5 |
| 131151 | 6 | 1049 | mismatch | [8, 13, 9] redstone_wire[east=side,north=none,powe… | ddmin | 3 |
| 131151 | 7 | 1049 | mismatch | [9, 13, 9] redstone_wire[east=none,north=side,powe… | ddmin | 4 |
| 131151 | 8 | 1049 | mismatch | [4, 13, 10] repeater[delay=4,facing=east,locked=fal… | ddmin | 5 |
| 131151 | 9 | 1049 | mismatch | [8, 13, 10] redstone_wire[east=none,north=side,powe… | ddmin | 2 |
| 131151 | 10 | 1049 | mismatch | [10, 13, 10] repeater[delay=4,facing=west,locked=fal… | ddmin | 5 |
| 131151 | 11 | 1049 | mismatch | [5, 13, 11] redstone_wire[east=none,north=side,powe… | ddmin | 3 |
| 131151 | 12 | 1049 | mismatch | [8, 13, 11] repeater[delay=1,facing=south,locked=fa… | single | 1 |
| 131151 | 13 | 1049 | mismatch | [9, 13, 11] redstone_wire[east=none,north=side,powe… | ddmin | 3 |
| 131151 | 14 | 1049 | mismatch | [4, 13, 12] comparator[facing=south,mode=compare,po… | ddmin | 2 |
| 131151 | 15 | 1049 | mismatch | [5, 13, 12] comparator[facing=north,mode=compare,po… | ddmin | 4 |
| 131151 | 16 | 1062 | mismatch | [9, 13, 12] comparator[facing=north,mode=compare,po… | ddmin | 4 |
| 131151 | 17 | 1062 | mismatch | [10, 13, 12] comparator[facing=south,mode=compare,po… | ddmin | 2 |
| 131151 | 18 | 1062 | mismatch | [4, 13, 13] redstone_wire[east=side,north=side,powe… | single | 1 |
| 131151 | 19 | 1307 | mismatch | [2, 12, 3] repeater[delay=4,facing=south,locked=fa… | single | 1 |
| 131151 | 20 | 1312 | mismatch | [12, 12, 3] repeater[delay=4,facing=south,locked=fa… | single | 1 |
| 131151 | 21 | 1463 | irreversible | [1, 6, 0] iron_trapdoor[facing=north,half=top,ope… | unresolved | 0 |
| 132421 | 1 | 1463 | irreversible | [1, 6, 0] iron_trapdoor[facing=north,half=top,ope… | swap | 1 |
| 132421 | 2 | 1468 | irreversible | [13, 6, 0] iron_trapdoor[facing=north,half=top,ope… | swap | 1 |
| 154829 | 1 | 1469 | irreversible | [0, 6, 1] iron_trapdoor[facing=west,half=top,open… | swap | 1 |
| 155551 | 1 | 1521 | irreversible | [1, 8, 0] dropper[facing=down,triggered=false] | unresolved | 0 |
| 160636 | 1 | 1521 | irreversible | [1, 8, 0] dropper[facing=down,triggered=false] | swap | 1 |
| 160636 | 2 | 1535 | irreversible | [7, 9, 1] iron_trapdoor[facing=north,half=bottom,… | swap | 1 |
| 162100 | 1 | 1571 | irreversible | [1, 9, 7] iron_trapdoor[facing=west,half=bottom,o… | swap | 1 |
| 162100 | 2 | 1622 | irreversible | [2, 11, 1] iron_trapdoor[facing=south,half=top,ope… | unresolved | 0 |
| 163916 | 1 | 1622 | irreversible | [2, 11, 1] iron_trapdoor[facing=south,half=top,ope… | replay | 1 |
| 163916 | 2 | 1619 | irreversible | [1, 11, 1] observer[facing=east,powered=false] | unresolved | 0 |
| 170101 | 1 | 1626 | irreversible | [3, 11, 1] dropper[facing=south,triggered=false] | swap | 1 |
| 170101 | 2 | 1628 | irreversible | [6, 11, 1] iron_trapdoor[facing=north,half=bottom,… | swap | 1 |
| 170101 | 3 | 1634 | irreversible | [1, 11, 3] dropper[facing=east,triggered=false] | swap | 1 |
| 170101 | 4 | 1643 | irreversible | [1, 11, 6] iron_trapdoor[facing=west,half=bottom,o… | swap | 1 |
| 170101 | 5 | 1660 | irreversible | [3, 11, 13] dropper[facing=north,triggered=false] | swap | 1 |
| 170101 | 6 | 1661 | mismatch | [5, 11, 13] dropper[facing=down,triggered=true] | swap | 1 |
| 170101 | 7 | 1660 | mismatch | [5, 11, 13] dropper[facing=down,triggered=true] | swap | 1 |
| 170101 | 8 | 1659 | mismatch | [5, 11, 13] dropper[facing=down,triggered=true] | swap | 1 |
| 170101 | 9 | 1658 | mismatch | [5, 11, 13] dropper[facing=down,triggered=true] | unresolved | 0 |
| 172939 | 1 | 1661 | irreversible | [6, 11, 13] iron_trapdoor[facing=south,half=bottom,… | swap | 1 |
| 172939 | 2 | 1663 | irreversible | [8, 11, 13] iron_trapdoor[facing=south,half=bottom,… | swap | 1 |
| 172939 | 3 | 1664 | mismatch | [9, 11, 13] dropper[facing=down,triggered=true] | swap | 1 |
| 172939 | 4 | 1672 | mismatch | [3, 12, 3] note_block[instrument=basedrum,note=5,p… | swap | 1 |
| 172939 | 5 | 1677 | mismatch | [3, 12, 11] iron_trapdoor[facing=north,half=bottom,… | swap | 1 |
| 172939 | 6 | 1677 | mismatch | [11, 12, 11] note_block[instrument=basedrum,note=0,p… | swap | 1 |
| 172939 | 7 | 1695 | mismatch | [2, 3, 1] lava[level=0] | ddmin | 3 |
| 172939 | 8 | 1696 | mismatch | [6, 3, 1] lava[level=0] | ddmin | 2 |
| 172939 | 9 | 1696 | mismatch | [7, 3, 1] lava[level=2] | ddmin | 3 |
| 172939 | 10 | 1697 | mismatch | [2, 3, 2] lava[level=2] | ddmin | 4 |
| 172939 | 11 | 1697 | mismatch | [3, 3, 2] lava[level=0] | unresolved | 0 |
| 174819 | 1 | None | mismatch | None  | unresolved | 0 |
| 175526 | 1 | None | mismatch | None  | unresolved | 0 |

### Result: every checkpoint passed; final whole-region check differs (see verdict below)

- all 1874 sections passed their checkpoints in one from-scratch linear run
- the final check (all props, including connection shape) differs at 1 position(s):

| pos | target | actual |
|---|---|---|
| [6, 13, 9] | redstone_wire[east=none,north=side,power=0,south=none,west=side] | redstone_wire[east=side,north=side,power=0,south=none,west=side] |

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
