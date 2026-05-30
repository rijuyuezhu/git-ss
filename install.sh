#!/bin/sh
set -eu

repo="rijuyuezhu/git-ss"
install_dir="${GIT_SS_INSTALL_DIR:-/usr/local/bin}"
version="${GIT_SS_VERSION:-latest}"

die() {
  printf 'git-ss install: %s\n' "$1" >&2
  exit 1
}

info() {
  printf 'git-ss install: %s\n' "$1"
}

download() {
  url="$1"
  output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$output"
  else
    die "curl or wget is required"
  fi
}

[ "$(uname -s)" = "Linux" ] || die "this installer only supports Linux"

case "$(uname -m)" in
  x86_64 | amd64)
    target="x86_64-unknown-linux-musl"
    ;;
  aarch64 | arm64)
    target="aarch64-unknown-linux-musl"
    ;;
  *)
    die "unsupported architecture: $(uname -m)"
    ;;
esac

asset="git-ss-${target}"

if [ "${GIT_SS_BASE_URL:-}" ]; then
  base_url="${GIT_SS_BASE_URL%/}"
elif [ "$version" = "latest" ]; then
  base_url="https://github.com/${repo}/releases/latest/download"
else
  case "$version" in
    v*) tag="$version" ;;
    *) tag="v${version}" ;;
  esac
  base_url="https://github.com/${repo}/releases/download/${tag}"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

info "downloading ${asset}"
download "${base_url}/${asset}" "${tmp_dir}/git-ss"
chmod 0755 "${tmp_dir}/git-ss"

if command -v sha256sum >/dev/null 2>&1; then
  if download "${base_url}/SHA256SUMS" "${tmp_dir}/SHA256SUMS"; then
    if grep "  ${asset}\$" "${tmp_dir}/SHA256SUMS" > "${tmp_dir}/SHA256SUM"; then
      (cd "$tmp_dir" && sha256sum -c SHA256SUM)
    else
      info "checksum entry for ${asset} not found; skipping verification"
    fi
  else
    info "SHA256SUMS not found; skipping verification"
  fi
else
  info "sha256sum not found; skipping verification"
fi

sudo_cmd=""
if [ "$(id -u)" -eq 0 ]; then
  mkdir -p "$install_dir"
elif mkdir -p "$install_dir" 2>/dev/null && [ -w "$install_dir" ]; then
  sudo_cmd=""
elif command -v sudo >/dev/null 2>&1; then
  sudo_cmd="sudo"
  $sudo_cmd mkdir -p "$install_dir"
else
  die "sudo is required to install to ${install_dir}; set GIT_SS_INSTALL_DIR to a writable directory"
fi

if command -v install >/dev/null 2>&1; then
  $sudo_cmd install -m 0755 "${tmp_dir}/git-ss" "${install_dir}/git-ss"
else
  $sudo_cmd cp "${tmp_dir}/git-ss" "${install_dir}/git-ss"
  $sudo_cmd chmod 0755 "${install_dir}/git-ss"
fi

info "installed ${install_dir}/git-ss"
"${install_dir}/git-ss" --version
