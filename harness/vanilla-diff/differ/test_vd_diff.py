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


def recs(ticks):
    """As `run`, but the raw records (for the anchored compare and the phase)."""
    out = []
    for t, pk in sorted(ticks.items()):
        out += [rec(t, dr, n, s) for dr, n, s in pk]
        out.append(rec(t, "S", "ClientTickEnd"))
    return out


EFFECT = ("C", "UpdateMobEffect", "")
FALL = lambda y: ("S", "MovePlayerPos", f"(y: {y})")


class AnchorTests(unittest.TestCase):
    # The vanilla run sees the effect one tick later and everything after it is one tick later too.
    null = recs({0: [MOVE], 1: [MOVE], 2: [EFFECT], 3: [FALL(1)], 4: [FALL(2)], 5: [FALL(3)]})
    van = recs({0: [MOVE], 1: [MOVE], 2: [], 3: [EFFECT], 4: [FALL(1)], 5: [FALL(2)], 6: [FALL(3)]})

    def test_without_anchors_a_late_arrival_is_a_difference(self):
        diffs, _, _ = d.compare(d.by_tick(self.null), d.by_tick(self.van), (0, 6), 0)
        self.assertTrue(diffs)

    def test_with_an_anchor_the_shifted_stretch_matches(self):
        diffs, n, problems, notes = d.compare_anchored(self.null, self.van, (0, 6), 0, ["UpdateMobEffect"])
        self.assertEqual((diffs, problems), ([], []))
        self.assertEqual(n, 6)  # 2 ticks before the anchor, 4 from it
        self.assertTrue(any("shift +1" in x for x in notes))

    def test_negative_control_a_changed_packet_after_the_anchor_is_still_reported(self):
        van = recs({0: [MOVE], 1: [MOVE], 2: [], 3: [EFFECT], 4: [FALL(1)], 5: [FALL(2.5)], 6: [FALL(3)]})
        diffs, _, _, _ = d.compare_anchored(self.null, van, (0, 6), 0, ["UpdateMobEffect"])
        self.assertEqual([x["tick"] for x in diffs], [4])

    def test_a_missing_vanilla_anchor_is_said(self):
        van = recs({0: [MOVE], 1: [MOVE], 2: [], 3: [], 4: [FALL(1)], 5: [FALL(2)], 6: [FALL(3)]})
        _, _, _, notes = d.compare_anchored(self.null, van, (0, 6), 0, ["UpdateMobEffect"])
        self.assertTrue(any("no vanilla match" in x for x in notes))


def pos(x, y, z, name="MovePlayerPos"):
    return ("S", name, f"(Serverbound{name} {{ pos: Vec3 {{ x: {x}, y: {y}, z: {z} }} }})")


class NormalizeTests(unittest.TestCase):
    def test_own_entity_id_and_teleport_id_are_not_differences(self):
        def one(eid, tp):
            return run({1: [("C", "PlayerPosition", f"(id: {tp}, change: 0)")],
                        2: [("S", "AcceptTeleportation", f"(ServerboundAcceptTeleportation {{ id: {tp} }})"),
                            ("S", "PlayerCommand", f"(ServerboundPlayerCommand {{ id: MinecraftEntityId({eid}), action: StartSprinting }})")]})
        self.assertEqual(d.compare(one(13756, 14), one(14871, 4), (1, 3), 0)[0], [])

    def test_negative_control_a_teleport_id_that_does_not_echo_is_a_difference(self):
        good = run({1: [("C", "PlayerPosition", "(id: 4, change: 0)")], 2: [("S", "AcceptTeleportation", "(ServerboundAcceptTeleportation { id: 4 })")]})
        bad = run({1: [("C", "PlayerPosition", "(id: 4, change: 0)")], 2: [("S", "AcceptTeleportation", "(ServerboundAcceptTeleportation { id: 9 })")]})
        self.assertEqual(len(d.compare(good, bad, (1, 3), 0)[0]), 1)

    def test_a_repeated_position_is_a_reminder_and_the_teleport_answer_is_not(self):
        r = recs({1: [pos(1, 2, 3)], 2: [pos(1, 2, 3)], 3: [("S", "AcceptTeleportation", "(id: 1)"), pos(5, 5, 5, "MovePlayerPosRot"), pos(5, 5, 5)]})
        names = [[n for n, _ in d.by_tick(r)[t][0]] for t in (1, 2, 3)]
        self.assertEqual(names, [["MovePlayerPos"], ["MovePlayerPos[reminder]"], ["AcceptTeleportation", "MovePlayerPosRot", "MovePlayerPos"]])


