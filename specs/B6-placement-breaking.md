# B6 Placement and breaking

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.9, §9, §10; ONLINE_BUILDER.md §0, §2, §2b, §3.3; reference/builder-hard-cases.md §3.1, §3.5. Conventions: `CONVENTIONS.md`.
**Depends on:** B1, B3, F2, F5. **Used by:** C1 (builder), C2, B4, everything that places or breaks a block.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Reach and stand checks | Deterministic | Eye position, reach 4.5, collision shapes |
| 4.2 | Click choice and placement recipes | Deterministic | Probe the game's own placement logic per version |
| 4.3 | Follow-up clicks (`tune`) | Deterministic | Each tunable property has a fixed click count |
| 4.4 | Break timing | Deterministic | Break time is computed from block hardness, tool, enchantments and effects |
| 4.5 | Drop collection | Deterministic | Pick up within a radius, count, report |
| 4.6 | Acknowledgement and readback | Deterministic | Wait for the server's block-change sequence; compare with the expected state |
| 4.7 | Packet replay executor | Deterministic | Send a certified stream open-loop, halt on any divergence |
| 4.8 | Rate and rotation limits | Measured | Stay within what a vanilla client produces; limits measured per server |
| 4.9 | Unreachable states | Deterministic | Detected and reported; resolving them is an input to C1 (§4.5) |

## Architecture decisions

### AD-B6.1 Precompute placement recipes per version, or probe them on demand? (decided)

**Decided by Allen, 2026-10-15: (c), a table for common states and probing for the rest.** No ⚑. The options were (a) a full table per version, (b) probing on demand, and (c) both. B6's interface is the same for all three.

---

## 1. Purpose

B6 is the set of low-level actions the builder and gathering use: choosing a click that produces the intended block state, breaking with the right tool in the right time, collecting the drops, and confirming with the server that it happened. It also runs the builder's certified packet streams on the live server.

## 2. Scope

**Owns:** reach and stand checks; click choice; placement recipes; tune clicks; break timing; drop collection; acknowledgement waits and readback; the packet replay executor; click and rotation pacing.

**Does not own:** build order, certification or repair (C1); paths to stands outside replay (B1); which items to carry (B3, C3).

## 3. Interfaces

Skills:
- `place(cell, target_state, options)` → placed state as read back, or `failed(reason)`.
- `tune(cell, property, value)`.
- `break(cell, tool_policy, collect: bool)` → drops collected.
- `replay(stream, step_id)` → `completed | halted(divergence)`.

`target_state` is a full block state. `options` can force a stand, posture, face or look, which the builder supplies from its plan.

## 4. Requirements

### 4.1 Reach and stand checks

- Reach measured from the eye at the click's posture (standing 1.62, sneaking 1.27) to the click point, ≤ 4.5 by default (ONLINE_BUILDER §2b), configurable lower per server profile.
- Vanilla refuses to place a block whose collision box intersects the bot; B6 checks this before clicking.

### 4.2 Click choice and placement recipes

- A recipe per target state: item, clicked face, hit point, look yaw and pitch, sneak, and follow-up clicks.
- Recipes are computed by running the game's own placement logic against the planned look and neighbours (Baritone's `approxPlaceable` idea, probed per version in the simulator), not by hand-written facing rules (builder-hard-cases §3.1, §5.3).
- Placing clicks are sneaked so they never use the clicked block (ONLINE_BUILDER §2b). **Crouch handling (Allen, 2026-10-18):**
  - Vanilla rule: a click with an item in either hand while crouching skips the clicked block's own action and places (`MultiPlayerGameMode#performUseItemOn`). The crouch must reach the server at least one tick before the click.
  - Fork (mechanism): a generated per-version table of blocks that use a click (block classes that override `useWithoutItem` or `useItemOn`), exposed as a query ("does this click need a crouch?"). The place command takes `auto_crouch`. With it off, a click on such a block while not crouching is refused with a reason and nothing is sent. With it on, the command crouches, clicks on the next tick, then restores the previous crouch state: for one-off placements.
  - **Temporary: the table is wide (Allen, 2026-10-18).** It works by class, so it also lists blocks that use the click only for some items or states: a fence (lead only), a cauldron (buckets and bottles), a campfire (food), an iron door (never by hand), redstone wire (dot or cross only). A standing click on these with a block in hand places in vanilla; the bot refuses it, or crouches first with `auto_crouch`. That is safe (no wrong packet) but costs extra crouches and refuses some clicks a player makes. Narrowing it to item and state conditions is roadmapped for step 6, builder integration (BUILD_PLAN), when placement volume makes the cost real.
  - Skills (policy): a placement sequence holds the crouch itself and passes `auto_crouch` off. It stays crouched while the next placements need it and are in reach, and stands up only before a longer walk (crouching walks at 30%), with a short delay so the crouch does not flicker between blocks. Only the skill sees the sequence, so the smoothing is not in the fork.
