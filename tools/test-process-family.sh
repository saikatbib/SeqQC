#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
actual=$(printf '%s\n' '1 0' '7 1' '100 7' '101 100' '102 101' '200 7' '201 200' | awk -v root=100 -f "$script_dir/process-family.awk" | sort -n)
expected=$(printf '%s\n' 100 101 102)
if [[ "$actual" != "$expected" ]]; then
  printf 'Wrong process family:\n%s\n' "$actual" >&2
  exit 1
fi
echo 'Process-family attribution excludes ancestors, unrelated siblings, and PID zero.'
