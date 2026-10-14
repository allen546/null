# Universal schematic builder: the hard cases

Research notes, 2026-10-13. Scope: (1) placing blocks so redstone and update-sensitive blocks don't fire, pop or take the wrong state during the build, and (2) getting entities into place. This does not assume anything about Allen's prototype code, which this thread hasn't seen.

It does build on `ONLINE_BUILDER.md` (design v4.1) in the project folder. Where that design already answers something, this file says so instead of re-deriving it.

## 0. Short version

1. **Nobody has solved update-safe ordering for legit (survival, client-side) building.** Every existing tool does one of two things. Either it suppresses updates server-side (commands, `strict` setblock, mods), or it places by type-based stages and accepts some breakage. Printers mostly ignore the problem and leave it to the player.
2. **How hard this is depends on what the server allows.** Pick the tier per server, because the difficulty changes by an order of magnitude:
   - **Tier A, op or commands:** `/setblock … strict` (vanilla 1.21.5+) or Carpet `fillUpdates false` removes most ordering hazards. What remains is that the finished build must be *update-stable*.
   - **Tier B, an accurate-placement server** (Carpet `accurateBlockPlacement`, or a Litematica easy-place protocol handler): any block state can be placed directly. Only ordering is hard.
   - **Tier C, vanilla SMP survival:** state comes only from look direction, clicked face, hit point and sneaking, plus follow-up clicks. Ordering is hard, some states are unreachable, and anti-cheat limits apply. Faction SMP is this tier.
   - **The project's constraint** (`ARCHITECTURE.md`: live survival servers, no op, no creative, no `/forceload`) rules out tier A on live. Tier B is available only where the server owner happens to run such a handler. So **tier C is the case to design for.** Tiers A and B remain useful in the offline simulator and for testing.
3. **The core method is "build cold, arm last".** Build everything with the circuit open. Leave a small *arming set* of cells (dust, a repeater, a power source) unplaced. Close them last, in an order checked by simulation. Most redstone builds become tractable this way without full search.
4. **Treat entities as their own late phase.** They are actions that can change redstone too: plates, detector rails, comparators reading frames. Accept them with tolerances, not exact equality. Mobs, villagers especially, can't be placed exactly in survival. They are a transport-and-confine problem with retries, and they can't be certified the way blocks can.
5. **v4.1's simulator-certified search is the right backbone.** Two additions would help it most. A cheap static pre-pass, classifying every cell and compiling its placement recipe, gives the search good orderings so it rarely needs repair. And the entity phase needs designing; v4.1 currently pushes all of it into hand-written macros.

## 1. Survey of existing approaches

| tool | how it places | what it does about updates and redstone | entities |
|---|---|---|---|
| **Litematica paste** (creative) | Single player writes the world directly. On a server it sends `/setblock`, `/fill` and `/summon` commands. | Direct writes skip most updates. Over commands it depends on the server; `strict` (1.21.5+) skips block and shape updates. | Pasted from the schematic's entity NBT with `/summon`. |
| **Litematica Easy Place** | The player clicks; the mod picks the item, face and look so the placed state matches the schematic. | Ordering is up to the player. With an accurate-placement protocol on the server, the hit vector carries the full block state (facing, half, type, …), so state is exact but order still isn't. | None. |
| **Printer forks** (aleksilassila, eatmyvenom, IceTank, "hana", "4th", INPRIM, and a "redstone-aware Easyplace extension") | Easy Place automated: places every reachable mismatched block around the player each tick. | aleksilassila's README lists liquids, mid-air placement and some rails as unsupported, and refuses repair or excavation. The redstone-aware extension claims redstone awareness but publishes no mechanism (§7). | None found. |
| **Baritone `BuilderProcess`** | Pathfinds to goals adjacent to wrong cells. Lower Y is favored, with optional `buildInLayers`. A block is skipped while the one below it is still placeable. `approxPlaceable()` simulates `getStateForPlacement` with a planned yaw and pitch to pick an item and look that gives the right state. | None for redstone. Leniency settings (`buildIgnoreDirection`, `buildIgnoreProperties`, substitutes) let you skip states it can't reach. | None. |
| **mineflayer-builder** (PrismarineJS) | Its README says it's a work in progress and not usable yet. It needs a creative op bot. | None. | None. |
| **WorldEdit `MultiStageReorder`** (deprecated) | Buffers writes and commits them in fixed stages by block type. FIRST: everything else. LATE: fluids, sand, gravel. LAST: attached blocks, redstone, rails, pistons, plates. FINAL: doors, signs, banners, cactus, cane, piston heads. | Purely type-based, with no facing or attachment logic. It's the classic heuristic, and it's what a naive order should beat. It was replaced by side-effect control (`SideEffectExtent`: neighbours, updates, lighting, validation toggled per edit). | Copies entity NBT. |
| **RedstoneAwareEdits** (mod) | Suppresses neighbour, placement and shape updates during bulk edits (≥10 changes in a tick, or manual), then recomputes redstone states and runs one clean update pass. | This is "suppress then settle" as a server mod. | None. |
| **Vanilla `/setblock`, `/fill`, `/clone` `strict`** (Java 1.21.5, snapshot 25w02a) | Commands. | Per the wiki, places "as-is without triggering block updates and shape updates". | `/summon` separately. |
| **Structure blocks and templates** | Server-side. | Places with limited update flags, then fixes shapes. Same family as paste. | Optional entities. |
| **Papers** (APT, arXiv 2411.17255; MineCEraft, arXiv 2608.28884; Builder Dialogue benchmark, arXiv 2407.12734; LLM-as-Minecraft-agents, arXiv 2402.08392) | LLM planning of what to build, mostly in simple blocks and often via commands. | None of them deal with update-safe ordering or entity placement, going by their abstracts. | None. |

**Closest relevant theory.** Robotic *assembly sequence planning*: precedence graphs, disassembly-based planning (plan the teardown, then reverse it), and AND/OR graphs. v4.1's "wrap" and "backtrack" are close cousins. Disassembly planning is worth a look. Removing a block from a finished, inert machine is often easier to reason about than adding one, because the final state is known, and reversing a safe teardown gives a candidate build order. It isn't automatically safe in reverse, because updates aren't symmetric, so each step still needs a trial.

**Fast redstone simulation.** MCHPRS (a Rust redstone server with a compiled "redpiler" backend) shows that circuit behaviour can be simulated far faster than a full server. It could serve as a pre-filter ahead of v4.1's S1 trials, but the full server stays the judge.

## 2. What goes wrong: a hazard taxonomy

Every cell in a schematic falls into one or more of these classes. The pre-pass (§3.1) labels them.

| class | examples | failure if mis-ordered |
|---|---|---|
| **Support** | torches, buttons, levers, rails, carpets, dust, plates, signs, banners, lanterns, bells, cane, cactus (also pops if a side neighbour is solid), kelp, vines, chorus, dripleaf, amethyst buds | pops into an item, or can't be placed at all |
| **Gravity** | sand, gravel, concrete powder, anvils, scaffolding (distance), pointed dripstone, dragon egg | falls |
| **Derived shape** | stairs shape, fences, walls, panes, iron bars, chest pairing, wire connections and dot/cross, rail shape, tripwire, note-block instrument (from the block below), mushroom-block faces, `snowy`, door halves, beds | wrong final state, or a wrong neighbour later "locks in" (rails, chests) |
| **Actuator** (reacts to updates or power) | observers, pistons and sticky pistons (including quasi-connectivity and BUD), droppers, dispensers and crafters (QC), powered and activator rails, TNT, note blocks, doors, trapdoors and gates, bells, lamps | fires mid-build, pushes or moves blocks, spends items |
| **Memory** (state persists after the cause) | piston extended, repeater lock, copper bulb (toggles on each rising edge), rail shape, container contents, lectern page, note pitch, crafter slots, T flip-flops | silently wrong end state that a later update won't fix |
| **Sensor** | sculk and calibrated sculk (vibrations from *any* placement or break within range), tripwire, plates, detector rails, target blocks, daylight detectors, comparators on containers or frames | spurious pulse into the circuit |
| **Fluid** | water, lava, waterlogging, cobble, stone and obsidian generation, concrete powder hardening, sponges, bubble columns, coral needing water, farmland | floods, converts blocks, breaks attached blocks |
| **Ticking** (scheduled or random) | leaves (decay; player-placed leaves get `persistent=true`), crops, grass and mycelium spread, ice and snow melting, copper oxidising, coral dying, saplings, budding amethyst, fire | changes after placement without anyone acting |
| **Item movement** | hoppers, hopper minecarts, droppers into containers | moves items and changes comparator signal strengths |
| **Unreachable by placement** | extended pistons with no power, budded pistons, lit lamps with no power, states only reachable through update tricks, block 36, illegal or "floating" states saved by a mod | can't be built by any ordering of placements |

