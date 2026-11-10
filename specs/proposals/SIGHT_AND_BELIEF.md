# Sight mode and hidden state: what a bot may know

> **Status (2026-11-10): proposal, decisions open (§10).** Design level only: the exact code is decided when each part is built. Nothing here is built. It builds on B4 §4.11 (sight mode, Allen, 2026-10-19), B4 §4.13 (underground hazards), F2 §4.1 (the `how` of an observation) and `crates/null-belief`, which has no `how` yet. It is needed before `mine` (OPEN §1 item 3).

## 1. Why

On a server with anti-x-ray, the chunk data for a hidden block can be fake, and a bot that acts on hidden blocks gets banned. Allen's rule (B4 §4.11): **in sight mode, no decision depends on a block the bot could not have seen.**

B4 §4.11 applies the rule to targets: a bot mines only ore it saw. But the rule says "no decision", and three more kinds of decision read blocks:

- **Hazard checks.** B4 §4.13 says the miner checks the neighbours of the cell it opens, before the break. Those neighbours are behind the rock. The bot has not seen them. So §4.13 as written breaks §4.11.
- **Paths.** The pathfinder reads every loaded block. It can plan a tunnel straight to a cave it has never seen. That is x-ray too.
- **Belief.** `null-belief` records what `block_at` returns, with no note of how the bot came to know it. Under anti-x-ray it stores fake ore as fact, and counts built on it are wrong.

This proposal makes the rule hold for every decision, with one mechanism.

## 2. Two questions about a block

They are different, and today's design mixes them.

| Question | Why it matters | Answer comes from |
|---|---|---|
| **Is the data real?** | Anti-x-ray sends fake data for hidden blocks | How the bot got it |
| **Could a player know it?** | A decision on knowledge a player cannot have looks like x-ray, even when the data is real | Sight |

| How the bot got the block | Real? | A player could know it? |
|---|---|---|
| Chunk data, block not exposed | Not known | No |
| Chunk data, block exposed, no clear ray from the eye | Yes | No (B4 §4.11: the ray decides) |
| A clear ray from the eye reached a face | Yes | Yes |
| A block update for a cell out of sight | Yes | No |
| The server's result of the bot's own break or place | Yes | Yes |
| The cell the bot stands in or on | Yes | Yes |

## 3. The rule as a view

**The client reads the level. The mind reads the seen view.**

- **The client** is everything Null computes as a vanilla client: physics, collisions, the crosshair, prediction. It reads the bot's own `World`, as a vanilla client does. Sight mode changes nothing here.
- **The mind** is every decision above the client: skills, target choice, hazard handling, the pathfinder, belief. In sight mode it reads the **seen view**: a cell is its block if the bot may know it (the "yes" rows of the last column in §2), and **unknown** if not.

Unknown is a value of its own, not air and not stone. Each reader states what it does with unknown (§5, §6). With this, a new skill cannot break the rule by accident: it has no way to read an unseen block.

With sight mode off, the seen view is the level, and nothing changes.

## 4. How a cell becomes seen

1. **Candidates by exposure.** Only a block with a face on a transparent cell can be seen. This is cheap to test and removes almost all cells under ground.
2. **Confirmation by ray.** A candidate is seen when a ray from the bot's eye reaches one of its faces with nothing opaque in the way, within render range (B4 §4.11). The bot's look direction does not matter: a player can turn its head.
3. **When.** After the bot moves, and after a cell near it changes. Nearest first, within a work budget per tick. A block that the bot's own digging exposes counts once the server has sent it and the ray confirms it.
4. **How long.** A sighting holds until the cell changes. A change that the bot does not see makes the cell unknown again.

Sightings are per bot by default. The swarm-wide option of B4 §4.11 lets a bot use another bot's sighting from belief; that is a policy of the job.

## 5. Hazards in sight mode: no look-ahead

A player cannot check what is behind the block it is about to break. It mines so that the worst case is survivable, and it reacts when the break shows something. Sight mode does the same:

