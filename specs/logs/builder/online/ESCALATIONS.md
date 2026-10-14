# Escalations


## B4 `inert` op: E3, the actor's server-side velocity is never zero

- Task: B4 (and the inert phase of `play`, which shares its implementation).
- Tried:
  - Refactored play's inert phase and the new `inert` op into one `InertRun` (conditions in the spec's order, `reason` = first failing one). With no actor in the world the three B4 tests pass (water, hopper+chest, observer pair).
  - Once the actor exists (any earlier test creates it, so always in the full suite) `inert` is never true: reason `actor_velocity` on every tick.
  - Logged the actor's `getDeltaMovement()` inside the check.
- Evidence (temporary debug log, since removed; actor standing on a stone floor, `onGround=true`, position unchanged):

  ```
  DBG vel (0.0, -0.0784000015258789, 0.0) pos (7007.5, -59.0, 7.5) ground true     (every tick, repeated)
  DBG vel (0.0, -0.098, 0.0) -> (0.0, -0.1744...) -> (0.0, -0.2493...)             (actor floating in air: gravity accumulates every tick, position never changes)
  ```

  So the server does run `LivingEntity` gravity on the actor (it just never applies the movement), and the velocity is never below 1e-6. The premise "velocity stays 0 unless something sets it" is wrong. It looked true only because the `actor` op calls `setDeltaMovement(ZERO)` on every call, even with no arguments, so reads always return `[0,0,0]`. The same zeroing makes the velocity assertions in `snapshot.sim.test.js` vacuous (B3 stores and restores a velocity that is always the one just zeroed).
- Why it matters: design §2 says inert includes "the bot at rest (zero velocity)", and B4 says "the actor's velocity is zero (|v| < 1e-6)". Taken literally the region can never be inert once the actor is in the level. Any fix is a special case (ignore vy, compare with the standing value -0.0784, zero the velocity each tick, use the last move packet's delta), which is a design decision, not mine (E8).
- Status: plugin code and `Sim.inert` wrapper are committed. `online/test/inert.sim.test.js` is withheld because it can't pass in the full suite (copy kept at `/private/tmp/claude-502/-Users-allen-Desktop-null/a162edcc-9164-4391-b88f-09542f1fa117/scratchpad/inert.sim.test.js.b4`: tests 1-3 from the plan plus a 4th that checks play's inert result carries the same reason).
- Question for Allen: what does "the bot at rest" mean for the actor's server-side velocity, given the server's `deltaMovement` for it is gravity noise? Options: (a) require the actor's position to be unchanged for the quiet window instead of velocity; (b) ignore the vertical component; (c) zero the actor's velocity at the start of each tick (it is meaningless for a client-driven player); (d) use the last `move` packet's delta. Also: should the `actor` op stop zeroing velocity when called without `pos`?
- Resolved by orchestrator: (a), position-based rest; actor op reads are pure.

## D4 `bridge(c)`: E9, the solver's grazing-angle rule rejects every edge bridging click

