#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
demo="$repo_root/server/target/release"
baseline="$repo_root/baseline/target/release"

bots=1000
seconds=20
warmup=5
measure_bots=100
rounds=20000
repeats=3
port=9420
out_dir="${TMPDIR:-/tmp}/fomoxa-example-baseline"

usage() {
    cat >&2 <<EOF
usage: tools/baseline.sh [--bots=1000] [--seconds=20] [--warmup=5] [--measure-bots=100]
                         [--rounds=20000] [--repeats=3] [--port=9420] [--out=<dir>]

  compares three stacks on the same workload and the same measurement code:

    fomoxa        fomoxa-net + codec sinh bằng fomoxac
    protobuf-net  fomoxa-net + payload protobuf (chỉ thay codec)
    protobuf-tcp  TCP thô + framing tự viết + payload protobuf

  also runs the codec-only micro-benchmark for both codecs.
  json for every run is written to <dir> (default $out_dir)
EOF
    exit 2
}

for argument in "$@"; do
    case "$argument" in
        --bots=*) bots="${argument#*=}" ;;
        --seconds=*) seconds="${argument#*=}" ;;
        --warmup=*) warmup="${argument#*=}" ;;
        --measure-bots=*) measure_bots="${argument#*=}" ;;
        --rounds=*) rounds="${argument#*=}" ;;
        --repeats=*) repeats="${argument#*=}" ;;
        --port=*) port="${argument#*=}" ;;
        --out=*) out_dir="${argument#*=}" ;;
        -h | --help) usage ;;
        *)
            echo "tools/baseline.sh: unknown argument $argument" >&2
            usage
            ;;
    esac
done

cargo build --release --quiet --manifest-path "$repo_root/server/Cargo.toml"
cargo build --release --quiet --manifest-path "$repo_root/baseline/Cargo.toml"
mkdir -p "$out_dir"

echo "=== codec only ($bots players, $rounds rounds, $repeats lượt) ==="
for _ in $(seq "$repeats"); do
    "$demo/fomoxa-example-bench" --codec "$bots" --rounds "$rounds" | sed -n '1,3p'
    "$baseline/baseline-codec" --players "$bots" --rounds "$rounds" | sed -n '1,4p'
done

server_pid=""
cleanup() {
    [[ -n "$server_pid" ]] && kill "$server_pid" 2>/dev/null || true
}
trap cleanup EXIT

run_stack() {
    local name="$1" server_binary="$2" bench_binary="$3" stack_port="$4"
    local log="$out_dir/server-$name.log"
    local json="$out_dir/$name.json"

    "$server_binary" --addr "127.0.0.1:$stack_port" --quiet --profile --status-interval 10 >"$log" 2>&1 &
    server_pid=$!

    for _ in $(seq 100); do
        grep -q "listening" "$log" && break
        sleep 0.1
    done
    if ! grep -q "listening" "$log"; then
        echo "tools/baseline.sh: $name server never reported listening, see $log" >&2
        exit 1
    fi

    echo "=== $name ==="
    "$bench_binary" \
        --addr "127.0.0.1:$stack_port" \
        --bots "$bots" \
        --measure-bots "$measure_bots" \
        --seconds "$seconds" \
        --warmup "$warmup" \
        --server-pid "$server_pid" \
        --json "$json"

    grep "status" "$log" | tail -1 || true
    grep "profile" "$log" | tail -1 || true
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
    sleep 1
}

run_stack fomoxa "$demo/fomoxa-example-server" "$demo/fomoxa-example-bench" "$port"
run_stack protobuf-net "$baseline/baseline-net-server" "$baseline/baseline-net-bench" "$((port + 1))"
run_stack protobuf-tcp "$baseline/baseline-tcp-server" "$baseline/baseline-tcp-bench" "$((port + 2))"

python3 - "$out_dir" fomoxa protobuf-net protobuf-tcp <<'EOF'
import json, sys

out_dir, names = sys.argv[1], sys.argv[2:]
rows = []
for name in names:
    with open(f"{out_dir}/{name}.json") as handle:
        rows.append((name, json.load(handle)))

header = ["stack", "joined", "p50", "p95", "p99", "snapshot/s", "nhận được", "payload", "snapshot", "CPU server", "RSS server"]
print()
print("| " + " | ".join(header) + " |")
print("|" + "|".join(["---"] * len(header)) + "|")
for name, row in rows:
    latency = row["latency_ms"]
    print(
        f'| {name} | {row["joined"]}/{row["bots"]} '
        f'| {latency["p50"]:.1f} ms | {latency["p95"]:.1f} ms | {latency["p99"]:.1f} ms '
        f'| {row["snapshots_per_second"]:.0f} | {row["delivered_percent"]:.0f}% '
        f'| {row["payload_mib_per_second"]:.0f} MiB/s | {row["snapshot_bytes"]:.0f} B '
        f'| {row["server_cpu_cores_mean"]:.2f} | {row["server_rss_mib_peak"]:.0f} MiB |'
    )
EOF
