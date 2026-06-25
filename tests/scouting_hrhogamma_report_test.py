#!/usr/bin/env python3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
REPORT_SCRIPT = REPO_ROOT / "scripts" / "write_scouting_hrhogamma_report.py"
PRODUCTION_SCRIPT = REPO_ROOT / "scripts" / "run_scouting_hrhogamma_signal.py"
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_small.csv"
PER_FILE_DIR = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_signal" / "per_file"


class ScoutingHToRhoGammaReportTest(unittest.TestCase):
    def test_writes_physics_summary_with_statistics_windows_and_plot_links(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            plots = outdir / "plots"
            plots.mkdir(parents=True)
            for name in ("h_mass.png", "rho_mass.png"):
                (plots / name).write_bytes(b"fake png")
            combined_csv = outdir / "combined_candidates.csv"
            combined_csv.write_text(FIXTURE.read_text())

            result = subprocess.run(
                [
                    sys.executable,
                    str(REPORT_SCRIPT),
                    "--csv",
                    str(combined_csv),
                    "--outdir",
                    str(outdir),
                    "--config",
                    "configs/scouting/h_rho_gamma.toml",
                    "--dataset",
                    "/Test/HToRhoGamma/NANOAODSIM",
                    "--selected-files",
                    "2",
                    "--successful-files",
                    "2",
                    "--failed-files",
                    "0",
                    "--processed-events",
                    "200",
                    "--accepted-candidates",
                    "3",
                    "--combined-root",
                    str(outdir / "combined_candidates.root"),
                    "--command-line",
                    "python scripts/run_scouting_hrhogamma_signal.py --local-files a.root",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            report = outdir / "physics_summary.md"
            self.assertTrue(report.exists())
            text = report.read_text()
            self.assertIn("# HToRhoGamma Signal Physics Report", text)
            self.assertIn("signal-sample sanity report", text)
            self.assertIn("dataset: `/Test/HToRhoGamma/NANOAODSIM`", text)
            self.assertIn("candidate rows: `3`", text)
            self.assertIn("candidate rate per processed event: `0.015000`", text)
            self.assertIn("| h_mass | 3 | 118.000000 | 124.166667 | 124.500000 |", text)
            self.assertIn("| rho_mass | 3 | 0.700000 | 0.760000 | 0.760000 |", text)
            self.assertIn("100 < h_mass < 150: `3` / `3` (`1.000000`)", text)
            self.assertIn("115 < h_mass < 135: `3` / `3` (`1.000000`)", text)
            self.assertIn("rho_mass configured window 0.300000 < rho_mass < 1.200000: `3` / `3` (`1.000000`)", text)
            self.assertIn("- [h_mass](plots/h_mass.png)", text)
            self.assertIn("- [rho_mass](plots/rho_mass.png)", text)
            self.assertIn("## Expected plots not produced", text)
            self.assertIn("plots/photon_pt.png", text)

    def test_report_clearly_describes_missing_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            outdir.mkdir()
            combined_csv = outdir / "combined_candidates.csv"
            combined_csv.write_text(FIXTURE.read_text())

            result = subprocess.run(
                [
                    sys.executable,
                    str(REPORT_SCRIPT),
                    "--csv",
                    str(combined_csv),
                    "--outdir",
                    str(outdir),
                    "--plots-status",
                    "skipped (matplotlib unavailable)",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "physics_summary.md").read_text()
            self.assertIn("PNG plots were not produced: skipped (matplotlib unavailable)", text)
            self.assertIn("Expected plots not produced", text)
            self.assertIn("plots/h_mass.png", text)

    def test_production_report_only_reuses_fixture_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            result = subprocess.run(
                [
                    sys.executable,
                    str(PRODUCTION_SCRIPT),
                    "--local-files",
                    "/tmp/file_000001.root",
                    "/tmp/file_000002.root",
                    "--outdir",
                    str(outdir),
                    "--plots-only",
                    "--report-only",
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
            self.assertTrue((outdir / "combined_candidates.csv").exists())
            report = outdir / "physics_summary.md"
            self.assertTrue(report.exists())
            text = report.read_text()
            self.assertIn("candidate rows: `3`", text)
            self.assertIn("total processed events: `0`", text)
            self.assertIn("candidate rate per processed event: `unavailable`", text)


if __name__ == "__main__":
    unittest.main()
