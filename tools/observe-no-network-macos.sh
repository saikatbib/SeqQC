#!/usr/bin/env bash
# Endpoint sampling, not syscall/packet-level assurance. Requires an active GUI.
set -euo pipefail
executable="${1:?usage: observe-no-network-macos.sh <qualification executable>}"
command -v lsof >/dev/null
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
  ids=$(ps -axo pid=,ppid= | awk -v root="$app_pid" '
    { parent[$1]=$2 } END {
      seen[root]=1; changed=1;
      while(changed) { changed=0; for(pid in parent) if(seen[parent[pid]] && !seen[pid]) {seen[pid]=1; changed=1} }
      separator=""; for(pid in seen) {printf "%s%s",separator,pid; separator=","}
    }')
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
