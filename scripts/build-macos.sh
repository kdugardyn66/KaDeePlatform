#!/usr/bin/env bash
# Build the KaDee Platform (KadeeStudio) for macOS: a release binary, KaDeePlatform.app and a .dmg in dist/.
#
#   ./scripts/build-macos.sh                 # this Mac's architecture → .app + .dmg
#   ./scripts/build-macos.sh --run           # build the .app and open it
#   ./scripts/build-macos.sh --universal     # Apple silicon + Intel in one binary
#   ./scripts/build-macos.sh --no-dmg        # .app only
#   ./scripts/build-macos.sh --clean         # cargo clean first
#
# GPUI compiles its Metal shaders with `xcrun metal`. When that compiler is missing (Command
# Line Tools only, or Xcode 26 without the Metal Toolchain: xcodebuild -downloadComponent
# MetalToolchain) the build switches to GPUI's runtime-compiled shaders by itself.
# RUNTIME_SHADERS=1 forces that, RUNTIME_SHADERS=0 forbids it.
#
# The .dmg is made with `diskutil image` on macOS 26 and newer (hdiutil is deprecated there)
# and with hdiutil on older systems. Finder lays out its window with a background; allow
# Terminal to control Finder when asked, or set DMG_LAYOUT=0 (headless/CI, no background).
#   DMG_BACKGROUND=my.png   another background (640x400; my@2x.png beside it for Retina)
#
# Signing (optional; without it the app is ad-hoc signed and runs on this Mac):
#   APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
# Notarization (optional; needs the identity above and a stored notarytool profile):
#   xcrun notarytool store-credentials kadee-notary --apple-id you@example.com --team-id TEAMID
#   APPLE_NOTARY_PROFILE=kadee-notary
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APP_NAME="KaDeePlatform"   # the .app, the volume and the .dmg
BIN_NAME="kadee-studio"    # cargo package and binary
BUNDLE_ID="be.kadee.KaDeePlatform"
DISPLAY_NAME="KadeeStudio"
# GPUI needs macOS 10.15.7; the Metal features it uses are there from 11.
MIN_MACOS="11.0"
DIST="$ROOT/dist"

UNIVERSAL=0 RUN=0 DMG=1 CLEAN=0
for arg in "$@"; do
  case "$arg" in
    --universal) UNIVERSAL=1 ;;
    --native) UNIVERSAL=0 ;;
    --run) RUN=1 ;;
    --no-dmg) DMG=0 ;;
    --clean) CLEAN=1 ;;
    -h|--help) sed -n '2,28p' "$0"; exit 0 ;;
    *) echo "Unknown option: $arg (see --help)" >&2; exit 2 ;;
  esac
done

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == "Darwin" ]] || die "this script runs on macOS"
command -v cargo >/dev/null || die "Rust is not installed: https://rustup.rs"
command -v xcrun >/dev/null || die "Xcode command line tools are missing: xcode-select --install"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
[[ -n "$VERSION" ]] || die "could not read the version from Cargo.toml"
export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS"

if [[ $CLEAN -eq 1 ]]; then step "cargo clean"; cargo clean; fi

FEATURES=()
case "${RUNTIME_SHADERS:-auto}" in
  1) FEATURES=(--features runtime-shaders) ;;
  0) xcrun -f metal >/dev/null 2>&1 || die "the Metal shader compiler is missing (xcodebuild -downloadComponent MetalToolchain)" ;;
  *) if ! xcrun -f metal >/dev/null 2>&1 || ! xcrun metal --version >/dev/null 2>&1; then
       echo "The Metal shader compiler is not installed: GPUI compiles its shaders when the app starts."
       FEATURES=(--features runtime-shaders)
     fi ;;
esac

# ---------------------------------------------------------------- build
if [[ $UNIVERSAL -eq 1 ]]; then
  ARCH_LABEL="universal"
  TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
  if command -v rustup >/dev/null; then rustup target add "${TARGETS[@]}" >/dev/null; fi
  for t in "${TARGETS[@]}"; do
    step "Building $t (release)"
    cargo build --release --locked -p "$BIN_NAME" --target "$t" ${FEATURES[@]+"${FEATURES[@]}"}
  done
  mkdir -p target/universal-apple-darwin/release
  BIN="target/universal-apple-darwin/release/$BIN_NAME"
  lipo -create -output "$BIN" \
    "target/aarch64-apple-darwin/release/$BIN_NAME" \
    "target/x86_64-apple-darwin/release/$BIN_NAME"
else
  case "$(uname -m)" in arm64) ARCH_LABEL="arm64" ;; *) ARCH_LABEL="x86_64" ;; esac
  step "Building for $ARCH_LABEL (release)"
  cargo build --release --locked -p "$BIN_NAME" ${FEATURES[@]+"${FEATURES[@]}"}
  BIN="target/release/$BIN_NAME"
fi
lipo -info "$BIN"

# ---------------------------------------------------------------- .app
step "Assembling $APP_NAME.app"
APP="$DIST/$APP_NAME.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"

