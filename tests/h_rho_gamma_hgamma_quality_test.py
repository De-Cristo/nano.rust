#!/usr/bin/env python3
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "analyze_h_rho_gamma_hgamma_closure_quality.py"
FIXTURE = (
    REPO_ROOT
    / "tests"
    / "fixtures"
    / "h_rho_gamma_candidates_hgamma_quality_small.csv"
)

SPEC = importlib.util.spec_from_file_location("hgamma_quality", SCRIPT)
hgamma_quality = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["hgamma_quality"] = hgamma_quality
SPEC.loader.exec_module(hgamma_quality)


class HgammaClosureQualityTest(unittest.TestCase):
    def test_fixture_categories_are_counted(self):
        rows, fieldnames = hgamma_quality.read_rows(FIXTURE)
        hgamma_quality.require_hgamma_columns(fieldnames)
        categories = hgamma_quality.categorize_rows(rows)

        self.assertEqual(len(rows), 5)
        self.assertEqual(len(categories["all_candidates"]), 5)
        self.assertEqual(len(categories["photon_matched_dr0p1"]), 4)
        self.assertEqual(len(categories["hgamma_closed_mass15"]), 2)
        self.assertEqual(len(categories["photon_matched_but_not_closed_mass15"]), 2)
        self.assertEqual(len(categories["not_photon_matched_dr0p1"]), 1)

    def test_mass_closed_without_photon_match_is_not_counted_as_hgamma_closed(self):
        rows, _ = hgamma_quality.read_rows(FIXTURE)
        not_photon = dict(rows[2])
        not_photon["hgamma_higgs_closed_mass_15"] = "1"
        not_photon["hgamma_closure_matched"] = "0"
        rows = [*rows, not_photon]

        categories = hgamma_quality.categorize_rows(rows)

        self.assertEqual(len(categories["hgamma_closed_mass15"]), 2)
        self.assertEqual(len(categories["not_photon_matched_dr0p1"]), 2)

    def test_summary_statistics_handle_missing_optional_fields(self):
        rows, _ = hgamma_quality.read_rows(FIXTURE)
        categories = hgamma_quality.categorize_rows(rows)
        summaries = hgamma_quality.variable_summaries(categories)

        h_mass = summaries["hgamma_closed_mass15"]["h_mass"]
        self.assertEqual(h_mass["count"], 2)
        self.assertAlmostEqual(h_mass["mean"], 125.0)
        recoil = summaries["not_photon_matched_dr0p1"][
            "reco_rho_mass_minus_gen_rho_recoil_mass"
        ]
        self.assertEqual(recoil["count"], 1)

    def test_threshold_scan_reports_expected_counts(self):
        rows, _ = hgamma_quality.read_rows(FIXTURE)
        scans = hgamma_quality.threshold_scans(rows)

        rho_010 = next(
            row
            for row in scans["rho_mass_window"]
            if row["label"] == "|rho_mass-0.775|<0.10"
        )
        self.assertEqual(rho_010["n_total_photon_matched"], 4)
        self.assertEqual(rho_010["n_pass"], 3)
        self.assertEqual(rho_010["n_closed_pass"], 2)
        self.assertAlmostEqual(rho_010["closure_fraction_after_cut"], 2 / 3)
        self.assertAlmostEqual(rho_010["closed_efficiency_relative_to_all_closed"], 1.0)

    def test_candidate_multiplicity_and_best_policy_are_deterministic(self):
        rows, _ = hgamma_quality.read_rows(FIXTURE)
        multiplicity = hgamma_quality.candidate_multiplicity(rows)
        policies = hgamma_quality.best_candidate_policies(rows)

        self.assertEqual(multiplicity["unique_events"], 4)
        self.assertEqual(multiplicity["multiplicity_counts"], {"1": 3, "2": 1})
        self.assertEqual(
            policies["best_by_abs_h_mass_minus_125"]["selected_events"],
            4,
        )
        self.assertEqual(policies["best_by_abs_h_mass_minus_125"]["closed_count"], 2)
        self.assertAlmostEqual(
            policies["best_by_abs_h_mass_minus_125"]["closed_fraction"],
            0.5,
        )

    def test_cli_writes_json_and_markdown_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "quality"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--candidate-csv",
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
            summary_md = outdir / "hgamma_closure_quality_summary.md"
            summary_json = outdir / "hgamma_closure_quality_summary.json"
            self.assertTrue(summary_md.exists())
            self.assertTrue(summary_json.exists())
            text = summary_md.read_text()
            self.assertIn("# Hgamma Closure Candidate-Quality Study", text)
            self.assertIn("candidates read: `5`", text)
            self.assertIn("photon-matched candidates: `4`", text)
            self.assertIn("hgamma-closed candidates: `2` / `5` (`0.400000`)", text)
            self.assertIn("best_by_abs_h_mass_minus_125", text)
            self.assertIn("plots: `skipped (--no-plots)`", text)


if __name__ == "__main__":
    unittest.main()
