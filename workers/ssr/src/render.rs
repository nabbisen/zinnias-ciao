//! HTML render helpers — shared shell, escape, and UI components.
//!
//! Design-vocabulary items (constants, helper fns) are declared in child
//! modules for future use and kept for reference; suppress dead_code warnings
//! at module level.
#![allow(dead_code)]

mod errors;
mod event_card;
mod nav;
mod notes;
mod participants;
mod shell;
mod status;
mod time;

#[allow(unused_imports)]
pub use errors::{internal_error, not_found, placeholder, session_expired};
#[allow(unused_imports)]
pub use event_card::{CardDay, event_card};
pub use nav::{bottom_nav, header, header_with_switcher, header_with_switcher_next};
pub use notes::{admin_note_hide_form, note_form};
pub use participants::{ParticipantEntry, participant_list};
pub use shell::{escape_html, page};
#[allow(unused_imports)]
pub use status::{status_chip, status_display, status_form, status_triplet};
pub use time::{
    apply_offset_time_pub, format_day_time_tz, tz_offset_minutes_pub, utc_to_local_parts_pub,
};

#[cfg(test)]
mod tests;