Note on observers: whether an observer pulses when it is itself placed or moved differs between versions and editions, and between player placement and pistons. Don't hard-code it. Read it from simulation in the target version.

## 3. Strategies for blocks

### 3.1 Static pre-pass (cheap, no simulator)

Before any search, compile the schematic into:

1. **Per-cell labels** from §2.
2. **A precedence graph**:
   - *hard* edges for physics (support, gravity, the block below a note block, a door's lower half);
   - *soft* edges for hazards. For example, "a cell watched by an observer must be final before the observer goes in", "a piston's push column before its power source", "containers filled before the comparator reading them is connected".
3. **A placement recipe per cell** for the server's tier: item, clicked face, look yaw and pitch, hit point (top or bottom half), sneaking, and follow-up clicks (repeater delay, comparator mode, note pitch, dot/cross toggle, door or trapdoor open, item-frame rotation, sign text, container contents). Compute it the way Baritone's `approxPlaceable()` does: run the game's own `getStateForPlacement` against the planned look and neighbours, instead of hand-coding facing rules. Facing conventions (piston versus observer versus hopper) are easy to get backwards.
4. **An unreachable list**: cells with no recipe in this tier. These go into a report *before* the build starts, with suggested options: a macro, a substitution the user approves, or "needs a higher tier".
5. **A hazard neighbourhood per cell**: power reach, QC reach, observer watch cells, sculk range, fluid flood-fill.

This is what v4.1 calls the hazard model and ranking (§3.2 there). It's worth building first and testing on its own. A good pre-pass means the simulator search rarely enters repair.

### 3.1b Placement versus redstone (Allen, 2026-10-13)

Placement can be fully emulated offline. It is a finite classification: solid, stair and slab, waterloggable, connectable (panes, walls, fences), rail, attached, gravity, and so on, each with a placement recipe. Redstone can't be emulated by a hand-written model, because it sprawls into survival mechanics, entities and nearly every block type.

So the design splits in two:

- **Placement is offline and static.** It covers the pre-pass, the recipes and the click geometry.
- **Redstone uses as little behaviour as possible.** Build cold, arm last (§3.2) keeps the circuit unpowered, so most of the build reduces to pure placement. Real behaviour only matters for the hazard cells around actuators and sensors, and for the arming steps at the end.
- **Where behaviour does matter, use the real vanilla server jar, not a reimplementation.** That is a private copy of the same version, as v4.1's S1 does. It is exact for game logic. What it can't reproduce is the live world's unseen state (tick queues, block entity contents, chunk and entity ordering, other players, lag). That gap is why the build stays inert at step boundaries, and why live replay halts on any unexpected block change.

### 3.2 Build cold, arm last

The single most useful idea. A finished redstone build at rest has some cells that, once removed, leave every actuator with no way to receive power. The **arming set** is a small set of such cells, typically a dust segment, a repeater, a redstone block or torch, or a lever already in the on position.

1. Compute the arming set as a minimal cut in the power-flow graph, between power sources and sensors on one side and actuators on the other. Prefer cells that are cheap to place last and have no other dependants.
2. Build everything else with the arming set left as air, or as a non-conducting temp (glass, which conducts nothing and blocks QC power through it).
3. Close the arming set last. Order and timing come from simulation: each close must take the circuit to its target rest state, not merely to *a* rest state.
4. Clocks and other never-quiet machines: leave the loop open and close it last. This is v4.1 §9.2.

A simple heuristic order that covers most builds without search:

1. Inert solids, layer by layer.
2. Gravity blocks once their support exists.
3. Derived-shape blocks, in an order where neighbour rules produce the target. For rails, place each curve's two neighbours first and the curve last. For chests, sneak-place single chests next to other chests so they don't pair.
4. Actuators, unpowered. For each observer: everything in its watched cell is final first. For each piston, dropper or dispenser: no block that QC-powers it, and no neighbour placement after it unless that neighbour is in the arming set.
5. Containers and their contents, before any comparator, hopper or dropper that reads or moves them is connected. Hoppers can be built locked by putting their lock power in early, and that power is then removed as part of arming if the target has them unlocked.
6. Sensors: sculk sensors after everything within their range, or shielded meanwhile by temporary wool (wool blocks vibrations) that is removed during arming. Plates and tripwire after the bot no longer needs to walk there.
7. Fluids and waterlogging, behind temporary walls where needed.
8. Entities (§4).
9. Arm.
10. Forced-update stability check (v4.1 §5).

The simulator remains the judge (v4.1 P2). This order only ranks candidates.

### 3.3 "Suppress then settle" when the server allows it (tier A)

- **Vanilla 1.21.5+:** `/setblock <pos> <state> strict` or `/fill … strict` places the exact state with no block or shape updates. Order no longer matters for popping or firing. Place block entities with their NBT in the same command.
- **Carpet:** the `fillUpdates false` rule, plus `/player` fake players for the simulated bot (v4.1 S2).
- **Settle phase:** afterwards, do a controlled neighbour update of every cell in the simulator and diff. Any change means the schematic isn't update-stable as saved: a BUD piston, a lamp lit with no power, a rail in a non-derivable shape. It's either an intended "unreachable" state that needs a macro or protection, or a bug in the schematic. Report it rather than "fixing" it.
- Even on a creative tech server this is much cheaper than full search. The search is only needed where a player has to place blocks legitimately.

### 3.4 Exact states without commands (tier B)

- Carpet `accurateBlockPlacement` and Litematica's easy-place protocol encode the wanted property values in the click's hit vector, and a server-side handler decodes them. The bot gets exact facing, half and type in one click.
- If the faction server runs one of these, use it. If not, the bot falls back to tier C recipes. Some servers actively block this: anti-Litematica plugins exist on Paper's Hangar.

### 3.5 Survival-only realities (tier C)

- **Rotation and look:** plan exact yaw and pitch per click. Anti-cheat may flag snapping, so interpolate the look over a few ticks, and keep reach well within the server's limit (v4.1 uses 4.5).
- **Follow-up clicks are vibrations and updates.** Tuning a note block or repeater is a sculk event and may also update neighbours. Schedule tuning before sensors and arming.
- **Leaves:** player-placed leaves are persistent. Schematic leaves with `persistent=false` will compare as different. Treat `persistent` and `distance` as "don't care" in verification, or place them deliberately.
- **Items run out.** Survival needs materials logistics. That's laborious but straightforward, and handled elsewhere.

### 3.6 When ordering alone can't do it

- **Wrap** (undo, place, redo) and **backtrack**, as in v4.1 §4.
- **Temporary scaffolding is double-edged.** Placing and removing it are updates too. v4.1's temp-safe-cell rules cover this.
- **Macros** for unreachable states. Seed a library with known patterns: BUD-extended pistons (power, extend, remove power without updating the piston), piston-placed blocks, minecarts and boats locked by pistons. Each macro is trialled like one compound step.

### 3.7 Test corpus

The builder should be judged on a fixed set of nasty schematics. Each one isolates a hazard class:

- observer chain and observer clock;
- 2-wide and 3×3 piston doors (QC, BUD, slime and honey, pistons moving observers);
- flying machine (must be built inert);
- rail loop with curves, detector and powered rails, and a minecart on it;
- sculk-sensor network with calibrated sensors;
- hopper clock and item sorter (exact item counts);
- copper-bulb T flip-flop (target lit and unlit);
- item-frame combination lock (rotation read by comparator);
- villager trading hall (job sites, cells, locked trades);
- water and lava farm with waterlogged blocks;
- a decorative build with stairs, panes, walls and doors, as a baseline.

## 4. Strategies for entities

### 4.1 What schematics contain

`.litematic` files store each region's entities with full NBT: type, position relative to the region, rotation, UUID, and per-type data (frame item and rotation, armor-stand pose and equipment, villager profession, level and offers, minecart contents). The builder should read this list and label each entity, as it does for blocks.

### 4.2 Classes and how to place each

| class | survival placement | notes |
|---|---|---|
| **Item frame, glow frame** | Place the frame on the support face (sneak if the support is interactable). Insert the item, then click N times for rotation (8 states, 4 for maps). | Rotation is comparator-readable (0–7), so it's a redstone action. Invisible and fixed frames come from NBT and commands only. |
| **Painting** | Survival picks a *random variant of the largest size that fits*. Build temporary blocks around the spot so only the target size fits, place, and break and replace until the variant matches. | Java creative has preset painting items, and the `painting/variant` item component sets it exactly with commands. Breaks if support goes; support can be replaced within 20 ticks. |
| **Armor stand** | 8 orientations from the placer's look. Equipment and armor by clicking; held items need arms. | Arms, small, invisible, marker and pose need NBT in Java, so commands only. Stands are pushable and fall, so place them after the area is final and settled. |
| **Minecart variants** | Use the cart on a rail; it spawns centred on the rail. Fill chest and hopper carts after placing. | Place before the rail's power is armed, or the cart leaves. A cart on a detector rail powers it, so it's a redstone action. Positions in the schematic are floats, so accept "on this rail cell". |
| **Boat, chest boat** | Use on a surface; yaw comes from the look. | Same tolerance idea. Boats locked in place by pistons need a macro (v4.1 §6). |
| **End crystal** | Use on obsidian or bedrock. | Explosive, so stay well clear. |
| **Leash knot** | Comes with a leashed mob tied to a fence. | Belongs to the mob step. |
| **Mobs, villagers** | See §4.3. | Can't be certified exactly. |
| **Item entities, falling blocks, TNT, projectiles, display/interaction/marker entities** | Usually discard. Display, interaction and marker entities need commands. | Flag in the pre-pass report. |

### 4.3 Mobs and villagers

- **Tier A:** `/summon` with the schematic's NBT, which is exact, including profession, level and offers. Give non-persistent mobs persistence.
- **Tier C:** a logistics problem, not a placement problem.
  1. **Source:** breed at the site, cure zombie villagers, or transport from elsewhere.
  2. **Transport:** villagers in boats (a player can row a boat with a mob in it, even over land) or minecarts on temporary rail. Many other mobs can use leads; whether villagers can be leashed depends on the version, so check it in sim. Water streams work for bulk.
  3. **Insert:** bring the mob into its cell while the cell is still open, then close it with the final block as one step. This is an access reservation just like v4.1 witnesses: the cell and its approach must stay open until the mob is in.
  4. **Bind:**
     - **Workstation:** a Java villager claims an unclaimed job site within about 48 blocks and must reach within about 2 blocks of it. So place the workstation *after* the villager is confined next to it, or other villagers will claim it first.
     - **Lock:** the profession locks only after a trade, so trade once to lock it.
     - **Persistence:** name-tag mobs that would otherwise despawn.
  5. **Accept** by cell, not by exact position. Villager offers and levels from the schematic can't be reproduced in survival, so report them as don't-care unless the user specifies otherwise.
- **Mobs interact with the build.** They trample farmland, open wooden doors, press plates, trip wires and get pushed. Mobs go in after blocks are inert, the builder re-verifies the build afterwards, and mob insertion is trialled like any action in the simulator. Its outcome is stochastic, so expect retries rather than a certificate.

### 4.4 Where entities sit in the order

- After their support, and after anything that would move or break them is final (pistons, water, gravity).
- Before arming, if arming would move them (carts on powered rails), or if they are part of the circuit's rest state (frames read by comparators, carts on detector rails).
- Mobs last, with their cells reserved from the start.
- An entity's own appearance counts as a change in the trial classifier, but an *intended* one. v4.1 currently treats any entity appearing as damage, so an entity step needs its own intended-change rule there.

## 5. Suggested changes to the v4.1 design

### 5.1 v4.1 assumptions that break under survival-only, no-op

Allen clarified (2026-10-13) that creative in the prototype is deliberate. It only skips resource and block-breaking logistics while prototyping. Flying is banned, and the final version runs on Azalea in strict survival. So the table below is not a list of mistakes. It lists what the creative prototype defers, which the survival version has to handle. The design should keep each of these as a seam so that swapping creative for survival is not a rewrite.

`ONLINE_BUILDER.md` v4.1 differs from strict survival in these places:

| v4.1 assumption | what changes in survival without op |
|---|---|
| Bot and S2 in **creative**: unlimited items, instant breaking, no drops (§0 S5) | Every block is carried, so steps need a bill of materials and restock stops. Breaking temps takes ticks with the right tool. **Breaking drops item entities**, which v4.1's classifier counts as damage ("any item entity is damage"). Drops from the bot's own breaks must become intended changes that the bot then collects. Temp material should be cheap and quick to break (dirt, or scaffolding blocks if movement allows them). |
| **`/forceload`** keeps the site ticking (§0 S5) | Chunks tick only near players. A bot, or a chunk loader the build is allowed to contain, must keep the site plus margin within simulation distance for the whole build. Another bot standing guard can provide this, but the guard counts as an entity near the site. |
| **`/data get block`** readback of containers (§0 S3) | Only blocks, entities and the bot's own inventory are visible. Container contents can be checked only by opening the container. That is itself an action: opening changes barrel `open` state and powers trapped chests, so it has to be trialled. |
| **Start state exported server-side** (§0 S5) | The margin can only be scanned from the client. Block entity contents, tick queues and pending block events are invisible. Certification must either require an inert margin (no block entities, no power, no pending ticks) or accept a weaker guarantee. |
| **Gamerules copied from live**, including `randomTickSpeed` | Reading gamerules needs op. Assume vanilla defaults unless the server owner states otherwise, and say so in the certificate. |
| **No foreign entities, weather or mobs** reach the site | Survival has hostile mobs at night and possibly raiders on a faction server. Light the site and margin first (mob-proofing becomes an early build phase), pause on intrusion as v4.1 already does, and accept that divergence halts will happen more often. |
| Reach 4.5, survival movement, no flight | Already survival-compatible. Anti-cheat may also constrain look-snapping and click rate (§3.5). |

What survives unchanged: the offline simulator (S1 and S2 can run creative and op in a private copy, as long as the *simulated* bot obeys survival rules), the trial classifier, witnesses, repair search and packet replay. The architecture doc lists the same question in §3.7; it should be settled in the builder's own design before the interface is fixed.

### 5.2 Additions

1. **A pre-pass compiler** (§3.1): labels, precedence graph, per-tier recipes, unreachable report. This makes ranking concrete and catches impossible schematics before a long search.
2. **Arming-set computation** (§3.2) as a first-class ranking input. The arming set goes last, with v4.1 §9.2 (never-quiet builds) handled through it.
3. **Tier selection** (§0 item 2) as a site setting. v4.1 assumes creative on a technical server. Faction SMP is tier C, which changes reach, materials, recipes and anti-cheat constraints.
4. **An entity phase** (§4) with intended-entity steps, tolerances, reservations for mob cells, and stochastic retries. Today it's only "macros".
5. **A test corpus** (§3.7) as the acceptance bar.

## 5.3 Target version and portability

Allen's target is Java 1.21.11, and the builder must be portable to other versions. What that means for the design:

- **Keep every version-dependent fact out of hand-written code.** That covers block states and properties, shapes, placement rules (which way an observer or piston faces for a given look), attachment and support rules, quasi-connectivity, and observer pulse-on-placement. Take them from two sources:
  - **Generated data per version:** the vanilla data generator (`--reports` gives blocks and states) plus extracted shapes. Azalea already generates its block, state and shape tables per Minecraft version.
  - **Probing in the simulator per version:** placement recipes (§3.1) come from running a placement in S1 and reading the result, not from a lookup table. That is Baritone's `approxPlaceable` idea, moved server-side so it is exact.
- **The hazard taxonomy (§2) is mostly stable, but its membership changes.** Copper bulbs and crafters (1.21) did not exist before. Calibrated sculk arrived in 1.20. Keep the per-block labels in a data file per version, and let the simulator flag a block whose trial behaviour contradicts its label. That is v4.1 §7's hazard-model corrections.
- **Re-run the test corpus (§3.7) on every version port.** It is the regression suite that says a port works.
- **Version-specific entity facts** (painting variant items, leashing villagers, armor stand rotations) go in the same per-version data file, and are probed in the simulator where possible.

## 6. Questions that change the design

- Settled (Allen, 2026-10-13): only vanilla can be assumed, so every recipe is tier C. Tier B is not available.
- Is the prototype following `ONLINE_BUILDER.md`, or a different design? This thread hasn't seen its code.

## 7. Sources

- Litematica printer (aleksilassila): https://github.com/aleksilassila/litematica-printer
- Other printer forks: https://github.com/eatmyvenom/litematica-printer, https://github.com/IceTank/litematica-printer, https://github.com/diegogarciarojo/litematica-printer-fixes, https://modrinth.com/mod/litematica-printer-hana, https://modrinth.com/mod/litematica-printer-4th, https://www.curseforge.com/projects/1312873
- Printer Easyplace Extension (described only as "redstone-aware"; no mechanism published): https://www.9minecraft.net/?p=722477
- Baritone `BuilderProcess`: https://github.com/cabaletta/baritone/blob/master/src/main/java/baritone/process/BuilderProcess.java
- mineflayer-builder: https://github.com/PrismarineJS/mineflayer-builder
- WorldEdit `MultiStageReorder`: https://github.com/EngineHub/WorldEdit/blob/master/worldedit-core/src/main/java/com/sk89q/worldedit/extent/reorder/MultiStageReorder.java
- RedstoneAwareEdits: https://moddex.gg/mod/redstoneawareedits
- `/setblock` and `strict`: https://minecraft.wiki/w/Commands/setblock (and MC-279972 for the 25w02a context)
- Anti-Litematica plugin (servers blocking easy-place): https://hangar.papermc.io/ALingqing/AntiLitematica
- Entities: https://minecraft.wiki/w/Item_Frame, https://minecraft.wiki/w/Painting, https://minecraft.wiki/w/Armor_Stand, https://minecraft.wiki/w/Villager
- Papers: APT https://arxiv.org/abs/2411.17255, MineCEraft https://arxiv.org/abs/2608.28884, Builder Dialogue benchmark https://arxiv.org/abs/2407.12734, LLMs as Minecraft agents https://arxiv.org/abs/2402.08392
- Stated from general knowledge and not re-checked in this pass: details of Carpet `accurateBlockPlacement`, the easy-place protocol encoding, MCHPRS/redpiler, and assembly-sequence-planning literature. Verify before relying on specifics.
