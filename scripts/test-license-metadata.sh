#!/usr/bin/env bash
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT
mkdir -p "$temp/scripts" "$temp/a/src" "$temp/b/src"
cp "$repo/scripts/check-license-metadata.sh" "$temp/scripts/"
printf '%s\n' 'pub fn marker() {}' > "$temp/a/src/lib.rs"
printf '%s\n' 'pub fn marker() {}' > "$temp/b/src/lib.rs"

workspace() {
    printf '%s\n' '[workspace]' 'members = ["a", "b"]' 'resolver = "2"' \
        '[workspace.package]' "license = \"$1\"" > "$temp/Cargo.toml"
    printf '%s\n' '[package]' 'name = "a"' 'version = "0.1.0"' \
        'edition = "2021"' 'license.workspace = true' > "$temp/a/Cargo.toml"
    printf '%s\n' '[package]' 'name = "b"' 'version = "0.1.0"' \
        'edition = "2021"' "$2" > "$temp/b/Cargo.toml"
    (cd "$temp" && cargo generate-lockfile --offline --quiet)
}

# Coherent current state passes; a single stale Apache declaration must fail.
# Signature-only parser fixtures, not licenses for any distributed code.
printf '%s\n' 'GNU GENERAL PUBLIC LICENSE' 'Version 3, 29 June 2007' > "$temp/LICENSE"
workspace GPL-3.0-only 'license.workspace = true'
bash "$temp/scripts/check-license-metadata.sh" > "$temp/output" 2>&1
workspace GPL-3.0-only 'license = "Apache-2.0"'
if bash "$temp/scripts/check-license-metadata.sh" > "$temp/output" 2>&1; then
    printf '%s\n' 'FAILED: stale member license passed' >&2
    exit 1
fi
rg -q 'b: Apache-2.0' "$temp/output"

# A future root-license change cannot leave stale package declarations behind.
printf '%s\n' 'MIT License' > "$temp/LICENSE"
workspace GPL-3.0-only 'license.workspace = true'
if bash "$temp/scripts/check-license-metadata.sh" > "$temp/output" 2>&1; then
    printf '%s\n' 'FAILED: root/member mismatch passed' >&2
    exit 1
fi
workspace MIT 'license.workspace = true'
bash "$temp/scripts/check-license-metadata.sh" > "$temp/output" 2>&1

printf '%s\n' 'License metadata regression probes passed (consistent GPL, stale member, root mismatch, consistent MIT fixture).'
