#!/usr/bin/env bash
# Release-locked Fang package installer. The complete implementation is built
# test-first in the following installer tasks.

main() {
set -euo pipefail
readonly VERSION='0.9.4'
readonly RELEASE_TAG='v0.9.4'
printf 'Fang %s installer is not ready yet.\n' "$VERSION" >&2
return 1
}

main "$@"
