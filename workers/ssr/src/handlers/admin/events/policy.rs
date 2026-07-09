use crate::db::event as event_db;
use zinnias_ciao_domain::EventValidationError;
use zinnias_ciao_domain::event_admin::{EVENT_DESC_MAX, EVENT_LOCATION_MAX, EVENT_TITLE_MAX};

pub(super) struct EventDetailsEdit {
    pub(super) title: String,
    pub(super) location: Option<String>,
    pub(super) description: Option<String>,
}

pub(super) struct EventDayUpdate {
    pub(super) day_date: String,
    pub(super) starts_at_utc: String,
    pub(super) ends_at_utc: String,
}

pub(super) struct EventEditSubmission {
    pub(super) details: EventDetailsEdit,
    pub(super) day_update: Option<EventDayUpdate>,
}

pub(super) fn event_is_recurring(event: &event_db::EventRow) -> bool {
    event.repeat_rule != "none" || event.repeat_count.is_some()
}

pub(super) fn event_schedule_editable(
    event: &event_db::EventRow,
    days: &[event_db::EventDayRow],
) -> bool {
    days.len() == 1 && !event_is_recurring(event)
}

pub(super) fn event_can_seed_recreate(event: &event_db::EventRow) -> bool {
    event.status == "cancelled"
}

pub(super) fn edit_post_contains_schedule_fields(body: &worker::FormData) -> bool {
    [
        "day_date",
        "starts_at",
        "ends_at",
        "repeat_rule",
        "repeat_count",
    ]
    .iter()
    .any(|name| body.get_field(name).is_some())
}

pub(super) fn validate_event_details(
    title_raw: String,
    location_raw: String,
    description_raw: String,
) -> std::result::Result<EventDetailsEdit, EventValidationError> {
    let title = title_raw.trim().to_string();
    if title.is_empty() {
        return Err(EventValidationError::TitleEmpty);
    }
    if title.chars().count() > EVENT_TITLE_MAX {
        return Err(EventValidationError::TitleTooLong);
    }

    let location = Some(location_raw.trim())
        .filter(|s| !s.is_empty())
        .map(String::from);
    if location
        .as_deref()
        .map(|l| l.chars().count() > EVENT_LOCATION_MAX)
        .unwrap_or(false)
    {
        return Err(EventValidationError::LocationTooLong);
    }

    let description = Some(description_raw.trim())
        .filter(|s| !s.is_empty())
        .map(String::from);
    if description
        .as_deref()
        .map(|d| d.chars().count() > EVENT_DESC_MAX)
        .unwrap_or(false)
    {
        return Err(EventValidationError::DescriptionTooLong);
    }

    Ok(EventDetailsEdit {
        title,
        location,
        description,
    })
}

pub(super) fn admin_events_new_next(prefill_day: Option<&str>) -> String {
    match prefill_day {
        Some(day) => format!("admin_events_new:{day}"),
        None => "admin_events_new".to_string(),
    }
}

pub(super) fn valid_prefill_day(day: &str) -> bool {
    if day.len() != 10 || day.get(4..5) != Some("-") || day.get(7..8) != Some("-") {
        return false;
    }
    let Some(year) = day.get(..4).and_then(|part| part.parse::<i32>().ok()) else {
        return false;
    };
    let Some(month) = day.get(5..7).and_then(|part| part.parse::<i32>().ok()) else {
        return false;
    };
    let Some(day_num) = day.get(8..10).and_then(|part| part.parse::<i32>().ok()) else {
        return false;
    };
    (1..=12).contains(&month)
        && (1..=zinnias_ciao_contracts::tz::days_in_month(year, month)).contains(&day_num)
}
