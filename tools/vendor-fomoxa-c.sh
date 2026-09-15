#!/usr/bin/env bash
set -euo pipefail

repository="${FOMOXA_C_REPOSITORY:-https://github.com/fomoxa/c}"
ref="${1:-main}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
module_dir="$repo_root/clients/unreal/Source/FomoxaExample"
target_dir="$module_dir/ThirdParty/fomoxa"
annotation_header="$module_dir/Protocol/fomoxa.h"

usage() {
    cat >&2 <<EOF
usage: tools/vendor-fomoxa-c.sh [ref]

  git clone $repository at [ref] (branch, tag or commit; default: main)
  and place the runtime Unreal compiles into clients/unreal:

    include/fomoxa/  -> ${target_dir#"$repo_root"/}/include/fomoxa/
    src/             -> ${target_dir#"$repo_root"/}/src/
    LICENSE          -> ${target_dir#"$repo_root"/}/LICENSE
    include/fomoxa.h -> ${annotation_header#"$repo_root"/}

  the fetched commit is recorded in ${target_dir#"$repo_root"/}/VERSION
  override the repository with FOMOXA_C_REPOSITORY=<url>
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
mkdir -p "$target_dir/include"
cp -r "$checkout/include/fomoxa" "$target_dir/include/"
cp -r "$checkout/src" "$target_dir/"
cp "$checkout/LICENSE" "$target_dir/"
cp "$checkout/include/fomoxa.h" "$annotation_header"

cat > "$target_dir/VERSION" <<EOF
repository $repository
ref $ref
commit $commit
EOF

echo "fomoxa/c $ref ($commit) -> ${target_dir#"$repo_root"/}"
