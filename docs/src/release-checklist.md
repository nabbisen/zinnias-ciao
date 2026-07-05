# MVP Release Gate Checklist (RFC-015)

Fill this for every release candidate before promoting to production.

Legend: `[x]` = verified by code inspection or automated test · `[~]` = requires human QA pass · `[ ]` = not yet done

---

## Functional gates

- [x] Invite-code onboarding works end-to-end. *(join.rs: HMAC lookup → display name → atomic user+membership+session)*
- [x] Display name is collected and visible in the community. *(join.rs post_profile, membership.display_name)*
- [x] Session is issued, validated, and revoked on logout. *(session.rs, auth.rs post_logout)*
- [x] Community membership enforced (non-members see generic 404). *(authz.rs require_membership: checks user_id AND community_id)*
- [x] Home shows active communities one by one with nearby event links, without a header community switcher. *(home.rs: `home_upcoming_for_communities`, `render_home_communities`)*
- [x] Calendar tab shows the active community's month overview and event links, supports route-backed month navigation and day filtering, and keeps the community switcher. *(communities.rs: `calendar_month_for_community`, `render_calendar_month`, `render_calendar_events`, `header_with_switcher_next` — RFC-058)*
- [x] Community switcher auto-submit is implemented in external `app.js`, not inline `onchange`, and the shell cache-busts `app.js` with a visible submit fallback for stale/no JS. *(render.rs + app.js + release gate)*
- [x] Event Detail shows status, participants, and notes. *(event.rs get_event_detail)*
- [x] Member can set own status (Going / No Go / clear). *(event.rs post_my_status + validate_status_transition)*
- [x] Member can save, edit, and delete own note. *(event.rs post_my_note, delete_my_note)*
- [x] Admin can create event (single and multi-day). *(admin.rs post_create_event + event_write::create_event)*
- [x] Admin Create Event switcher stays on Create Event for the selected community, so new events bind to the selected community URL. *(admin/events.rs + `/switch?next=admin_events_new`)*
- [x] Admin can cancel event (confirmation required). *(admin.rs get_cancel_event shows confirmation; post_cancel_event soft-cancels)*
- [x] Admin can generate and revoke invite codes. *(admin/members.rs: rejection-sampling generator writes HMACs to `invite_codes`; `codlet::revoke_invite` delegates to `invite_db::revoke`)*
- [x] Admin can remove member (last-admin guard active). *(admin.rs post_remove_member: count_admins guard)*
- [x] Admin can mark Attended after event day ends. *(admin.rs get_attendance / post_attendance; classify_day gate)*
- [x] Existing active admins can create an additional community when `COMMUNITY_CREATION_ENABLED=true`; creator becomes first admin and redirects to the new Home. *(community_create.rs, community.rs DB helper — RFC-057)*

## Safety gates

