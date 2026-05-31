//! Command handler for listing remote snapshot branches.

use crate::{
    cli::RemoteArgs,
    git::{ListedSnapshot, discover_repo, list_snapshots},
    metadata::UploadKind,
};
use chrono::{DateTime, FixedOffset, Local};
use comfy_table::{
    Cell, ColumnConstraint, ContentArrangement, Table, Width, presets::UTF8_NO_BORDERS,
};
use git2::{Oid, Repository};
use std::cmp::Ordering;

const ID_COLUMN: usize = 0;
const SOURCE_COLUMN: usize = 4;
const BASE_COLUMN: usize = 6;
const CHANGES_COLUMN: usize = 8;

/// Fetches snapshot refs from the selected remote and prints a tabular listing.
pub fn run(args: RemoteArgs) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo = discover_repo(&current_dir)?;
    let mut snapshots = list_snapshots(&repo, &args.remote)?;
    snapshots.sort_by(compare_snapshots);

    let mut table = Table::new();
    table
        .load_preset(UTF8_NO_BORDERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "ID", "CREATED", "AGE", "TYPE", "SOURCE", "IGNORED", "BASE", "SNAPSHOT", "CHANGES",
        ]);
    constrain_columns(&mut table);

    for snapshot in snapshots {
        match snapshot.metadata {
            Ok(metadata) => {
                let (kind, ignored) = match metadata.kind {
                    UploadKind::Workdir => (
                        "workdir",
                        if metadata.include_ignored {
                            "yes"
                        } else {
                            "no"
                        },
                    ),
                    UploadKind::Ref => ("ref", "-"),
                };
                table.add_row([
                    Cell::new(snapshot.id),
                    Cell::new(format_created(metadata.created_at)),
                    Cell::new(format_age(metadata.created_at)),
                    Cell::new(kind),
                    Cell::new(metadata.source),
                    Cell::new(ignored),
                    Cell::new(commit_label(&repo, &metadata.source_commit)),
                    Cell::new(short_oid(snapshot.commit)),
                    Cell::new(changes_summary(&repo, snapshot.commit)),
                ]);
            }
            Err(err) => {
                table.add_row([
                    Cell::new(snapshot.id),
                    Cell::new("WARN"),
                    Cell::new("WARN"),
                    Cell::new("WARN"),
                    Cell::new("WARN"),
                    Cell::new("WARN"),
                    Cell::new("WARN"),
                    Cell::new(short_oid(snapshot.commit)),
                    Cell::new(clean_cell(&err)),
                ]);
            }
        }
    }

    println!("{table}");
    Ok(())
}

fn constrain_columns(table: &mut Table) {
    for (index, width) in [
        (ID_COLUMN, 24),
        (SOURCE_COLUMN, 24),
        (BASE_COLUMN, 40),
        (CHANGES_COLUMN, 32),
    ] {
        if let Some(column) = table.column_mut(index) {
            column.set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(width)));
        }
    }
}

fn compare_snapshots(left: &ListedSnapshot, right: &ListedSnapshot) -> Ordering {
    match (&left.metadata, &right.metadata) {
        (Ok(left_metadata), Ok(right_metadata)) => right_metadata
            .created_at
            .cmp(&left_metadata.created_at)
            .then_with(|| right.id.cmp(&left.id)),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => right.id.cmp(&left.id),
    }
}

fn format_created(created_at: DateTime<FixedOffset>) -> String {
    created_at.format("%Y-%m-%d %H:%M:%S %:z").to_string()
}

fn format_age(created_at: DateTime<FixedOffset>) -> String {
    let seconds = Local::now()
        .fixed_offset()
        .signed_duration_since(created_at)
        .num_seconds();
    let absolute_seconds = if seconds == i64::MIN {
        i64::MAX
    } else {
        seconds.abs()
    };
    let duration = compact_duration(absolute_seconds);

    if seconds >= 0 {
        format!("{duration} ago")
    } else {
        format!("in {duration}")
    }
}

fn compact_duration(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const MONTH: i64 = 30 * DAY;
    const YEAR: i64 = 365 * DAY;

    if seconds < MINUTE {
        format!("{seconds}s")
    } else if seconds < HOUR {
        format!("{}m", seconds / MINUTE)
    } else if seconds < DAY {
        format!("{}h", seconds / HOUR)
    } else if seconds < MONTH {
        format!("{}d", seconds / DAY)
    } else if seconds < YEAR {
        format!("{}mo", seconds / MONTH)
    } else {
        format!("{}y", seconds / YEAR)
    }
}

fn commit_label(repo: &Repository, oid_text: &str) -> String {
    let short = short_text(oid_text);
    let Some(commit) = Oid::from_str(oid_text)
        .ok()
        .and_then(|oid| repo.find_commit(oid).ok())
    else {
        return short;
    };

    match commit.summary() {
        Ok(Some(summary)) => {
            let summary = clean_cell(summary);
            if summary.is_empty() {
                short
            } else {
                format!("{short} {summary}")
            }
        }
        _ => short,
    }
}

fn changes_summary(repo: &Repository, commit_id: Oid) -> String {
    let Ok(commit) = repo.find_commit(commit_id) else {
        return "unknown".to_string();
    };
    let Ok(parent) = commit.parent(0) else {
        return "unknown".to_string();
    };
    let Ok(parent_tree) = parent.tree() else {
        return "unknown".to_string();
    };
    let Ok(snapshot_tree) = commit.tree() else {
        return "unknown".to_string();
    };
    let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&snapshot_tree), None) else {
        return "unknown".to_string();
    };
    let Ok(stats) = diff.stats() else {
        return "unknown".to_string();
    };

    let files = stats.files_changed();
    if files == 0 {
        return "no changes".to_string();
    }

    let file_word = if files == 1 { "file" } else { "files" };
    format!(
        "{files} {file_word}, +{} -{}",
        stats.insertions(),
        stats.deletions()
    )
}

fn short_oid(oid: Oid) -> String {
    short_text(&oid.to_string())
}

fn short_text(value: &str) -> String {
    value.chars().take(7).collect()
}

fn clean_cell(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