- Placing a block or using a block is `UseOnCrosshair` or a face-specific `UseItemOn`; an item whose effect doesn't depend on the block (bucket, pearl, food, bow) is `UseItem`. Azalea never sends `UseItem` while a block is under the crosshair, so picking the wrong command silently does nothing (F5 §4.1, spike §6.3).
- A recipe's face and hit point are produced by setting the look and letting Azalea's raycast make the hit, or by sending `ServerboundUseItemOn` with the recipe's values. Never Azalea's `force_block`, which invents a hit point (block centre, face up) (Azalea reality check, 2026-10-13).
- Vanilla survival only: state comes from look, face, hit point and sneaking (tier C). No accurate-placement protocol.

### 4.3 Follow-up clicks (`tune`)

Repeater delay, comparator mode, note pitch, door, trapdoor and gate state, item frame rotation, sign text, container contents (via B3 `fill_exact`). Tune clicks are unsneaked (vanilla skips block use when sneaking with an item). Each tune click is an update and, near sculk sensors, a vibration; the builder orders them.

### 4.4 Break timing

- Break time from hardness, tool tier and type, Efficiency, Haste, Mining Fatigue, on-ground and underwater penalties, all from F5 data.
- Hold the break for exactly the computed ticks plus a margin for latency; don't spam.
- Choose the tool by policy (fastest, cheapest durability, silk touch or fortune when the drop matters).

### 4.5 Drop collection

After a break, collect item entities within a radius before moving on. In survival, drops from the bot's own breaks are intended changes the builder accounts for (builder-hard-cases §5.1), and B6 reports what it collected and what it missed.

### 4.6 Acknowledgement and readback

- Wait for the server's block-change acknowledgement (sequence number) before any action that depends on the placed block.
- Read back the resulting state from the client's chunk data; mismatch → `failed(state_mismatch, got)`.

**The result mailbox (AD-F5.4, built 2026-10-19).** Every command that has a result returns an `ActionId`: `break_block`, `place_block`, `open_inventory`, `open_container`, `open_entity`, `click_slot`, `respawn`. Ids come from one counter for the whole process and are never reused, so an id from before a reconnect never matches a new one. A skill reads the result with `BotView::action(id)` as often as it likes:

- `Pending(Option<record>)`: not finished. The record is the partial one (a break in progress shows `finish_tick` once the finishing packet is out). The id a step has just got reads pending in that same step.
- `Finished(record)`: the final record. A client-side refusal (nothing sent) is a finished result like any other; its reason is in the record (`BlockAction.reason`, `OpenResult.refused`, `ClickOutcome.refused`, `RespawnSent.refused`). There is no separate refusal query.
- `Expired`: released, pruned, or never issued. It never reads as pending.

One enum, `ActionRecord`, covers the record types: `Block(BlockAction)` for breaks and places, `Open(OpenResult)` (target `Block`, `Entity` or `Inventory`), `Click(ClickOutcome)`, `Respawn(RespawnSent)`. Whether to wait for an ack is the skill's choice for each action: it may send the next break as soon as the finishing packet of the last one has gone out and read every id later.

*Owner and pruning.* The owner of a command is the skill instance or the reflex that sent it; `Brain::tick` stamps the owner (`BotCommands::set_owner`), the host keeps one mailbox per bot (`null_bot_api::Mailbox`, pure and unit-tested). Nothing is pruned by time.
- Pending results are never pruned and never cause a refusal. Each kind finishes within a bounded time (table below), so their number is bounded by the send rate.
- The owner releases a finished result after reading it: `BotCommands::release(id)`, which takes effect at the end of the tick. Releasing a pending result drops it when it finishes. A stranger's `release` is ignored.
- All of a skill's results go when the skill ends (`Done`, `Failed`, replaced by `BotBrain::set_skill`, or `AfterDeath::End`). A skill that returns `AfterDeath::Keep` keeps its results (and its pending tracks) across the death; the reflexes' results do not cross it.
- All results are cleared at a disconnect (the brain is removed).
- The owner may declare how many finished results it expects to hold at once: `BotCommands::expect_results(n)`, default 64. Going over logs one warning per owner (tracing `warn`) and prunes nothing.
- Leak guard: 4,096 unreleased finished results end a skill with `Failed("result leak")` (`SKILL_LEAK`). A reflex cannot end, so the host turns it off at bot scope (`Scope::Bot`, `Target::Reflex(id)`), drops its results and emits `ReflexDisabled` (as `HostEvent::ReflexDisabled`).
- The history is a mailbox, not a log: a skill that wants totals keeps its own tally.

*Finish bounds.* The fork keeps one record per kind and settles only the newest, so the adapter copies each record into the mailbox every tick (`mc-adapter/src/actions.rs`) and bounds what the fork does not.

