#!/bin/sh
set -eu

repo="${LLMCTL_REPO:-aazevedo/llmctl}"
version="${LLMCTL_VERSION:-latest}"
install_dir="${LLMCTL_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) platform="apple-darwin" ;;
  Linux) platform="unknown-linux-gnu" ;;
  *) echo "unsupported OS: $os" >&2; exit 1 ;;
esac

case "$arch" in
  arm64|aarch64) cpu="aarch64" ;;
  x86_64|amd64) cpu="x86_64" ;;
  *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
esac

asset="llmctl-${cpu}-${platform}.tar.gz"
base_url="https://github.com/${repo}/releases"

if [ "$version" = "latest" ]; then
  url="${base_url}/latest/download/${asset}"
else
  url="${base_url}/download/${version}/${asset}"
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$install_dir"
curl -fsSL "$url" -o "$tmpdir/$asset"
tar -xzf "$tmpdir/$asset" -C "$tmpdir"
install "$tmpdir/llmctl" "$install_dir/llmctl"

echo "installed llmctl to $install_dir/llmctl"
