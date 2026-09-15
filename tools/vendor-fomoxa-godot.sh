#!/usr/bin/env bash
set -euo pipefail

repository="${FOMOXA_GODOT_REPOSITORY:-https://github.com/fomoxa/godot}"
ref="${1:-main}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="$repo_root/clients/godot/addons/fomoxa"

usage() {
    cat >&2 <<EOF
usage: tools/vendor-fomoxa-godot.sh [ref]

  git clone $repository at [ref] (branch, tag or commit; default: main)
  and place the addon Godot loads into clients/godot:

    addons/fomoxa/ -> ${target_dir#"$repo_root"/}/
    LICENSE        -> ${target_dir#"$repo_root"/}/LICENSE

  the fetched commit is recorded in ${target_dir#"$repo_root"/}/VERSION
  override the repository with FOMOXA_GODOT_REPOSITORY=<url>
EOF
    exit 2
}

[[ "$ref" == "-h" || "$ref" == "--help" ]] && usage

checkout="$(mktemp -d)"
trap 'rm -rf "$checkout"' EXIT

git -C "$checkout" init --quiet
git -C "$checkout" remote add origin "$repository"
git -C "$checkout" fetch --quiet --depth 1 origin "$ref"
git -C "$checkout" checkout --quiet FETCH_HEAD
commit="$(git -C "$checkout" rev-parse HEAD)"

rm -rf "$target_dir"
mkdir -p "$(dirname "$target_dir")"
cp -r "$checkout/addons/fomoxa" "$target_dir"
cp "$checkout/LICENSE" "$target_dir/"

cat > "$target_dir/VERSION" <<EOF
repository $repository
ref $ref
commit $commit
EOF

echo "fomoxa/godot $ref ($commit) -> ${target_dir#"$repo_root"/}"
