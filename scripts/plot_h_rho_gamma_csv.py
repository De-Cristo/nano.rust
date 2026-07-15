#!/usr/bin/env python3
"""Summarize and optionally plot H->rho gamma offline NanoAOD candidates."""

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
TRUTH_REQUIRED_COLUMNS = ("truth_available", "truth_topology", "truth_matched")
TRUTH_SUMMARY_COLUMNS = (
    "reco_h_mass_minus_gen_h_mass",
    "reco_rho_mass_minus_gen_rho_mass",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_rho_pt_over_gen_rho_pt",
)
TRUTH_PLOT_COLUMNS = (
    "reco_h_mass_minus_gen_h_mass",
    "reco_rho_mass_minus_gen_rho_mass",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_rho_pt_over_gen_rho_pt",
)
TRUTH_PLOT_2D_COLUMNS = (
    ("gen_h_mass", "h_mass", "reco_h_mass_vs_gen_h_mass.png"),
    ("gen_rho_mass", "rho_mass", "reco_rho_mass_vs_gen_rho_mass.png"),
    ("gen_photon_pt", "photon_pt", "reco_photon_pt_vs_gen_photon_pt.png"),
    ("gen_rho_pt", "rho_pt", "reco_rho_pt_vs_gen_rho_pt.png"),
)
TRUTH_EXPECTED_PLOTS = (
    "truth_matched_fraction.png",
    "h_mass_truth_matched_vs_unmatched.png",
    *tuple(f"{column}.png" for column in TRUTH_PLOT_COLUMNS),
    *tuple(filename for _, _, filename in TRUTH_PLOT_2D_COLUMNS),
)
TRUTH_PROXY_REQUIRED_COLUMNS = (
    "truth_strategy",
    "truth_available",
    "truth_proxy_matched",
    "truth_proxy_matched_dr_0p1",
    "truth_proxy_matched_dr_0p2",
    "truth_proxy_matched_dr_0p3",
)
TRUTH_PROXY_SUMMARY_COLUMNS = (
    "reco_h_mass_minus_gen_h_proxy_mass",
    "reco_rho_mass_minus_gen_rho_proxy_mass",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_pi_plus_pt_over_gen_pi_plus_pt",
    "reco_pi_minus_pt_over_gen_pi_minus_pt",
    "reco_rho_pt_over_gen_rho_proxy_pt",
    "reco_h_pt_over_gen_h_proxy_pt",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "delta_r_reco_rho_gen_rho_proxy",
    "delta_r_reco_h_gen_h_proxy",
)
TRUTH_PROXY_PLOT_COLUMNS = (
    "reco_h_mass_minus_gen_h_proxy_mass",
    "reco_rho_mass_minus_gen_rho_proxy_mass",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_pi_plus_pt_over_gen_pi_plus_pt",
    "reco_pi_minus_pt_over_gen_pi_minus_pt",
    "reco_rho_pt_over_gen_rho_proxy_pt",
    "reco_h_pt_over_gen_h_proxy_pt",
)
TRUTH_PROXY_PLOT_2D_COLUMNS = (
    ("gen_h_proxy_mass", "h_mass", "reco_h_mass_vs_gen_h_proxy_mass.png"),
    ("gen_rho_proxy_mass", "rho_mass", "reco_rho_mass_vs_gen_rho_proxy_mass.png"),
    ("gen_photon_pt", "photon_pt", "reco_photon_pt_vs_gen_photon_pt.png"),
    ("gen_pi_plus_pt", "pi_plus_pt", "reco_pi_plus_pt_vs_gen_pi_plus_pt.png"),
    ("gen_pi_minus_pt", "pi_minus_pt", "reco_pi_minus_pt_vs_gen_pi_minus_pt.png"),
    ("gen_h_proxy_pt", "h_pt", "reco_h_pt_vs_gen_h_proxy_pt.png"),
)
TRUTH_PROXY_EXPECTED_PLOTS = (
    "truth_proxy_match_thresholds.png",
    "h_mass_truth_proxy_matched_vs_unmatched.png",
    "rho_mass_truth_proxy_matched_vs_unmatched.png",
    *tuple(f"{column}.png" for column in TRUTH_PROXY_PLOT_COLUMNS),
    *tuple(filename for _, _, filename in TRUTH_PROXY_PLOT_2D_COLUMNS),
)
HGAMMA_CLOSURE_REQUIRED_COLUMNS = (
    "truth_strategy",
    "hgamma_closure_available",
    "hgamma_closure_matched",
    "hgamma_photon_matched_dr_0p1",
    "hgamma_higgs_closed_mass_15",
)
HGAMMA_CLOSURE_SUMMARY_COLUMNS = (
    "delta_r_reco_photon_gen_photon",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_photon_eta_minus_gen_photon_eta",
    "reco_photon_phi_minus_gen_photon_phi",
    "reco_h_mass_minus_gen_h_mass",
    "reco_h_pt_over_gen_h_pt",
    "delta_r_reco_h_gen_h",
    "hgamma_gen_rho_recoil_mass",
    "reco_rho_mass_minus_gen_rho_recoil_mass",
    "reco_rho_pt_over_gen_rho_recoil_pt",
    "delta_r_reco_rho_gen_rho_recoil",
)
HGAMMA_CLOSURE_PLOT_COLUMNS = (
    "delta_r_reco_photon_gen_photon",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_h_mass_minus_gen_h_mass",
    "reco_h_pt_over_gen_h_pt",
    "delta_r_reco_h_gen_h",
    "hgamma_gen_rho_recoil_mass",
    "reco_rho_mass_minus_gen_rho_recoil_mass",
    "reco_rho_pt_over_gen_rho_recoil_pt",
    "delta_r_reco_rho_gen_rho_recoil",
)
HGAMMA_CLOSURE_PLOT_2D_COLUMNS = (
    ("hgamma_gen_h_mass", "h_mass", "hgamma_reco_h_mass_vs_gen_h_mass.png"),
    (
        "hgamma_gen_rho_recoil_mass",
        "rho_mass",
        "hgamma_reco_rho_mass_vs_gen_rho_recoil_mass.png",
    ),
    ("hgamma_gen_gamma_pt", "photon_pt", "hgamma_reco_photon_pt_vs_gen_photon_pt.png"),
    ("hgamma_gen_rho_recoil_pt", "rho_pt", "hgamma_reco_rho_pt_vs_gen_rho_recoil_pt.png"),
    ("hgamma_gen_h_pt", "h_pt", "hgamma_reco_h_pt_vs_gen_h_pt.png"),
)
HGAMMA_CLOSURE_EXPECTED_PLOTS = (
    "hgamma_higgs_closure_thresholds.png",
    "hgamma_photon_match_dr.png",
    *tuple(
        f"{column}.png" if column.startswith("hgamma_") else f"hgamma_{column}.png"
        for column in HGAMMA_CLOSURE_PLOT_COLUMNS
        if column != "delta_r_reco_photon_gen_photon"
    ),
    *tuple(filename for _, _, filename in HGAMMA_CLOSURE_PLOT_2D_COLUMNS),
)
HIGGS_MASS_WINDOW = (100.0, 150.0)
AXIS_LABELS = {
    "h_mass": "m(H candidate) [GeV]",
    "rho_mass": "m(rho candidate) [GeV]",
    "photon_pt": "Photon pT [GeV]",
    "rho_pt": "rho pT [GeV]",
    "h_pt": "H candidate pT [GeV]",
    "pi_plus_pt": "pi+ pT [GeV]",
    "pi_minus_pt": "pi- pT [GeV]",
    "delta_r_pipi": "DeltaR(pi+, pi-)",
    "delta_r_gamma_rho": "DeltaR(gamma, rho)",
    "rho_pt_over_photon_pt": "rho pT / photon pT",
    "reco_h_mass_minus_gen_h_mass": "mH(reco) - mH(gen) [GeV]",
    "reco_rho_mass_minus_gen_rho_mass": "mrho(reco) - mrho(gen) [GeV]",
    "delta_r_reco_photon_gen_photon": "DeltaR(reco gamma, gen gamma)",
    "delta_r_reco_pi_plus_gen_pi_plus": "DeltaR(reco pi+, gen pi+)",
    "delta_r_reco_pi_minus_gen_pi_minus": "DeltaR(reco pi-, gen pi-)",
    "reco_photon_pt_over_gen_photon_pt": "reco photon pT / gen photon pT",
    "reco_rho_pt_over_gen_rho_pt": "reco rho pT / gen rho pT",
    "gen_h_proxy_mass": "mH(gen proxy) [GeV]",
    "gen_rho_proxy_mass": "mrho(gen proxy) [GeV]",
    "gen_h_proxy_pt": "Gen H proxy pT [GeV]",
    "gen_rho_proxy_pt": "Gen rho proxy pT [GeV]",
    "gen_pi_plus_pt": "Gen pi+ pT [GeV]",
    "gen_pi_minus_pt": "Gen pi- pT [GeV]",
    "reco_h_mass_minus_gen_h_proxy_mass": "mH(reco) - mH(gen proxy) [GeV]",
    "reco_rho_mass_minus_gen_rho_proxy_mass": "mrho(reco) - mrho(gen proxy) [GeV]",
    "delta_r_reco_rho_gen_rho_proxy": "DeltaR(reco rho, gen rho proxy)",
    "delta_r_reco_h_gen_h_proxy": "DeltaR(reco H, gen H proxy)",
    "reco_pi_plus_pt_over_gen_pi_plus_pt": "reco pi+ pT / gen pi+ pT",
    "reco_pi_minus_pt_over_gen_pi_minus_pt": "reco pi- pT / gen pi- pT",
    "reco_rho_pt_over_gen_rho_proxy_pt": "reco rho pT / gen rho proxy pT",
    "reco_h_pt_over_gen_h_proxy_pt": "reco H pT / gen H proxy pT",
    "gen_h_mass": "mH(gen) [GeV]",
    "gen_rho_mass": "mrho(gen) [GeV]",
    "gen_photon_pt": "Gen photon pT [GeV]",
    "gen_rho_pt": "Gen rho pT [GeV]",
    "reco_photon_eta_minus_gen_photon_eta": "eta(reco gamma) - eta(gen gamma)",
    "reco_photon_phi_minus_gen_photon_phi": "phi(reco gamma) - phi(gen gamma)",
    "delta_r_reco_h_gen_h": "DeltaR(reco H, gen H)",
    "reco_h_pt_over_gen_h_pt": "reco H pT / gen H pT",
    "hgamma_gen_rho_recoil_mass": "m(H gen - gamma gen) [GeV]",
    "delta_r_reco_rho_gen_rho_recoil": "DeltaR(reco rho, gen H-gamma recoil)",
    "reco_rho_mass_minus_gen_rho_recoil_mass": "mrho(reco) - m(H-gamma recoil) [GeV]",
    "reco_rho_pt_over_gen_rho_recoil_pt": "reco rho pT / gen H-gamma recoil pT",
    "hgamma_gen_h_mass": "mH(gen) [GeV]",
    "hgamma_gen_gamma_pt": "Gen gamma pT [GeV]",
    "hgamma_gen_rho_recoil_pt": "Gen H-gamma recoil pT [GeV]",
    "hgamma_gen_h_pt": "Gen H pT [GeV]",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize and plot Stage-7 HToRhoGamma candidate CSV output."
    )
    parser.add_argument("csv_path", type=Path, help="candidate CSV from h_rho_gamma")
    parser.add_argument(
        "--outdir",
        type=Path,
        default=Path("plots/h_rho_gamma"),
        help="directory for summary.txt and PNG plots",
    )
    parser.add_argument(
        "--prefix",
        default="h_rho_gamma",
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


def optional_numeric_values(rows: list[dict[str, str]], column: str) -> list[float]:
    values = []
    for index, row in enumerate(rows, start=2):
        raw = row.get(column, "")
        if raw == "":
            continue
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


def has_truth_columns(fieldnames: list[str]) -> bool:
    return all(column in fieldnames for column in TRUTH_REQUIRED_COLUMNS)


def has_truth_proxy_columns(fieldnames: list[str]) -> bool:
    return all(column in fieldnames for column in TRUTH_PROXY_REQUIRED_COLUMNS)


def has_hgamma_closure_columns(fieldnames: list[str]) -> bool:
    return all(column in fieldnames for column in HGAMMA_CLOSURE_REQUIRED_COLUMNS)


def truth_count(rows: list[dict[str, str]], column: str, value: str) -> int:
    return sum(1 for row in rows if row.get(column, "").lower() == value)


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
    fieldnames = list(rows[0].keys()) if rows else []
    if has_truth_columns(fieldnames):
        lines.append("truth_columns: present")
        lines.append(f"truth_available_count: {truth_count(rows, 'truth_available', '1')}")
        lines.append(f"truth_matched_count: {truth_count(rows, 'truth_matched', '1')}")
        for column in TRUTH_SUMMARY_COLUMNS:
            if column in fieldnames:
                lines.append(stats_line(column, optional_numeric_values(rows, column)))
        for filename in TRUTH_EXPECTED_PLOTS:
            lines.append(f"truth_plot_expected: {filename}")
    else:
        lines.append("truth_columns: absent")
    if has_truth_proxy_columns(fieldnames):
        lines.append("truth_proxy_columns: present")
        lines.append(f"truth_proxy_available_count: {truth_count(rows, 'truth_available', '1')}")
        for column in (
            "truth_proxy_matched_dr_0p1",
            "truth_proxy_matched_dr_0p2",
            "truth_proxy_matched_dr_0p3",
        ):
            lines.append(f"{column}_count: {truth_count(rows, column, '1')}")
        for column in TRUTH_PROXY_SUMMARY_COLUMNS:
            if column in fieldnames:
                lines.append(stats_line(column, optional_numeric_values(rows, column)))
        for filename in TRUTH_PROXY_EXPECTED_PLOTS:
            lines.append(f"truth_proxy_plot_expected: {filename}")
    else:
        lines.append("truth_proxy_columns: absent")
    if has_hgamma_closure_columns(fieldnames):
        lines.append("hgamma_closure_columns: present")
        lines.append(
            f"hgamma_closure_available_count: {truth_count(rows, 'hgamma_closure_available', '1')}"
        )
        lines.append(
            f"hgamma_closure_matched_count: {truth_count(rows, 'hgamma_closure_matched', '1')}"
        )
        for column in (
            "hgamma_photon_matched_dr_0p1",
            "hgamma_photon_matched_dr_0p2",
            "hgamma_higgs_closed_mass_10",
            "hgamma_higgs_closed_mass_15",
            "hgamma_higgs_closed_mass_20",
            "hgamma_higgs_closed_dr_0p3",
            "hgamma_higgs_closed_dr_0p5",
        ):
            lines.append(f"{column}_count: {truth_count(rows, column, '1')}")
        for column in HGAMMA_CLOSURE_SUMMARY_COLUMNS:
            if column in fieldnames:
                lines.append(stats_line(column, optional_numeric_values(rows, column)))
        for filename in HGAMMA_CLOSURE_EXPECTED_PLOTS:
            lines.append(f"hgamma_plot_expected: {filename}")
    else:
        lines.append("hgamma_closure_columns: absent")
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
    
    hep_plot_style(plt)
    try:
        import mplhep as hep
    except ImportError:
        hep = None

    written = []
    for column in PLOT_COLUMNS:
        values = numeric_values(rows, column)
        figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        if values:
            bins_count = 100
            if hep:
                counts, bins = np.histogram(values, bins=bins_count)
                hep.histplot(counts, bins, ax=axis, histtype="step", label="Signal")
                hep.cms.label("Simulation", data=False, loc=0, ax=axis, fontsize=11)
            else:
                axis.hist(values, bins=bins_count, histtype="step", linewidth=1.5)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(column))
        axis.set_ylabel("Candidates / bin")
        output = outdir / f"{column}.png"
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)
    for x_column, y_column, filename in PLOT_2D_COLUMNS:
        x_values = numeric_values(rows, x_column)
        y_values = numeric_values(rows, y_column)
        figure, axis = plt.subplots(figsize=(6.0, 5.2), constrained_layout=True)
        if x_values and y_values:
            bins_count = 50
            if hep:
                h, xedges, yedges = np.histogram2d(x_values, y_values, bins=bins_count)
                hep.hist2dplot(h, xedges, yedges, ax=axis, cmap="viridis")
                hep.cms.label("Simulation", data=False, loc=0, ax=axis, fontsize=11)
            else:
                axis.hist2d(x_values, y_values, bins=bins_count)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(x_column))
        axis.set_ylabel(axis_label(y_column))
        output = outdir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)
    written.extend(plot_truth(rows, outdir, hep, plt, np))
    written.extend(plot_truth_proxy(rows, outdir, hep, plt, np))
    written.extend(plot_hgamma_closure(rows, outdir, hep, plt, np))
    return written


