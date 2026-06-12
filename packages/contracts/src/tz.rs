//! UTC → local time conversion for event display (RFC-018).
//!
//! Cloudflare Workers V8 isolates have no OS timezone data, so we maintain
//! an explicit IANA name → fixed-offset table. DST-observing zones use their
//! standard-time offset; DST handling is a future RFC-018 amendment.
//! Unknown timezone names fall back to UTC (no silent wrong conversion).
//!
//! This module contains only pure arithmetic — no Worker or WASM dependencies —
//! so it can be tested natively via `cargo test`.

/// Return the UTC offset in minutes for an IANA timezone name.
/// Positive = east of UTC (e.g. Asia/Tokyo = +540).
pub fn offset_minutes(tz: &str) -> i32 {
    match tz {
        // UTC
        "UTC" | "Etc/UTC" | "Etc/GMT" => 0,
        // Asia
        "Asia/Tokyo" | "Japan"                          =>  9 * 60,
        "Asia/Seoul" | "Asia/Pyongyang"                 =>  9 * 60,
        "Asia/Shanghai" | "Asia/Hong_Kong" |
        "Asia/Taipei" | "Asia/Singapore" |
        "Asia/Kuala_Lumpur"                             =>  8 * 60,
        "Asia/Bangkok" | "Asia/Jakarta" |
        "Asia/Saigon" | "Asia/Ho_Chi_Minh"              =>  7 * 60,
        "Asia/Dhaka"                                    =>  6 * 60,
        "Asia/Kolkata" | "Asia/Calcutta"                =>  5 * 60 + 30,
        "Asia/Karachi"                                  =>  5 * 60,
        "Asia/Dubai"                                    =>  4 * 60,
        "Asia/Tehran"                                   =>  3 * 60 + 30,
        "Asia/Riyadh" | "Asia/Baghdad"                  =>  3 * 60,
        "Asia/Jerusalem" | "Asia/Tel_Aviv"              =>  2 * 60,
        // Europe
        "Europe/London" | "GB"                          =>  0,
        "Europe/Paris" | "Europe/Berlin" |
        "Europe/Rome" | "Europe/Madrid" |
        "Europe/Amsterdam" | "Europe/Brussels" |
        "Europe/Vienna" | "Europe/Zurich" |
        "Europe/Stockholm" | "Europe/Oslo" |
        "Europe/Copenhagen" | "Europe/Warsaw" |
        "Europe/Prague" | "Europe/Budapest"             =>  1 * 60,
        "Europe/Helsinki" | "Europe/Riga" |
        "Europe/Tallinn" | "Europe/Vilnius" |
        "Europe/Athens" | "Europe/Bucharest" |
        "Europe/Kyiv"                                   =>  2 * 60,
        "Europe/Moscow" | "Europe/Minsk"                =>  3 * 60,
        // Americas
        "America/Sao_Paulo" |
        "America/Argentina/Buenos_Aires"                => -3 * 60,
        "America/Halifax"                               => -4 * 60,
        "America/New_York" | "America/Detroit" |
        "America/Toronto" | "America/Boston" |
        "US/Eastern"                                    => -5 * 60,
        "America/Chicago" | "America/Winnipeg" |
        "US/Central"                                    => -6 * 60,
        "America/Denver" | "America/Edmonton" |
        "US/Mountain"                                   => -7 * 60,
        "America/Los_Angeles" | "America/Vancouver" |
        "US/Pacific"                                    => -8 * 60,
        "America/Anchorage" | "US/Alaska"               => -9 * 60,
        "Pacific/Honolulu" | "US/Hawaii"                => -10 * 60,
        // Pacific / Oceania
        "Australia/Sydney" | "Australia/Melbourne" |
        "Australia/Canberra"                            =>  10 * 60,
        "Australia/Adelaide"                            =>  9 * 60 + 30,
        "Australia/Darwin"                              =>  9 * 60 + 30,
        "Australia/Perth"                               =>  8 * 60,
        "Pacific/Auckland" | "NZ"                       =>  12 * 60,
        // Unknown → display as UTC
        _ => 0,
    }
}

