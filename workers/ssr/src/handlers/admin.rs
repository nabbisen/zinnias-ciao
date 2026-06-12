//! Admin handlers — re-exports from submodules (RFC-009 / RFC-010).
//!
//! Implementations live in:
//!   - `admin/events.rs`:  event create, cancel, edit, attendance, hide-note
//!   - `admin/members.rs`: invite codes and member management

pub use events::{
    get_create_event, post_create_event,
    get_cancel_event, post_cancel_event,
    get_edit_event, post_edit_event,
    get_attendance, post_attendance,
    get_admin_hide_note_confirm, post_admin_hide_note,
};
pub use members::{
    get_invites, post_generate_invite, post_revoke_invite,
    get_members, get_remove_member, post_remove_member,
};

#[path = "admin/events.rs"]
mod events;
#[path = "admin/members.rs"]
mod members;