def axis_label(column: str) -> str:
    return AXIS_LABELS.get(column, column)


def hep_plot_style(plt) -> None:
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


def plot_truth(rows, outdir, hep, plt, np) -> list[Path]:
    if not rows or not has_truth_columns(list(rows[0].keys())):
        return []
    written = []
    truth_values = [row.get("truth_matched", "0") for row in rows]
    matched = sum(1 for value in truth_values if value == "1")
    unmatched = len(truth_values) - matched
    figure, axis = plt.subplots(figsize=(5.0, 4.0), constrained_layout=True)
    axis.bar(["matched", "unmatched"], [matched, unmatched], color=["#2f6fbb", "#bbbbbb"])
    axis.set_ylabel("Candidates")
    axis.set_title("Truth match summary", fontsize=11)
    output = outdir / "truth_matched_fraction.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
    matched_h = [float(row["h_mass"]) for row in rows if row.get("truth_matched") == "1"]
    unmatched_h = [float(row["h_mass"]) for row in rows if row.get("truth_matched") != "1"]
    if matched_h:
        axis.hist(matched_h, bins=50, histtype="step", linewidth=1.5, label="matched")
    if unmatched_h:
        axis.hist(unmatched_h, bins=50, histtype="step", linewidth=1.5, label="unmatched")
    if matched_h or unmatched_h:
        axis.legend(frameon=False)
    axis.set_title("HToRhoGamma signal", fontsize=11)
    axis.set_xlabel(axis_label("h_mass"))
    axis.set_ylabel("Candidates / bin")
    output = outdir / "h_mass_truth_matched_vs_unmatched.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    fieldnames = list(rows[0].keys())
    for column in TRUTH_PLOT_COLUMNS:
        if column not in fieldnames:
            continue
        values = optional_numeric_values(rows, column)
        if not values:
            continue
        figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        axis.hist(values, bins=80, histtype="step", linewidth=1.5)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(column))
        axis.set_ylabel("Candidates / bin")
        output = outdir / f"{column}.png"
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)

    for x_column, y_column, filename in TRUTH_PLOT_2D_COLUMNS:
        if x_column not in fieldnames or y_column not in fieldnames:
            continue
        x_values = optional_numeric_values(rows, x_column)
        y_values = optional_numeric_values(rows, y_column)
        if not x_values or len(x_values) != len(y_values):
            continue
        figure, axis = plt.subplots(figsize=(6.0, 5.2), constrained_layout=True)
        axis.hist2d(x_values, y_values, bins=50)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(x_column))
        axis.set_ylabel(axis_label(y_column))
        output = outdir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)
    return written


