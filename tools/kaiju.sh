#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
client="$repo_root/clients/kaiju"
engine="${KAIJU_ENGINE:-${XDG_CACHE_HOME:-$HOME/.cache}/kaiju-engine}"
repository="${KAIJU_REPOSITORY:-https://github.com/KaijuEngine/kaiju}"
staged="$engine/src/fomoxa_example"
game_file="$engine/src/fomoxa_example_game.go"
binary="$engine/bin/fomoxa-example-kaiju"
sysroot="${KAIJU_SYSROOT:-}"

usage() {
    cat >&2 <<EOF
usage: tools/kaiju.sh <command> [arguments]

  check            go vet the engine-independent packages and compare fingerprints
  smoke [args]     build and run cmd/smoke against a running server
                   (-addr, -name, -seconds, -expect-players)
  setup [ref]      git clone $repository (default ref: master) into
                   $engine and record the commit in its VERSION file
  stage            copy clients/kaiju into the engine checkout
  build            stage, then build the game binary with -tags kaiju
  run [args]       run the built binary from the engine directory

  build and run need the engine's C dependencies: X11, ALSA and a libvulkan.so
  (the engine dlopens that exact name). on Debian/Ubuntu:
    sudo apt install libx11-dev libxcursor-dev libxrandr-dev libasound2-dev libvulkan-dev
  without root, extract those packages somewhere and point KAIJU_SYSROOT=<dir>
  at the prefix holding usr/include and usr/lib/x86_64-linux-gnu

  the engine checkout is not part of this repository (about 50 MB of engine
  source plus prebuilt libraries). override its location with KAIJU_ENGINE=<dir>
EOF
    exit 2
}

require_engine() {
    if [[ ! -d "$engine/src" ]]; then
        echo "tools/kaiju.sh: no Kaiju engine at $engine, run tools/kaiju.sh setup" >&2
        exit 1
    fi
}

command_check() {
    (cd "$client" && go vet ./network/... ./cmd/smoke/...)
    "$repo_root/tools/check-fingerprints.sh"
}

command_smoke() {
    (cd "$client" && go run ./cmd/smoke "$@")
}

command_setup() {
    local ref="${1:-master}"
    mkdir -p "$(dirname "$engine")"
    if [[ -d "$engine/.git" ]]; then
        git -C "$engine" fetch --quiet --depth 1 origin "$ref"
    else
        git init --quiet "$engine"
        git -C "$engine" remote add origin "$repository"
        git -C "$engine" fetch --quiet --depth 1 origin "$ref"
    fi
    git -C "$engine" checkout --quiet FETCH_HEAD
    git -C "$engine" submodule update --init --depth 1 --recursive src/libs

    cat > "$engine/VERSION" <<EOF
repository $repository
ref $ref
commit $(git -C "$engine" rev-parse HEAD)
EOF
    echo "kaiju engine $ref ($(git -C "$engine" rev-parse HEAD)) -> $engine"
}

command_stage() {
    require_engine
    rm -rf "$staged"
    mkdir -p "$staged"
    cp -r "$client/network" "$staged/"
    cp "$client/cmd/kaijugame/game.go" "$game_file"
    rm -f "$engine/src/main.test.go"

    local runtime
    runtime="$(cd "$client" && go list -m -f '{{.Path}}@{{.Version}}' github.com/fomoxa/go)"
    (cd "$engine/src" && go get "$runtime" >/dev/null && go mod tidy >/dev/null)
    echo "staged clients/kaiju into $engine/src (fomoxa runtime $runtime)"
}

command_build() {
    command_stage
    mkdir -p "$engine/bin"
    if [[ -n "$sysroot" ]]; then
        (cd "$engine/src" &&
            CGO_CFLAGS="-I$sysroot/usr/include" \
            CGO_LDFLAGS="-L$sysroot/usr/lib/x86_64-linux-gnu -Wl,-rpath-link,$sysroot/usr/lib/x86_64-linux-gnu" \
            go build -tags kaiju -o "$binary" .)
    else
        (cd "$engine/src" && go build -tags kaiju -o "$binary" .)
    fi
    echo "built $binary"
}

command_run() {
    require_engine
    if [[ ! -x "$binary" ]]; then
        echo "tools/kaiju.sh: $binary is missing, run tools/kaiju.sh build" >&2
        exit 1
    fi
    if [[ -n "$sysroot" ]]; then
        (cd "$engine" && LD_LIBRARY_PATH="$sysroot/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" "$binary" "$@")
    else
        (cd "$engine" && "$binary" "$@")
    fi
}

case "${1:-}" in
    check) shift && command_check "$@" ;;
    smoke) shift && command_smoke "$@" ;;
    setup) shift && command_setup "$@" ;;
    stage) shift && command_stage "$@" ;;
    build) shift && command_build "$@" ;;
    run) shift && command_run "$@" ;;
    *) usage ;;
esac
