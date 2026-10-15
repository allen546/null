# World transformation: the builder, generalized

> **Status (2026-10-14): proposal, in design with Allen.** Replaces the role of [ONLINE_BUILDER.md](../ONLINE_BUILDER.md) v4.1, which Allen now treats as a specialized attempt that won't generalize. Nothing here is measured. Game facts from memory are marked `[c]`. Decisions Allen made in the design session are marked **(Allen, 2026-10-14)**; everything else is proposed. The tests in §8 are recorded, not started.

## 1. Why v4.1 is replaced

- **Rules grow per schematic.** F7 runs 2 to 10 each stopped on one new problem, and each fix added a named rule in planner code: K1, K2, K3, M1, support temps, inside access ([logs/builder/online/PROGRESS.md](../logs/builder/online/PROGRESS.md)). This is the loop [INTERRUPTS.md](../INTERRUPTS.md) §1 says doesn't finish.
- **Its certificate needs op.** v4.1 assumes creative, `/forceload`, `/data get` and a server-side copy of the margin (§0 S1, S5). Null's servers are pure vanilla survival with no op.
- **It can't simulate the server.** Bots can't read hidden state (pending ticks, container contents, entity NBT, RNG), can't see unloaded chunks, and the rest of the server keeps acting. A copy of the server is not available. **(Allen, 2026-10-14)**
- **A simulator can't be on the build path.** A commercial CPU doesn't run a server past about 1k TPS, so a simulator is at most about 50× real time. Per-step trials for a swarm building in parallel are orders of magnitude out of reach. **(Allen, 2026-10-14)**
- **It is a separate stack for one bot.** A JavaScript client replays packets open-loop, shares nothing with B1/B6 on Azalea, F2 or F3, and has no answer for many bots.

Kept from v4.1: the guarantee as the product, arming last, and reports instead of retries on live.

## 2. The problem

> **World transformation.** Given a region, its observed state, a target, and enough items, bots and time: produce a plan that turns the observed state into the target. No step may make a lasting change the target doesn't allow. If no plan is found, report why. **(Allen, 2026-10-14: the problem covers blocks and entities, and isn't limited to the builder.)**

State is Minecraft's own data model, which is closed per version (F5):

- block states, per cell;
- block-entity NBT: container contents, sign text, comparator output;
- entities: type, position and NBT (boats, villagers, item frames and their items, minecarts).

Each element of the region is one of three kinds:

| kind | meaning |
|---|---|
| target | must end as stated, within its tolerance |
| free | anything may happen during the work; must end as stated (usually air) |
| protected | must never change |

This generalizes v4.1's build region, work envelope and margin. Candidate jobs that share the problem: building (C1), site prep (C2), entity installs (B7, C9), repair after damage (the target is the last good state), demolition (the target is air). Which of them merge into one service is open (§7).

## 3. Where targets come from

- **Design stays outside Null.** A target comes from a pack (litematics plus a descriptor, C11) or from a generator. A generator is a tool such as `generate_wall(coordinates, height, material)`, which an LLM can write on demand. Null builds targets; it doesn't design them. **(Allen, 2026-10-14)**
- **Snapshots are stable.** A person passes a snapshot only after the machine has settled, for example a TNT duper that is powered but inactive. Null never receives a contraption captured mid-move. Stable doesn't mean unpowered: the target may hold powered components. **(Allen, 2026-10-14)**
- **A snapshot alone can't express every target.** OSC's chunk-loader boats are entities in the nether piece, and they can only be placed once the nether side exists. The descriptor (§4) carries what the snapshot can't.

## 4. The descriptor

No field per situation. The target is written in Minecraft's data model (§2), plus three closed additions:

| addition | what it says | example |
|---|---|---|
| tolerance per element | which parts of a state or NBT must match, and how: exact, any, range, or one of a set (a matcher on NBT paths, as `/execute if data` uses) | boat: type exact, position ±0.5 in cell (x,y,z), other NBT don't care |
| relations between pieces | relative placement, including across dimensions | nether piece at overworld origin ÷ 8, ±1; control panel at offset (dx,dy,dz) from the cannon |
| ports and arming | cells that may couple to the outside, and steps done last | the portal cells; the circuit break |

