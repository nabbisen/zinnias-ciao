//! Regenerating the recovery credential — RFC-081 §3.1 (Handoff 057 §5.1).
//!
//! One-click, no separate confirmation step (unlike link and unlink): the
//! handoff does not ask for one here, and the immediate reveal makes the
//! outcome self-evident the same way `handlers/admin/members.rs`'s invite
//! generation already is. Doubles as first-time generation for a member
//! linked before this package existed and holding no credential yet.

use worker::{Env, Request, Response, Result};
use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_domain::invite::INVITE_CODE_ALPHABET;
use zinnias_ciao_domain::recovery::ACCOUNT_RECOVERY_CODE_LEN;

use crate::authz;
use crate::crypto::{hmac_hex, normalize_invite_code, random_token};
use crate::db::recovery as recovery_db;
use crate::form_token::ConsumeResult;

fn redirect(url: &str) -> Result<Response> {
    let mut resp = Response::empty()?.with_status(303);
    resp.headers_mut().set("Location", url)?;
    Ok(resp)
}

/// Shared with `handlers/identity/mod.rs::link_outcome`, which issues the
/// member's very first recovery credential atomically alongside their
/// first link — the code shape must not drift between the two issuance
/// sites.
pub(crate) fn generate_code() -> Result<String> {
    let alpha_len = INVITE_CODE_ALPHABET.len();
    let ceiling = 256 - (256 % alpha_len);
    let mut raw = String::with_capacity(ACCOUNT_RECOVERY_CODE_LEN);
    while raw.len() < ACCOUNT_RECOVERY_CODE_LEN {
        let mut buf = [0u8; 1];
        getrandom::fill(&mut buf).map_err(|e| worker::Error::RustError(format!("rng: {e}")))?;
        let b = buf[0] as usize;
        if b < ceiling {
            raw.push(INVITE_CODE_ALPHABET[b % alpha_len] as char);
        }
    }
    // Grouped for readability (Handoff 057 §5.1: "a shape a non-technical
    // member can read aloud and retype") — `normalize_invite_code` strips
    // these hyphens back out before hashing or lookup, the same way it
    // already does for invite/relink codes.
    Ok(raw
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-"))
}

pub async fn post_regenerate(mut req: Request, env: &Env, rid: &str) -> Result<Response> {
    let auth = crate::require_auth_or!(&req, env, crate::render::session_expired());
    authz::require_account_surface(env, &auth, rid).await?;

    let freshness_window_start =
        crate::db::subtract_seconds_from_now(authz::ACCOUNT_OPERATION_FRESHNESS_SECONDS);
    if !authz::is_fresh_for_account_operations(&auth, &freshness_window_start) {
        return redirect("/identity/start?action=sign_in");
    }

    let body = req.form_data().await?;
    let raw_token = body.get_field("_token").unwrap_or_default();
    let consumed = crate::codlet::consume_token(
        env,
        &auth.user_id,
        token_purpose::REGENERATE_RECOVERY,
        &raw_token,
        None,
    )
    .await?;
    if matches!(consumed, ConsumeResult::Replay(_)) {
        return redirect("/account");
    }

    let pepper = crate::crypto::pepper(env)?;
    let db = env.d1("DB")?;
    let code = generate_code()?;
    let normalized = normalize_invite_code(&code);
    let code_hmac = hmac_hex(pepper.as_str(), &normalized);
    let credential_id = format!("rec_{}", &random_token()[..24]);

    recovery_db::regenerate_required(&db, rid, &auth.user_id, &credential_id, &code_hmac).await?;

    super::render_account_page(env, &req, &auth.user_id, true, Some(&code)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use zinnias_ciao_domain::recovery::validate_recovery_code_input;

    #[test]
    fn generated_code_normalizes_to_the_expected_length_and_alphabet() {
        for _ in 0..50 {
            let code = generate_code().unwrap();
            assert!(
                validate_recovery_code_input(&code).is_ok(),
                "generated code {code:?} failed its own domain validation"
            );
            let normalized = normalize_invite_code(&code);
            assert_eq!(normalized.len(), ACCOUNT_RECOVERY_CODE_LEN);
            assert!(
                normalized
                    .chars()
                    .all(|c| INVITE_CODE_ALPHABET.contains(&(c as u8)))
            );
        }
    }

    #[test]
    fn generated_code_is_grouped_with_hyphens_for_readability() {
        let code = generate_code().unwrap();
        assert!(
            code.contains('-'),
            "expected a grouped, hyphenated code, got {code:?}"
        );
    }

    #[test]
    fn repeated_generation_is_not_constant() {
        // Cheap sanity check that this is actually drawing from the RNG,
        // not returning a fixed value — vanishingly unlikely to collide
        // twice in a row for a 12-character, 32-symbol alphabet.
        let a = generate_code().unwrap();
        let b = generate_code().unwrap();
        assert_ne!(a, b);
    }
}
