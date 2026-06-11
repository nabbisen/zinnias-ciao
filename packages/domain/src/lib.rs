// domain — pure business logic, no Worker/WASM deps.
// All rules live here so they can be unit-tested on the native target.

pub mod community;
pub mod display_name;
pub mod event;
pub mod invite;
pub mod membership;
pub mod session;
pub mod status;

pub use community::Community;
pub use display_name::{DISPLAY_NAME_MAX, DisplayNameError, validate_display_name};
pub use event::{Event, EventDay};
pub use invite::{INVITE_CODE_LEN, InviteValidationError, validate_invite_input};
pub use membership::{Membership, Role};
pub use session::SessionState;
pub use status::{
    AttendanceStatus, DayTimeState, StatusTransitionError, validate_status_transition,
};
