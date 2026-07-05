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
/// Return the UTC offset in minutes for a known IANA timezone name.
/// Returns `None` for unrecognized names.
///
/// Call sites that write event times **must** use this function and reject
/// unknown zones — a silent UTC fallback would store wrong times for
/// communities with a misconfigured timezone (P1-timezone, architect review).
///
/// Call sites that only *display* times may use `offset_minutes_or_utc`.
pub fn offset_minutes(tz: &str) -> Option<i32> {
    Some(match tz {
        // UTC
        "UTC" | "Etc/UTC" | "Etc/GMT" => 0,
        // Asia
        "Asia/Tokyo" | "Japan" => 9 * 60,
        "Asia/Seoul" | "Asia/Pyongyang" => 9 * 60,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Taipei" | "Asia/Singapore"
        | "Asia/Kuala_Lumpur" => 8 * 60,
        "Asia/Bangkok" | "Asia/Jakarta" | "Asia/Saigon" | "Asia/Ho_Chi_Minh" => 7 * 60,
        "Asia/Dhaka" => 6 * 60,
        "Asia/Kolkata" | "Asia/Calcutta" => 5 * 60 + 30,
        "Asia/Karachi" => 5 * 60,
        "Asia/Dubai" => 4 * 60,
        "Asia/Tehran" => 3 * 60 + 30,
        "Asia/Riyadh" | "Asia/Baghdad" => 3 * 60,
        "Asia/Jerusalem" | "Asia/Tel_Aviv" => 2 * 60,
        // Europe
        "Europe/London" | "GB" => 0,
        "Europe/Paris" | "Europe/Berlin" | "Europe/Rome" | "Europe/Madrid" | "Europe/Amsterdam"
        | "Europe/Brussels" | "Europe/Vienna" | "Europe/Zurich" | "Europe/Stockholm"
        | "Europe/Oslo" | "Europe/Copenhagen" | "Europe/Warsaw" | "Europe/Prague"
        | "Europe/Budapest" => 60,
        "Europe/Helsinki" | "Europe/Riga" | "Europe/Tallinn" | "Europe/Vilnius"
        | "Europe/Athens" | "Europe/Bucharest" | "Europe/Kyiv" => 2 * 60,
        "Europe/Moscow" | "Europe/Minsk" => 3 * 60,
        // Americas
        "America/Sao_Paulo" | "America/Argentina/Buenos_Aires" => -3 * 60,
        "America/Halifax" => -4 * 60,
        "America/New_York" | "America/Detroit" | "America/Toronto" | "America/Boston"
        | "US/Eastern" => -5 * 60,
        "America/Chicago" | "America/Winnipeg" | "US/Central" => -6 * 60,
        "America/Denver" | "America/Edmonton" | "US/Mountain" => -7 * 60,
        "America/Los_Angeles" | "America/Vancouver" | "US/Pacific" => -8 * 60,
        "America/Anchorage" | "US/Alaska" => -9 * 60,
        "Pacific/Honolulu" | "US/Hawaii" => -10 * 60,
        // Pacific / Oceania
        "Australia/Sydney" | "Australia/Melbourne" | "Australia/Canberra" => 10 * 60,
        "Australia/Adelaide" => 9 * 60 + 30,
        "Australia/Darwin" => 9 * 60 + 30,
        "Australia/Perth" => 8 * 60,
        "Pacific/Auckland" | "NZ" => 12 * 60,
        // Unknown → None (callers must reject or fall back explicitly)
        _ => return None,
    })
}

