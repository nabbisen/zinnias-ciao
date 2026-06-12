// domain — pure business logic, no Worker/WASM deps.
// All rules live here so they can be unit-tested on the native target.

pub mod community;
pub mod display_name;
pub mod event;
pub mod invite;
pub mod membership;
pub mod note;
pub mod session;
pub mod status;

pub use community::Community;
pub use display_name::{validate_display_name, DisplayNameError, DISPLAY_NAME_MAX};
pub use event::{Event, EventDay};
pub use invite::{validate_invite_input, InviteValidationError, INVITE_CODE_LEN};
pub use membership::{Membership, Role};
pub use note::{validate_note, NoteError, NOTE_MAX_CHARS};
pub use session::SessionState;
pub use status::{
    AttendanceStatus, DayTimeState, StatusTransitionError, validate_status_transition,
};
