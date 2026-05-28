//! Snapshot metadata format used in git-ss snapshot commits.

use chrono::{DateTime, FixedOffset};
use thiserror::Error;

/// Type of source captured by a snapshot commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadKind {
    /// Snapshot created from the working directory and index.
    Workdir,
    /// Snapshot created from an existing ref or revision expression.
    Ref,
}

impl UploadKind {
    /// Returns the stable metadata value written into snapshot commit messages.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Workdir => "workdir",
            Self::Ref => "ref",
        }
    }
}

/// Structured metadata embedded in every valid git-ss snapshot commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMetadata {
    /// User-facing snapshot id, also used as the remote branch suffix.
    pub id: String,
    /// Kind of snapshot that was uploaded.
    pub kind: UploadKind,
    /// Original upload source, such as `HEAD`, `main`, or `workdir`.
    pub source: String,
    /// Commit id that was current or resolved when the snapshot was created.
    pub source_commit: String,
    /// Local timestamp when the snapshot commit was created.
    pub created_at: DateTime<FixedOffset>,
    /// Remote name selected for the upload.
    pub remote: String,
    /// Whether ignored files were included in a working-directory snapshot.
    pub include_ignored: bool,
    /// `git-ss` package version that created the snapshot.
    pub tool_version: String,
}

/// Errors returned while validating ids or parsing snapshot metadata.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetadataError {
    /// Snapshot ids must contain at least one character.
    #[error("snapshot id cannot be empty")]
    EmptyId,
    /// Snapshot ids are limited to ASCII alphanumeric characters plus `.`, `_`, and `-`.
    #[error("snapshot id contains invalid character {0:?}")]
    InvalidIdChar(char),
    /// A required `Git-SS-*` metadata field is absent.
    #[error("missing metadata field {0}")]
    MissingField(&'static str),
    /// A metadata field is present but cannot be interpreted.
    #[error("invalid metadata field {field}: {value}")]
    InvalidField {
        /// Metadata field name that failed parsing.
        field: &'static str,
        /// Raw metadata field value that failed parsing.
        value: String,
    },
}

/// Formats the default snapshot id for a timestamp.
pub fn format_default_id(now: DateTime<FixedOffset>) -> String {
    now.format("%Y%m%d-%H%M%S").to_string()
}

/// Validates that a snapshot id is safe to use as a branch suffix.
pub fn validate_id(id: &str) -> Result<(), MetadataError> {
    if id.is_empty() {
        return Err(MetadataError::EmptyId);
    }

    for ch in id.chars() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')) {
            return Err(MetadataError::InvalidIdChar(ch));
        }
    }

    Ok(())
}

impl SnapshotMetadata {
    /// Renders metadata as a snapshot commit message.
    pub fn to_commit_message(&self) -> String {
        format!(
            "git-ss snapshot\n\nGit-SS-Id: {}\nGit-SS-Type: {}\nGit-SS-Source: {}\nGit-SS-Source-Commit: {}\nGit-SS-Created-At: {}\nGit-SS-Remote: {}\nGit-SS-Include-Ignored: {}\nGit-SS-Tool-Version: {}\n",
            self.id,
            self.kind.as_str(),
            self.source,
            self.source_commit,
            self.created_at.to_rfc3339(),
            self.remote,
            self.include_ignored,
            self.tool_version
        )
    }
}

/// Parses a snapshot commit message back into structured metadata.
pub fn parse_metadata(message: &str) -> Result<SnapshotMetadata, MetadataError> {
    let id = field(message, "Git-SS-Id")?.to_string();
    validate_id(&id)?;

    let kind = match field(message, "Git-SS-Type")? {
        "workdir" => UploadKind::Workdir,
        "ref" => UploadKind::Ref,
        value => {
            return Err(MetadataError::InvalidField {
                field: "Git-SS-Type",
                value: value.to_string(),
            });
        }
    };

    let created_at_value = field(message, "Git-SS-Created-At")?;
    let created_at = DateTime::parse_from_rfc3339(created_at_value).map_err(|_| {
        MetadataError::InvalidField {
            field: "Git-SS-Created-At",
            value: created_at_value.to_string(),
        }
    })?;

    let include_ignored_value = field(message, "Git-SS-Include-Ignored")?;
    let include_ignored =
        include_ignored_value
            .parse::<bool>()
            .map_err(|_| MetadataError::InvalidField {
                field: "Git-SS-Include-Ignored",
                value: include_ignored_value.to_string(),
            })?;

    Ok(SnapshotMetadata {
        id,
        kind,
        source: field(message, "Git-SS-Source")?.to_string(),
        source_commit: field(message, "Git-SS-Source-Commit")?.to_string(),
        created_at,
        remote: field(message, "Git-SS-Remote")?.to_string(),
        include_ignored,
        tool_version: field(message, "Git-SS-Tool-Version")?.to_string(),
    })
}

/// Finds and returns a single metadata field value from a commit message.
fn field<'a>(message: &'a str, name: &'static str) -> Result<&'a str, MetadataError> {
    let prefix = format!("{name}:");

    message
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::trim_start))
        .ok_or(MetadataError::MissingField(name))
}
