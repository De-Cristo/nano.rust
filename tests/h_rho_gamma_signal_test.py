#!/usr/bin/env python3
import subprocess
import sys
import tempfile
import unittest
import importlib.util
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "run_h_rho_gamma_signal.py"
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "h_rho_gamma_signal"
PER_FILE_DIR = FIXTURE_DIR / "per_file"
MANIFEST = FIXTURE_DIR / "manifest.json"

SPEC = importlib.util.spec_from_file_location("run_signal", SCRIPT)
run_signal = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["run_signal"] = run_signal
SPEC.loader.exec_module(run_signal)


class HToRhoGammaSignalTest(unittest.TestCase):
    def test_remote_detection_recognizes_root_urls_and_store_lfns(self):
        self.assertTrue(run_signal.is_remote_input("root://cms-xrd-global.cern.ch//store/a.root"))
        self.assertTrue(run_signal.is_remote_input("/store/mc/a.root"))
        self.assertFalse(run_signal.is_remote_input("/tmp/a.root"))

    def test_store_lfn_converts_to_global_xrootd_url(self):
        self.assertEqual(
            run_signal.remote_source_for_download("/store/mc/a.root"),
            "root://cms-xrd-global.cern.ch//store/mc/a.root",
        )
        self.assertEqual(
            run_signal.remote_source_for_download("root://host//store/mc/a.root"),
            "root://host//store/mc/a.root",
        )

    def test_cache_path_is_deterministic_and_collision_safe(self):
        cache_dir = Path("/tmp/cache")
        first = run_signal.cache_path_for_input(
            cache_dir,
            1,
            "root://cms-xrd-global.cern.ch//store/mc/abc.root",
        )
        second = run_signal.cache_path_for_input(
            cache_dir,
            1,
            "root://cms-xrd-global.cern.ch//store/other/abc.root",
        )
        repeat = run_signal.cache_path_for_input(
            cache_dir,
            1,
            "root://cms-xrd-global.cern.ch//store/mc/abc.root",
        )

        self.assertEqual(first, repeat)
        self.assertNotEqual(first, second)
        self.assertEqual(first.parent, cache_dir)
        self.assertTrue(first.name.startswith("file_000001_"))
        self.assertTrue(first.name.endswith("_abc.root"))

    def test_local_files_path_remains_unchanged(self):
        self.assertEqual(
            run_signal.effective_input_plan(
                1,
                run_signal.InputFile("file_000001", "/tmp/file_000001.root"),
                Path("/tmp/cache"),
                download_remote=False,
            ).run_input,
            "/tmp/file_000001.root",
        )

    def test_local_dir_discovery_returns_sorted_root_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            (directory / "b.root").write_text("")
            (directory / "a.root").write_text("")
            (directory / "notes.txt").write_text("")

            dataset, inputs = run_signal.local_dir_inputs(directory, "*.root")

            self.assertEqual(dataset, "local-dir")
            self.assertEqual([Path(item.path).name for item in inputs], ["a.root", "b.root"])
            self.assertEqual([item.label for item in inputs], ["file_000001", "file_000002"])

    def test_local_dir_with_max_files_selects_sorted_prefix(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for name in ("c.root", "a.root", "b.root"):
                (directory / name).write_text("")

            _, inputs = run_signal.local_dir_inputs(directory, "*.root")
            selected = run_signal.select_inputs(inputs, 2)

            self.assertEqual([Path(item.path).name for item in selected], ["a.root", "b.root"])

    def test_remote_without_download_fails_clearly(self):
        with self.assertRaisesRegex(
            ValueError,
            "remote ROOT reading is not supported by the current Rust reader; "
            "rerun with --download-remote to cache files locally first",
        ):
            run_signal.effective_input_plan(
                1,
                run_signal.InputFile("file_000001", "/store/mc/file.root"),
                Path("/tmp/cache"),
                download_remote=False,
            )

    def test_remote_with_download_plans_cached_input(self):
        plan = run_signal.effective_input_plan(
            1,
            run_signal.InputFile("file_000001", "/store/mc/file.root"),
            Path("/tmp/cache"),
            download_remote=True,
        )

        self.assertEqual(plan.original_input, "/store/mc/file.root")
        self.assertEqual(plan.download_source, "root://cms-xrd-global.cern.ch//store/mc/file.root")
        self.assertEqual(plan.run_input, str(plan.cached_input))
        self.assertTrue(str(plan.cached_input).startswith("/tmp/cache/file_000001_"))

    def test_reco_command_forwards_hgamma_closure_truth_strategy(self):
        args = run_signal.argparse.Namespace(
            use_cargo_run=False,
            release=False,
            max_events_per_file=100,
            config=Path("configs/h_rho_gamma.toml"),
            no_csv=False,
            truth=True,
            truth_strategy="hgamma-closure",
        )
        binaries = run_signal.Binaries(
            Path("target/debug/examples/h_rho_gamma"),
            Path("target/debug/examples/h_rho_gamma_csv_to_root"),
            "debug",
        )

        command = run_signal.reco_command(
            args,
            binaries,
            "/tmp/input.root",
            Path("/tmp/candidates.csv"),
        )

        self.assertIn("--truth", command)
        self.assertIn("--truth-strategy", command)
        strategy_index = command.index("--truth-strategy")
        self.assertEqual(command[strategy_index + 1], "hgamma-closure")

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

    def test_dry_run_reports_remote_cache_download_plan(self):
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
                    "--xrootd",
                    "--download-remote",
                    "--dry-run",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            text = (outdir / "production_summary.txt").read_text()
            self.assertIn("download_remote: True", text)
            self.assertIn(f"cache_dir: {outdir / 'cache'}", text)
            self.assertIn("would_run_inputs:", text)
            self.assertIn("original_input=root://", text)
            self.assertIn("cached_input=", text)
            self.assertIn("download_source=root://", text)

    def test_remote_dry_run_without_download_fails_clearly(self):
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
                    "--xrootd",
                    "--dry-run",
                ],
                cwd=REPO_ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(
                "remote ROOT reading is not supported by the current Rust reader; "
                "rerun with --download-remote to cache files locally first",
                result.stderr,
            )

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
