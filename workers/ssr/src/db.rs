#![allow(dead_code)]
//! D1 data-access layer.

pub mod attendance;
pub mod community;
pub mod event;
pub mod event_note;
pub mod event_write;
pub mod invite;
pub mod membership;
pub mod session;

use worker::D1Database;
pub type Db<'a> = &'a D1Database;

pub fn now_utc() -> String {
    let ms = worker::Date::now().as_millis();
    let secs = ms / 1000;
    let millis = ms % 1000;
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

pub fn add_seconds_to_now(seconds: u64) -> String {
    let ms = worker::Date::now().as_millis() + seconds * 1000;
    let secs = ms / 1000;
    let millis = ms % 1000;
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

pub fn utc_days_ahead(days: u64) -> String {
    let ms = worker::Date::now().as_millis() + days * 86_400 * 1_000;
    let secs = ms / 1000;
    let (y, mo, d, _, _, _) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T00:00:00.000Z")
}

fn epoch_to_ymd_hms(epoch_secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s  = (epoch_secs % 60) as u32;
    let mi = ((epoch_secs / 60) % 60) as u32;
    let h  = ((epoch_secs / 3600) % 24) as u32;
    let days = epoch_secs / 86400;
    let z   = days + 719468;
    let era = z / 146097;
    let doe = z % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo  = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y   = if mo <= 2 { y + 1 } else { y } as u32;
    (y, mo, d, h, mi, s)
}
