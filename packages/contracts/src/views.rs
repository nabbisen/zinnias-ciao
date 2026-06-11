use zinnias_ciao_domain::status::AttendanceStatus;
use serde::{Deserialize, Serialize};

// ── Capabilities ──────────────────────────────────────────────────────────

/// Server-computed permission flags for a rendered page.
/// The renderer uses these directly — it never infers permissions from role
/// strings (RFC-013 §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCapabilities {
    pub can_set_status: bool,
    /// Plain-language reason shown when the status control is disabled.
    /// None when the control is enabled.
    pub set_status_disabled_reason: Option<String>,
    pub can_set_attended_self: bool,
    pub can_edit_event: bool,
    pub can_cancel_event: bool,
    pub can_admin_attendance: bool,
}

// ── Community views ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityView {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub my_role: String, // "admin" | "member" — plain string for i18n
    pub my_display_name: String,
    pub upcoming_event_count: u32,
}

// ── Home / card view ──────────────────────────────────────────────────────

/// The minimal view used to render one event card on the Home list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCardView {
    pub event_id: String,
    pub title: String,
    pub location: Option<String>,
    pub is_cancelled: bool,
    pub days: Vec<EventDaySummary>,
    pub my_status: Option<AttendanceStatus>, // None = No answer, scoped to first upcoming day
    pub counts: StatusCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDaySummary {
    pub day_id: String,
    pub day_date: String,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub seq: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusCounts {
    pub going: u32,
    pub not_going: u32,
    pub attended: u32,
    pub no_answer: u32,
}

// ── Event detail view ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetailView {
    pub event_id: String,
    pub community_id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_cancelled: bool,
    pub days: Vec<EventDayDetailView>,
    pub my_note: Option<NoteView>,
    pub participants: Vec<ParticipantView>,
    pub capabilities: EventCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDayDetailView {
    pub day_id: String,
    pub seq: u32,
    pub day_date: String,
    pub starts_at_utc: String,
    pub ends_at_utc: String,
    pub my_status: Option<AttendanceStatus>,
    pub counts: StatusCounts,
    /// Server-issued form token for the set-status form on this day
    pub form_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantView {
    pub membership_id: String,
    pub display_name: String,
    pub initials: String,
    pub status: Option<AttendanceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteView {
    pub note: String,
    pub updated_at: String,
    /// Form token for saving/editing
    pub save_token: String,
    /// Form token for deletion
    pub delete_token: String,
}

// ── Join / profile views ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JoinFormView {
    pub form_token: String,
    pub error: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct JoinProfileView {
    pub community_name: String,
    pub form_token: String,
    pub error: Option<&'static str>,
}

// ── Me / profile view ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MeView {
    pub display_name: String,
    pub current_community: CommunityView,
    pub all_communities: Vec<CommunityView>,
    pub edit_name_token: String,
    pub logout_token: String,
}
