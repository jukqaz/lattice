#!/usr/bin/env bash
# lattice real HOME read-only health check
#
# Runs only read-only Lattice diagnostics against the caller's real HOME/XDG
# environment. This is intended for local release dogfood before tagging. It must
# not create config, register services, back up files, restore files, prune
# snapshots, or write commits.

set -euo pipefail

READ_ONLY_COMMANDS=(
  "lattice --version"
  "lattice doctor"
  "lattice validate"
  "lattice bootstrap check"
  "lattice service list"
  "lattice status --json <service>"
  "lattice plan --json <service>"
  "lattice discover --json"
  "lattice group list --json"
  "lattice group status --json <group>"
  "lattice group plan --json <group>"
)

MUTATING_COMMANDS=(
  "init"
  "backup"
  "restore"
  "adopt"
  "track"
  "undo --yes"
  "snapshot prune"
  "service add"
  "service remove"
  "include add"
  "include remove"
  "exclude add"
  "exclude remove"
  "permission set"
  "permission remove"
  "repo pull"
  "repo commit"
  "repo push"
)

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

if [[ -n "${LATTICE_BIN:-}" ]]; then
  LATTICE_CMD=("${LATTICE_BIN}")
elif [[ -x "${REPO_ROOT}/target/debug/lattice" ]]; then
  LATTICE_CMD=("${REPO_ROOT}/target/debug/lattice")
else
  printf 'Set LATTICE_BIN=/path/to/lattice or run cargo build -p lattice first.\n' >&2
  exit 127
fi

COMMAND_STATUSES=()
FAILED=0
LAST_OUTPUT=""
LAST_STATUS=0

print_list() {
  local title="$1"
  shift
  printf '%s\n' "${title}"
  local item
  for item in "$@"; do
    printf '  - %s\n' "${item}"
  done
}

run_lattice() {
  local label="$1"
  shift

  printf '\n## %s\n\n' "${label}"
  printf '$'
  printf ' %q' "${LATTICE_CMD[@]}" "$@"
  printf '\n\n'

  local output
  local status
  if output=$("${LATTICE_CMD[@]}" "$@" 2>&1); then
    status=0
  else
    status=$?
  fi

  LAST_OUTPUT="${output}"
  LAST_STATUS="${status}"
  printf '%s\n' "${output}"
  printf '\n(exit %s)\n' "${status}"
  COMMAND_STATUSES+=("${label}: ${status}")
  if [[ ${status} -ne 0 ]]; then
    FAILED=1
  fi
}

parse_group_names() {
  if ! command -v python3 >/dev/null 2>&1; then
    return 0
  fi

  python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
for group in data.get("groups", []):
    name = group.get("name")
    if isinstance(name, str) and name:
        print(name)
'
}

printf '# Lattice real HOME read-only health check\n\n'
printf 'HOME=%s\n' "${HOME:-}"
printf 'XDG_CONFIG_HOME=%s\n' "${XDG_CONFIG_HOME:-<unset>}"
printf 'XDG_DATA_HOME=%s\n' "${XDG_DATA_HOME:-<unset>}"
printf 'XDG_STATE_HOME=%s\n' "${XDG_STATE_HOME:-<unset>}"
printf 'XDG_CACHE_HOME=%s\n\n' "${XDG_CACHE_HOME:-<unset>}"
print_list 'READ_ONLY_COMMANDS:' "${READ_ONLY_COMMANDS[@]}"
printf '\n'
print_list 'MUTATING_COMMANDS intentionally not run:' "${MUTATING_COMMANDS[@]}"

run_lattice "lattice --version" --version
run_lattice "lattice doctor" doctor
run_lattice "lattice validate" validate
run_lattice "lattice bootstrap check" bootstrap check
run_lattice "lattice service list" service list
service_list="${LAST_OUTPUT}"
service_list_status="${LAST_STATUS}"

if [[ ${service_list_status} -eq 0 ]]; then
  while IFS= read -r service; do
    [[ -n "${service}" ]] || continue
    run_lattice "lattice status --json ${service}" status --json "${service}"
    run_lattice "lattice plan --json ${service}" plan --json "${service}"
  done <<<"${service_list}"
else
  printf '\nSkipping per-service status/plan because lattice service list exited %s.\n' "${service_list_status}"
fi

run_lattice "lattice discover --json" discover --json
run_lattice "lattice group list --json" group list --json
group_list_json="${LAST_OUTPUT}"
group_list_status="${LAST_STATUS}"

if [[ ${group_list_status} -eq 0 ]]; then
  while IFS= read -r group; do
    [[ -n "${group}" ]] || continue
    run_lattice "lattice group status --json ${group}" group status --json "${group}"
    run_lattice "lattice group plan --json ${group}" group plan --json "${group}"
  done < <(printf '%s\n' "${group_list_json}" | parse_group_names)
else
  printf '\nSkipping per-group status/plan because lattice group list --json exited %s.\n' "${group_list_status}"
fi

printf '\n## Summary\n\n'
print_list 'command exit statuses:' "${COMMAND_STATUSES[@]}"
printf '\nNo mutating Lattice commands were run. Non-zero diagnostic exits above are health findings, not script mutations.\n'

exit "${FAILED}"
