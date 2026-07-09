//! Admin event creation/edit validation (RFC-009).
//! Recurrence expansion (RFC-022): bounded materialization at creation time.

use thiserror::Error;

pub const EVENT_TITLE_MAX: usize = 80;
pub const EVENT_LOCATION_MAX: usize = 120;
pub const EVENT_DESC_MAX: usize = 500;
/// Maximum number of occurrences an admin can generate in one operation (RFC-022).
pub const RECURRENCE_MAX_COUNT: u32 = 52;
/// RFC-065 global forward materialization window.
pub const RECURRENCE_MATERIALIZATION_MONTHS_AHEAD: u32 = 6;
/// RFC-065 hard cap for one materialization operation.
pub const RECURRENCE_MATERIALIZATION_INSERT_CAP: usize = 64;

/// Recurrence frequency for event creation (RFC-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFreq {
    None,
    Weekly,
    Biweekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrenceEnd {
    AfterCount(u32),
    UntilDate(String),
    OpenEnded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceOccurrence {
    pub ordinal: u32,
    pub day: DayInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationWindow {
    pub from_day_date: String,
    pub through_day_date: String,
}

impl RecurrenceFreq {
    /// Parse the value from a form `<select>` option.
    pub fn parse_form_value(s: &str) -> Self {
        match s.trim() {
            "weekly" => Self::Weekly,
            "biweekly" => Self::Biweekly,
            "monthly" => Self::Monthly,
            _ => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Weekly => "weekly",
            Self::Biweekly => "biweekly",
            Self::Monthly => "monthly",
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
    #[error("Repeat count is required.")]
    RepeatCountMissing,
    #[error("Repeat count must be 1 or greater.")]
    RepeatCountInvalid,
    #[error("Repeat end date is invalid.")]
    RepeatUntilInvalid,
    #[error("Repeat start is too far in the past.")]
    RepeatStartTooFarPast,
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
    pub day_date: String,
    pub starts_at: String, // "HH:MM" local time
    pub ends_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInput {
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub days: Vec<DayInput>,
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

    let location = raw
        .location
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    if location
        .as_deref()
        .map(|l| l.chars().count() > EVENT_LOCATION_MAX)
        .unwrap_or(false)
    {
        return Err(EventValidationError::LocationTooLong);
    }

    let description = raw
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    if description
        .as_deref()
        .map(|d| d.chars().count() > EVENT_DESC_MAX)
        .unwrap_or(false)
    {
        return Err(EventValidationError::DescriptionTooLong);
    }

    if raw.days.is_empty() {
        return Err(EventValidationError::NoDays);
    }

    let mut days: Vec<DayInput> = Vec::new();
    for (i, day) in raw.days.iter().enumerate() {
        let n = i + 1;
        let day_date = day.day_date.trim().to_string();
        let starts_at = day.starts_at.trim().to_string();
        let ends_at = day.ends_at.trim().to_string();

        if day_date.is_empty() {
            return Err(EventValidationError::DayDateMissing(n));
        }
        if starts_at.is_empty() {
            return Err(EventValidationError::DayStartMissing(n));
        }
        if ends_at.is_empty() {
            return Err(EventValidationError::DayEndMissing(n));
        }

        // Compare as "YYYY-MM-DDTHH:MM" strings — lexicographic order is correct for ISO-8601
        let start_key = format!("{day_date}T{starts_at}");
        let end_key = format!("{day_date}T{ends_at}");
        if end_key <= start_key {
            return Err(EventValidationError::DayEndBeforeStart(n));
        }

        days.push(DayInput {
            day_date,
            starts_at,
            ends_at,
        });
    }

    Ok(EventInput {
        title,
        location,
        description,
        days,
    })
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
    let base_date =
        parse_day_date(&base.day_date).ok_or(EventValidationError::DayDateMissing(1))?;

    let mut days = Vec::with_capacity(count as usize);
    for i in 0..count {
        let next_date = advance_date(base_date, freq, i);
        days.push(DayInput {
            day_date: format_date(next_date),
            starts_at: base.starts_at.clone(),
            ends_at: base.ends_at.clone(),
        });
    }
    Ok(days)
}

pub fn recurrence_materialization_window(today_day_date: &str) -> Option<MaterializationWindow> {
    let today = parse_day_date(today_day_date)?;
    let (year, month) = add_months_to_year_month(
        today.year(),
        today.month() as u8,
        RECURRENCE_MATERIALIZATION_MONTHS_AHEAD,
    );
    let month_enum = time::Month::try_from(month).ok()?;
    let last_day = days_in_month(year, month_enum);
    let through = time::Date::from_calendar_date(year, month_enum, last_day).ok()?;
    Some(MaterializationWindow {
        from_day_date: today_day_date.to_string(),
        through_day_date: format_date(through),
    })
}

pub fn month_intersects_materialization_window(
    month_start: &str,
    next_month_start: &str,
    window: &MaterializationWindow,
) -> bool {
    month_start <= window.through_day_date.as_str()
        && next_month_start > window.from_day_date.as_str()
}

pub fn validate_recurrence_end(
    freq: RecurrenceFreq,
    mode: &str,
    count: Option<u32>,
    until_day_date: Option<&str>,
) -> Result<Option<RecurrenceEnd>, EventValidationError> {
    if !freq.is_recurring() {
        return Ok(None);
    }
    match mode.trim() {
        "open_ended" => Ok(Some(RecurrenceEnd::OpenEnded)),
        "until_date" => {
            let until = until_day_date
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or(EventValidationError::RepeatUntilInvalid)?;
            if parse_day_date(until).is_none() {
                return Err(EventValidationError::RepeatUntilInvalid);
            }
            Ok(Some(RecurrenceEnd::UntilDate(until.to_string())))
        }
        "after_count" | "" => {
            let count = count.ok_or(EventValidationError::RepeatCountMissing)?;
            if count == 0 {
                return Err(EventValidationError::RepeatCountInvalid);
            }
            Ok(Some(RecurrenceEnd::AfterCount(
                count.min(RECURRENCE_MAX_COUNT),
            )))
        }
        _ => Err(EventValidationError::RepeatCountInvalid),
    }
}

pub fn generate_recurrence_occurrences(
    base: &DayInput,
    freq: RecurrenceFreq,
    end: &RecurrenceEnd,
    through_day_date: &str,
    skip_day_dates: &[String],
) -> Result<Vec<RecurrenceOccurrence>, EventValidationError> {
    generate_recurrence_occurrences_after(
        base,
        freq,
        end,
        None,
        through_day_date,
        skip_day_dates,
        RECURRENCE_MATERIALIZATION_INSERT_CAP,
    )
}

pub fn generate_recurrence_occurrences_after(
    base: &DayInput,
    freq: RecurrenceFreq,
    end: &RecurrenceEnd,
    after_day_date: Option<&str>,
    through_day_date: &str,
    skip_day_dates: &[String],
    max_results: usize,
) -> Result<Vec<RecurrenceOccurrence>, EventValidationError> {
    if max_results == 0 {
        return Ok(Vec::new());
    }
    let base_date =
        parse_day_date(&base.day_date).ok_or(EventValidationError::DayDateMissing(1))?;
    let through =
        parse_day_date(through_day_date).ok_or(EventValidationError::RepeatUntilInvalid)?;
    let after = after_day_date
        .map(|date| parse_day_date(date).ok_or(EventValidationError::RepeatUntilInvalid))
        .transpose()?;
    if through < base_date {
        return Ok(Vec::new());
    }

    let until = match end {
        RecurrenceEnd::AfterCount(count) => RecurrenceStop::Count(*count),
        RecurrenceEnd::UntilDate(date) => RecurrenceStop::Date(
            parse_day_date(date).ok_or(EventValidationError::RepeatUntilInvalid)?,
        ),
        RecurrenceEnd::OpenEnded => RecurrenceStop::Date(through),
    };

    let mut out = Vec::new();
    let mut ordinal = 1u32;
    loop {
        if matches!(until, RecurrenceStop::Count(count) if ordinal > count) {
            break;
        }
        let next_date = advance_date(base_date, freq, ordinal - 1);
        if next_date > through {
            break;
        }
        if matches!(until, RecurrenceStop::Date(stop_date) if next_date > stop_date) {
            break;
        }
        let day_date = format_date(next_date);
        let after_cutoff_reached = match after {
            Some(after) => next_date > after,
            None => true,
        };
        if after_cutoff_reached && !skip_day_dates.iter().any(|skip| skip == &day_date) {
            out.push(RecurrenceOccurrence {
                ordinal,
                day: DayInput {
                    day_date,
                    starts_at: base.starts_at.clone(),
                    ends_at: base.ends_at.clone(),
                },
            });
        }
        if out.len() >= max_results {
            break;
        }
        ordinal += 1;
    }
    Ok(out)
}

enum RecurrenceStop {
    Count(u32),
    Date(time::Date),
}

fn parse_day_date(s: &str) -> Option<time::Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u8 = parts[1].parse().ok()?;
    let d: u8 = parts[2].parse().ok()?;
    let month = time::Month::try_from(m).ok()?;
    time::Date::from_calendar_date(y, month, d).ok()
}

fn advance_date(base: time::Date, freq: RecurrenceFreq, steps: u32) -> time::Date {
    match freq {
        RecurrenceFreq::None => base,
        RecurrenceFreq::Weekly => base + time::Duration::weeks(steps as i64),
        RecurrenceFreq::Biweekly => base + time::Duration::weeks((steps * 2) as i64),
        RecurrenceFreq::Monthly => {
            // Add `steps` months to base date, clamping day to end-of-month.
            // month is 1-indexed; convert to 0-indexed for arithmetic.
            let total_months = (base.month() as u32 - 1) + steps;
            let year = base.year() + (total_months / 12) as i32;
            let month = (total_months % 12 + 1) as u8; // back to 1-indexed
            let month_enum = time::Month::try_from(month).unwrap_or(time::Month::January);
            let days_in = days_in_month(year, month_enum);
            let day = base.day().min(days_in);
            time::Date::from_calendar_date(year, month_enum, day).unwrap_or(base)
        }
    }
}

fn add_months_to_year_month(year: i32, month: u8, delta: u32) -> (i32, u8) {
    let zero_based = year * 12 + (month as i32 - 1) + delta as i32;
    (
        zero_based.div_euclid(12),
        (zero_based.rem_euclid(12) + 1) as u8,
    )
}

fn days_in_month(year: i32, month: time::Month) -> u8 {
    use time::Month::*;
    match month {
        January | March | May | July | August | October | December => 31,
        April | June | September | November => 30,
        February => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn format_date(d: time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

#[cfg(test)]
mod tests;
