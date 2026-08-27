#!/usr/bin/env bash
set -euo pipefail

executable="${1:?usage: observe-no-network-linux.sh <SeqQC executable>}"
trace_file="$(mktemp)"
trap 'rm -f "$trace_file"' EXIT

set +e
timeout 10s strace -f -qq -e trace=network -o "$trace_file" "$executable"
status=$?
set -e

if grep -E 'socket\(AF_INET6?|connect\([^,]+, \{sa_family=AF_INET6?' "$trace_file"; then
  echo "SeqQC attempted an internet-family socket operation." >&2
  exit 1
fi

if [[ $status -ne 0 && $status -ne 124 ]]; then
  echo "SeqQC exited unexpectedly during no-network observation: $status" >&2
  exit "$status"
fi

echo "No AF_INET or AF_INET6 socket operation was observed."
