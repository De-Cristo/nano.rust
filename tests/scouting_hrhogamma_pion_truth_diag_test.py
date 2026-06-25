#!/usr/bin/env python3
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "diagnose_hrhogamma_pion_truth_proxy.py"
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "scouting_hrhogamma_candidates_truth_proxy_small.csv"

SPEC = importlib.util.spec_from_file_location("pion_truth_diag", SCRIPT)
pion_truth_diag = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["pion_truth_diag"] = pion_truth_diag
SPEC.loader.exec_module(pion_truth_diag)


class HToRhoGammaPionTruthDiagnosisTest(unittest.TestCase):
    def test_local_dir_discovery_returns_sorted_root_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "b.root").write_text("")
            (directory / "a.root").write_text("")
            (directory / "notes.txt").write_text("")

            inputs = pion_truth_diag.local_dir_inputs(directory, "*.root")

            self.assertEqual([item.path.name for item in inputs], ["a.root", "b.root"])
            self.assertEqual([item.label for item in inputs], ["file_000001", "file_000002"])

    def test_candidate_csv_grouping_by_event_key(self):
        rows, fieldnames = pion_truth_diag.read_candidate_rows(FIXTURE, max_candidates=None)

        grouped = pion_truth_diag.group_candidates_by_event(rows)

        self.assertIn(("1", "10", "1001"), grouped)
        self.assertEqual(len(grouped), 3)
        self.assertEqual(len(grouped[("1", "10", "1001")]), 1)
        self.assertIn("pi_plus_eta", fieldnames)

    def test_merge_summaries_combines_thresholds_and_categories(self):
        first = {
            "candidate_rows": 2,
            "diagnosed_candidates": 2,
            "category_counts": {
                "both_reco_pions_match_gen_pions_dr0p1": 1,
                "photon_matched_but_no_pion_match": 1,
            },
            "leg_threshold_counts": {
                "pi_plus_leg": {"pi_only_same_charge": {"0.10": 1}},
            },
            "both_threshold_counts": {
                "pi_only_same_charge": {"0.10": 1},
            },
            "pool_event_counts": {"all_pi_plus": 2},
            "pool_particle_counts": {"all_pi_plus": 3},
            "nearest_dr": {"nearest_pi_plus_dr": [0.03, 0.4]},
            "candidate_diagnostics": [{"event": 1001}],
            "files": [{"label": "file_000001"}],
        }
        second = {
            "candidate_rows": 1,
            "diagnosed_candidates": 1,
            "category_counts": {
                "photon_matched_but_no_pion_match": 1,
            },
            "leg_threshold_counts": {
                "pi_plus_leg": {"pi_only_same_charge": {"0.10": 1}},
            },
            "both_threshold_counts": {
                "pi_only_same_charge": {"0.10": 0},
            },
            "pool_event_counts": {"all_pi_plus": 1},
            "pool_particle_counts": {"all_pi_plus": 1},
            "nearest_dr": {"nearest_pi_plus_dr": [0.2]},
            "candidate_diagnostics": [{"event": 1002}],
            "files": [{"label": "file_000002"}],
        }

        merged = pion_truth_diag.merge_summaries([first, second])

        self.assertEqual(merged["candidate_rows"], 3)
        self.assertEqual(merged["diagnosed_candidates"], 3)
        self.assertEqual(merged["category_counts"]["photon_matched_but_no_pion_match"], 2)
        self.assertEqual(merged["leg_threshold_counts"]["pi_plus_leg"]["pi_only_same_charge"]["0.10"], 2)
        self.assertEqual(merged["both_threshold_counts"]["pi_only_same_charge"]["0.10"], 1)
        self.assertEqual(merged["pool_event_counts"]["all_pi_plus"], 3)
        self.assertEqual(merged["pool_particle_counts"]["all_pi_plus"], 4)
        self.assertEqual(merged["nearest_dr"]["nearest_pi_plus_dr"], [0.03, 0.4, 0.2])
        self.assertEqual(len(merged["files"]), 2)

    def test_report_writer_handles_missing_matplotlib(self):
        with tempfile.TemporaryDirectory() as tmp:
            outdir = Path(tmp)
            summary = {
                "mode": "run",
                "selected_files": 1,
                "candidate_rows": 3,
                "diagnosed_candidates": 3,
                "category_counts": {
                    "both_reco_pions_match_gen_pions_dr0p1": 1,
                    "photon_matched_but_no_pion_match": 2,
                },
                "leg_threshold_counts": {
                    "pi_plus_leg": {"pi_only_same_charge": {"0.10": 1}},
                    "pi_minus_leg": {"pi_only_same_charge": {"0.10": 1}},
                },
                "both_threshold_counts": {
                    "pi_only_same_charge": {"0.10": 1},
                    "any_charged_hadron_same_charge": {"0.10": 2},
                },
                "pool_event_counts": {"all_pi_plus": 2, "all_pi_minus": 2},
                "pool_particle_counts": {"all_pi_plus": 3, "all_pi_minus": 3},
                "nearest_dr": {"nearest_pi_plus_dr": [0.03, 0.4]},
                "packed_genpart_branches_present": False,
                "candidate_diagnostics": [],
                "recommendation": "Use photon truth only until better charged-particle truth is available.",
            }

            pion_truth_diag.write_report(
                outdir / "pion_truth_proxy_diagnosis.md",
                summary,
                plot_status="skipped (matplotlib unavailable)",
            )

            text = (outdir / "pion_truth_proxy_diagnosis.md").read_text()
            self.assertIn("# HToRhoGamma Charged-Pion Truth-Proxy Diagnosis", text)
            self.assertIn("diagnosed candidates: `3`", text)
            self.assertIn("photon_matched_but_no_pion_match", text)
            self.assertIn("plots: `skipped (matplotlib unavailable)`", text)
            self.assertIn("PackedGenPart branches present: `false`", text)


if __name__ == "__main__":
    unittest.main()
