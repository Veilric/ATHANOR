#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
python3 scripts/cargo-local.py test --workspace --locked
python3 scripts/cargo-local.py build --release --locked -p athanor --bin athanor
if [ "$(uname -s)" = Linux ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
    printf '%s\n' 'Launch target/release/athanor first, then run python3 scripts/check-native-x11.py.'
fi
printf '%s\n' 'Automated checks passed. Native acceptance requires docs/verification/native-checklist.md; unchecked observations remain unverified.'