/// Return the UTC offset in minutes for display purposes only.
/// Falls back to UTC (0) for unknown zones — incorrect for scheduling, but
/// safe for display since the user sees UTC times rather than silently wrong
/// local times. Write paths must use `offset_minutes` and reject None.
pub fn offset_minutes_or_utc(tz: &str) -> i32 {
    offset_minutes(tz).unwrap_or(0)
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
    if parts.len() < 2 {
        return fallback;
    }
    let date_str = parts[0];
    let time_str = parts[1].get(..5).unwrap_or("");
    if time_str.len() < 5 {
        return fallback;
    }

    let segs: Vec<&str> = date_str.split('-').collect();
    if segs.len() < 3 {
        return fallback;
    }
    let year: i32 = segs[0].parse().unwrap_or(0);
    let month: i32 = segs[1].parse().unwrap_or(0);
    let day: i32 = segs[2].parse().unwrap_or(0);
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

    (
        format!("{fy:04}-{fm:02}-{fd:02}"),
        format!("{lh:02}:{lm:02}"),
    )
}

/// Convert a community-local date + "HH:MM" time to a UTC ISO-8601 string
/// `"YYYY-MM-DDTHH:MM:00.000Z"`. Inverse of `to_local_parts`: subtracts the
/// offset (UTC = local − offset). Handles day wrap across the conversion.
/// On unparseable input, falls back to appending the input as-is (degrades to
/// previous behaviour rather than panicking).
pub fn local_to_utc(date: &str, time: &str, offset_mins: i32) -> String {
    let fallback = format!("{date}T{time}:00.000Z");

    let segs: Vec<&str> = date.split('-').collect();
    if segs.len() < 3 {
        return fallback;
    }
    let year: i32 = match segs[0].parse() {
        Ok(v) => v,
        Err(_) => return fallback,
    };
    let month: i32 = match segs[1].parse() {
        Ok(v) => v,
        Err(_) => return fallback,
    };
    let day: i32 = match segs[2].parse() {
        Ok(v) => v,
        Err(_) => return fallback,
    };
    if time.len() < 5 {
        return fallback;
    }
    let h: i32 = match time.get(..2).and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return fallback,
    };
    let m: i32 = match time.get(3..5).and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return fallback,
    };

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
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
mod tests;

// ── Date label formatting (RFC-047) ──────────────────────────────────────
//
// Day-of-week for a Gregorian date via Zeller's congruence. Returns 0=Sunday
// .. 6=Saturday. Pure arithmetic, no external deps.
pub fn weekday_index(year: i32, month: i32, day: i32) -> i32 {
    // Zeller's congruence (Gregorian). Treat Jan/Feb as months 13/14 of prior year.
    let (m, y) = if month < 3 {
        (month + 12, year - 1)
    } else {
        (month, year)
    };
    let k = y % 100;
    let j = y / 100;
    let h = (day + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Zeller: 0=Saturday..6=Friday. Convert to 0=Sunday..6=Saturday.
    (h + 6) % 7
}

/// Japanese single-character weekday label for 0=Sunday..6=Saturday.
pub fn weekday_ja(index: i32) -> &'static str {
    match index.rem_euclid(7) {
        0 => "日",
        1 => "月",
        2 => "火",
        3 => "水",
        4 => "木",
        5 => "金",
        _ => "土",
    }
}

/// English 3-letter month abbreviation for 1..=12.
pub fn month_abbr_en(month: i32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

/// Format a `YYYY-MM-DD` local date as a Japanese calendar label with weekday,
/// e.g. `"6月14日（土）"`. Falls back to the raw string if unparseable.
pub fn date_label_ja(local_date: &str) -> String {
    let segs: Vec<&str> = local_date.split('-').collect();
    if segs.len() < 3 {
        return local_date.to_owned();
    }
    let (y, m, d) = match (
        segs[0].parse::<i32>(),
        segs[1].parse::<i32>(),
        segs[2].parse::<i32>(),
    ) {
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
    if segs.len() < 3 {
        return local_date.to_owned();
    }
    let (m, d) = match (segs[1].parse::<i32>(), segs[2].parse::<i32>()) {
        (Ok(m), Ok(d)) => (m, d),
        _ => return local_date.to_owned(),
    };
    format!("{d} {}", month_abbr_en(m))
}

#[cfg(test)]
mod date_label_tests;