/// Apply a UTC offset (minutes) to a UTC ISO timestamp.
/// Returns ("YYYY-MM-DD", "HH:MM") in local time.
/// Input format: "2026-06-14T09:00:00.000Z" or "2026-06-14T09:00:00Z".
pub fn to_local_parts(utc: &str, offset_mins: i32) -> (String, String) {
    let fallback = (
        utc.get(..10).unwrap_or("").to_owned(),
        utc.get(11..16).unwrap_or("").to_owned(),
    );
    let parts: Vec<&str> = utc.splitn(2, 'T').collect();
    if parts.len() < 2 { return fallback; }
    let date_str = parts[0];
    let time_str = parts[1].get(..5).unwrap_or("");
    if time_str.len() < 5 { return fallback; }

    let segs: Vec<&str> = date_str.split('-').collect();
    if segs.len() < 3 { return fallback; }
    let year:  i32 = segs[0].parse().unwrap_or(0);
    let month: i32 = segs[1].parse().unwrap_or(0);
    let day:   i32 = segs[2].parse().unwrap_or(0);
    let h: i32 = time_str.get(..2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let m: i32 = time_str.get(3..5).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut total_mins = h * 60 + m + offset_mins;
    let mut day_delta: i32 = 0;

    if total_mins < 0 {
        total_mins += 24 * 60;
        day_delta = -1;
    } else if total_mins >= 24 * 60 {
        total_mins -= 24 * 60;
        day_delta = 1;
    }
    let lh = total_mins / 60;
    let lm = total_mins % 60;

    let (fy, fm, fd) = if day_delta == 0 {
        (year, month, day)
    } else {
        add_days(year, month, day, day_delta)
    };

    (format!("{fy:04}-{fm:02}-{fd:02}"), format!("{lh:02}:{lm:02}"))
}

/// Convert a community-local date + "HH:MM" time to a UTC ISO-8601 string
/// `"YYYY-MM-DDTHH:MM:00.000Z"`. Inverse of `to_local_parts`: subtracts the
/// offset (UTC = local − offset). Handles day wrap across the conversion.
/// On unparseable input, falls back to appending the input as-is (degrades to
/// previous behaviour rather than panicking).
pub fn local_to_utc(date: &str, time: &str, offset_mins: i32) -> String {
    let fallback = format!("{date}T{time}:00.000Z");

    let segs: Vec<&str> = date.split('-').collect();
    if segs.len() < 3 { return fallback; }
    let year:  i32 = match segs[0].parse() { Ok(v) => v, Err(_) => return fallback };
    let month: i32 = match segs[1].parse() { Ok(v) => v, Err(_) => return fallback };
    let day:   i32 = match segs[2].parse() { Ok(v) => v, Err(_) => return fallback };
    if time.len() < 5 { return fallback; }
    let h: i32 = match time.get(..2).and_then(|s| s.parse().ok()) { Some(v) => v, None => return fallback };
    let m: i32 = match time.get(3..5).and_then(|s| s.parse().ok()) { Some(v) => v, None => return fallback };

    // UTC = local - offset.
    let mut total_mins = h * 60 + m - offset_mins;
    let mut day_delta: i32 = 0;
    if total_mins < 0 {
        total_mins += 24 * 60;
        day_delta = -1;
    } else if total_mins >= 24 * 60 {
        total_mins -= 24 * 60;
        day_delta = 1;
    }
    let uh = total_mins / 60;
    let um = total_mins % 60;

    let (uy, umth, ud) = if day_delta == 0 {
        (year, month, day)
    } else {
        add_days(year, month, day, day_delta)
    };

    format!("{uy:04}-{umth:02}-{ud:02}T{uh:02}:{um:02}:00.000Z")
}

fn add_days(y: i32, m: i32, d: i32, delta: i32) -> (i32, i32, i32) {
    let nd = d + delta;
    if nd < 1 {
        let pm = if m == 1 { 12 } else { m - 1 };
        let py = if m == 1 { y - 1 } else { y };
        (py, pm, days_in_month(py, pm))
    } else if nd > days_in_month(y, m) {
        let nm = if m == 12 { 1 } else { m + 1 };
        let ny = if m == 12 { y + 1 } else { y };
        (ny, nm, 1)
    } else {
        (y, m, nd)
    }
}

pub fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 },
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── local_to_utc tests (RFC-018 write path) ───────────────────────────

    #[test]
    fn tokyo_local_to_utc_subtracts_nine_hours() {
        // Architect acceptance case: 09:00 Asia/Tokyo -> 00:00Z same day.
        let off = offset_minutes("Asia/Tokyo");
        assert_eq!(local_to_utc("2026-06-14", "09:00", off), "2026-06-14T00:00:00.000Z");
    }

    #[test]
    fn tokyo_local_to_utc_wraps_to_previous_day() {
        // 06:00 JST -> 21:00Z the previous day.
        let off = offset_minutes("Asia/Tokyo");
        assert_eq!(local_to_utc("2026-06-14", "06:00", off), "2026-06-13T21:00:00.000Z");
    }

    #[test]
    fn new_york_local_to_utc_adds_five_hours() {
        // -5h zone: 20:00 local -> 01:00Z next day.
        let off = offset_minutes("America/New_York");
        assert_eq!(local_to_utc("2026-06-14", "20:00", off), "2026-06-15T01:00:00.000Z");
    }

    #[test]
    fn utc_local_to_utc_is_identity() {
        assert_eq!(local_to_utc("2026-06-14", "09:00", 0), "2026-06-14T09:00:00.000Z");
    }

    #[test]
    fn local_to_utc_round_trips_with_to_local_parts() {
        let off = offset_minutes("Asia/Tokyo");
        let utc = local_to_utc("2026-06-14", "09:00", off);
        let (d, t) = to_local_parts(&utc, off);
        assert_eq!((d.as_str(), t.as_str()), ("2026-06-14", "09:00"));
    }

    #[test]
    fn local_to_utc_month_boundary_backward() {
        // 00:30 JST on the 1st -> 15:30Z on the last day of previous month.
        let off = offset_minutes("Asia/Tokyo");
        assert_eq!(local_to_utc("2026-07-01", "00:30", off), "2026-06-30T15:30:00.000Z");
    }

    #[test]
    fn local_to_utc_bad_input_falls_back() {
        assert_eq!(local_to_utc("bad", "09:00", 540), "badT09:00:00.000Z");
    }


    #[test]
    fn utc_offset_is_zero() {
        assert_eq!(offset_minutes("UTC"), 0);
    }

    #[test]
    fn tokyo_is_plus_nine_hours() {
        assert_eq!(offset_minutes("Asia/Tokyo"), 9 * 60);
    }

    #[test]
    fn new_york_is_minus_five() {
        assert_eq!(offset_minutes("America/New_York"), -5 * 60);
    }

    #[test]
    fn kolkata_half_hour_offset() {
        assert_eq!(offset_minutes("Asia/Kolkata"), 5 * 60 + 30);
    }

    #[test]
    fn unknown_tz_falls_back_to_utc() {
        assert_eq!(offset_minutes("Atlantis/Underwater"), 0);
    }

    #[test]
    fn tokyo_no_day_wrap() {
        // UTC 01:00 + 9h = JST 10:00, same date
        let (d, t) = to_local_parts("2026-06-14T01:00:00.000Z", 9 * 60);
        assert_eq!(d, "2026-06-14");
        assert_eq!(t, "10:00");
    }

    #[test]
    fn tokyo_forward_day() {
        // UTC 16:00 + 9h = JST 01:00 next day
        let (d, t) = to_local_parts("2026-06-14T16:00:00.000Z", 9 * 60);
        assert_eq!(d, "2026-06-15");
        assert_eq!(t, "01:00");
    }

    #[test]
    fn new_york_backward_day() {
        // UTC 03:00 − 5h = Eastern 22:00 previous day
        let (d, t) = to_local_parts("2026-06-14T03:00:00.000Z", -5 * 60);
        assert_eq!(d, "2026-06-13");
        assert_eq!(t, "22:00");
    }

    #[test]
    fn month_boundary_forward() {
        // UTC 2026-06-30 16:00 + 9h = JST 2026-07-01 01:00
        let (d, t) = to_local_parts("2026-06-30T16:00:00.000Z", 9 * 60);
        assert_eq!(d, "2026-07-01");
        assert_eq!(t, "01:00");
    }

    #[test]
    fn month_boundary_backward() {
        // UTC 2026-07-01 02:00 − 5h = Eastern 2026-06-30 21:00
        let (d, t) = to_local_parts("2026-07-01T02:00:00.000Z", -5 * 60);
        assert_eq!(d, "2026-06-30");
        assert_eq!(t, "21:00");
    }

    #[test]
    fn leap_day_boundary_forward() {
        // 2028 is a leap year: UTC 2028-02-28 16:00 + 9h = JST 2028-02-29 01:00
        let (d, t) = to_local_parts("2028-02-28T16:00:00.000Z", 9 * 60);
        assert_eq!(d, "2028-02-29");
        assert_eq!(t, "01:00");
    }

    #[test]
    fn non_leap_feb_boundary() {
        // 2026 is not a leap year: UTC 2026-02-28 16:00 + 9h = JST 2026-03-01 01:00
        let (d, t) = to_local_parts("2026-02-28T16:00:00.000Z", 9 * 60);
        assert_eq!(d, "2026-03-01");
        assert_eq!(t, "01:00");
    }

    #[test]
    fn year_boundary_forward() {
        // UTC 2026-12-31 16:00 + 9h = JST 2027-01-01 01:00
        let (d, t) = to_local_parts("2026-12-31T16:00:00.000Z", 9 * 60);
        assert_eq!(d, "2027-01-01");
        assert_eq!(t, "01:00");
    }

    #[test]
    fn year_boundary_backward() {
        // UTC 2027-01-01 02:00 − 5h = Eastern 2026-12-31 21:00
        let (d, t) = to_local_parts("2027-01-01T02:00:00.000Z", -5 * 60);
        assert_eq!(d, "2026-12-31");
        assert_eq!(t, "21:00");
    }

    #[test]
    fn days_in_month_leap_feb() {
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29); // divisible by 400
        assert_eq!(days_in_month(1900, 2), 28); // divisible by 100 but not 400
    }

    #[test]
    fn exact_midnight_utc_no_change() {
        // UTC midnight stays midnight in UTC
        let (d, t) = to_local_parts("2026-06-14T00:00:00.000Z", 0);
        assert_eq!(d, "2026-06-14");
        assert_eq!(t, "00:00");
    }

    #[test]
    fn kolkata_half_hour_arithmetic() {
        // UTC 05:30 + 5:30 = IST 11:00
        let (d, t) = to_local_parts("2026-06-14T05:30:00.000Z", 5 * 60 + 30);
        assert_eq!(d, "2026-06-14");
        assert_eq!(t, "11:00");
    }
}

