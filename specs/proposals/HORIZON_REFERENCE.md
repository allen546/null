# Horizon as a reference: missing parity and a second opinion

> **Status (2026-11-15): proposal, decisions open (§9).** Nothing is built. Facts were read on 2026-11-15 from a blobless shallow clone of `ludovic111/horizon` at `~/Desktop/null-spikes/ext/horizon` (read only, nothing run). Allen raised it on 2026-11-15: use Horizon's code for the parity Null has not built, and as a comparison to Null's.

## 1. The question

Horizon is a Rust rewrite of the Minecraft Java client, much further along than Null's fork. Can Null use it to fill its parity gaps faster, and to check its own code?

## 2. What Horizon is

- A full game client (rendering, UI, launcher, integrated server, LAN host), about 50 crates. Created 2026-10-03, about 300 contributors, about 10,000 stars, MIT licence for its own work. It is AI-assisted (it has `CLAUDE.md`, `AGENTS.md` and a `vanilla-parity` skill).
- Game logic is vanilla **26.3**. `horizon-protocol` renumbers and rewrites packets for older servers down to 1.21.11 and 1.20.2, but the logic behind them stays 26.3.
- Parity method: about 281 differential harnesses (`tools/parity.py`, `tools/test_*_parity.py`) compare Rust probes with the vanilla jar method by method, at the bit level, with edge inputs (NaN, infinity, -0.0, 1e-40, coordinates at 30,000,000). A port ledger (`tools/port_map/*.tsv`, 14,897 rows) pairs each vanilla method with the Rust function that ports it, or marks it missing. Its own docs say about 1 in 12 rows is wrong.
- `horizon-bots` is a load tester only (creative bots that fly and hold no world). `horizon-anticheat` is for Horizon's own server. Nothing covers Grim or exact client timing.
- Measured cost: the client is about 1,080 MB working set at render distance 8 (with rendering). No headless figure exists. Null's own-world design is about 1.2 MB per bot.

## 3. Why not a switch

1. **Version.** Null computes what a vanilla 1.21.11 client computes, and Grim on our servers judges against 1.21.11. Horizon on a 1.21.11 server behaves like 26.3.
2. **Cost per bot.** Horizon is built as one player's whole game; its gameplay is not split from its renderer and integrated server.
3. **Grim and timing.** Null's hardest work (tick timing, refusals, beliefs about server state) is not in Horizon.

So Horizon stays outside Null's build. It is a reference.

## 4. Use 1: the reference for parity Null has not built

**Rule: reference only.** An implementer reads Horizon's port to find the vanilla method, its order, its edge cases and the state it touches. Then the implementer writes Null's own code in the fork's style. No Horizon code is copied into the fork or Null. Why: Horizon translates each vanilla method closely (same order, constants and random draws), so its code is close to Mojang's in structure, and Null's rule is never to copy Mojang code. Its code is also not Bevy ECS, so a copy would not fit.

**Version gate.** A Horizon port helps only where the vanilla method did not change between 1.21.11 and 26.3. Decompile the 26.3 jar into `~/Desktop/null-reference/` beside `vanilla-1.21.11-src/`, and diff method by method. Unchanged: Horizon's port is a valid reference. Changed: read the 1.21.11 method, and use Horizon's only for structure.

**Proof stays ours.** Every change built from a Horizon reading gets a test against real vanilla 1.21.11 (an oracle in `harness/vanilla-*`, or an existing test with vanilla's values). Horizon's agreement is never the proof. The ledger row says "How found: Horizon reference" and names Horizon's function.

**Gaps it covers today** (looked up in the port map, 2026-11-15):

| Null gap | Vanilla method | Horizon |
|---|---|---|
| Piston moves (`BLOCK_ENTITY_STATE.md` §2, large, not built) | `PistonMovingBlockEntity.tick`, `.moveCollidedEntities` | `horizon-world/src/level/pistons.rs`, `horizon-client/src/game/piston_push.rs` |
| Beds place two cells (`Unknown`) | `AbstractBedBlock.getStateForPlacement` | `horizon-world/src/level/place.rs` (one gap listed: world border) |
| Elytra flight (`null-features-elytra`, offline only) | `LivingEntity.updateFallFlyingMovement` | `horizon-client/src/motion.rs` |
| Prediction handler (patch 76) | `BlockStatePredictionHandler` | `horizon-client/src/game/remote.rs` |
| Window clicks | `AbstractContainerMenu.doClick` | `horizon-client/src/menu.rs` (inline) |
| Firework boost, redstone wire placement | `FireworkRocketEntity.tick`, `RedStoneWireBlock.getStateForPlacement` | Not in the map |

## 5. Use 2: a second opinion and test inputs

- **Vanilla is the reference; Horizon is a second opinion.** The oracles run vanilla's own code, so a disagreement with vanilla is a bug, and a disagreement with Horizon is a lead. Horizon helps most where Null has no oracle yet.
- **Edge inputs.** Feed Horizon's kinds of edge values to Null's oracles: NaN, infinity, -0.0, subnormal values, coordinates at the world limit. The move oracle used ordinary values only.
- **A checklist.** Horizon's harness list (281) and its area table (`docs/architecture.md`) name areas Null tests nowhere. Each one Null needs becomes an oracle or a row.
- **The ledger idea.** Null can keep its own table of vanilla methods and the fork functions that compute them, with gaps (the `Spec:` and vanilla-method comments are most of it already). This fits `TEST_GATE.md`.

## 6. What to build

1. The 26.3 jar, checked by hash, decompiled with the tool used for 1.21.11, into `~/Desktop/null-reference/vanilla-26.3-src/`, with a `MANIFEST.md` row.
2. A method diff (uv Python): for each vanilla method, unchanged, changed or new, between 1.21.11 and 26.3. Bodies compared after normalising names the decompiler made up.
3. The join: Null's gaps (OPEN §4 "Parity gaps", REFUSED_ABILITIES, `Unknown` place families, BLOCK_ENTITY_STATE kinds), each with its vanilla methods, changed or not, Horizon's function and its port-map verdict, and Null's state. Output: `specs/reference/HORIZON_MAP.md`.

## 7. Spike

Steps 1–3 of §6, then one gap chosen from the table. Done when the table exists and lists every gap in OPEN §4 and REFUSED_ABILITIES with a row. No fork change in the spike.

## 8. Risks

| Risk | Answer |
|---|---|
| A Horizon port is wrong (1 in 12 map rows) | Our vanilla test decides, never Horizon |
| A change between 1.21.11 and 26.3 is missed | The method diff, and the vanilla test |
| Code drifts toward a copy | Rule in §4; the verifier compares the fork change with Horizon's function and fails a near copy |
| Horizon moves fast | Pin the commit read in each ledger row |

## 9. Decisions for Allen

1. **Reference only** (§4): read Horizon, write our own code, cite it; never copy. If yes, CLAUDE.md "The Azalea fork" gets one line. Recommended.
2. **26.3 sources** in `~/Desktop/null-reference/` for the method diff. Recommended.
3. **The spike** (§7), by a Sonnet agent, no live run. Recommended.
4. **First gap after the spike:** piston moves, unless the table shows a cheaper gap that blocks `mine`.
