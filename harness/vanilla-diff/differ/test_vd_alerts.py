"""Unit tests for vd_alerts.py and the packet verdict. Run: cd harness/vanilla-diff/differ && uv run python -m unittest test_vd_alerts
Spec: specs/plans/VD3_BRIEF.md T1."""
import os
import tempfile
import unittest

import vd_alerts as a
import vd_diff as d

LOG = (
    "[10:00:01 INFO]: \x1b[38;5;1mGrim \x1b[0m» VD1 failed Simulation (x1) \n"
    "[10:00:02 INFO]: Grim » VD1 failed Simulation (x3)\n"
    "[10:00:03 INFO]: Grim » VD12 failed Reach (x1)\n"
    "[10:00:04 INFO]: Grim » null_check failed Timer (x1)\n"
    "[10:00:05 INFO]: VD1 moved wrongly!\n"
)


class ParseTests(unittest.TestCase):
    def test_colour_codes_are_stripped_and_lines_counted_for_the_player(self):
        self.assertEqual(a.parse_alerts(LOG, "VD1"), {"Simulation": 2})
        self.assertEqual(a.parse_alerts(LOG, "null_check"), {"Timer": 1})

    def test_negative_control_a_longer_name_is_another_player(self):
        # Without the whole-word rule VD1 would also take VD12's Reach.
        self.assertNotIn("Reach", a.parse_alerts(LOG, "VD1"))
        self.assertEqual(a.parse_alerts(LOG, "VD12"), {"Reach": 1})

    def test_window_reads_only_its_bytes(self):
        with tempfile.TemporaryDirectory() as t:
            p = os.path.join(t, "server.log")
            lines = LOG.splitlines(True)
            open(p, "w").write(LOG)
            start = len(lines[0].encode())
            end = start + len(lines[1].encode())
            self.assertEqual(a.parse_alerts(a.read_window(p, start, end), "VD1"), {"Simulation": 1})
            with self.assertRaises(ValueError):
                a.read_window(p, 0, 10**6)

    def test_null_window_from_marks(self):
        marks = {"start:A1:x": 5, "logoff:start:A1:x:100": 5, "logoff:end:A1:x:250": 9, "logoff:start:A1:y:7": 1}
        self.assertEqual(a.null_window(marks, "A1:x"), (100, 250))
        self.assertIsNone(a.null_window(marks, "A1:z"))
        self.assertIsNone(a.null_window({"logoff:start:A1:x:100": 5}, "A1:x"))


class CompareTests(unittest.TestCase):
    def test_an_alert_only_on_null_is_a_finding(self):
        c = a.compare_alerts({"Simulation": 2}, {})
        self.assertTrue(c["finding"])
        self.assertEqual(c["checks"]["Simulation"]["class"], "null_only")
        self.assertIn("FINDING", a.report(c, {"Simulation": 2}, {}))

    def test_negative_control_the_same_alert_on_both_is_shared_not_a_finding(self):
        c = a.compare_alerts({"Simulation": 2}, {"Simulation": 1})
        self.assertFalse(c["finding"])
        self.assertEqual(c["checks"]["Simulation"]["class"], "shared")

    def test_an_alert_only_on_vanilla_is_reported_not_a_finding(self):
        c = a.compare_alerts({}, {"Reach": 1})
        self.assertFalse(c["finding"])
        self.assertEqual(c["checks"]["Reach"]["class"], "vanilla_only")
        self.assertIn("vanilla only", a.report(c, {}, {"Reach": 1}))

    def test_vanilla_only_alert_lines_are_counted_apart(self):
        # Spec: specs/plans/VD3_BRIEF.md T2
        c = a.compare_alerts({"Timer": 1}, {"Timer": 4, "Reach": 2, "Aim": 1})
        self.assertEqual(c["vanilla_only_alerts"], 3)
        self.assertEqual(a.compare_alerts({}, {})["vanilla_only_alerts"], 0)

    def test_the_printed_summary_flags_vanilla_only_alerts(self):
        import sys, os
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
        import run_rows
        pk = {"verdict": "MATCH", "classes": {}}
        al = {"finding": False, "null": {}, "vanilla": {"RotationPlace": 1}, "vanilla_only_alerts": 1}
        self.assertIn("VANILLA-ONLY x1", run_rows.summary_line("R", {"packets": pk, "alerts": al}))
        al = {**al, "vanilla": {}, "vanilla_only_alerts": 0}
        self.assertNotIn("VANILLA-ONLY", run_rows.summary_line("R", {"packets": pk, "alerts": al}))

    def test_an_allowed_alert_is_labelled_and_not_a_finding(self):
        c = a.compare_alerts({"Timer": 1}, {}, {"Timer": "the forced stall"})
        self.assertFalse(c["finding"])
        self.assertIn("ALLOWED by the row (the forced stall)", a.report(c, {"Timer": 1}, {}))
        # the check not allowed still counts
        self.assertTrue(a.compare_alerts({"Timer": 1, "Reach": 1}, {}, {"Timer": "x"})["finding"])

    def test_no_alerts_on_either_side(self):
        c = a.compare_alerts({}, {})
        self.assertFalse(c["finding"])
        self.assertIn("none", a.report(c, {}, {}))


