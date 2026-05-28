# git-ss

`git-ss` is a Git plugin for sharing temporary snapshot branches. It lets you push the tree from an existing ref or your current working directory to `refs/heads/gitss/<id>` on a remote, list available snapshots, and download one into a detached checkout.

The binary is named `git-ss`, so Git also exposes it as `git ss` when it is on your `PATH`.

## Install

### Linux static binary

Download the latest Linux musl binary from the GitHub release page:

```bash
curl -L -o git-ss https://github.com/rijuyuezhu/git-ss/releases/download/v0.1.1/git-ss-x86_64-unknown-linux-musl
chmod +x git-ss
sudo mv git-ss /usr/local/bin/git-ss
```

Verify the installation:

```bash
git ss --help
```

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

The command fetches `refs/heads/gitss/*` from the selected remote and prints a tab-separated table with the snapshot id, creation time, type, source, source commit, and remote-tracking ref.

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

Build the static Linux release binary:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```
