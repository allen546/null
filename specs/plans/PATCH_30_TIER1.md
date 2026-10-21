# Fork patches 30–32: tier 1 of the safe envelope (draft for Allen)

Draft, 2026-10-20, by the lead. Status: **approved by Allen 2026-10-20, all four decisions at their defaults** (recorded in DECISIONS AD-F5.5). Nothing is built. The rules are now in B1 §4.2 and §4.3 and F5 §4.6 (the refusals too); this file is the source for the implementer briefs.

## 1. What this is and why

AD-F5.5 (Allen, 2026-10-20) decides a safe envelope in tiers, not full parity with the vanilla client. Tier 1 is the minimum set:
- Levitation and Slow Falling. A bot hit by a slow-falling arrow must not be banned.
- Pushes from nearby entities. The swarm crowds together, and mobs push in melee.
- The disconnect for `Movement` entries in `BotView::unmodelled_inputs()`.
- Client refusals for the inputs only the bot can start (elytra, riptide, mounting).
- Pathfinder walls for the blocks the fork does not model.

The calibration is in `specs/reference/CLIENT_INPUT_COVERAGE.md`. The source research for this draft also found one patch 27 gap: attributes on respawn (§2.5). The draft adds it to tier 1, because it causes wrong predictions after every death.

Vanilla source: `~/Desktop/null-reference/vanilla-1.21.11-src/`. Grim source: `~/Desktop/null-reference/grimac/`.

## 2. Rules (proposed spec text)

### 2.1 Levitation (`LivingEntity#travelInAir`)

- With Levitation in force, air travel replaces gravity with a pull toward a target vertical speed. The new vertical speed is the old one plus (0.05 × (amplifier + 1) − old) × 0.2. Then the usual air drag (× 0.98) applies.
- The old value is the vertical speed after the climb and friction step (`handleRelativeFrictionAndCalculateMovement`), so a bot on a ladder also gets the pull.
- Levitation acts only in air travel. In water or lava (`travelInFluid`) it has no effect.
- The client keeps the effect until `ClientboundRemoveMobEffect` arrives. `MobEffectInstance#tickClient` counts the duration down but never removes the effect. The fork must not drop the effect when its duration reaches zero.
- The effect is in force from the packet (`ClientPacketListener#handleUpdateMobEffect`). Grim starts to expect it from the client's reply to the transaction sent with it. The fork applies it on the packet, as vanilla does.
- Grim models Levitation from the effect level (`PointThreeEstimator`, `positiveLevitation`). The `Simulation` check must stay clean.

### 2.2 Slow Falling (`LivingEntity#getEffectiveGravity`)

- With Slow Falling in force and a vertical speed ≤ 0, gravity is the smaller of the gravity attribute and 0.01. With an upward speed, gravity is unchanged. Air travel reads the speed after the move step (`travelInAir` calls `getEffectiveGravity` after `handleRelativeFrictionAndCalculateMovement`); fluid travel reads it once at the start (`travelInFluid`). Corrected 2026-10-20 by the patch 30 implementer from the source.
- `getEffectiveGravity` is used in air travel, water travel and lava travel. All three must use the capped value. (It is also used for elytra flight, which stays refused.)
- Fall distance resets every tick under Slow Falling or Levitation (`LivingEntity#aiStep`). Only the server uses fall distance for damage, so the fork needs no change for it.

### 2.3 Pushes from entities (`LivingEntity#pushEntities`, `Entity#push`)