- **Before a break,** the miner uses only seen cells. Every unknown neighbour of the cell it opens counts as the worst case for its position: a fluid at the side or above, a falling block above, a drop below.
- **So the method sets the posture.** The presets of B4 §4.12 must be safe under the worst case: never dig the cell under the feet while standing on it, never dig straight up, keep a seal block in the hotbar, keep a retreat cell behind.
- **After a break,** the newly exposed cells become seen at once (§4). If one is a hazard, the miner handles it then: seal the fluid, step back from falling blocks, cover the hole.

This replaces "checks the cell it opens and that cell's neighbours before each break" in B4 §4.13 for sight mode. With sight mode off, the look-ahead of §4.13 stays as written.

The cost is that a sight-mode miner sometimes takes damage a look-ahead miner would avoid. That is how a player mines, and B2's survival reflexes cover it.

## 6. Paths in sight mode

The pathfinder plans on the seen view. An unknown cell is solid rock of unknown kind: it can be dug at a default cost, inside the bot's claim, and it is never a known opening. So the bot does not tunnel towards a cave it has not seen, and it does not walk into a region whose blocks may be fake.

The fork's pathfinder reads the world directly, and its passability test is not pluggable (DECISIONS AD-B1.1). So this needs a small fork change: the planner reads blocks through a view that the bot supplies. On the surface, where sight mode is off, the view is the level.

## 7. Belief

- **Every entry records how it was learned** (`how`, F2 §4.1) and whether a sighting holds for it.
- **Queries choose their trust.** A decision asks for seen entries. A count for the operation's ledger asks for real entries. Nothing asks for "any" by default.
- **Fake data never enters as fact.** Chunk data for a hidden block is stored as not known to be real, or not stored at all. Step 1 of §9 decides which, by memory cost.

**Anti-x-ray is a belief about the server** (F5 §4.6, server settings as beliefs). The default belief is that it is on. Evidence comes free: each time a hidden cell becomes exposed, compare what the chunk data said with what the server now sends. A difference is proof that the server fakes hidden blocks, and its direction shows the mode: stone that turns out to be ore, or ore that turns out to be stone. No difference over many exposures is evidence that it is off. The belief changes what the operation may count, never what a sight-mode bot may decide.

## 8. Cost

- **Memory:** one bit per cell, kept only for sections that hold a sighting. Under ground the seen set is the walls of the tunnels. On the surface it would be large, and surface work runs with sight mode off.
- **CPU:** rays to the candidates near the bot after each move. The work budget per tick bounds it; a late sighting only delays a target. To measure in the first `mine` rows.

## 9. Build order

1. **`how` and the sighting flag in `null-belief`,** with queries by trust. Decision crate, offline tests.
2. **The seen view in the bot API:** block reads for skills return unknown for unseen cells in sight mode. The ray uses the fork's existing clip code through the adapter.
3. **Sightings:** candidates, rays, budget, invalidation. Rows on both servers: a block behind a wall is unknown, a block seen through glass and through water is seen, an exposed block around a corner is unknown.
4. **Hazard rules of §5 in the `mine` presets,** with the live rows B4 §4.13 already requires, in both modes.
5. **The pathfinder view** (fork patch), with a row: a cave behind rock is not a route.
6. **Anti-x-ray evidence,** with a row on the Paper server with its anti-x-ray engine on.

Step 6 needs a Paper server with anti-x-ray enabled. The Paper + Grim test server can turn it on for those rows.

## 10. Decisions for Allen

1. **Sight mode is a view for every decision above the client** (targets, hazards, paths, belief), not only a filter on targets. Recommended: yes. It is what "no decision" in the rule says.
2. **Hazards in sight mode have no look-ahead** (§5): worst-case posture, then react. This changes B4 §4.13 for sight mode. Recommended: yes.
3. **The pathfinder plans on the seen view** in sight mode, by a small fork patch. Recommended: yes.
4. **Anti-x-ray is believed on by default,** and the bot gathers evidence at each exposure. Recommended: yes.
5. **A sighting needs a clear ray, not only exposure.** This is the rule of 2026-10-19, kept. The alternative (any exposed block in the same open space counts) is cheaper and lets a bot target ore around a corner. Recommended: keep the ray.
