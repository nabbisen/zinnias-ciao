use std::collections::HashMap;

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::db::{attendance as attendance_db, event as event_db, membership as membership_db};
use crate::render;

use super::support::redirect;

/// Handoff 037: the `calendar_flash_message` pattern. Handoff 062: locale-
/// aware (RFC-083 Slice D1a). Unknown codes return `None`; the caller must
/// render no flash element in that case, not echo the code.
fn attendance_flash_message(locale: Locale, code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("attendance_saved") => Some(i18n::t(locale, i18n::ADMIN_ATTENDANCE_SAVED_FLASH)),
        _ => None,
    }
}

/// Substitute a template's `{}` placeholders positionally, in order. Mirrors
/// `matrix::cells::substitute_positional` (RFC-072 Slice C) — duplicated
/// rather than imported since that helper is `pub(super)` to the `matrix`
/// module tree. Not `format!` (the template is a runtime `&str`, not a
/// literal). Excess values are ignored; a template asking for more values
/// than given leaves that placeholder's `{}` in the output rather than
/// panicking.
fn substitute_positional(template: &str, values: &[&str]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut next = values.iter();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'}') {
            chars.next();
            if let Some(value) = next.next() {
                result.push_str(value);
            } else {
                result.push_str("{}");
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub async fn get_attendance(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;

    let event = match event_db::find_for_community(&db, event_id, community_id).await? {
        Some(e) => e,
        None => return render::not_found(),
    };
    // Only allow attendance correction after the event (status=ended or any non-scheduled)
    // For MVP we allow it for any non-cancelled event (the admin controls when to correct).
    if event.status == "cancelled" {
        return render::page_localized(
            locale,
            i18n::t(locale, i18n::GENERAL_ERROR),
            &format!(
                "<main class=\"cz-admin-error-main\"><p>{}</p><p><a href=\"javascript:history.back()\">{}</a></p></main>",
                i18n::t(locale, i18n::ADMIN_ATTEND_CANCELLED),
                i18n::t(locale, i18n::GENERAL_BACK)
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
    .await?;

    let communities_for_switcher = membership_db::list_communities_for_user(
        &db,
        &auth.user_id,
        auth.scope_community_id.as_deref(),
    )
    .await
    .unwrap_or_default();
    let community_pairs: Vec<(String, String)> = communities_for_switcher
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect();
    let nav = render::bottom_nav_localized(community_id, "home", locale);

    let mut days_html = String::new();
    for day in &days {
        let attendances = crate::db::attendance::list_for_day(&db, &day.id).await?;
        let att_map: HashMap<&str, Option<&str>> = attendances
            .iter()
            .map(|a| (a.membership_id.as_str(), a.status.as_deref()))
            .collect();

        let day_label = render::escape_html(&day.day_date);
        days_html.push_str(&format!(
            "<h3 class=\"cz-admin-day-heading\">{day_label}</h3>"
        ));

        for m in &members {
            let current = att_map.get(m.id.as_str()).copied().flatten();
            let sel = |v: &str| if current == Some(v) { " selected" } else { "" };
            days_html.push_str(&format!(
                "<div class=\"cz-admin-attendance-row\">\
                 <span class=\"cz-admin-attendance-name\">{name}</span>\
                 <select name=\"att_{day_id}_{mid}\" \
                   class=\"cz-admin-attendance-select\" \
                   aria-label=\"{aria_label}\">\
                   <option value=\"\"{no_ans}>{opt_na}</option>\
                   <option value=\"going\"{going}>{opt_go}</option>\
                   <option value=\"not_going\"{notgoing}>{opt_ng}</option>\
                   <option value=\"attended\"{attended}>{opt_at}</option>\
                 </select>\
                 </div>",
                name = render::escape_html(&m.display_name),
                aria_label = substitute_positional(
                    i18n::t(locale, i18n::ADMIN_ATTEND_MEMBER_ARIA_LABEL),
                    &[&render::escape_html(&m.display_name)]
                ),
                day_id = render::escape_html(&day.id),
                mid = render::escape_html(&m.id),
                no_ans = if current.is_none() { " selected" } else { "" },
                going = sel("going"),
                notgoing = sel("not_going"),
                opt_na = i18n::t(locale, i18n::STATUS_NO_ANSWER),
                opt_go = i18n::t(locale, i18n::STATUS_GOING),
                opt_ng = i18n::t(locale, i18n::STATUS_NOT_GOING),
                opt_at = i18n::t(locale, i18n::STATUS_ATTENDED),
                attended = sel("attended"),
            ));
        }
    }

    let flash_code: Option<String> = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "flash")
        .map(|(_, v)| v.to_string());
    let flash_html = attendance_flash_message(locale, flash_code.as_deref())
        .map(|message| {
            format!(
                "<p role=\"status\" class=\"cz-admin-flash-success\">{}</p>",
                render::escape_html(message)
            )
        })
        .unwrap_or_default();

    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--tight\">{at}</h1>\
         <p class=\"cz-admin-subtitle\">{title}</p>\
         {flash}\
         <form method=\"post\" action=\"/c/{cid}/admin/events/{eid}/attendance\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           {days}\
           <button type=\"submit\" \
             class=\"cz-admin-submit-button cz-admin-submit-button--loose\">{aas}</button>\
         </form>\
         <div class=\"cz-admin-back-row--tight\">\
           <a href=\"/c/{cid}/events/{eid}\" class=\"cz-admin-back-link\">\
             {back}</a>\
         </div>\
         </main>{nav}",
        header = render::header_with_switcher_localized(
            i18n::t(locale, i18n::ADMIN_ATTEND_TITLE),
            community_id,
            &community_pairs,
            locale,
        ),
        title = render::escape_html(&event.title),
        cid = render::escape_html(community_id),
        eid = render::escape_html(event_id),
        tok = render::escape_html(&token),
        days = days_html,
        flash = flash_html,
        nav = nav,
        at = i18n::t(locale, i18n::ADMIN_ATTEND_TITLE),
        aas = i18n::t(locale, i18n::ADMIN_ATTEND_SUBMIT),
        back = i18n::t(locale, i18n::NAV_BACK),
    );
    render::page_localized(locale, i18n::t(locale, i18n::ADMIN_ATTEND_TITLE), &body)
}

pub async fn post_attendance(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    event_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
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
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
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

    let submitted_cells = days
        .len()
        .checked_mul(members.len())
        .ok_or_else(|| worker::Error::RustError("attendance override size overflow".to_owned()))?;
    if submitted_cells > attendance_db::ADMIN_OVERRIDE_CELL_CAP {
        return render::internal_error();
    }
    let mut submitted = Vec::with_capacity(submitted_cells);
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
            submitted.push(serde_json::json!({
                "day_id": day.id,
                "membership_id": m.id,
                "status": status,
            }));
        }
    }
    let submitted_json = serde_json::to_string(&submitted)
        .map_err(|_| worker::Error::RustError("attendance override encoding failed".to_owned()))?;
    attendance_db::admin_override_matrix(
        &db,
        rid,
        community_id,
        event_id,
        &membership.membership_id,
        &submitted_json,
        submitted_cells,
    )
    .await?;

    redirect(&format!(
        "/c/{community_id}/admin/events/{event_id}/attendance?flash=attendance_saved"
    ))
}

#[cfg(test)]
#[path = "attendance/tests.rs"]
mod tests;
