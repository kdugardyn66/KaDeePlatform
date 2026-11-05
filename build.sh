#!/usr/bin/env bash
# Build KaDeePlatform for the OS you are on. Options are passed to the platform script:
#   ./build.sh            macOS: .app + .dmg      Linux: .tar.gz (+ .deb/.rpm/AppImage)
#   ./build.sh --run      build and start
#   ./build.sh --help     options of the platform script
# Windows: .\scripts\build-windows.ps1  (PowerShell)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
case "$(uname -s)" in
  Darwin) exec ./scripts/build-macos.sh "$@" ;;
  Linux) exec ./scripts/build-linux.sh "$@" ;;
  MINGW*|MSYS*|CYGWIN*) exec powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1 "$@" ;;
  *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac
