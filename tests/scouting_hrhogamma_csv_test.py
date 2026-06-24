#!/usr/bin/env python3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "plot_scouting_hrhogamma_csv.py"
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_small.csv"


class ScoutingHToRhoGammaCsvTest(unittest.TestCase):
    def test_writes_summary_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "plots"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(FIXTURE),
                    "--outdir",
                    str(outdir),
                    "--no-plots",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            summary = outdir / "summary.txt"
            self.assertTrue(summary.exists())
            text = summary.read_text()
            self.assertIn(f"input_csv: {FIXTURE}", text)
            self.assertIn("candidate_rows: 3", text)
            self.assertIn("unique_events: 2", text)
            self.assertIn("duplicate_event_entries: 1", text)
            self.assertIn("h_mass: min=118.000000 mean=124.166667 max=130.000000", text)
            self.assertIn("rho_pt_over_photon_pt: min=0.358300 mean=0.419000 max=0.471400", text)
            self.assertIn("plots: skipped (--no-plots)", text)


if __name__ == "__main__":
    unittest.main()