// ── Date label formatting (RFC-047) ──────────────────────────────────────
//
// Day-of-week for a Gregorian date via Zeller's congruence. Returns 0=Sunday
// .. 6=Saturday. Pure arithmetic, no external deps.
pub fn weekday_index(year: i32, month: i32, day: i32) -> i32 {
    // Zeller's congruence (Gregorian). Treat Jan/Feb as months 13/14 of prior year.
    let (m, y) = if month < 3 { (month + 12, year - 1) } else { (month, year) };
    let k = y % 100;
    let j = y / 100;
    let h = (day + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Zeller: 0=Saturday..6=Friday. Convert to 0=Sunday..6=Saturday.
    (h + 6) % 7
}

/// Japanese single-character weekday label for 0=Sunday..6=Saturday.
pub fn weekday_ja(index: i32) -> &'static str {
    match index.rem_euclid(7) {
        0 => "日", 1 => "月", 2 => "火", 3 => "水",
        4 => "木", 5 => "金", _ => "土",
    }
}

/// English 3-letter month abbreviation for 1..=12.
pub fn month_abbr_en(month: i32) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "",
    }
}

/// Format a `YYYY-MM-DD` local date as a Japanese calendar label with weekday,
/// e.g. `"6月14日（土）"`. Falls back to the raw string if unparseable.
pub fn date_label_ja(local_date: &str) -> String {
    let segs: Vec<&str> = local_date.split('-').collect();
    if segs.len() < 3 { return local_date.to_owned(); }
    let (y, m, d) = match (segs[0].parse::<i32>(), segs[1].parse::<i32>(), segs[2].parse::<i32>()) {
        (Ok(y), Ok(m), Ok(d)) => (y, m, d),
        _ => return local_date.to_owned(),
    };
    let wd = weekday_ja(weekday_index(y, m, d));
    format!("{m}月{d}日（{wd}）")
}

