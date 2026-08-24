//! Admin help-signin handlers — RFC-024.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::i18n;

use crate::authz::require_admin;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::{membership as membership_db, relink as relink_db};
use crate::render;

fn redirect(url: &str) -> Result<Response> {
    let mut r = Response::empty()?;
    r.headers_mut().set("Location", url)?;
    Ok(r.with_status(303))
}

async fn community_pairs_for_user(
    db: &worker::D1Database,
    user_id: &str,
    scope_community_id: Option<&str>,
) -> Vec<(String, String)> {
    membership_db::list_communities_for_user(db, user_id, scope_community_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|c| (c.community_id.clone(), c.community_name.clone()))
        .collect()
}

// ── GET /c/:cid/admin/members/:mid/help-signin ───────────────────────────

pub async fn get_help_signin(
    req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;
    let target =
        match membership_db::find_active_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };
    let token = crate::codlet::issue_token(
        env,
        &auth.user_id,
        token_purpose::HELP_SIGNIN,
        Some(target_membership_id),
    )
    .await?;
    let community_pairs =
        community_pairs_for_user(&db, &auth.user_id, auth.scope_community_id.as_deref()).await;
    let nav = render::bottom_nav_localized(community_id, "home", locale);

    let title = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_TITLE);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{title}</h1>\
         <p class=\"cz-admin-confirm-subtitle\">\
           <strong>{name}</strong><br>{consequence}\
         </p>\
         <div class=\"cz-admin-role-actions\">\
           <a href=\"/c/{cid}/admin/members\" \
              class=\"cz-admin-role-keep-link\">\
              {keep}</a>\
           <form method=\"post\" action=\"/c/{cid}/admin/members/{mid}/help-signin\" class=\"cz-confirm-delete-form\">\
             <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
             <button type=\"submit\" \
               class=\"cz-admin-role-confirm-button\">\
               {confirm}</button>\
           </form>\
         </div></main>{nav}",
        header = render::header_with_switcher_next_localized(
            title,
            community_id,
            &community_pairs,
            "admin_members",
            locale,
        ),
        title = title,
        name = render::escape_html(&target.display_name),
        consequence = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_CONSEQUENCE),
        cid = render::escape_html(community_id),
        mid = render::escape_html(target_membership_id),
        tok = render::escape_html(&token),
        keep = i18n::t(locale, i18n::ADMIN_REMOVE_KEEP),
        confirm = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_CREATE),
        nav = nav,
    );
    render::page_localized(locale, title, &body)
}

// ── POST /c/:cid/admin/members/:mid/help-signin ──────────────────────────

