//! ICS (iCalendar) formatting utilities (RFC-023).
//!
//! Pure arithmetic — no Worker/WASM dependencies — so these functions can be
//! unit-tested natively via `cargo test`.

/// Escape special characters in ICS text property values (RFC 5545 §3.3.11).
pub fn ics_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
        .replace('\r', "")
}

/// Fold an ICS content line at 75 octets as per RFC 5545 §3.1.
/// Continuation lines begin with a single SPACE.
pub fn fold_line(line: &str) -> String {
    let bytes = line.as_bytes();
    if bytes.len() <= 75 {
        return format!("{line}\r\n");
    }
    let mut result = String::new();
    let mut pos = 0usize;
    let mut first = true;
    while pos < bytes.len() {
        let max = if first { 75 } else { 74 }; // 74 + 1 space prefix = 75
        let mut end = (pos + max).min(bytes.len());
        // Never split a multi-byte UTF-8 sequence.
        while end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        if first {
            result.push_str(std::str::from_utf8(&bytes[pos..end]).unwrap_or(""));
            result.push_str("\r\n");
            first = false;
        } else {
            result.push(' ');
            result.push_str(std::str::from_utf8(&bytes[pos..end]).unwrap_or(""));
            result.push_str("\r\n");
        }
        pos = end;
    }
    result
}

/// Convert a UTC ISO string to ICS DATETIME UTC format.
/// "2026-06-14T09:00:00.000Z" → "20260614T090000Z"
/// "2026-06-14T09:00:00Z"     → "20260614T090000Z"
pub fn utc_to_ics_dt(utc: &str) -> String {
    let parts: Vec<&str> = utc.splitn(2, 'T').collect();
    if parts.len() < 2 {
        return "19700101T000000Z".to_owned();
    }
    let date = parts[0].replace('-', "");
    let time = parts[1].get(..8).unwrap_or("00:00:00").replace(':', "");
    format!("{date}T{time}Z")
}

/// Sanitize a string for use as a filename component (alphanumeric, hyphens, underscores only).
pub fn sanitize_filename(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .chars()
        .take(40)
        .collect()
}

/// An event day record sufficient to render one VEVENT.
pub struct IcsDay<'a> {
    pub uid: &'a str, // unique ID for this day (stable across refreshes)
    pub title: &'a str,
    pub location: Option<&'a str>,
    pub status: &'a str, // "scheduled" | "cancelled"
    pub starts_at_utc: &'a str,
    pub ends_at_utc: &'a str,
}

/// Build a complete VCALENDAR string for the given event days.
pub fn build_vcalendar(cal_name: &str, days: &[IcsDay<'_>]) -> String {
    let mut out = String::with_capacity(512 + days.len() * 256);

    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//ciao.zinnias//EN\r\n");
    out.push_str(&fold_line(&format!("X-WR-CALNAME:{}", ics_text(cal_name))));
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("METHOD:PUBLISH\r\n");

    for day in days {
        let dtstart = utc_to_ics_dt(day.starts_at_utc);
        let dtend = utc_to_ics_dt(day.ends_at_utc);

        out.push_str("BEGIN:VEVENT\r\n");
        out.push_str(&fold_line(&format!("UID:{}", day.uid)));
        out.push_str(&fold_line(&format!("DTSTART:{dtstart}")));
        out.push_str(&fold_line(&format!("DTEND:{dtend}")));

        if day.status == "cancelled" {
            out.push_str(&fold_line(&format!(
                "SUMMARY:[Cancelled] {}",
                ics_text(day.title)
            )));
            out.push_str("STATUS:CANCELLED\r\n");
        } else {
            out.push_str(&fold_line(&format!("SUMMARY:{}", ics_text(day.title))));
            out.push_str("STATUS:CONFIRMED\r\n");
        }

        if let Some(loc) = day.location
            && !loc.is_empty()
        {
            out.push_str(&fold_line(&format!("LOCATION:{}", ics_text(loc))));
        }

        out.push_str("END:VEVENT\r\n");
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}

#[cfg(test)]
mod tests;
