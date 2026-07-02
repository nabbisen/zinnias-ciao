//! Calendar page for the active community (RFC-056).

use worker::{Env, Request, Response, Result};

use crate::db::{self, event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;
use zinnias_ciao_contracts::{i18n, tz};

pub async fn get_communities(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let db = env.d1("DB")?;

    let summaries = membership_db::list_communities_for_user(&db, &auth.user_id).await?;
    if !summaries.iter().any(|s| s.community_id == community_id) {
        return render::not_found();
    }

    let community = db::community::find_active(&db, community_id).await?;
    let community_tz = community
        .as_ref()
        .map(|c| c.timezone.as_str())
        .unwrap_or("UTC");
    let now_prefix = db::now_utc();
    let tz_offset = tz::offset_minutes_or_utc(community_tz);
    let (today_date, _) = tz::to_local_parts(&now_prefix, tz_offset);
    let rows =
        event_db::home_upcoming(&db, community_id, &db::now_utc(), &db::utc_days_ahead(30)).await?;
    let calendar = super::home::render_month_calendar(&today_date, &rows);

    // Header uses list_communities_for_user result as switcher pairs.
    let community_pairs: Vec<(String, String)> = summaries
        .iter()
        .map(|s| (s.community_id.clone(), s.community_name.clone()))
        .collect();

    let nav = render::bottom_nav(community_id, "communities");
    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
           {calendar}\
         </main>{nav}",
        header = render::header_with_switcher_next(
            i18n::JA_NAV_COMMUNITIES,
            community_id,
            &community_pairs,
            "communities"
        ),
        calendar = calendar,
        nav = nav,
    );
    render::page(i18n::JA_NAV_COMMUNITIES, &body)
}
