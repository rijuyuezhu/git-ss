use chrono::{DateTime, FixedOffset};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadKind {
    Workdir,
    Ref,
}

impl UploadKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Workdir => "workdir",
            Self::Ref => "ref",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub id: String,
    pub kind: UploadKind,
    pub source: String,
    pub source_commit: String,
    pub created_at: DateTime<FixedOffset>,
    pub remote: String,
    pub include_ignored: bool,
    pub tool_version: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetadataError {
    #[error("snapshot id cannot be empty")]
    EmptyId,
    #[error("snapshot id contains invalid character {0:?}")]
    InvalidIdChar(char),
    #[error("missing metadata field {0}")]
    MissingField(&'static str),
    #[error("invalid metadata field {field}: {value}")]
    InvalidField { field: &'static str, value: String },
}

pub fn format_default_id(now: DateTime<FixedOffset>) -> String {
    now.format("%Y%m%d-%H%M%S").to_string()
}

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

fn field<'a>(message: &'a str, name: &'static str) -> Result<&'a str, MetadataError> {
    let prefix = format!("{name}:");

    message
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::trim_start))
        .ok_or(MetadataError::MissingField(name))
}
