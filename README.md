# git-ss

`git-ss` is a Git plugin for sharing temporary snapshot branches. It lets you push the tree from an existing ref or your current working directory to `refs/heads/gitss/<id>` on a remote, list available snapshots, and download one into a detached checkout.

The binary is named `git-ss`, so Git also exposes it as `git ss` when it is on your `PATH`.

## Install

### Release Assets

GitHub releases publish binaries for Linux, macOS, and Windows:

| Platform | Release assets |
| --- | --- |
| Linux x86_64 | `git-ss-x86_64-unknown-linux-musl`, `git-ss-x86_64-unknown-linux-musl.tar.gz`, `git-ss-x86_64-unknown-linux-gnu`, `git-ss-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `git-ss-aarch64-unknown-linux-musl`, `git-ss-aarch64-unknown-linux-musl.tar.gz`, `git-ss-aarch64-unknown-linux-gnu`, `git-ss-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `git-ss-x86_64-apple-darwin`, `git-ss-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `git-ss-aarch64-apple-darwin`, `git-ss-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `git-ss-x86_64-pc-windows-msvc.exe`, `git-ss-x86_64-pc-windows-msvc.zip` |
| Windows arm64 | `git-ss-aarch64-pc-windows-msvc.exe`, `git-ss-aarch64-pc-windows-msvc.zip` |

Each release also includes `SHA256SUMS`.

### Linux Install Script

For Linux servers, install the musl static binary for the current CPU architecture with:

```bash
curl -fsSL https://github.com/rijuyuezhu/git-ss/releases/latest/download/install.sh | sh
```

The installer supports x86_64 and arm64 Linux. To install a specific release or avoid `sudo`, pass environment variables to `sh`:

```bash
curl -fsSL https://github.com/rijuyuezhu/git-ss/releases/latest/download/install.sh | GIT_SS_VERSION=v0.2.0 GIT_SS_INSTALL_DIR="$HOME/.local/bin" sh
```

### Linux Packages

GitHub releases provide amd64 and arm64 Linux packages for Debian/Ubuntu, Fedora/RHEL-compatible systems, and Arch Linux. These packages use the musl static binary to avoid coupling the upstream package to the glibc version on the CI runner; glibc builds are available as standalone release assets.

```bash
# Debian/Ubuntu x86_64
sudo apt install ./git-ss_*_amd64.deb

# Debian/Ubuntu arm64
sudo apt install ./git-ss_*_arm64.deb

# Fedora/RHEL-compatible x86_64
sudo dnf install ./git-ss-*.x86_64.rpm

# Fedora/RHEL-compatible arm64
sudo dnf install ./git-ss-*.aarch64.rpm

# Arch Linux x86_64
sudo pacman -U ./git-ss-*-x86_64.pkg.tar.zst

# Arch Linux arm64
sudo pacman -U ./git-ss-*-aarch64.pkg.tar.zst
```

The release CI installs each package in a matching Linux container and verifies `git-ss --version`, `git-ss --help`, and the Git plugin entry point with `git ss -h`.

### Linux Static Binary

Download the latest Linux musl binary from the GitHub release page:

```bash
curl -L -o git-ss https://github.com/rijuyuezhu/git-ss/releases/download/v0.2.0/git-ss-x86_64-unknown-linux-musl
chmod +x git-ss
sudo mv git-ss /usr/local/bin/git-ss
```

Verify the installation:

```bash
git ss -h
```

### macOS

Download the `x86_64-apple-darwin` archive for Intel Macs or the `aarch64-apple-darwin` archive for Apple Silicon Macs from the GitHub release page, then place `git-ss` on your `PATH`.

### Windows

Download `git-ss-x86_64-pc-windows-msvc.zip` for x86_64 Windows or `git-ss-aarch64-pc-windows-msvc.zip` for Windows on Arm from the GitHub release page, then place `git-ss.exe` on your `PATH`.

### Build From Source

```bash
cargo install --git https://github.com/rijuyuezhu/git-ss.git
```

For a local checkout:

```bash
cargo build --release
cp target/release/git-ss ~/.local/bin/git-ss
```

