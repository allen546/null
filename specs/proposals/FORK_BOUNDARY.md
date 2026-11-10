# The fork boundary: which parts of Azalea are Null's own

> **Status (2026-11-10): adopted by Allen (§8: yes to all four). CLAUDE.md and F5 §4.7 carry the new rules; no code changed.** Design level only. The numbers were measured on 2026-11-10 in `~/Desktop/null-spikes/azalea`: branch `patch-stack-burst` (`ee7d418`) against its base on upstream's `1.21.11` branch (`41a9ae6`, 2026-03-22), and against upstream `main` as the clone last fetched it (`0d38e0d`, 2026-10-16). It answers the question deferred in DECISIONS on 2026-10-16 ("replace Azalea's stateful client glue with our own") and fits under AD-F5.5.

## 1. The question

Null keeps a fork of Azalea with one commit per patch, and the standing rule is to keep patches small and mostly additive so that rebases onto a new upstream stay cheap. Upstream takes no AI-written contributions, so no patch ever goes back.

On 2026-10-16 Allen raised and deferred: own the stateful client glue, or the whole client. The trigger was "the patch count, or a bug that can't be patched". Both have arrived:

- 54 patches are pushed and the stack holds 18 more.
- Three adopted designs cannot be small additive patches: own worlds with the client level model, block-entity state, and shards. The load contract audit also found packets handled out of arrival order (L2), which is how the client is built, not a bug in one place.

## 2. What the fork is today

Lines changed in `src`, stack against base (tests and data files not counted):

| Crate | What it is | Lines of source | Null added | Null removed |
|---|---|---|---|---|
| `azalea-protocol` | Packets, connection | 8,400 | 64 | 6 |
| `azalea-registry`, `azalea-core` | Game data, basic types | 23,300 | 210 | 56 |
| `azalea-block` | Block states and behaviours | 10,700 | 5,227 | 45 |
| `azalea-inventory` | Items, components, menus | 10,500 | 880 | 390 |
| `azalea-physics` | Collision, travel, fluids | 18,700 | 1,124 | 240 |
| `azalea-entity` | Entity state and systems | 19,200 | 2,150 | 204 |
| `azalea-world` | Chunks, worlds | 3,200 | 597 | 12 |
| `azalea-client` | The client: packet handling, movement, mining, clicks, the loop | 18,300 | 9,294 | 530 |
| `azalea` | Swarm runner, pathfinder, helpers | 12,200 | 1,691 | 411 |

About half of the client crate is Null's. The protocol and the generated game data are almost untouched.

**Upstream since the base** (59 commits in about 7 months): the client changed by under 500 lines; physics and inventory changed by thousands, mostly regenerated data. 122 files changed on both sides.

**A trial merge of upstream `main` into the stack** (made on a scratch copy and discarded): 43 files in conflict, 95 conflict hunks. 11 files are in the client, 14 in `azalea` (the pathfinder and helpers), 6 in physics.

## 3. Three layers

| Layer | Crates | What upstream gives | What Null needs |
|---|---|---|---|
| **Data and wire** | `azalea-protocol`, `-buf`, `-registry`, `-core`, `-chat`, `-auth`, `-nbt`, and the generated parts of `-block` and `-inventory` | Every game version's packets and data, by its code generator. This is the costly part to make, and upstream does it for each version | Exactly this, unchanged |
| **Simulation** | `azalea-physics`, `azalea-entity`, `azalea-world`, the behaviour parts of `-block` and `-inventory` | A good start, close to vanilla | Exact vanilla results. Null's changes here are corrections, each checked against a vanilla dump or method |
| **Client and bot** | `azalea-client`, `azalea` (runner, pathfinder, helpers) | A convenient bot library: events in queues, a shared world for swarms, approximations where exactness is costly | What a vanilla client decides and sends, tick for tick. This is Null's product (F5 §4.6, "What Null computes") |

