use chrono::{FixedOffset, TimeZone};
use git_ss::metadata::{
    SnapshotMetadata, UploadKind, format_default_id, parse_metadata, validate_id,
};

#[test]
fn formats_default_id_as_local_date_time() {
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let now = offset.with_ymd_and_hms(2026, 5, 28, 15, 30, 12).unwrap();
    assert_eq!(format_default_id(now), "20260528-153012");
}

#[test]
fn validates_safe_ids() {
    assert!(validate_id("20260528-153012").is_ok());
    assert!(validate_id("my_test.1").is_ok());
    assert!(validate_id("has/slash").is_err());
    assert!(validate_id("has space").is_err());
    assert!(validate_id("").is_err());
}

#[test]
fn renders_and_parses_workdir_metadata() {
    let created_at = FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .with_ymd_and_hms(2026, 5, 28, 15, 30, 12)
        .unwrap();

    let metadata = SnapshotMetadata {
        id: "20260528-153012".to_string(),
        kind: UploadKind::Workdir,
        source: "HEAD".to_string(),
        source_commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
        created_at,
        remote: "origin".to_string(),
        include_ignored: false,
        tool_version: "0.1.0".to_string(),
    };

    let message = metadata.to_commit_message();
    assert!(message.contains("git-ss snapshot"));
    assert!(message.contains("Git-SS-Id: 20260528-153012"));
    assert!(message.contains("Git-SS-Type: workdir"));

    let parsed = parse_metadata(&message).expect("metadata parses");
    assert_eq!(parsed.id, metadata.id);
    assert_eq!(parsed.kind, metadata.kind);
    assert_eq!(parsed.source, metadata.source);
    assert_eq!(parsed.include_ignored, metadata.include_ignored);
}
