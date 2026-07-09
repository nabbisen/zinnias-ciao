use std::collections::HashMap;

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::audit;
use crate::authz::require_admin;
use crate::db::{event as event_db, membership as membership_db};
use crate::render;
use crate::session::require_auth;

use super::support::redirect;

pub async fn get_attendance(
    req: Request,
    env: &Env,
    _rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let _membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    // Only allow attendance correction after the event (status=ended or any non-scheduled)
    // For MVP we allow it for any non-cancelled event (the admin controls when to correct).
    if event.status == "cancelled" {
        return render::page(
            i18n::JA_GENERAL_ERROR,
            &format!(
                "<main style=\"padding:2rem\"><p>{}</p><p><a href=\"javascript:history.back()\">{}</a></p></main>",
                i18n::JA_ADMIN_ATTEND_CANCELLED,
                i18n::JA_GENERAL_BACK
            ),
        );
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE,
        Some(event_id),
    )
    .await;

    let communities_for_switcher = membership_db::list_communities_for_user(&db, &auth.user_id)
        .await
        .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav(community_id, "home");

    let mut days_html = String::new();
    for day in &days {
        let attendances = crate::db::attendance::list_for_day(&db, &day.id).await?;
        let att_map: HashMap<&str, Option<&str>> = attendances
            .iter()
            .map(|a| (a.membership_id.as_str(), a.status.as_deref()))
            .collect();

        let day_label = render::escape_html(&day.day_date);
        days_html.push_str(&format!(
            "<h3 style=\"font-size:.9375rem;font-weight:600;margin:1rem 0 .5rem\">{day_label}</h3>"
        ));

        for m in &members {
            let current = att_map.get(m.id.as_str()).copied().flatten();
            let sel = |v: &str| if current == Some(v) { " selected" } else { "" };
            days_html.push_str(&format!(
                "<div style=\"display:flex;align-items:center;gap:.75rem;padding:.5rem 0;\
                 border-bottom:1px solid #F5F5F7\">\
                 <span style=\"flex:1;font-size:.9375rem\">{name}</span>\
                 <select name=\"att_{day_id}_{mid}\" \
                   style=\"font-size:.875rem;padding:.375rem .5rem;border:1px solid #E5E5EA;\
                   border-radius:8px;min-height:44px\" \
                   aria-label=\"Attendance for {name_raw}\">\
                   <option value=\"\"{no_ans}>{opt_na}</option>\
                   <option value=\"going\"{going}>{opt_go}</option>\
                   <option value=\"not_going\"{notgoing}>{opt_ng}</option>\
                   <option value=\"attended\"{attended}>{opt_at}</option>\
                 </select>\
                 </div>",
                name = render::escape_html(&m.display_name),
                name_raw = render::escape_html(&m.display_name),
                day_id = render::escape_html(&day.id),
                mid = render::escape_html(&m.id),
                no_ans = if current.is_none() { " selected" } else { "" },
                going = sel("going"),
                notgoing = sel("not_going"),
                opt_na = i18n::JA_STATUS_NO_ANSWER,
                opt_go = i18n::JA_STATUS_GOING,
                opt_ng = i18n::JA_STATUS_NOT_GOING,
                opt_at = i18n::JA_STATUS_ATTENDED,
                attended = sel("attended"),
            ));
        }
    }

    let flash: Option<String> = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = flash
        .map(|f| {
            format!(
                "<p role=\"status\" style=\"color:#167A34;font-size:.875rem;margin-bottom:1rem\">{}</p>",
                render::escape_html(&f)
            )
        })
        .unwrap_or_default();

    let body = format!(
        "{header}\
         <main style=\"padding:1rem 1rem 5rem\">\
         <h1 style=\"font-size:1.25rem;font-weight:600;margin-bottom:.25rem\">{at}</h1>\
         <p style=\"font-size:.875rem;color:#6E6E73;margin-bottom:1rem\">{title}</p>\
         {flash}\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/attendance\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {days}\
           <button type=\"submit\" \
             style=\"width:100%;padding:.875rem;background:#007AFF;color:#fff;\
             border:none;border-radius:14px;font-size:1rem;font-weight:600;\
             min-height:44px;cursor:pointer;margin-top:1.5rem\">{aas}</button>\
         </form>\
         <div style=\"margin-top:1rem\">\
           <a href=\"/c/{cid}/events/{eid}\" style=\"color:#6E6E73;font-size:.875rem\">\
             {back}</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher(
            i18n::JA_ADMIN_ATTEND_TITLE,
            community_id,
            &community_pairs
        ),
        title = render::escape_html(&event.title),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        days = days_html,
        flash = flash_html,
        nav = nav,
        at = i18n::JA_ADMIN_ATTEND_TITLE,
        aas = i18n::JA_ADMIN_ATTEND_SUBMIT,
        back = i18n::JA_NAV_BACK,
    );
    render::page(i18n::JA_ADMIN_ATTEND_TITLE, &body)
}

pub async fn post_attendance(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = match require_auth(&req, env).await {
        Ok(a) => a,
        Err(_) => return render::session_expired(),
    };
    let membership = require_admin(env, &auth, community_id).await?;
    let db = env.d1("DB")?;

    let form = req.form_data().await?;
    let raw_token = form.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::ATTENDANCE_OVERRIDE,
        &raw_token,
        Some(event_id),
    )
    .await?;
    if replay.is_some() {
        return redirect(&format!("/c/{community_id}/events/{event_id}"));
    }

    if event_db::find_for_community(&db, event_id, community_id)
        .await?
        .is_none()
    {
        return render::not_found();
    }

    let days = event_db::days_for_event(&db, event_id).await?;
    let members = membership_db::list_all_active(&db, community_id).await?;

    let mut changes: u32 = 0;
    for day in &days {
        for m in &members {
            let field_name = format!("att_{}_{}", day.id, m.id);
            let value = form.get_field(&field_name).unwrap_or_default();
            let status: Option<&str> = match value.as_str() {
                "going" => Some("going"),
                "not_going" => Some("not_going"),
                "attended" => Some("attended"),
                _ => None,
            };
            crate::db::attendance::upsert(&db, &day.id, &m.id, status).await?;
            changes += 1;
        }
    }

    let _ = audit::write(
        &db,
        rid,
        Some(community_id),
        Some(&membership.membership_id),
        "attendance",
        Some(event_id),
        "admin_override",
        Some(serde_json::json!({ "changes": changes })),
    )
    .await;

    redirect(&format!(
        "/c/{community_id}/admin/events/{event_id}/attendance?flash=Saved"
    ))
}
