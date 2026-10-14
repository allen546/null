# Scout report: vanilla Java mechanics for safe, fast survival building

Scope: Minecraft Java 1.21.x, vanilla server, survival, no op. Sources are the Minecraft Wiki (minecraft.wiki, mirrored at mcdf/breezewiki where the main host blocked us), Mojang snapshot articles, Mojang feedback/bug pages, and Paper source diffs (for what vanilla validation looks like). Several wiki pages were partly truncated or blocked (Piston sticky-drop section, Stairs, Rail re-shape rules, Reach server validation); those points are marked unverified rather than filled from memory.

## 1. Update propagation is specified, so placement effects can be computed locally

Source: https://minecraft.wiki/w/Block_update (wiki's own table; the priority numbers are described by the wiki as unofficial).

- Two update kinds matter. "NC" (neighbour changed, the redstone update) and "PP" (post-placement, which the wiki also calls "shape update"). They have different neighbour orders:
  - PP order: west, east, north, south, down, up.
  - NC order: west, east, down, up, north, south.
  - Comparator-only updates: north, east, south, west.
- Propagation is depth-first: one update can trigger a whole chain before sibling updates run.
- Placing a block sends PP updates to its six immediate neighbours in the PP order, and most blocks also send NC updates on placement.
- Wire does not send PP updates to observers, so dust placed next to an observer does not wake it.
- Many PP receivers do not react immediately; observers, water, falling blocks and others request a scheduled tick instead. Scheduled-tick reactions can therefore fire after the placement batch has finished.
- Sticky pistons do not send NC updates around their head when they start pulling.
- The wiki also lists a placed activated observer as going silently inactive (no pulse).

Mapping: this is the core of a fast pre-filter. For each candidate placement the planner can enumerate exactly which neighbours get which update, in which order, and which of them schedule ticks. Most candidate actions touch no reactive block at all, and those can be accepted with a local check in microseconds. The simulator is then used only for the cells whose local update graph reaches a reactive component. Honest estimate: a 5-20x cut in simulator calls for the pre-filter stage, not a 1000x end-to-end win, because the state-dependent cases still need the real server. Caveats: the wiki says its priority table is not in the game's code; the wiki also reports a self-update bug (a change that produces no general update); it describes depth-first chaining but not the full tick order. Treat the model as a filter that must be validated against the real server, never as a certifier.

Rejected for this project: the update-suppression material at https://mcdf.wiki.gg/wiki/Java_Edition:Update_suppression (throwing exceptions in update handlers, duplication via placement suppression, crash-based tricks). These are server-crash or dupe exploits, not survival builds, and they should be excluded from the builder.

## 2. "Cold" regions by chunk ticking: a negative result for cells the bot places

Source: https://minecraft.wiki/w/Chunk (Java load levels), https://minecraft.fandom.com/wiki/Chunk.

