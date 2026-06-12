# RFC 053 — ICS Feed Privacy and Revocation UX

**Status.** Proposed
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Extends RFC-016 (calendar feed). Closes architect review v0.29.0/v0.30.0 P2 (ICS bearer URL privacy).

## 1. Summary

ICS feed bearer URLs are effectively secrets. Calendar apps may store, sync, or expose them. This RFC defines user-facing copy, feed content scope, token rotation UX, and log/referrer controls.

## 2. Proposed content

- **User-facing copy:** "このカレンダーリンクは、持っている人なら誰でもあなたのコミュニティの予定を見られます。公開しないでください。こちらで再発行または無効化できます。"
- **Feed content:** event titles and times only. No participant status, notes, invite codes, or admin data in the ICS output.
- **Logs:** ICS bearer token never appears in audit metadata, Logpush logs, or Referrer headers (already enforced in code).
- **Token rotation:** regenerate action revokes old and issues new in one step (already implemented).

## 3. Source verification (completed)

The ICS feed handler (`get_ics_feed` in `calendar.rs`) and `build_vcalendar` in
`contracts/ics.rs` were verified against source: the feed emits SUMMARY (event
title), DTSTART/DTEND, LOCATION, and STATUS (confirmed/cancelled) only. No
participant status, notes, invite codes, or member names are included. The ICS
feed scope concern from the architect review is satisfied in code.

The remaining work is UX copy and token-rotation wording (see §2), which needs
Japanese copy review (RFC-054).

## 4. Blocker

Japanese copy review (RFC-054) should review the proposed wording.
