use super::shell::shell;
use worker::{Response, Result};
use zinnias_ciao_contracts::i18n;

pub fn placeholder() -> Result<Response> {
    let body = format!(
        "<main style=\"padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto\">\
  <h1 style=\"font-size:1.25rem;font-weight:600\">{}</h1>\
  <p style=\"color:#6E6E73;font-size:.875rem\">{}</p>\
</main>",
        i18n::JA_JOIN_HEADING,
        i18n::JA_GENERAL_ERROR,
    );
    Response::from_html(shell(i18n::JA_JOIN_HEADING, &body))
}

pub fn not_found() -> Result<Response> {
    let body = format!(
        "<main style=\"padding:2rem\"><p>{}</p></main>",
        i18n::JA_NOT_FOUND
    );
    Ok(Response::from_html(shell(i18n::JA_NOT_FOUND, &body))?.with_status(404))
}

pub fn internal_error() -> Result<Response> {
    let body = format!(
        "<main style=\"padding:2rem\"><p>{}</p></main>",
        i18n::JA_INTERNAL_ERROR
    );
    Ok(Response::from_html(shell(i18n::JA_GENERAL_ERROR, &body))?.with_status(500))
}

pub fn session_expired() -> Result<Response> {
    let body = format!(
        "<main style=\"padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto\">\
         <p style=\"color:#FF3B30\">{msg}</p>\
         <a href=\"/join\" style=\"display:inline-block;margin-top:1rem;color:#007AFF\">{join}</a></main>",
        msg = i18n::JA_SESSION_EXPIRED,
        join = i18n::JA_JOIN_SUBMIT,
    );
    Ok(Response::from_html(shell(i18n::JA_GENERAL_ERROR, &body))?.with_status(401))
}
