//! HTML render helpers — shared shell, escape, and UI components.

mod errors;
mod event_card;
mod nav;
mod notes;
mod participants;
mod shell;
mod status;
mod time;

pub use errors::{
    configuration_unavailable, internal_error, not_found, service_unavailable, session_expired,
    suspended,
};
pub use event_card::CardDay;
pub use nav::{
    bottom_nav_localized, header, header_with_switcher_localized,
    header_with_switcher_next_localized,
};
pub use notes::{admin_note_hide_form, note_form};
pub use participants::{ParticipantEntry, participant_list};
pub use shell::{escape_html, page_localized};
pub use status::status_form;
pub use time::{
    apply_offset_time_pub, format_day_time_tz_localized, tz_offset_minutes_pub,
    utc_to_local_parts_pub,
};

#[cfg(test)]
mod tests;
