# RFC 053 — ICS Feed Privacy and Revocation UX

**Status.** Implemented (v0.46.0)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Shipped in:** v0.46.0
**Relationship:** Extends RFC-016 (calendar feed). Closes architect review v0.29.0/v0.30.0 P2 (ICS bearer URL privacy).

## 1. Summary

ICS feed bearer URLs are effectively secrets. Calendar apps may store, sync, or expose them. This RFC defines user-facing copy, feed content scope, token rotation UX, and log/referrer controls.

## 2. Implemented content

- **User-facing copy:** "このカレンダーリンクは、持っている人なら誰でもあなたのコミュニティの予定を見られます。公開しないでください。こちらで再発行または無効化できます。"
- **Feed content:** event titles and times only. No participant status, notes, invite codes, or admin data in the ICS output.
- **Logs:** ICS bearer token never appears in audit metadata. The feed response is `no-store, private` and now also sends `Referrer-Policy: no-referrer`.
- **Token rotation:** regenerate action revokes old and issues new in one step.
- **UX status messages:** post-action redirects use fixed flash codes (`generated`, `disabled`) and render reviewed Japanese copy. Raw query text is not reflected into the page.

## 3. Source verification

The ICS feed handler (`get_ics_feed` in `calendar.rs`) and `build_vcalendar` in
`contracts/ics.rs` were verified against source: the feed emits SUMMARY (event
title), DTSTART/DTEND, LOCATION, and STATUS (confirmed/cancelled) only. No
participant status, notes, invite codes, or member names are included. The ICS
feed scope concern from the architect review is satisfied in code.

v0.46.0 adds a source release gate for the same contract:

- calendar feed UI uses the reviewed privacy note and fixed Japanese flash copy;
- generate/revoke audit writes use `None` metadata;
- ICS responses use private no-store cache control plus no-referrer and nosniff headers;
- `events_for_feed` remains community-scoped and does not select attendance, notes, invite codes, member names, or descriptions;
- the ICS builder does not emit attendee, description, comment, or organizer fields.

## 4. Review status

The prior RFC-054 Japanese copy hardening supplied the member-facing wording
used here. The v0.46.0 release gate now keeps the calendar privacy and
revocation copy inside the RFC-054 jargon check.
