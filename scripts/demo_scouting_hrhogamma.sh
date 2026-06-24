#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/demo_scouting_hrhogamma.sh <input.root> [max-events] [config.toml] [--csv candidates.csv]
   or: NANO_SCOUTING_HRHOGAMMA_FILE=<input.root> scripts/demo_scouting_hrhogamma.sh [--csv candidates.csv]
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

positional=()
csv=""
while (( $# > 0 )); do
  case "$1" in
    --csv)
      shift
      if [[ -z "${1:-}" ]]; then
        printf 'error: missing value after --csv\n' >&2
        exit 2
      fi
      csv="$1"
      shift
      ;;
    --*)
      printf 'error: unknown option: %s\n' "$1" >&2
      exit 2
      ;;
    *)
      positional+=("$1")
      shift
      ;;
  esac
done

if (( ${#positional[@]} > 3 )); then
  usage >&2
  exit 2
fi

input="${positional[0]:-${NANO_SCOUTING_HRHOGAMMA_FILE:-}}"
max_events="${positional[1]:-}"
config="${positional[2]:-}"

if [[ -z "$input" ]]; then
  usage >&2
  exit 2
fi

if [[ ! -f "$input" ]]; then
  printf 'error: input ROOT file does not exist: %s\n' "$input" >&2
  exit 1
fi

cmd=(cargo run -p nano-io --example scouting_h_rho_gamma -- "$input")
if [[ -n "$max_events" ]]; then
  cmd+=("$max_events")
fi
if [[ -n "$config" ]]; then
  cmd+=("$config")
fi
if [[ -n "$csv" ]]; then
  cmd+=(--csv "$csv")
fi

cd "$repo_root"
printf '$'
printf ' %q' "${cmd[@]}"
printf '\n'
exec "${cmd[@]}"
