# The test gate: recorded vanilla sessions as the everyday check

> **Status (2026-11-10): adopted by Allen as the direction, not built (§10).** The parts in §5 are a sketch: the exact code is decided when the spike is built (Allen, 2026-11-10). It fills in the "trace replay" part of `plans/LOAD_CONTRACT.md` "Testing tiers", which says that part is not yet shown to work. Nothing here is built. The counts in §2 come from the fork ledger (`reference/AZALEA_PATCHES.md`), the commit log and the pair summaries in `~/Desktop/null-spikes/run-logs/`. The one-at-a-time pair stays the gate until §8 is done and Allen decides.

## 1. The problem

A fork patch is pushed only after a full pair passes. A pair on the Pi takes about 66 minutes of run time (`p48_full`: 1,857 s and 1,883 s), plus the deploy and the server starts. That is not the real cost. The real costs are:

- **One at a time.** A pair needs the Pi alone. A Grim run on the Mac allows no heavy build beside it. VD3 legs, own-world runs and pairs queue for the same machines.
- **The map says `all` for most fork changes.** So the selective run is nearly a full run, on both servers.
- **Each unexplained alert costs a session.** OPEN §4 lists `Timer` and `Simulation` alerts from pairs that nobody has explained.
- **The cadence.** At most 3 patches between pairs. The stack now holds 17, and the client level model makes 18.

## 2. What each check has found

Patches 31 to 54 (24 patches), by how the problem was found:

| Found by | Patches | Count |
|---|---|---|
| Planning and design | 31, 32, 34, 36 | 4 |
| A verifier, or queued items | 35, 37, 39 | 3 |
| A full pair | 33, 38, 42, 43, 44 | 5 |
| A selective or milestone live run | 40, 41 | 2 |
| The vanilla diff (tools 45, 46; departures 47 to 51) | 45 to 51 | 7 |
| The shared-world test and the stall test | 52, 53, 54 | 3 |

Three of the five pair finds (42, 43, 44) are shared-world bugs. Own worlds close that class.

The five pairs since patch 44 (`p45_full`, `p48_full`, `p51_full`, `p53_full`, `mac54`) found no Null bug. They found five harness faults and the unexplained alerts. In the same weeks the vanilla diff found five departures, and none of them drew a Grim alert.

So the pair is a weak detector of Null bugs today, and the vanilla diff is a strong one. The pair still answers questions that nothing else answers (§4).

## 3. The idea: a recorded vanilla session is a test with no server

The vanilla diff already runs one row in a real vanilla client and records its packets by client tick. Three streams fully describe that session from the client's side:

1. the inputs per client tick (keys, look, clicks), from the input log;
2. every packet the server sent, with the client tick in which the client handled it;
3. every packet the client sent, per client tick.

A client tick is a function of state, queued packets and inputs (LOAD_CONTRACT, "The model"). So streams 1 and 2 determine stream 3. Give the fork streams 1 and 2 with no server, from the first login packet, and it must produce stream 3. This is a **fixture**.

**Why it is valid.** The server's replies in stream 2 answer the vanilla client's packets. If Null sends the same packets, the same replies are the right replies. If Null sends something else, the test fails at that tick, and only that first difference is reported, because the replies after it no longer fit.

**What it gives.**

- **No server-timing noise.** In a live diff, a server packet can reach the two clients in different ticks. In the Pi trial of 2026-11-10 that class was the main or only difference in about half of the 18 differing rows. A fixture delivers each packet in the recorded tick, so the class cannot occur.
- **The whole suite in minutes, on any machine, beside any build.** A pair's rows are about 37,000 game ticks. With no server and no waiting, that is seconds to minutes of CPU.
- **The server's answers are inside.** Refusals, acks, rollbacks and window contents are in stream 2. So the fork's judgement code runs against real server replies.

## 4. What each tier answers

| Tier | Runs | The question only it answers | When |
|---|---|---|---|
| 0 | Fork and Null unit and simulation tests, contract tests | Does the code hold its own rules, and the load contract? | Every change |
| 1 | All fixtures, offline | Does the client still decide what a vanilla client decides, on every recorded row? | Every change |
| 2 | Live rows, one module or one row, with the vanilla client beside them | What does a real server answer to a new or changed input? This is where fixtures are recorded | A new row, a changed row or harness, a new ability |
| 3 | The full pair, mimicked load, real load | State that lives across modules, load, what nobody thought to record | By cadence (§9) |

