#!/usr/bin/env python3
"""Manifest-driven H->rho gamma scouting signal production helper."""

from __future__ import annotations

import argparse
import csv
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_DATASET = (
    "/GluGluHtoRhoG_Par-M-125_TuneCP5_13p6TeV_powheg-pythia8-evtgen/"
    "RunIII2024Summer24NanoAODv15-150X_mcRun3_2024_realistic_v2-v2/NANOAODSIM"
)
DEFAULT_CONFIG = Path("configs/scouting/h_rho_gamma.toml")
DEFAULT_OUTDIR = Path("outputs/scouting_hrhogamma_signal")
DEFAULT_SAFE_MAX_FILES = 5
CSV_HEADER = (
    "run,luminosityBlock,event,photon_pt,photon_eta,photon_phi,"
    "pi_plus_pt,pi_plus_eta,pi_plus_phi,pi_minus_pt,pi_minus_eta,pi_minus_phi,"
    "rho_mass,rho_pt,rho_eta,rho_phi,h_mass,h_pt,h_eta,h_phi,"
    "delta_r_pipi,delta_r_gamma_rho,rho_pt_over_photon_pt"
)


@dataclass(frozen=True)
class InputFile:
    label: str
    path: str
    lfn: str | None = None


@dataclass
class FileResult:
    index: int
    input_path: str
    stdout_path: Path
    csv_path: Path | None
    processed_events: int
    accepted_candidates: int
    csv_rows: int
    status: str


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
        for enabled in (args.manifest is not None, args.resolve_das, args.local_files is not None)
        if enabled
    )
    if sources != 1:
        raise ValueError("choose exactly one of --manifest, --resolve-das, or --local-files")
    if args.max_events_per_file is not None and args.max_events_per_file < 0:
        raise ValueError("--max-events-per-file must be non-negative")
    if args.no_csv and args.plots_only:
        raise ValueError("--plots-only requires CSV input")
    if args.no_csv and not args.no_root:
        raise ValueError("--no-csv cannot write a combined ROOT skim")


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


def run_file(
    args: argparse.Namespace,
    input_file: InputFile,
    index: int,
    outdir: Path,
    per_file_dir: Path,
) -> FileResult:
    stdout_path, csv_path = per_file_paths(outdir, index, args.per_file_dir or per_file_dir)
    stdout_path.parent.mkdir(parents=True, exist_ok=True)

    if args.plots_only:
        rows = count_csv_rows(csv_path)
        return FileResult(index, input_file.path, stdout_path, csv_path, 0, rows, rows, "plots-only")

    if not input_file.path.startswith("root://") and not Path(input_file.path).exists():
        raise FileNotFoundError(f"input file does not exist: {input_file.path}")

    if args.skip_existing and csv_path.exists() and stdout_path.exists():
        stdout = stdout_path.read_text()
        return FileResult(
            index,
            input_file.path,
            stdout_path,
            csv_path,
            parse_count(stdout, "processed_events"),
            parse_count(stdout, "accepted_candidates"),
            count_csv_rows(csv_path),
            "skipped",
        )

    command = [
        "cargo",
        "run",
        "-p",
        "nano-io",
        "--example",
        "scouting_h_rho_gamma",
        "--",
        input_file.path,
    ]
    if args.max_events_per_file is not None:
        command.append(str(args.max_events_per_file))
    command.append(str(args.config))
    if not args.no_csv:
        command.extend(["--csv", str(csv_path)])

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

    return FileResult(
        index,
        input_file.path,
        stdout_path,
        None if args.no_csv else csv_path,
        parse_count(result.stdout, "processed_events"),
        parse_count(result.stdout, "accepted_candidates"),
        0 if args.no_csv else count_csv_rows(csv_path),
        "ok",
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


def write_combined_root(combined_csv: Path, combined_root: Path) -> str:
    command = [
        "cargo",
        "run",
        "-p",
        "nano-io",
        "--example",
        "scouting_h_rho_gamma_csv_to_root",
        "--",
        str(combined_csv),
        str(combined_root),
    ]
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
    return "requested"


def write_summary(
    path: Path,
    args: argparse.Namespace,
    dataset: str,
    manifest_path: Path | None,
    resolved_count: int,
    selected_inputs: list[InputFile],
    results: list[FileResult],
    combined_csv_rows: int,
    plot_status: str,
    root_status: str,
) -> None:
    total_processed = sum(result.processed_events for result in results)
    total_candidates = sum(result.accepted_candidates for result in results)
    if args.plots_only:
        total_candidates = combined_csv_rows
    mode = "dry-run" if args.dry_run else "plots-only" if args.plots_only else "run"
    lines = [
        "HToRhoGamma signal production summary",
        f"mode: {mode}",
        f"dataset: {dataset}",
        f"config: {args.config}",
        f"manifest: {manifest_path if manifest_path is not None else 'none'}",
        f"das_resolution: {'nano-cli dataset resolve' if args.resolve_das else 'not requested'}",
        f"xrootd: {args.xrootd}",
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
        for input_file in selected_inputs:
            lines.append(f"  {input_file.label} {input_file.path}")
    else:
        lines.append("per_file:")
        for result in results:
            lines.append(
                f"  file_{result.index:06d} candidates={result.csv_rows} "
                f"status={result.status} "
                f"processed={result.processed_events} "
                f"accepted={result.accepted_candidates} "
                f"input={result.input_path} "
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
        outdir.mkdir(parents=True, exist_ok=True)
        per_file_dir.mkdir(parents=True, exist_ok=True)

        manifest_path = None
        if args.resolve_das:
            manifest_path = resolve_das_manifest(args, outdir, limit)
            dataset, inputs = read_manifest_inputs(manifest_path, args.xrootd)
        elif args.manifest is not None:
            manifest_path = copy_manifest(args.manifest, outdir)
            dataset, inputs = read_manifest_inputs(manifest_path, args.xrootd)
        else:
            dataset, inputs = local_inputs(args.local_files)

        selected_inputs = select_inputs(inputs, limit)
        print(f"resolved files: {len(inputs)}")
        print(f"selected files: {len(selected_inputs)}")
        if not args.all_files and args.max_files is None:
            print(f"using safe default --max-files {DEFAULT_SAFE_MAX_FILES}")

        summary_path = outdir / "production_summary.txt"
        if args.dry_run:
            write_summary(
                summary_path,
                args,
                dataset,
                manifest_path,
                len(inputs),
                selected_inputs,
                [],
                0,
                "not run (dry-run)",
                "not run (dry-run)",
            )
            print(f"summary: {summary_path}")
            return 0

        results = [
            run_file(args, input_file, index, outdir, per_file_dir)
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
                write_combined_root(combined_csv, combined_root)
                root_status = str(combined_root)

        write_summary(
            summary_path,
            args,
            dataset,
            manifest_path,
            len(inputs),
            selected_inputs,
            results,
            combined_csv_rows,
            plot_status,
            root_status,
        )
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"summary: {summary_path}")
    if not args.no_csv:
        print(f"combined_csv: {combined_csv}")
    if not args.no_root:
        print(f"combined_root: {outdir / 'combined_candidates.root'}")
    print(f"plots: {plots_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
