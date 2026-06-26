#!/usr/bin/env python3
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
POLICY_SCRIPT = REPO_ROOT / "scripts" / "hrhogamma_plotting_policy.py"
QUALITY_SCRIPT = REPO_ROOT / "scripts" / "analyze_hrhogamma_hgamma_closure_quality.py"
FIXTURE = (
    REPO_ROOT
    / "tests"
    / "fixtures"
    / "scouting_hrhogamma_candidates_hgamma_quality_small.csv"
)
PLOT_CONFIG = REPO_ROOT / "configs" / "scouting" / "h_rho_gamma_plotting.toml"
QUALITY_CONFIG = REPO_ROOT / "configs" / "scouting" / "h_rho_gamma_quality_categories.toml"


SPEC = importlib.util.spec_from_file_location("hrhogamma_plotting_policy", POLICY_SCRIPT)
plotting_policy = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["hrhogamma_plotting_policy"] = plotting_policy
SPEC.loader.exec_module(plotting_policy)

QUALITY_SPEC = importlib.util.spec_from_file_location("hgamma_quality", QUALITY_SCRIPT)
hgamma_quality = importlib.util.module_from_spec(QUALITY_SPEC)
assert QUALITY_SPEC.loader is not None
sys.modules["hgamma_quality"] = hgamma_quality
QUALITY_SPEC.loader.exec_module(hgamma_quality)


class HgammaPlottingPolicyTest(unittest.TestCase):
    def test_plotting_config_loads_named_profiles(self):
        policy = plotting_policy.load_plotting_policy(PLOT_CONFIG, "physics_focus")

        self.assertEqual(policy.selected_profile, "physics_focus")
        self.assertIn("physics_focus", policy.profiles)
        self.assertIn("full_range_sanity", policy.profiles)
        self.assertEqual(policy.spec_for("h_mass").range, (80.0, 180.0))
        self.assertEqual(policy.spec_for("h_mass").bins, 50)

    def test_missing_profile_gives_clear_error(self):
        with self.assertRaisesRegex(ValueError, "unknown plot profile 'missing'"):
            plotting_policy.load_plotting_policy(PLOT_CONFIG, "missing")

    def test_variable_without_explicit_range_falls_back_safely(self):
        policy = plotting_policy.load_plotting_policy(PLOT_CONFIG, "signal_window")
        fallback = policy.spec_for("photon_pt")

        self.assertIsNone(fallback.range)
        self.assertEqual(fallback.bins, 60)
        self.assertFalse(fallback.overflow)

    def test_range_coverage_counts_underflow_inside_and_overflow(self):
        spec = plotting_policy.PlotSpec(range=(0.0, 10.0), bins=5, overflow=True)
        coverage = plotting_policy.range_coverage("x", "test", spec, [-1.0, 0.0, 2.5, 10.0, 12.0])

        self.assertEqual(coverage["n_total"], 5)
        self.assertEqual(coverage["n_underflow"], 1)
        self.assertEqual(coverage["n_inside"], 3)
        self.assertEqual(coverage["n_overflow"], 1)
        self.assertAlmostEqual(coverage["frac_inside"], 0.6)

    def test_full_range_sanity_output_can_be_requested(self):
        policy = plotting_policy.load_plotting_policy(
            PLOT_CONFIG,
            "physics_focus",
            write_full_range_sanity=True,
        )

        self.assertEqual(policy.output_profiles(), ["physics_focus", "full_range_sanity"])

    def test_quality_category_config_loads_and_evaluates(self):
        categories = plotting_policy.load_quality_categories(QUALITY_CONFIG)
        rows, _ = hgamma_quality.read_rows(FIXTURE)
        summary = hgamma_quality.summarize_quality_categories(rows, categories)

        self.assertIn("quality_medium", summary["categories"])
        self.assertEqual(summary["categories"]["inclusive"]["n_candidates"], 5)
        self.assertEqual(summary["categories"]["quality_loose"]["n_candidates"], 1)
        self.assertEqual(summary["categories"]["quality_loose"]["n_hgamma_closed"], 1)

    def test_anti_circular_category_warning_is_emitted(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bad_quality.toml"
            path.write_text(
                """
[quality_categories.mass_based]
label = "Mass based"
requirements = [
  { variable = "h_mass", op = ">", value = 100.0 },
]
"""
            )

            categories = plotting_policy.load_quality_categories(path)

            self.assertTrue(categories.warnings)
            self.assertIn("anti-circularity", categories.warnings[0])

    def test_cli_writes_policy_and_category_reports_without_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "quality"
            result = subprocess.run(
                [
                    sys.executable,
                    str(QUALITY_SCRIPT),
                    "--candidate-csv",
                    str(FIXTURE),
                    "--outdir",
                    str(outdir),
                    "--no-plots",
                    "--plot-config",
                    str(PLOT_CONFIG),
                    "--plot-profile",
                    "physics_focus",
                    "--write-full-range-sanity",
                    "--quality-config",
                    str(QUALITY_CONFIG),
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            coverage_md = outdir / "range_coverage_summary.md"
            coverage_json = outdir / "range_coverage_summary.json"
            category_md = outdir / "hgamma_quality_categories_summary.md"
            category_json = outdir / "hgamma_quality_categories_summary.json"
            self.assertTrue(coverage_md.exists())
            self.assertTrue(coverage_json.exists())
            self.assertTrue(category_md.exists())
            self.assertTrue(category_json.exists())
            self.assertIn("physics_focus", coverage_md.read_text())
            self.assertIn("quality_medium", category_md.read_text())
            self.assertIn("plots: `skipped (--no-plots)`", category_md.read_text())


if __name__ == "__main__":
    unittest.main()