Default tolerance: exact block state, entity in its cell, random NBT (UUIDs, health, trades) don't care. A new situation adds values to these matchers, not fields. This keeps the schema inside the budget rule of [GENERALIZATION.md](GENERALIZATION.md) §1.

## 5. Order

### 5.1 Order classes

| class | example | planner |
|---|---|---|
| printable | WorldEater v5.0: any order that obeys support and physics works, and it can't fly away | any order; many bots in parallel are safe, since every interleaving is another order |
| printable while cold | OSC with the arming break open | as printable; arming last |
| order-sensitive | a build that fires during construction | ordering from the rules (§5.2) and the pack; if no order is found, the pack is refused at load with the conflict attached |

### 5.2 A condensed rule set

**Knowledge lives in data plus learned constraints, not planner code. (Allen, 2026-10-14)** Each rule is a pattern plus an ordering constraint, stored as a data row, so the set can grow. A new rule is reviewed by a person, as C1 §4.9 requires for hazard rules.

Principle: only **actuators** make lasting changes (pistons, dispensers, droppers, TNT, hoppers moving items, rails with carts, fluids). **Signal blocks** (wire, repeaters, comparators, observers, torches) carry state that settles, and a transient on them heals itself (v4.1's K2). So the order follows signal flow: each component goes after its inputs, including what it observes, and before its outputs.

| # | pattern | constraint |
|---|---|---|
| R1 | signal flow A → B (power, observation, comparator reading) | A before B: the topological order of the flow graph |
| R2 | actuator | last in its cluster, placed only when its inputs are at their target state |
| R3 | observer watching cell X | X before the observer, including X's final shape; X's neighbours come first too, since wire and fence connections change X's state `[c]` |
| R4 | QC-capable block (piston, dispenser, dropper) with power reaching its QC zone (the cell above and that cell's neighbours) | the actuator before whatever powers its QC zone, or the zone proven unpowered; later power there doesn't update the actuator and leaves a hidden BUD state `[c]` |
| R5 | cycle in the flow graph (clock, loop) | a break cell left out until arming |
| R6 | update-sensitive blocks (gravity blocks, fluids, rails, BUD-able blocks) | support and neighbours first; fluids last in their cluster |
| R7 | anything at or through a port (portal, link to another piece) | both sides complete first; it is an arming step |

R7 covers OSC's boats without the author stating the relation: the author marks the portal as a port, and portal blocks could be found automatically.

Known weak spots, which need their own rules or a check: Java's location-dependent wire update order, zero-tick pulses, piston timing, comparator subtract mode, and sculk at range. Reaching a stable target that holds power (the powered, inactive duper) may need actuators placed in a specific state context; R2 handles the simple cases and the tests (§8) show the rest.

### 5.3 Access and support are separate

Not locking a bot out of a ring, temps and support are geometry, the same for every build, and belong to the generic planner, not to the pack. Most of F7's stops were this kind, not redstone order: K1 (a temp pillar left unreachable), M1 (a closed ring kept the bot out), and "1413 of 1881 place steps are never protected" by an access witness ([logs/builder/online/PROGRESS.md](../logs/builder/online/PROGRESS.md)).

**Proposed (Allen, 2026-10-15: adopt, and devise something else if it proves unsafe for large technical builds):** access is worked out at load, never discovered on live.

1. **Placement options.** For each target cell, list every option: a stand cell, a clicked face and a look that satisfy the cell's recipe (§5.5), within reach (4.5 from the eye, B6 §4.1) and with line of sight.
2. **Blocking becomes order.** If every option of cell X is blocked by cell Y (Y fills the stand, the line of sight, or the path to the stand), then X goes before Y. With support (the clicked neighbour before X) and the rules of §5.2, these constraints give one topological order.
3. **Default shape: strict layers.** The base order is bottom-up by layer, with a barrier: a layer is finished and verified before work goes up. Bots have no flight in survival, so the layer order alone doesn't give access.
4. **Temporary stands.** A cycle in the constraints is broken by a temp in a free cell. The preferred temp is a scaffolding column `[c]`: bots climb it, and breaking its lowest block drops the whole column, so it leaves nothing behind. Every temp is recorded as Null's own, with its removal step and its own placement options, so cleanup touches only blocks Null placed.
5. **Refuse at load.** If a cycle remains after temps, the target is refused before any bot moves, and the report names the cells.

On live, the only choice is between precomputed options, for example when a mob or a player stands in the chosen stand. Open: whether the path checks are fast enough on a 20,000-block build (v4.1's access check took 55% of its planner CPU). The tests in §8 measure it.

### 5.4 Other sources of order

- **Community printability.** Large technical builds are often designed to be printable, because people paste them with Litematica's printer.
- **Videos, for relations only.** Coordinates are hard to read from a video, but the schematic already has them. A transcript gives ordering statements ("build it with the lever off"), and grounding them onto schematic cells is a small matching problem.
- **One check per pack, optional.** One simulator replay per pack per version fits the 1k TPS ceiling, because it happens once, not per build. Whether Null checks a pack's claims or trusts the author (C11 §4.6) is open (§7).

### 5.5 Placement accuracy (proposed, Allen, 2026-10-15)

A placement is right when the block ends in its target state. Null has no creative `block_state` items, no debug stick and no `/tick freeze`, so state comes only from the click.

- **Recipes come from the game.** AD-B6.1's table is made by probing the real jar on a private server: for each block, every face, look octant, hit half and sneak setting is tried and the resulting state recorded. The table is data per version. States the table lacks are probed on demand. No facing rule is written by hand.
- **Two kinds of property.** Properties set by the click (facing, axis, half, slab type, door hinge) are checked at placement. Properties derived from neighbours (wire and fence connections, stair shape, power) are checked only when their cluster is complete. v4.1 mixed the two.
- **Follow-up clicks** set repeater delay, comparator mode and similar properties (B6 §4.3).
- **Readback.** After every placement the bot waits for the server's acknowledgement and reads the state back (B6 §4.6). A cell counts as done only after readback.
- **No recipe** for a target state (B6 §4.9): the target is refused at load with the cell named.

### 5.6 When a step fails (proposed, Allen, 2026-10-15)

Arming last (§5.1, §5.2 R5, R7) is what makes failure cheap: until a cluster is armed it is cold, so a wrong block is inert and can be broken and placed again.

| failure | response |
|---|---|
| readback shows a wrong click-set property, before arming | break the block and place it again; after 2 failures of the same recipe, mark the recipe row suspect and set the step aside |
| the bot can't place (no free stand, items missing, the server refuses) | wait, or restock on need (AD-C1.2); then set aside the step and every step that depends on it, finish the rest, and report (INTERRUPTS, builder) |
| a change outside the step's expected changes (a rule missed an interaction) | stop that cluster, don't arm it, and report with the difference |
| a failure after arming | an interrupt in a running machine; the repair is a new transformation whose target is the last good state |
| the same failure recurs | the batch review fixes data: the recipe row is probed again, or a rule row is added (§5.2). Never a code rule for one schematic |

Null can't roll back a live world. The cold-until-armed state and the set-aside rule take that role.

## 6. Isolation and the guarantee (proposed)

The simulator, where one is used at all, only has to be right inside a region if nothing outside can reach the work. So the first job of a plan is an inert shell around the region. Coupling channels and their cuts, all `[c]`:

| channel | reach | cut by |
|---|---|---|
| redstone power, quasi-connectivity | 1–2 blocks | a gap or non-conductive blocks |
| block updates, observers | 1 block, chained | a gap with no observer or updater across it |
| comparators reading containers | 1–2 blocks | no containers in the shell |
| pistons pushing | up to 12 blocks | no piston whose push crosses the shell |
| fluids | flow length | a dry shell |
| sculk vibrations | up to 16 blocks | wool, or distance |
| entities | anywhere | fences, light, a closed shell |
| explosions, fire | radius | blast-resistant blocks, nothing flammable |

Time of day, weather and RNG reach everything; they are variants, not cuts.

- **Ports** are the declared exceptions (§4), opened last.
- **Stable step boundaries.** After every finished step the bots can leave and nothing more changes. Actions that can't be split (a mob on a lead, a cart in motion) are one atomic step. Interrupts, upkeep and other bots then only ever meet a stable state.
- **The guarantee, stated honestly:** if the shell holds and global conditions stay within the variants, the end state meets the target and nothing outside what it allows has changed. Bots watch the shell on live; any change there is an interrupt (INTERRUPTS §2).
- **Parallel work** uses the same tool: two regions with an inert boundary can be built at once (AD-C1.1).

## 7. Decided and open

Decided **(Allen, 2026-10-14)**: v4.1 is replaced; the problem is world transformation over blocks and entities; no full-server simulation and no simulator on the build path; design stays outside Null; knowledge as data plus learned constraints; snapshots are stable; the execution model (packet replay, skills, or a hybrid) is deferred.

Open:

1. Which jobs merge into the one service (C1, C2, entity installs, repair, demolition).
2. The isolation shell and ports (§6) as the basis of the guarantee.
3. Trust an author's printability or arming-break claim, or check it once per pack per version.
4. Refuse an order-sensitive pack that no source can order, or allow some search at build time.
5. ~~AD-C1.1, AD-C1.2, AD-B6.1 and AD-F3.1 need answers in this frame.~~ Decided (Allen, 2026-10-15): C1.1, parallel work across inert boundaries (§6) and any interleaving in printable regions; C1.2, a builder bot restocks on need; B6.1, a table for common states and probing for the rest; F3.1, one server pool. See C1, B6 and F3.

Items 1 to 4 are deferred (Allen, 2026-10-15).
6. Whether access by placement options, strict layers and scaffolding temps (§5.3) holds for large technical builds. If it proves unsafe, devise something else (Allen, 2026-10-15). The §8 access tests decide.

## 8. Tests (recorded, not started)

Each test runs the rule set (§5.2) over a real schematic from the builder archive (`null-builder-archive-2026-10-14`) and reports the order produced, the cycles found, and the cells no rule covers. Where a test says so, the order is then replayed once on an archived simulator.

| test | schematic | expected | prerequisites |
|---|---|---|---|
| T-WE WorldEater | `WorldEaterV5.0-2243.Generated-270x80x274(9).litematic` (20,083 blocks) | printable: almost nothing unresolved | **special setup (Allen, 2026-10-14):** the anvil trenches must be dug, and the full 270×80×274 footprint must exist as a site, before any replay |
| T-OSC Orbital Strike Cannon | `The Orbital Strike Cannon Mk.6.1.litematic`, `OSC 6 Control Panel.litematic`, `OSC 6 Nether.litematic` | printable while cold; R7 puts the chunk-loader boats after the nether side | the three pieces' relations written as a descriptor (§4); the portal cells marked as ports |
| T-ACC access (§5.3) | F7 and WorldEater | every cell has an option at its place in the order, or the load refuses with named cells; time to compute | none |
| T-COB cobble farm | `planner/cobble.litematic`, `2025.7.1 515k cobblestone farm.litematic` | a stable target that holds power: the TNT duper's lever is inverted, `powered=false` means charged | none beyond the archived sims |

## 9. Relation to other documents

- [ONLINE_BUILDER.md](../ONLINE_BUILDER.md) v4.1 and its prototype stay as a record of what was learned; this proposal replaces their role once adopted.
- [reference/BUILDER_ALTERNATIVES.md](../reference/BUILDER_ALTERNATIVES.md) is input: its cold cut (§D) is §5.2's core, and its DAG for many bots (§I) fits §6.
- [C1-builder.md](../C1-builder.md) keeps its interface until this is adopted; its request would take the descriptor (§4) instead of a single schematic.
