//! Admin event handlers - event create, cancel, edit, attendance, hide-note.

pub use attendance::{get_attendance, post_attendance};
pub use cancel::{get_cancel_event, post_cancel_event};
pub use create::{get_create_event, post_create_event};
pub use edit::{get_edit_event, post_edit_event};
pub use notes::{get_admin_hide_note_confirm, post_admin_hide_note};
pub use occurrence::{get_cancel_occurrence, post_cancel_occurrence};
pub use recreate::get_recreate_event;

#[path = "events/attendance.rs"]
mod attendance;
#[path = "events/cancel.rs"]
mod cancel;
#[path = "events/create.rs"]
mod create;
#[path = "events/edit.rs"]
mod edit;
#[path = "events/forms.rs"]
mod forms;
#[path = "events/notes.rs"]
mod notes;
#[path = "events/occurrence.rs"]
mod occurrence;
#[path = "events/policy.rs"]
mod policy;
#[path = "events/recreate.rs"]
mod recreate;
#[path = "events/summary.rs"]
mod summary;
#[path = "events/support.rs"]
mod support;

#[cfg(test)]
#[path = "events/tests.rs"]
mod tests;