- [x] Cross-community data access blocked (manual test: direct URL to another community's event). *(authz.rs require_membership: community_id check on every request)*
- [x] Removed members lose access on next request. *(membership.rs find_active: AND removed_at IS NULL)*
- [x] Invite codes are single-use. `invite.rs::mark_used` performs a conditional `UPDATE invite_codes SET used_at = ? WHERE used_at IS NULL ...`; the caller aborts if it loses. `used_by_membership_id` is filled only after the winning membership row exists, satisfying the FK. *(join.rs post_profile, invite.rs — RFC-041)*
- [x] Failed invite redemptions are rate-limited. *(rate_limit.rs: 10 failures / 5-minute window per IP, KV-backed via `RATE_LIMIT`, fails open on KV unavailability)*
- [x] Session cookies have `HttpOnly; Secure; SameSite=Strict`. Host-only by default (no `Domain` attribute unless `SESSION_COOKIE_DOMAIN` var is set). *(session.rs build_session_cookie — RFC-038)*
- [x] Form token absent/replayed → POST rejected. `codlet::consume_token` is the handler-facing compatibility wrapper; it delegates to `form_token.rs::consume`, which performs a conditional UPDATE on `form_tokens`. Subject is the authenticated `user_id`; replay returns `Ok(Some(...))`, invalid returns `Err`. *(codlet.rs, form_token.rs — RFC-037)*
- [x] Community creation is authenticated, active-admin-only, feature-flagged, token-protected, idempotent, rate-limited by user/session/IP, audited, and does not auto-generate invite codes. *(release_gates.rs RFC-057 gates)*
- [x] Script tag in note/title/name renders as text (not executed). *(render.rs escape_html used at every user-content insertion; test: escape_script_tag)*
- [x] Private page cache cleared on logout. *(RFC-042: authenticated HTML is never cached; only static shell assets are stored. No private cache exists to clear — the property holds trivially. PURGE_PRIVATE is retained for defence-in-depth.)*

## Auth storage gates (v0.38.6)

- [ ] `HMAC_PEPPER` secret is set in the target environment (`wrangler secret put HMAC_PEPPER`).
- [ ] `RATE_LIMIT` KV namespace is created and bound in `wrangler.toml` for the target environment.
- [ ] New invite code generation writes to `invite_codes` only (verify: `SELECT COUNT(*) FROM invite_codes` increases after admin generates a code).
- [ ] New session issuance writes to `sessions` (verify: `SELECT COUNT(*) FROM sessions` increases after a successful join).
- [ ] Form tokens write to `form_tokens` (verify: `SELECT COUNT(*) FROM form_tokens` increases after rendering/submitting forms).
- [ ] Invite revocation sets `invite_codes.revoked_at`.
- [ ] Session cookie name remains `ciao_sid`.


## Offline gates

- [x] Previously visited Home/Event Detail opens offline with banner. *(RFC-042: authenticated routes are network-only; offline navigation returns the pre-cached static `/offline` page. No stale private page is served.)*
- [x] Unvisited page shows the offline fallback. *(sw.js `OFFLINE_URL = '/offline'`; shell assets pre-cached on install — code-verified)*
- [x] Form submit while offline does not falsely succeed. *(sw.js: `if (req.method !== 'GET') return;` — non-GET requests bypass SW and reach network, so browser shows its own network error — code-verified via AD-1)*
- [x] Status, note, and attendance submit buttons are disabled when `navigator.onLine` is false; Japanese tooltip explains the read-only contract. *(app.js `setOfflineSubmitState` — RFC-055)*

## UX / accessibility gates

- [~] Core join-and-mark-attendance flow completes under 2 minutes on a phone. *(requires phone test)*
- [x] All critical controls ≥ 44 × 44 px. *(app.css L88: `button, a, [role="button"] { min-height: var(--cz-touch-min); }` where `--cz-touch-min: 44px` (L57); all inline buttons also set `min-height:44px` — code-verified)*
- [x] Status chip shows icon + label + colour (grayscale test: still legible). *(render.rs `status_display()` always returns `(fg_color, icon, label)` tuple; AA-passing fg colors on white: Going 5.0:1, Not Going 5.9:1, Attended 4.7:1 — code-verified)*
- [~] Event Detail usable at 200% text scaling. *(requires browser test)*
- [x] Home multi-community nearby-events dashboard and Calendar overview usable at 360-428 px and 200% text scaling. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc056/rfc056-route-split-smoke-results.json`)*
- [x] Calendar month navigation, selected-day agenda, and switcher month/day preservation usable at 360-428 px and 200% text scaling. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc058/rfc058-calendar-smoke-results.json`)*
- [~] Community creation form usable at 360-428 px, 200% text scaling, and with JavaScript disabled. *(requires browser smoke for RFC-057)*
- [x] Reduced-motion mode disables animations. *(app.css: `@media (prefers-reduced-motion: reduce) { *, *::before, *::after { transition: none !important; animation: none !important; } }` — code-verified)*
- [x] Error messages use plain language (no SQL/JWT/token/cookie). *(release_gates.rs `not_found_and_forbidden_same_message`; domain tests verify no 'sql'/'panic' in event/note error strings — automated)*

## Stabilization gates (v0.23.0 — RFC-037–042)

- [x] Member can set Going/No Go/Attended. Form token issued via `codlet::issue_token(SET_STATUS, event_id)`, consumed via `codlet::consume_token` bound to event_id; both delegate to service-owned `form_tokens`. *(event.rs — RFC-037)*
- [x] Member can delete their own note (same token-subject fix — RFC-037).
- [x] Form-token consume is a conditional UPDATE; concurrent double-submit executes at most once (RFC-037).
- [x] Session cookie is host-only when `SESSION_COOKIE_DOMAIN` is unset; no `Domain=localhost` fallback (RFC-038).
- [x] All handlers use `crypto::pepper` — no divergent `env.var`/`env.secret` mix (RFC-038).
- [x] Event create converts community-local time to UTC at write time. Tokyo admin entering 09:00 stores 00:00Z (RFC-039).
- [x] Event edit persists date/time for single-day events; form prefills current values (RFC-040).
- [x] Invite redemption claims the invite atomically first; a lost race aborts without creating a second member (RFC-041).
- [x] Authenticated HTML (`/c/*`, `/`, `/join`) is never stored in the service-worker cache (RFC-042).
- [x] `sw.js CACHE_VERSION` matches the package version. *(verify: `grep CACHE_VERSION workers/ssr/static/sw.js` matches `version` in `Cargo.toml`)*
- [~] Admin creating 09:00 in a non-UTC community displays 09:00 after round-trip. *(staging smoke test)*
- [~] No-JS destructive confirmations (cancel event, remove member, delete note) work without scripting. *(implementation in v0.24.0 — verify on a JS-disabled browser)*

## Stabilization gates (v0.27.0 — RFC-045–047 + i18n/XSS hardening)

