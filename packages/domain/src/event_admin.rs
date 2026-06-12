//! Admin event creation/edit validation (RFC-009).
//! Recurrence expansion (RFC-022): bounded materialization at creation time.

use thiserror::Error;

pub const EVENT_TITLE_MAX: usize    = 80;
pub const EVENT_LOCATION_MAX: usize = 120;
pub const EVENT_DESC_MAX: usize     = 500;
/// Maximum number of occurrences an admin can generate in one operation (RFC-022).
pub const RECURRENCE_MAX_COUNT: u32 = 52;

/// Recurrence frequency for event creation (RFC-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFreq {
    None,
    Weekly,
    Biweekly,
    Monthly,
}

impl RecurrenceFreq {
    /// Parse the value from a form `<select>` option.
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "weekly"    => Self::Weekly,
            "biweekly"  => Self::Biweekly,
            "monthly"   => Self::Monthly,
            _           => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None      => "none",
            Self::Weekly    => "weekly",
            Self::Biweekly  => "biweekly",
            Self::Monthly   => "monthly",
        }
    }

    pub fn is_recurring(self) -> bool {
        !matches!(self, Self::None)
    }
}

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
    if location.as_deref().map(|l| l.chars().count() > EVENT_LOCATION_MAX).unwrap_or(false) {
        return Err(EventValidationError::LocationTooLong);
    }

    let description = raw.description.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    if description.as_deref().map(|d| d.chars().count() > EVENT_DESC_MAX).unwrap_or(false) {
        return Err(EventValidationError::DescriptionTooLong);
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

/// Expand a single base `DayInput` into `count` occurrences spaced by `freq`.
///
/// The first occurrence is the base day itself.
/// Returns an error if the date cannot be parsed or count exceeds the cap.
/// Saturates silently at `RECURRENCE_MAX_COUNT`.
pub fn expand_recurrence(
    base: &DayInput,
    freq: RecurrenceFreq,
    count: u32,
) -> Result<Vec<DayInput>, EventValidationError> {
    if !freq.is_recurring() || count <= 1 {
        return Ok(vec![base.clone()]);
    }
    let count = count.min(RECURRENCE_MAX_COUNT);

    // Parse "YYYY-MM-DD" into a time::Date.
    let base_date = parse_day_date(&base.day_date)
        .ok_or(EventValidationError::DayDateMissing(1))?;

    let mut days = Vec::with_capacity(count as usize);
    for i in 0..count {
        let next_date = advance_date(base_date, freq, i);
        days.push(DayInput {
            day_date:  format_date(next_date),
            starts_at: base.starts_at.clone(),
            ends_at:   base.ends_at.clone(),
        });
    }
    Ok(days)
}

fn parse_day_date(s: &str) -> Option<time::Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 { return None; }
    let y: i32 = parts[0].parse().ok()?;
    let m: u8  = parts[1].parse().ok()?;
    let d: u8  = parts[2].parse().ok()?;
    let month = time::Month::try_from(m).ok()?;
    time::Date::from_calendar_date(y, month, d).ok()
}

fn advance_date(base: time::Date, freq: RecurrenceFreq, steps: u32) -> time::Date {
    match freq {
        RecurrenceFreq::None     => base,
        RecurrenceFreq::Weekly   => base + time::Duration::weeks(steps as i64),
        RecurrenceFreq::Biweekly => base + time::Duration::weeks((steps * 2) as i64),
        RecurrenceFreq::Monthly  => {
            // Add `steps` months to base date, clamping day to end-of-month.
            // month is 1-indexed; convert to 0-indexed for arithmetic.
            let total_months = (base.month() as u32 - 1) + steps;
            let year  = base.year() + (total_months / 12) as i32;
            let month = (total_months % 12 + 1) as u8; // back to 1-indexed
            let month_enum = time::Month::try_from(month).unwrap_or(time::Month::January);
            let days_in = days_in_month(year, month_enum);
            let day = base.day().min(days_in);
            time::Date::from_calendar_date(year, month_enum, day).unwrap_or(base)
        }
    }
}

fn days_in_month(year: i32, month: time::Month) -> u8 {
    use time::Month::*;
    match month {
        January | March | May | July | August | October | December => 31,
        April | June | September | November => 30,
        February => if is_leap(year) { 29 } else { 28 },
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn format_date(d: time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────

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

    fn base_day() -> DayInput {
        DayInput { day_date: "2026-06-06".into(), starts_at: "09:00".into(), ends_at: "10:30".into() }
    }

    // ── validate_event tests ───────────────────────────────────────────────

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

    // ── expand_recurrence tests (RFC-022) ─────────────────────────────────

    #[test]
    fn none_freq_returns_single_day() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::None, 4).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].day_date, "2026-06-06");
    }

    #[test]
    fn count_one_returns_single_day() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 1).unwrap();
        assert_eq!(days.len(), 1);
    }

    #[test]
    fn weekly_four_weeks() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 4).unwrap();
        assert_eq!(days.len(), 4);
        assert_eq!(days[0].day_date, "2026-06-06");
        assert_eq!(days[1].day_date, "2026-06-13");
        assert_eq!(days[2].day_date, "2026-06-20");
        assert_eq!(days[3].day_date, "2026-06-27");
    }

    #[test]
    fn biweekly_three_occurrences() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::Biweekly, 3).unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days[0].day_date, "2026-06-06");
        assert_eq!(days[1].day_date, "2026-06-20");
        assert_eq!(days[2].day_date, "2026-07-04");
    }

    #[test]
    fn monthly_crosses_year_boundary() {
        let base = DayInput {
            day_date: "2026-11-15".into(),
            starts_at: "10:00".into(), ends_at: "11:00".into()
        };
        let days = expand_recurrence(&base, RecurrenceFreq::Monthly, 3).unwrap();
        assert_eq!(days[0].day_date, "2026-11-15");
        assert_eq!(days[1].day_date, "2026-12-15");
        assert_eq!(days[2].day_date, "2027-01-15");
    }

    #[test]
    fn monthly_clamps_to_end_of_feb() {
        let base = DayInput {
            day_date: "2026-01-31".into(),
            starts_at: "10:00".into(), ends_at: "11:00".into()
        };
        let days = expand_recurrence(&base, RecurrenceFreq::Monthly, 2).unwrap();
        assert_eq!(days[0].day_date, "2026-01-31");
        assert_eq!(days[1].day_date, "2026-02-28"); // Feb 2026 has 28 days
    }

    #[test]
    fn count_capped_at_max() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 200).unwrap();
        assert_eq!(days.len(), RECURRENCE_MAX_COUNT as usize);
    }

    #[test]
    fn times_preserved_across_occurrences() {
        let days = expand_recurrence(&base_day(), RecurrenceFreq::Weekly, 3).unwrap();
        for d in &days {
            assert_eq!(d.starts_at, "09:00");
            assert_eq!(d.ends_at,   "10:30");
        }
    }

    #[test]
    fn freq_round_trip() {
        assert_eq!(RecurrenceFreq::from_str("weekly").as_str(), "weekly");
        assert_eq!(RecurrenceFreq::from_str("biweekly").as_str(), "biweekly");
        assert_eq!(RecurrenceFreq::from_str("monthly").as_str(), "monthly");
        assert_eq!(RecurrenceFreq::from_str("none").as_str(), "none");
        assert_eq!(RecurrenceFreq::from_str("unknown").as_str(), "none");
    }
}
