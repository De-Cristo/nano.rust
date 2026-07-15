#!/usr/bin/env python3
"""Survey GenPart topology in H->rho gamma NanoAOD-like signal files."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_OUTDIR = Path("outputs/h_rho_gamma_genpart_survey")
DEFAULT_SAFE_MAX_FILES = 5
COUNT_KEYS = (
    "processed_events",
    "events_with_genpart",
    "events_with_higgs",
    "events_with_rho0",
    "events_with_photon",
    "events_with_pi_plus_pi_minus",
    "events_with_h_gamma_pions",
    "events_with_explicit_h_to_rho_gamma",
    "events_with_explicit_rho_to_pions",
    "events_with_photon_higgs_ancestor",
    "events_with_pi_plus_higgs_ancestor",
    "events_with_pi_minus_higgs_ancestor",
    "events_with_pi_plus_rho_ancestor",
    "events_with_pi_minus_rho_ancestor",
    "events_with_rho_higgs_ancestor",
)
MAP_KEYS = (
    "signed_pdg_counts",
    "abs_pdg_counts",
    "status_counts",
    "status_flags_counts",
    "status_status_flags_counts",
    "mother_daughter_counts",
)
KEY_PDGS = ("25", "22", "113", "211", "-211", "111", "221", "223", "213", "-213")


@dataclass(frozen=True)
class InputFile:
    label: str
    path: Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Survey GenPart topology for the HToRhoGamma offline NanoAOD signal sample."
    )
    parser.add_argument("--local-dir", type=Path, required=True)
    parser.add_argument("--local-glob", default="*.root")
    parser.add_argument("--outdir", type=Path, default=DEFAULT_OUTDIR)
    parser.add_argument("--max-files", type=int, default=None)
    parser.add_argument("--all-files", action="store_true")
    parser.add_argument("--max-events-per-file", type=int, default=None)
    parser.add_argument("--examples", type=int, default=3)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
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


def survey_binary(release: bool) -> Path:
    profile = "release" if release else "debug"
    return repo_root() / "target" / profile / "examples" / "h_rho_gamma_genpart_survey"


def build_binary(release: bool) -> Path:
    command = ["cargo", "build", "-p", "nano-io", "--example", "h_rho_gamma_genpart_survey"]
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
            f"survey binary build failed with exit code {result.returncode}: {shell_join(command)}\n"
            f"{result.stderr}"
        )
    return survey_binary(release)


def prepare_binary(args: argparse.Namespace) -> Path:
    binary = survey_binary(args.release)
    if args.no_build:
        if not binary.exists():
            raise FileNotFoundError(f"survey binary does not exist: {binary}")
        return binary
    return build_binary(args.release)


def survey_command(
    binary: Path,
    input_path: Path,
    json_path: Path,
    examples_path: Path,
    max_events: int | None,
    examples: int,
) -> list[str]:
    command = [
        str(binary),
        str(input_path),
    ]
    if max_events is not None:
        command.append(str(max_events))
    command.extend(
        [
            "--out-json",
            str(json_path),
            "--out-text",
            str(json_path.with_suffix(".txt")),
            "--out-examples",
            str(examples_path),
            "--examples",
            str(examples),
        ]
    )
    return command


def run_file(
    binary: Path,
    input_file: InputFile,
    per_file_dir: Path,
    max_events: int | None,
    examples: int,
    skip_existing: bool,
) -> dict:
    json_path = per_file_dir / f"{input_file.label}.summary.json"
    examples_path = per_file_dir / f"{input_file.label}.examples.txt"
    stdout_path = per_file_dir / f"{input_file.label}.stdout.txt"
    if skip_existing and json_path.exists() and stdout_path.exists():
        data = json.loads(json_path.read_text())
        data["files"] = [
            {
                "label": input_file.label,
                "path": str(input_file.path),
                "summary_json": str(json_path),
                "examples": str(examples_path),
                "stdout": str(stdout_path),
                "run_status": "skipped",
            }
        ]
        return data

    command = survey_command(binary, input_file.path, json_path, examples_path, max_events, examples)
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
            "examples": str(examples_path),
            "stdout": str(stdout_path),
            "run_status": "ok",
        }
    ]
    return data


def merge_summaries(summaries: list[dict]) -> dict:
    merged: dict = {key: 0 for key in COUNT_KEYS}
    for key in MAP_KEYS:
        merged[key] = {}
    merged["files"] = []

    for summary in summaries:
        for key in COUNT_KEYS:
            merged[key] += int(summary.get(key, 0))
        for key in MAP_KEYS:
            merge_count_map(merged[key], summary.get(key, {}))
        merged["files"].extend(summary.get("files", []))

    merged["recommendation"] = recommendation(merged)
    return merged


def merge_count_map(target: dict[str, int], source: dict) -> None:
    for key, count in source.items():
        target[str(key)] = target.get(str(key), 0) + int(count)


def recommendation(summary: dict) -> str:
    if (
        summary.get("events_with_explicit_h_to_rho_gamma", 0) > 0
        and summary.get("events_with_explicit_rho_to_pions", 0) > 0
    ):
        return "explicit rho0 exists; use last-copy rho0 and descendants"
    if (
        summary.get("events_with_photon_higgs_ancestor", 0) > 0
        and summary.get("events_with_pi_plus_higgs_ancestor", 0) > 0
        and summary.get("events_with_pi_minus_higgs_ancestor", 0) > 0
    ):
        return "rho0 absent or incomplete; match final-state photon and charged pions from Higgs ancestry"
    return (
        "ancestry unreliable; use nearest final-state photon/pi+pi- matching in signal MC, "
        "plus generator-level invariant-mass checks"
    )


def write_text_summary(path: Path, args: argparse.Namespace, inputs: list[InputFile], summary: dict) -> None:
    lines = [
        "HToRhoGamma GenPart topology survey",
        f"mode: {'dry-run' if args.dry_run else 'run'}",
        f"local_dir: {args.local_dir}",
        f"local_glob: {args.local_glob}",
        f"resolved_files: {len(inputs)}",
        f"selected_files: {summary.get('selected_files', len(summary.get('files', [])))}",
        f"max_events_per_file: {args.max_events_per_file if args.max_events_per_file is not None else 'all'}",
        f"processed_events: {summary.get('processed_events', 0)}",
        f"events_with_genpart: {summary.get('events_with_genpart', 0)}",
        f"events_with_higgs: {summary.get('events_with_higgs', 0)}",
        f"events_with_rho0: {summary.get('events_with_rho0', 0)}",
        f"events_with_photon: {summary.get('events_with_photon', 0)}",
        f"events_with_pi_plus_pi_minus: {summary.get('events_with_pi_plus_pi_minus', 0)}",
        f"events_with_h_gamma_pions: {summary.get('events_with_h_gamma_pions', 0)}",
        f"events_with_explicit_h_to_rho_gamma: {summary.get('events_with_explicit_h_to_rho_gamma', 0)}",
        f"events_with_explicit_rho_to_pions: {summary.get('events_with_explicit_rho_to_pions', 0)}",
        f"events_with_photon_higgs_ancestor: {summary.get('events_with_photon_higgs_ancestor', 0)}",
        f"events_with_pi_plus_higgs_ancestor: {summary.get('events_with_pi_plus_higgs_ancestor', 0)}",
        f"events_with_pi_minus_higgs_ancestor: {summary.get('events_with_pi_minus_higgs_ancestor', 0)}",
        f"events_with_pi_plus_rho_ancestor: {summary.get('events_with_pi_plus_rho_ancestor', 0)}",
        f"events_with_pi_minus_rho_ancestor: {summary.get('events_with_pi_minus_rho_ancestor', 0)}",
        f"events_with_rho_higgs_ancestor: {summary.get('events_with_rho_higgs_ancestor', 0)}",
        f"recommendation: {summary.get('recommendation', recommendation(summary))}",
        "",
        "key signed pdgId counts:",
    ]
    signed = summary.get("signed_pdg_counts", {})
    for pdg in KEY_PDGS:
        lines.append(f"  {pdg}: {signed.get(pdg, 0)}")
    lines.extend(["", "key abs pdgId counts:"])
    abs_counts = summary.get("abs_pdg_counts", {})
    for pdg in KEY_PDGS:
        lines.append(f"  {pdg}: {abs_counts.get(pdg.lstrip('-'), 0)}")
    lines.extend(["", "top abs pdgId counts:"])
    for pdg, count in sorted(
        abs_counts.items(), key=lambda item: (-int(item[1]), int(item[0]) if item[0].isdigit() else 0)
    )[:20]:
        lines.append(f"  {pdg}: {count}")
    lines.extend(["", "mother -> daughter counts involving key pdgIds:"])
    for relation, count in sorted(summary.get("mother_daughter_counts", {}).items()):
        left, _, right = relation.partition("->")
        if left in KEY_PDGS or right in KEY_PDGS:
            lines.append(f"  {relation}: {count}")
    lines.extend(["", "status counts for key pdgIds:"])
    for key, count in sorted(summary.get("status_counts", {}).items()):
        pdg, _, _status = key.partition(":")
        if pdg in KEY_PDGS:
            lines.append(f"  {key}: {count}")
    lines.extend(["", "statusFlags counts for key pdgIds:"])
    for key, count in sorted(summary.get("status_flags_counts", {}).items()):
        pdg, _, _flags = key.partition(":")
        if pdg in KEY_PDGS:
            lines.append(f"  {key}: {count}")
    lines.extend(["", "per_file:"])
    for file_info in summary.get("files", []):
        lines.append(
            "  "
            f"{file_info.get('label')} status={file_info.get('run_status')} "
            f"path={file_info.get('path')} summary={file_info.get('summary_json')}"
        )
    path.write_text("\n".join(lines) + "\n")


def dry_run_summary(args: argparse.Namespace, inputs: list[InputFile], selected: list[InputFile]) -> dict:
    return {
        "mode": "dry-run",
        "resolved_files": len(inputs),
        "selected_files": len(selected),
        "files": [
            {"label": item.label, "path": str(item.path), "run_status": "dry-run"}
            for item in selected
        ],
    }


def main() -> int:
    args = parse_args()
    inputs = local_dir_inputs(args.local_dir, args.local_glob)
    selected = select_inputs(inputs, args.max_files, args.all_files)
    args.outdir.mkdir(parents=True, exist_ok=True)
    per_file_dir = args.outdir / "per_file"
    per_file_dir.mkdir(parents=True, exist_ok=True)

    if args.dry_run:
        summary = dry_run_summary(args, inputs, selected)
        json_path = args.outdir / "genpart_topology_summary.json"
        text_path = args.outdir / "genpart_topology_summary.txt"
        json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
        write_text_summary(text_path, args, inputs, summary)
        print(f"resolved files: {len(inputs)}")
        print(f"selected files: {len(selected)}")
        print(f"summary: {text_path}")
        return 0

    binary = prepare_binary(args)
    summaries = [
        run_file(
            binary,
            input_file,
            per_file_dir,
            args.max_events_per_file,
            args.examples,
            args.skip_existing,
        )
        for input_file in selected
    ]
    merged = merge_summaries(summaries)
    merged.update(
        {
            "mode": "run",
            "local_dir": str(args.local_dir),
            "local_glob": args.local_glob,
            "resolved_files": len(inputs),
            "selected_files": len(selected),
            "all_files": args.all_files,
            "max_events_per_file": args.max_events_per_file,
            "survey_binary": str(binary),
        }
    )
    json_path = args.outdir / "genpart_topology_summary.json"
    text_path = args.outdir / "genpart_topology_summary.txt"
    json_path.write_text(json.dumps(merged, indent=2, sort_keys=True) + "\n")
    write_text_summary(text_path, args, inputs, merged)
    print(f"resolved files: {len(inputs)}")
    print(f"selected files: {len(selected)}")
    print(f"processed_events: {merged['processed_events']}")
    print(f"summary: {text_path}")
    print(f"json: {json_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
