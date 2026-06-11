use serde::{Deserialize, Serialize};

/// An event belonging to one community.
/// Times live on EventDay — not here (RFC-002 grain decision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub community_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_cancelled: bool,
}

/// One day (or session) of an event.
/// Status is per EventDay; note is per Event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDay {
    pub id: String,
    pub event_id: String,
    pub community_id: String,
    pub seq: u32,
    /// Local calendar date string in community timezone, e.g. "2026-06-14"
    pub day_date: String,
    /// UTC instant as ISO-8601 string
    pub starts_at_utc: String,
    /// UTC instant as ISO-8601 string
    pub ends_at_utc: String,
}
