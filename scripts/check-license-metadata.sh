#!/usr/bin/env bash
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"

if rg -q '^MIT License[[:space:]]*$' LICENSE; then
    expected=MIT
elif rg -q '^[[:space:]]*Apache License[[:space:]]*$' LICENSE; then
    expected=Apache-2.0
elif rg -q '^[[:space:]]*GNU GENERAL PUBLIC LICENSE[[:space:]]*$' LICENSE \
    && rg -q '^[[:space:]]*Version 3, 29 June 2007[[:space:]]*$' LICENSE; then
    # Current project declaration is GPLv3-only; embedded third-party notices
    # may grant different terms. A bare license text does not grant "or later".
    expected=GPL-3.0-only
else
    printf '%s\n' 'Unrecognized root LICENSE; review metadata policy explicitly.' >&2
    exit 1
fi

metadata=$(cargo metadata --locked --offline --no-deps --format-version 1)
if ! jq -e --arg expected "$expected" '
    .workspace_members as $members |
    [.packages[] | select(.id as $id | $members | index($id))] as $packages |
    ($packages | length > 0) and all($packages[]; .license == $expected)
' <<< "$metadata" >/dev/null; then
    printf 'Workspace package licenses must match current project declaration %s:\n' "$expected" >&2
    jq -r '.workspace_members as $members | .packages[] |
        select(.id as $id | $members | index($id)) |
        "  \(.name): \(.license // "<missing>")"' <<< "$metadata" >&2
    exit 1
fi
printf 'Workspace license metadata matches %s.\n' "$expected"
printf '%s\n' 'This is metadata consistency, not dependency/provenance or distribution clearance.'