pub async fn post_help_signin(
    mut req: Request,
    env: &Env,
    rid: &str,
    community_id: &str,
    target_membership_id: &str,
) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, render::session_expired());
    let membership = require_admin(env, &auth, community_id, rid).await?;
    let locale = membership.locale;
    let db = env.d1("DB")?;

    let target =
        match membership_db::find_active_summary(&db, target_membership_id, community_id).await? {
            Some(target) => target,
            None => return render::not_found(),
        };

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let replay = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::HELP_SIGNIN,
        &raw_token,
        Some(target_membership_id),
    )
    .await?;
    if matches!(replay, crate::codlet::ConsumeResult::Replay(_)) {
        return redirect(&format!("/c/{community_id}/admin/members"));
    }

    let code = random_token()[..16].to_ascii_uppercase();
    let normalized = normalize_invite_code(&code);
    let pepper = crate::crypto::pepper(env)?;
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let code_id = random_token()[..24].to_owned();
    let expires_at = relink_db::expires_at();

    if !relink_db::issue_required(
        &db,
        rid,
        &code_id,
        &code_hmac,
        community_id,
        target_membership_id,
        &membership.membership_id,
        &expires_at,
        None,
    )
    .await?
    {
        return render::not_found();
    }

    let nav = render::bottom_nav_localized(community_id, "home", locale);
    let title = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_TITLE);
    let body = format!(
        "{header}\
         <main class=\"cz-page-main\">\
         <h1 class=\"cz-admin-title cz-admin-title--snug\">{title}</h1>\
         <p class=\"cz-admin-code-hint\">\
           <strong>{name}</strong><br>{hint}</p>\
         <div class=\"cz-admin-reveal-box\">\
           <div class=\"cz-admin-code-display\" aria-label=\"{code_label}\" data-copy-code-value=\"true\">{code}</div>\
           <button type=\"button\" data-copy-code-button=\"true\" hidden \
             data-copy-success=\"{copy_done}\" data-copy-error=\"{copy_failed}\" \
             class=\"cz-admin-copy-button\">{copy_code}</button>\
           <span data-copy-code-status=\"true\" aria-live=\"polite\" \
             class=\"cz-admin-copy-status\"></span>\
         </div>\
         <p class=\"cz-admin-relink-hint\">{relink_hint}</p>\
         <p><a href=\"/relink\" target=\"_blank\" rel=\"noopener\" \
           class=\"cz-plain-link\">{relink_link}</a></p>\
         <p><a href=\"/c/{cid}/admin/members\" \
           class=\"cz-plain-link\">{back}</a></p>\
         </main>{nav}",
        header = render::header_with_switcher_next_localized(
            title,
            community_id,
            &community_pairs_for_user(&db, &auth.user_id, auth.scope_community_id.as_deref()).await,
            "admin_members",
            locale,
        ),
        title = title,
        name = render::escape_html(&target.display_name),
        hint = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_CODE_HINT),
        code_label = i18n::t(locale, i18n::RELINK_CODE_LABEL),
        code = render::escape_html(&code),
        copy_code = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_COPY_CODE),
        copy_done = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_COPY_DONE),
        copy_failed = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_COPY_FAILED),
        relink_hint = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_RELINK_HINT),
        relink_link = i18n::t(locale, i18n::ADMIN_HELP_SIGNIN_RELINK_LINK),
        cid = render::escape_html(community_id),
        back = i18n::t(locale, i18n::ADMIN_INVITES_BACK_TO_MEMBERS),
        nav = nav,
    );
    render::page_localized(locale, title, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zinnias_ciao_contracts::Locale;

    /// Duplicated locally per `admin/members.rs`'s own precedent (Handoff
    /// 072) rather than shared/exported.
    fn contains_japanese_codepoint(s: &str) -> bool {
        s.chars().any(|c| {
            let cp = c as u32;
            (0x3040..=0x30FF).contains(&cp)
                || (0x4E00..=0x9FFF).contains(&cp)
                || (0x3000..=0x303F).contains(&cp)
                || (0xFF00..=0xFFEF).contains(&cp)
        })
    }

    /// Handoff 072 §4.4: second of the required ≥2 pages, covering the
    /// nav/header leak class §3 warns about (`bottom_nav`/
    /// `header_with_switcher_next` are locale-blind and invisible to a
    /// source scan) on a different page than `members.rs`'s own test.
    #[test]
    fn admin_help_signin_page_renders_with_no_japanese_codepoint_in_english_locale() {
        let header = render::header_with_switcher_next_localized(
            i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_TITLE),
            "community-a",
            &[("community-a".to_string(), "Community A".to_string())],
            "admin_members",
            Locale::En,
        );
        let nav = render::bottom_nav_localized("community-a", "home", Locale::En);
        let body = format!(
            "{consequence}{keep}{confirm}{hint}{code_label}{copy_code}{copy_done}{copy_failed}{relink_hint}{relink_link}{back}",
            consequence = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_CONSEQUENCE),
            keep = i18n::t(Locale::En, i18n::ADMIN_REMOVE_KEEP),
            confirm = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_CREATE),
            hint = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_CODE_HINT),
            code_label = i18n::t(Locale::En, i18n::RELINK_CODE_LABEL),
            copy_code = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_COPY_CODE),
            copy_done = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_COPY_DONE),
            copy_failed = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_COPY_FAILED),
            relink_hint = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_RELINK_HINT),
            relink_link = i18n::t(Locale::En, i18n::ADMIN_HELP_SIGNIN_RELINK_LINK),
            back = i18n::t(Locale::En, i18n::ADMIN_INVITES_BACK_TO_MEMBERS),
        );
        let en_page = format!("{header}<main>{body}</main>{nav}");

        assert!(
            !contains_japanese_codepoint(&en_page),
            "English-locale admin help-signin page must contain no Japanese codepoint, found some in: {en_page}"
        );

        // Sanity: the same nav/header composition at Locale::Ja must
        // contain Japanese.
        let ja_header = render::header_with_switcher_next_localized(
            i18n::t(Locale::Ja, i18n::ADMIN_HELP_SIGNIN_TITLE),
            "community-a",
            &[("community-a".to_string(), "Community A".to_string())],
            "admin_members",
            Locale::Ja,
        );
        let ja_nav = render::bottom_nav_localized("community-a", "home", Locale::Ja);
        assert!(
            contains_japanese_codepoint(&ja_header),
            "Japanese-locale header render must contain Japanese text"
        );
        assert!(
            contains_japanese_codepoint(&ja_nav),
            "Japanese-locale nav render must contain Japanese text"
        );
    }
}
