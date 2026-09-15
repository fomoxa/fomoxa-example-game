#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
reference="$repo_root/server/.fomoxa/schema.json"

read_fingerprint() {
    grep -m1 -o '"fingerprint_u64": *"[^"]*"' "$1" | sed -E 's/.*"(0x[0-9A-Fa-f]+)"$/\1/'
}

if [[ ! -f "$reference" ]]; then
    echo "missing $reference - run fomoxac generate in server/ first" >&2
    exit 1
fi

expected="$(read_fingerprint "$reference")"
printf 'server   %s\n' "$expected"

status=0
while IFS= read -r schema; do
    actual="$(read_fingerprint "$schema")"
    label="${schema#"$repo_root"/}"
    if [[ "$actual" == "$expected" ]]; then
        printf 'OK       %s  %s\n' "$actual" "$label"
    else
        printf 'MISMATCH %s  %s\n' "$actual" "$label"
        status=1
    fi
done < <(find "$repo_root" -path '*/.fomoxa/schema.json' -not -path "$reference" -not -path '*/target/*' | sort)

exit "$status"