- Task: D4 (`bridge`). `pillar_up` and `pillar_down` are done and pass; only `bridge` is blocked.
- Tried:
  - `trace.bridge` sneaks towards c until the sneaking edge clamp holds the player (prismarine-physics), then asks `solver.solveFromStands` (added with E1's signature, `stands` = that one feet position, `sneak: true` for eye height 1.27) for the click. It returns null for every one of the 4 cases (one off each side of a 5x1x5 floor).
  - Checked by hand-aiming (scratch script, not committed) that the click itself is fine: from the clamped position the ray hits the east face of the floor block at the edge at pitch 77 to 80 degrees, and playing it in mc_server2 placed the stone with 0 corrections (`5,-1,2 = minecraft:stone`).
  - Looked for another stand/click that passes the rule: none exists, since the clamped position is the furthest out the sneaking player can be.
- Evidence (`node --test test/withheld/bridge.sim.test.js`, and the hand-aimed run):

  ```
  AssertionError: bridge: no click for [5,-1,2] from [5.293494471749092,0,2.5] at the sneaking eye height
  hand-aimed: point y=-0.05 -> yaw 90 pitch 77.46 hit {x:4,y:-1,z:2} east ; y=-0.5 -> pitch 80.59 hit east ; corrections 0 ; 5,-1,2 = minecraft:stone
  ```

  Cause: `solver.js` `solveWith` skips every candidate with `u.dot(n) > -0.25` ("must look into the face", about 75 degrees from the face normal). The clamped player's eye is only 0.2935 past the edge and 1.27 above the top of the block, so the best case (the top edge of the east face) gives u.dot(n) = -0.225, and every other point on the face is more grazing. Real bridging looks like that, vanilla accepts it, the rule is a solver safety margin.
- Status: `trace.bridge` and `solver.solveFromStands` are committed (`solveWith` got two optional fields, `spots` and `eyeHeight`, defaults unchanged, `actions.test.js` passes). It currently returns `{ok: false, unavailable: true, reason: 'bridge: no click ...'}`. The oracle is `online/test/withheld/bridge.sim.test.js` (outside the `test/*.test.js` glob so the suite stays green).
- Question for Allen: may the grazing limit be a parameter of `solveFromStands` (default -0.25, so `solve` and E1 stay identical) with a looser value for bridging (for example -0.2; the hand-aimed click that the sim accepted had u.dot(n) of about -0.21)? Or should bridging take its click from somewhere other than the solver (the face is always the side face of the block you stand on, from the clamped spot)?
- Resolved by orchestrator: grazing option, -0.2 for bridge. (`solveFromStands` takes `grazing`, default -0.25; `trace.bridge` passes -0.2; the oracle is back in `test/bridge.sim.test.js`.)

## E2 monotonicity property: E9, the statement as written is false

- Task: E2 (`lib/witness.js`). `witnesses`, `reachable`, `veto` and the consistency property are done and pass; only the monotonicity property is blocked.
- The plan says: "Adding blocks never increases a pending step's witness count, except by creating a new reference."
- Tried:
  - Tested it literally on 12 random worlds, 2 pending steps each, 6 added stone blocks per step (`online/test/withheld/witness_monotonicity.test.js`, test 1). It fails: of 72 additions, 5 created alive witnesses that do not click x.
  - Looked at each one. There are three ways a block creates a witness without being its reference: it is the floor of a new stand (stand (6,3,5) over x=(6,2,5)); it makes a stand reachable that was not (connectivity: x is floor on the route, a witness elsewhere comes alive; first failing case: stand (2,1,2) clicking (4,-1,2) after x=(1,3,2), `wasReachable: false`); and it changes which route or pillar the flood fill finds first.
  - A narrower statement holds on the same worlds (test 2, passes, 47 created witnesses, 0 outside the rule): witnesses from real stands, ignoring reachability, grow only through x as the reference or as the floor of the stand.
- Evidence (`node --test test/withheld/witness_monotonicity.test.js`):

  ```
  1. (the plan) ... AssertionError: 5 alive witnesses created otherwise, first: {"y":"s1","x":[1,3,2],"witness":"2,1,2|sneak|4,-1,2|up","viaRef":false,"viaFloor":false,"wasReachable":false}
  2. (narrower) ... ok (47 witnesses created by 72 additions; not via reference or floor: 0)
  ```

- Consequence for the veto: it is conservative by construction. It looks only at the witnesses that exist before x (a stand that x newly creates is ignored), and exempts y when x is itself one of y's references. That is what the consistency property tests, exactly, against a from-scratch recomputation (107 queries, 29 vetoes, 0 disagreements).
- Status: committed without the monotonicity test in the suite; the withheld file holds both tests.
- Question for Allen: is the narrower statement (or "the veto is conservative: witnesses that x newly creates are not counted") the intended property? If the alive-witness claim is meant literally, the design must say what a block that adds floor or connectivity does to witness counts.

Resolved by orchestrator (E2 monotonicity): the plan's statement is wrong; the narrower property (growth only via x as reference or as the stand's floor, ignoring reachability) is the tested one, in `test/witness_monotonicity.test.js`. The veto stays conservative because it ignores stands x creates.