class RowAlertsTests(unittest.TestCase):
    """The whole row path (`run_rows.row_alerts`) on a log with a line injected on one side only."""

    def setUp(self):
        import json
        import pathlib
        import sys
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
        import run_rows
        self.run_rows, self.json = run_rows, json
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = pathlib.Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)

    def build(self, null_lines, vanilla_lines):
        """A server.log with a Null window, noise, and a vanilla window; returns (marks, log path)."""
        log = self.dir / "server.log"
        parts = ["before\n", "".join(null_lines), "between\n", "".join(vanilla_lines)]
        n0 = len(parts[0]); n1 = n0 + len(parts[1]); v0 = n1 + len(parts[2]); v1 = v0 + len(parts[3])
        log.write_text("".join(parts))
        (self.dir / "r_vtap.jsonl.logwin").write_text(self.json.dumps({"player": "VD9", "log": str(log), "start": v0, "end": v1}))
        return {"logoff:start:A1:x:%d" % n0: 3, "logoff:end:A1:x:%d" % n1: 9}, log

    def alerts(self, null_lines, vanilla_lines, allowed=None):
        marks, log = self.build(null_lines, vanilla_lines)
        return self.run_rows.row_alerts({"allowed_alerts": allowed or {}}, "A1:x", marks, "null_c", str(self.dir / "r"), log)

    def test_negative_control_an_alert_only_on_null_is_a_finding(self):
        r = self.alerts(["Grim \u00bb null_c failed Simulation (x1)\n"], [])
        self.assertTrue(r["finding"])
        self.assertEqual((r["null"], r["vanilla"]), ({"Simulation": 1}, {}))

    def test_an_alert_only_on_vanilla_is_not_a_finding_but_is_listed(self):
        r = self.alerts([], ["Grim \u00bb VD9 failed Reach (x1)\n"])
        self.assertFalse(r["finding"])
        self.assertEqual(r["checks"]["Reach"]["class"], "vanilla_only")

    def test_the_same_alert_on_both_is_shared(self):
        r = self.alerts(["Grim \u00bb null_c failed Reach (x1)\n"], ["Grim \u00bb VD9 failed Reach (x2)\n"])
        self.assertEqual((r["finding"], r["checks"]["Reach"]["class"]), (False, "shared"))

    def test_an_alert_outside_the_row_window_is_not_counted(self):
        marks, log = self.build([], [])
        with open(log, "a") as f:
            f.write("Grim \u00bb null_c failed Timer (x1)\n")
        r = self.run_rows.row_alerts({}, "A1:x", marks, "null_c", str(self.dir / "r"), log)
        self.assertEqual(r["null"], {})

    def test_a_missing_window_is_an_error_not_a_pass(self):
        r = self.run_rows.row_alerts({}, "A1:x", {}, "null_c", str(self.dir / "r"), self.dir / "server.log")
        self.assertIn("error", r)


class SummaryTests(unittest.TestCase):
    def test_summary_counts_classes_and_keeps_the_first(self):
        diffs = [{"tick": 6, "vanilla_tick": 6, "label": "client", "null": [("A", "x")], "vanilla": []},
                 {"tick": 7, "vanilla_tick": 7, "label": "server timing", "null": [], "vanilla": []}]
        s = d.summarize(diffs, 10, [])
        self.assertEqual((s["verdict"], s["ticks_differing"], s["classes"]), ("DIFF", 2, {"client": 1, "server timing": 1}))
        self.assertEqual(s["first"]["tick"], 6)

    def test_negative_control_no_diffs_is_a_match(self):
        self.assertEqual(d.summarize([], 10, [])["verdict"], "MATCH")
        self.assertEqual(d.summarize([], 10, [(3, 2, 1)])["verdict"], "DIFF")


if __name__ == "__main__":
    unittest.main()