/// Format a `YYYY-MM-DD` local date as an English calendar label,
/// e.g. `"14 Jun"`. Falls back to the raw string if unparseable.
pub fn date_label_en(local_date: &str) -> String {
    let segs: Vec<&str> = local_date.split('-').collect();
    if segs.len() < 3 { return local_date.to_owned(); }
    let (m, d) = match (segs[1].parse::<i32>(), segs[2].parse::<i32>()) {
        (Ok(m), Ok(d)) => (m, d),
        _ => return local_date.to_owned(),
    };
    format!("{d} {}", month_abbr_en(m))
}

#[cfg(test)]
mod date_label_tests {
    use super::*;

    #[test]
    fn weekday_known_dates() {
        // 2026-06-14 is a Sunday.
        assert_eq!(weekday_index(2026, 6, 14), 0);
        // 2026-06-13 is a Saturday.
        assert_eq!(weekday_index(2026, 6, 13), 6);
        // 2000-01-01 was a Saturday.
        assert_eq!(weekday_index(2000, 1, 1), 6);
        // 2026-01-01 is a Thursday.
        assert_eq!(weekday_index(2026, 1, 1), 4);
    }

    #[test]
    fn ja_label_has_month_day_weekday() {
        // 2026-06-13 is Saturday → 土
        assert_eq!(date_label_ja("2026-06-13"), "6月13日（土）");
        // 2026-06-14 is Sunday → 日
        assert_eq!(date_label_ja("2026-06-14"), "6月14日（日）");
    }

    #[test]
    fn ja_label_no_english_month() {
        let label = date_label_ja("2026-06-14");
        assert!(!label.contains("Jun"), "JA label must not contain English month: {label}");
        assert!(label.contains("月"), "JA label must use 月");
    }

    #[test]
    fn en_label_format() {
        assert_eq!(date_label_en("2026-06-14"), "14 Jun");
        assert_eq!(date_label_en("2026-12-01"), "1 Dec");
    }

    #[test]
    fn malformed_date_falls_back() {
        assert_eq!(date_label_ja("not-a-date-x"), "not-a-date-x");
        assert_eq!(date_label_en("garbage"), "garbage");
    }
}