## Usage

### Upload an Existing Ref

```bash
git ss upload --id review-demo HEAD
```

This resolves `HEAD`, creates a snapshot commit with the same tree, records metadata in the commit message, and pushes it to:

```text
refs/heads/gitss/review-demo
```

### Upload the Working Directory

```bash
git ss upload --id work-in-progress workdir
```

Working-directory snapshots include tracked changes, tracked deletions, and untracked non-ignored files. Ignored files are excluded by default.

To include ignored files:

```bash
git ss upload --id full-scratch --include-ignored workdir
```

### List Snapshots

```bash
git ss list
```

The command fetches `refs/heads/gitss/*` from the selected remote and prints an aligned terminal table with the snapshot id, creation time, age, type, source, whether ignored files were included, base commit summary, snapshot commit, and change stats.

For machine-readable output, pass `--format csv` or `--format json`. These formats print raw snapshot records with full commit ids, metadata fields, and numeric change stats.

### Download a Snapshot

```bash
git ss download review-demo
```

Downloads fetch a single snapshot branch and check it out as a detached `HEAD`. By default, the command refuses to overwrite local changes.

To overwrite local worktree changes and untracked files:

```bash
git ss download --force review-demo
```

### Select a Remote

All commands default to `origin`. Use `--remote` to select another remote:

```bash
git ss upload --remote fork --id demo HEAD
git ss list --remote fork
git ss download --remote fork demo
```

## Snapshot IDs

If `--id` is omitted, `git-ss` uses a local timestamp in this format:

```text
YYYYMMDD-HHMMSS
```

Custom ids may contain ASCII letters, digits, `.`, `_`, and `-`.

## Notes

- `git-ss` does not shell out to the `git` executable; Git operations are handled through libgit2.
- Empty repositories are not supported yet because snapshots require a valid `HEAD` commit.
- `download` intentionally leaves the repository in detached `HEAD` state at the snapshot commit.
- Snapshot deletion is not implemented in the first version.
- SSH and HTTPS remotes are supported through libgit2 credential callbacks; behavior may not match every credential flow supported by system Git.

## Development

Run the local verification suite:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps --document-private-items
```

Build a release binary locally:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

CI builds and smoke-tests release binaries for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and `aarch64-pc-windows-msvc`.

Build the Linux release packages locally after building a musl binary:

```bash
go install github.com/goreleaser/nfpm/v2/cmd/nfpm@v2.46.3
export PATH="$(go env GOPATH)/bin:$PATH"
PACKAGE_VERSION=$(cargo pkgid | cut -d# -f2) PACKAGE_RELEASE=1 PACKAGE_ARCH=amd64 PACKAGE_BINARY=target/x86_64-unknown-linux-musl/release/git-ss nfpm package --packager deb --target dist/
PACKAGE_VERSION=$(cargo pkgid | cut -d# -f2) PACKAGE_RELEASE=1 PACKAGE_ARCH=amd64 PACKAGE_BINARY=target/x86_64-unknown-linux-musl/release/git-ss nfpm package --packager rpm --target dist/
PACKAGE_VERSION=$(cargo pkgid | cut -d# -f2) PACKAGE_RELEASE=1 PACKAGE_ARCH=amd64 PACKAGE_BINARY=target/x86_64-unknown-linux-musl/release/git-ss nfpm package --packager archlinux --target dist/
```

### AUR Publishing

AUR accepts `PKGBUILD` sources, not the generated `.pkg.tar.zst` binary package. The easiest AUR path is a `git-ss-bin` package whose `PKGBUILD` downloads the `git-ss-x86_64-unknown-linux-musl` release asset, installs it to `/usr/bin/git-ss`, and declares `depends=('git')`.

Release tags publish `git-ss-bin` to AUR with `KSXGitHub/github-actions-deploy-aur@v4.1.3`. To enable it, create an AUR SSH key, add the public key to the AUR account, and store `AUR_USERNAME`, `AUR_EMAIL`, and `AUR_SSH_PRIVATE_KEY` as GitHub Actions secrets.
