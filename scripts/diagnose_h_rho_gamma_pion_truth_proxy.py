#!/usr/bin/env python3
"""Diagnose charged-pion truth-proxy matching for HToRhoGamma candidates."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_OUTDIR = Path("outputs/h_rho_gamma_pion_truth_diag")
DEFAULT_SAFE_MAX_FILES = 5
THRESHOLDS = ("0.05", "0.10", "0.20", "0.30", "0.50")
POOLS = (
    "pi_only_same_charge",
    "pi_only_opposite_charge",
    "any_charged_hadron_same_charge",
    "any_charged_stable_like_same_charge",
    "any_charged_particle_same_charge",
)
PLOT_NAMES = (
    "nearest_pi_plus_dr.png",
    "nearest_pi_minus_dr.png",
    "nearest_any_charged_hadron_plus_dr.png",
    "nearest_any_charged_hadron_minus_dr.png",
    "match_fraction_by_pool_and_threshold.png",
    "h_mass_by_pion_match_category.png",
    "rho_mass_by_pion_match_category.png",
)


@dataclass(frozen=True)
class InputFile:
    label: str
    path: Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Diagnose why accepted HToRhoGamma reco pions rarely match GenPart pions."
    )
    parser.add_argument("--local-dir", type=Path, required=True)
    parser.add_argument("--local-glob", default="*.root")
    parser.add_argument("--candidate-csv", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, default=DEFAULT_OUTDIR)
    parser.add_argument("--max-files", type=int, default=None)
    parser.add_argument("--all-files", action="store_true")
    parser.add_argument("--max-events-per-file", type=int, default=None)
    parser.add_argument("--max-candidates", type=int, default=None)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument("--no-plots", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def shell_join(command: list[str]) -> str:
    return " ".join(str(part) for part in command)


def local_dir_inputs(directory: Path, pattern: str) -> list[InputFile]:
    if not directory.exists():
        raise FileNotFoundError(f"local directory does not exist: {directory}")
    if not directory.is_dir():
        raise NotADirectoryError(f"not a directory: {directory}")
    paths = sorted(path for path in directory.glob(pattern) if path.is_file())
    return [
        InputFile(f"file_{index:06d}", path)
        for index, path in enumerate(paths, start=1)
    ]


def select_inputs(
    inputs: list[InputFile], max_files: int | None, all_files: bool
) -> list[InputFile]:
    if all_files:
        return inputs
    limit = max_files if max_files is not None else DEFAULT_SAFE_MAX_FILES
    return inputs[:limit]


def read_candidate_rows(
    path: Path, max_candidates: int | None
) -> tuple[list[dict[str, str]], list[str]]:
    if max_candidates is not None and max_candidates < 0:
        raise ValueError("--max-candidates must be non-negative")
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"empty candidate CSV: {path}")
        rows = []
        for index, row in enumerate(reader):
            if max_candidates is not None and index >= max_candidates:
                break
            rows.append(row)
        return rows, list(reader.fieldnames)


def write_candidate_subset(path: Path, rows: list[dict[str, str]], fieldnames: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def group_candidates_by_event(
    rows: list[dict[str, str]]
) -> dict[tuple[str, str, str], list[dict[str, str]]]:
    grouped: dict[tuple[str, str, str], list[dict[str, str]]] = {}
    for row in rows:
        key = (row["run"], row["luminosityBlock"], row["event"])
        grouped.setdefault(key, []).append(row)
    return grouped


def diag_binary(release: bool) -> Path:
    profile = "release" if release else "debug"
    return repo_root() / "target" / profile / "examples" / "h_rho_gamma_pion_truth_diag"


def build_binary(release: bool) -> Path:
    command = ["cargo", "build", "-p", "nano-io", "--example", "h_rho_gamma_pion_truth_diag"]
    if release:
        command.append("--release")
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"diagnostic binary build failed with exit code {result.returncode}: {shell_join(command)}\n"
            f"{result.stderr}"
        )
    return diag_binary(release)


def prepare_binary(args: argparse.Namespace) -> Path:
    binary = diag_binary(args.release)
    if args.no_build:
        if not binary.exists():
            raise FileNotFoundError(f"diagnostic binary does not exist: {binary}")
        return binary
    return build_binary(args.release)


def diagnostic_command(
    binary: Path,
    input_path: Path,
    candidate_csv: Path,
    json_path: Path,
    max_events: int | None,
) -> list[str]:
    command = [str(binary), str(input_path)]
    if max_events is not None:
        command.append(str(max_events))
    command.extend(
        [
            "--candidate-csv",
            str(candidate_csv),
            "--out-json",
            str(json_path),
            "--out-text",
            str(json_path.with_suffix(".txt")),
        ]
    )
    return command


def run_file(
    binary: Path,
    input_file: InputFile,
    candidate_csv: Path,
    per_file_dir: Path,
    max_events: int | None,
    skip_existing: bool,
) -> dict:
    json_path = per_file_dir / f"{input_file.label}.summary.json"
    stdout_path = per_file_dir / f"{input_file.label}.stdout.txt"
    if skip_existing and json_path.exists() and stdout_path.exists():
        data = json.loads(json_path.read_text())
        data["files"] = [{"label": input_file.label, "path": str(input_file.path), "run_status": "skipped"}]
        return data

    command = diagnostic_command(binary, input_file.path, candidate_csv, json_path, max_events)
    result = subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    stdout_path.write_text(
        f"$ {shell_join(command)}\n\n[stdout]\n{result.stdout}\n[stderr]\n{result.stderr}\n"
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{input_file.label} failed with exit code {result.returncode}: {input_file.path}\n"
            f"stdout log: {stdout_path}"
        )
    data = json.loads(json_path.read_text())
    data["files"] = [
        {
            "label": input_file.label,
            "path": str(input_file.path),
            "summary_json": str(json_path),
            "stdout": str(stdout_path),
            "run_status": "ok",
        }
    ]
    return data


def merge_summaries(summaries: list[dict]) -> dict:
    merged = {
        "candidate_rows": 0,
        "diagnosed_candidates": 0,
        "category_counts": {},
        "leg_threshold_counts": {},
        "both_threshold_counts": {},
        "pool_event_counts": {},
        "pool_particle_counts": {},
        "nearest_dr": {},
        "candidate_diagnostics": [],
        "files": [],
        "packed_genpart_branches_present": False,
    }
    for summary in summaries:
        merged["candidate_rows"] += int(summary.get("candidate_rows", 0))
        merged["diagnosed_candidates"] += int(summary.get("diagnosed_candidates", 0))
        merge_count_map(merged["category_counts"], summary.get("category_counts", {}))
        merge_count_map(merged["pool_event_counts"], summary.get("pool_event_counts", {}))
        merge_count_map(merged["pool_particle_counts"], summary.get("pool_particle_counts", {}))
        merge_nested_counts(merged["leg_threshold_counts"], summary.get("leg_threshold_counts", {}))
        merge_nested_counts(merged["both_threshold_counts"], summary.get("both_threshold_counts", {}))
        merge_vectors(merged["nearest_dr"], summary.get("nearest_dr", {}))
        merged["candidate_diagnostics"].extend(summary.get("candidate_diagnostics", []))
        merged["files"].extend(summary.get("files", []))
        merged["packed_genpart_branches_present"] |= bool(summary.get("packed_genpart_branches_present", False))
    merged["recommendation"] = recommendation(merged)
    return merged


def merge_count_map(target: dict, source: dict) -> None:
    for key, count in source.items():
        target[str(key)] = int(target.get(str(key), 0)) + int(count)


def merge_nested_counts(target: dict, source: dict) -> None:
    for outer_key, nested in source.items():
        if all(isinstance(value, (int, float)) for value in nested.values()):
            merge_count_map(target.setdefault(str(outer_key), {}), nested)
            continue
        outer = target.setdefault(str(outer_key), {})
        for inner_key, counts in nested.items():
            merge_count_map(outer.setdefault(str(inner_key), {}), counts)


def merge_vectors(target: dict, source: dict) -> None:
    for key, values in source.items():
        target.setdefault(str(key), []).extend(float(value) for value in values)


def recommendation(summary: dict) -> str:
    diagnosed = max(int(summary.get("diagnosed_candidates", 0)), 1)
    category = summary.get("category_counts", {})
    pion = int(category.get("both_reco_pions_match_gen_pions_dr0p1", 0))
    hadron = int(category.get("both_match_any_charged_hadron_dr0p1", 0))
    pi_plus_events = int(summary.get("pool_event_counts", {}).get("all_pi_plus", 0))
    pi_minus_events = int(summary.get("pool_event_counts", {}).get("all_pi_minus", 0))
    if min(pi_plus_events, pi_minus_events) * 10 < diagnosed:
        return "Use photon truth only until PackedGenPart or tracking truth can provide charged-pion truth."
    if hadron > pion * 5 and hadron > 0:
        return "Charged-hadron matching works better than pion-only matching; treat rho as a charged-candidate mass hypothesis."
    if pion == 0:
        return "GenPart charged-particle matches are sparse or far from accepted reco pions; survey PackedGenPart or tracking truth next."
    return "GenPart pions exist but pion matching is sparse; use this as a diagnostic before any truth-informed cut study."


def fraction(count: int, total: int) -> str:
    if total <= 0:
        return "n/a"
    return f"{count / total:.6f}"


def numeric_stats(values: list[float]) -> str:
    if not values:
        return "no values"
    ordered = sorted(values)
    return (
        f"count={len(values)} min={min(values):.6f} "
        f"mean={statistics.mean(values):.6f} "
        f"median={ordered[len(ordered) // 2]:.6f} max={max(values):.6f}"
    )


def write_report(path: Path, summary: dict, plot_status: str) -> None:
    diagnosed = int(summary.get("diagnosed_candidates", 0))
    lines = [
        "# HToRhoGamma Charged-Pion Truth-Proxy Diagnosis",
        "",
        "This is a diagnostic report for accepted HToRhoGamma candidates. It does not change reconstruction cuts or claim final pion-level truth efficiency.",
        "",
        "## Inputs And Scope",
        "",
        f"- mode: `{summary.get('mode', 'run')}`",
        f"- selected files: `{summary.get('selected_files', len(summary.get('files', [])))}`",
        f"- candidate rows supplied: `{summary.get('candidate_rows', 0)}`",
        f"- diagnosed candidates: `{diagnosed}`",
        f"- PackedGenPart branches present: `{str(bool(summary.get('packed_genpart_branches_present', False))).lower()}`",
        f"- plots: `{plot_status}`",
        "",
        "## Candidate Category Counts",
        "",
    ]
    for category, count in sorted(summary.get("category_counts", {}).items()):
        lines.append(f"- {category}: `{count}` / `{diagnosed}` (`{fraction(int(count), diagnosed)}`)")
    lines.extend(["", "## GenPart Pool Breakdown", ""])
    for pool, count in sorted(summary.get("pool_event_counts", {}).items()):
        particle_count = int(summary.get("pool_particle_counts", {}).get(pool, 0))
        lines.append(
            f"- {pool}: events=`{count}` / `{diagnosed}` (`{fraction(int(count), diagnosed)}`), particles=`{particle_count}`"
        )
    lines.extend(["", "## Matching Threshold Scan", ""])
    lines.append("| pool | dR<0.05 | dR<0.10 | dR<0.20 | dR<0.30 | dR<0.50 |")
    lines.append("| --- | ---: | ---: | ---: | ---: | ---: |")
    for pool in POOLS:
        counts = summary.get("both_threshold_counts", {}).get(pool, {})
        values = [str(int(counts.get(threshold, 0))) for threshold in THRESHOLDS]
        lines.append(f"| {pool} | " + " | ".join(values) + " |")
    lines.extend(["", "## Nearest-Neighbor DeltaR Summaries", ""])
    for name, values in sorted(summary.get("nearest_dr", {}).items()):
        lines.append(f"- {name}: {numeric_stats([float(value) for value in values])}")
    lines.extend(["", "## Reco-Pion Sanity Checks", ""])
    diagnostics = summary.get("candidate_diagnostics", [])
    for key in ("pi_plus_pt", "pi_minus_pt", "pi_plus_eta", "pi_minus_eta", "h_mass", "rho_mass", "rho_pt_over_photon_pt"):
        values = [float(item[key]) for item in diagnostics if item.get(key) is not None]
        lines.append(f"- {key}: {numeric_stats(values)}")
    lines.extend(["", "## Interpretation", "", summary.get("recommendation", recommendation(summary)), ""])
    lines.extend(["## Plot Index", ""])
    for filename in PLOT_NAMES:
        lines.append(f"- `plots/{filename}`")
    path.write_text("\n".join(lines) + "\n")


def plot_diagnostics(summary: dict, outdir: Path, no_plots: bool) -> str:
    if no_plots:
        return "skipped (--no-plots)"
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        return "skipped (matplotlib unavailable)"

    outdir.mkdir(parents=True, exist_ok=True)
    plt.rcParams.update(
        {
            "font.size": 10,
            "axes.titlesize": 11,
            "axes.labelsize": 10,
            "xtick.labelsize": 9,
            "ytick.labelsize": 9,
            "legend.fontsize": 9,
            "figure.dpi": 120,
            "savefig.bbox": "tight",
        }
    )
    written = []
    nearest = summary.get("nearest_dr", {})
    axis_labels = {
        "nearest_pi_plus_dr": "Nearest GenPart pi+ DeltaR",
        "nearest_pi_minus_dr": "Nearest GenPart pi- DeltaR",
        "nearest_any_charged_hadron_plus_dr": "Nearest charged hadron (+) DeltaR",
        "nearest_any_charged_hadron_minus_dr": "Nearest charged hadron (-) DeltaR",
    }
    for key, label in axis_labels.items():
        values = [float(value) for value in nearest.get(key, [])]
        output = outdir / f"{key}.png"
        fig, ax = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        if values:
            ax.hist(values, bins=80, histtype="step", linewidth=1.5)
            for threshold in (0.1, 0.2, 0.3):
                ax.axvline(threshold, color="0.45", linestyle="--", linewidth=0.8)
        ax.set_title("HToRhoGamma pion truth diagnostic")
        ax.set_xlabel(label)
        ax.set_ylabel("Reco pion legs / bin")
        fig.savefig(output)
        fig.savefig(output.with_suffix(".pdf"))
        plt.close(fig)
        written.append(output)

    output = outdir / "match_fraction_by_pool_and_threshold.png"
    fig, ax = plt.subplots(figsize=(6.5, 5.0), constrained_layout=True)
    diagnosed = max(int(summary.get("diagnosed_candidates", 0)), 1)
    for pool in POOLS:
        counts = summary.get("both_threshold_counts", {}).get(pool, {})
        fractions = [int(counts.get(threshold, 0)) / diagnosed for threshold in THRESHOLDS]
        ax.step([float(t) for t in THRESHOLDS], fractions, where="mid", label=pool)
    ax.set_xlabel("DeltaR threshold")
    ax.set_ylabel("Both-leg match fraction")
    ax.set_title("Pion truth-proxy matching scan")
    ax.legend(frameon=False)
    fig.savefig(output)
    fig.savefig(output.with_suffix(".pdf"))
    plt.close(fig)
    written.append(output)

    diagnostics = summary.get("candidate_diagnostics", [])
    for value_key, filename, xlabel in (
        ("h_mass", "h_mass_by_pion_match_category.png", "m(H candidate) [GeV]"),
        ("rho_mass", "rho_mass_by_pion_match_category.png", "m(rho candidate) [GeV]"),
    ):
        matched = [
            float(item[value_key])
            for item in diagnostics
            if "both_reco_pions_match_gen_pions_dr0p1" in item.get("categories", [])
        ]
        unmatched = [
            float(item[value_key])
            for item in diagnostics
            if "both_reco_pions_match_gen_pions_dr0p1" not in item.get("categories", [])
        ]
        output = outdir / filename
        fig, ax = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        if matched:
            ax.hist(matched, bins=80, histtype="step", linewidth=1.5, label="both pion matched")
        if unmatched:
            ax.hist(unmatched, bins=80, histtype="step", linewidth=1.5, label="not both pion matched")
        if matched or unmatched:
            ax.legend(frameon=False)
        ax.set_title("HToRhoGamma pion truth diagnostic")
        ax.set_xlabel(xlabel)
        ax.set_ylabel("Candidates / bin")
        fig.savefig(output)
        fig.savefig(output.with_suffix(".pdf"))
        plt.close(fig)
        written.append(output)

    return f"wrote {len(written)} PNG files and {len(written)} PDF files"


def dry_run_summary(args: argparse.Namespace, inputs: list[InputFile], selected: list[InputFile], rows: list[dict[str, str]]) -> dict:
    return {
        "mode": "dry-run",
        "local_dir": str(args.local_dir),
        "candidate_csv": str(args.candidate_csv),
        "resolved_files": len(inputs),
        "selected_files": len(selected),
        "candidate_rows": len(rows),
        "diagnosed_candidates": 0,
        "files": [{"label": item.label, "path": str(item.path), "run_status": "dry-run"} for item in selected],
    }


def main() -> int:
    args = parse_args()
    rows, fieldnames = read_candidate_rows(args.candidate_csv, args.max_candidates)
    inputs = local_dir_inputs(args.local_dir, args.local_glob)
    selected = select_inputs(inputs, args.max_files, args.all_files)
    args.outdir.mkdir(parents=True, exist_ok=True)
    per_file_dir = args.outdir / "per_file"
    per_file_dir.mkdir(parents=True, exist_ok=True)
    candidate_subset = args.outdir / "selected_candidates.csv"
    write_candidate_subset(candidate_subset, rows, fieldnames)

    if args.dry_run:
        summary = dry_run_summary(args, inputs, selected, rows)
    else:
        binary = prepare_binary(args)
        summaries = [
            run_file(
                binary,
                input_file,
                candidate_subset,
                per_file_dir,
                args.max_events_per_file,
                args.skip_existing,
            )
            for input_file in selected
        ]
        summary = merge_summaries(summaries)
        summary.update(
            {
                "mode": "run",
                "local_dir": str(args.local_dir),
                "local_glob": args.local_glob,
                "candidate_csv": str(args.candidate_csv),
                "resolved_files": len(inputs),
                "selected_files": len(selected),
                "all_files": args.all_files,
                "max_events_per_file": args.max_events_per_file,
                "max_candidates": args.max_candidates,
                "diagnostic_binary": str(binary),
            }
        )
        summary["candidate_rows"] = len(rows)

    plot_status = plot_diagnostics(summary, args.outdir / "plots", args.no_plots or args.dry_run)
    summary["plot_status"] = plot_status
    summary.setdefault("recommendation", recommendation(summary))
    json_path = args.outdir / "pion_truth_proxy_diagnosis.json"
    report_path = args.outdir / "pion_truth_proxy_diagnosis.md"
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    write_report(report_path, summary, plot_status)

    print(f"resolved files: {len(inputs)}")
    print(f"selected files: {len(selected)}")
    print(f"candidate rows: {len(rows)}")
    print(f"diagnosed candidates: {summary.get('diagnosed_candidates', 0)}")
    print(f"report: {report_path}")
    print(f"json: {json_path}")
    print(f"plots: {plot_status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
