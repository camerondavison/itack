//! Storage layer for issues.

pub mod db;
pub mod markdown;
pub mod metadata;

pub use db::Database;
pub use markdown::write_issue;
pub use metadata::Metadata;
