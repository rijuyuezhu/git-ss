//! Command handler for listing remote snapshot branches.

use std::cmp::Ordering;
use std::io::{self, Write};

use chrono::{DateTime, FixedOffset, Local};
use comfy_table::{
    Cell, ColumnConstraint, ContentArrangement, Table, Width, presets::UTF8_NO_BORDERS,
};
use git2::{Oid, Repository};
use serde::Serialize;

use super::Result;
use crate::cli::{ListArgs, ListFormat};
use crate::git::{ListedSnapshot, discover_repo, list_snapshots};
use crate::metadata::UploadKind;

const ID_COLUMN: usize = 0;
const SOURCE_COLUMN: usize = 4;
const BASE_COLUMN: usize = 6;
const CHANGES_COLUMN: usize = 8;

#[derive(Debug, Serialize)]
struct OutputSnapshot {
    id: String,
    remote_ref: String,
    commit: String,
    metadata_id: Option<String>,
    r#type: Option<String>,
    source: Option<String>,
    source_commit: Option<String>,
    created_at: Option<String>,
    remote: Option<String>,
    include_ignored: Option<bool>,
    tool_version: Option<String>,
    metadata_error: Option<String>,
    files_changed: Option<usize>,
    insertions: Option<usize>,
    deletions: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct ChangeStats {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

/// Fetches snapshot refs from the selected remote and prints the requested listing format.
pub fn run(args: ListArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo = discover_repo(&current_dir)?;
    let mut snapshots = list_snapshots(&repo, &args.remote)?;
    snapshots.sort_by(compare_snapshots);

    match args.format {
        ListFormat::Human => print_human(&repo, snapshots),
        ListFormat::Csv => print_csv(&repo, snapshots)?,
        ListFormat::Json => print_json(&repo, snapshots)?,
    }

    Ok(())
}

fn print_human(repo: &Repository, snapshots: Vec<ListedSnapshot>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_NO_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
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
                    Cell::new(commit_label(repo, &metadata.source_commit)),
                    Cell::new(short_oid(snapshot.commit)),
                    Cell::new(changes_summary(change_stats(repo, snapshot.commit))),
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
}

fn print_csv(repo: &Repository, snapshots: Vec<ListedSnapshot>) -> Result<()> {
    let stdout = io::stdout();
    let mut writer = csv::Writer::from_writer(stdout.lock());
    for snapshot in output_snapshots(repo, snapshots) {
        writer.serialize(snapshot)?;
    }

    writer.flush()?;
    Ok(())
}

fn print_json(repo: &Repository, snapshots: Vec<ListedSnapshot>) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &output_snapshots(repo, snapshots))?;
    writeln!(stdout)?;
    Ok(())
}

fn output_snapshots(repo: &Repository, snapshots: Vec<ListedSnapshot>) -> Vec<OutputSnapshot> {
    snapshots
        .into_iter()
        .map(|snapshot| {
            let stats = change_stats(repo, snapshot.commit);
            let files_changed = stats.map(|stats| stats.files_changed);
            let insertions = stats.map(|stats| stats.insertions);
            let deletions = stats.map(|stats| stats.deletions);
            let commit = snapshot.commit.to_string();

            match snapshot.metadata {
                Ok(metadata) => {
                    let snapshot_type = upload_kind_name(&metadata.kind).to_string();
                    OutputSnapshot {
                        id: snapshot.id,
                        remote_ref: snapshot.remote_ref,
                        commit,
                        metadata_id: Some(metadata.id),
                        r#type: Some(snapshot_type),
                        source: Some(metadata.source),
                        source_commit: Some(metadata.source_commit),
                        created_at: Some(metadata.created_at.to_rfc3339()),
                        remote: Some(metadata.remote),
                        include_ignored: Some(metadata.include_ignored),
                        tool_version: Some(metadata.tool_version),
                        metadata_error: None,
                        files_changed,
                        insertions,
                        deletions,
                    }
                }
                Err(err) => OutputSnapshot {
                    id: snapshot.id,
                    remote_ref: snapshot.remote_ref,
                    commit,
                    metadata_id: None,
                    r#type: None,
                    source: None,
                    source_commit: None,
                    created_at: None,
                    remote: None,
                    include_ignored: None,
                    tool_version: None,
                    metadata_error: Some(err),
                    files_changed,
                    insertions,
                    deletions,
                },
            }
        })
        .collect()
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

fn upload_kind_name(kind: &UploadKind) -> &'static str {
    match kind {
        UploadKind::Workdir => "workdir",
        UploadKind::Ref => "ref",
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

fn changes_summary(stats: Option<ChangeStats>) -> String {
    let Some(stats) = stats else {
        return "unknown".to_string();
    };

    if stats.files_changed == 0 {
        return "no changes".to_string();
    }

    let file_word = if stats.files_changed == 1 {
        "file"
    } else {
        "files"
    };
    format!(
        "{} {file_word}, +{} -{}",
        stats.files_changed, stats.insertions, stats.deletions
    )
}

fn change_stats(repo: &Repository, commit_id: Oid) -> Option<ChangeStats> {
    (|| -> std::result::Result<ChangeStats, git2::Error> {
        let commit = repo.find_commit(commit_id)?;
        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let snapshot_tree = commit.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&snapshot_tree), None)?;
        let stats = diff.stats()?;

        Ok(ChangeStats {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    })()
    .ok()
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
