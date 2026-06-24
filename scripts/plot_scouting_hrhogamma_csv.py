#!/usr/bin/env python3
"""Summarize and optionally plot H->rho gamma scouting candidate CSV files."""

from __future__ import annotations

import argparse
import csv
import statistics
import sys
from pathlib import Path


EVENT_COLUMNS = ("run", "luminosityBlock", "event")
SUMMARY_COLUMNS = (
    "h_mass",
    "rho_mass",
    "photon_pt",
    "rho_pt",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "rho_pt_over_photon_pt",
)
PLOT_COLUMNS = SUMMARY_COLUMNS


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize and plot Stage-7 HToRhoGamma candidate CSV output."
    )
    parser.add_argument("csv_path", type=Path, help="candidate CSV from scouting_h_rho_gamma")
    parser.add_argument(
        "--outdir",
        type=Path,
        default=Path("plots/scouting_hrhogamma"),
        help="directory for summary.txt and PNG plots",
    )
    parser.add_argument(
        "--prefix",
        default="scouting_hrhogamma",
        help="label used in plot titles and summary metadata",
    )
    parser.add_argument(
        "--max-rows",
        type=int,
        default=None,
        help="read at most N candidate rows from the CSV",
    )
    parser.add_argument(
        "--no-plots",
        action="store_true",
        help="write summary.txt only",
    )
    return parser.parse_args()


def read_rows(path: Path, max_rows: int | None) -> tuple[list[dict[str, str]], list[str]]:
    if max_rows is not None and max_rows < 0:
        raise ValueError("--max-rows must be non-negative")
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"empty CSV or missing header: {path}")
        rows = []
        for index, row in enumerate(reader):
            if max_rows is not None and index >= max_rows:
                break
            rows.append(row)
        return rows, list(reader.fieldnames)


def require_columns(fieldnames: list[str]) -> None:
    missing = [
        name
        for name in (*EVENT_COLUMNS, *SUMMARY_COLUMNS)
        if name not in fieldnames
    ]
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


def event_key(row: dict[str, str]) -> tuple[str, str, str]:
    return tuple(row[name] for name in EVENT_COLUMNS)


def stats_line(column: str, values: list[float]) -> str:
    if not values:
        return f"{column}: no values"
    return (
        f"{column}: min={min(values):.6f} "
        f"mean={statistics.mean(values):.6f} "
        f"max={max(values):.6f}"
    )


def build_summary(
    csv_path: Path,
    outdir: Path,
    prefix: str,
    rows: list[dict[str, str]],
    plot_status: str,
) -> str:
    unique_events = {event_key(row) for row in rows}
    duplicate_event_entries = len(rows) - len(unique_events)
    lines = [
        "HToRhoGamma candidate CSV validation",
        f"input_csv: {csv_path}",
        f"outdir: {outdir}",
        f"prefix: {prefix}",
        f"candidate_rows: {len(rows)}",
        f"unique_events: {len(unique_events)}",
        f"duplicate_event_entries: {duplicate_event_entries}",
    ]
    if not rows:
        lines.append("warning: no candidate rows present")
    for column in SUMMARY_COLUMNS:
        lines.append(stats_line(column, numeric_values(rows, column)))
    lines.append(f"plots: {plot_status}")
    return "\n".join(lines) + "\n"


def write_summary(path: Path, text: str) -> None:
    path.write_text(text)


def plot_histograms(
    rows: list[dict[str, str]],
    outdir: Path,
    prefix: str,
) -> list[Path]:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    written = []
    for column in PLOT_COLUMNS:
        values = numeric_values(rows, column)
        figure, axis = plt.subplots(figsize=(6.0, 4.0))
        if values:
            axis.hist(values, bins=min(20, max(1, len(values))))
        axis.set_title(f"{prefix}: {column}")
        axis.set_xlabel(column)
        axis.set_ylabel("candidates")
        figure.tight_layout()
        output = outdir / f"{column}.png"
        figure.savefig(output)
        plt.close(figure)
        written.append(output)
    return written


def main() -> int:
    args = parse_args()
    try:
        rows, fieldnames = read_rows(args.csv_path, args.max_rows)
        require_columns(fieldnames)
        args.outdir.mkdir(parents=True, exist_ok=True)

        plot_status = "skipped (--no-plots)"
        written_plots: list[Path] = []
        if not args.no_plots:
            try:
                written_plots = plot_histograms(rows, args.outdir, args.prefix)
                plot_status = f"wrote {len(written_plots)} PNG files"
            except ImportError:
                plot_status = "skipped (matplotlib unavailable)"
                print("plots skipped: matplotlib is not available", file=sys.stderr)

        summary = build_summary(args.csv_path, args.outdir, args.prefix, rows, plot_status)
        summary_path = args.outdir / "summary.txt"
        write_summary(summary_path, summary)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(f"summary: {summary_path}")
    for path in written_plots:
        print(f"plot: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
