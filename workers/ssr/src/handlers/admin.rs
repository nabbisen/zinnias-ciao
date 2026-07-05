//! Admin handlers — re-exports from submodules (RFC-009 / RFC-010).
//!
//! Implementations live in:
//!   - `admin/events.rs`:  event create, cancel, edit, attendance, hide-note
//!   - `admin/members.rs`: invite codes and member management

pub use events::{
    get_admin_hide_note_confirm, get_attendance, get_cancel_event, get_create_event,
    get_edit_event, get_recreate_event, post_admin_hide_note, post_attendance, post_cancel_event,
    post_create_event, post_edit_event,
};
pub use members::{
    get_invites, get_members, get_remove_member, post_generate_invite, post_remove_member,
    post_revoke_invite,
};

#[path = "admin/events.rs"]
mod events;
#[path = "admin/members.rs"]
mod members;
