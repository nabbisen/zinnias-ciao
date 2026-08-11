//! UI string constants (i18n scaffold — RFC-026).
//!
//! All user-visible strings are collected here so they can be translated
//! without touching handler logic.  Currently English only; Japanese parity
//! is enforced by the i18n lint test below.
//!
//! Naming: `<LANG>_<CONTEXT>_<KEY>` in SCREAMING_SNAKE_CASE.

mod access;
mod account;
mod admin;
mod calendar;
mod community;
mod events;
mod export;
mod general;
mod home;
mod me;
mod notes;
mod templates;

pub use access::*;
pub use account::*;
pub use admin::*;
pub use calendar::*;
pub use community::*;
pub use events::*;
pub use export::*;
pub use general::*;
pub use home::*;
pub use me::*;
pub use notes::*;
pub use templates::*;

use crate::locale::Locale;

/// A JA/EN string pair, resolved at render time by [`t`] (RFC-072). This is
/// the entire i18n accessor boundary: migrating a page to a locale replaces
/// `i18n::JA_X` with `i18n::t(locale, i18n::X)`, once per string, with no
/// per-call-site `match locale { Ja => JA_X, En => EN_X }`. Adding a new
/// pair costs one line beside its existing `EN_X`/`JA_X` constants — see
/// e.g. `me.rs`'s `ME_SECTION_NAME` — and nothing about `t` itself changes.
#[derive(Debug, Clone, Copy)]
pub struct Localized {
    pub ja: &'static str,
    pub en: &'static str,
}

/// Resolves a [`Localized`] pair to the string for `locale`. Both fields
/// are named, compile-time-checked constants, not a runtime key lookup —
/// a missing translation stays a compile error, not a render-time miss.
/// RFC-072 rejects a runtime message-catalogue for exactly that reason.
pub fn t(locale: Locale, pair: Localized) -> &'static str {
    match locale {
        Locale::Ja => pair.ja,
        Locale::En => pair.en,
    }
}

#[cfg(test)]
mod tests;