- Entity-ticking: level 31 and below, everything runs.
- Block-ticking: level 32 ("lazy" chunks in the wiki's wording). Entities are not naturally spawned or processed; chunk ticks stop.
- Border: level 33. No game aspects run, but blocks and entities can be read and modified.
- Inaccessible: level 34 and above.
- Redstone and command blocks run in the simulation square plus a one-chunk frame. A scheduled tick requested in a level-33 chunk is held until the chunk reaches level 32 or lower.
- Water, lava and fire stop at the first block outside a block-ticking chunk. Entities moving from an entity-ticking chunk into a block-ticking chunk are suspended until its level rises.

Consequence: a player's own ticket keeps every chunk within its reach at entity-ticking level, because reach (4.5 blocks) is far inside the simulation distance. So a chunk-level "cold" zone cannot contain the cell the bot is placing. Chunk coldness is only available for regions no player is near. The wiki's Java formula for the entity-ticking square is min(2s+1, 63) for simulation distance s; the default s is not confirmed here (unverified; I recall 10 but could not verify it).

What chunk coldness does give:
- Multi-bot isolation: a bot working far from another bot's region keeps that region out of simulation. This helps only when schematic regions are separated by more than the simulation distance, which is unlikely for a compact schematic.
- Deferred danger: a scheduled tick requested in a cold chunk is not cancelled; it waits and fires when the chunk heats. "Build far, then approach" plans must assume queued reactions will fire later. This is a hazard for the "cold build" idea, not a tool.
- Entity pause: a minecart or mob crossing into a block-ticking chunk stops until the chunk heats (wiki). Niche, unverified for our case.

Conclusion for the team: "cold" must be built at the block level (update order, what is placed when, what is not yet powered), not at the chunk level.

## 3. Quasi-connectivity is a precise static rule for pistons, dispensers and droppers

Source: https://minecraft.wiki/w/Tutorial:Quasi-connectivity; https://minecraft.wiki/w/Piston.

- QC affects pistons, dispensers and droppers in Java. Crafters are not affected. Observers are not covered by the tutorial.
- Normal activation: an active power component adjacent to the piston, a powered opaque block, a powered comparator or repeater facing it, or dust pointed at it or on top of it. A redstone torch does not power the block it is attached to.
- QC: the piston also activates when a normal method would activate the space above it, even if that space is empty or holds a non-conductive block.
- Some sources (wiki secondary sites) describe QC as "diagonally above" or "two blocks above". The wiki calls that a simplification. The "top-front corner" phrasing comes only from craftdex.net (secondary) and is unverified.
- A piston can be powered by QC without being activated; some QC sources (buttons, levers, tripwire hooks, the daylight sensor) only act on the next block update.
- Piston activation comes from any side except the head. Pistons are placed facing the player (wiki).
- A Mojang feedback post proposes transparent blocks in the gap to block QC; the post is garbled and its claim is unverified.

Mapping: for every piston in the target design, the static checker can compute the "QC-forbidden set": the cell above the piston plus the normal activators of that cell. A cell in that set that is or will become powered during the build is a hazard. This turns a class of silent misfires into a static check run before any placement, and it tells the planner to place the piston after its above-cell is final. Estimated effect: removes a whole category of stuck reports, cheaply, without simulation. Note the observer and dispenser cases still need checking against the real server.

## 4. Side-locked repeaters are a survival-buildable "hold" primitive

Source: https://minecraft.wiki/w/Repeater.

- Delay settings are 2, 4, 6 or 8 game ticks (1 to 4 redstone ticks). Each use adds two ticks, wrapping from 8 back to 2.
- A repeater is locked by a powered repeater or powered comparator facing its side. A plain powered block beside it is not listed as a lock source (inferred from omission).
- While locked, the repeater "completely ignores all block updates and scheduled ticks" and keeps its output. It unlocks when the side source turns off.
- Edge case from the wiki: if lock and input change in the same tick, the repeater may not switch states.

Mapping: a powered side repeater can freeze a whole downstream chain for as long as the source stays powered, and removing that source releases everything at once. That changes "arm last" from placing many cells at the end into removing one lock source. It also gives the planner a memory-safe hold during the build, since the hold is a survival-reachable state that ignores the very updates that would otherwise cause damage. Caveats: the lock source must itself be armed, the hold is a timing dependency (the same-tick edge case), and the wiki does not say how the lock interacts with the locked repeater's own scheduled tick beyond "ignores". Verify in the simulator before use. Estimated effect: a substantial simplification of the arming stage for any circuit that can be gated behind one repeater.

## 5. Survival-reachable states by click counting

Several blocks change state by repeated right-clicks, so the state is a count the planner can compute exactly with no search:

- Note block (https://minecraft.wiki/w/Note_Block): each right-click raises the pitch one semitone; 25 pitches per instrument; the 25th press resets to F#3. Pitch = 2^((n-12)/12) with n the click count. The instrument is set by the block directly underneath (sand/gravel gives Snare Drum, glass gives Hat, wood gives Bass; stone rows not checked). Attacking in survival plays a note; the wiki does not say it changes pitch. Placement as a trigger is unverified.
- Repeater delay (see section 4): from 2 to 8 ticks, the count is 0 to 3 clicks.
- Item frame (https://minecraft.wiki/w/Item_frame): right-click rotates the item 45 degrees, giving 8 positions for non-map items, and a comparator reads the rotation phase as a signal. Java remembers the last item orientation (wiki).
- Copper bulb (https://minecraft.wiki/w/Copper_Bulb): each pulse toggles lit/unlit; a comparator reading it gives a T flip-flop. The edge type (rising vs. falling), timing and placement behaviour are not stated (unverified). The state is a pulse-parity, so the planner can count pulses.
- Crafter (https://minecraft.wiki/w/Crafter): an empty slot is toggled by clicking it (survival behaviour not explicitly confirmed; unverified). Disabled slots count toward the comparator signal, which is a count-based readout. Not QC-affected in Java.
- Comparator mode: I believe it toggles per right-click, but I did not verify it here.

Mapping: a "state = click count from a known base" table lets the planner solve these states without searching, and the bot can read the current state from block data after each click. Honest estimate: modest total speed-up, but it moves a class of stuck cases (wrong note pitch, wrong repeater delay, wrong bulb parity) from search to arithmetic. Repair becomes "click k more times", which is also an easy undo.

## 6. Orientation comes from standing position; rails are order-dependent

- Pistons are placed facing the player (wiki). The head's facing is therefore fixed by the bot's standing side, not by the click face.
- Rails (https://minecraft.wiki/w/Rail; the page says its mechanics "need to be checked"): a new rail with no neighbours takes the direction the player faces in Java. Four neighbours always make the centre rail curve south-to-east. A T-junction curves to join two sides. A rail placed beside a straight rail re-orients it. A rail can join a rail one level up or down via a ramp. Breaking and relaying in a different order changes which shape results, so placement order is the control knob. The wiki says redstone power can rotate a curved T-junction rail. Support: rails cannot attach to a side or bottom face; they sit on rimmed tops.
- Repeater: the arrow points to the front; front is probably the direction the player faces when placing (inferred, unverified).

Mapping: orientation and rail shape are a function of (bot standing position, placement order), which can be enumerated per cell and included in the planner's action set without a search over click faces. Estimated effect: moderate, mostly by making the orientation decision explicit and checkable.

## 7. Redstone Experiments: deterministic wire order, but not available on a vanilla server by default

Sources: https://minecraft.wiki/w/Java_Edition_1.21.2; https://minecraft.wiki/w/Redstone_Experiments; https://www.minecraft.net/en-us/article/minecraft-snapshot-24w33a; https://www.minecraft.net/en-us/article/minecraft-snapshot-24w34a.

- Shipped as an experimental data pack in 1.21.2 (introduced in 24w33a); not in the full release notes.
- Wire turns on: wires nearer the source update first. Turns off: wires nearer the former source update first. Ties go to the left wire (24w34a replaced the earlier random tie-break).
- Block updates around wire use a fixed sequence based on direction: back, front, left, right, down, up. Wire only updates blocks that can receive its power. Wire signal strengths are set before updates are sent.
- The wiki states the wire is order-independent of location and orientation, which is the entire point of the pack.

Mapping: if the server ran this pack, the simulator would be exact for wire. For this project, the bot cannot enable a data pack on a vanilla no-op server, so the pack is useful only as a design reference and as a reason to keep wire-heavy designs out of the builder's critical path. Status in 1.21.11 is unverified.

## 8. Entities: minecarts, item frames, leads

- Minecart (https://minecraft.wiki/w/Minecart): placed "only on top of a rail". Pushed off a track end, it derails and decelerates within one or two blocks. A minecart pushed or falling onto tracks snaps to them. It bounces off opaque blocks at a track end and stops against transparent ones (wiki wording). Behaviour when dropped onto a non-rail surface from height is unverified.
- Item frame (https://minecraft.wiki/w/Item_frame): attaches to the sides of most solid blocks, and to top and bottom faces (not the top of fences, walls or gates in Java). Several frames can share one block on different faces. An item goes in only by a player "use" action, so content is set by a click, not by an update. The wiki says "fixed" frames refuse item insertion and removal, and that the fixed tag comes from NBT, so it is not survival-reachable. How a survival frame behaves when its support is removed is not confirmed (the page implies it drops, unverified).
- Lead (https://minecraft.wiki/w/Lead): maximum 12 blocks (16 for a happy ghast). A leashed mob moves toward the player or post; if it cannot move toward them while the lead is stretched, the lead breaks. Mobs tied to a fence stay within about 5 blocks of the post. Mobs suspend when the player or knot is more than 7 blocks above the ground. Villagers are not in the wiki's standard leashable list (the page says they can be leashed only by map or NBT editing; conflicting history noted). Use standard animals for lead-based movement and treat villagers as unverified.

Mapping: entity placement in survival reduces to "click on the rail / face / knot", and entity motion can be steered with a fence knot and a lead for standard mobs. This is simple to encode as actions with known preconditions. Estimated effect: small on overall speed, but it removes the need to plan entity placement as a search problem.

## 9. Speed, reach and server validation

- Reach: survival block interaction range 4.5 blocks and entity interaction range 3 blocks; creative is 5 blocks for both (https://minecraft.wiki/w/Reach). These are the attribute defaults.
- Server validation: the Paper patch that moved the place-reach check up (https://ayakael.net/mirrors/papermc/commit/325ba14ca5f9fd53da37c536c099c91c21025a19) shows vanilla-style logic in handleUseItemOn: a squared-distance check from the player's eye position, plus a finite-coordinate check on the hit location. The entity path adds 1.0 of leniency (Paper's clientInteractionLeniencyDistance defaults to the vanilla 1.0D; https://ayakael.net/mirrors/papermc/commit/645a677122257816e3dc77530dded6480ea52439). I could not confirm the block-side leniency; treat it as likely about 1.0 (unverified).
- "Moved too quickly": not covered by the wiki; treat as an unverified server movement check, not a placement limiter.
- Rate limiting: server.properties has rate-limit, default 0 (off), which kicks players sending too many packets (https://minecraft.wiki/w/Server.properties). Max-tick-time defaults to 60000 ms. So vanilla does not throttle placement by policy by default; the bot's real limit is reach and its own packet pace.
- Client-side click repeat delay: I believe vanilla clients wait about 4 ticks between held-button uses, but I could not find a source. Bots do not run a client, so this would not bind them. Unverified.

Mapping: the placement rate is not a binding constraint. The binding constraints are reach (one standing spot covers a radius of about 4.5 blocks) and verification. The practical idea is batching: send all placements for one standing spot in one packet batch per game tick. Immediate PP/NC chains then run inside the batch, and scheduled reactions run afterwards. This is a shape-changing guarantee if true (see the top-5 item below), and it should be tested. Estimated effect: fewer interleavings for timing-perturbation certification.

## 10. Top 5 (ranked)

1. **Exact local update-propagation model as a pre-filter.** The wiki's PP/NC orders, depth-first propagation, scheduled-tick deferral and the sticky-piston and wire-observer exceptions let the planner decide "cannot react" for most placements in microseconds, leaving the 0.4 s simulator for the rest. Estimate 5-20x on the filter stage; must be validated against the server because the priority table is unofficial.
2. **Side-locked repeater as a hold primitive.** One powered repeater or comparator on the side freezes a downstream chain (ignoring block updates and scheduled ticks), and removing it releases everything. This turns "arm last" from many placements into one removable lock source. Verify the same-tick edge case first.
3. **Quasi-connectivity as a static safety set per piston.** Compute P+up and its normal activators; forbid them from becoming powered during the build; place the piston after its above-cell is final. This removes a silent-misfire class without simulation; observer and dispenser cases still need the server.
4. **State by click count.** Note pitch, repeater delay, item-frame rotation and copper-bulb pulse parity are arithmetic on click counts from a known base, read back from block data after each click. This turns a set of stuck states into a count with an obvious undo.
5. **Per-tick packet batches per standing spot.** Vanilla has no placement-rate limit by default (rate-limit = 0), so batching is limited only by reach. If the server processes a tick's packets before its redstone ticks (unverified), each batch is an atomic step before any scheduled reaction fires, which shrinks the timing-perturbation space the certifier needs. Test this on the real server before relying on it.

## Unverified (listed for the team)

- Piston placed while powered: does it extend immediately? (not stated by the wiki page we read)
- Sticky piston block-dropping rules beyond the wiki's Java table (torches, rails, pressure plates break and drop; whether regular torches drop is unverified).
- Block-side interaction leniency on the server; "moved too quickly" checks.
- Client-side 4-tick placement repeat delay.
- Default simulation distance; whether immediate (non-scheduled) neighbour updates run in border chunks.
- Stairs placement and shape rules (wiki page did not load).
- Rail re-shape rules beyond the wiki's placement examples; Rail page says its mechanics need checking.
- Repeater facing rule (player facing when placed) and whether a plain powered block locks a repeater.
- Whether Redstone Experiments can be enabled on the target server; status in 1.21.11.
- Copper bulb edge type; comparator mode toggling on right-click.
- Villager leashing in survival.

Sources:
- https://minecraft.wiki/w/Block_update
- https://mcdf.wiki.gg/wiki/Java_Edition:Update_suppression (rejected material)
- https://minecraft.wiki/w/Chunk ; https://minecraft.fandom.com/wiki/Chunk
- https://minecraft.wiki/w/Tutorial:Quasi-connectivity ; https://minecraft.wiki/w/Piston ; https://craftdex.net/articles/piston-quasi-connectivity (secondary)
- https://minecraft.wiki/w/Repeater ; https://minecraft.wiki/w/Observer ; https://minecraft.wiki/w/Redstone_Dust
- https://minecraft.wiki/w/Note_Block ; https://minecraft.wiki/w/Item_frame ; https://minecraft.wiki/w/Copper_Bulb ; https://minecraft.wiki/w/Crafter
- https://minecraft.wiki/w/Rail ; https://minecraft.wiki/w/Minecart ; https://minecraft.wiki/w/Lead
- https://minecraft.wiki/w/Java_Edition_1.21.2 ; https://minecraft.wiki/w/Redstone_Experiments ; https://www.minecraft.net/en-us/article/minecraft-snapshot-24w33a ; https://www.minecraft.net/en-us/article/minecraft-snapshot-24w34a
- https://minecraft.wiki/w/Reach ; https://minecraft.wiki/w/Server.properties
- https://ayakael.net/mirrors/papermc/commit/325ba14ca5f9fd53da37c536c099c91c21025a19 ; https://ayakael.net/mirrors/papermc/commit/645a677122257816e3dc77530dded6480ea52439 (Paper mirrors, secondary)
- https://feedback.minecraft.net/hc/en-us/community/posts/4410779408781-limit-quasi-connectivity-using-transparent-blocks-in-the-gap-Java
