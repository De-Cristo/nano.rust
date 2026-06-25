#!/usr/bin/env python3
"""Manifest-driven H->rho gamma scouting signal production helper."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import posixpath
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path


DEFAULT_DATASET = (
    "/GluGluHtoRhoG_Par-M-125_TuneCP5_13p6TeV_powheg-pythia8-evtgen/"
    "RunIII2024Summer24NanoAODv15-150X_mcRun3_2024_realistic_v2-v2/NANOAODSIM"
)
DEFAULT_CONFIG = Path("configs/scouting/h_rho_gamma.toml")
DEFAULT_OUTDIR = Path("outputs/scouting_hrhogamma_signal")
DEFAULT_SAFE_MAX_FILES = 5
DEFAULT_DOWNLOAD_TOOL = "xrdcp"
DEFAULT_DOWNLOAD_TIMEOUT_SECONDS = 3600
GLOBAL_XROOTD_HOST = "root://cms-xrd-global.cern.ch/"
REMOTE_READING_UNSUPPORTED = (
    "remote ROOT reading is not supported by the current Rust reader; "
    "rerun with --download-remote to cache files locally first"
)
CSV_HEADER = (
    "run,luminosityBlock,event,photon_pt,photon_eta,photon_phi,"
    "pi_plus_pt,pi_plus_eta,pi_plus_phi,pi_minus_pt,pi_minus_eta,pi_minus_phi,"
    "rho_mass,rho_pt,rho_eta,rho_phi,h_mass,h_pt,h_eta,h_phi,"
    "delta_r_pipi,delta_r_gamma_rho,rho_pt_over_photon_pt"
)
HGAMMA_EVENT_COUNT_KEYS = (
    "hgamma_events_total",
    "hgamma_events_with_gen_h",
    "hgamma_events_with_gen_hgamma",
    "hgamma_events_with_reco_photon_preselection",
    "hgamma_events_with_reco_photon_matched_dr_0p1",
    "hgamma_events_with_reco_photon_matched_dr_0p2",
    "hgamma_events_with_reco_photon_matched_dr_0p1_and_any_os_track_pair",
    "hgamma_events_with_reco_photon_matched_dr_0p1_and_accepted_candidate",
    "hgamma_events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_10",
    "hgamma_events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_15",
    "hgamma_events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_20",
)


@dataclass(frozen=True)
class InputFile:
    label: str
    path: str
    lfn: str | None = None


@dataclass(frozen=True)
class InputPlan:
    original_input: str
    run_input: str
    cached_input: Path | None
    download_source: str | None


@dataclass(frozen=True)
class Binaries:
    reco: Path | None
    csv_to_root: Path | None
    build_mode: str


@dataclass
class FileResult:
    index: int
    original_input: str
    cached_input: Path | None
    stdout_path: Path
    csv_path: Path | None
    processed_events: int
    accepted_candidates: int
    csv_rows: int
    download_status: str
    run_status: str
    hgamma_counts: dict[str, int] = field(default_factory=dict)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the HToRhoGamma scouting demo over a signal manifest."
    )
    parser.add_argument("--dataset", default=DEFAULT_DATASET, help="DAS dataset name")
    parser.add_argument("--manifest", type=Path, help="existing nano-das manifest JSON")
    parser.add_argument(
        "--resolve-das",
        action="store_true",
        help="resolve --dataset with nano-cli dataset resolve before running",
    )
    parser.add_argument(
        "--local-files",
        nargs="+",
        help="local ROOT file paths to run instead of resolving/reading DAS",
    )
    parser.add_argument(
        "--local-dir",
        type=Path,
        help="directory of local ROOT files to run in sorted order",
    )
    parser.add_argument(
        "--local-glob",
        default="*.root",
        help="glob used with --local-dir",
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="HToRhoGamma TOML config",
    )
    parser.add_argument("--outdir", type=Path, default=DEFAULT_OUTDIR)
    parser.add_argument("--max-files", type=int, default=None)
    parser.add_argument(
        "--all-files",
        action="store_true",
        help="process every file in the manifest or DAS result",
    )
    parser.add_argument("--max-events-per-file", type=int, default=None)
    parser.add_argument(
        "--xrootd",
        action="store_true",
        help="use manifest global XRootD URLs instead of local paths/LFNs",
    )
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument(
        "--plots-only",
        action="store_true",
        help="reuse existing per-file CSVs and only merge/plot/summarize",
    )
    parser.add_argument("--no-root", action="store_true")
    parser.add_argument("--no-csv", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--physics-report",
        action="store_true",
        help="explicitly request physics_summary.md; reports are written by default when CSV exists",
    )
    parser.add_argument(
        "--no-physics-report",
        action="store_true",
        help="skip physics_summary.md generation",
    )
    parser.add_argument(
        "--report-title",
        default="HToRhoGamma Signal Physics Report",
        help="title for physics_summary.md",
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="reuse existing CSV outputs and regenerate plots/report without ROOT processing",
    )
    parser.add_argument(
        "--truth",
        action="store_true",
        help="request optional GenPart truth validation in the Rust example",
    )
    parser.add_argument(
        "--truth-strategy",
        choices=("topology-proxy", "hgamma-closure"),
        default="topology-proxy",
        help="truth validation strategy forwarded to the Rust example when --truth is set",
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="build and run release example binaries",
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="assume example binaries already exist",
    )
    parser.add_argument(
        "--use-cargo-run",
        action="store_true",
        help="fall back to cargo run per file instead of build-once execution",
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=None,
        help="directory for cached remote ROOT files",
    )
    parser.add_argument(
        "--download-remote",
        action="store_true",
        help="copy root:// or /store inputs into --cache-dir before reading",
    )
    parser.add_argument(
        "--download-tool",
        default=DEFAULT_DOWNLOAD_TOOL,
        help="remote copy tool, normally xrdcp",
    )
    parser.add_argument(
        "--keep-cache",
        action="store_true",
        help="keep cached ROOT files after processing",
    )
    parser.add_argument(
        "--clean-cache",
        action="store_true",
        help="remove cached ROOT files after successful per-file processing",
    )
    parser.add_argument(
        "--force-download",
        action="store_true",
        help="overwrite existing non-empty cached files",
    )
    parser.add_argument(
        "--download-timeout",
        type=int,
        default=DEFAULT_DOWNLOAD_TIMEOUT_SECONDS,
        help="seconds before remote file downloads time out",
    )
    parser.add_argument(
        "--per-file-dir",
        type=Path,
        default=None,
        help="advanced: directory containing/reusing file_*.candidates.csv outputs",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def shell_join(command: list[str]) -> str:
    return " ".join(shlex_quote(part) for part in command)


def shlex_quote(value: str) -> str:
    if value and all(c.isalnum() or c in "-_./:=+" for c in value):
        return value
    return "'" + value.replace("'", "'\"'\"'") + "'"


def safe_file_limit(args: argparse.Namespace) -> int | None:
    if args.all_files:
        return None
    if args.max_files is None:
        return DEFAULT_SAFE_MAX_FILES
    if args.max_files < 0:
        raise ValueError("--max-files must be non-negative")
    return args.max_files


def validate_source_args(args: argparse.Namespace) -> None:
    sources = sum(
        1
        for enabled in (
            args.manifest is not None,
            args.resolve_das,
            args.local_files is not None,
            args.local_dir is not None,
        )
        if enabled
    )
    if sources != 1:
        raise ValueError(
            "choose exactly one of --manifest, --resolve-das, --local-files, or --local-dir"
        )
    if args.max_events_per_file is not None and args.max_events_per_file < 0:
        raise ValueError("--max-events-per-file must be non-negative")
    if args.no_csv and args.plots_only:
        raise ValueError("--plots-only requires CSV input")
    if args.no_csv and args.report_only:
        raise ValueError("--report-only requires CSV input")
    if args.no_csv and not args.no_root:
        raise ValueError("--no-csv cannot write a combined ROOT skim")
    if args.physics_report and args.no_physics_report:
        raise ValueError("choose at most one of --physics-report or --no-physics-report")
    if args.clean_cache and args.keep_cache:
        raise ValueError("choose at most one of --clean-cache or --keep-cache")
    if args.download_timeout < 1:
        raise ValueError("--download-timeout must be positive")
    if args.no_build and args.use_cargo_run:
        raise ValueError("--no-build cannot be combined with --use-cargo-run")


def target_profile(args: argparse.Namespace) -> str:
    return "release" if args.release else "debug"


def example_binary(name: str, args: argparse.Namespace) -> Path:
    return repo_root() / "target" / target_profile(args) / "examples" / name


def build_example(name: str, args: argparse.Namespace, outdir: Path) -> Path:
    binary = example_binary(name, args)
    if args.no_build:
        if not binary.exists():
            raise FileNotFoundError(f"required example binary does not exist: {binary}")
        return binary

    command = ["cargo", "build", "-p", "nano-io", "--example", name]
    if args.release:
        command.append("--release")
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (outdir / f"build_{name}.stdout.txt").write_text(result.stdout)
    (outdir / f"build_{name}.stderr.txt").write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            f"build failed with exit code {result.returncode}: {shell_join(command)}\n"
            f"{result.stderr}"
        )
    if not binary.exists():
        raise FileNotFoundError(f"build succeeded but binary was not found: {binary}")
    return binary


def prepare_binaries(args: argparse.Namespace, outdir: Path) -> Binaries:
    if args.use_cargo_run:
        return Binaries(None, None, "cargo-run")
    mode = target_profile(args)
    reco = None if args.plots_only or args.report_only else build_example("scouting_h_rho_gamma", args, outdir)
    csv_to_root = None
    if not args.no_root and not args.no_csv:
        csv_to_root = build_example("scouting_h_rho_gamma_csv_to_root", args, outdir)
    return Binaries(reco, csv_to_root, mode)


def build_cache_dir(args: argparse.Namespace, outdir: Path) -> Path:
    return args.cache_dir if args.cache_dir is not None else outdir / "cache"


def is_remote_input(path: str) -> bool:
    return path.startswith("root://") or path.startswith("/store/")


def remote_source_for_download(path: str) -> str:
    if path.startswith("root://"):
        return path
    if path.startswith("/store/"):
        return f"{GLOBAL_XROOTD_HOST}{path}"
    return path


def cache_path_for_input(cache_dir: Path, index: int, input_path: str) -> Path:
    basename = posixpath.basename(input_path.rstrip("/")) or f"input_{index:06d}.root"
    digest = hashlib.sha1(input_path.encode("utf-8")).hexdigest()[:12]
    return cache_dir / f"file_{index:06d}_{digest}_{basename}"


def effective_input_plan(
    index: int,
    input_file: InputFile,
    cache_dir: Path,
    download_remote: bool,
) -> InputPlan:
    if not is_remote_input(input_file.path):
        return InputPlan(
            original_input=input_file.path,
            run_input=input_file.path,
            cached_input=None,
            download_source=None,
        )
    if not download_remote:
        raise ValueError(REMOTE_READING_UNSUPPORTED)
    cached_input = cache_path_for_input(cache_dir, index, input_file.path)
    return InputPlan(
        original_input=input_file.path,
        run_input=str(cached_input),
        cached_input=cached_input,
        download_source=remote_source_for_download(input_file.path),
    )


def resolve_das_manifest(args: argparse.Namespace, outdir: Path, limit: int | None) -> Path:
    manifest_path = outdir / "manifest.json"
    command = [
        "cargo",
        "run",
        "-p",
        "nano-cli",
        "--",
        "dataset",
        "resolve",
        "--dataset",
        args.dataset,
        "--output",
        str(manifest_path),
    ]
    if limit is not None:
        command.extend(["--max-files", str(limit)])
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (outdir / "manifest_resolve.stdout.txt").write_text(result.stdout)
    (outdir / "manifest_resolve.stderr.txt").write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            "DAS manifest resolution failed with "
            f"exit code {result.returncode}: {shell_join(command)}\n{result.stderr}"
        )
    return manifest_path


def copy_manifest(manifest: Path, outdir: Path) -> Path:
    output = outdir / "manifest.json"
    if manifest.resolve() != output.resolve():
        shutil.copyfile(manifest, output)
    return output


def read_manifest_inputs(path: Path, use_xrootd: bool) -> tuple[str, list[InputFile]]:
    with path.open() as handle:
        manifest = json.load(handle)
    files = []
    for index, item in enumerate(manifest.get("files", []), start=1):
        source = item.get("source", {})
        lfn = item.get("lfn") or source.get("lfn")
        if use_xrootd:
            selected = source.get("preferred_url") or source.get("global_xrootd_url") or lfn
        else:
            selected = source.get("local_path") or source.get("preferred_url") or lfn
        if not selected:
            raise ValueError(f"manifest file {index} has no usable path")
        files.append(InputFile(label=f"file_{index:06d}", path=selected, lfn=lfn))
    return manifest.get("dataset", "unknown"), files


def local_inputs(paths: list[str]) -> tuple[str, list[InputFile]]:
    return (
        "local-files",
        [
            InputFile(label=f"file_{index:06d}", path=str(Path(path)), lfn=None)
            for index, path in enumerate(paths, start=1)
        ],
    )


def local_dir_inputs(directory: Path, pattern: str) -> tuple[str, list[InputFile]]:
    if not directory.exists():
        raise FileNotFoundError(f"local directory does not exist: {directory}")
    if not directory.is_dir():
        raise NotADirectoryError(f"--local-dir is not a directory: {directory}")
    paths = sorted(path for path in directory.glob(pattern) if path.is_file())
    if not paths:
        raise FileNotFoundError(f"no files matching {pattern!r} in {directory}")
    return (
        "local-dir",
        [
            InputFile(label=f"file_{index:06d}", path=str(path), lfn=None)
            for index, path in enumerate(paths, start=1)
        ],
    )


def select_inputs(inputs: list[InputFile], limit: int | None) -> list[InputFile]:
    if limit is None:
        return inputs
    return inputs[:limit]


def per_file_paths(outdir: Path, index: int, per_file_dir: Path | None) -> tuple[Path, Path]:
    directory = per_file_dir if per_file_dir is not None else outdir / "per_file"
    stem = f"file_{index:06d}"
    return directory / f"{stem}.stdout.txt", directory / f"{stem}.candidates.csv"


def count_csv_rows(path: Path) -> int:
    if not path.exists():
        return 0
    with path.open(newline="") as handle:
        reader = csv.reader(handle)
        try:
            next(reader)
        except StopIteration:
            return 0
        return sum(1 for _ in reader)


def parse_count(stdout: str, key: str) -> int:
    prefix = f"{key}:"
    for line in stdout.splitlines():
        if line.startswith(prefix):
            return int(line.split(":", 1)[1].strip())
    return 0


def parse_hgamma_counts(stdout: str) -> dict[str, int]:
    return {key: parse_count(stdout, key) for key in HGAMMA_EVENT_COUNT_KEYS}


def download_remote_input(
    args: argparse.Namespace,
    plan: InputPlan,
    index: int,
    outdir: Path,
) -> str:
    if plan.cached_input is None or plan.download_source is None:
        return "not needed"
    if plan.cached_input.exists() and plan.cached_input.stat().st_size > 0 and not args.force_download:
        return "reused"

    plan.cached_input.parent.mkdir(parents=True, exist_ok=True)
    command = [args.download_tool, "-f", plan.download_source, str(plan.cached_input)]
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=args.download_timeout,
    )
    download_stdout = outdir / "per_file" / f"file_{index:06d}.download.stdout.txt"
    download_stderr = outdir / "per_file" / f"file_{index:06d}.download.stderr.txt"
    download_stdout.write_text(result.stdout)
    download_stderr.write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            f"file {index} download failed with exit code {result.returncode}: "
            f"{plan.download_source}\ncommand: {shell_join(command)}\nstderr: {download_stderr}"
        )
    if not plan.cached_input.exists() or plan.cached_input.stat().st_size == 0:
        raise RuntimeError(f"file {index} download produced an empty cache file: {plan.cached_input}")
    return "downloaded"


def reco_command(
    args: argparse.Namespace,
    binaries: Binaries,
    run_input: str,
    csv_path: Path,
) -> list[str]:
    if args.use_cargo_run:
        command = ["cargo", "run", "-p", "nano-io", "--example", "scouting_h_rho_gamma"]
        if args.release:
            command.append("--release")
        command.extend(["--", run_input])
    else:
        if binaries.reco is None:
            raise ValueError("scouting_h_rho_gamma binary was not prepared")
        command = [str(binaries.reco), run_input]

    if args.max_events_per_file is not None:
        command.append(str(args.max_events_per_file))
    command.append(str(args.config))
    if not args.no_csv:
        command.extend(["--csv", str(csv_path)])
    if args.truth:
        command.append("--truth")
        command.extend(["--truth-strategy", args.truth_strategy])
    return command


def run_file(
    args: argparse.Namespace,
    binaries: Binaries,
    input_file: InputFile,
    index: int,
    outdir: Path,
    per_file_dir: Path,
    cache_dir: Path,
) -> FileResult:
    stdout_path, csv_path = per_file_paths(outdir, index, args.per_file_dir or per_file_dir)
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    plan = effective_input_plan(index, input_file, cache_dir, args.download_remote)

    if args.plots_only or args.report_only:
        rows = count_csv_rows(csv_path)
        stdout = stdout_path.read_text() if stdout_path.exists() else ""
        return FileResult(
            index,
            plan.original_input,
            plan.cached_input,
            stdout_path,
            csv_path,
            0,
            rows,
            rows,
            "not run (report-only)" if args.report_only else "not run (plots-only)",
            "report-only" if args.report_only else "plots-only",
            parse_hgamma_counts(stdout),
        )

    if args.skip_existing and csv_path.exists() and stdout_path.exists():
        stdout = stdout_path.read_text()
        return FileResult(
            index,
            plan.original_input,
            plan.cached_input,
            stdout_path,
            csv_path,
            parse_count(stdout, "processed_events"),
            parse_count(stdout, "accepted_candidates"),
            count_csv_rows(csv_path),
            "not run (skip-existing)",
            "skipped",
            parse_hgamma_counts(stdout),
        )

    download_status = download_remote_input(args, plan, index, outdir)
    if plan.cached_input is None and not Path(plan.run_input).exists():
        raise FileNotFoundError(f"input file does not exist: {plan.run_input}")

    command = reco_command(args, binaries, plan.run_input, csv_path)

    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    stdout_path.write_text(
        f"$ {shell_join(command)}\n\n"
        f"[stdout]\n{result.stdout}\n"
        f"[stderr]\n{result.stderr}\n"
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"file {index} failed with exit code {result.returncode}: {input_file.path}\n"
            f"command: {shell_join(command)}\nstdout: {stdout_path}"
        )

    if args.clean_cache and plan.cached_input is not None and plan.cached_input.exists():
        plan.cached_input.unlink()

    return FileResult(
        index,
        plan.original_input,
        plan.cached_input,
        stdout_path,
        None if args.no_csv else csv_path,
        parse_count(result.stdout, "processed_events"),
        parse_count(result.stdout, "accepted_candidates"),
        0 if args.no_csv else count_csv_rows(csv_path),
        download_status,
        "ok",
        parse_hgamma_counts(result.stdout),
    )


def merge_candidate_csvs(csv_paths: list[Path], output: Path) -> int:
    output.parent.mkdir(parents=True, exist_ok=True)
    header: list[str] | None = None
    rows_written = 0
    with output.open("w", newline="") as out_handle:
        writer = None
        for path in csv_paths:
            if not path.exists():
                raise FileNotFoundError(f"missing per-file candidate CSV: {path}")
            with path.open(newline="") as in_handle:
                reader = csv.reader(in_handle)
                try:
                    current_header = next(reader)
                except StopIteration as exc:
                    raise ValueError(f"empty per-file candidate CSV: {path}") from exc
                if header is None:
                    header = current_header
                    writer = csv.writer(out_handle, lineterminator="\n")
                    writer.writerow(header)
                elif current_header != header:
                    raise ValueError(f"candidate CSV header mismatch: {path}")
                assert writer is not None
                for row in reader:
                    writer.writerow(row)
                    rows_written += 1
    if header is None:
        output.write_text(CSV_HEADER + "\n")
    return rows_written


def write_combined_root(
    args: argparse.Namespace,
    binaries: Binaries,
    combined_csv: Path,
    combined_root: Path,
) -> str:
    if args.use_cargo_run:
        command = [
            "cargo",
            "run",
            "-p",
            "nano-io",
            "--example",
            "scouting_h_rho_gamma_csv_to_root",
        ]
        if args.release:
            command.append("--release")
        command.extend(["--", str(combined_csv), str(combined_root)])
    else:
        if binaries.csv_to_root is None:
            raise ValueError("scouting_h_rho_gamma_csv_to_root binary was not prepared")
        command = [str(binaries.csv_to_root), str(combined_csv), str(combined_root)]
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (combined_root.parent / "combined_root.stdout.txt").write_text(result.stdout)
    (combined_root.parent / "combined_root.stderr.txt").write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            "combined ROOT writing failed with "
            f"exit code {result.returncode}: {shell_join(command)}\n{result.stderr}"
        )
    return result.stdout.strip()


def run_plots(args: argparse.Namespace, combined_csv: Path, plots_dir: Path) -> str:
    command = [
        sys.executable,
        str(repo_root() / "scripts" / "plot_scouting_hrhogamma_csv.py"),
        str(combined_csv),
        "--outdir",
        str(plots_dir),
        "--config",
        str(args.config),
    ]
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (plots_dir.parent / "plots.stdout.txt").write_text(result.stdout)
    (plots_dir.parent / "plots.stderr.txt").write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            f"plotting failed with exit code {result.returncode}: {shell_join(command)}\n"
            f"{result.stderr}"
        )
    if "plots skipped" in result.stderr:
        return "skipped (matplotlib unavailable)"
    summary_path = plots_dir / "summary.txt"
    if summary_path.exists():
        for line in summary_path.read_text().splitlines():
            if line.startswith("plots:"):
                return line.split(":", 1)[1].strip()
    return "requested"


def fraction(numerator: int, denominator: int) -> str:
    if denominator <= 0:
        return "unavailable"
    return f"{numerator / denominator:.6f}"


def truth_column_count(csv_path: Path, column: str) -> int:
    if not csv_path.exists():
        return 0
    with csv_path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None or column not in reader.fieldnames:
            return 0
        return sum(1 for row in reader if row.get(column) == "1")


def write_hgamma_event_flow_plot(summary: dict[str, object], plots_dir: Path) -> str:
    plots_dir.mkdir(parents=True, exist_ok=True)
    labels = [
        "total",
        "gen H",
        "gen Hgamma",
        "reco gamma",
        "gamma dR<0.1",
        "OS tracks",
        "accepted",
        "|dmH|<15",
    ]
    values = [
        int(summary.get("hgamma_events_total", 0)),
        int(summary.get("hgamma_events_with_gen_h", 0)),
        int(summary.get("hgamma_events_with_gen_hgamma", 0)),
        int(summary.get("hgamma_events_with_reco_photon_preselection", 0)),
        int(summary.get("hgamma_events_with_reco_photon_matched_dr_0p1", 0)),
        int(
            summary.get(
                "hgamma_events_with_reco_photon_matched_dr_0p1_and_any_os_track_pair",
                0,
            )
        ),
        int(
            summary.get(
                "hgamma_events_with_reco_photon_matched_dr_0p1_and_accepted_candidate",
                0,
            )
        ),
        int(
            summary.get(
                "hgamma_events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_15",
                0,
            )
        ),
    ]
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        return "skipped (matplotlib unavailable)"

    figure, axis = plt.subplots(figsize=(7.2, 4.2), constrained_layout=True)
    axis.bar(labels, values, color="#2f6fbb")
    axis.tick_params(axis="x", labelrotation=25)
    axis.set_ylabel("Events")
    axis.set_title("Photon-anchored Higgs closure event flow", fontsize=11)
    output = plots_dir / "hgamma_event_flow.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    return str(output)


def write_hgamma_closure_summary(
    args: argparse.Namespace,
    outdir: Path,
    results: list[FileResult],
    combined_csv: Path,
    plots_dir: Path,
) -> Path | None:
    if not args.truth or args.truth_strategy != "hgamma-closure":
        return None
    totals = {
        key: sum(result.hgamma_counts.get(key, 0) for result in results)
        for key in HGAMMA_EVENT_COUNT_KEYS
    }
    candidate_rows = count_csv_rows(combined_csv) if combined_csv.exists() else 0
    summary = {
        "truth_strategy": args.truth_strategy,
        **totals,
        "candidate_rows": candidate_rows,
        "hgamma_closure_available_candidates": truth_column_count(
            combined_csv, "hgamma_closure_available"
        ),
        "hgamma_closure_matched_candidates": truth_column_count(
            combined_csv, "hgamma_closure_matched"
        ),
        "hgamma_photon_matched_dr_0p1_candidates": truth_column_count(
            combined_csv, "hgamma_photon_matched_dr_0p1"
        ),
        "hgamma_higgs_closed_mass_10_candidates": truth_column_count(
            combined_csv, "hgamma_higgs_closed_mass_10"
        ),
        "hgamma_higgs_closed_mass_15_candidates": truth_column_count(
            combined_csv, "hgamma_higgs_closed_mass_15"
        ),
        "hgamma_higgs_closed_mass_20_candidates": truth_column_count(
            combined_csv, "hgamma_higgs_closed_mass_20"
        ),
    }
    event_flow_plot = write_hgamma_event_flow_plot(summary, plots_dir)
    summary["hgamma_event_flow_plot"] = event_flow_plot

    json_path = outdir / "hgamma_closure_summary.json"
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    lines = [
        "# Photon-Anchored Higgs Closure Summary",
        "",
        "This report matches the selected reco photon to a Higgs-descendant GenPart photon, "
        "builds the gen rho recoil as gen H - gen gamma, and checks reco rho/H closure.",
        "",
        f"truth_strategy: `{args.truth_strategy}`",
        f"candidate rows: `{candidate_rows}`",
        f"hgamma closure available candidates: `{summary['hgamma_closure_available_candidates']}` / `{candidate_rows}` (`{fraction(summary['hgamma_closure_available_candidates'], candidate_rows)}`)",
        f"hgamma closure matched candidates: `{summary['hgamma_closure_matched_candidates']}` / `{candidate_rows}` (`{fraction(summary['hgamma_closure_matched_candidates'], candidate_rows)}`)",
        f"photon matched dR<0.1 candidates: `{summary['hgamma_photon_matched_dr_0p1_candidates']}` / `{candidate_rows}` (`{fraction(summary['hgamma_photon_matched_dr_0p1_candidates'], candidate_rows)}`)",
        f"Higgs closed |m(reco H)-m(gen H)|<15 GeV candidates: `{summary['hgamma_higgs_closed_mass_15_candidates']}` / `{candidate_rows}` (`{fraction(summary['hgamma_higgs_closed_mass_15_candidates'], candidate_rows)}`)",
        "",
        "## Event Flow",
        "",
    ]
    for key in HGAMMA_EVENT_COUNT_KEYS:
        lines.append(f"- {key}: `{summary[key]}`")
    lines.extend(
        [
            "",
            "## Artifacts",
            "",
            f"- JSON: [{json_path.name}]({json_path.name})",
            f"- event flow plot: `{event_flow_plot}`",
        ]
    )
    md_path = outdir / "hgamma_closure_summary.md"
    md_path.write_text("\n".join(lines) + "\n")
    return md_path


def write_physics_report(
    args: argparse.Namespace,
    dataset: str,
    manifest_path: Path | None,
    inputs: list[InputFile],
    selected_inputs: list[InputFile],
    results: list[FileResult],
    combined_csv: Path,
    root_status: str,
    plots_dir: Path,
    plot_status: str,
) -> Path | None:
    if args.no_physics_report or args.no_csv or not combined_csv.exists():
        return None
    total_processed = sum(result.processed_events for result in results)
    total_candidates = sum(result.accepted_candidates for result in results)
    if args.plots_only or args.report_only:
        total_candidates = count_csv_rows(combined_csv)
    successful_files = sum(
        1 for result in results if result.run_status in {"ok", "skipped", "plots-only", "report-only"}
    )
    command = [
        sys.executable,
        str(repo_root() / "scripts" / "write_scouting_hrhogamma_report.py"),
        "--csv",
        str(combined_csv),
        "--outdir",
        str(args.outdir),
        "--dataset",
        dataset,
        "--manifest",
        str(manifest_path) if manifest_path is not None else "none",
        "--config",
        str(args.config),
        "--selected-files",
        str(len(selected_inputs)),
        "--successful-files",
        str(successful_files),
        "--failed-files",
        str(len(results) - successful_files),
        "--processed-events",
        str(total_processed),
        "--accepted-candidates",
        str(total_candidates),
        "--combined-root",
        root_status,
        "--plots-dir",
        str(plots_dir),
        "--plots-status",
        plot_status,
        "--command-line",
        shell_join([sys.executable, *sys.argv]),
        "--title",
        args.report_title,
    ]
    if dataset in {"local-files", "local-dir"}:
        command.extend(["--local-files-count", str(len(inputs))])
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    (args.outdir / "physics_report.stdout.txt").write_text(result.stdout)
    (args.outdir / "physics_report.stderr.txt").write_text(result.stderr)
    if result.returncode != 0:
        raise RuntimeError(
            "physics report writing failed with "
            f"exit code {result.returncode}: {shell_join(command)}\n{result.stderr}"
        )
    return args.outdir / "physics_summary.md"


def write_summary(
    path: Path,
    args: argparse.Namespace,
    binaries: Binaries,
    dataset: str,
    manifest_path: Path | None,
    resolved_count: int,
    selected_inputs: list[InputFile],
    input_plans: list[InputPlan],
    results: list[FileResult],
    combined_csv_rows: int,
    plot_status: str,
    root_status: str,
    cache_dir: Path,
) -> None:
    total_processed = sum(result.processed_events for result in results)
    total_candidates = sum(result.accepted_candidates for result in results)
    if args.plots_only:
        total_candidates = combined_csv_rows
    mode = (
        "dry-run"
        if args.dry_run
        else "report-only"
        if args.report_only
        else "plots-only"
        if args.plots_only
        else "run"
    )
    lines = [
        "HToRhoGamma signal production summary",
        f"mode: {mode}",
        f"dataset: {dataset}",
        f"config: {args.config}",
        f"manifest: {manifest_path if manifest_path is not None else 'none'}",
        f"das_resolution: {'nano-cli dataset resolve' if args.resolve_das else 'not requested'}",
        f"xrootd: {args.xrootd}",
        f"execution_binary: {binaries.reco if binaries.reco is not None else 'cargo run'}",
        f"csv_to_root_binary: {binaries.csv_to_root if binaries.csv_to_root is not None else 'cargo run' if not args.no_root and not args.no_csv else 'not needed'}",
        f"build_mode: {binaries.build_mode}",
        f"cache_dir: {cache_dir}",
        f"download_remote: {args.download_remote}",
        f"download_tool: {args.download_tool}",
        f"download_timeout: {args.download_timeout}",
        f"cache_policy: {'clean after successful file' if args.clean_cache else 'keep cached files'}",
        f"force_download: {args.force_download}",
        f"truth: {args.truth}",
        f"truth_strategy: {args.truth_strategy}",
        f"local_glob: {args.local_glob}",
        f"resolved_files: {resolved_count}",
        f"selected_files: {len(selected_inputs)}",
        f"max_files: {'all' if args.all_files else safe_file_limit(args)}",
        f"all_files: {args.all_files}",
        f"max_events_per_file: {args.max_events_per_file if args.max_events_per_file is not None else 'all'}",
        f"skip_existing: {args.skip_existing}",
        f"no_csv: {args.no_csv}",
        f"combined_csv: {'disabled (--no-csv)' if args.no_csv else path.parent / 'combined_candidates.csv'}",
        f"combined_csv_rows: {combined_csv_rows}",
        f"combined_root: {root_status}",
        f"plots_dir: {path.parent / 'plots'}",
        f"plots: {plot_status}",
        f"total_processed_events: {total_processed}",
        f"total_accepted_candidates: {total_candidates}",
    ]
    if args.dry_run:
        lines.append(f"would_run_files: {len(selected_inputs)}")
        lines.append("would_run_inputs:")
        for input_file, plan in zip(selected_inputs, input_plans):
            lines.append(
                f"  {input_file.label} original_input={plan.original_input} "
                f"run_input={plan.run_input} "
                f"cached_input={plan.cached_input if plan.cached_input is not None else 'none'} "
                f"download_source={plan.download_source if plan.download_source is not None else 'none'}"
            )
    else:
        lines.append("per_file:")
        for result in results:
            lines.append(
                f"  file_{result.index:06d} candidates={result.csv_rows} "
                f"download_status={result.download_status} "
                f"run_status={result.run_status} "
                f"processed={result.processed_events} "
                f"accepted={result.accepted_candidates} "
                f"original_input={result.original_input} "
                f"cached_input={result.cached_input if result.cached_input is not None else 'none'} "
                f"stdout={result.stdout_path}"
            )
    path.write_text("\n".join(lines) + "\n")


def main() -> int:
    args = parse_args()
    try:
        validate_source_args(args)
        limit = safe_file_limit(args)
        outdir = args.outdir
        per_file_dir = outdir / "per_file"
        plots_dir = outdir / "plots"
        cache_dir = build_cache_dir(args, outdir)
        outdir.mkdir(parents=True, exist_ok=True)
        per_file_dir.mkdir(parents=True, exist_ok=True)

        manifest_path = None
        if args.resolve_das:
            manifest_path = resolve_das_manifest(args, outdir, limit)
            dataset, inputs = read_manifest_inputs(manifest_path, args.xrootd)
        elif args.manifest is not None:
            manifest_path = copy_manifest(args.manifest, outdir)
            dataset, inputs = read_manifest_inputs(manifest_path, args.xrootd)
        elif args.local_dir is not None:
            dataset, inputs = local_dir_inputs(args.local_dir, args.local_glob)
        else:
            dataset, inputs = local_inputs(args.local_files)

        selected_inputs = select_inputs(inputs, limit)
        input_plans = [
            effective_input_plan(index, input_file, cache_dir, args.download_remote)
            for index, input_file in enumerate(selected_inputs, start=1)
        ]
        print(f"resolved files: {len(inputs)}")
        print(f"selected files: {len(selected_inputs)}")
        if not args.all_files and args.max_files is None:
            print(f"using safe default --max-files {DEFAULT_SAFE_MAX_FILES}")

        summary_path = outdir / "production_summary.txt"
        if args.dry_run:
            write_summary(
                summary_path,
                args,
                Binaries(None, None, "dry-run"),
                dataset,
                manifest_path,
                len(inputs),
                selected_inputs,
                input_plans,
                [],
                0,
                "not run (dry-run)",
                "not run (dry-run)",
                cache_dir,
            )
            print(f"summary: {summary_path}")
            return 0

        if args.download_remote:
            cache_dir.mkdir(parents=True, exist_ok=True)
        binaries = prepare_binaries(args, outdir)

        results = [
            run_file(args, binaries, input_file, index, outdir, per_file_dir, cache_dir)
            for index, input_file in enumerate(selected_inputs, start=1)
        ]

        combined_csv_rows = 0
        plot_status = "disabled (--no-csv)"
        root_status = "disabled (--no-root)" if args.no_root else "not written"
        combined_csv = outdir / "combined_candidates.csv"
        if not args.no_csv:
            csv_paths = [result.csv_path for result in results if result.csv_path is not None]
            combined_csv_rows = merge_candidate_csvs(csv_paths, combined_csv)
            plot_status = run_plots(args, combined_csv, plots_dir)
            if args.no_root:
                root_status = "disabled (--no-root)"
            else:
                combined_root = outdir / "combined_candidates.root"
                write_combined_root(args, binaries, combined_csv, combined_root)
                root_status = str(combined_root)

        if not args.no_csv:
            write_hgamma_closure_summary(args, outdir, results, combined_csv, plots_dir)

        write_summary(
            summary_path,
            args,
            binaries,
            dataset,
            manifest_path,
            len(inputs),
            selected_inputs,
            input_plans,
            results,
            combined_csv_rows,
            plot_status,
            root_status,
            cache_dir,
        )
        physics_report = write_physics_report(
            args,
            dataset,
            manifest_path,
            inputs,
            selected_inputs,
            results,
            combined_csv,
            root_status,
            plots_dir,
            plot_status,
        )
    except (OSError, RuntimeError, ValueError, subprocess.TimeoutExpired) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"summary: {summary_path}")
    if not args.no_csv:
        print(f"combined_csv: {combined_csv}")
    if not args.no_root:
        print(f"combined_root: {outdir / 'combined_candidates.root'}")
    print(f"plots: {plots_dir}")
    if physics_report is not None:
        print(f"physics_summary: {physics_report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