How vanilla pushes the local player:
- The local player is never pushed by its own `pushEntities`. On the client, `EntitySelector#pushableBy` accepts only the local player as a target. The player excludes itself.
- So the client pushes the local player from the other entities it ticks. Each living entity calls `pushEntities` at the end of its `aiStep` on the client too, with its own box. Remote players do the same (`RemotePlayer#aiStep`). Boats push (`AbstractBoat#tick`, box inflated 0.2 sideways and −0.01 vertically). Minecarts do not push on the client: both minecart behaviours call their push code only on the server (corrected 2026-10-20 by the patch 31 research). Exceptions: an armor stand never pushes a player (`ArmorStand#pushEntities` pushes only rideable minecarts), a bat pushes nothing, a parrot skips players (`Parrot#doPush`), a sleeping player is not pushed by a living entity or a remote player (`LivingEntity#push`; a boat has no such check), and a pusher with `noPhysics` pushes nothing (a spectator remote player, a Vex, the Ender Dragon). A ridden horse still pushes (`AbstractHorse#isPushable` is about the horse as a target), and so does a dying mob until the server removes it. A boat pushes the bot only when the bot's box bottom is at or below the boat's, and never a bot riding it (`AbstractBoat#push`; corrected 2026-10-21 by the patch 32 research).
- A push is `Entity#push(Entity)`. It is skipped when the two ride the same vehicle or either has `noPhysics`. Take the horizontal offset between the two entities. If its larger axis is under 0.01, nothing happens. Otherwise divide the offset by the square root of that larger axis, scale it by min(1, 1/√larger axis), then by 0.05. Add it to the local player's velocity, away from the other entity. The player must be pushable: alive, not a spectator, not on a climbable block (`LivingEntity#isPushable`).
- Team collision rules decide who may push whom (`Team.CollisionRule`: `always`, `never`, `pushOtherTeams`, `pushOwnTeam`). The fork must track teams from `ClientboundSetPlayerTeam` for this.
- Order in the tick: the client ticks entities in the order they entered its level (`ClientLevel#tickEntities`, `EntityTickList`, an insertion-ordered map). A push from an entity that ticks before the player changes this tick's travel. A push from one that ticks after changes the next tick's travel. A respawn adds a new local player, so it moves to the end of the order.
- Position used: each pusher's position after its own interpolation step in this tick (`LivingEntity#aiStep` calls `InterpolationHandler#interpolate` first). Vanilla moves another entity toward a new server position over several ticks (3 by default for living entities and boats; minecarts use their own rule). The fork now sets the server position at once.
- Bot-to-bot view (Allen, 2026-10-20): the swarm shares one world, and a bot's own client writes its exact position, but a vanilla client sees another player at the position the server sent, smoothed. So each bot keeps its own smoothed view of the other bots within 16 blocks (reach plus 3 ticks of movement, with margin), at most its 32 nearest, with its aim target always kept; reaching the cap is logged. Outside the radius nothing is kept. Mobs and other non-bot entities keep one shared smoothed copy, because the server sends every client the same positions. Memory is linear: at most about 3 KB per bot. A counter of the entries feeds the felling and crowd tests (OPEN item 7). Only position is smoothed; no client decision reads another entity's rotation.
- Split (Allen, 2026-10-20): patch 31 is interpolation, with the bot-to-bot view, per-entity step counts and the bounding box refreshed each tick; patch 32 is the pushes (team store and collision rules, the tick order for each bot, the push); the refusals and pathfinder walls move to patch 33.
- Cramming damage (`maxEntityCramming`) is server-only. The fork skips it.
- Grim does not simulate pushes exactly. It counts the entities that could overlap the player and widens its tolerance (`MovementTicker#handleEntityCollisions`, `UncertaintyHandler.collidingEntities`). So a small error passes Grim. The standing rule still asks for vanilla's exact result.

### 2.4 The disconnect for `Movement` entries

- A `Movement` entry means an input in force moves the bot by itself, and the fork cannot predict how. The client's next position would be wrong. AD-F5.5 decides: disconnect at once. A vanilla player can always quit.
- Where: a Null-only fork system at the start of each `GameTick`, before every physics and packet system. It acts only when a component `DisconnectOnUnmodelledMovement` is on the client; `mc-adapter` inserts it for every bot. Since patch 23 an `Update` applies the packets before the frame's ticks run, so the first tick after the packet disconnects. No position computed with a wrong model is sent.
- What it sends: nothing more. It closes the connection, as a vanilla client does when it quits. (The implementer confirms that a play-state client sends no quit packet.) It records `UnmodelledDisconnect { tick, inputs }`. `BotView` shows the record, and the host reports it. What happens next (stay off, or rejoin when the input ends) is decided above the fork. The default is to stay off and report.
- After patches 30–32, Levitation and Slow Falling are modelled, so they are no longer `Movement` entries (decision P30.2). The `Movement` entries that remain:
  - scale away from 1 (hitbox and eye height);
  - Dolphin's Grace;
  - **new:** the bot is a passenger (a server `/ride` or a plugin can mount it);
  - **new, decision P30.3:** the bot's box touches a block the fork does not model: bubble column, slime block, bed (landing), honey block from the side, scaffolding, moving piston, or the world border. A knockback or a push can put the bot there even with every pathfinder wall in place.

### 2.5 Attributes across a respawn (patch 27 gap)

- Vanilla builds a new local player on every `ClientboundRespawn` (`ClientPacketListener#handleRespawn`). The packet's data-to-keep byte decides what carries over to the new attribute map. Base values always carry over (`AttributeMap#assignBaseValues`). With bit 1 (`KEEP_ATTRIBUTE_MODIFIERS`) the modifiers also carry over (`assignAllValues`). A death respawn sets the bit only when the respawn keeps all data. A dimension change sets it.
- The fork rebuilds `Attributes` from defaults on every respawn (`EntityBundle::new` in the `respawn` handler). After `/attribute ... base set` and a death, the bot predicts with the default until the server resends the attribute. Fix: carry the values over as vanilla does.
- Effects are not carried over. The new player starts with none, and the server resends them (`PlayerList#respawn` → `sendActivePlayerEffects`).

### 2.6 Client refusals (inputs only the bot can start)

