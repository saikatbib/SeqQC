#!/usr/bin/env bash
# Endpoint sampling, not syscall/packet-level assurance. Requires an active GUI.
set -euo pipefail
executable="${1:?usage: observe-no-network-macos.sh <qualification executable>}"
command -v lsof >/dev/null
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
trace_file="$(mktemp)"
"$executable" >"$trace_file" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; rm -f "$trace_file"' EXIT
deadline=$((SECONDS + 10))
while [[ $SECONDS -lt $deadline ]]; do
  if ! kill -0 "$app_pid" 2>/dev/null || [[ "$(ps -o stat= -p "$app_pid")" == *Z* ]]; then
    cat "$trace_file" >&2
    echo 'Application exited early; observation cannot pass.' >&2
    exit 1
  fi
  ids=$(ps -axo pid=,ppid= | awk -v root="$app_pid" -f "$script_dir/process-family.awk" | paste -sd, -)
  if [[ -z "$ids" ]]; then echo 'Empty process family; observation cannot pass.' >&2; exit 1; fi
  set +e
  endpoints=$(lsof -nP -a -p "$ids" -iTCP -iUDP 2>&1)
  status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    printf '%s\n' "$endpoints" >&2
    echo 'TCP/UDP endpoint observed in process family.' >&2
    exit 1
  fi
  if [[ $status -ne 1 || -n "$endpoints" ]]; then
    printf '%s\n' "$endpoints" >&2
    echo 'Endpoint enumeration failed; observation cannot pass.' >&2
    exit 1
  fi
  sleep 0.1
done
echo 'No TCP/UDP endpoints sampled in the process family for 10 seconds; brief attempts between samples are not excluded.'
