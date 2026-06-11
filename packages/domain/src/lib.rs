// domain — pure business logic, no Worker/WASM deps.
// All rules live here so they can be unit-tested on the native target.

pub mod community;
pub mod event;
pub mod membership;
pub mod session;
pub mod status;

pub use community::Community;
pub use event::{Event, EventDay};
pub use membership::{Membership, Role};
pub use session::SessionState;
pub use status::{
    AttendanceStatus, DayTimeState, StatusTransitionError, validate_status_transition,
};
