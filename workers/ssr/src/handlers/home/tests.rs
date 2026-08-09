use super::*;
use zinnias_ciao_contracts::Locale;

fn event_row(
    day_id: &str,
    event_id: &str,
    community_id: &str,
    occurrence_status: &str,
) -> event_db::HomeEventRow {
    event_db::HomeEventRow {
        community_id: community_id.to_string(),
        event_id: event_id.to_string(),
        event_title: "Sample Event".to_string(),
        event_location: None,
        event_status: "scheduled".to_string(),
        day_id: day_id.to_string(),
        day_date: "2026-07-05".to_string(),
        starts_at_utc: "2026-07-05T00:00:00Z".to_string(),
        ends_at_utc: "2026-07-05T01:00:00Z".to_string(),
        occurrence_status: occurrence_status.to_string(),
    }
}

fn community(community_id: &str) -> membership_db::CommunitySummary {
    membership_db::CommunitySummary {
        community_id: community_id.to_string(),
        community_name: "Community A".to_string(),
        timezone: "Asia/Tokyo".to_string(),
        role: "member".to_string(),
    }
}

#[test]
fn render_home_communities_cancelled_badge_follows_locale() {
    let communities = vec![community("community-a")];
    let rows = vec![event_row("day_1", "event_1", "community-a", "cancelled")];

    let ja = render_home_communities(&communities, &rows, Locale::Ja);
    assert!(ja.contains(i18n::JA_OCCURRENCE_CANCELLED_BADGE));
    assert!(!ja.contains(i18n::EN_OCCURRENCE_CANCELLED_BADGE));

    let en = render_home_communities(&communities, &rows, Locale::En);
    assert!(en.contains(i18n::EN_OCCURRENCE_CANCELLED_BADGE));
    assert!(!en.contains(i18n::JA_OCCURRENCE_CANCELLED_BADGE));
}

#[test]
fn render_home_communities_empty_state_follows_locale() {
    let communities = vec![community("community-a")];
    let rows: Vec<event_db::HomeEventRow> = vec![];

    let ja = render_home_communities(&communities, &rows, Locale::Ja);
    assert!(ja.contains(i18n::JA_HOME_CALENDAR_EMPTY));
    assert!(!ja.contains(i18n::EN_HOME_CALENDAR_EMPTY));

    let en = render_home_communities(&communities, &rows, Locale::En);
    assert!(en.contains(i18n::EN_HOME_CALENDAR_EMPTY));
    assert!(!en.contains(i18n::JA_HOME_CALENDAR_EMPTY));
}
