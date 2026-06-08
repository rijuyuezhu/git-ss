# Using git-ss

`git-ss` helps you share short-lived Git snapshots without creating long-term feature branches. A snapshot is pushed to `refs/heads/gitss/<id>` on a remote, then another user can list or download it.

## Share Your Current Work

Use this when you want someone else to inspect your local changes before they are ready for a real branch or pull request.

```bash
git ss upload --id review-demo workdir
```

The snapshot includes tracked changes, tracked deletions, and untracked non-ignored files. Ignored files stay out of the snapshot unless you opt in:

```bash
git ss upload --id full-scratch --include-ignored workdir
```

## Share an Existing Commit

Use this when the exact content you want to share already exists at a ref such as `HEAD`, a branch, or a tag.

```bash
git ss upload --id release-check HEAD
```

The recipient sees a snapshot with the same tree as that ref, plus metadata describing where it came from.

## Find Available Snapshots

List shared snapshots before choosing one to download:

```bash
git ss list
```

The default view is a terminal table with the snapshot id, creation time, source, base commit, snapshot commit, and change stats. If you want to feed the data to another tool, use a machine-readable format:

```bash
git ss list --format json
git ss list --format csv
```

## Try Someone Else's Snapshot

Download a snapshot by id:

```bash
git ss download review-demo
```

`git-ss` checks out the snapshot as a detached `HEAD`, so your local branches are not moved. By default it refuses to overwrite local work. If you intentionally want to replace local changes and untracked files, use:

```bash
git ss download --force review-demo
```

## Use Another Remote

All commands default to `origin`. Choose another remote when your snapshots live on a fork or a different shared repository:

```bash
git ss upload --remote fork --id demo workdir
git ss list --remote fork
git ss download --remote fork demo
```

## Clean Up Shared Snapshots

Remove all `git-ss` snapshots from the selected remote when they are no longer useful:

```bash
git ss clean
```

This deletes remote branches under `refs/heads/gitss/*`. It does not delete your normal project branches.

## Snapshot IDs

If you do not pass `--id`, `git-ss` creates one from the local timestamp:

```text
YYYYMMDD-HHMMSS
```

Custom ids may contain ASCII letters, digits, `.`, `_`, and `-`.

## Notes

- Empty repositories are not supported yet because snapshots require a valid `HEAD` commit.
- `download` intentionally leaves the repository in detached `HEAD` state at the snapshot commit.
- SSH and HTTPS remotes are supported through libgit2 credential callbacks; behavior may not match every credential flow supported by system Git.