class LateArrivalTests(unittest.TestCase):
    def test_an_anchor_just_before_the_tick_end_counts_for_the_next_tick(self):
        def timed(t_arrive):
            return [{"state": "Game", "dir": "C", "name": "UpdateMobEffect", "tick": 4, "t_ms": t_arrive, "summary": ""},
                    {"state": "Game", "dir": "S", "name": "ClientTickEnd", "tick": 4, "t_ms": 1000, "summary": ""}]
        self.assertEqual(d.anchor_events(timed(999), {"UpdateMobEffect"}, 0, 10)[0][0], 5)
        self.assertEqual(d.anchor_events(timed(950), {"UpdateMobEffect"}, 0, 10)[0][0], 4)

    def test_reminders_are_left_out_of_a_shifted_stretch_only(self):
        null = recs({0: [pos(1, 1, 1)], 1: [EFFECT], 2: [pos(1, 1, 1)], 3: []})
        van = recs({0: [pos(1, 1, 1)], 1: [], 2: [EFFECT], 3: [pos(1, 1, 1)], 4: []})
        # not shifted: a reminder one tick off is a difference
        self.assertTrue(d.compare(d.by_tick(null), d.by_tick(van), (0, 4), 0)[0])
        diffs, _, _, notes = d.compare_anchored(null, van, (0, 4), 0, ["UpdateMobEffect"])
        self.assertEqual(diffs, [])
        self.assertTrue(any("reminder" in x for x in notes))


class PhaseTests(unittest.TestCase):
    def test_phase_counts_ticks_since_the_last_position_packet(self):
        r = recs({3: [MOVE], 4: [], 5: [], 6: []})
        self.assertEqual(d.phase_at(r, 7), 3)  # sent in tick 3; ticks 4, 5, 6 passed; counter 3 at the head of tick 7
        self.assertEqual(d.phase_at(r, 4), 0)

    def test_the_teleport_answer_does_not_reset_the_counter(self):
        r = recs({3: [MOVE], 5: [("S", "AcceptTeleportation", ""), ("S", "MovePlayerPosRot", "")]})
        self.assertEqual(d.phase_at(r, 8), 4)  # the reset was in tick 3

    def test_the_position_packet_after_the_answer_in_the_same_tick_resets_it(self):
        r = recs({3: [MOVE], 5: [("S", "AcceptTeleportation", ""), ("S", "MovePlayerPosRot", ""), MOVE]})
        self.assertEqual(d.phase_at(r, 8), 2)

class SeqTests(unittest.TestCase):
    # Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
    def act(self, seq):
        return ("S", "PlayerAction", f"(action: StartDestroyBlock, seq: {seq})")

    def test_a_connection_that_ran_other_rows_has_the_same_sequence_numbers_from_the_window(self):
        null = run({5: [self.act(67)], 6: [self.act(68)], 7: [("S", "PlayerAction", "(action: AbortDestroyBlock, seq: 0)")]})
        van = run({5: [self.act(1)], 6: [self.act(2)], 7: [("S", "PlayerAction", "(action: AbortDestroyBlock, seq: 0)")]})
        diffs, n, problems = d.compare(null, van, (5, 8), 0)
        self.assertEqual(diffs, [])

    def test_negative_control_a_missing_predicted_action_is_still_a_difference(self):
        null = run({5: [self.act(67)], 6: [self.act(69)]})
        van = run({5: [self.act(1)], 6: [self.act(2)]})
        diffs, n, problems = d.compare(null, van, (5, 7), 0)
        self.assertEqual([x["tick"] for x in diffs], [6])

class InputLogTests(unittest.TestCase):
    # Spec: specs/plans/VANILLA_DIFF_PLAN.md VD2
    def test_events_go_with_the_sample_that_follows_and_do_not_replace_it(self):
        import json
        import os
        import tempfile

        base = {"f": False, "b": False, "l": False, "r": False, "jump": False, "sneak": False, "sprint": False, "yaw": 0.0, "pitch": 0.0, "slot": 0}
        lines = [
            {**base, "tick": 0, "attack": False},
            {"ev": "swap_offhand", "tick": 1},
            {"mark": "start:x", "tick": 1},
            {"ev": "click", "win": 0, "slot": 36, "button": 0, "type": 0, "tick": 1},
            {**base, "tick": 1, "attack": True},
        ]
        with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False) as f:
            f.write("\n".join(json.dumps(x) for x in lines) + "\n")
        try:
            samples, marks = d.load_input(f.name)
        finally:
            os.unlink(f.name)
        self.assertEqual(marks, {"start:x": 1})
        self.assertEqual(samples[0]["events"], [])
        self.assertEqual([e["ev"] for e in samples[1]["events"]], ["swap_offhand", "click"])
        text = d.describe(samples[1])
        self.assertIn("ATTACK", text)
        self.assertIn("swap_offhand", text)
        self.assertIn("slot=36", text)
        # Negative control: describe says nothing of an action that was not logged.
        self.assertNotIn("ATTACK", d.describe(samples[0]))



if __name__ == "__main__":
    unittest.main()
