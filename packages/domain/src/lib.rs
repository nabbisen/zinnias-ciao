// domain — pure business logic, no Worker/WASM deps.

pub mod community;
pub mod display_name;
pub mod event;
pub mod event_admin;
pub mod invite;
pub mod membership;
pub mod note;
pub mod session;
pub mod status;

pub use community::{COMMUNITY_NAME_MAX, Community, CommunityNameError, validate_community_name};
pub use display_name::{DISPLAY_NAME_MAX, DisplayNameError, validate_display_name};
pub use event::{Event, EventDay};
pub use event_admin::{
    DayInput, EventInput, EventValidationError, RECURRENCE_MAX_COUNT, RecurrenceFreq,
    expand_recurrence, validate_event,
};
pub use invite::{INVITE_CODE_LEN, InviteValidationError, validate_invite_input};
pub use membership::{Membership, Role};
pub use note::{NOTE_MAX_CHARS, NoteError, validate_note};
pub use session::SessionState;
pub use status::{
    AttendanceStatus, DayTimeState, StatusTransitionError, validate_status_transition,
};
