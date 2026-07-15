#!/usr/bin/env python3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "plot_h_rho_gamma_csv.py"
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "h_rho_gamma_candidates_small.csv"
TRUTH_FIXTURE = REPO_ROOT / "tests" / "fixtures" / "h_rho_gamma_candidates_truth_small.csv"
TRUTH_PROXY_FIXTURE = (
    REPO_ROOT / "tests" / "fixtures" / "h_rho_gamma_candidates_truth_proxy_small.csv"
)
HGAMMA_CLOSURE_FIXTURE = (
    REPO_ROOT / "tests" / "fixtures" / "h_rho_gamma_candidates_hgamma_closure_small.csv"
)


class HToRhoGammaCsvTest(unittest.TestCase):
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

    def test_truth_columns_are_summarized_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "plots"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(TRUTH_FIXTURE),
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
            text = (outdir / "summary.txt").read_text()
            self.assertIn("truth_columns: present", text)
            self.assertIn("truth_available_count: 2", text)
            self.assertIn("truth_matched_count: 1", text)
            self.assertIn("truth_plot_expected: truth_matched_fraction.png", text)
            self.assertIn("truth_plot_expected: reco_h_mass_vs_gen_h_mass.png", text)

    def test_truth_proxy_columns_are_summarized_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "plots"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(TRUTH_PROXY_FIXTURE),
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
            text = (outdir / "summary.txt").read_text()
            self.assertIn("truth_proxy_columns: present", text)
            self.assertIn("truth_proxy_available_count: 3", text)
            self.assertIn("truth_proxy_matched_dr_0p1_count: 1", text)
            self.assertIn("truth_proxy_matched_dr_0p2_count: 2", text)
            self.assertIn("truth_proxy_matched_dr_0p3_count: 2", text)
            self.assertIn("truth_proxy_plot_expected: truth_proxy_match_thresholds.png", text)
            self.assertIn("truth_proxy_plot_expected: reco_h_mass_vs_gen_h_proxy_mass.png", text)

    def test_hgamma_closure_columns_are_summarized_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "plots"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(HGAMMA_CLOSURE_FIXTURE),
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
            text = (outdir / "summary.txt").read_text()
            self.assertIn("hgamma_closure_columns: present", text)
            self.assertIn("hgamma_closure_available_count: 2", text)
            self.assertIn("hgamma_closure_matched_count: 1", text)
            self.assertIn("hgamma_photon_matched_dr_0p1_count: 2", text)
            self.assertIn("hgamma_higgs_closed_mass_15_count: 1", text)
            self.assertIn("hgamma_plot_expected: hgamma_photon_match_dr.png", text)
            self.assertIn("hgamma_plot_expected: hgamma_reco_h_mass_minus_gen_h_mass.png", text)

    def test_hgamma_closure_plots_are_generated_when_matplotlib_is_available(self):
        try:
            import matplotlib  # noqa: F401
        except ImportError:
            self.skipTest("matplotlib is not available")

        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "plots"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(HGAMMA_CLOSURE_FIXTURE),
                    "--outdir",
                    str(outdir),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((outdir / "hgamma_photon_match_dr.png").exists())
            self.assertTrue((outdir / "hgamma_reco_h_mass_minus_gen_h_mass.png").exists())
            self.assertTrue((outdir / "hgamma_reco_rho_mass_vs_gen_rho_recoil_mass.png").exists())

    def test_hep_style_configuration_helper_is_plain_matplotlib_safe(self):
        script = (REPO_ROOT / "scripts" / "plot_h_rho_gamma_csv.py").read_text()
        self.assertIn("def hep_plot_style", script)
        self.assertIn("histtype=\"step\"", script)
        self.assertIn(".pdf", script)


if __name__ == "__main__":
    unittest.main()
