#!/usr/bin/env python3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "run_scouting_hrhogamma_signal.py"
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_signal"
PER_FILE_DIR = FIXTURE_DIR / "per_file"
MANIFEST = FIXTURE_DIR / "manifest.json"


class ScoutingHToRhoGammaSignalTest(unittest.TestCase):
    def test_dry_run_from_manifest_reports_limited_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--manifest",
                    str(MANIFEST),
                    "--outdir",
                    str(outdir),
                    "--max-files",
                    "1",
                    "--dry-run",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            summary = outdir / "production_summary.txt"
            self.assertTrue(summary.exists())
            text = summary.read_text()
            self.assertIn("mode: dry-run", text)
            self.assertIn("dataset: /Test/HToRhoGamma/NANOAODSIM", text)
            self.assertIn("resolved_files: 2", text)
            self.assertIn("selected_files: 1", text)
            self.assertIn("would_run_files: 1", text)
            self.assertIn("/tmp/file_000001.root", text)

    def test_merge_command_keeps_one_header_and_writes_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--local-files",
                    "/tmp/file_000001.root",
                    "/tmp/file_000002.root",
                    "--outdir",
                    str(outdir),
                    "--plots-only",
                    "--no-root",
                    "--per-file-dir",
                    str(PER_FILE_DIR),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            combined = outdir / "combined_candidates.csv"
            self.assertTrue(combined.exists())
            lines = combined.read_text().splitlines()
            self.assertEqual(len(lines), 4)
            self.assertEqual(sum(1 for line in lines if line.startswith("run,")), 1)

            summary = outdir / "production_summary.txt"
            self.assertTrue(summary.exists())
            text = summary.read_text()
            self.assertIn("mode: plots-only", text)
            self.assertIn("total_processed_events: 0", text)
            self.assertIn("total_accepted_candidates: 3", text)
            self.assertIn("combined_csv_rows: 3", text)
            self.assertIn("combined_root: disabled (--no-root)", text)
            self.assertIn("file_000001 candidates=2", text)
            self.assertIn("file_000002 candidates=1", text)

            plot_summary = outdir / "plots" / "summary.txt"
            self.assertTrue(plot_summary.exists())
            plot_text = plot_summary.read_text()
            self.assertIn("candidate_rows: 3", plot_text)
            self.assertIn("h_mass_window_100_150: 3", plot_text)


if __name__ == "__main__":
    unittest.main()
