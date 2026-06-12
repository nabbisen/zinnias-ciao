# RFC 047 — Japanese Date/Time Presentation

**Status.** Implemented (v0.27.0)
**Phase:** F7 / Stabilization (handoff-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes handoff-review finding P1-3 (English date fragments in a Japan-first deployment). Refines RFC-018 (timezone and time display) and the i18n parity discipline. Independent of RFC-046.

---

## 1. Summary

Event Detail day labels previously rendered the calendar date with an English
month abbreviation, e.g. `14 Jun 09:00–10:30`. For a Japan-first deployment
targeting non-technical users, the natural form is `6月14日（土）09:00–10:30`.

This RFC adds pure, tested date-label formatters to `contracts::tz` — a Japanese
formatter (`date_label_ja`), an English formatter (`date_label_en`), and the
supporting weekday computation (`weekday_index` via Zeller's congruence,
`weekday_ja`) — and wires `format_day_label` to render the Japanese form.

## 2. Motivation

The product is positioned as calm and friendly for IT-averse users, with a
Japan-first launch. An English month abbreviation in an otherwise-Japanese UI is
not a correctness bug (the time is right) but it is a trust-and-polish defect: it
signals that the app was not built for its users. The architect handoff review
(P1-3) recommended fixing this before a real Japanese pilot.

## 3. Goals

- Render calendar dates in Japanese convention: `6月14日（土）`.
- Include the weekday in Japanese single-character form (日月火水木金土).
- Keep the formatters pure and unit-tested (no I/O, no external date crate).
- Provide a parallel English formatter for when locale switching is added.

## 4. Non-Goals

- A full runtime locale-switching system. The app currently renders EN-only at
  the string level (every call site uses `i18n::EN_*`); building a `Lang`-aware
  rendering pipeline across every template is a larger change tracked separately.
  This RFC ships the Japan-first **date format** because that is the immediate,
  high-value, low-risk improvement the review asked for.
- Era-based years (令和). Gregorian numeric years are used; the day label omits
  the year entirely (the event's year is clear from context on the page).
- Changing stored data. Only presentation changes.

## 5. External Behavior

Event Detail day headers now read, for a single-day event on Saturday 14 June:

```
6月14日（土）09:00–10:30
```

For a multi-day event, the existing `Day N — ` prefix is retained ahead of the
Japanese date. (Localizing the `Day N` prefix itself is deferred to the broader
locale-switching work.)

## 6. Internal Design

### Weekday computation (Zeller's congruence)

`weekday_index(year, month, day) -> i32` returns `0=Sunday .. 6=Saturday` using
Zeller's congruence over the proleptic Gregorian calendar. Verified against known
dates: 2000-01-01 = Saturday, 2026-06-14 = Sunday, 2026-01-01 = Thursday.

### Formatters

- `weekday_ja(index) -> &'static str` maps 0..6 to 日..土.
- `month_abbr_en(month) -> &'static str` maps 1..12 to Jan..Dec.
- `date_label_ja("YYYY-MM-DD") -> String` → `"6月14日（土）"`.
- `date_label_en("YYYY-MM-DD") -> String` → `"14 Jun"`.

All formatters fall back to the raw input string on unparseable dates rather than
panicking — matching the degrade-gracefully convention used by `to_local_parts`
and `local_to_utc`.

### Wiring

`handlers/event.rs::format_day_label` computes the local `YYYY-MM-DD` via the
existing `utc_to_local_parts_pub`, then calls `tz::date_label_ja` for the
displayed date.

## 7. Data Model Notes

None. Presentation-only.

## 8. API and UI Contract Notes

The `format_day_label` signature is unchanged. Internally it now produces a
Japanese date label. When locale switching is added, the function will take a
`Lang` parameter and dispatch to `date_label_ja` or `date_label_en`; the
formatters are already split to make that a one-line change.

## 9. Security, Privacy, and Safety

No security surface. The Japanese weekday parenthesis uses full-width `（）` to
match Japanese typographic convention; the output is inserted through the normal
`escape_html` path like any other label.

## 10. Acceptance Criteria

- A Saturday date renders `…（土）`; a Sunday date renders `…（日）`.
- No English month abbreviation appears in the day label.
- Malformed date input falls back to the raw string without panicking.
- Zero warnings; all existing tests still pass.

## 11. Test Plan

Unit tests in `contracts/tz.rs` (`date_label_tests`):

- `weekday_known_dates` — four independently-verified reference dates.
- `ja_label_has_month_day_weekday` — exact string match for two dates.
- `ja_label_no_english_month` — asserts absence of `Jun`, presence of `月`.
- `en_label_format` — English form for two dates.
- `malformed_date_falls_back` — fallback behavior.

## 12. Rollout Plan

Shipped in v0.27.0. Presentation-only; no migration, no operator action.

## 13. Open Decisions

- **Weekday in English mode.** When locale switching lands, decide whether the
  English label should also include a weekday (`Sat 14 Jun`) for symmetry. Left
  open; the Japanese pilot does not need it.
- **`Day N` prefix localization.** Deferred to the locale-switching work; the
  prefix is currently English in multi-day labels.
