"""Unit tests for vd_diff.py with hand-made logs. Run: uv run python -m unittest harness/vanilla-diff/differ/test_vd_diff.py"""
import unittest

import vd_diff as d


def rec(tick, dir_, name, summary="", state="Game"):
    return {"tick": tick, "dir": dir_, "name": name, "summary": summary, "state": state}


def run(ticks):
    """ticks: {tick: [(dir, name, summary)]}; every tick also gets its ClientTickEnd."""
    out = []
    for t, pk in sorted(ticks.items()):
        out += [rec(t, dr, n, s) for dr, n, s in pk]
        out.append(rec(t, "S", "ClientTickEnd"))
    return d.by_tick(out)


MOVE = ("S", "MovePlayerPos", "(x: 1.0)")


class DifferTests(unittest.TestCase):
    def test_identical_runs_match_with_a_tick_offset(self):
        null = run({5: [MOVE], 6: [MOVE], 7: []})
        van = run({25: [MOVE], 26: [MOVE], 27: []})
        diffs, n, problems = d.compare(null, van, (5, 8), 20)
        self.assertEqual((diffs, n, problems), ([], 3, []))
        self.assertIn("MATCH", d.report(diffs, n, problems))

    def test_negative_control_a_changed_packet_is_reported(self):
        null = run({5: [MOVE], 6: [MOVE]})
        van = run({5: [MOVE], 6: [("S", "MovePlayerPos", "(x: 1.5)")]})
        diffs, n, problems = d.compare(null, van, (5, 7), 0)
        self.assertEqual(len(diffs), 1)
        self.assertEqual(diffs[0]["tick"], 6)
        self.assertEqual(diffs[0]["label"], "client")
        text = d.report(diffs, n, problems)
        self.assertIn("DIFF", text)
        self.assertIn("x: 1.5", text)

    def test_a_missing_packet_is_reported(self):
        diffs, _, _ = d.compare(run({1: [MOVE]}), run({1: []}), (1, 2), 0)
        self.assertEqual(len(diffs), 1)

    def test_timer_packets_are_ignored(self):
        null = run({1: [MOVE]})
        van = run({1: [("S", "KeepAlive", ""), MOVE, ("S", "ChunkBatchReceived", ""), ("S", "Pong", ""), ("S", "ClientInformation", "")]})
        self.assertEqual(d.compare(null, van, (1, 2), 0)[0], [])

    def test_a_different_server_arrival_labels_the_difference_server_timing(self):
        null = run({1: [("C", "BlockUpdate", "")], 2: [MOVE]})
        van = run({1: [], 2: [("C", "BlockUpdate", ""), ("S", "MovePlayerPos", "(x: 2.0)")]})
        diffs, _, _ = d.compare(null, van, (1, 3), 0)
        self.assertEqual([x["label"] for x in diffs], ["server timing"])
        self.assertEqual(diffs[0]["first_server_diff"], 1)

    def test_timer_arrivals_do_not_make_server_timing(self):
        null = run({1: [("C", "KeepAlive", "")], 2: [MOVE]})
        van = run({1: [], 2: [("S", "MovePlayerPos", "(x: 2.0)")]})
        diffs, _, _ = d.compare(null, van, (1, 3), 0)
        self.assertEqual(diffs[0]["label"], "client")

    def test_logs_out_of_step_warn(self):
        null = run({1: [MOVE]})
        van = d.by_tick([rec(1, "S", "MovePlayerPos", "(x: 1.0)")])  # no tick end
        _, _, problems = d.compare(null, van, (1, 2), 0)
        self.assertEqual(problems, [(1, 1, 0)])

    def test_other_states_are_not_compared(self):
        recs = [rec(1, "S", "Hello", state="Login"), rec(1, "S", "ClientTickEnd")]
        self.assertEqual(d.by_tick(recs)[1][0], [])


if __name__ == "__main__":
    unittest.main()