- [x] All 11 source-verification claims from architect handoff review §8 confirmed against code (RFC-045 §5).
- [x] `SET_STATUS` token issued once per Event Detail render, bound to `event_id`; day validated via days_for_event lookup (RFC-046).
- [x] Day labels render in Japanese convention (`6月14日（土）`); no English month abbreviation (RFC-047).
- [x] Logout, calendar-token generate, and calendar-token revoke are audited (review P1-5).
- [x] DST scope limitation documented in `docs/src/operations.md` (review P1-2).
- [x] No-JS community switcher has a visible `<noscript>` submit fallback; confirmed in `render.rs` (review P1-4).
- [x] i18n parity test covers all 171 EN/JA string pairs; catches empty strings and copy-paste errors. *(release_gates.rs `i18n_en_ja_parity_count`)*
- [x] `escape_html` moved to tested `contracts::html` module; 10 unit tests including XSS vector and attribute injection; `render::escape_html` delegates to the tested implementation.
- [~] Staging runtime verification (RFC-045 §6): timezone round-trip, concurrent invite/token races. *(requires Cloudflare staging deployment)*

## Pre-pilot hardening gates (v0.30.0 — RFC-048, RFC-049 + timezone hardening)

- [x] `Cache-Control: no-store` on all authenticated HTML responses (RFC-048); static assets retain public/max-age.
- [x] CSP extended: `base-uri 'none'`, `form-action 'self'`, `object-src 'none'` added; `unsafe-inline` exception documented (RFC-048). *(tightened to `base-uri 'none'` in v0.30.x — app uses no `<base>` tag)*
- [x] `Permissions-Policy` header added (RFC-048).
- [x] `Referrer-Policy` changed to `same-origin` (RFC-048).
- [x] All UI strings render in Japanese (`JA_*`); HTML `lang="ja"` (RFC-049).
- [x] Unknown community timezone returns a hard error on write paths, not a silent UTC fallback (P1-timezone).
- [x] Query budget for max-recurring Event Detail updated from ≤65 to ≤13 (correct after RFC-046).
- [~] Security header values verified in a real browser on staging. *(staging runtime)*

## Release-gate hardening (v0.34.0 — RFC-044 partial)

- [x] i18n parity gate covers all 171 EN/JA pairs. *(release_gates.rs `i18n_en_ja_parity_count`)*
- [x] Static query-count gates: home.rs, event.rs, export.rs `.await` counts verified within ceiling bounds. *(release_gates.rs `*_await_count_within_budget` — v0.34.0)*
- [x] SW `CACHE_VERSION` matches workspace version. *(release_gates.rs `sw_cache_version_matches_workspace_version`)*

## Community creation gates (v0.41.0 — RFC-057)

- [x] `/communities/new` is a top-level route guarded by `require_auth` and `require_active_admin_somewhere`. *(release_gates.rs `rfc057_community_creation_is_guarded_active_admin_only`)*
- [x] `COMMUNITY_CREATION_ENABLED` is explicit: true for dev/staging review, false for production default. *(wrangler.toml + release gate)*
- [x] Create POST uses `CREATE_COMMUNITY` form-token purpose, stores the created community id as replay result, and redirects duplicate submits to the created community. *(release_gates.rs `rfc057_token_idempotency_rate_limit_and_timezone_are_fixed`)*
- [x] Creation is rate-limited by authenticated user, session, and client IP. *(rate_limit.rs + release gate)*
- [x] Production UI exposes Japan time only and rejects unsupported timezone submissions server-side. *(community_create.rs + release gate)*
- [x] D1 writes are limited to `communities`, `community_memberships`, and `audit_log`; no members/events/templates/notes/invites are copied or generated. *(release_gates.rs `rfc057_creation_writes_only_community_membership_and_audit`)*
- [~] Staging/local smoke verifies eligible admin success, anonymous denial, non-admin denial, token replay, rate limit, and audit rows. *(runtime evidence pending)*

## Calendar workflow gates (v0.42.0 — RFC-058)

- [x] Calendar supports route-backed previous/current/next month links and day agenda filters. *(release_gates.rs `rfc056_calendar_page_owns_calendar_and_switcher`)*
- [x] Calendar event queries remain active-community and visible-month scoped. *(communities.rs `calendar_month_for_community`)*
- [x] Calendar community switching preserves selected month/day with validated `communities:YYYY-MM[:YYYY-MM-DD]` next values. *(community.rs `calendar_next_destination`)*
- [x] Browser smoke verifies month navigation, day filtering, clear filter, and community switching at mobile widths and with JavaScript disabled. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc058/rfc058-calendar-smoke-results.json`)*

## Operational gates

- [x] `GET /healthz` returns `{"ok":true}`. *(health.rs get_health)*
- [x] `GET /version` returns build version. *(health.rs get_version reads BUILD_VERSION var)*
- [x] Rollback procedure documented and understood. *(docs/src/deployment.md §Rollback: `wrangler rollback --env production`)*
- [x] Log persistence approach documented. *(docs/src/deployment.md §Log persistence: Cloudflare Logpush to R2/S3)*
- [ ] D1 migration applied to staging and rehearsed. *(operator task: `bun run migrate:prod` against staging env)*
- [ ] Secrets (`HMAC_PEPPER`) set in production via `wrangler secret put`. `SESSION_COOKIE_DOMAIN` is a **`[vars]` binding** (not a secret) — set it per environment in `wrangler.toml`; leave unset for a host-only cookie. *(operator task — RFC-038)*
- [ ] Logpush configured for production. *(operator task: Cloudflare dashboard)*
- [ ] No critical open security issues. *(final security review before go-live)*
