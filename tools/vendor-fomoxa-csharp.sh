#!/usr/bin/env bash
set -euo pipefail

repository="${FOMOXA_CSHARP_REPOSITORY:-https://github.com/fomoxa/csharp}"
ref="${1:-main}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plugins_dir="$repo_root/clients/unity/Assets/Plugins"
assembly="$plugins_dir/Fomoxa.Net.dll"
version_file="$plugins_dir/Fomoxa.Net.VERSION.txt"

usage() {
    cat >&2 <<EOF
usage: tools/vendor-fomoxa-csharp.sh [ref]

  git clone $repository at [ref] (branch, tag or commit; default: main),
  build Fomoxa.Net for netstandard2.1 and place the assembly Unity loads:

    src/Fomoxa.Net -> ${assembly#"$repo_root"/}

  the fetched commit is recorded in ${version_file#"$repo_root"/}
  override the repository with FOMOXA_CSHARP_REPOSITORY=<url>
  needs the .NET SDK; this WSL has no libicu, so the build runs with
  DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
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

export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT="${DOTNET_SYSTEM_GLOBALIZATION_INVARIANT:-1}"
dotnet build "$checkout/src/Fomoxa.Net/Fomoxa.Net.csproj" --configuration Release --nologo --verbosity quiet

mkdir -p "$plugins_dir"
cp "$checkout/src/Fomoxa.Net/bin/Release/netstandard2.1/Fomoxa.Net.dll" "$assembly"

cat > "$version_file" <<EOF
repository $repository
ref $ref
commit $commit
EOF

echo "fomoxa/csharp $ref ($commit) -> ${assembly#"$repo_root"/}"
