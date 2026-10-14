# modules as built

Documents `online/lib/trace.js`, `online/lib/replay_core.js` and `online/lib/classify.js`.

## lib/trace.js

Packet streams (design §1.4) for the bot's moves. D3: walking along a D2 path. D4 builds on the same Tracer.

### Closed loop

Everything is closed-loop against lib/physics.js (prismarine-physics): a controller picks the keys each tick, the physics says where the client would be, and that position is what the stream sends. The server only checks the stream (collisions, speed); it does no physics of its own, so the stream is only as good as this model.

A trace is {ok: true, ticks, end} or {ok: false, unavailable: true, edge, reason}. An edge the controller cannot finish within MAX_EDGE_TICKS makes the trace "unavailable": not an error.

### Tracer.tick

Run one client tick with `controls` ({forward, jump, sneak, yaw?, pitch?, ...}) and record its packets: an input packet when any of sneak/jump/forward changed (always on the first tick, which also normalises the actor's leftover state), `pre` packets, the move packet, then `post` packets. `after(state)` may return more post packets that depend on where this tick left the player (a click once the feet are high enough).

### pillar_up

pillar_up(n): from `pos` (feet, standing on the block below the feet cell), look straight down, and n times: hold the block, jump, and once the feet are at least cell y + 1.0 click the top face (centre) of the block below the feet cell, wait for the acknowledgement, and land on the new block. `states` (F2): one block state per level, bottom first (length n); each level then holds that block and the hotbar is set to it. Without it every level holds `state`.

### pillar_down

pillar_down(n): from `pos`, look straight down, and n times: break the block below the feet (the break packet, then the acknowledgement) and fall until on the ground.

### bridge

bridge(c): from `stand` (a cell, standing posture D2 stand), sneak towards the cell c until the sneaking edge clamp holds the player at the edge, pick the click for placing `state` at c from that exact position at the sneaking eye height (solver.solveFromStands), then look, hold the block, click and wait for the acknowledgement.

## lib/replay_core.js

The pure parts of the live replay bot (tools/replay.js; design planner/ONLINE_BUILDER.md §0 S3, IMPLEMENTATION.md I1). Nothing here touches the network: packet mapping, the expected world, the monitoring rule, the readback diff.

### Packet mapping

Packet mapping (IMPLEMENTATION.md §1.4 table -> minecraft-protocol names for 1.21.11; the unit tests serialize every mapped packet with minecraft-protocol's own 1.21.11 serializer, so a wrong name or field fails there):

- move -> position_look  {x,y,z,yaw,pitch,flags:{onGround,hasHorizontalCollision:false}}  (ServerboundMovePlayerPacket.PosRot)
- input -> player_input   {inputs:{forward,backward:false,left:false,right:false,jump,shift:sneak,sprint:false}}
- slot -> set_creative_slot {slot:36 (hotbar 0), item}, then held_item_slot {slotId:0}
- use_on -> block_place    {hand:0,location,direction,cursorX/Y/Z (point - block),insideBlock:false,worldBorderHit:false,sequence}
- use -> use_item       {hand:0,sequence,rotation:{x:yaw,y:pitch}}
- break -> block_dig      {status:0 (START_DESTROY_BLOCK),location,face,sequence}
- await_ack -> not a packet: the mapper returns {await: seq}, the sequence number of the last use_on / use / break.

The simulator hands the server the same values by building the vanilla packet objects directly (plugin deliverPacket), with the same defaults (an omitted `point` is the centre of the clicked face; omitted yaw/pitch/onGround of a move keep the previous ones). The plan's coordinates are plan coordinates; `origin` turns them into world coordinates.

### PacketMapper constructor

@param {{origin: number[], nextSequence: function, item?: function, look?: {yaw: number, pitch: number, onGround?: boolean}}} o

origin: the site origin (world = plan + origin). nextSequence: the shared block-prediction counter (mineflayer's bot._nextSequence). item: name -> slot structure (default: from prismarine-item).

### Expected state and the monitoring rule

Expected state and the monitoring rule (the documented contract of the live bot):

- The expected state after step i is the start state (plan.start.blocks) with the `changes` of steps 0..i applied (in tick order, the last change of a cell wins). Cells of the start bounds that start.blocks does not list are air.
- While step i runs (and while the bot waits for its settle time), a block update for a cell in the start bounds is EXPECTED when the new state equals a state that step i's `changes` gives that cell at any tick, or equals the cell's expected state from before the step (a client can receive a re-send of an unchanged state). Anything else is unexpected and the bot halts. The tick numbers in `changes` are NOT compared: live ticks drift by network latency, so only the set of states each cell may show during the step is judged. The tick order is judged by the readback at the end of the step, which compares the complete state.
- Cells outside the start bounds are ignored.

### ExpectedWorld.diff

The readback: every cell of the bounds against its expected state. `read(x, y, z)` returns a state id, or null when the client does not have the cell (chunk not loaded). Returns the differences [{cell, expected, actual}] in y, z, x order.

## lib/classify.js

Classifier (design planner/ONLINE_BUILDER.md §2 "Trial"; IMPLEMENTATION.md C1). Pure: no simulator.

Entry point: `classify({before, after, intended, actorBefore, actorAfter, actorPlanned, shapeUpdate}) -> {clean, derived, power, memory, damage}`.

### Inputs

- before, after: {blocks: Map<"x,y,z", state>, tiles: Map<"x,y,z", nbt>, entities: [entity]}. Both must cover the same block cells; a cell present on one side only throws.
- intended: Map<"x,y,z", state>, the action's own target states. An intended cell is judged against its target (not against before), so a settled change on it is still classified, and a match is not reported.
- actorBefore, actorAfter, actorPlanned: {pos: [x,y,z], ...}. Only pos is compared (tolerance 1e-6).
- shapeUpdate(pos, state): async; pos is [x,y,z]; returns vanilla's neighbour-shape update of state (B6).

### Result

Result. Each list holds entries of this one shape: {at, kind, why, before, after, props?, planned?}.

- at: 'cell x,y,z' | 'entity <type> at <pos json>' | 'actor'
- kind: 'block' | 'tile' | 'entity' | 'actor'
- why: the rule that put the entry there, with the properties involved
- before: block: the baseline state (before, or the target for an intended cell); tile: NBT or null; entity: the entity or null; actor: [x,y,z] before
- after: block: the state after; tile: NBT or null; entity: the entity or null; actor: [x,y,z] after
- props: block entries only: the property names that differ
- planned: actor entries only: [x,y,z] the plan asked for

clean is true only when damage and memory are both empty.
