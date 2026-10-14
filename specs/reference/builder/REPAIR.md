# repair as built

Documents `online/lib/repair.js`.

## Entry

Repair search. Design references: planner/ONLINE_BUILDER.md §1b, §4; IMPLEMENTATION.md G1.

Entry point: `repairFailure(loop, {budget?}) -> {ok: true, summary} | {ok: false, reason, report}`.

Called by the step loop (lib/loop.js) when no candidate step has a clean trial. The failure is a blocked step s (the top-ranked eligible steps are tried as s in turn). A damaging memory change is the same thing: the trial of the step that caused it was dirty. A failed end check is NOT repaired (see "Not done" below).

## Candidates and generators

A candidate is a sequence of core actions (design §2: `break`, `place`, `untemp`, ...), each planned and trialled by the loop's OWN attempt path (Loop.planFor / planBreak, then Loop.attempt: trial, F3 exit and escape checks, commit), from a checkpoint (the simulator snapshot plus the loop's mirror, Loop.saveMirror).

Any failed action, a candidate that ends without progress, or a candidate that does not restore what it undid rolls the WHOLE candidate back: simulator restore (state hash checked against the checkpoint's), mirror restored, the snapshots the candidate made dropped.

Only a candidate whose every action was trialled clean and which made progress is left committed.

So a repair is exactly as certified as a normal step: the committed records are ordinary step records, replayed by the certifier like any other.

There is no way to commit anything without a clean trial.

Generators, cheapest first (they run in order of plan length, the length being the number of core block-changing actions: iterative deepening on length; ties go to reorder, then wrap, then backtrack):

- reorder: `[p, s]` and `[p1, p2, s]`: pending steps p near the failure (or on its provenance chain, or a reference of s) are done first.
- wrap: `undo(O) s redo(O)`: break the committed cells O (newest first), do s, place O again (reverse order, the first one that is eligible). O comes from the suspects below. When a break is itself dirty, its cause is the next suspect: a new candidate undoes it first, outside of O (nesting outwards: undo(O2) undo(O1) s redo(O1) redo(O2)). When s is still dirty with O undone, the cause is added to O. A memory cell may be in O only if its redo restores its full state: the redo is the cell's own `place` (compound, judged against the full target state including tuning and tile NBT), and after the redo the cell must equal what it was before the wrap, else the candidate is rolled back.
- backtrack: suspect j (a committed place step): restore the simulator to before j (the undo log), unwind the mirror (the commit inverses in Loop.ledger), then replay: s, the committed steps after j that are not in j's block (in their old order), and last j's block. The block is j plus the steps that used a block cell as a reference. These are the steps that were pending at j and were committed since; steps made later by repairs (re-pended cells) are not replayed: they do not exist at j.

## Suspects

From update provenance (G2, in the trial result of the failed attempt): the closure of the failed cells under "was changed because of" (pos <- source), keeping committed place cells; newest commit first.

After them, the distance rule of IMPLEMENTATION.md G1: committed cells within Manhattan distance 3 of the failed cells (of s when the failure has no cells, e.g. no click), plus the redstone components connected to those through 6-neighbour chains of redstone blocks.

## Progress measure (termination)

A repair counts as success only if the pair (pending place and waterlog steps, pending untemp steps) is strictly smaller, lexicographically, than at the checkpoint. A pair of naturals under this order has no infinite descending chain.

Every other change the loop makes is also a strict decrease: a normal place commit lowers the first number (and may raise the second by its new untemps), an untemp commit lowers the second.

K1: a pillar is taken down right after the bot has used it, by its untemps, so those commits lower the second number again at once; the order and the argument are unchanged.

The pillar rule also restricts the candidates: while the bot is on a pillar a place step needs a click from the pillar top, and while the pillar comes down only untemps and breaks are allowed, so a candidate that cannot meet it simply fails.

A repair's own break steps raise the first number (their cells are pending again) and the redo steps lower it again, so only the net change counts, and only a net decrease is adopted.

So the loop can run only finitely many successful steps and repairs; and each repair call is finite on its own: a trial budget (default 200) bounds the trials, and the candidate set is finite (finitely many suspects, nesting depth and wrap size capped).

Hence the loop cannot cycle by undoing what an earlier repair did: such a repair would restore the earlier state, with no decrease, and be rolled back.

## Memo

Loop.attempt keeps the memo of failed actions, keyed by (action, state hash, bot position); a candidate that repeats a failed action from the same state costs no trial.

Budget: `budget` trials per call (200).

## Not done

Not done (reported, never faked):

- A failed end check is not repaired: with nothing pending the progress measure cannot decrease.
- `drain` (design §4's worked example) does not exist as a loop step; a candidate that needs one is not generated.
- No pillars for break clicks: a block the bot cannot break from places it can walk to is not undone.