# Icon: rebuild the .icns from the 1024 px PNG when iconutil is available,
# otherwise use the checked-in .icns.
ICONS="$ROOT/assets/app-icon"
if command -v iconutil >/dev/null && command -v sips >/dev/null; then
  ISET="$(mktemp -d)/$APP_NAME.iconset"
  mkdir -p "$ISET"
  for s in 16 32 128 256 512; do
    sips -z $s $s "$ICONS/kadeestudio-1024.png" --out "$ISET/icon_${s}x${s}.png" >/dev/null
    sips -z $((s * 2)) $((s * 2)) "$ICONS/kadeestudio-1024.png" --out "$ISET/icon_${s}x${s}@2x.png" >/dev/null
  done
  iconutil -c icns "$ISET" -o "$APP/Contents/Resources/$APP_NAME.icns"
else
  cp "$ICONS/KadeeStudio.icns" "$APP/Contents/Resources/$APP_NAME.icns"
fi

YEAR="$(date +%Y)"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$DISPLAY_NAME</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key><string>$BIN_NAME</string>
  <key>CFBundleIconFile</key><string>$APP_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>LSMinimumSystemVersion</key><string>$MIN_MACOS</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSHumanReadableCopyright</key><string>© $YEAR Kris Dugardyn. MIT OR Apache-2.0.</string>
</dict>
</plist>
PLIST
printf 'APPL????' > "$APP/Contents/PkgInfo"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# ---------------------------------------------------------------- sign
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  step "Signing with: $APPLE_SIGNING_IDENTITY"
  codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$APP"
else
  step "Ad-hoc signing (runs on this Mac; set APPLE_SIGNING_IDENTITY to distribute)"
  codesign --force --sign - "$APP"
fi
codesign --verify --strict --verbose=1 "$APP"

# ---------------------------------------------------------------- dmg
# Same flow as mdcraft's packaging/macos/package.sh: a writable image, the app plus an
# Applications shortcut and a background laid out by Finder, then compressed to UDZO.
# `diskutil image` (macOS 26+) replaces the deprecated hdiutil; older systems use hdiutil.
DEVICE=""
WORK=""
eject_device() { # eject, retrying while Spotlight or a scanner still holds the volume
  local dev="$1"
  for _ in 1 2 3 4 5; do
    diskutil eject "$dev" >/dev/null 2>&1 && return 0
    echo "  the disk image is still busy, retrying in 2 s…" >&2
    sleep 2
  done
  diskutil unmountDisk force "$dev" >/dev/null 2>&1 || true
  diskutil eject "$dev" >/dev/null 2>&1
}
cleanup_dmg() {
  if [[ -n "$DEVICE" ]]; then eject_device "$DEVICE" || true; fi
  if [[ -n "$WORK" ]]; then rm -rf "$WORK"; fi
}

