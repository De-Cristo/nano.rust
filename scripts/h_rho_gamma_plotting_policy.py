#!/usr/bin/env python3
"""Plot-profile and candidate-quality helpers for HToRhoGamma diagnostics."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import tomllib
except ImportError:  # pragma: no cover - Python < 3.11 fallback for local users.
    try:
        import tomli as tomllib
    except ImportError:
        tomllib = None


ANTI_CIRCULAR_VARIABLES = {"h_mass", "reco_h_mass_minus_gen_h_mass"}
DEFAULT_BINS = 60
DEFAULT_COVERAGE_WARNING_THRESHOLD = 0.10


@dataclass(frozen=True)
class PlotSpec:
    range: tuple[float, float] | None = None
    bins: int = DEFAULT_BINS
    overflow: bool = False


@dataclass(frozen=True)
class PlottingPolicy:
    selected_profile: str
    profiles: dict[str, dict[str, PlotSpec]]
    default_profile: str
    write_full_range_sanity: bool
    write_range_coverage: bool
    coverage_warning_threshold: float

    def spec_for(self, variable: str, profile: str | None = None) -> PlotSpec:
        profile_name = profile or self.selected_profile
        return self.profiles.get(profile_name, {}).get(variable, PlotSpec())

    def output_profiles(self) -> list[str]:
        profiles = [self.selected_profile]
        if self.write_full_range_sanity and self.selected_profile != "full_range_sanity":
            if "full_range_sanity" in self.profiles:
                profiles.append("full_range_sanity")
        return profiles


@dataclass(frozen=True)
class Requirement:
    variable: str
    op: str
    value: Any


@dataclass(frozen=True)
class QualityCategory:
    name: str
    label: str
    requirements: tuple[Requirement, ...]


@dataclass(frozen=True)
class QualityCategories:
    categories: tuple[QualityCategory, ...]
    warnings: tuple[str, ...]


def _load_toml(path: Path) -> dict[str, Any]:
    if tomllib is None:
        return _load_toml_subset(path)
    with path.open("rb") as handle:
        return tomllib.load(handle)


def _strip_comment(line: str) -> str:
    in_string = False
    escaped = False
    chars = []
    for char in line:
        if char == "\\" and in_string:
            escaped = not escaped
            chars.append(char)
            continue
        if char == '"' and not escaped:
            in_string = not in_string
        escaped = False
        if char == "#" and not in_string:
            break
        chars.append(char)
    return "".join(chars).strip()


def _load_toml_subset(path: Path) -> dict[str, Any]:
    root: dict[str, Any] = {}
    current: dict[str, Any] = root
    pending_key: str | None = None
    pending_lines: list[str] = []
    bracket_depth = 0
    for raw_line in path.read_text().splitlines():
        line = _strip_comment(raw_line)
        if not line:
            continue
        if pending_key is not None:
            pending_lines.append(line)
            bracket_depth += line.count("[") - line.count("]")
            if bracket_depth <= 0:
                current[pending_key] = _parse_toml_value(" ".join(pending_lines))
                pending_key = None
                pending_lines = []
            continue
        if line.startswith("[") and line.endswith("]"):
            current = root
            for part in line[1:-1].split("."):
                current = current.setdefault(part, {})
            continue
        if "=" not in line:
            raise ValueError(f"unsupported TOML line in {path}: {line}")
        key, raw_value = [part.strip() for part in line.split("=", 1)]
        bracket_depth = raw_value.count("[") - raw_value.count("]")
        if bracket_depth > 0:
            pending_key = key
            pending_lines = [raw_value]
            continue
        current[key] = _parse_toml_value(raw_value)
    if pending_key is not None:
        raise ValueError(f"unterminated TOML array for key {pending_key} in {path}")
    return root


def _split_top_level(raw: str) -> list[str]:
    parts = []
    depth_square = 0
    depth_curly = 0
    in_string = False
    start = 0
    for index, char in enumerate(raw):
        if char == '"':
            in_string = not in_string
        elif not in_string:
            if char == "[":
                depth_square += 1
            elif char == "]":
                depth_square -= 1
            elif char == "{":
                depth_curly += 1
            elif char == "}":
                depth_curly -= 1
            elif char == "," and depth_square == 0 and depth_curly == 0:
                parts.append(raw[start:index].strip())
                start = index + 1
    tail = raw[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _parse_toml_value(raw: str) -> Any:
    raw = raw.strip().rstrip(",")
    if raw.startswith('"') and raw.endswith('"'):
        return raw[1:-1]
    if raw == "true":
        return True
    if raw == "false":
        return False
    if raw.startswith("{") and raw.endswith("}"):
        parsed: dict[str, Any] = {}
        for item in _split_top_level(raw[1:-1]):
            key, value = [part.strip() for part in item.split("=", 1)]
            parsed[key] = _parse_toml_value(value)
        return parsed
    if raw.startswith("[") and raw.endswith("]"):
        body = raw[1:-1].strip()
        if not body:
            return []
        return [_parse_toml_value(item) for item in _split_top_level(body)]
    try:
        if any(char in raw for char in (".", "e", "E")):
            return float(raw)
        return int(raw)
    except ValueError:
        return raw


def _parse_plot_spec(raw: dict[str, Any]) -> PlotSpec:
    plot_range = raw.get("range")
    parsed_range = None
    if plot_range is not None:
        if not isinstance(plot_range, list) or len(plot_range) != 2:
            raise ValueError(f"plot range must be a two-value list, got {plot_range!r}")
        xmin, xmax = float(plot_range[0]), float(plot_range[1])
        if xmax <= xmin:
            raise ValueError(f"plot range maximum must exceed minimum, got {plot_range!r}")
        parsed_range = (xmin, xmax)
    bins = int(raw.get("bins", DEFAULT_BINS))
    if bins <= 0:
        raise ValueError(f"plot bins must be positive, got {bins}")
    return PlotSpec(
        range=parsed_range,
        bins=bins,
        overflow=bool(raw.get("overflow", False)),
    )


def builtin_plotting_policy(
    profile: str = "physics_focus",
    write_full_range_sanity: bool = False,
) -> PlottingPolicy:
    profiles = {
        "physics_focus": {
            "h_mass": PlotSpec((80.0, 180.0), 50, True),
            "rho_mass": PlotSpec((0.30, 1.20), 45, True),
            "photon_pt": PlotSpec((0.0, 250.0), 50, True),
            "rho_pt": PlotSpec((0.0, 250.0), 50, True),
            "h_pt": PlotSpec((0.0, 300.0), 60, True),
            "delta_r_pipi": PlotSpec((0.0, 0.25), 50, True),
            "delta_r_gamma_rho": PlotSpec((0.0, 5.0), 50, True),
            "rho_pt_over_photon_pt": PlotSpec((0.0, 3.0), 60, True),
        },
        "full_range_sanity": {
            "h_mass": PlotSpec((0.0, 2000.0), 100, True),
            "rho_mass": PlotSpec((0.0, 10.0), 100, True),
            "photon_pt": PlotSpec((0.0, 1000.0), 100, True),
            "rho_pt": PlotSpec((0.0, 1000.0), 100, True),
            "h_pt": PlotSpec((0.0, 1000.0), 100, True),
            "delta_r_pipi": PlotSpec((0.0, 1.0), 100, True),
            "delta_r_gamma_rho": PlotSpec((0.0, 6.0), 100, True),
            "rho_pt_over_photon_pt": PlotSpec((0.0, 10.0), 100, True),
        },
        "signal_window": {
            "h_mass": PlotSpec((100.0, 160.0), 60, True),
            "rho_mass": PlotSpec((0.50, 1.05), 55, True),
            "delta_r_pipi": PlotSpec((0.0, 0.15), 50, True),
            "rho_pt_over_photon_pt": PlotSpec((0.4, 1.8), 56, True),
        },
    }
    if profile not in profiles:
        raise ValueError(f"unknown plot profile '{profile}'")
    return PlottingPolicy(
        selected_profile=profile,
        profiles=profiles,
        default_profile="physics_focus",
        write_full_range_sanity=write_full_range_sanity,
        write_range_coverage=True,
        coverage_warning_threshold=DEFAULT_COVERAGE_WARNING_THRESHOLD,
    )


def load_plotting_policy(
    path: Path | None,
    profile: str | None,
    write_full_range_sanity: bool | None = None,
) -> PlottingPolicy:
    if path is None:
        selected = profile or "physics_focus"
        return builtin_plotting_policy(
            selected,
            write_full_range_sanity=bool(write_full_range_sanity),
        )
    data = _load_toml(path)
    plotting = data.get("plotting", {})
    raw_profiles = plotting.get("profiles", {})
    if not raw_profiles:
        raise ValueError(f"plot config has no [plotting.profiles] entries: {path}")
    profiles: dict[str, dict[str, PlotSpec]] = {}
    for profile_name, raw_profile in raw_profiles.items():
        profiles[profile_name] = {
            variable: _parse_plot_spec(raw_spec)
            for variable, raw_spec in raw_profile.items()
        }
    default_profile = str(plotting.get("default_profile", "physics_focus"))
    selected = profile or default_profile
    if selected not in profiles:
        choices = ", ".join(sorted(profiles))
        raise ValueError(f"unknown plot profile '{selected}' in {path}; available: {choices}")
    full_range = bool(plotting.get("write_full_range_sanity", False))
    if write_full_range_sanity is not None:
        full_range = write_full_range_sanity
    return PlottingPolicy(
        selected_profile=selected,
        profiles=profiles,
        default_profile=default_profile,
        write_full_range_sanity=full_range,
        write_range_coverage=bool(plotting.get("write_range_coverage", True)),
        coverage_warning_threshold=float(
            plotting.get("coverage_warning_threshold", DEFAULT_COVERAGE_WARNING_THRESHOLD)
        ),
    )


def range_coverage(
    variable: str,
    profile: str,
    spec: PlotSpec,
    values: list[float],
) -> dict[str, float | int | str | None]:
    total = len(values)
    if spec.range is None:
        return {
            "variable": variable,
            "profile": profile,
            "xmin": None,
            "xmax": None,
            "bins": spec.bins,
            "n_total": total,
            "n_inside": total,
            "n_underflow": 0,
            "n_overflow": 0,
            "frac_inside": 1.0 if total else None,
            "frac_underflow": 0.0 if total else None,
            "frac_overflow": 0.0 if total else None,
            "excluded_fraction": 0.0 if total else None,
        }
    xmin, xmax = spec.range
    underflow = sum(1 for value in values if value < xmin)
    overflow = sum(1 for value in values if value > xmax)
    inside = total - underflow - overflow
    return {
        "variable": variable,
        "profile": profile,
        "xmin": xmin,
        "xmax": xmax,
        "bins": spec.bins,
        "n_total": total,
        "n_inside": inside,
        "n_underflow": underflow,
        "n_overflow": overflow,
        "frac_inside": inside / total if total else None,
        "frac_underflow": underflow / total if total else None,
        "frac_overflow": overflow / total if total else None,
        "excluded_fraction": (underflow + overflow) / total if total else None,
    }


def load_quality_categories(path: Path | None) -> QualityCategories:
    if path is None:
        return QualityCategories((QualityCategory("inclusive", "Inclusive accepted candidates", ()),), ())
    data = _load_toml(path)
    raw_categories = data.get("quality_categories", {})
    if not raw_categories:
        raise ValueError(f"quality config has no [quality_categories] entries: {path}")
    categories: list[QualityCategory] = []
    warnings: list[str] = []
    for name, raw_category in raw_categories.items():
        requirements = []
        for raw_requirement in raw_category.get("requirements", []):
            requirement = Requirement(
                variable=str(raw_requirement["variable"]),
                op=str(raw_requirement["op"]),
                value=raw_requirement.get("value"),
            )
            requirements.append(requirement)
            if requirement.variable in ANTI_CIRCULAR_VARIABLES:
                warnings.append(
                    "anti-circularity warning: category "
                    f"'{name}' uses '{requirement.variable}', which is tied to hgamma closure"
                )
        categories.append(
            QualityCategory(
                name=name,
                label=str(raw_category.get("label", name)),
                requirements=tuple(requirements),
            )
        )
    return QualityCategories(tuple(categories), tuple(warnings))


def _coerce_row_value(raw: str | None, expected: Any) -> Any:
    if isinstance(expected, bool):
        return str(raw).lower() in {"1", "true", "yes"}
    if isinstance(expected, (int, float)):
        if raw is None or raw == "":
            return None
        return float(raw)
    return raw


def requirement_passes(row: dict[str, str], requirement: Requirement) -> bool:
    value = _coerce_row_value(row.get(requirement.variable), requirement.value)
    target = requirement.value
    if value is None:
        return False
    if requirement.op == ">":
        return value > target
    if requirement.op == ">=":
        return value >= target
    if requirement.op == "<":
        return value < target
    if requirement.op == "<=":
        return value <= target
    if requirement.op == "==":
        return value == target
    if requirement.op == "!=":
        return value != target
    raise ValueError(f"unsupported requirement operator '{requirement.op}'")


def category_passes(row: dict[str, str], category: QualityCategory) -> bool:
    return all(requirement_passes(row, requirement) for requirement in category.requirements)
