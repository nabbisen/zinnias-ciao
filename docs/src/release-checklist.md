# MVP Release Gate Checklist (RFC-015)

Fill this for every release candidate before promoting to production.

Legend: `[x]` = verified by code inspection or automated test · `[~]` = requires human QA pass · `[ ]` = not yet done

---

## Functional gates

- [x] Invite-code onboarding works end-to-end. *(join.rs: HMAC lookup → display name → atomic user+membership+session)*
- [x] Display name is collected and visible in the community. *(join.rs post_profile, membership.display_name)*
- [x] Session is issued, validated, and revoked on logout. *(session.rs, auth.rs post_logout)*
- [x] Community membership enforced (non-members see generic 404). *(authz.rs require_membership: checks user_id AND community_id)*
- [x] Home shows upcoming events grouped Today / This Week / Later. *(home.rs: day_date comparisons against today_date)*
- [x] Event Detail shows status, participants, and notes. *(event.rs get_event_detail)*
- [x] Member can set own status (Going / No Go / clear). *(event.rs post_my_status + validate_status_transition)*
- [x] Member can save, edit, and delete own note. *(event.rs post_my_note, delete_my_note)*
- [x] Admin can create event (single and multi-day). *(admin.rs post_create_event + event_write::create_event)*
- [x] Admin can cancel event (confirmation required). *(admin.rs get_cancel_event shows confirmation; post_cancel_event soft-cancels)*
- [x] Admin can generate and revoke invite codes. *(admin.rs post_generate_invite + post_revoke_invite; invite_db::revoke)*
- [x] Admin can remove member (last-admin guard active). *(admin.rs post_remove_member: count_admins guard)*
- [x] Admin can mark Attended after event day ends. *(admin.rs get_attendance / post_attendance; classify_day gate)*

## Safety gates

- [x] Cross-community data access blocked (manual test: direct URL to another community's event). *(authz.rs require_membership: community_id check on every request)*
- [x] Removed members lose access on next request. *(membership.rs find_active: AND removed_at IS NULL)*
- [x] Invite codes are single-use. *(invite.rs find_valid: AND used_at IS NULL; mark_used sets it atomically)*
- [x] Failed invite redemptions are rate-limited. *(rate_limit.rs: 10 failures / 5-min window per IP, KV-backed)*
- [x] Session cookies have `HttpOnly; Secure; SameSite=Strict`. *(session.rs build_session_cookie: verified in string literal)*
- [x] Form token absent/replayed → POST rejected. *(form_token.rs consume: single-use; all POST handlers call consume)*
- [x] Script tag in note/title/name renders as text (not executed). *(render.rs escape_html used at every user-content insertion; test: escape_script_tag)*
- [x] Private page cache cleared on logout. *(sw.js PURGE_PRIVATE message; app.js sends it before logout POST)*

## Offline gates

- [x] Previously visited Home/Event Detail opens offline with banner. *(sw.js network-first with page cache fallback; app.js toggles `offline-banner` on `online`/`offline` events — code-verified)*
- [x] Unvisited page shows the offline fallback. *(sw.js `OFFLINE_URL = '/offline'`; shell assets pre-cached on install — code-verified)*
- [x] Form submit while offline does not falsely succeed. *(sw.js: `if (req.method !== 'GET') return;` — non-GET requests bypass SW and reach network, so browser shows its own network error — code-verified via AD-1)*

## UX / accessibility gates

- [~] Core join-and-mark-attendance flow completes under 2 minutes on a phone. *(requires phone test)*
- [x] All critical controls ≥ 44 × 44 px. *(app.css L88: `button, a, [role="button"] { min-height: var(--cz-touch-min); }` where `--cz-touch-min: 44px` (L57); all inline buttons also set `min-height:44px` — code-verified)*
- [x] Status chip shows icon + label + colour (grayscale test: still legible). *(render.rs `status_display()` always returns `(fg_color, icon, label)` tuple; AA-passing fg colors on white: Going 5.0:1, Not Going 5.9:1, Attended 4.7:1 — code-verified)*
- [~] Event Detail usable at 200% text scaling. *(requires browser test)*
- [x] Reduced-motion mode disables animations. *(app.css: `@media (prefers-reduced-motion: reduce) { *, *::before, *::after { transition: none !important; animation: none !important; } }` — code-verified)*
- [x] Error messages use plain language (no SQL/JWT/token/cookie). *(release_gates.rs `not_found_and_forbidden_same_message`; domain tests verify no 'sql'/'panic' in event/note error strings — automated)*

## Operational gates

- [x] `GET /healthz` returns `{"ok":true}`. *(health.rs get_health)*
- [x] `GET /version` returns build version. *(health.rs get_version reads BUILD_VERSION var)*
- [x] Rollback procedure documented and understood. *(docs/src/deployment.md §Rollback: `wrangler rollback --env production`)*
- [x] Log persistence approach documented. *(docs/src/deployment.md §Log persistence: Cloudflare Logpush to R2/S3)*
- [ ] D1 migration applied to staging and rehearsed. *(operator task: `bun run migrate:prod` against staging env)*
- [ ] Secrets (`HMAC_PEPPER`, `SESSION_COOKIE_DOMAIN`) set in production. *(operator task: `wrangler secret put`)*
- [ ] Logpush configured for production. *(operator task: Cloudflare dashboard)*
- [ ] No critical open security issues. *(final security review before go-live)*
