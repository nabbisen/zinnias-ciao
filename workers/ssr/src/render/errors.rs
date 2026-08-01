use super::shell::shell;
use worker::{Response, Result};
use zinnias_ciao_contracts::i18n;

pub fn placeholder() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
  <h1 class=\"cz-anon-title\">{}</h1>\
  <p class=\"cz-anon-hint-text\">{}</p>\
</main>",
        i18n::JA_JOIN_HEADING,
        i18n::JA_GENERAL_ERROR,
    );
    Response::from_html(shell(i18n::JA_JOIN_HEADING, &body))
}

pub fn not_found() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <p>{}</p>{}</main>",
        i18n::JA_NOT_FOUND,
        recovery_links()
    );
    Ok(Response::from_html(shell(i18n::JA_NOT_FOUND, &body))?.with_status(404))
}

pub fn internal_error() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <p>{}</p>{}</main>",
        i18n::JA_INTERNAL_ERROR,
        recovery_links()
    );
    Ok(Response::from_html(shell(i18n::JA_GENERAL_ERROR, &body))?.with_status(500))
}

pub fn service_unavailable() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <p>{}</p>{}</main>",
        i18n::JA_GENERAL_ERROR,
        recovery_links()
    );
    Ok(Response::from_html(shell(i18n::JA_GENERAL_ERROR, &body))?.with_status(503))
}

/// Fixed RFC-077 response for unavailable security configuration.
/// It intentionally contains no recovery links or configuration details.
pub fn configuration_unavailable() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <p>{}</p></main>",
        i18n::JA_CONFIGURATION_UNAVAILABLE,
    );
    Ok(Response::from_html(shell(i18n::JA_CONFIGURATION_UNAVAILABLE, &body))?.with_status(503))
}

pub fn session_expired() -> Result<Response> {
    let body = format!(
        "<main class=\"cz-anon-main\">\
         <p class=\"cz-anon-error-text\">{msg}</p>\
         <div class=\"cz-error-recovery-links\">\
           <a href=\"/relink\" class=\"cz-error-recovery-link\">\
             {relink}</a>\
           <a href=\"/join\" class=\"cz-error-recovery-link\">\
             {join}</a>\
         </div></main>",
        msg = i18n::JA_SESSION_EXPIRED,
        join = i18n::JA_JOIN_SUBMIT,
        relink = i18n::JA_JOIN_RELINK_LINK,
    );
    Ok(Response::from_html(shell(i18n::JA_GENERAL_ERROR, &body))?.with_status(401))
}

fn recovery_links() -> String {
    format!(
        "<div class=\"cz-error-recovery-links\">\
           <a href=\"/\" class=\"cz-error-recovery-link\">{home}</a>\
           <a href=\"/join\" class=\"cz-error-recovery-link\">{join}</a>\
         </div>",
        home = i18n::JA_NAV_HOME,
        join = i18n::JA_JOIN_SUBMIT,
    )
}
