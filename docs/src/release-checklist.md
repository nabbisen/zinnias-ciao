# MVP Release Gate Checklist (RFC-015)

Fill this for every release candidate before promoting to production.

## Functional gates

- [ ] Invite-code onboarding works end-to-end.
- [ ] Display name is collected and visible in the community.
- [ ] Session is issued, validated, and revoked on logout.
- [ ] Community membership enforced (non-members see generic 404).
- [ ] Home shows upcoming events grouped Today / This Week / Later.
- [ ] Event Detail shows status, participants, and notes.
- [ ] Member can set own status (Going / No Go / clear).
- [ ] Member can save, edit, and delete own note.
- [ ] Admin can create event (single and multi-day).
- [ ] Admin can cancel event (confirmation required).
- [ ] Admin can generate and revoke invite codes.
- [ ] Admin can remove member (last-admin guard active).
- [ ] Admin can mark Attended after event day ends.

## Safety gates

- [ ] Cross-community data access blocked (manual test: direct URL to another community's event).
- [ ] Removed members lose access on next request.
- [ ] Invite codes are single-use.
- [ ] Failed invite redemptions are rate-limited.
- [ ] Session cookies have `HttpOnly; Secure; SameSite=Strict`.
- [ ] Form token absent/replayed → POST rejected.
- [ ] Script tag in note/title/name renders as text (not executed).
- [ ] Private page cache cleared on logout.

## Offline gates

- [ ] Previously visited Home/Event Detail opens offline with banner.
- [ ] Unvisited page shows the offline fallback.
- [ ] Form submit while offline does not falsely succeed.

## UX / accessibility gates

- [ ] Core join-and-mark-attendance flow completes under 2 minutes on a phone.
- [ ] All critical controls ≥ 44 × 44 px.
- [ ] Status chip shows icon + label + colour (grayscale test: still legible).
- [ ] Event Detail usable at 200% text scaling.
- [ ] Reduced-motion mode disables animations.
- [ ] Error messages use plain language (no SQL/JWT/token/cookie).

## Operational gates

- [ ] `GET /healthz` returns `{"ok":true}`.
- [ ] `GET /version` returns build version.
- [ ] D1 migration applied to staging and rehearsed.
- [ ] Secrets (`HMAC_PEPPER`, `SESSION_COOKIE_DOMAIN`) set in production.
- [ ] Logpush configured for production (no filesystem logging on isolates).
- [ ] Rollback procedure documented and understood.
- [ ] No critical open security issues.
