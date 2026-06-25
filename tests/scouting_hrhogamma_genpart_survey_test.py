#!/usr/bin/env python3
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "survey_hrhogamma_genpart_topology.py"

SPEC = importlib.util.spec_from_file_location("survey_genpart", SCRIPT)
survey_genpart = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["survey_genpart"] = survey_genpart
SPEC.loader.exec_module(survey_genpart)


class HToRhoGammaGenPartSurveyTest(unittest.TestCase):
    def test_local_dir_discovery_returns_sorted_root_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "b.root").write_text("")
            (directory / "a.root").write_text("")
            (directory / "notes.txt").write_text("")

            inputs = survey_genpart.local_dir_inputs(directory, "*.root")

            self.assertEqual([item.path.name for item in inputs], ["a.root", "b.root"])
            self.assertEqual([item.label for item in inputs], ["file_000001", "file_000002"])

    def test_select_inputs_applies_max_files(self):
        inputs = [
            survey_genpart.InputFile("file_000001", Path("/tmp/a.root")),
            survey_genpart.InputFile("file_000002", Path("/tmp/b.root")),
            survey_genpart.InputFile("file_000003", Path("/tmp/c.root")),
        ]

        selected = survey_genpart.select_inputs(inputs, max_files=2, all_files=False)

        self.assertEqual([item.label for item in selected], ["file_000001", "file_000002"])

    def test_command_includes_max_events_and_output_paths(self):
        command = survey_genpart.survey_command(
            Path("/repo/target/debug/examples/scouting_h_rho_gamma_genpart_survey"),
            Path("/tmp/input.root"),
            Path("/tmp/out/file_000001.summary.json"),
            Path("/tmp/out/file_000001.examples.txt"),
            max_events=1000,
            examples=3,
        )

        self.assertEqual(command[0], "/repo/target/debug/examples/scouting_h_rho_gamma_genpart_survey")
        self.assertIn("/tmp/input.root", command)
        self.assertIn("1000", command)
        self.assertIn("--out-json", command)
        self.assertIn("/tmp/out/file_000001.summary.json", command)
        self.assertIn("--out-examples", command)
        self.assertIn("/tmp/out/file_000001.examples.txt", command)
        self.assertIn("--examples", command)
        self.assertIn("3", command)

    def test_merge_summaries_combines_counts(self):
        first = {
            "processed_events": 2,
            "events_with_genpart": 2,
            "events_with_higgs": 1,
            "events_with_rho0": 0,
            "signed_pdg_counts": {"25": 1, "22": 2},
            "mother_daughter_counts": {"25->22": 1},
            "status_counts": {"25:22": 1},
            "files": [{"label": "file_000001"}],
        }
        second = {
            "processed_events": 3,
            "events_with_genpart": 3,
            "events_with_higgs": 0,
            "events_with_rho0": 1,
            "signed_pdg_counts": {"113": 1, "22": 4},
            "mother_daughter_counts": {"113->211": 2},
            "status_counts": {"113:22": 1},
            "files": [{"label": "file_000002"}],
        }

        merged = survey_genpart.merge_summaries([first, second])

        self.assertEqual(merged["processed_events"], 5)
        self.assertEqual(merged["events_with_genpart"], 5)
        self.assertEqual(merged["events_with_higgs"], 1)
        self.assertEqual(merged["events_with_rho0"], 1)
        self.assertEqual(merged["signed_pdg_counts"]["22"], 6)
        self.assertEqual(merged["mother_daughter_counts"]["25->22"], 1)
        self.assertEqual(merged["mother_daughter_counts"]["113->211"], 2)
        self.assertEqual(len(merged["files"]), 2)

    def test_dry_run_writes_summary_without_running_rust(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp) / "inputs"
            outdir = Path(tmp) / "survey"
            directory.mkdir()
            (directory / "a.root").write_text("")

            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--local-dir",
                    str(directory),
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
            summary = outdir / "genpart_topology_summary.json"
            self.assertTrue(summary.exists())
            data = json.loads(summary.read_text())
            self.assertEqual(data["mode"], "dry-run")
            self.assertEqual(data["selected_files"], 1)


if __name__ == "__main__":
    unittest.main()
