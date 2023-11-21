#!/bin/bash

# This script builds a ripgrep release for the aarch64-apple-darwin target.
# At time of writing (2023-11-21), GitHub Actions does not free Apple silicon
# runners. Since I have somewhat recently acquired an M2 mac mini, I just use
# this script to build the release tarball and upload it with `gh`.
#
# Once GitHub Actions has proper support for Apple silicon, we should add it
# to our release workflow and drop this script.

set -e

version="$1"
if [ -z "$version" ]; then
  echo "missing version" >&2
  echo "Usage: "$(basename "$0")" <version>" >&2
  exit 1
fi
if ! grep -q "version = \"$version\"" Cargo.toml; then
  echo "version does not match Cargo.toml" >&2
  exit 1
fi

target=aarch64-apple-darwin
cargo build --release --features pcre2 --target $target
BIN=target/$target/release/rg
NAME=ripgrep-$version-$target
ARCHIVE="deployment/m2/$NAME"

mkdir -p "$ARCHIVE"/{complete,doc}
cp "$BIN" "$ARCHIVE"/
strip "$ARCHIVE/rg"
cp {README.md,COPYING,UNLICENSE,LICENSE-MIT} "$ARCHIVE"/
cp {CHANGELOG.md,FAQ.md,GUIDE.md} "$ARCHIVE"/doc/
"$BIN" --generate complete-bash > "$ARCHIVE/complete/rg.bash"
"$BIN" --generate complete-fish > "$ARCHIVE/complete/rg.fish"
"$BIN" --generate complete-powershell > "$ARCHIVE/complete/_rg.ps1"
"$BIN" --generate complete-zsh > "$ARCHIVE/complete/_rg"
"$BIN" --generate man > "$ARCHIVE/doc/rg.1"

tar c -C deployment/m2 -z -f "$ARCHIVE.tar.gz" "$NAME"
shasum -a 256 "$ARCHIVE.tar.gz" > "$ARCHIVE.tar.gz.sha256"
gh release upload "$version" "$ARCHIVE.tar.gz" "$ARCHIVE.tar.gz.sha256"
