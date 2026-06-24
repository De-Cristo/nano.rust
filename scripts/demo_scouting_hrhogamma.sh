#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/demo_scouting_hrhogamma.sh <input.root> [max-events]
   or: NANO_SCOUTING_HRHOGAMMA_FILE=<input.root> scripts/demo_scouting_hrhogamma.sh
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

if (( $# > 2 )); then
  usage >&2
  exit 2
fi

input="${1:-${NANO_SCOUTING_HRHOGAMMA_FILE:-}}"
max_events="${2:-}"

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

cd "$repo_root"
printf '$'
printf ' %q' "${cmd[@]}"
printf '\n'
exec "${cmd[@]}"
