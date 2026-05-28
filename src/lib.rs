#![warn(missing_docs)]

//! Library components for the `git-ss` command-line tool.
//!
//! The crate is organized so CLI parsing, command orchestration, snapshot
//! metadata handling, and libgit2-backed Git operations remain separate.
//! External users normally invoke the `git-ss` binary, but these modules are
//! documented to make the command behavior and Git boundary easier to inspect.

/// Command-line argument types parsed by `clap`.
pub mod cli;
/// Command handlers that connect parsed arguments to Git operations.
pub mod commands;
/// Libgit2-backed repository, remote, snapshot, and checkout operations.
pub mod git;
/// Snapshot metadata rendering, parsing, and validation.
pub mod metadata;
