# RFC 049 — Japanese-Language Pilot Rendering

**Status.** Implemented (v0.30.0)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Hardening RFC. Closes architect-review v0.29.0 P1 (mixed-language UX — Japanese dates with English buttons). Resolves the hybrid created by RFC-047 (Japanese date labels) without a full runtime locale-switching system. Accompanies RFC-048 as a joint pre-pilot milestone.

---

## 1. Summary

After RFC-047, day labels rendered Japanese dates (`6月14日（土）`) but all other UI strings — buttons, nav tabs, status labels, headings, confirmations — still used `i18n::EN_*` constants. The app was advertising a Japan-first deployment while presenting a hybrid Japanese/English interface.

The architect review (v0.29.0, P1) made the decision explicit: for the Japan pilot, make the core flow consistently Japanese, or document that the pilot is English-first.

This RFC makes the app **Japanese-first** by switching all handler and render call sites from `i18n::EN_*` to `i18n::JA_*` and changing the HTML `lang` attribute from `en` to `ja`. The EN/JA constants remain in `i18n.rs` for future runtime locale switching; this change selects the language at compile time.

## 2. Motivation

For non-technical Japanese users, an English UI creates friction and reduces trust. "Going / No Go / No answer" are less intuitive than their Japanese equivalents for a user who primarily reads Japanese. A hybrid interface signals that the product was not built for them.

The 120 JA strings already existed and were validated by the parity test (RFC-028). No new translation work was required — the infrastructure was already there.

## 3. Goals

- All user-visible UI strings render from `JA_*` constants.
- HTML `lang` attribute is `"ja"`.
- Japanese date labels (RFC-047) remain consistent with the rest of the UI.
- EN strings remain available in `i18n.rs` for future runtime language selection.
- Zero new compile errors (the JA strings are validated by the 120-pair parity test).

## 4. Non-Goals

- Runtime `Accept-Language`-based locale selection. This is a future RFC.
- A `Lang` enum threading through all render functions. Deferred.
- Translating error messages that embed dynamic values (event titles, etc.). The JA error-text constants already exist; callers use them.
- Operator/admin documentation strings (in English by convention).

## 5. External Behavior

The user sees Japanese strings throughout: navigation (`ホーム`, `コミュニティ`, `マイページ`), status buttons (`参加`, `不参加`, `出席`), event creation labels, invite pages, confirmation dialogs, error messages, and help text. Day labels remain in Japanese convention (`6月14日（土）`).

## 6. Internal Design

Mechanical replacement of all `i18n::EN_*` references with `i18n::JA_*` in:
- `workers/ssr/src/render.rs`
- `workers/ssr/src/handlers/home.rs`
- `workers/ssr/src/handlers/event.rs`
- `workers/ssr/src/handlers/admin/events.rs`
- `workers/ssr/src/handlers/admin/members.rs`
- `workers/ssr/src/handlers/join.rs`
- `workers/ssr/src/handlers/me.rs`
- `workers/ssr/src/handlers/communities.rs`
- `workers/ssr/src/handlers/community.rs`
- `workers/ssr/src/handlers/templates.rs`
- `workers/ssr/src/handlers/calendar.rs`
- `workers/ssr/src/handlers/export.rs`

HTML `lang` attribute in `render::shell` changed from `"en"` to `"ja"`.

The Rust compiler validates every `JA_*` reference at compile time. The parity test ensures no JA string is empty or identical to its EN counterpart (with one exception: `ciao.zinnias` product name). Zero additional test failures.

## 7. Reversibility

To switch back to English, change `JA_` to `EN_` throughout and `lang="ja"` to `lang="en"`. The i18n architecture supports this without schema or data changes.

When runtime locale switching is implemented, this compile-time choice is replaced by a dynamic dispatch function that selects EN or JA based on user/community preference.

## 8. Acceptance Criteria

- All primary user flows (join, Home, Event Detail, Going/No Go, save note) render in Japanese on a real device.
- Admin flows (create event, generate invite, remove member, attendance correction) render in Japanese.
- No EN string appears in the user-facing HTML (except the product name `ciao.zinnias`).
- HTML `lang="ja"` reported by browser.
- `cargo check --target wasm32-unknown-unknown` — zero errors, zero warnings.

## 9. Test Plan

The parity test in `release_gates.rs` ensures all JA strings are non-empty. Compile-time type checking ensures all constants exist. Visual verification on a real device is required (RFC-045 §S9 usability check).

## 10. Open Decisions

- **Runtime locale switching.** Deferred. When a community or user can choose EN/JA at runtime, this compile-time selection is replaced by a `lang` parameter on the render functions.
- **Admin documentation.** Operations docs, wrangler.toml comments, and code comments remain in English per project convention.
