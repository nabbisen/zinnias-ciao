use super::event_card::CardDay;

/// Format a day's time range for display, adjusted for the community timezone.
pub fn format_day_time_tz(day: &CardDay<'_>, tz: &str) -> String {
    let offset_mins = tz_offset_minutes(tz);
    let starts = apply_offset_display(day.starts_at_utc, offset_mins);
    let ends = apply_offset_time(day.ends_at_utc, offset_mins);
    format!("{starts}–{ends}")
}

/// Format a day's time range for display (UTC — used when timezone is unknown).
fn format_day_time(day: &CardDay<'_>) -> String {
    let starts = parse_utc_display(day.starts_at_utc);
    let ends = parse_utc_time(day.ends_at_utc);
    format!("{starts}–{ends}")
}

/// Fixed UTC-offset map for IANA names — delegates to contracts::tz (RFC-018).
fn tz_offset_minutes(tz: &str) -> i32 {
    zinnias_ciao_contracts::tz::offset_minutes_or_utc(tz)
}

/// Apply a UTC offset and return ("YYYY-MM-DD", "HH:MM") in local time.
fn utc_to_local_parts(utc: &str, offset_mins: i32) -> (String, String) {
    zinnias_ciao_contracts::tz::to_local_parts(utc, offset_mins)
}

/// Apply a UTC offset and return "Mon D, HH:MM" in local time.
fn apply_offset_display(utc: &str, offset_mins: i32) -> String {
    let (local_date, time_hm) = utc_to_local_parts(utc, offset_mins);
    if local_date.is_empty() {
        return parse_utc_display(utc);
    }
    let date_label = zinnias_ciao_contracts::tz::date_label_ja(&local_date);
    format!("{date_label} {time_hm}")
}

/// Apply a UTC offset and return only "HH:MM" in local time.
fn apply_offset_time(utc: &str, offset_mins: i32) -> String {
    utc_to_local_parts(utc, offset_mins).1
}

pub(super) fn parse_utc_display(utc: &str) -> String {
    let parts: Vec<&str> = utc.splitn(2, 'T').collect();
    if parts.len() < 2 {
        return utc.to_owned();
    }
    let date = parts[0];
    let time = parts[1].get(..5).unwrap_or("");
    let date_label = zinnias_ciao_contracts::tz::date_label_ja(date);
    format!("{date_label} {time}")
}

pub(super) fn parse_utc_time(utc: &str) -> String {
    utc.split_once('T')
        .map(|(_, time)| time)
        .and_then(|t| t.get(..5))
        .unwrap_or("")
        .to_owned()
}

/// Public re-export for handlers that need offset arithmetic (e.g. event.rs).
pub fn tz_offset_minutes_pub(tz: &str) -> i32 {
    tz_offset_minutes(tz)
}

pub fn utc_to_local_parts_pub(utc: &str, offset: i32) -> (String, String) {
    utc_to_local_parts(utc, offset)
}

pub fn apply_offset_time_pub(utc: &str, offset: i32) -> String {
    apply_offset_time(utc, offset)
}
