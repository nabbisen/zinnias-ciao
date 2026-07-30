use super::detail::render_date_detail;
use crate::db::attendance;
use crate::db::event as event_db;
use std::collections::HashMap;
use zinnias_ciao_contracts::{Locale, i18n};

#[test]
fn date_detail_no_day_selected_follows_locale() {
    let rows_by_date: HashMap<String, Vec<&event_db::HomeEventRow>> = HashMap::new();
    let attendances: HashMap<String, Vec<attendance::AttendanceRow>> = HashMap::new();

    let ja = render_date_detail(
        "community-a",
        "Asia/Tokyo",
        None,
        &rows_by_date,
        3,
        &attendances,
        Locale::Ja,
    );
    assert!(ja.contains(&format!(">{}</h3>", i18n::JA_HOME_AGENDA_TITLE)));
    assert!(ja.contains(i18n::JA_CALENDAR_EMPTY_MONTH));
    assert!(!ja.contains(&format!(">{}</h3>", i18n::EN_HOME_AGENDA_TITLE)));
    assert!(!ja.contains(i18n::EN_CALENDAR_EMPTY_MONTH));

    let en = render_date_detail(
        "community-a",
        "Asia/Tokyo",
        None,
        &rows_by_date,
        3,
        &attendances,
        Locale::En,
    );
    assert!(en.contains(&format!(">{}</h3>", i18n::EN_HOME_AGENDA_TITLE)));
    assert!(en.contains(i18n::EN_CALENDAR_EMPTY_MONTH));
    assert!(!en.contains(&format!(">{}</h3>", i18n::JA_HOME_AGENDA_TITLE)));
    assert!(!en.contains(i18n::JA_CALENDAR_EMPTY_MONTH));
}
