#!/usr/bin/env python3
"""Write a Markdown physics report for HToRhoGamma candidate production."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import statistics
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:  # pragma: no cover - Python < 3.11 fallback.
    tomllib = None


SUMMARY_COLUMNS = (
    "h_mass",
    "rho_mass",
    "photon_pt",
    "rho_pt",
    "h_pt",
    "pi_plus_pt",
    "pi_minus_pt",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "rho_pt_over_photon_pt",
)
EXPECTED_PLOTS = (
    "h_mass.png",
    "rho_mass.png",
    "photon_pt.png",
    "rho_pt.png",
    "h_pt.png",
    "pi_plus_pt.png",
    "pi_minus_pt.png",
    "delta_r_pipi.png",
    "delta_r_gamma_rho.png",
    "rho_pt_over_photon_pt.png",
    "h_mass_vs_rho_mass.png",
    "photon_pt_vs_h_mass.png",
    "rho_pt_vs_h_mass.png",
    "delta_r_gamma_rho_vs_h_mass.png",
)
DEFAULT_RHO_WINDOW = (0.3, 1.2)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Write physics_summary.md for HToRhoGamma signal production."
    )
    parser.add_argument("--csv", type=Path, required=True, help="combined candidate CSV")
    parser.add_argument("--outdir", type=Path, required=True, help="production output directory")
    parser.add_argument("--config", type=Path, default=None)
    parser.add_argument("--dataset", default="unknown")
    parser.add_argument("--manifest", default="none")
    parser.add_argument("--local-files-count", type=int, default=None)
    parser.add_argument("--branch-catalogue", default=None)
    parser.add_argument("--selected-files", type=int, default=0)
    parser.add_argument("--successful-files", type=int, default=0)
    parser.add_argument("--failed-files", type=int, default=0)
    parser.add_argument("--processed-events", type=int, default=0)
    parser.add_argument("--accepted-candidates", type=int, default=0)
    parser.add_argument("--combined-root", default="not written")
    parser.add_argument("--plots-dir", type=Path, default=None)
    parser.add_argument("--plots-status", default="unknown")
    parser.add_argument("--command-line", default="unknown")
    parser.add_argument("--title", default="HToRhoGamma Signal Physics Report")
    return parser.parse_args()


def read_rows(path: Path) -> tuple[list[dict[str, str]], list[str]]:
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"empty CSV or missing header: {path}")
        return list(reader), list(reader.fieldnames)


def require_columns(fieldnames: list[str]) -> None:
    missing = [column for column in SUMMARY_COLUMNS if column not in fieldnames]
    if missing:
        raise ValueError(f"missing required CSV columns: {', '.join(missing)}")


def numeric_values(rows: list[dict[str, str]], column: str) -> list[float]:
    values = []
    for index, row in enumerate(rows, start=2):
        raw = row.get(column, "")
        try:
            values.append(float(raw))
        except ValueError as exc:
            raise ValueError(f"row {index}: column {column} is not numeric: {raw!r}") from exc
    return values


def percentile(ordered: list[float], fraction: float) -> float:
    if len(ordered) == 1:
        return ordered[0]
    position = fraction * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def summarize(values: list[float]) -> dict[str, float | int] | None:
    if not values:
        return None
    ordered = sorted(values)
    return {
        "count": len(values),
        "min": min(values),
        "mean": statistics.mean(values),
        "median": statistics.median(values),
        "p16": percentile(ordered, 0.16),
        "p50": percentile(ordered, 0.50),
        "p84": percentile(ordered, 0.84),
        "max": max(values),
    }


def count_window(values: list[float], low: float, high: float) -> int:
    return sum(1 for value in values if low < value < high)


def fraction(count: int, total: int) -> str:
    if total == 0:
        return "unavailable"
    return f"{count / total:.6f}"


def load_config_metadata(config_path: Path | None) -> tuple[tuple[float, float], str | None, str]:
    if config_path is None:
        return DEFAULT_RHO_WINDOW, None, "default rho window"
    if not config_path.exists():
        return DEFAULT_RHO_WINDOW, None, "default rho window; config not found"
    if tomllib is None:
        return load_config_metadata_text(config_path)
    with config_path.open("rb") as handle:
        config = tomllib.load(handle)
    baseline = config.get("baseline", {}).get("zcountinghlt_naive", {})
    analysis = config.get("analysis", {})
    try:
        rho_window = (float(baseline["rho_mass_min"]), float(baseline["rho_mass_max"]))
        source = "configured rho window"
    except KeyError:
        rho_window = DEFAULT_RHO_WINDOW
        source = "default rho window; config missing rho window"
    branch_catalogue = analysis.get("branch_catalogue")
    return rho_window, branch_catalogue, source


def load_config_metadata_text(config_path: Path) -> tuple[tuple[float, float], str | None, str]:
    current_table = None
    baseline_values = {}
    branch_catalogue = None
    for raw_line in config_path.read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            current_table = line.strip("[]")
            continue
        if "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        key = key.strip()
        value = raw_value.strip().strip('"')
        if current_table == "analysis" and key == "branch_catalogue":
            branch_catalogue = value
        if current_table == "baseline.zcountinghlt_naive" and key in {
            "rho_mass_min",
            "rho_mass_max",
        }:
            baseline_values[key] = float(value)
    if {"rho_mass_min", "rho_mass_max"} <= baseline_values.keys():
        return (
            (baseline_values["rho_mass_min"], baseline_values["rho_mass_max"]),
            branch_catalogue,
            "configured rho window",
        )
    return DEFAULT_RHO_WINDOW, branch_catalogue, "default rho window; config missing rho window"


def relative_link(base: Path, path: Path) -> str:
    try:
        return path.relative_to(base).as_posix()
    except ValueError:
        return path.as_posix()


def plot_index(outdir: Path, plots_dir: Path, plots_status: str) -> tuple[list[str], list[str], str]:
    produced = []
    missing = []
    for name in EXPECTED_PLOTS:
        path = plots_dir / name
        if path.exists():
            produced.append(relative_link(outdir, path))
        else:
            missing.append(relative_link(outdir, path))
    if missing:
        reason = plots_status
        if reason in {"unknown", "requested"}:
            reason = "PNG file not present in plots directory"
    else:
        reason = ""
    return produced, missing, reason


def markdown_table(statistics_by_column: dict[str, dict[str, float | int] | None]) -> list[str]:
    lines = [
        "| variable | count | min | mean | median | p16 | p50 | p84 | max |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for column in SUMMARY_COLUMNS:
        summary = statistics_by_column[column]
        if summary is None:
            lines.append(f"| {column} | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a |")
            continue
        lines.append(
            f"| {column} | {summary['count']} | {summary['min']:.6f} | "
            f"{summary['mean']:.6f} | {summary['median']:.6f} | "
            f"{summary['p16']:.6f} | {summary['p50']:.6f} | "
            f"{summary['p84']:.6f} | {summary['max']:.6f} |"
        )
    return lines


def build_report(args: argparse.Namespace, rows: list[dict[str, str]], fieldnames: list[str]) -> str:
    require_columns(fieldnames)
    outdir = args.outdir
    plots_dir = args.plots_dir if args.plots_dir is not None else outdir / "plots"
    rho_window, discovered_catalogue, rho_window_source = load_config_metadata(args.config)
    branch_catalogue = args.branch_catalogue or discovered_catalogue or "unknown"
    values_by_column = {column: numeric_values(rows, column) for column in SUMMARY_COLUMNS}
    statistics_by_column = {
        column: summarize(values_by_column[column]) for column in SUMMARY_COLUMNS
    }
    h_mass_values = values_by_column["h_mass"]
    rho_mass_values = values_by_column["rho_mass"]
    candidate_rows = len(rows)
    candidate_rate = (
        f"{args.accepted_candidates / args.processed_events:.6f}"
        if args.processed_events > 0
        else "unavailable"
    )
    timestamp = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()
    produced_plots, missing_plots, missing_reason = plot_index(outdir, plots_dir, args.plots_status)

    h100_150 = count_window(h_mass_values, 100.0, 150.0)
    h115_135 = count_window(h_mass_values, 115.0, 135.0)
    h120_130 = count_window(h_mass_values, 120.0, 130.0)
    rho_count = count_window(rho_mass_values, *rho_window)

    lines = [
        f"# {args.title}",
        "",
        "This is a signal-sample sanity report for the HToRhoGamma scouting-oriented demonstrator. It is not a final analysis result.",
        "",
        "## Dataset And Command",
        "",
        f"- dataset: `{args.dataset}`",
        f"- manifest: `{args.manifest}`",
        f"- local files count: `{args.local_files_count if args.local_files_count is not None else 'n/a'}`",
        f"- config: `{args.config if args.config is not None else 'none'}`",
        f"- branch catalogue: `{branch_catalogue}`",
        f"- output directory: `{outdir}`",
        f"- command line: `{args.command_line}`",
        f"- timestamp UTC: `{timestamp}`",
        "",
        "## Production Summary",
        "",
        f"- selected files: `{args.selected_files}`",
        f"- successful files: `{args.successful_files}`",
        f"- failed files: `{args.failed_files}`",
        f"- total processed events: `{args.processed_events}`",
        f"- total accepted candidates: `{args.accepted_candidates}`",
        f"- candidate rows: `{candidate_rows}`",
        f"- candidate rate per processed event: `{candidate_rate}`",
        f"- combined CSV: `{args.csv}`",
        f"- combined ROOT: `{args.combined_root}`",
        f"- plots directory: `{plots_dir}`",
        "",
        "## Candidate Variable Summaries",
        "",
        *markdown_table(statistics_by_column),
        "",
        "## Physics Windows",
        "",
        f"- 100 < h_mass < 150: `{h100_150}` / `{candidate_rows}` (`{fraction(h100_150, candidate_rows)}`)",
        f"- 115 < h_mass < 135: `{h115_135}` / `{candidate_rows}` (`{fraction(h115_135, candidate_rows)}`)",
        f"- 120 < h_mass < 130: `{h120_130}` / `{candidate_rows}` (`{fraction(h120_130, candidate_rows)}`)",
        (
            f"- rho_mass configured window {rho_window[0]:.6f} < rho_mass < {rho_window[1]:.6f}: "
            f"`{rho_count}` / `{candidate_rows}` (`{fraction(rho_count, candidate_rows)}`)"
        ),
        f"- rho mass window source: `{rho_window_source}`",
        "",
        "## Plot Index",
        "",
    ]
    if produced_plots:
        lines.extend(f"- [{Path(path).stem}]({path})" for path in produced_plots)
    else:
        lines.append("- No PNG plots are present.")
    lines.extend(["", "## Expected plots not produced", ""])
    if missing_plots:
        lines.append(f"PNG plots were not produced: {missing_reason}")
        lines.append("")
        lines.extend(f"- `{path}`" for path in missing_plots)
    else:
        lines.append("- All expected PNG plots are present.")
    lines.extend(
        [
            "",
            "## Interpretation Notes",
            "",
            "- The report checks reconstruction-level candidate shapes only.",
            "- The current candidate selection is loose and demonstrator-oriented.",
            "- A Higgs-scale accumulation in signal MC is a sanity check, not a measurement.",
            "- No background modeling, trigger efficiency, scale factors, or systematic uncertainties are included.",
            "- GenPart truth matching is deferred to a later physics-validation stage.",
            "",
            "## Known Limitations",
            "",
            "- Native root:// streaming is not implemented; DAS production uses xrdcp cache staging.",
            "- NanoAODv15-like signal MC is not identical to true reduced Run-3 scouting object collections.",
            "- Matplotlib may be missing in the current environment.",
            "- The report is candidate-level and reconstruction-level only.",
            "- No truth matching is included yet.",
            "- No background comparison is included yet.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    try:
        rows, fieldnames = read_rows(args.csv)
        args.outdir.mkdir(parents=True, exist_ok=True)
        report = build_report(args, rows, fieldnames)
        output = args.outdir / "physics_summary.md"
        output.write_text(report)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    print(f"physics_summary: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
