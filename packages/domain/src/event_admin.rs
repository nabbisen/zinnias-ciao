//! Admin event creation/edit validation (RFC-009).

use thiserror::Error;

pub const EVENT_TITLE_MAX: usize    = 80;
pub const EVENT_LOCATION_MAX: usize = 120;
pub const EVENT_DESC_MAX: usize     = 500;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventValidationError {
    #[error("Title is required.")]
    TitleEmpty,
    #[error("Title must be 80 characters or fewer.")]
    TitleTooLong,
    #[error("Location must be 120 characters or fewer.")]
    LocationTooLong,
    #[error("Description must be 500 characters or fewer.")]
    DescriptionTooLong,
    #[error("At least one day is required.")]
    NoDays,
    #[error("Day {0}: date is required.")]
    DayDateMissing(usize),
    #[error("Day {0}: start time is required.")]
    DayStartMissing(usize),
    #[error("Day {0}: end time is required.")]
    DayEndMissing(usize),
    #[error("Day {0}: end must be after start.")]
    DayEndBeforeStart(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayInput {
    pub day_date:  String,
    pub starts_at: String, // "HH:MM" local time
    pub ends_at:   String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInput {
    pub title:       String,
    pub location:    Option<String>,
    pub description: Option<String>,
    pub days:        Vec<DayInput>,
}

/// Validate an event creation / edit form submission.
/// Returns the normalised input on success.
pub fn validate_event(raw: EventInput) -> Result<EventInput, EventValidationError> {
    let title = raw.title.trim().to_string();
    if title.is_empty() {
        return Err(EventValidationError::TitleEmpty);
    }
    if title.chars().count() > EVENT_TITLE_MAX {
        return Err(EventValidationError::TitleTooLong);
    }

    let location = raw.location.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    if let Some(ref l) = location {
        if l.chars().count() > EVENT_LOCATION_MAX {
            return Err(EventValidationError::LocationTooLong);
        }
    }

    let description = raw.description.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    if let Some(ref d) = description {
        if d.chars().count() > EVENT_DESC_MAX {
            return Err(EventValidationError::DescriptionTooLong);
        }
    }

    if raw.days.is_empty() {
        return Err(EventValidationError::NoDays);
    }

    let mut days: Vec<DayInput> = Vec::new();
    for (i, day) in raw.days.iter().enumerate() {
        let n = i + 1;
        let day_date  = day.day_date.trim().to_string();
        let starts_at = day.starts_at.trim().to_string();
        let ends_at   = day.ends_at.trim().to_string();

        if day_date.is_empty()  { return Err(EventValidationError::DayDateMissing(n)); }
        if starts_at.is_empty() { return Err(EventValidationError::DayStartMissing(n)); }
        if ends_at.is_empty()   { return Err(EventValidationError::DayEndMissing(n)); }

        // Compare as "YYYY-MM-DDTHH:MM" strings — lexicographic order is correct for ISO-8601
        let start_key = format!("{day_date}T{starts_at}");
        let end_key   = format!("{day_date}T{ends_at}");
        if end_key <= start_key {
            return Err(EventValidationError::DayEndBeforeStart(n));
        }

        days.push(DayInput { day_date, starts_at, ends_at });
    }

    Ok(EventInput { title, location, description, days })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(date: &str, start: &str, end: &str) -> DayInput {
        DayInput { day_date: date.into(), starts_at: start.into(), ends_at: end.into() }
    }

    fn valid_input() -> EventInput {
        EventInput {
            title: "Saturday Walk".into(),
            location: Some("Station Gate".into()),
            description: None,
            days: vec![day("2026-06-14", "09:00", "10:30")],
        }
    }

    #[test]
    fn valid_single_day() {
        assert!(validate_event(valid_input()).is_ok());
    }

    #[test]
    fn valid_multi_day() {
        let mut inp = valid_input();
        inp.days.push(day("2026-06-15", "09:00", "10:00"));
        assert!(validate_event(inp).is_ok());
    }

    #[test]
    fn empty_title_rejected() {
        let mut inp = valid_input();
        inp.title = "   ".into();
        assert_eq!(validate_event(inp), Err(EventValidationError::TitleEmpty));
    }

    #[test]
    fn title_too_long() {
        let mut inp = valid_input();
        inp.title = "A".repeat(EVENT_TITLE_MAX + 1);
        assert_eq!(validate_event(inp), Err(EventValidationError::TitleTooLong));
    }

    #[test]
    fn end_before_start_rejected() {
        let inp = EventInput {
            title: "Walk".into(), location: None, description: None,
            days: vec![day("2026-06-14", "10:00", "09:00")],
        };
        assert_eq!(validate_event(inp), Err(EventValidationError::DayEndBeforeStart(1)));
    }

    #[test]
    fn end_equal_start_rejected() {
        let inp = EventInput {
            title: "Walk".into(), location: None, description: None,
            days: vec![day("2026-06-14", "09:00", "09:00")],
        };
        assert_eq!(validate_event(inp), Err(EventValidationError::DayEndBeforeStart(1)));
    }

    #[test]
    fn no_days_rejected() {
        let mut inp = valid_input();
        inp.days.clear();
        assert_eq!(validate_event(inp), Err(EventValidationError::NoDays));
    }

    #[test]
    fn missing_day_date() {
        let inp = EventInput {
            title: "Walk".into(), location: None, description: None,
            days: vec![day("", "09:00", "10:00")],
        };
        assert_eq!(validate_event(inp), Err(EventValidationError::DayDateMissing(1)));
    }
}
