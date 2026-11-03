# Refused abilities: to-do list and the feature branch

Allen, 2026-11-03: refused abilities keep being refused on the stable fork, and they are set up as to-dos. One branch develops them concurrently, while a stable `null-azalea` gets its test on `mine`.

## What this is

The safe envelope (F5 §4.6, AD-F5.5) makes a bot refuse, stop or disconnect when the fork cannot simulate what a vanilla client would do. These limits are correct and stay on stable until the feature replaces them. Each one is also an ability vanilla has and Null lacks. VD3 skips their rows for that reason (VD3_BRIEF).

## To-do

Sizes are estimates from the names and the vanilla methods, not from a design. Nothing here is started.

| Ability | What Null does today | Vanilla method | Row | Size |
|---|---|---|---|---|
| Item `use_effects` (use speed, sprint) | refuses the use | item use slow-down, the `use_effects` component | A20 `refuse_use_effects_speed` | small |
| Dolphin's Grace | disconnects | `LivingEntity#travelInWater` | A16 `dolphins_grace_...` | small |
| `scale` attribute | disconnects | hitbox and eye height from the attribute | A16 `scale_2_...`, A17 `disconnect_on_scale` | medium |
| Slime floor | disconnects, pathfinder goes around | `SlimeBlock#stepOn`, `#updateEntityMovementAfterFallOn` | A21 slime | small |
| Bed bounce | disconnects, pathfinder goes around | `BedBlock#updateEntityMovementAfterFallOn` | A21 beds | small |
| Honey wall | disconnects, pathfinder goes around | honey block side contact | A21 honey | small |
| Scaffolding | disconnects, pathfinder goes around | scaffolding collision and climb | A21 scaffolding | medium |
| Bubble column | disconnects, pathfinder goes around | bubble column motion | A21 bubble, A17 `disconnect_in_bubble_column` | medium |
| Piston faces | disconnects, pathfinder goes around | block push and sticky faces | A21 piston | medium |
| World border contact | disconnects, pathfinder stays out | `WorldBorder` collision | A21 border | medium |
| Riptide | refuses the use | `TridentItem#releaseUsing`; needs weather state | A20 `refuse_riptide_in_water` | medium |
| Elytra | refuses the jump | `LocalPlayer#aiStep`, `LivingEntity#updateFallFlying`, `travelFallFlying` | A20 `refuse_elytra_jump_in_air` | large |
| Riding (boat, minecart, horse family, camel, pig, strider, happy ghast) | refuses the click, disconnects as a passenger | each vehicle's control and motion | A20 `refuse_mount_*`, A17 `disconnect_on_ride` | large, one per vehicle family |
| Place prediction stages 2 and 3 (722 `Unknown` block families, reacting neighbours) | refuses inside the movement zone, sends unpredicted outside it | `Block#getStateForPlacement` per family | A22 refusal rows, B6 §4.10 | large; also closes patch 54's "any change counts" gap |
| Levitation and Slow Falling in plans | physics modelled, skill stops | skills and pathfinder plan with normal gravity | A16 action rows | medium, above the fork |

Order: decided when the branch starts. Default: what `mine` and building meet first (block contacts, place prediction), then the small ones, then riding and elytra.

## The branch

- **Stable:** fork `null-main`, the pin in `Cargo.toml`, the `mine` work.
- **Features:** fork branch `null-features`, starting from a pushed `null-main` head, kept rebased on it. Work on the Null side lives in its own worktree and branch.
- **One clone cannot serve both.** `~/.cargo/config.toml` builds every worktree against `~/Desktop/null-spikes/azalea`. So the features branch needs its own clone and a `.cargo/config.toml` in its worktree pointing there. First step: prove that override wins, with a build that names the path it used.
- **Moving a feature to stable:** the same gate as any patch: named vanilla method and Grim check, tests, selective run on both servers, the pair cadence. The refusal goes in the same patch, and its VD3 row stops being a skip and must match the vanilla client. A feature that is not ready stays refused on stable.
- **Live test servers and heavy builds are shared.** Grim runs must not overlap a heavy build, so the two lines must take turns on the Mac.

Can start now that patch 54 is pushed (2026-11-03). Not started yet.
