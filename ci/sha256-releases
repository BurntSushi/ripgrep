#!/bin/sh

set -e

if [ $# != 1 ]; then
  echo "Usage: $(basename $0) version" >&2
  exit 1
fi
version="$1"

# Linux and Darwin builds.
for arch in i686 x86_64; do
  for target in apple-darwin unknown-linux-musl; do
    url="https://github.com/BurntSushi/ripgrep/releases/download/$version/ripgrep-$version-$arch-$target.tar.gz"
    sha=$(curl -sfSL "$url" | sha256sum)
    echo "$version-$arch-$target $sha"
  done
done

# Source.
for ext in zip tar.gz; do
  url="https://github.com/BurntSushi/ripgrep/archive/$version.$ext"
  sha=$(curl -sfSL "$url" | sha256sum)
  echo "source.$ext $sha"
done