Each is refused on the client with a reason, and nothing is sent:
- **Elytra:** a jump input in the air while the bot wears a glider and could start to glide (`LocalPlayer#aiStep` → `tryToStartFallFlying`). A vanilla client would send `START_FALL_FLYING`. The fork sends neither the jump nor the command.
- **Riptide:** releasing a trident with Riptide while in water or rain (`TridentItem#releaseUsing` launches the player on the client). Simpler alternative: refuse the start of the use.
- **Mounting:** patch 24 refuses a click that would mount a chest boat (`EntityUseRefusal::WouldMount`). Extend it to every entity that mounts on a plain right-click: boats, rideable minecarts, saddled horses, donkeys, mules, camels, pigs, striders, llamas, happy ghasts, and any other entity whose `interact` override mounts the player. The implementer lists them from the `interact` overrides.
- **Item use with a slowdown other than 0.2:** refuse the start of a use when the item's `use_effects` component sets another speed.

### 2.7 Pathfinder walls

The pathfinder treats these as walls until a later tier models them: bubble columns, slime blocks, beds, the cells beside a honey block at body height, scaffolding, moving pistons, the cell in front of a piston face, and the world border with a margin. Patch 26 did the same for cobwebs (B1 §4.3). These walls lower the chance of a `Movement` disconnect (§2.4). They do not replace it.

## 3. Decisions for Allen

Each has a default. The draft assumes the defaults.

| id | question | what it changes | default |
|---|---|---|---|
| P30.1 | Model the client's position interpolation of other entities, or push from their server positions? | Exact pushes need it. It also changes entity aim (patch 24) and attack reach, which then use the positions a vanilla client uses; A8 and A10E must stay green. Without it, pushes are close but not exact; Grim's tolerance hides the difference. | model it (the standing rule: compute what the vanilla client computes) |
| P30.2 | Once Levitation and Slow Falling are modelled, what does a skill do while one is in force? | Physics is right, but skills and the pathfinder plan with normal gravity. | `Action` entry: stop the skill, stay connected |
| P30.3 | Touching an unmodelled block: a `Movement` entry (disconnect)? | Safety when a push or knockback puts the bot into a bubble column, onto slime, etc. Costs a disconnect in rare cases. | yes |
| P30.4 | Split and order | Keeps each task at 500–800 lines. | four patches (revised 2026-10-20): 30 (Levitation, Slow Falling, gate kinds, disconnect, respawn attributes), 31 (interpolation and the bot-to-bot view), 32 (pushes, teams, entity tick order), 33 (refusals, pathfinder walls) |

## 4. Tests

Unit and simulation tests in the fork compare each formula with vanilla (Levitation I and II from rest and from a fall; Slow Falling in air and in water; a push at each offset band; tick order; interpolation steps; the disconnect sends no packet after the trigger). New live scenario A17 (`scenarios_envelope.rs`). Each row passes on vanilla and on Paper + Grim with zero Grim alerts:
- `levitation_i_rises`, `levitation_ii_rises`: `/effect give`, read the height back by RCON tick by tick against the vanilla formula; then `/effect clear`, the bot falls and lands. The skill stops with a reason (P30.2).
- `slow_falling_fall_20_blocks`: health unchanged (no fall damage), ticks to land as the formula gives.
- `slow_falling_arrow`: a tipped arrow summoned by RCON hits the bot in a jump (the case AD-F5.5 names).
- `pushed_by_mob`: a mob with no AI overlaps the idle bot; the bot drifts as vanilla predicts.
- `pushed_by_bot`: two bots in one cell move apart.
- `team_never_no_push`: a team with `collisionRule never`; the bots do not move.
- `pushed_by_boat`: a boat summoned into the bot.
- `disconnect_on_scale`: `/attribute ... minecraft:scale base set 2`; the bot leaves before its next position packet (a fork counter shows none sent after the attribute packet); the server log shows the leave.
- `disconnect_on_ride`: `/ride <bot> mount <boat>`.
- `disconnect_in_bubble_column` (if P30.3): knock the bot into one with RCON motion.
- `refuse_elytra_jump`, `refuse_riptide`, `refuse_mount_*`: refused with a reason, nothing sent, server state unchanged.
- `path_avoids_unmodelled`: a goal behind a bubble column and slime; the path goes round or the goal is refused.
- `respawn_keeps_base_attribute`: base `movement_speed` 0.15, `/kill`, respawn, walk; `Simulation` stays clean.

## 5. How it runs

The usual rules apply (CLAUDE.md): Sonnet implementers, one task per patch, each about 500–800 lines with one done-when. Each runs only its new rows, once per server. Allen runs the full-suite pair offline after all three. A verifier checks the logs and runs negative controls (Levitation with the old gravity; pushes with no push system; the disconnect gate off on `disconnect_on_scale`). Then the commit, the fork push and the ledger rows.