## 4. The decision

**Null owns the client and bot layer, corrects the simulation layer, and takes the data and wire layer from upstream unchanged.**

In practice:

1. **Data and wire: track upstream.** No Null change, apart from a bug that blocks work. Each such change is listed, because it is the only kind that costs at every version.
2. **Simulation: upstream is the reference, the vanilla tests decide.** On an upstream update, a conflict is settled by the tests that hold vanilla's values (shape dumps, component hashes, physics traces), not by whose code it is.
3. **Client and bot: Null's own code.** It may be restructured where vanilla's behaviour needs it. The rule "small and mostly additive" ends for this layer. Upstream's changes here are read at each update for what they say about a new game version, and applied by hand where needed.
4. **Still one public fork, still MIT, still credited.** Nothing moves to a new repository. Only the rules per directory change.

**Not chosen.**

- *A whole client of our own.* The data and wire layer is the part that follows each game version, and upstream maintains it. Rewriting it buys nothing.
- *Staying with small additive patches everywhere.* It cannot hold the three adopted designs, and it keeps the client in a shape where packets are not handled in arrival order.
- *Moving the client into the Null repository.* It would break the boundary rule that only `mc-adapter` and `null-node` import Azalea, and it changes nothing about the cost.

## 5. Updates: merge, never rebase

Today's rule implies re-applying each patch on a new upstream. With 72 patches and more to come, one conflict can repeat down the chain.

**New rule: an upstream update is one merge into `null-main`.** Conflicts are settled once, against the final code. The trial in §2 gives the size of one such update after 7 months: 43 files, 95 hunks. The cost follows how much both sides changed the same lines, not how many patches Null has.

- The one-commit-per-patch history stays as it is in `null-main`. A merge commit is added on top.
- The ledger stays. Its column "bug fix or Null-only" loses its purpose, since nothing goes upstream. It is replaced by the layer the patch touches.
- An update is done when the tests of rule 2 pass, the recorded sessions pass (TEST_GATE), and a pair passes.

**A new game version** is the same merge, plus the work F5 §4.7 lists. The recorded sessions are recorded again on the new version, and they are the acceptance test for the port: it is done when they pass. That makes a port a job for machines and implementers, with a clear end.

## 6. What changes in the standing rules

| Rule today | After |
|---|---|
| "Keep patches small and mostly additive, so rebases onto a new upstream stay cheap" (CLAUDE.md) | Holds for the data and wire layer only. Elsewhere: one patch is one reviewable change with its vanilla method and its test, of whatever size the change needs |
| Raw game capability goes in the fork as normal Azalea APIs; `mc-adapter` only translates | Unchanged |
| Only `mc-adapter` and `null-node` import Azalea or Bevy | Unchanged |
| F5 §4.7 step 1: merge the version-bump commit and re-apply the patches | Merge only (§5) |
| Ledger column: bug fix or Null-only | Layer touched |

## 7. What to watch

- **The size of each upstream merge** (files and hunks in conflict, hours to settle). If it grows much faster than upstream's own change, the simulation layer is drifting, and rule 2 needs more tests.
- **Changes in the data and wire layer.** The count should stay near zero.
- **Upstream changing the base it stands on** (a new Bevy, a new Rust nightly). This cost comes with any option.

A first real merge of upstream `main` is worth doing soon, after the stack is pushed, while it is 43 files and not more. It also brings upstream's fixes of 7 months.

## 8. Decisions (Allen, 2026-11-10: yes to all four)

1. **The three layers and their rules** (§4). Recommended: yes.
2. **Merge, never rebase** (§5). Recommended: yes.
3. **End "small and mostly additive" for the simulation and client layers** (§6). Recommended: yes. Each patch keeps its vanilla method, its test and its ledger row.
4. **Merge upstream `main` once, after the stack is pushed and a pair passes** (§7). Recommended: yes, as a task with its own pair.
