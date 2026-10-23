#!/usr/bin/env bash
# Exercise rejection paths with real Mach-O helpers; run on both Mac targets.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_path="${1:?usage: test_verify_macos_bundle.sh App.app arm64|x86_64}"
arch="${2:?missing architecture}"
case "$arch" in
  arm64) other_arch=x86_64 ;;
  x86_64) other_arch=arm64 ;;
  *) echo 'unsupported architecture' >&2; exit 1 ;;
esac
verifier="$repo_root/scripts/verify_macos_bundle.sh"
bash "$verifier" "$app_path" "$arch"

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT
fixture="$fixture_root/Negative.app"
mkdir -p "$fixture/Contents/MacOS" "$fixture/Contents/Resources"
cp "$app_path/Contents/Info.plist" "$fixture/Contents/Info.plist"
name="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$fixture/Contents/Info.plist")"
cp "$app_path/Contents/MacOS/$name" "$fixture/Contents/MacOS/$name"

expect_rejected() {
  local expected="$1"
  if bash "$verifier" "$fixture" "$arch" > "$fixture_root/result.log" 2>&1; then
    echo "bundle verifier accepted negative fixture: $expected" >&2
    exit 1
  fi
  grep -F "$expected" "$fixture_root/result.log"
}

touch "$fixture/Contents/Resources/unexpected.dylib"
expect_rejected 'bundle contains an unexpected dynamic library or framework'
rm "$fixture/Contents/Resources/unexpected.dylib"

helper="$fixture/Contents/MacOS/helper"
cc -arch "$arch" -mmacosx-version-min=15.0 -x c - -o "$helper" <<<'int main(void) { return 0; }'
expect_rejected 'above supported'
rm "$helper"

cc -arch "$other_arch" -mmacosx-version-min=13.4 -x c - -o "$helper" <<<'int main(void) { return 0; }'
expect_rejected "does not contain architecture $arch"
rm "$helper"

cc -arch "$arch" -mmacosx-version-min=13.4 \
  -Wl,-rpath,/Users/synthetic-developer/lib -x c - -o "$helper" <<<'int main(void) { return 0; }'
expect_rejected 'contains a developer-machine load path'

echo "macOS bundle rejection tests passed for $arch"