**What a fixture cannot answer, so a live tier keeps it.**

- What a server does with packets it has not seen. A fixture only proves "the same as vanilla on this input".
- Grim's verdict. It follows from equal packets in equal ticks, but timing under load is the load contract's job.
- Skills and the pathfinder. The inputs are fixed, so code above the client is not exercised. The ability rows' server readback keeps that job.
- State across rows. Each fixture starts at a login.

## 5. The parts to build

1. **Exact tick for each server packet.** Today the proxy infers the tick from arrival time, and the differ reports packets that arrive within a few ms of a tick's end as uncertain. The replay mod runs inside the client, so it can write the exact count of handled packets at the head of each tick. The tap's packet order plus these counts give stream 2 with no guess.
2. **Raw bytes in the tap.** `vd-tap --raw-out`, already agreed in LOAD_CONTRACT step 3 and not built.
3. **An input player in the fork.** The inverse of the input log (patch 45): it applies one recorded line per tick. Test code only.
4. **A fixture runner.** A fork simulation that starts from the recorded login and feeds streams 1 and 2. It reuses `RawConnection::injected_clientbound_packets`. It compares with the differ's rules (the same ignored packets).
5. **A fixture store.** Outside git if large; a manifest in the repo with the row, the server, the game version, the recording date and a checksum.

## 6. Rules for fixtures

- **The recording is the truth.** A fixture never changes because Null changed. When a patch fixes a departure, the fixture that failed starts to pass.
- **Known departures are a list.** A fixture that fails today is listed with its OPEN §4 line. The list may only shrink. A new failure outside the list fails the gate.
- **Record on both servers.** Paper and Grim send their own packets (Grim's pings), so each row has a vanilla-server fixture and a Paper + Grim fixture.
- **When to record again.** When the row's inputs change (a harness or skill change), record that row. When the game version changes, record everything. This is the main cost of a version port, and it is machine time, not session time.
- **Rows Null refuses** have no fixture. They keep their refusal rows.
- **A row with a second bot** records the other bot as the server packets that describe it. The fixture needs no second bot.

## 7. Risks, and the spike that answers them

| Risk | What would show it |
|---|---|
| The fork cannot start from a recorded login (registries, configuration phase, chunks) | The spike's first fixture does not reach the row's window |
| The exact tick is not recordable in the mod | The handled-packet count at a tick's head does not reproduce the vanilla session when replayed into vanilla itself |
| Null has a source of difference outside the inputs and packets (a wall clock on the allowed list, an id) | A fixture passes live and fails offline at a packet on the allowed list |
| Fixtures are too large | Size per row; chunks dominate. Store chunk packets once by content if needed |

**Spike.** Take three rows that already match in VD3 (one walk, one break, one window click). Build parts 1 to 4 just far enough. Done when all three pass offline twice with the same result, and a known bug put back (patch 47 off: the missing swing in the destroy delay) fails its row at the right tick.

## 8. Proof before it replaces anything

As in LOAD_CONTRACT, unchanged:

1. Put known past bugs back, one at a time (patches 10, 47 to 51, 53, 54). Tier 1 must fail on each. A miss is listed, and that kind stays live.
2. Run tier 1 beside the pair for several pushes. Every pair failure caused by Null must also fail tier 1, or be listed as live-only.
3. Allen decides what leaves the everyday path.

## 9. The gate after the proof (to decide then)

A proposal for that decision, not a change now:

| A patch that | Needs |
|---|---|
| Changes what the client decides | Tiers 0 and 1, all fixtures |
| Adds an ability or a row | The above, and tier 2 for the new rows on both servers, which also records their fixtures |
| Touches the loop, the connection or the tick clock | The above, the mimicked-load rows, and a pair |
| Changes only the harness | The module, live |

The pair moves from "every 3 patches" to "every push to `null-main`", with pushes at milestones or weekly. "The pushed head is a tested head" stays true.

## 10. Decisions (Allen, 2026-11-10)

1. The spike (§7) is built now. It needs no Grim run, only a short recording session on the vanilla server. The exact code comes later, with the spike.
2. VD3 T3 to T6 in full as a live diff before `mine` (Allen, 2026-11-03) stands until the spike passes. After that, the scenes are finished only as far as fixtures need them.
3. The VD3 side owns the fixture format, because the recording tools live there. The load contract work reuses it.
