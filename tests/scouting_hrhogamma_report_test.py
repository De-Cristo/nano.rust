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
TRUTH_FIXTURE = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_truth_small.csv"
TRUTH_PROXY_FIXTURE = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_truth_proxy_small.csv"
HGAMMA_CLOSURE_FIXTURE = (
    REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_hgamma_closure_small.csv"
)
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

    def test_report_recognizes_truth_columns_and_writes_truth_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            plots = outdir / "plots"
            plots.mkdir(parents=True)
            for name in ("truth_matched_fraction.png", "reco_h_mass_minus_gen_h_mass.png"):
                (plots / name).write_bytes(b"fake png")
            combined_csv = outdir / "combined_candidates.csv"
            combined_csv.write_text(TRUTH_FIXTURE.read_text())

            result = subprocess.run(
                [
                    sys.executable,
                    str(REPORT_SCRIPT),
                    "--csv",
                    str(combined_csv),
                    "--outdir",
                    str(outdir),
                    "--plots-status",
                    "wrote 27 PNG files and 27 PDF files",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "physics_summary.md").read_text()
            self.assertIn("## Truth Validation", text)
            self.assertIn("truth available candidates: `2` / `2` (`1.000000`)", text)
            self.assertIn("truth matched candidates: `1` / `2` (`0.500000`)", text)
            self.assertIn("explicit_rho: `1`", text)
            self.assertIn("fallback_no_explicit_rho: `1`", text)
            self.assertIn("dR(photon), dR(pi+), dR(pi-) < 0.1", text)
            self.assertIn("| delta_r_reco_photon_gen_photon | 2 | 0.010000 | 0.325000 |", text)
            self.assertIn("- [truth_matched_fraction](plots/truth_matched_fraction.png)", text)

    def test_report_recognizes_truth_proxy_columns_and_writes_efficiency_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            plots = outdir / "plots"
            plots.mkdir(parents=True)
            for name in (
                "truth_proxy_match_thresholds.png",
                "reco_h_mass_minus_gen_h_proxy_mass.png",
            ):
                (plots / name).write_bytes(b"fake png")
            combined_csv = outdir / "combined_candidates.csv"
            combined_csv.write_text(TRUTH_PROXY_FIXTURE.read_text())

            result = subprocess.run(
                [
                    sys.executable,
                    str(REPORT_SCRIPT),
                    "--csv",
                    str(combined_csv),
                    "--outdir",
                    str(outdir),
                    "--processed-events",
                    "100",
                    "--accepted-candidates",
                    "3",
                    "--plots-status",
                    "wrote 40 PNG files and 40 PDF files",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "physics_summary.md").read_text()
            truth_proxy = outdir / "truth_proxy_summary.md"
            self.assertTrue(truth_proxy.exists())
            proxy_text = truth_proxy.read_text()
            self.assertIn("## Truth-Proxy Efficiency And Resolution", text)
            self.assertIn("[truth_proxy_summary.md](truth_proxy_summary.md)", text)
            self.assertIn("truth-proxy matched dR<0.1: `1` / `3` (`0.333333`)", proxy_text)
            self.assertIn("truth-proxy matched dR<0.2: `2` / `3` (`0.666667`)", proxy_text)
            self.assertIn("truth-proxy matched dR<0.3: `2` / `3` (`0.666667`)", proxy_text)
            self.assertIn("unique accepted-event fraction: `0.030000`", proxy_text)
            self.assertIn("| reco_h_mass_minus_gen_h_proxy_mass | 2 | 0.400000 | 1.200000 |", proxy_text)
            self.assertIn("- [truth_proxy_match_thresholds](plots/truth_proxy_match_thresholds.png)", proxy_text)

    def test_report_recognizes_hgamma_closure_columns_and_writes_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "signal"
            plots = outdir / "plots"
            plots.mkdir(parents=True)
            for name in (
                "hgamma_photon_match_dr.png",
                "hgamma_reco_h_mass_minus_gen_h_mass.png",
            ):
                (plots / name).write_bytes(b"fake png")
            combined_csv = outdir / "combined_candidates.csv"
            combined_csv.write_text(HGAMMA_CLOSURE_FIXTURE.read_text())

            result = subprocess.run(
                [
                    sys.executable,
                    str(REPORT_SCRIPT),
                    "--csv",
                    str(combined_csv),
                    "--outdir",
                    str(outdir),
                    "--processed-events",
                    "100",
                    "--accepted-candidates",
                    "3",
                    "--plots-status",
                    "wrote 50 PNG files and 50 PDF files",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "physics_summary.md").read_text()
            hgamma = outdir / "hgamma_closure_summary.md"
            self.assertTrue(hgamma.exists())
            hgamma_text = hgamma.read_text()
            self.assertIn("## Photon-Anchored Higgs Closure", text)
            self.assertIn("[hgamma_closure_summary.md](hgamma_closure_summary.md)", text)
            self.assertIn("hgamma closure available candidates: `2` / `3` (`0.666667`)", hgamma_text)
            self.assertIn("hgamma closure matched candidates: `1` / `3` (`0.333333`)", hgamma_text)
            self.assertIn("photon matched dR<0.1: `2` / `3` (`0.666667`)", hgamma_text)
            self.assertIn("Higgs closed |m(reco H)-m(gen H)|<15 GeV: `1` / `3` (`0.333333`)", hgamma_text)
            self.assertIn("| reco_h_mass_minus_gen_h_mass | 2 | -1.000000 | 19.500000 |", hgamma_text)
            self.assertIn("- [hgamma_photon_match_dr](plots/hgamma_photon_match_dr.png)", hgamma_text)

    def test_report_handles_absent_truth_columns_gracefully(self):
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
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "physics_summary.md").read_text()
            self.assertIn("## Truth Validation", text)
            self.assertIn("Truth validation was not requested or truth columns are absent.", text)

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
