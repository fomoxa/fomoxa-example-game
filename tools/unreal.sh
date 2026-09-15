#!/usr/bin/env bash
set -euo pipefail

UE_ROOT="${UE_ROOT:-C:\\Program Files\\Epic Games\\UE_5.8}"
WINDOWS_PROJECT_DIR="${FOMOXA_EXAMPLE_UNREAL_PROJECT:-D:\\code\\fomoxa-example-game\\clients\\unreal}"
UNREAL_TIMEOUT_SECONDS="${UNREAL_TIMEOUT_SECONDS:-1800}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-180}"
SMOKE_MARKER="-FomoxaExampleSmoke"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
uproject="$WINDOWS_PROJECT_DIR\\FomoxaExample.uproject"
engine_dir="$(wslpath -u "$UE_ROOT")/Engine"
editor="$engine_dir/Binaries/Win64/UnrealEditor.exe"
editor_cmd="$engine_dir/Binaries/Win64/UnrealEditor-Cmd.exe"
log_dir="$repo_root/clients/unreal/Saved/Logs"

usage() {
    cat >&2 <<EOF
usage: tools/unreal.sh <command> [unreal arguments]

  the project is $uproject, the Windows side of the clients/ bind mount

  build    compile FomoxaExampleEditor Win64 Development
  map      rebuild /Game/Maps/FomoxaExample with Scripts/create_map.py
  smoke    run the game with -nullrhi against a running server until the first snapshot
           e.g. tools/unreal.sh smoke -host=127.0.0.1 -name=unreal-smoke
  open     open the project in the Unreal editor
EOF
    exit 2
}

stop_smoke_game() {
    powershell.exe -NoProfile -Command "Get-CimInstance Win32_Process -Filter \"Name='UnrealEditor-Cmd.exe'\" | Where-Object { \$_.CommandLine -like '*$SMOKE_MARKER*' } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force }" >/dev/null 2>&1 || true
}

command="${1:-}"
[[ -n "$command" ]] || usage
shift
mkdir -p "$log_dir"

case "$command" in
    build)
        log="$log_dir/unreal-build.log"
        status=0
        (cd /mnt/c && timeout "$UNREAL_TIMEOUT_SECONDS" powershell.exe -NoProfile -Command "& '$UE_ROOT\\Engine\\Build\\BatchFiles\\Build.bat' FomoxaExampleEditor Win64 Development '-Project=$uproject' -WaitMutex $*; exit \$LASTEXITCODE") > "$log" 2>&1 || status=$?
        echo "=== exit code: $status ==="
        grep -n "error\|Result:\|Total execution time" "$log" | tr -d '\r' | head -60 || true
        echo "=== full log: $log ==="
        exit "$status"
        ;;
    map)
        log="$log_dir/unreal-map.log"
        status=0
        timeout "$UNREAL_TIMEOUT_SECONDS" "$editor_cmd" "$uproject" -run=pythonscript "-script=$WINDOWS_PROJECT_DIR\\Scripts\\create_map.py" -unattended -nop4 -nosplash -stdout -FullStdOutLogOutput "$@" > "$log" 2>&1 || status=$?
        echo "=== exit code: $status ==="
        grep -n "FOMOXA-EXAMPLE-MAP\|Traceback\|RuntimeError\|Error:" "$log" | tr -d '\r' | head -40 || echo "no result line - see full log"
        echo "=== full log: $log ==="
        exit "$status"
        ;;
    smoke)
        log="$log_dir/unreal-smoke.log"
        stop_smoke_game
        "$editor_cmd" "$uproject" -game -nullrhi -nosound -unattended -nosplash -stdout -FullStdOutLogOutput "$SMOKE_MARKER" "$@" > "$log" 2>&1 &
        game=$!
        result="no snapshot within ${SMOKE_TIMEOUT_SECONDS}s"
        for (( waited = 0; waited < SMOKE_TIMEOUT_SECONDS; waited++ )); do
            if grep -q "LogFomoxaExample: first snapshot" "$log"; then
                result=""
                break
            fi
            if grep -q "LogFomoxaExample: Warning: failed" "$log"; then
                result="session failed"
                break
            fi
            if ! kill -0 "$game" 2>/dev/null; then
                result="game exited"
                break
            fi
            sleep 1
        done
        stop_smoke_game
        wait "$game" 2>/dev/null || true
        grep -n "LogFomoxaExample" "$log" | tr -d '\r' || true
        echo "=== full log: $log ==="
        if [[ -n "$result" ]]; then
            echo "UNREAL-SMOKE: FAIL - $result"
            exit 1
        fi
        echo "UNREAL-SMOKE: PASS"
        ;;
    open)
        "$editor" "$uproject" "$@" > /dev/null 2>&1 &
        disown
        echo "opening $uproject"
        ;;
    *)
        usage
        ;;
esac
