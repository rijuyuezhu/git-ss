# git-ss

`git-ss` is a Git plugin for sharing temporary snapshot branches. It lets you push the tree from an existing ref or your current working directory to `refs/heads/gitss/<id>` on a remote, list available snapshots, and download one into a detached checkout.

The binary is named `git-ss`, so Git also exposes it as `git ss` when it is on your `PATH`.

## Documentation

- [Install git-ss](INSTALL.md): platform-specific installation options for Linux, macOS, Windows, and source builds.
- [Using git-ss](USAGE.md): user-focused examples for sharing, listing, downloading, and cleaning snapshots.

## Quick Start

Install `git-ss`, then share your current working directory as a temporary snapshot:

```bash
git ss upload --id review-demo workdir
```

Another user can list and download snapshots from the same remote:

```bash
git ss list
git ss download review-demo
```

For more details, see [Using git-ss](USAGE.md).