| Kind | Finishes when | Bound |
|---|---|---|
| break | the fork settles it: acknowledged (`Done` or `Refused`), refused at once by a client check, or `Cancelled` (`cancel_break`, or mining ended without a finishing packet) | the fork's ack timeout, 60 ticks after the finishing packet (`Refused`, "no ack"); no finishing packet within 6,000 ticks (5 minutes, past the longest vanilla hand-break) and the host cancels it (`Cancelled`, "too long") |
| place | acknowledged or refused, as for a break | 60 ticks after the click (the fork's ack timeout) |
| `open_container`, `open_entity` | the server opens a window (`Opened`), the ack comes with no window (`NoScreen`, 4 ticks of grace after the ack), or a client check refuses | the fork's 60-tick ack timeout, then judged on the ack alone |
| `open_inventory` | local, no server round trip: `Opened` or `Refused` in the tick after the command | 1 tick |
| `click_slot` | refused at once by a client check, or 8 ticks after the click (the server's corrections come within a few; the ability check waits the same) | 8 ticks |
| `respawn` | refused at once (not dead, hardcore), or the server's respawn packet arrives (`completed_tick`) | 600 ticks without the packet: finished with `refused: Some("no answer")` |
| any command whose fork result never appears (the fork dropped it) | `Refused`, reason "not started" | 20 ticks after the command |

*Limits of one record per kind.* Repeated `break_block` commands for the block already being broken, and repeated `respawn` commands for one death, name the same action: their ids finish together with the same record. A newer command of the same kind that replaces a queued one before it starts finishes the older as `Refused`/`Cancelled` ("replaced ... before it started"; nothing was sent). When a newer action of the same kind replaces the fork's record after the packet was sent and before the server answered, the older finishes as `BlockOutcome::Superseded`, `OpenOutcome::Superseded` or `ClickOutcome.superseded`: the outcome is unknown, read `block_at` or `window`. This needs an action sent in the tick right after another of its kind before the first is acknowledged: two places, opens or clicks, or two instant breaks (an instamine sets no destroy delay, as in vanilla, so a chain sends one break per tick). A fork patch that settles every in-flight action by sequence (the server's ack is cumulative) removes this case; it is the next step (2026-10-19). A click record holds the corrections seen until the next click.

*Tests.* Unit: `null-bot-api/src/action.rs` (one test per rule: release, skill end, kept skill, disconnect, expected count warning once, leak guard, expired and pending ids), `fake.rs` (`FakeView::finish`, `set_pending`, `expire` for skill tests), `reflex.rs` (owner stamping); simulator: `mc-adapter/tests/sim.rs` (id pending at once, finished, expired after release; skill leak ends the skill; reflex leak turns the reflex off). Live: A13 (`scenarios_mailbox.rs`): 16 breaks sent without waiting for an ack, all read by id at the end; a place sent right after a break.

### 4.7 Packet replay executor

For certified builder steps (ONLINE_BUILDER §0 S3):
- Send the stream open-loop, one packet group per client tick, with B1 in replay mode and every reflex off for the job.
- Before a movement that depends on a just-placed block, wait for its acknowledgement.
- Halt on any server position correction.
- Watch every block-change packet in the work envelope and margin, and halt on the first change not in the step's certified change sequence.
- After the step, wait the certified settle time plus a margin, in game ticks.
- Before each step, check no other player or mob is within the step's trigger range; if one is, pause at the step boundary.
- Report `completed` with the readback, or `halted(divergence, first_diff, tick)`.
- Raw sending exists in Azalea (`write_packet`), but stopping Azalea's own movement output during replay needs the fork patch in B1 §4.8 (Azalea reality check, 2026-10-13).

### 4.8 Rate and rotation limits

Clicks per tick and rotation speed stay within what a vanilla client produces; look changes are interpolated over a few ticks rather than snapped (builder-hard-cases §3.5). Azalea's `look_at` snaps instantly, so interpolation is ours (Azalea reality check, 2026-10-13). Limits are per server profile and measured in the harness; the builder's plan respects them.

### 4.9 Unreachable states

Some states can't be produced by any placement (extended pistons with no power, lit lamps with no power, update-trick states). B6 detects when no recipe exists for a target state and returns `failed(unreachable)`. Resolving these is an input C1 takes from a person (macros, substitutions, C1 §4.5–4.6).

## 5. Failure handling

| failure | response |
|---|---|
| Placement rejected or wrong state | `failed(state_mismatch)`; outside replay, the caller decides; in replay, halt |
| Break cut short | Resume; if it can't resume, the job ends `interrupted(incident)` and is contained (INTERRUPTS §2.2) |
| Drop lost (despawn, fell into lava) | Report to the ledger as lost |
| Divergence during replay | Halt immediately; C1 repairs or reports stuck |

## 6. Scale and performance

Pacing is per bot. Replay throughput is bounded by the server's tick rate and settle times, not by B6.

## 7. Version-sensitive parts

Placement logic, block states, break speeds, drop rules, acknowledgement protocol, via F5 and simulator probing.

## 8. Tests

- Recipe probe: for every block state in the builder corpus, the recipe produces that state in the harness, or it's listed as unreachable.
- Break timing: every vanilla tool and block combination breaks within one tick of the computed time.
- Replay: a certified step with an injected foreign block change halts on the first changed packet.

## 9. Open questions

None. AD-B6.1 was decided 2026-10-15.
