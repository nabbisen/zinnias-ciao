# RFC 007 — Notes and Comment Safety

**Status.** Proposed
**Phase:** M2 / Member MVP Flow
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1/AD-4 (explicit Save = form POST + token; no client draft autosave), per-event note grain.

---

## 1. Summary

One short plain-text note per member per event (≤200 chars). Editing and saving are an explicit form submission — which, under SSR, is exactly the "no accidental publish" behavior the product wants, with no client draft machinery.

## 2. Goals

- One note per member per event; ≤200 chars; plain text.
- Member edits/deletes own note; admin can delete (moderate) with audit.
- Publish only on explicit Save (a POST), never on blur/navigation.

## 3. Non-Goals

- No chat, threads, multiple notes, Markdown/HTML rendering, media, mentions, or client-side autosave/draft queue (AD-1).

## 4. External Behavior

Event Detail shows a note textarea inside a `<form method="post">` with a character counter and a Save Note button (and Delete if a note exists). Saving POSTs → 303 → the note renders as escaped text in the notes list. Over-length disables Save (server also rejects). With JS off, the counter is static but Save still works; optional progressive enhancement makes the counter live and disables Save past 200.

States shown after submit: "Saved." / "Could not save. Please try again." There is no "saved on this phone / will sync" state — writes are online-only (AD-1).

## 5. Internal Design

Note lives in `event_notes (event_id, membership_id, note, …)` (RFC-002). The save form carries a `save_note` token bound to `event_id`. Handler: validate token → re-authorize → server-side normalize (trim control chars, Unicode-aware length ≤200, store plain text) → upsert → 303. Delete is a separate `delete_note` token form. Admin moderation delete sets `note_deleted_at`/`hidden_by_admin_at` and writes an audit record without copying the body.

## 6. Data and API Design

```text
POST /c/:cid/events/:eid/my-note          # token, note
POST /c/:cid/events/:eid/my-note/delete   # token
POST /c/:cid/admin/events/:eid/notes/:membershipId/delete  # admin; token
```

## 7. Security, Privacy, and Safety

- Render notes as escaped text only; never as HTML/Markdown (RFC-012).
- Do not log note bodies (RFC-014); audit keeps metadata, not content.
- Notes from removed members may remain visible per retention policy but grant no access (RFC-019).

## 8. Acceptance Criteria

- Save persists a ≤200-char note; >200 rejected server-side.
- Edit/delete own note works; closing/navigating away never publishes.
- Admin delete works with confirmation + audit.
- Script-like note content renders harmlessly as text.

## 9. Test Plan

- Length/normalization unit tests (Unicode-aware).
- XSS payload render test.
- Own-note vs other-note authorization tests.
- Token single-use test.
- JS-disabled save path test.

## 10. Open Questions / Decisions

Decision: explicit Save only; no autosave. Note grain is per-event for MVP, with a documented seam to per-day (RFC-002 §5) if a community needs it.
