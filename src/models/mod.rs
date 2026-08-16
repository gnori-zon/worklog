use chrono::{self};
use uuid::Uuid;

pub type DateTime = chrono::DateTime<chrono::Utc>;

pub type ProjectId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub archived: bool,
}

pub type EntryId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: EntryId,
    pub project_id: ProjectId,
    pub name: String,
    pub started_at: DateTime,
    pub ended_at: Option<DateTime>,
}