def plot_truth_proxy(rows, outdir, hep, plt, np) -> list[Path]:
    if not rows or not has_truth_proxy_columns(list(rows[0].keys())):
        return []
    written = []
    thresholds = ["0p1", "0p2", "0p3"]
    counts = [truth_count(rows, f"truth_proxy_matched_dr_{suffix}", "1") for suffix in thresholds]
    figure, axis = plt.subplots(figsize=(5.2, 4.0), constrained_layout=True)
    axis.bar(["dR<0.1", "dR<0.2", "dR<0.3"], counts, color="#2f6fbb")
    axis.set_ylabel("Matched candidates")
    axis.set_title("Truth-proxy match thresholds", fontsize=11)
    output = outdir / "truth_proxy_match_thresholds.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    for column, filename in (
        ("h_mass", "h_mass_truth_proxy_matched_vs_unmatched.png"),
        ("rho_mass", "rho_mass_truth_proxy_matched_vs_unmatched.png"),
    ):
        matched = [
            float(row[column])
            for row in rows
            if row.get("truth_proxy_matched_dr_0p1") == "1"
        ]
        unmatched = [
            float(row[column])
            for row in rows
            if row.get("truth_proxy_matched_dr_0p1") != "1"
        ]
        figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        if matched:
            axis.hist(matched, bins=60, histtype="step", linewidth=1.5, label="matched dR<0.1")
        if unmatched:
            axis.hist(unmatched, bins=60, histtype="step", linewidth=1.5, label="unmatched")
        if matched or unmatched:
            axis.legend(frameon=False)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(column))
        axis.set_ylabel("Candidates / bin")
        output = outdir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)

    fieldnames = list(rows[0].keys())
    for column in TRUTH_PROXY_PLOT_COLUMNS:
        if column not in fieldnames:
            continue
        values = optional_numeric_values(rows, column)
        if not values:
            continue
        figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        axis.hist(values, bins=80, histtype="step", linewidth=1.5)
        if column.startswith("delta_r_"):
            for threshold in (0.1, 0.2, 0.3):
                axis.axvline(threshold, color="#777777", linestyle="--", linewidth=0.8)
        if "_over_" in column:
            axis.axvline(1.0, color="#777777", linestyle="--", linewidth=0.8)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(column))
        axis.set_ylabel("Candidates / bin")
        output = outdir / f"{column}.png"
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)

    for x_column, y_column, filename in TRUTH_PROXY_PLOT_2D_COLUMNS:
        if x_column not in fieldnames or y_column not in fieldnames:
            continue
        pairs = []
        for row in rows:
            x_raw = row.get(x_column, "")
            y_raw = row.get(y_column, "")
            if x_raw == "" or y_raw == "":
                continue
            pairs.append((float(x_raw), float(y_raw)))
        if not pairs:
            continue
        x_values, y_values = zip(*pairs)
        figure, axis = plt.subplots(figsize=(6.0, 5.2), constrained_layout=True)
        axis.hist2d(x_values, y_values, bins=50)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(x_column))
        axis.set_ylabel(axis_label(y_column))
        output = outdir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)
    return written


