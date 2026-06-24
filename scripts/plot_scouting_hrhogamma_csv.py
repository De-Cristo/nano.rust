#!/usr/bin/env python3
"""Summarize and optionally plot H->rho gamma scouting candidate CSV files."""

from __future__ import annotations

import argparse
import csv
import statistics
import sys
from pathlib import Path
try:
    import tomllib
except ImportError:  # pragma: no cover - Python < 3.11 fallback for local users.
    tomllib = None


EVENT_COLUMNS = ("run", "luminosityBlock", "event")
SUMMARY_COLUMNS = (
    "h_mass",
    "rho_mass",
    "photon_pt",
    "rho_pt",
    "pi_plus_pt",
    "pi_minus_pt",
    "h_pt",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "rho_pt_over_photon_pt",
)
PLOT_COLUMNS = SUMMARY_COLUMNS
PLOT_2D_COLUMNS = (
    ("rho_mass", "h_mass", "h_mass_vs_rho_mass.png"),
    ("h_mass", "photon_pt", "photon_pt_vs_h_mass.png"),
    ("h_mass", "rho_pt", "rho_pt_vs_h_mass.png"),
    ("h_mass", "delta_r_gamma_rho", "delta_r_gamma_rho_vs_h_mass.png"),
)
HIGGS_MASS_WINDOW = (100.0, 150.0)


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
        "--config",
        type=Path,
        default=None,
        help="optional HToRhoGamma TOML config for rho-mass summary counts",
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


def approximate_quantiles(values: list[float]) -> tuple[float, float, float] | None:
    if not values:
        return None
    ordered = sorted(values)
    return (
        percentile(ordered, 0.25),
        percentile(ordered, 0.50),
        percentile(ordered, 0.75),
    )


def percentile(ordered: list[float], fraction: float) -> float:
    if len(ordered) == 1:
        return ordered[0]
    position = fraction * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def count_in_window(values: list[float], low: float, high: float) -> int:
    return sum(1 for value in values if low < value < high)


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


def load_rho_mass_window(config_path: Path | None) -> tuple[float, float] | None:
    if config_path is None:
        return None
    if tomllib is None:
        return load_rho_mass_window_text(config_path)
    with config_path.open("rb") as handle:
        config = tomllib.load(handle)
    try:
        baseline = config["baseline"]["zcountinghlt_naive"]
        return (float(baseline["rho_mass_min"]), float(baseline["rho_mass_max"]))
    except KeyError as exc:
        raise ValueError(
            f"{config_path}: missing [baseline.zcountinghlt_naive].{exc.args[0]}"
        ) from exc


def load_rho_mass_window_text(config_path: Path) -> tuple[float, float]:
    current_table = None
    values = {}
    for raw_line in config_path.read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            current_table = line.strip("[]")
            continue
        if current_table != "baseline.zcountinghlt_naive" or "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        key = key.strip()
        if key in {"rho_mass_min", "rho_mass_max"}:
            values[key] = float(raw_value.strip())
    try:
        return (values["rho_mass_min"], values["rho_mass_max"])
    except KeyError as exc:
        raise ValueError(
            f"{config_path}: missing [baseline.zcountinghlt_naive].{exc.args[0]}"
        ) from exc


def build_summary(
    csv_path: Path,
    outdir: Path,
    prefix: str,
    rows: list[dict[str, str]],
    plot_status: str,
    rho_mass_window: tuple[float, float] | None = None,
) -> str:
    unique_events = {event_key(row) for row in rows}
    duplicate_event_entries = len(rows) - len(unique_events)
    h_mass_values = numeric_values(rows, "h_mass")
    rho_mass_values = numeric_values(rows, "rho_mass")
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
    for column, values in (("h_mass", h_mass_values), ("rho_mass", rho_mass_values)):
        quantiles = approximate_quantiles(values)
        if quantiles is None:
            lines.append(f"{column}_quantiles_approx: no values")
        else:
            q25, q50, q75 = quantiles
            lines.append(
                f"{column}_quantiles_approx: q25={q25:.6f} "
                f"median={q50:.6f} q75={q75:.6f}"
            )
    lines.append(
        f"h_mass_window_100_150: {count_in_window(h_mass_values, *HIGGS_MASS_WINDOW)}"
    )
    if rho_mass_window is None:
        lines.append("rho_mass_window_config: unavailable")
    else:
        low, high = rho_mass_window
        lines.append(f"rho_mass_window_config: {low:.6f} < rho_mass < {high:.6f}")
        lines.append(f"rho_mass_window_count: {count_in_window(rho_mass_values, low, high)}")
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
    import numpy as np
    
    try:
        import mplhep as hep
        plt.style.use(hep.style.CMS)
    except ImportError:
        hep = None

    written = []
    for column in PLOT_COLUMNS:
        values = numeric_values(rows, column)
        figure, axis = plt.subplots(figsize=(8.0, 6.0))
        if values:
            bins_count = 100
            if hep:
                counts, bins = np.histogram(values, bins=bins_count)
                hep.histplot(counts, bins, ax=axis, histtype="fill", label="Signal")
                hep.cms.label("Simulation Preliminary", data=False, loc=0, ax=axis)
            else:
                axis.hist(values, bins=bins_count)
        axis.set_title(f"{prefix}: {column}")
        axis.set_xlabel(column)
        axis.set_ylabel("candidates")
        figure.tight_layout()
        output = outdir / f"{column}.png"
        figure.savefig(output)
        plt.close(figure)
        written.append(output)
    for x_column, y_column, filename in PLOT_2D_COLUMNS:
        x_values = numeric_values(rows, x_column)
        y_values = numeric_values(rows, y_column)
        figure, axis = plt.subplots(figsize=(8.0, 6.0))
        if x_values and y_values:
            bins_count = 50
            if hep:
                h, xedges, yedges = np.histogram2d(x_values, y_values, bins=bins_count)
                hep.hist2dplot(h, xedges, yedges, ax=axis, cmap="viridis")
                hep.cms.label("Simulation Preliminary", data=False, loc=0, ax=axis)
            else:
                axis.hist2d(x_values, y_values, bins=bins_count)
        axis.set_title(f"{prefix}: {filename.removesuffix('.png')}")
        axis.set_xlabel(x_column)
        axis.set_ylabel(y_column)
        figure.tight_layout()
        output = outdir / filename
        figure.savefig(output)
        plt.close(figure)
        written.append(output)
    return written


def main() -> int:
    args = parse_args()
    try:
        rows, fieldnames = read_rows(args.csv_path, args.max_rows)
        require_columns(fieldnames)
        rho_mass_window = load_rho_mass_window(args.config)
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

        summary = build_summary(
            args.csv_path,
            args.outdir,
            args.prefix,
            rows,
            plot_status,
            rho_mass_window,
        )
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
