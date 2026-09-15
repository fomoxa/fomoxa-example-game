#!/usr/bin/env bash
set -euo pipefail

UNITY_VERSION="${UNITY_VERSION:-6000.5.7f1}"
WINDOWS_PROJECT="${FOMOXA_EXAMPLE_UNITY_PROJECT:-D:\\code\\fomoxa-example-game\\clients\\unity}"
UNITY_TIMEOUT_SECONDS="${UNITY_TIMEOUT_SECONDS:-900}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
unity="/mnt/c/Program Files/Unity/Hub/Editor/$UNITY_VERSION/Editor/Unity.exe"
log_dir="$repo_root/clients/unity/Logs"

usage() {
    cat >&2 <<EOF
usage: tools/unity.sh <command> [unity arguments]

  the project is $WINDOWS_PROJECT, the Windows side of the clients/ bind mount

  check    compile the project in batchmode
  scene    rebuild Assets/FomoxaExample/Scenes/Main.unity with FomoxaExample.EditorTools.FomoxaExampleSceneBuilder
  smoke    run FomoxaExample.EditorTools.FomoxaExampleSmokeTest against a running server
           e.g. tools/unity.sh smoke --host=127.0.0.1 --seconds=4 --expect-players=2
  open     open the project in the Unity editor
EOF
    exit 2
}

stop_batchmode_unity() {
    local pattern="*${WINDOWS_PROJECT}*"
    powershell.exe -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='Unity.exe'\" | Where-Object { \$_.CommandLine -like '*-batchmode*' -and \$_.CommandLine -like '$pattern' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1 || true
}

run_batchmode() {
    local log="$1"
    shift
    mkdir -p "$log_dir"
    stop_batchmode_unity
    set +e
    timeout "$UNITY_TIMEOUT_SECONDS" "$unity" -batchmode -nographics -projectPath "$WINDOWS_PROJECT" -logFile - "$@" > "$log" 2>&1
    local status=$?
    set -e
    if [[ "$status" -eq 124 ]]; then
        stop_batchmode_unity
        echo "unity timed out after ${UNITY_TIMEOUT_SECONDS}s" >&2
    fi
    return "$status"
}

command="${1:-}"
[[ -n "$command" ]] || usage
shift

case "$command" in
    check)
        log="$log_dir/unity-check.log"
        status=0
        run_batchmode "$log" -quit || status=$?
        echo "=== exit code: $status ==="
        grep -n "error CS\|Scripts have compiler errors\|Aborting batchmode\|Fatal Error" "$log" || echo "no compiler errors"
        echo "=== full log: $log ==="
        exit "$status"
        ;;
    scene)
        log="$log_dir/unity-scene.log"
        status=0
        run_batchmode "$log" -quit -executeMethod FomoxaExample.EditorTools.FomoxaExampleSceneBuilder.Build "$@" || status=$?
        echo "=== exit code: $status ==="
        grep -n "FOMOXA-EXAMPLE-SCENE\|error CS\|Exception\|Fatal Error" "$log" || echo "no result line - see full log"
        echo "=== full log: $log ==="
        exit "$status"
        ;;
    smoke)
        log="$log_dir/unity-smoke.log"
        status=0
        run_batchmode "$log" -executeMethod FomoxaExample.EditorTools.FomoxaExampleSmokeTest.Run "$@" || status=$?
        echo "=== exit code: $status ==="
        grep -n "FOMOXA-EXAMPLE-SMOKE\|error CS\|Fatal Error" "$log" || echo "no result line - see full log"
        echo "=== full log: $log ==="
        exit "$status"
        ;;
    open)
        "$unity" -projectPath "$WINDOWS_PROJECT" "$@" > /dev/null 2>&1 &
        disown
        echo "opening $WINDOWS_PROJECT in Unity $UNITY_VERSION"
        ;;
    *)
        usage
        ;;
esac
