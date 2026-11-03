# VD3: the vanilla diff in full

Written 2026-11-03, while the Mac pair for patch 54 ran. Plan: [VANILLA_DIFF_PLAN](VANILLA_DIFF_PLAN.md). Results go in `harness/vanilla-diff/README.md`, section "VD3".

## Where it stands

`harness/vanilla-diff/rows.json` holds 92 rows: A1 2, A2 6, A3 1, A4 6, A9 46, A13 4, A15 9, A17 9, A22 9. The suite has 449, so 357 rows in 19 modules have never been compared with a vanilla client: A4S 145, A2S 52, A10E 28, A10 26, A16 16, A11 14, A12 11, A20 10, A14 9, A21 7, A19 7, A6 6, A4C 6, A18 5, AT 4, A5 4, A8 3, A7 3, A23 1.

Each module needs a `prepare_row` (a thin dispatch, 8 to 21 lines in the modules that have one) and `--prepare-scene` wiring in `main.rs`. All replays so far ran on the vanilla server, without `--companion`. No Grim alert list has been compared yet.

## Row classes

Each new row gets one class in `rows.json`, set by reading the row, never by guess:

1. **Static:** only the bot acts in a scene RCON sets. Compare packet for packet.
2. **Server-random:** mobs, drops or other random server behaviour steer the bot's input or its packets. Compare up to the first server difference, and report the rest as server timing with the arrival that caused it.
3. **Second player:** the row needs another client. Reproduce it.
4. **Injected:** Null's harness does something a vanilla client has no input for (the AT stall rows). The replay mod reproduces it.
5. **Null refuses on purpose:** skip with the reason, as the A17 disconnect rows do.

**Skips (Allen, 2026-11-03).** Only class 5 skips: about 13 rows, A20 `refuse_*` and the A16 `scale_2` and `dolphins_grace` rows, as the A17 disconnect rows already do. Each names its reason in `rows.json` and is listed in the result. Class 2 is compared up to the first server difference and marked partial. Class 4 is reproduced: the replay mod sleeps for the same time. Class 3 is reproduced: a stand-in second player first, a second vanilla client only if a row needs it (`push_tick_order` probably does). Refused abilities stay refused; they are to-dos in [REFUSED_ABILITIES](REFUSED_ABILITIES.md).

## Tasks, in order

Each is one Sonnet task with one done-when. A task that exceeds about 800 lines is split.

- **T1, the runner and the Grim list.** `null-abilitycheck --vanilla-diff` or an extended `run_rows.py`, whichever is smaller; reuse the replay and the differ. Add `--server` runs on Paper + Grim through the tap, with the fresh vanilla player not op, and compare the alert lists of the two runs per row. An alert only Null draws is a finding; an alert both draw is recorded in F5 §4.6 with the vanilla run as its proof. Done when A1-A4 and A22 rows run from one command on both servers and print alert lists side by side, including one row with an alert injected by a negative control.
- **T2, shapes.** A4S, A2S, A4C (203 rows). Done when each row matches or each difference is a finding.
- **T3, containers and crafting.** A10, A10E, A11.
- **T4, combat and items.** A5, A6, A7, A8, A18, A20.
- **T5, movement and pathing.** A14, A16, A19, A21.
- **T6, death, stalls, resync.** A12, A23, AT. Likely the hardest: respawn timing, injected stalls.
- **T7, the old rows on Grim.** The 92 rows on Paper + Grim, and the stale A17 `slow_falling_arrow` replay (OPEN.md §4).

Findings are not fixed inside these tasks. They go to the lead, in the same way as VD2. Every Null departure gets fixed, also one that draws no alert (Allen, 2026-11-03: "Fix all"), batched by the pair cadence.

## Before T1 starts

- Patch 54 is pushed and pinned, or VD3 knows it runs on a held head.
- A heavy build or another Grim run must not overlap a Grim leg.

## Rules for every brief

- Selective runs by the module map, once per server. A run over about 4 minutes goes detached with `nohup`: write state and what is left to a file, then end; no waiting or polling. The lead waits. Don't touch the Pi.
- No tool call may block for more than 270 s.
- Every major function has a one-line `// Spec:` link; a missing or dead link fails verification.
- A difference is classified from the source in `~/Desktop/null-reference/`, naming the vanilla method; describe it in our own words and never copy code.
- Python through uv only, never pip or system packages. Never read or change network configuration on any device.
- Budget a cold build and world setup apart from code.
