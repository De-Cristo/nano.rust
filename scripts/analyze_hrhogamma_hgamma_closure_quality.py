#!/usr/bin/env python3
"""Analyze Hgamma-closure candidate quality from Stage-16A candidate CSV output."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hrhogamma_plotting_policy as plotting_policy


EVENT_COLUMNS = ("run", "luminosityBlock", "event")
HGAMMA_REQUIRED_COLUMNS = (
    "truth_strategy",
    "hgamma_closure_available",
    "hgamma_closure_matched",
    "hgamma_photon_matched_dr_0p1",
    "hgamma_higgs_closed_mass_15",
)
SUMMARY_VARIABLES = (
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
    "reco_h_mass_minus_gen_h_mass",
    "reco_h_pt_over_gen_h_pt",
    "delta_r_reco_h_gen_h",
    "reco_rho_mass_minus_gen_rho_recoil_mass",
    "reco_rho_pt_over_gen_rho_recoil_pt",
    "delta_r_reco_rho_gen_rho_recoil",
    "reco_photon_pt_over_gen_photon_pt",
    "delta_r_reco_photon_gen_photon",
)
OVERLAY_VARIABLES = (
    "h_mass",
    "rho_mass",
    "photon_pt",
    "rho_pt",
    "h_pt",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "rho_pt_over_photon_pt",
    "reco_h_mass_minus_gen_h_mass",
    "delta_r_reco_rho_gen_rho_recoil",
)
CATEGORY_SUMMARY_VARIABLES = (
    "h_mass",
    "rho_mass",
    "rho_pt",
    "rho_pt_over_photon_pt",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "photon_pt",
    "pi_plus_pt",
    "pi_minus_pt",
)
CATEGORY_OVERLAY_VARIABLES = (
    "h_mass",
    "rho_mass",
    "rho_pt",
    "delta_r_pipi",
)
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
    "reco_h_pt_over_gen_h_pt": "H pT(reco) / H pT(gen)",
    "delta_r_reco_h_gen_h": "DeltaR(reco H, gen H)",
    "reco_rho_mass_minus_gen_rho_recoil_mass": "mrho(reco) - m(H-gamma recoil) [GeV]",
    "reco_rho_pt_over_gen_rho_recoil_pt": "rho pT(reco) / recoil pT(gen)",
    "delta_r_reco_rho_gen_rho_recoil": "DeltaR(reco rho, H-gamma recoil)",
    "reco_photon_pt_over_gen_photon_pt": "photon pT(reco) / photon pT(gen)",
    "delta_r_reco_photon_gen_photon": "DeltaR(reco gamma, gen gamma)",
}
RHO_MASS_TARGET = 0.775
HIGGS_MASS_TARGET = 125.0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Study hgamma-closure candidate quality from a Stage-16A candidate CSV."
    )
    parser.add_argument("--candidate-csv", type=Path, required=True)
    parser.add_argument("--summary-json", type=Path, default=None)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--max-rows", type=int, default=None)
    parser.add_argument("--no-plots", action="store_true")
    parser.add_argument(
        "--plot-config",
        type=Path,
        default=None,
        help="optional TOML plotting policy with named plot profiles",
    )
    parser.add_argument(
        "--plot-profile",
        default=None,
        help="named plot profile to use, e.g. physics_focus or signal_window",
    )
    parser.add_argument(
        "--write-full-range-sanity",
        action="store_true",
        help="also write full_range_sanity plots and range accounting",
    )
    parser.add_argument(
        "--quality-config",
        type=Path,
        default=None,
        help="optional TOML candidate-quality category definitions",
    )
    return parser.parse_args()


def read_rows(path: Path, max_rows: int | None = None) -> tuple[list[dict[str, str]], list[str]]:
    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"empty CSV or missing header: {path}")
        rows = []
        for row in reader:
            rows.append(row)
            if max_rows is not None and len(rows) >= max_rows:
                break
        return rows, list(reader.fieldnames)


def require_hgamma_columns(fieldnames: list[str]) -> None:
    missing = [
        column
        for column in (*EVENT_COLUMNS, *HGAMMA_REQUIRED_COLUMNS, *SUMMARY_VARIABLES)
        if column not in fieldnames
    ]
    if missing:
        raise ValueError(f"missing required hgamma quality columns: {', '.join(missing)}")


def event_key(row: dict[str, str]) -> tuple[str, str, str]:
    return tuple(row[column] for column in EVENT_COLUMNS)


def truthy(row: dict[str, str], column: str) -> bool:
    return row.get(column, "").lower() in {"1", "true", "yes"}


def is_hgamma_closed(row: dict[str, str]) -> bool:
    return truthy(row, "hgamma_closure_matched")


def optional_float(raw: str | None) -> float | None:
    if raw is None or raw == "":
        return None
    return float(raw)


def row_float(row: dict[str, str], column: str) -> float:
    value = optional_float(row.get(column, ""))
    if value is None:
        raise ValueError(f"missing required numeric column {column}")
    return value


def optional_values(rows: list[dict[str, str]], column: str) -> list[float]:
    values = []
    for row in rows:
        value = optional_float(row.get(column, ""))
        if value is not None and math.isfinite(value):
            values.append(value)
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
        "std": statistics.pstdev(values) if len(values) > 1 else 0.0,
        "median": statistics.median(values),
        "p16": percentile(ordered, 0.16),
        "p50": percentile(ordered, 0.50),
        "p84": percentile(ordered, 0.84),
        "max": max(values),
    }


def fraction(numerator: int, denominator: int) -> str:
    if denominator <= 0:
        return "unavailable"
    return f"{numerator / denominator:.6f}"


def fraction_value(numerator: int, denominator: int) -> float | None:
    if denominator <= 0:
        return None
    return numerator / denominator


def categorize_rows(rows: list[dict[str, str]]) -> dict[str, list[dict[str, str]]]:
    photon = [row for row in rows if truthy(row, "hgamma_photon_matched_dr_0p1")]
    closed = [row for row in rows if is_hgamma_closed(row)]
    photon_not_closed = [
        row
        for row in rows
        if truthy(row, "hgamma_photon_matched_dr_0p1")
        and not is_hgamma_closed(row)
    ]
    not_photon = [row for row in rows if not truthy(row, "hgamma_photon_matched_dr_0p1")]
    return {
        "all_candidates": rows,
        "photon_matched_dr0p1": photon,
        "hgamma_closed_mass15": closed,
        "photon_matched_but_not_closed_mass15": photon_not_closed,
        "not_photon_matched_dr0p1": not_photon,
    }


def variable_summaries(
    categories: dict[str, list[dict[str, str]]],
) -> dict[str, dict[str, dict[str, float | int] | None]]:
    return {
        category: {
            variable: summarize(optional_values(rows, variable))
            for variable in SUMMARY_VARIABLES
        }
        for category, rows in categories.items()
    }


def separation_summaries(
    categories: dict[str, list[dict[str, str]]],
) -> list[dict[str, float | str | None]]:
    closed = categories["hgamma_closed_mass15"]
    nonclosed = categories["photon_matched_but_not_closed_mass15"]
    rows = []
    for variable in SUMMARY_VARIABLES:
        closed_summary = summarize(optional_values(closed, variable))
        nonclosed_summary = summarize(optional_values(nonclosed, variable))
        if closed_summary is None or nonclosed_summary is None:
            rows.append(
                {
                    "variable": variable,
                    "mean_closed_minus_nonclosed": None,
                    "median_closed_minus_nonclosed": None,
                    "abs_mean_difference_over_pooled_std": None,
                }
            )
            continue
        mean_delta = float(closed_summary["mean"]) - float(nonclosed_summary["mean"])
        median_delta = float(closed_summary["median"]) - float(nonclosed_summary["median"])
        pooled_std = math.sqrt(
            (float(closed_summary["std"]) ** 2 + float(nonclosed_summary["std"]) ** 2)
            / 2.0
        )
        rows.append(
            {
                "variable": variable,
                "mean_closed_minus_nonclosed": mean_delta,
                "median_closed_minus_nonclosed": median_delta,
                "abs_mean_difference_over_pooled_std": (
                    abs(mean_delta) / pooled_std if pooled_std > 0.0 else None
                ),
            }
        )
    return sorted(
        rows,
        key=lambda row: (
            row["abs_mean_difference_over_pooled_std"] is None,
            -(row["abs_mean_difference_over_pooled_std"] or 0.0),
        ),
    )


def threshold_row(
    label: str,
    photon_rows: list[dict[str, str]],
    predicate,
    closed_total: int,
) -> dict[str, float | int | str | None]:
    passed = [row for row in photon_rows if predicate(row)]
    closed_pass = [row for row in passed if is_hgamma_closed(row)]
    n_total = len(photon_rows)
    n_pass = len(passed)
    n_closed = len(closed_pass)
    closure_fraction = fraction_value(n_closed, n_pass)
    retention = fraction_value(n_pass, n_total)
    return {
        "label": label,
        "n_total_photon_matched": n_total,
        "n_pass": n_pass,
        "n_closed_pass": n_closed,
        "closure_fraction_after_cut": closure_fraction,
        "closed_efficiency_relative_to_all_closed": fraction_value(n_closed, closed_total),
        "candidate_retention": retention,
        "simple_score": (
            closure_fraction * math.sqrt(retention)
            if closure_fraction is not None and retention is not None
            else None
        ),
    }


def threshold_scans(rows: list[dict[str, str]]) -> dict[str, list[dict[str, float | int | str | None]]]:
    photon_rows = [row for row in rows if truthy(row, "hgamma_photon_matched_dr_0p1")]
    closed_total = sum(1 for row in photon_rows if is_hgamma_closed(row))

    scans: dict[str, list[dict[str, float | int | str | None]]] = {}
    scans["rho_mass_window"] = [
        threshold_row(
            f"|rho_mass-0.775|<{half_width:.2f}",
            photon_rows,
            lambda row, half_width=half_width: abs(row_float(row, "rho_mass") - RHO_MASS_TARGET)
            < half_width,
            closed_total,
        )
        for half_width in (0.10, 0.15, 0.20, 0.25, 0.30, 0.40)
    ]
    scans["h_mass_window"] = [
        threshold_row(
            f"|h_mass-125|<{half_width:.0f}",
            photon_rows,
            lambda row, half_width=half_width: abs(row_float(row, "h_mass") - HIGGS_MASS_TARGET)
            < half_width,
            closed_total,
        )
        for half_width in (10.0, 15.0, 20.0, 25.0, 30.0)
    ]
    scans["delta_r_pipi"] = [
        threshold_row(
            f"delta_r_pipi<{threshold:.2f}",
            photon_rows,
            lambda row, threshold=threshold: row_float(row, "delta_r_pipi") < threshold,
            closed_total,
        )
        for threshold in (0.03, 0.05, 0.07, 0.10, 0.15, 0.20)
    ]
    scans["delta_r_gamma_rho"] = [
        threshold_row(
            f"{low:.1f}<delta_r_gamma_rho<{high:.1f}",
            photon_rows,
            lambda row, low=low, high=high: low < row_float(row, "delta_r_gamma_rho") < high,
            closed_total,
        )
        for low, high in ((1.0, 3.0), (1.0, 4.0), (1.0, 5.0), (1.5, 5.0), (2.0, 5.0))
    ]
    scans["rho_pt_over_photon_pt"] = [
        threshold_row(
            f"{low:.1f}<rho_pt_over_photon_pt<{high:.1f}",
            photon_rows,
            lambda row, low=low, high=high: low
            < row_float(row, "rho_pt_over_photon_pt")
            < high,
            closed_total,
        )
        for low, high in ((0.2, 2.0), (0.4, 1.6), (0.5, 1.5), (0.7, 1.3))
    ]
    scans["photon_pt"] = [
        threshold_row(
            f"photon_pt>{threshold:.0f}",
            photon_rows,
            lambda row, threshold=threshold: row_float(row, "photon_pt") > threshold,
            closed_total,
        )
        for threshold in (15.0, 20.0, 30.0, 40.0, 50.0)
    ]
    scans["rho_pt"] = [
        threshold_row(
            f"rho_pt>{threshold:.0f}",
            photon_rows,
            lambda row, threshold=threshold: row_float(row, "rho_pt") > threshold,
            closed_total,
        )
        for threshold in (10.0, 20.0, 30.0, 40.0, 50.0)
    ]
    return scans


def group_by_event(rows: list[dict[str, str]]) -> dict[tuple[str, str, str], list[dict[str, str]]]:
    grouped: dict[tuple[str, str, str], list[dict[str, str]]] = {}
    for row in rows:
        grouped.setdefault(event_key(row), []).append(row)
    return grouped


def multiplicity_bucket(count: int) -> str:
    return ">=4" if count >= 4 else str(count)


def candidate_multiplicity(rows: list[dict[str, str]]) -> dict[str, object]:
    grouped = group_by_event(rows)
    counts: dict[str, int] = {}
    closure_by_bucket: dict[str, dict[str, int | float | None]] = {}
    for candidates in grouped.values():
        bucket = multiplicity_bucket(len(candidates))
        counts[bucket] = counts.get(bucket, 0) + 1
        entry = closure_by_bucket.setdefault(bucket, {"events": 0, "closed_events": 0})
        entry["events"] = int(entry["events"]) + 1
        if any(is_hgamma_closed(row) for row in candidates):
            entry["closed_events"] = int(entry["closed_events"]) + 1
    for bucket, entry in closure_by_bucket.items():
        entry["closed_event_fraction"] = fraction_value(
            int(entry["closed_events"]), int(entry["events"])
        )
    return {
        "unique_events": len(grouped),
        "multiplicity_counts": dict(sorted(counts.items(), key=lambda item: item[0])),
        "multiplicity_fractions": {
            bucket: fraction_value(count, len(grouped)) for bucket, count in counts.items()
        },
        "closure_by_multiplicity": closure_by_bucket,
    }


def best_candidate_policies(rows: list[dict[str, str]]) -> dict[str, dict[str, float | int | None]]:
    grouped = group_by_event(rows)
    policies = {
        "best_by_abs_h_mass_minus_125": lambda row: abs(row_float(row, "h_mass") - HIGGS_MASS_TARGET),
        "best_by_abs_rho_mass_minus_0p775": lambda row: abs(row_float(row, "rho_mass") - RHO_MASS_TARGET),
        "best_by_highest_rho_pt": lambda row: -row_float(row, "rho_pt"),
        "best_by_highest_photon_pt": lambda row: -row_float(row, "photon_pt"),
        "best_by_smallest_delta_r_pipi": lambda row: row_float(row, "delta_r_pipi"),
        "best_by_rho_pt_over_photon_pt_closest_to_1": lambda row: abs(
            row_float(row, "rho_pt_over_photon_pt") - 1.0
        ),
    }
    results: dict[str, dict[str, float | int | None]] = {}
    for name, score in policies.items():
        selected = []
        for candidates in grouped.values():
            indexed = list(enumerate(candidates))
            _, row = min(indexed, key=lambda item: (score(item[1]), item[0]))
            selected.append(row)
        closed_count = sum(1 for row in selected if is_hgamma_closed(row))
        h_mass = optional_values(selected, "h_mass")
        rho_mass = optional_values(selected, "rho_mass")
        results[name] = {
            "selected_events": len(selected),
            "closed_count": closed_count,
            "closed_fraction": fraction_value(closed_count, len(selected)),
            "mean_h_mass": statistics.mean(h_mass) if h_mass else None,
            "median_h_mass": statistics.median(h_mass) if h_mass else None,
            "mean_rho_mass": statistics.mean(rho_mass) if rho_mass else None,
            "median_rho_mass": statistics.median(rho_mass) if rho_mass else None,
        }
    return results


def summarize_quality_categories(
    rows: list[dict[str, str]],
    quality_categories: plotting_policy.QualityCategories,
) -> dict[str, object]:
    inclusive_count = len(rows)
    inclusive_closed = sum(1 for row in rows if is_hgamma_closed(row))
    category_summaries: dict[str, dict[str, object]] = {}
    for category in quality_categories.categories:
        selected = [
            row for row in rows if plotting_policy.category_passes(row, category)
        ]
        photon = [row for row in selected if truthy(row, "hgamma_photon_matched_dr_0p1")]
        closed = [row for row in selected if is_hgamma_closed(row)]
        category_summaries[category.name] = {
            "label": category.label,
            "requirements": [
                {
                    "variable": requirement.variable,
                    "op": requirement.op,
                    "value": requirement.value,
                }
                for requirement in category.requirements
            ],
            "n_candidates": len(selected),
            "n_photon_matched": len(photon),
            "n_hgamma_closed": len(closed),
            "closed_fraction_all_candidates": fraction_value(len(closed), len(selected)),
            "closed_fraction_photon_matched": fraction_value(len(closed), len(photon)),
            "candidate_retention_relative_to_inclusive": fraction_value(
                len(selected), inclusive_count
            ),
            "closed_candidate_retention_relative_to_inclusive_closed": fraction_value(
                len(closed), inclusive_closed
            ),
            "variable_summaries": {
                variable: summarize(optional_values(selected, variable))
                for variable in CATEGORY_SUMMARY_VARIABLES
            },
        }
    def best_category(excluded: set[str]) -> str | None:
        strongest = None
        for name, summary in category_summaries.items():
            if name in excluded:
                continue
            fraction_closed = summary["closed_fraction_all_candidates"]
            if fraction_closed is None:
                continue
            if strongest is None or fraction_closed > category_summaries[strongest][
                "closed_fraction_all_candidates"
            ]:
                strongest = name
        return strongest

    strongest = best_category({"inclusive"})
    strongest_reco = best_category({"inclusive", "photon_matched_reference"})
    return {
        "warnings": list(quality_categories.warnings),
        "categories": category_summaries,
        "strongest_closure_enriched_category": strongest,
        "strongest_reco_quality_category": strongest_reco,
    }


def category_rows_by_name(
    rows: list[dict[str, str]],
    quality_categories: plotting_policy.QualityCategories,
) -> dict[str, list[dict[str, str]]]:
    return {
        category.name: [
            row for row in rows if plotting_policy.category_passes(row, category)
        ]
        for category in quality_categories.categories
    }


def compute_range_coverage(
    rows: list[dict[str, str]],
    plot_policy: plotting_policy.PlottingPolicy,
) -> list[dict[str, object]]:
    variables = sorted(set((*OVERLAY_VARIABLES, *CATEGORY_OVERLAY_VARIABLES)))
    coverage_rows: list[dict[str, object]] = []
    for profile in plot_policy.output_profiles():
        for variable in variables:
            values = optional_values(rows, variable)
            spec = plot_policy.spec_for(variable, profile)
            coverage_rows.append(
                plotting_policy.range_coverage(variable, profile, spec, values)
            )
    return coverage_rows


def range_coverage_warnings(
    coverage_rows: list[dict[str, object]],
    threshold: float,
) -> list[str]:
    warnings = []
    for row in coverage_rows:
        excluded = row.get("excluded_fraction")
        if excluded is None:
            continue
        if float(excluded) > threshold:
            xmin = row.get("xmin")
            xmax = row.get("xmax")
            if xmin is None or xmax is None:
                continue
            warnings.append(
                f"WARNING: {row['variable']} {row['profile']} excludes "
                f"{100.0 * float(excluded):.1f}% of candidates outside [{xmin}, {xmax}]."
            )
    return warnings


def write_range_coverage(
    outdir: Path,
    plot_policy: plotting_policy.PlottingPolicy,
    coverage_rows: list[dict[str, object]],
) -> tuple[Path, Path]:
    json_path = outdir / "range_coverage_summary.json"
    md_path = outdir / "range_coverage_summary.md"
    warnings = range_coverage_warnings(
        coverage_rows, plot_policy.coverage_warning_threshold
    )
    json_path.write_text(
        json.dumps(
            {
                "selected_profile": plot_policy.selected_profile,
                "output_profiles": plot_policy.output_profiles(),
                "coverage_warning_threshold": plot_policy.coverage_warning_threshold,
                "range_coverage": coverage_rows,
                "warnings": warnings,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    lines = [
        "# Hgamma Plot Range Coverage",
        "",
        f"selected profile: `{plot_policy.selected_profile}`",
        f"output profiles: `{', '.join(plot_policy.output_profiles())}`",
        f"coverage warning threshold: `{plot_policy.coverage_warning_threshold:.3f}`",
        "",
        "| profile | variable | range | bins | total | inside | underflow | overflow | inside fraction |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in coverage_rows:
        xmin = row.get("xmin")
        xmax = row.get("xmax")
        plot_range = "auto" if xmin is None or xmax is None else f"[{xmin}, {xmax}]"
        lines.append(
            f"| {row['profile']} | {row['variable']} | {plot_range} | {row['bins']} | "
            f"{row['n_total']} | {row['n_inside']} | {row['n_underflow']} | "
            f"{row['n_overflow']} | {fmt(row['frac_inside'])} |"
        )
    if warnings:
        lines.extend(["", "## Warnings", ""])
        lines.extend(f"- {warning}" for warning in warnings)
    md_path.write_text("\n".join(lines) + "\n")
    return md_path, json_path


def write_quality_category_summary(
    outdir: Path,
    category_summary: dict[str, object],
    quality_config: Path | None,
    plot_status: str,
    plots: list[Path],
) -> tuple[Path, Path]:
    json_path = outdir / "hgamma_quality_categories_summary.json"
    md_path = outdir / "hgamma_quality_categories_summary.md"
    json_path.write_text(json.dumps(category_summary, indent=2, sort_keys=True) + "\n")
    categories = category_summary["categories"]
    lines = [
        "# Hgamma Quality Category Summary",
        "",
        f"quality config: `{quality_config if quality_config is not None else 'built-in inclusive fallback'}`",
        f"plots: `{plot_status}`",
        f"strongest closure-enriched category: `{category_summary['strongest_closure_enriched_category']}`",
        f"strongest reco-quality category: `{category_summary['strongest_reco_quality_category']}`",
        "",
        "These categories are diagnostic working regions, not optimized physics selections.",
        "",
    ]
    warnings = category_summary.get("warnings", [])
    if warnings:
        lines.extend(["## Warnings", ""])
        lines.extend(f"- {warning}" for warning in warnings)
        lines.append("")
    lines.extend(
        [
            "## Category Counts",
            "",
            "| category | label | candidates | photon matched | hgamma closed | closed fraction | photon-matched closed fraction | candidate retention | closed retention |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name, summary in categories.items():
        lines.append(
            f"| {name} | {summary['label']} | {summary['n_candidates']} | "
            f"{summary['n_photon_matched']} | {summary['n_hgamma_closed']} | "
            f"{fmt(summary['closed_fraction_all_candidates'])} | "
            f"{fmt(summary['closed_fraction_photon_matched'])} | "
            f"{fmt(summary['candidate_retention_relative_to_inclusive'])} | "
            f"{fmt(summary['closed_candidate_retention_relative_to_inclusive_closed'])} |"
        )
    lines.extend(["", "## Variable Summaries", ""])
    for name, summary in categories.items():
        lines.extend([f"### {name}", ""])
        lines.extend(category_stat_table(summary["variable_summaries"]))
        lines.append("")
    lines.extend(["## Plot Index", ""])
    if plots:
        for plot in plots:
            if plot.name.startswith("quality_category_") and plot.suffix == ".png":
                lines.append(f"- [{plot.stem}](plots/{plot.parent.name}/{plot.name})")
    else:
        lines.append(f"- No plots produced: {plot_status}")
    md_path.write_text("\n".join(lines) + "\n")
    return md_path, json_path


def fmt(value: object) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.6f}"
    return str(value)


def stat_table(summary: dict[str, dict[str, float | int] | None]) -> list[str]:
    lines = [
        "| variable | count | min | mean | std | median | p16 | p50 | p84 | max |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for variable in SUMMARY_VARIABLES:
        stats = summary.get(variable)
        if stats is None:
            lines.append(f"| {variable} | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |")
        else:
            lines.append(
                f"| {variable} | {stats['count']} | {fmt(stats['min'])} | {fmt(stats['mean'])} | "
                f"{fmt(stats['std'])} | {fmt(stats['median'])} | {fmt(stats['p16'])} | "
                f"{fmt(stats['p50'])} | {fmt(stats['p84'])} | {fmt(stats['max'])} |"
            )
    return lines


def category_stat_table(summary: dict[str, dict[str, float | int] | None]) -> list[str]:
    lines = [
        "| variable | count | min | mean | std | median | p16 | p50 | p84 | max |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for variable in CATEGORY_SUMMARY_VARIABLES:
        stats = summary.get(variable)
        if stats is None:
            lines.append(f"| {variable} | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |")
        else:
            lines.append(
                f"| {variable} | {stats['count']} | {fmt(stats['min'])} | {fmt(stats['mean'])} | "
                f"{fmt(stats['std'])} | {fmt(stats['median'])} | {fmt(stats['p16'])} | "
                f"{fmt(stats['p50'])} | {fmt(stats['p84'])} | {fmt(stats['max'])} |"
            )
    return lines


def scan_table(rows: list[dict[str, object]]) -> list[str]:
    lines = [
        "| threshold | n photon matched | n pass | n closed pass | closure fraction | closed efficiency | retention | score |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        lines.append(
            f"| {row['label']} | {row['n_total_photon_matched']} | {row['n_pass']} | "
            f"{row['n_closed_pass']} | {fmt(row['closure_fraction_after_cut'])} | "
            f"{fmt(row['closed_efficiency_relative_to_all_closed'])} | "
            f"{fmt(row['candidate_retention'])} | {fmt(row['simple_score'])} |"
        )
    return lines


def recommendation(separations: list[dict[str, object]], multiplicity: dict[str, object]) -> str:
    best = next(
        (
            row
            for row in separations
            if row.get("abs_mean_difference_over_pooled_std") is not None
        ),
        None,
    )
    multi_counts = multiplicity.get("multiplicity_counts", {})
    multi_event_count = sum(
        count for bucket, count in multi_counts.items() if bucket != "1"
    ) if isinstance(multi_counts, dict) else 0
    unique_events = int(multiplicity.get("unique_events", 0))
    multi_fraction = multi_event_count / unique_events if unique_events else 0.0
    if best and float(best["abs_mean_difference_over_pooled_std"]) >= 1.0:
        return (
            "A. Closure has clear separation in simple reco variables. "
            "Next: define candidate-quality categories or diagnostic cuts for Stage 16C."
        )
    if multi_fraction >= 0.25:
        return (
            "B. Candidate multiplicity is a major diagnostic handle. "
            "Next: improve best-candidate selection or add candidate ranking in Stage 16C."
        )
    return (
        "C. Closure is not strongly separated by simple variables in this diagnostic. "
        "Next: move toward background/reco-only comparison and avoid truth-driven tuning."
    )


def write_markdown(
    path: Path,
    candidate_csv: Path,
    summary_json: Path | None,
    rows: list[dict[str, str]],
    categories: dict[str, list[dict[str, str]]],
    summaries: dict[str, dict[str, dict[str, float | int] | None]],
    separations: list[dict[str, object]],
    scans: dict[str, list[dict[str, object]]],
    multiplicity: dict[str, object],
    policies: dict[str, dict[str, object]],
    plot_status: str,
    plots: list[Path],
    recommendation_text: str,
    plot_policy: plotting_policy.PlottingPolicy,
    range_coverage_md: Path,
    quality_categories_md: Path,
) -> None:
    total = len(rows)
    photon = len(categories["photon_matched_dr0p1"])
    closed = len(categories["hgamma_closed_mass15"])
    lines = [
        "# Hgamma Closure Candidate-Quality Study",
        "",
        f"Stage 16A candidate CSV: `{candidate_csv}`",
        f"Stage 16A summary JSON: `{summary_json if summary_json is not None else 'not supplied'}`",
        f"candidates read: `{total}`",
        f"photon-matched candidates: `{photon}`",
        f"hgamma-closed candidates: `{closed}` / `{total}` (`{fraction(closed, total)}`)",
        f"hgamma-closed among photon-matched: `{closed}` / `{photon}` (`{fraction(closed, photon)}`)",
        "closed label: `hgamma_closure_matched` from Stage 16A",
        f"plot profile: `{plot_policy.selected_profile}`",
        f"plot output profiles: `{', '.join(plot_policy.output_profiles())}`",
        f"plots: `{plot_status}`",
        f"range coverage summary: `{range_coverage_md.name}`",
        f"quality category summary: `{quality_categories_md.name}`",
        "",
        "## Category Counts",
        "",
        "| category | candidates | fraction of all |",
        "| --- | ---: | ---: |",
    ]
    for name, category_rows in categories.items():
        lines.append(f"| {name} | {len(category_rows)} | {fraction(len(category_rows), total)} |")
    lines.extend(["", "## Variable Summaries By Category", ""])
    for category in (
        "all_candidates",
        "photon_matched_dr0p1",
        "hgamma_closed_mass15",
        "photon_matched_but_not_closed_mass15",
        "not_photon_matched_dr0p1",
    ):
        lines.extend([f"### {category}", "", *stat_table(summaries[category]), ""])
    lines.extend(
        [
            "## Ranked Closed Vs Non-Closed Separation",
            "",
            "Primary comparison: `hgamma_closed_mass15` vs `photon_matched_but_not_closed_mass15`.",
            "",
            "| variable | mean closed - nonclosed | median closed - nonclosed | |mean diff| / pooled std |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for row in separations:
        lines.append(
            f"| {row['variable']} | {fmt(row['mean_closed_minus_nonclosed'])} | "
            f"{fmt(row['median_closed_minus_nonclosed'])} | "
            f"{fmt(row['abs_mean_difference_over_pooled_std'])} |"
        )
    lines.extend(["", "## Threshold Scans", ""])
    for name, scan_rows in scans.items():
        lines.extend([f"### {name}", "", *scan_table(scan_rows), ""])
    lines.extend(
        [
            "## Candidate Multiplicity",
            "",
            f"unique events with accepted candidates: `{multiplicity['unique_events']}`",
            "",
            "| multiplicity | events | fraction | closed-event fraction |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    counts = multiplicity["multiplicity_counts"]
    fractions = multiplicity["multiplicity_fractions"]
    closure = multiplicity["closure_by_multiplicity"]
    for bucket in sorted(counts, key=lambda item: 4 if item == ">=4" else int(item)):
        lines.append(
            f"| {bucket} | {counts[bucket]} | {fmt(fractions[bucket])} | "
            f"{fmt(closure[bucket]['closed_event_fraction'])} |"
        )
    lines.extend(
        [
            "",
            "## Best-Candidate Policy Diagnostics",
            "",
            "| policy | selected events | hgamma closed | closed fraction | mean h_mass | median h_mass | mean rho_mass | median rho_mass |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for name, row in policies.items():
        lines.append(
            f"| {name} | {row['selected_events']} | {row['closed_count']} | "
            f"{fmt(row['closed_fraction'])} | {fmt(row['mean_h_mass'])} | "
            f"{fmt(row['median_h_mass'])} | {fmt(row['mean_rho_mass'])} | "
            f"{fmt(row['median_rho_mass'])} |"
        )
    lines.extend(["", "## Plot Index", ""])
    if plots:
        lines.extend(
            f"- [{plot.stem}](plots/{plot.parent.name}/{plot.name})"
            for plot in plots
            if plot.suffix == ".png"
        )
    else:
        lines.append(f"- No plots produced: {plot_status}")
    lines.extend(
        [
            "",
            "## Recommendation For Stage 16C/16D",
            "",
            recommendation_text,
            "",
            "## Interpretation Notes",
            "",
            "- This is a diagnostic candidate-quality study over accepted candidates.",
            "- Threshold scans are not final optimized cuts.",
            "- Closure labels use Stage 16A photon-anchored Higgs closure and do not rely on pion ancestry.",
            "- Do not replace the production candidate-building policy from these numbers alone.",
            "",
        ]
    )
    path.write_text("\n".join(lines))


def write_json(
    path: Path,
    candidate_csv: Path,
    summary_json: Path | None,
    stage16a_summary: dict[str, object] | None,
    categories: dict[str, list[dict[str, str]]],
    summaries: dict[str, dict[str, dict[str, float | int] | None]],
    separations: list[dict[str, object]],
    scans: dict[str, list[dict[str, object]]],
    multiplicity: dict[str, object],
    policies: dict[str, dict[str, object]],
    plot_status: str,
    plots: list[Path],
    recommendation_text: str,
    plot_policy: plotting_policy.PlottingPolicy,
    range_coverage_path: Path,
    quality_categories_path: Path,
) -> None:
    payload = {
        "candidate_csv": str(candidate_csv),
        "summary_json": str(summary_json) if summary_json is not None else None,
        "stage16a_summary": stage16a_summary,
        "category_counts": {name: len(rows) for name, rows in categories.items()},
        "variable_summaries": summaries,
        "separation_summary": separations,
        "threshold_scans": scans,
        "candidate_multiplicity": multiplicity,
        "best_candidate_policies": policies,
        "plot_status": plot_status,
        "plots": [str(path) for path in plots],
        "recommendation": recommendation_text,
        "plot_policy": {
            "selected_profile": plot_policy.selected_profile,
            "output_profiles": plot_policy.output_profiles(),
            "write_range_coverage": plot_policy.write_range_coverage,
        },
        "range_coverage_summary": str(range_coverage_path),
        "quality_categories_summary": str(quality_categories_path),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def load_summary_json(path: Path | None) -> dict[str, object] | None:
    if path is None:
        return None
    with path.open() as handle:
        return json.load(handle)


def plot_all(
    outdir: Path,
    categories: dict[str, list[dict[str, str]]],
    scans: dict[str, list[dict[str, object]]],
    multiplicity: dict[str, object],
    policies: dict[str, dict[str, object]],
    plot_policy: plotting_policy.PlottingPolicy,
    quality_rows: dict[str, list[dict[str, str]]],
    category_summary: dict[str, object],
) -> tuple[str, list[Path]]:
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        return "skipped (matplotlib unavailable)", []

    plots_root = outdir / "plots"
    plots_root.mkdir(parents=True, exist_ok=True)
    plt.rcParams.update(
        {
            "font.size": 9,
            "axes.titlesize": 10,
            "axes.labelsize": 9,
            "xtick.labelsize": 8,
            "ytick.labelsize": 8,
            "legend.fontsize": 8,
            "figure.dpi": 120,
            "savefig.bbox": "tight",
        }
    )
    written: list[Path] = []
    closed_rows = categories["hgamma_closed_mass15"]
    nonclosed_rows = categories["photon_matched_but_not_closed_mass15"]
    for profile in plot_policy.output_profiles():
        plots_dir = plots_root / profile
        plots_dir.mkdir(parents=True, exist_ok=True)
        for variable in OVERLAY_VARIABLES:
            spec = plot_policy.spec_for(variable, profile)
            hist_kwargs = {"bins": spec.bins, "histtype": "step", "linewidth": 1.5}
            if spec.range is not None:
                hist_kwargs["range"] = spec.range
            closed_values = optional_values(closed_rows, variable)
            nonclosed_values = optional_values(nonclosed_rows, variable)
            figure, axis = plt.subplots(figsize=(6.0, 4.5), constrained_layout=True)
            if closed_values:
                axis.hist(closed_values, label="closed mass15", **hist_kwargs)
            if nonclosed_values:
                axis.hist(
                    nonclosed_values,
                    label="photon-matched non-closed",
                    **hist_kwargs,
                )
            axis.set_title(f"Hgamma closure quality ({profile})", fontsize=10)
            axis.set_xlabel(AXIS_LABELS.get(variable, variable))
            axis.set_ylabel("Candidates / bin")
            axis.legend(frameon=False)
            output = plots_dir / f"quality_{variable}_closed_vs_nonclosed.png"
            figure.savefig(output)
            figure.savefig(output.with_suffix(".pdf"))
            plt.close(figure)
            written.append(output)

        for variable in CATEGORY_OVERLAY_VARIABLES:
            spec = plot_policy.spec_for(variable, profile)
            hist_kwargs = {"bins": spec.bins, "histtype": "step", "linewidth": 1.3}
            if spec.range is not None:
                hist_kwargs["range"] = spec.range
            figure, axis = plt.subplots(figsize=(6.4, 4.7), constrained_layout=True)
            for name, selected_rows in quality_rows.items():
                values = optional_values(selected_rows, variable)
                if values:
                    axis.hist(values, label=name, **hist_kwargs)
            axis.set_title(f"Quality categories ({profile})", fontsize=10)
            axis.set_xlabel(AXIS_LABELS.get(variable, variable))
            axis.set_ylabel("Candidates / bin")
            axis.legend(frameon=False, fontsize=7)
            output = plots_dir / f"quality_category_{variable}_overlay.png"
            figure.savefig(output)
            figure.savefig(output.with_suffix(".pdf"))
            plt.close(figure)
            written.append(output)

    scan_plot_names = {
        "rho_mass_window": "quality_scan_rho_mass_window.png",
        "h_mass_window": "quality_scan_h_mass_window.png",
        "delta_r_pipi": "quality_scan_delta_r_pipi.png",
        "rho_pt_over_photon_pt": "quality_scan_rho_pt_over_photon_pt.png",
        "photon_pt": "quality_scan_photon_pt.png",
        "rho_pt": "quality_scan_rho_pt.png",
    }
    plots_dir = plots_root / plot_policy.selected_profile
    plots_dir.mkdir(parents=True, exist_ok=True)
    for name, filename in scan_plot_names.items():
        rows = scans.get(name, [])
        if not rows:
            continue
        figure, axis = plt.subplots(figsize=(6.0, 4.3), constrained_layout=True)
        labels = [str(row["label"]) for row in rows]
        closure = [
            row["closure_fraction_after_cut"] or 0.0
            for row in rows
        ]
        retention = [row["candidate_retention"] or 0.0 for row in rows]
        x = range(len(labels))
        axis.plot(x, closure, marker="o", label="closure fraction")
        axis.plot(x, retention, marker="s", label="candidate retention")
        axis.set_xticks(list(x))
        axis.set_xticklabels(labels, rotation=30, ha="right")
        axis.set_ylim(0.0, 1.05)
        axis.set_title(name.replace("_", " "), fontsize=10)
        axis.set_ylabel("Fraction")
        axis.legend(frameon=False, loc="best")
        output = plots_dir / filename
        figure.savefig(output)
        figure.savefig(output.with_suffix(".pdf"))
        plt.close(figure)
        written.append(output)

    counts = multiplicity["multiplicity_counts"]
    ordered_buckets = sorted(counts, key=lambda item: 4 if item == ">=4" else int(item))
    figure, axis = plt.subplots(figsize=(5.4, 4.0), constrained_layout=True)
    axis.bar(ordered_buckets, [counts[bucket] for bucket in ordered_buckets], color="#2f6fbb")
    axis.set_xlabel("Candidates per event")
    axis.set_ylabel("Events")
    axis.set_title("Candidate multiplicity", fontsize=10)
    output = plots_dir / "quality_candidate_multiplicity.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    figure, axis = plt.subplots(figsize=(5.4, 4.0), constrained_layout=True)
    closure = multiplicity["closure_by_multiplicity"]
    axis.plot(
        ordered_buckets,
        [closure[bucket]["closed_event_fraction"] or 0.0 for bucket in ordered_buckets],
        marker="o",
    )
    axis.set_ylim(0.0, 1.05)
    axis.set_xlabel("Candidates per event")
    axis.set_ylabel("Closed event fraction")
    axis.set_title("Closure fraction vs multiplicity", fontsize=10)
    output = plots_dir / "quality_closure_fraction_vs_multiplicity.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    figure, axis = plt.subplots(figsize=(7.2, 4.4), constrained_layout=True)
    labels = list(policies)
    values = [policies[name]["closed_fraction"] or 0.0 for name in labels]
    axis.bar(labels, values, color="#2f6fbb")
    axis.tick_params(axis="x", labelrotation=30)
    axis.set_ylim(0.0, 1.05)
    axis.set_ylabel("Closed fraction")
    axis.set_title("Best-candidate policy comparison", fontsize=10)
    output = plots_dir / "quality_best_candidate_policy_comparison.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    category_payload = category_summary["categories"]
    names = list(category_payload)
    counts = [category_payload[name]["n_candidates"] for name in names]
    closed_fractions = [
        category_payload[name]["closed_fraction_all_candidates"] or 0.0 for name in names
    ]
    retention = [
        category_payload[name]["candidate_retention_relative_to_inclusive"] or 0.0
        for name in names
    ]
    figure, axis = plt.subplots(figsize=(7.4, 4.2), constrained_layout=True)
    axis.bar(names, counts, color="#2f6fbb")
    axis.tick_params(axis="x", labelrotation=30)
    axis.set_ylabel("Candidates")
    axis.set_title("Quality-category candidate counts", fontsize=10)
    output = plots_dir / "quality_category_candidate_counts.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    figure, axis = plt.subplots(figsize=(7.4, 4.2), constrained_layout=True)
    axis.bar(names, closed_fractions, color="#2f6fbb")
    axis.tick_params(axis="x", labelrotation=30)
    axis.set_ylim(0.0, 1.05)
    axis.set_ylabel("Hgamma-closed fraction")
    axis.set_title("Quality-category closure fraction", fontsize=10)
    output = plots_dir / "quality_category_closure_fraction.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)

    figure, axis = plt.subplots(figsize=(5.4, 4.2), constrained_layout=True)
    axis.scatter(retention, closed_fractions, color="#2f6fbb")
    for name, x_value, y_value in zip(names, retention, closed_fractions):
        axis.annotate(name, (x_value, y_value), fontsize=7, xytext=(3, 3), textcoords="offset points")
    axis.set_xlim(0.0, 1.05)
    axis.set_ylim(0.0, 1.05)
    axis.set_xlabel("Candidate retention")
    axis.set_ylabel("Hgamma-closed fraction")
    axis.set_title("Quality-category retention vs closure", fontsize=10)
    output = plots_dir / "quality_category_retention_vs_closure.png"
    figure.savefig(output)
    figure.savefig(output.with_suffix(".pdf"))
    plt.close(figure)
    written.append(output)
    return f"wrote {len(written)} PNG files and {len(written)} PDF files", written


def analyze(args: argparse.Namespace) -> tuple[Path, Path, str]:
    rows, fieldnames = read_rows(args.candidate_csv, args.max_rows)
    require_hgamma_columns(fieldnames)
    plot_policy = plotting_policy.load_plotting_policy(
        args.plot_config,
        args.plot_profile,
        write_full_range_sanity=args.write_full_range_sanity,
    )
    quality_categories = plotting_policy.load_quality_categories(args.quality_config)
    categories = categorize_rows(rows)
    summaries = variable_summaries(categories)
    separations = separation_summaries(categories)
    scans = threshold_scans(rows)
    multiplicity = candidate_multiplicity(rows)
    policies = best_candidate_policies(rows)
    quality_rows = category_rows_by_name(rows, quality_categories)
    quality_summary = summarize_quality_categories(rows, quality_categories)
    recommendation_text = recommendation(separations, multiplicity)
    stage16a_summary = load_summary_json(args.summary_json)
    args.outdir.mkdir(parents=True, exist_ok=True)
    coverage_rows = compute_range_coverage(rows, plot_policy)
    range_coverage_md, range_coverage_json = write_range_coverage(
        args.outdir, plot_policy, coverage_rows
    )
    if args.no_plots:
        plot_status, plots = "skipped (--no-plots)", []
    else:
        plot_status, plots = plot_all(
            args.outdir,
            categories,
            scans,
            multiplicity,
            policies,
            plot_policy,
            quality_rows,
            quality_summary,
        )
    quality_categories_md, quality_categories_json = write_quality_category_summary(
        args.outdir,
        quality_summary,
        args.quality_config,
        plot_status,
        plots,
    )
    json_path = args.outdir / "hgamma_closure_quality_summary.json"
    md_path = args.outdir / "hgamma_closure_quality_summary.md"
    write_json(
        json_path,
        args.candidate_csv,
        args.summary_json,
        stage16a_summary,
        categories,
        summaries,
        separations,
        scans,
        multiplicity,
        policies,
        plot_status,
        plots,
        recommendation_text,
        plot_policy,
        range_coverage_json,
        quality_categories_json,
    )
    write_markdown(
        md_path,
        args.candidate_csv,
        args.summary_json,
        rows,
        categories,
        summaries,
        separations,
        scans,
        multiplicity,
        policies,
        plot_status,
        plots,
        recommendation_text,
        plot_policy,
        range_coverage_md,
        quality_categories_md,
    )
    return md_path, json_path, plot_status


def main() -> int:
    args = parse_args()
    try:
        md_path, json_path, plot_status = analyze(args)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    print(f"summary: {md_path}")
    print(f"json: {json_path}")
    print(f"plots: {plot_status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