if [[ $DMG -eq 1 ]]; then
  DMG_PATH="$DIST/$APP_NAME-$VERSION-macos-$ARCH_LABEL.dmg"
  VOL="$APP_NAME"
  BG="${DMG_BACKGROUND:-$ROOT/packaging/macos/dmg-background.png}" # 640x400
  BG2X="${BG%.png}@2x.png"                                         # optional 1280x800 for Retina
  WIN_W=640 WIN_H=400
  MACOS_MAJOR="$(sw_vers -productVersion | cut -d. -f1)"
  if [[ "$MACOS_MAJOR" -ge 26 ]]; then IMG=diskutil; else IMG=hdiutil; fi
  step "Creating $(basename "$DMG_PATH") with $IMG"

  trap cleanup_dmg EXIT
  WORK="$(mktemp -d "${TMPDIR:-/tmp}/kadee-platform-dmg.XXXXXX")"
  RW="$WORK/$APP_NAME-rw.dmg"

  # 0. A $VOL volume that is still mounted would make this one "$VOL 1",
  #    and Finder saves the window layout per volume name: eject it first.
  for vol in "/Volumes/$VOL" "/Volumes/$VOL "[0-9]*; do
    if [[ -d "$vol" ]]; then
      echo "Ejecting $vol"
      diskutil eject "$vol" >/dev/null 2>&1 || diskutil unmount force "$vol" >/dev/null 2>&1 || true
    fi
  done

  # 1. A blank writable image: the app plus headroom.
  APP_MB="$(du -sm "$APP" | awk '{print $1}')"
  SIZE_MB=$((APP_MB + APP_MB / 5 + 20))
  if [[ "$IMG" == diskutil ]]; then
    diskutil image create blank --size "${SIZE_MB}m" --volumeName "$VOL" "$RW" >/dev/null
  else
    hdiutil create -quiet -size "${SIZE_MB}m" -fs HFS+ -volname "$VOL" "$RW" 2>&1 | grep -v "deprecated" || true
  fi
  [[ -f "$RW" ]] || die "could not create the disk image"

  # 2. Attach it browsable (Finder has to see it to save the window layout).
  if [[ "$IMG" == diskutil ]]; then
    ATTACH_OUT="$(diskutil image attach "$RW")"
  else
    ATTACH_OUT="$(hdiutil attach "$RW" -noautoopen -noverify)"
  fi
  DEVICE="$(echo "$ATTACH_OUT" | grep -E '^/dev/disk' | head -1 | awk '{print $1}' || true)"
  [[ -n "$DEVICE" ]] || die "could not attach the disk image: $ATTACH_OUT"
  # The mount point is in the attach output; an APFS image mounts on a separate container
  # disk, so don't look it up by $DEVICE. Otherwise wait for /Volumes/$VOL.
  MNT="$(echo "$ATTACH_OUT" | grep -o '/Volumes/.*$' | sed -E 's/[[:space:]]+$//' | head -1 || true)"
  if [[ -z "$MNT" ]]; then
    for _ in $(seq 1 20); do [[ -d "/Volumes/$VOL" ]] && break; sleep 0.5; done
    MNT="/Volumes/$VOL"
  fi
  [[ -d "$MNT" ]] || die "the disk image did not mount (expected $MNT): $ATTACH_OUT"
  [[ "$(basename "$MNT")" == "$VOL" ]] || die "the image mounted as $MNT, not /Volumes/$VOL (eject the other $VOL volume)"
  mdutil -i off "$MNT" >/dev/null 2>&1 || true

  # 3. Contents: the app, an Applications shortcut, the background in a hidden folder.
  ditto "$APP" "$MNT/$APP_NAME.app"
  ln -s /Applications "$MNT/Applications"
  BG_FILE=""
  if [[ "${DMG_LAYOUT:-1}" != 0 ]]; then
    [[ -f "$BG" ]] || die "no background image at $BG (set DMG_BACKGROUND, or DMG_LAYOUT=0)"
    mkdir "$MNT/.background"
    if [[ -f "$BG2X" ]] && command -v tiffutil >/dev/null; then
      # one file with both sizes: Finder picks the sharp one on Retina screens
      tiffutil -cathidpicheck "$BG" "$BG2X" -out "$MNT/.background/background.tiff" >/dev/null 2>&1
      BG_FILE="background.tiff"
    else
      cp "$BG" "$MNT/.background/background.png"
      BG_FILE="background.png"
    fi
  fi

  # 4. Lay out the window with Finder; it saves the layout in .DS_Store on the volume.
  #    Finder asks once for permission (System Settings → Privacy & Security → Automation).
  if [[ -n "$BG_FILE" ]]; then
    echo "Laying out the window in Finder…"
    if ! osascript <<APPLESCRIPT
tell application "Finder"
  tell disk "$VOL"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {100, 100, $((100 + WIN_W)), $((100 + WIN_H))}
    set opts to the icon view options of container window
    set arrangement of opts to not arranged
    set icon size of opts to 128
    set text size of opts to 13
    set background picture of opts to file ".background:$BG_FILE"
    set position of item "$APP_NAME.app" of container window to {160, 200}
    set position of item "Applications" of container window to {480, 200}
    update without registering applications
    delay 2
    close
  end tell
end tell
APPLESCRIPT
    then
      die "Finder could not lay out the window. Allow Terminal to control Finder in System Settings → Privacy & Security → Automation, then run again (or DMG_LAYOUT=0 for no background)."
    fi
    for _ in $(seq 1 20); do [[ -f "$MNT/.DS_Store" ]] && break; sleep 0.5; done
    [[ -f "$MNT/.DS_Store" ]] || echo "warning: Finder did not save the window layout; the background may not show" >&2
  fi
  chmod -Rf go-w "$MNT" 2>/dev/null || true
  sync

  # 5. Eject (there is no `diskutil image detach`), then compress to a read-only .dmg.
  eject_device "$DEVICE" || die "could not eject $DEVICE"
  DEVICE=""
  rm -f "$DMG_PATH"
  if [[ "$IMG" == diskutil ]]; then
    diskutil image create from "$RW" "$DMG_PATH" --format UDZO >/dev/null
  else
    hdiutil convert "$RW" -quiet -format UDZO -imagekey zlib-level=9 -o "$DMG_PATH"
  fi
  [[ -f "$DMG_PATH" ]] || die "could not compress $DMG_PATH"
  cleanup_dmg
  WORK=""
  trap - EXIT

  if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$DMG_PATH"
  fi
  if [[ -n "${APPLE_NOTARY_PROFILE:-}" ]]; then
    [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]] || die "notarization needs APPLE_SIGNING_IDENTITY"
    step "Notarizing (this can take a few minutes)"
    xcrun notarytool submit "$DMG_PATH" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG_PATH"
    xcrun stapler staple "$APP"
  fi
  (cd "$DIST" && shasum -a 256 "$(basename "$DMG_PATH")" > "$(basename "$DMG_PATH").sha256")
fi

step "Done"
echo "  App: $APP"
[[ $DMG -eq 1 ]] && echo "  DMG: $DMG_PATH"

if [[ $RUN -eq 1 ]]; then
  open "$APP"
fi
