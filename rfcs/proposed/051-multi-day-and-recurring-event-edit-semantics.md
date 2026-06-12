# RFC 051 — Multi-Day and Recurring Event Edit Semantics

**Status.** Proposed
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Clarifies RFC-002 (EventDay grain), RFC-022 (recurrence), RFC-040 (event edit contract). Required before pilot: admin must not accidentally edit one day believing it affects all, or vice versa.

## 1. Summary

The current handoff describes edit behavior as "persists time for single-day events." Multi-day and recurring event edit semantics are unstated. This RFC defines the MVP rule explicitly and ensures the UI communicates it.

## 2. Proposed MVP rule

- **Single-day event:** title, location, description, date, and time are editable before the event starts.
- **Multi-day / recurring event:** title, location, and description are editable. Date/time changes to individual days are not supported via the edit form; cancel and recreate to change the schedule.
- **After attendance exists:** all fields remain editable; existing attendance rows are preserved and not shifted.
- **Cancellation:** always cancels the whole event (all days), not a single occurrence.

## 3. UI changes required

- Edit form for multi-day events hides date/time fields or shows "not editable for multi-day events."
- Confirmation copy explains the whole-event scope.

## 4. Blocker

Product decision from nabbisen required to confirm the MVP rule before implementation.
