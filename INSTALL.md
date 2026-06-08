# Install git-ss

## Contents

- [Choose a Release Asset](#choose-a-release-asset)
- [Linux](#linux)
- [Linux Install Script](#install-script)
- [Debian and Ubuntu](#debian-and-ubuntu)
- [Fedora, RHEL, and Compatible Systems](#fedora-rhel-and-compatible-systems)
- [Arch Linux](#arch-linux)
- [Linux Static Binary](#static-binary)
- [macOS](#macos)
- [Windows](#windows)
- [Build From Source](#build-from-source)
- [Verify the Installation](#verify-the-installation)

## Choose a Release Asset

GitHub releases publish binaries for Linux, macOS, and Windows. Each release also includes `SHA256SUMS`.

| Platform | Release assets |
| --- | --- |
| Linux x86_64 | `git-ss-x86_64-unknown-linux-musl`, `git-ss-x86_64-unknown-linux-musl.tar.gz`, `git-ss-x86_64-unknown-linux-gnu`, `git-ss-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `git-ss-aarch64-unknown-linux-musl`, `git-ss-aarch64-unknown-linux-musl.tar.gz`, `git-ss-aarch64-unknown-linux-gnu`, `git-ss-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `git-ss-x86_64-apple-darwin`, `git-ss-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `git-ss-aarch64-apple-darwin`, `git-ss-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `git-ss-x86_64-pc-windows-msvc.exe`, `git-ss-x86_64-pc-windows-msvc.zip` |
| Windows arm64 | `git-ss-aarch64-pc-windows-msvc.exe`, `git-ss-aarch64-pc-windows-msvc.zip` |

## Linux

### Install Script

For Linux servers, install the musl static binary for the current CPU architecture with:

```bash
curl -fsSL https://github.com/rijuyuezhu/git-ss/releases/latest/download/install.sh | sh
```

The installer supports x86_64 and arm64 Linux. To install a specific release or avoid `sudo`, pass environment variables to `sh`:

```bash
curl -fsSL https://github.com/rijuyuezhu/git-ss/releases/latest/download/install.sh | GIT_SS_VERSION=v0.3.1 GIT_SS_INSTALL_DIR="$HOME/.local/bin" sh
```

### Debian and Ubuntu

Download the matching `.deb` package from the release page, then install it locally:

```bash
# x86_64
sudo apt install ./git-ss_*_amd64.deb

# arm64
sudo apt install ./git-ss_*_arm64.deb
```

### Fedora, RHEL, and Compatible Systems

Download the matching `.rpm` package from the release page, then install it locally:

```bash
# x86_64
sudo dnf install ./git-ss-*.x86_64.rpm

# arm64
sudo dnf install ./git-ss-*.aarch64.rpm
```

### Arch Linux

Download the matching Arch package from the release page, then install it locally:

```bash
# x86_64
sudo pacman -U ./git-ss-*-x86_64.pkg.tar.zst

# arm64
sudo pacman -U ./git-ss-*-aarch64.pkg.tar.zst
```

Release packages use the musl static binary so they are not coupled to the glibc version on the CI runner. glibc builds are available as standalone release assets.

### Static Binary

You can also download a standalone Linux binary and place it on your `PATH`:

```bash
curl -L -o git-ss https://github.com/rijuyuezhu/git-ss/releases/download/v0.3.1/git-ss-x86_64-unknown-linux-musl
chmod +x git-ss
sudo mv git-ss /usr/local/bin/git-ss
```

Use the `aarch64-unknown-linux-musl` asset instead on arm64 systems.

## macOS

Download the archive for your Mac from the release page:

| Mac | Asset |
| --- | --- |
| Intel | `git-ss-x86_64-apple-darwin.tar.gz` |
| Apple Silicon | `git-ss-aarch64-apple-darwin.tar.gz` |

Extract the archive, then move `git-ss` to a directory on your `PATH`, for example:

```bash
tar -xzf git-ss-aarch64-apple-darwin.tar.gz
chmod +x git-ss
mv git-ss ~/.local/bin/git-ss
```

## Windows

Download the zip archive for your Windows machine from the release page:

| Windows | Asset |
| --- | --- |
| x86_64 | `git-ss-x86_64-pc-windows-msvc.zip` |
| Arm | `git-ss-aarch64-pc-windows-msvc.zip` |

Extract `git-ss.exe`, then place it in a directory on your `PATH`. Once it is on your `PATH`, Git can run it as `git ss`.

## Build From Source

Install directly from the Git repository with Cargo:

```bash
cargo install --git https://github.com/rijuyuezhu/git-ss.git
```

For a local checkout:

```bash
cargo build --release
cp target/release/git-ss ~/.local/bin/git-ss
```

## Verify the Installation

The binary is named `git-ss`. When it is on your `PATH`, Git also exposes it as `git ss`.

```bash
git-ss --version
git ss -h
```
