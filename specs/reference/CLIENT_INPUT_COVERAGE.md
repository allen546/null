# Client input coverage: what the vanilla client computes for itself, and what the fork has

Calibration, 2026-10-20, against fork head `34d54a5` (patch 26). Null computes what a vanilla client computes for its own movement and actions (F5 §4.6, "What Null computes"). This page lists every input that code reads and says whether the fork has it. "Present" means the code exists; it does not mean it is proven correct (the ability suites prove part of it).

How it was made: the inputs were read from vanilla 1.21.11 (`LocalPlayer`, `ClientInput`, `LivingEntity`, `Entity`, `Player`; every block class that overrides `entityInside`, `stepOn`, `updateEntityMovementAfterFallOn` or a context-dependent collision shape; per-block `friction`, `speedFactor`, `jumpFactor` in `Blocks`). Then each one was looked up in the fork (`azalea-physics`, `azalea-client`, `azalea-entity`, the pathfinder). Server-only inputs (damage, armour, display, fall damage) are left out.

## Present

Block friction (ice, packed ice, frosted ice, blue ice, slime), speed and jump factors (soul sand, honey), stuck speed (cobweb, sweet berry bush, powdered snow; patch 26), climbing (climbable tag, trapdoor over a ladder), water and lava travel and the push of flowing water, step height, sneaking speed, block and entity interaction range, break time (Haste, Mining Fatigue, Conduit Power, mining efficiency, submerged mining speed, block break speed), Jump Boost, knockback (`ClientboundSetEntityMotion`), explosion knockback (`ClientboundExplode`), crouching and swimming poses, collision with boats and shulkers, the 0.2 slowdown while using an item.

## Missing, group A: can happen to the bot without its choice

These move the bot even with no input, so stopping a skill does not make them safe.

| input | vanilla | fork at `34d54a5` | where a bot meets it |
|---|---|---|---|
| pushes from nearby entities (players and mobs) | `LivingEntity#pushEntities` runs on the client too and pushes the local player away from every pushable entity it overlaps (`Entity#push`) | none | the swarm itself (bots that crowd together), and any mob in melee range |
| bubble column | `BubbleColumnBlock#entityInside`, `Entity#onInsideBubbleColumn` | none | ocean ruins, magma or soul sand under water |
| gravity attribute | `LivingEntity#getEffectiveGravity` | hardcoded 0.08 (`azalea-physics/src/travel.rs:355`) | servers, commands (patch 27) |
| Slow Falling | `LivingEntity#getEffectiveGravity` | none (`TODO` at `travel.rs:356`) | potions, combat |
| Levitation | `LivingEntity#travelInAir` | none | shulkers in End cities |
| scale attribute | hitbox and eye height | none | servers, commands (patch 27) |
| slime block bounce and slowdown | `SlimeBlock#stepOn`, `SlimeBlock#updateEntityMovementAfterFallOn` | none | landing on slime, farms |
| bed bounce | `BedBlock#updateEntityMovementAfterFallOn` | none | landing on a bed |
| honey wall slide | `HoneyBlock#entityInside` | none | touching honey from the side |
| scaffolding: stand on top, sink while crouching | `ScaffoldingBlock#getCollisionShape` (context) | interaction shape only | builds |
| powdered snow with leather boots | `PowderSnowBlock#getCollisionShape` (context) | none (patch 26 follow-up) | snowy slopes, with boots |
| Dolphin's Grace (in water) | `LivingEntity#travelInFluid` | a comment only (`travel.rs:178`) | rare |
| piston push | `PistonMovingBlockEntity#moveCollidedEntities`, `Entity#limitPistonMovement` | a comment only (`collision/mod.rs:178`) | redstone, farms |
| world border | `WorldBorder` collision | `TODO` | only at the border |
| push out of a block | `LocalPlayer#moveTowardsClosestSpace` | none | a block placed into the bot |

## Missing, group B: happens only when the bot chooses it

A refusal on the client covers each one until it is modelled: the bot never starts it. Built in fork patch 34 (A20); the model itself is not built.

| input | fork at `34d54a5`, and the refusal (patch 34) |
|---|---|
| elytra (`glider` component) | hardcoded off (`azalea-client/src/plugins/movement.rs:399`). Refused: a jump edge in the air with a glider worn (`client_refusals.rs`, `LocalPlayer#aiStep`). A glide set by the server is the `Gliding` disconnect entry |
| trident riptide | none. Refused: the start of a use of a `minecraft:trident` whose spin strength may be above 0 (registry effects; unknown enchantment counts as above 0), in or out of water, because the fork has no weather |
| vehicles (boats, minecarts, horses) | `TODO`. Refused: every right-click that mounts (`entity_actions::click_mounts`: boats, minecarts, horse family, camel, pig, strider, happy ghast, nautilus), also on the `EntityInteractEvent` path |
| item-use slowdown other than 0.2 (`use_effects` component) | hardcoded 0.2 (`movement.rs:473`). Refused: the start of a use of a stack whose `use_effects` changes the speed or `can_sprint` |
| Blindness stops the sprint | hardcoded to absent (`movement.rs:405`) |

## Other bot engines (2026-10-20)

Two mineflayer physics engines, read from source:
- `prismarine-physics` (PrismarineJS, MIT, the mineflayer default; `4d573bc`, 1,020 lines).
- `@nxg-org/mineflayer-physics-utils` (GPL-3.0; `e25fe2d`, 2026-08-02). Its `engines/botcraft.ts` (1,796 lines) is a port of Botcraft's physics (adepierre's C++ bot framework).

The GPL-3.0 code may be read as a reference, but not copied: the Azalea fork is MIT. The reference is vanilla in any case.

| group A or B input | prismarine-physics | botcraft port |
|---|---|---|
| pushes from entities | no | no |
| bubble column | yes | yes |
| gravity attribute | yes | yes |
| Slow Falling | yes | yes |
| Levitation | yes | yes |
| scale attribute | no | partial |
| slime bounce and slowdown | yes | yes |
| bed bounce | yes | yes |
| honey wall slide | yes | yes |
| scaffolding | yes | yes (stand on top unless crouching) |
| powdered snow with boots | no | no (`TODO`) |
| Dolphin's Grace | yes | yes |
| piston push | no | no |
| world border | no | no |
| push out of a block | no | no |
| elytra | yes | yes |
| riptide | no | no |
| vehicles | no | no (commented out) |
| Blindness stops the sprint | no | yes |
| item-use slowdown | no | yes |

Neither engine has entity pushes, pistons, the world border, the push out of a block, riptide or vehicles.

## Notes

- Patch 27 (server-sent attributes) covers the gravity and scale attributes and gates unmodelled effects and attributes in `BotView`.
- Until a group A block is modelled, the pathfinder can treat it as a wall (bubble columns, slime, beds, honey sides, scaffolding, pistons).
- Pushes from entities come first: a swarm is made of players that crowd together, mobs in melee range push too, and no row tests it yet. Neither mineflayer engine has it.
