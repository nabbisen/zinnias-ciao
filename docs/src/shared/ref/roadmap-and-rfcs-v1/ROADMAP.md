# ciao.zinnias Implementation Roadmap

**Version:** 2.1 (reconciled, pass 2 complete)
**Date:** 2026-06-11
**Baseline:** `requirements-v1`, `external-design-v1` (with the amendments in `ARCHITECTURE-DECISIONS.md` §5)
**Architecture:** see `ARCHITECTURE-DECISIONS.md` — this roadmap assumes AD-1…AD-4.

---

## 0. What changed in 2.0

This pack reconciles the original roadmap + RFCs (packs v1.0 and v1.1-continuation) to four decisions that were settled after review: SSR + progressive enhancement (no hydration); invite-code + cookie session now with OIDC deferred; design to the 10 ms Workers Free CPU budget; one server-issued form token for CSRF + idempotency. All MVP-core RFCs (001–020) are reconciled accordingly. It also brings the pack into line with the project's `000-rfc-lifecycle-policy.md` (state folders, `NNN-slug.md`, `**Status.**` header). The duplicate pack-level RFC-000 ("process and project decisions") was removed; its decision-authority content is folded into §1 below.

## 1. Decision authority (folded from the removed pack RFC-000)

1. **Requirements** define product obligations.
2. **External design** defines visible behavior and the boundary.
3. **`ARCHITECTURE-DECISIONS.md`** records cross-cutting technical decisions; RFCs must not silently contradict it.
4. **RFCs** define implementation design per milestone.
5. **Code** implements accepted RFCs; **tests/release gates** verify behavior.

When an RFC finds a contradiction with requirements/external-design, it is recorded and the higher document is amended explicitly (the AD §5 amendments are the current open set). Post-MVP ideas are parked in the backlog (021–036), never folded into MVP silently.

## 2. Product direction to preserve

Private notice-board metaphor; mobile-first; invite-only entry; first-class community isolation; simple status model (No answer / Going / No Go / Attended); one ≤200-char plain-text note per member per event; read-only offline resilience with honest state; narrow, safe admin tools (cancel/soft-delete over hard delete); accessibility by default (never color alone).

## 3. Milestones

| Phase | Name | Main RFCs | Exit condition |
|---|---|---|---|
| M0 | Project foundation | 001 | SSR Workers/Leptos/D1 skeleton deploys; health/version respond. |
| M1 | Trust boundary | 002, 003, 004 | Cross-community access structurally denied and tested; invite→membership→session atomic. |
| M2 | Member flow | 005, 006, 007, 013 | Member joins, views event, sets status, saves a note — all via SSR forms. |
| M3 | Read-only offline + PWA | 008, 017 | Recently viewed pages open offline; honest "offline" fallback. **No offline writes.** |
| M4 | Admin flow | 009, 010 | Non-technical admin runs a small community; actions audited. |
| M5 | UX / a11y / security | 011, 012, 014, 015 | Release gates pass: isolation, CSRF (form token), XSS, a11y, audit/redaction. |
| M6 | Deployment readiness | 016, 018, 019 | Distinct envs; deterministic time/cutoff; documented retention/recovery. |
| M7 | Design handoff | 020 | Design deliverables complete; scope held. |

Two milestone corrections from v1.0: M2 status/note flows are **form POST → 303 → re-render** (not optimistic); M3 is **read-only** (the offline mutation queue is removed — AD-1).

## 4. Reconciliation status (this pack)

Pass 1 (governance migration + trust-boundary foundation) and pass 2 (member flow, security, contracts, PWA, admin, light edits) are complete. The full MVP core (001–020) is reconciled to AD-1…AD-4 and the Event→EventDay→Attendance grain.

| RFC set | Status |
|---|---|
| 001, 002, 003 | Reconciled (foundation; 002 grain finalized). |
| 005, 006, 007 | Reconciled — SSR forms + form token, per-day status, per-event note. |
| 008, 017 | Reconciled — read-only offline + SW; offline-write queue removed. |
| 012, 013 | Reconciled — CSRF=form token, strict CSP; contracts re-scoped to HTML+forms + small JSON surface. |
| 004, 009, 010 | Reconciled — identity from session, per-day admin events, peppered HMAC + tokens. |
| 014, 015, 016 | Reconciled — `request_id` across Service Bindings; offline-queue gate dropped; Logpush + multi-config wrangler. |
| 011, 018, 019, 020 | Reconciled — 018 per-day cutoff, 019 per-day retention; 011/020 unchanged. |
| 021–036 | Stub (backlog) — banner in file; detail on acceptance. 023 (ICS) partly shipped; 024 revisit once OIDC lands. |

Still owner-side (needs codebase knowledge, not done here): the `done/` **state audit** — move actually-shipped RFCs out of `proposed/` into `done/` with the version tag they shipped in.

## 5. Implementation ordering rules

Build membership/authorization before any event UI; never implement a UI path that bypasses authorization; admin flows require audit records from day one; PWA caching requires logout/expiry clearing; design tokens must not drift from implementation constants; status is never color-only; no future feature (chat, photos, recurring, analytics, payments, export, OIDC) enters MVP without reopening scope via its RFC.

## 6. MVP release-gate summary

Invite onboarding; finite secure logout-safe sessions; community isolation tested at query/render/error layers; member home/detail/status/note flows; admin event/invite/member/attendance flows; read-only offline with honest state; a11y checks; XSS/CSRF (form token)/SQL-injection/cross-community tests; logs/audit free of secrets and note bodies; documented deploy + recovery.
