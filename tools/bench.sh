#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binaries="$repo_root/server/target/release"

sizes="100 500 1000"
seconds=20
warmup=5
measure_bots=100
input_hz=""
port=9321
out_dir="${TMPDIR:-/tmp}/fomoxa-example-bench"

usage() {
    cat >&2 <<EOF
usage: tools/bench.sh [--bots="100 500 1000"] [--seconds=20] [--warmup=5]
                      [--measure-bots=100] [--input-hz=30] [--port=9321] [--out=<dir>]

  runs the server and fomoxa-example-bench once per bot count, then prints a
  markdown table. each run starts a fresh server so memory starts from zero.
  json for every run is written to <dir> (default $out_dir)
EOF
    exit 2
}

for argument in "$@"; do
    case "$argument" in
        --bots=*) sizes="${argument#*=}" ;;
        --seconds=*) seconds="${argument#*=}" ;;
        --warmup=*) warmup="${argument#*=}" ;;
        --measure-bots=*) measure_bots="${argument#*=}" ;;
        --input-hz=*) input_hz="${argument#*=}" ;;
        --port=*) port="${argument#*=}" ;;
        --out=*) out_dir="${argument#*=}" ;;
        -h | --help) usage ;;
        *)
            echo "tools/bench.sh: unknown argument $argument" >&2
            usage
            ;;
    esac
done

cargo build --release --manifest-path "$repo_root/server/Cargo.toml" --quiet
mkdir -p "$out_dir"

server_pid=""
cleanup() {
    [[ -n "$server_pid" ]] && kill "$server_pid" 2>/dev/null || true
}
trap cleanup EXIT

for bots in $sizes; do
    log="$out_dir/server-$bots.log"
    json="$out_dir/bench-$bots.json"

    "$binaries/fomoxa-example-server" --addr "127.0.0.1:$port" --quiet --profile --status-interval 10 >"$log" 2>&1 &
    server_pid=$!

    for _ in $(seq 100); do
        grep -q "listening" "$log" && break
        sleep 0.1
    done
    if ! grep -q "listening" "$log"; then
        echo "tools/bench.sh: server never reported listening, see $log" >&2
        exit 1
    fi

    echo "=== $bots bots ==="
    "$binaries/fomoxa-example-bench" \
        --addr "127.0.0.1:$port" \
        --bots "$bots" \
        --measure-bots "$measure_bots" \
        --seconds "$seconds" \
        --warmup "$warmup" \
        ${input_hz:+--input-hz "$input_hz"} \
        --server-pid "$server_pid" \
        --json "$json"

    grep "status" "$log" | tail -1 || true
    grep "profile" "$log" | tail -1 || true
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
    sleep 1
done

python3 - "$out_dir" $sizes <<'EOF'
import json, sys

out_dir, sizes = sys.argv[1], sys.argv[2:]
rows = []
for size in sizes:
    with open(f"{out_dir}/bench-{size}.json") as handle:
        rows.append(json.load(handle))

header = [
    "bot", "joined", "p50", "p95", "p99", "snapshot/s", "nhận được",
    "payload", "snapshot", "CPU server", "RSS server", "CPU generator",
]
print()
print("| " + " | ".join(header) + " |")
print("|" + "|".join(["---"] * len(header)) + "|")
for row in rows:
    latency = row["latency_ms"]
    print(
        f'| {row["bots"]} | {row["joined"]} '
        f'| {latency["p50"]:.1f} ms | {latency["p95"]:.1f} ms | {latency["p99"]:.1f} ms '
        f'| {row["snapshots_per_second"]:.0f} | {row["delivered_percent"]:.0f}% '
        f'| {row["payload_mib_per_second"]:.0f} MiB/s | {row["snapshot_bytes"]:.0f} B '
        f'| {row["server_cpu_cores_mean"]:.2f} core | {row["server_rss_mib_peak"]:.0f} MiB '
        f'| {row["generator_cpu_cores_mean"]:.2f} core |'
    )
EOF