def plot_hgamma_closure(rows, outdir, hep, plt, np) -> list[Path]:
    if not rows or not has_hgamma_closure_columns(list(rows[0].keys())):
        return []
    written = []
    counts = [
        truth_count(rows, "hgamma_photon_matched_dr_0p1", "1"),
        truth_count(rows, "hgamma_higgs_closed_mass_10", "1"),
        truth_count(rows, "hgamma_higgs_closed_mass_15", "1"),
        truth_count(rows, "hgamma_higgs_closed_mass_20", "1"),
        truth_count(rows, "hgamma_closure_matched", "1"),
    ]
    labels = ["gamma dR<0.1", "|dmH|<10", "|dmH|<15", "|dmH|<20", "matched"]
    figure, axis = plt.subplots(figsize=(6.4, 4.2), constrained_layout=True)
    axis.bar(labels, counts, color="#2f6fbb")
    axis.tick_params(axis="x", labelrotation=25)
    axis.set_ylabel("Candidates")
    axis.set_title("Photon-anchored Higgs closure", fontsize=11)
    output = outdir / "hgamma_higgs_closure_thresholds.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    fieldnames = list(rows[0].keys())
    for column in HGAMMA_CLOSURE_PLOT_COLUMNS:
        if column not in fieldnames:
            continue
        values = optional_numeric_values(rows, column)
        if not values:
            continue
        figure, axis = plt.subplots(figsize=(6.0, 5.0), constrained_layout=True)
        axis.hist(values, bins=80, histtype="step", linewidth=1.5)
        if column == "delta_r_reco_photon_gen_photon":
            for threshold in (0.1, 0.2):
                axis.axvline(threshold, color="#777777", linestyle="--", linewidth=0.8)
            output = outdir / "hgamma_photon_match_dr.png"
        elif column in {"reco_photon_pt_over_gen_photon_pt", "reco_h_pt_over_gen_h_pt", "reco_rho_pt_over_gen_rho_recoil_pt"}:
            axis.axvline(1.0, color="#777777", linestyle="--", linewidth=0.8)
            output = outdir / f"hgamma_{column}.png"
        elif column == "reco_h_mass_minus_gen_h_mass":
            for threshold in (-20.0, -15.0, -10.0, 10.0, 15.0, 20.0):
                axis.axvline(threshold, color="#777777", linestyle="--", linewidth=0.6)
            output = outdir / f"hgamma_{column}.png"
        elif column.startswith("hgamma_"):
            output = outdir / f"{column}.png"
        else:
            output = outdir / f"hgamma_{column}.png"
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(column))
        axis.set_ylabel("Candidates / bin")
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)

    for x_column, y_column, filename in HGAMMA_CLOSURE_PLOT_2D_COLUMNS:
        if x_column not in fieldnames or y_column not in fieldnames:
            continue
        pairs = []
        for row in rows:
            x_raw = row.get(x_column, "")
            y_raw = row.get(y_column, "")
            if x_raw == "" or y_raw == "":
                continue
            pairs.append((float(x_raw), float(y_raw)))
        if not pairs:
            continue
        x_values = [pair[0] for pair in pairs]
        y_values = [pair[1] for pair in pairs]
        figure, axis = plt.subplots(figsize=(6.0, 5.2), constrained_layout=True)
        axis.hist2d(x_values, y_values, bins=50)
        axis.set_title("HToRhoGamma signal", fontsize=11)
        axis.set_xlabel(axis_label(x_column))
        axis.set_ylabel(axis_label(y_column))
        output = outdir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
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
                plot_status = f"wrote {len(written_plots)} PNG files and {len(written_plots)} PDF files"
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
