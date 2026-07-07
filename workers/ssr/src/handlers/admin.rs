//! Admin handlers — re-exports from submodules (RFC-009 / RFC-010).
//!
//! Implementations live in:
//!   - `admin/events.rs`:  event create, cancel, edit, attendance, hide-note
//!   - `admin/members.rs`: invite codes and member management
//!   - `admin/member_remove.rs`: member removal
//!   - `admin/role_transfer.rs`: member promotion/demotion
//!   - `admin/help_signin.rs`: active-member lost-session help

pub use events::{
    get_admin_hide_note_confirm, get_attendance, get_cancel_event, get_create_event,
    get_edit_event, get_recreate_event, post_admin_hide_note, post_attendance, post_cancel_event,
    post_create_event, post_edit_event,
};
pub use help_signin::{get_help_signin, post_help_signin};
pub use member_remove::{get_remove_member, post_remove_member};
pub use members::{get_invites, get_members, post_generate_invite, post_revoke_invite};
pub use role_transfer::{
    get_demote_member, get_promote_member, post_demote_member, post_promote_member,
};

#[path = "admin/events.rs"]
mod events;
#[path = "admin/help_signin.rs"]
mod help_signin;
#[path = "admin/member_remove.rs"]
mod member_remove;
#[path = "admin/members.rs"]
mod members;
#[path = "admin/role_transfer.rs"]
mod role_transfer;
