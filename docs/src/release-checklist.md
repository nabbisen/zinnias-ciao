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
- [x] Calendar selected-day agenda offers active admins a route-backed Create Event action with the date prefilled. *(communities.rs + admin/events.rs — RFC-059)*
- [x] Community switcher auto-submit is implemented in external `app.js`, not inline `onchange`, and the shell cache-busts `app.js` with a visible submit fallback for stale/no JS. *(render.rs + app.js + release gate)*
- [x] Event Detail shows status, participants, and notes. *(event.rs get_event_detail)*
- [x] Member can set own status (Going / No Go / clear). *(event.rs post_my_status + validate_status_transition)*
- [x] Member can save, edit, and delete own note. *(event.rs post_my_note, delete_my_note)*
- [x] Admin can create event (single and multi-day). *(admin.rs post_create_event + event_write::create_event)*
- [x] Admin Create Event switcher stays on Create Event for the selected community, so new events bind to the selected community URL. *(admin/events.rs + `/switch?next=admin_events_new`)*
- [x] Admin can cancel event (confirmation required). *(admin.rs get_cancel_event shows confirmation; post_cancel_event soft-cancels)*
- [x] Admin can create a similar new event from a cancelled event without copying schedule, attendance, or memos. *(admin/events.rs `get_recreate_event`, `post_create_event` — RFC-060)*
- [x] Admin can generate and revoke invite codes. *(admin/members.rs: rejection-sampling generator writes HMACs to `invite_codes`; `codlet::revoke_invite` delegates to `invite_db::revoke`)*
- [x] Admin can remove member (last-admin guard active). *(admin.rs post_remove_member: count_admins guard)*
- [x] Admin member management is discoverable from Home and My Page, and invite generation is reachable as a child action. *(home.rs, me.rs, admin/members.rs — RFC-061)*
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
- [x] Cancel-and-recreate source IDs are revalidated on POST and rejected unless same-community and cancelled. *(release_gates.rs RFC-060 gate)*
- [x] Community switcher admin-member targets require an active admin role in the destination community; member-only destinations fall back to Home. *(community.rs + release gate — RFC-061)*
- [x] Script tag in note/title/name renders as text (not executed). *(render.rs escape_html used at every user-content insertion; test: escape_script_tag)*
- [x] Private page cache cleared on logout. *(RFC-042: authenticated HTML is never cached; only static shell assets are stored. No private cache exists to clear — the property holds trivially. PURGE_PRIVATE is retained for defence-in-depth.)*

## Auth storage gates (v0.38.6)

- [ ] `HMAC_PEPPER` secret is set in the target environment, either by bootstrap seeding or by `wrangler secret put` with the target environment's ignored local config.
- [ ] `RATE_LIMIT` KV namespace is created and bound in ignored local Wrangler config for the target environment.
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
- [x] Member management navigation, role visibility, invite child navigation, and removal confirmation copy are usable at mobile width and 200% text scaling. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc061/rfc061-member-management-smoke-results.json`)*
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
- [x] i18n parity test covers all 170 EN/JA string pairs; catches empty strings and copy-paste errors. *(release_gates.rs `i18n_en_ja_parity_count`)*
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

- [x] i18n parity gate covers all 170 EN/JA pairs. *(release_gates.rs `i18n_en_ja_parity_count`)*
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

## Calendar admin workflow gates (v0.43.0 — RFC-059)

- [x] Selected Calendar day renders a create-event link only for active admins. *(release_gates.rs `rfc059_calendar_create_from_day_is_route_backed`)*
- [x] Create Event validates `day=YYYY-MM-DD` and prefills the date field. *(admin/events.rs `valid_prefill_day`)*
- [x] Create Event community switcher preserves a valid Calendar-selected day. *(community.rs `admin_events_new_destination`)*
- [x] Browser smoke verifies admin create-from-day, date prefill, switch preservation, and non-admin absence. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc059/rfc059-calendar-create-day-smoke-results.json`)*

## Event edit semantics gates (v0.44.0 — RFC-051)

- [x] Schedule editing is limited to single-day non-recurring events. *(admin/events.rs `event_schedule_editable`)*
- [x] Multi-day and recurring edit screens render only title/location/description fields plus a read-only schedule summary. *(release_gates.rs `rfc051_event_edit_semantics_are_details_only_for_multi_day`)*
- [x] Details-only validation does not require schedule values and rejects direct schedule-field submissions. *(admin/events.rs `validate_event_details`, `edit_post_contains_schedule_fields`)*
- [x] Whole-event cancellation copy states all dates are cancelled for multi-day/recurring events. *(admin/events.rs + i18n)*
- [x] Browser smoke verifies single-day, multi-day, recurring edit screens and whole-event cancellation at mobile width and 200% text scaling. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc051/rfc051-event-edit-semantics-smoke-results.json`)*

## Cancel-and-recreate assistance gates (v0.45.0 — RFC-060)

- [x] Cancelled Event Detail renders `似た内容で新しいイベントを作成` only for active admins. *(release_gates.rs `rfc060_cancelled_event_recreate_is_admin_only_and_details_only`)*
- [x] `/c/:cid/admin/events/:eid/recreate` requires same-community admin access and a cancelled source event. *(admin/events.rs `get_recreate_event`)*
- [x] Replacement form pre-fills title, location, and description only, and leaves date/time/repeat controls blank/default. *(admin/events.rs `render_recreate_event_create_fields`)*
- [x] Replacement POST revalidates `copy_source_event_id`; active, cross-community, or inaccessible sources are rejected. *(admin/events.rs `post_create_event`)*
- [x] Replacement create records only safe source-event provenance when present. *(audit metadata `created_from_cancelled_event_id`)*
- [x] Browser smoke verifies mobile-width admin/member cancelled Event Detail, 200% replacement form, no horizontal scroll, explicit schedule entry, helper copy readability, and old event remains cancelled. *(sandboxed incognito Chromium smoke: `.git-exclude/evidence/rfc060/rfc060-cancel-recreate-smoke-results.json`)*

## Calendar feed privacy gates (v0.46.0 — RFC-053)

- [x] Calendar feed page shows the reviewed bearer-link privacy warning and fixed Japanese generate/disable status messages. *(calendar.rs + i18n)*
- [x] Calendar feed redirects use fixed flash codes, not raw user-controlled query text or English messages. *(release_gates.rs `rfc053_calendar_feed_privacy_and_revocation_ux_is_guarded`)*
- [x] Regenerating a feed revokes the old token before issuing the replacement; disabling revokes active tokens for that membership/community. *(calendar.rs + db/calendar.rs)*
- [x] Calendar token generate/revoke audit rows do not include token-bearing metadata. *(release gate)*
- [x] ICS output remains community-scoped and limited to title, time, location, and status; no participant status, notes, invite codes, member names, descriptions, or admin fields. *(release gate)*
- [x] ICS feed responses send `Cache-Control: no-store, private`, `Referrer-Policy: no-referrer`, and `X-Content-Type-Options: nosniff`. *(calendar.rs)*
- [x] Browser smoke verifies generate, regenerate, old URL revocation, disable, header values, scoped ICS body, mobile 200% text, and sandboxed/incognito Chromium launch. *(evidence: `.git-exclude/evidence/rfc053/rfc053-calendar-feed-privacy-smoke-results.json`)*

## Runtime evidence collector prototype (v0.47.0 — RFC-050/RFC-045)

- [x] `bun run smoke:runtime -- <url>` collects evidence from an already-running Worker URL; Wrangler remains the owner of local start/deploy. *(scripts/runtime-smoke.mjs)*
- [x] Prototype route checks cover `/healthz`, `/version`, `/join`, `/offline`, `/manifest.webmanifest`, and `/sw.js` with representative security/cache headers. *(scripts/runtime-smoke.mjs)*
- [x] Prototype browser checks launch sandboxed/incognito Chromium without `--no-sandbox`, capture mobile screenshots, exercise 200% text size, and render `/join` with JavaScript disabled. *(scripts/runtime-smoke.mjs)*
- [x] Prototype evidence path and manual RFC-050 evidence template are documented. *(docs/src/staging-runtime-prototype.md)*
- [~] Hosted Cloudflare staging smoke executed and evidence attached. *(operator task: deploy staging with `BUILD_VERSION` set to the release label, then `EXPECTED_VERSION=v0.53.0 bun run smoke:runtime -- <deployed-worker-url>`)*
- [~] Hosted staging exposure reviewed: non-production data only, separate staging resources/secrets, short public window, and route disabled/removed or Worker deleted after evidence if no longer needed. *(operator task — RFC-050 staging exposure policy)*
- [~] Hosted staging bootstrap invite generated for authenticated checks. *(operator task: `bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"`; keep the printed invite code private)*
- [~] Seeded authenticated RFC-050 flows, race checks, real-phone 200% scaling, Logpush, and CPU/runtime review completed. *(manual/operator evidence)*

## Member management navigation gates (v0.48.0 — RFC-061)

- [x] Admin Home exposes member management when the active user is an admin, and member Home does not. *(release_gates.rs `rfc061_member_management_is_discoverable_from_admin_workflows`)*
- [x] My Page shows a dedicated admin section with member-management and export links only for active admins. *(me.rs + release gate)*
- [x] Member management shows role labels, marks the current user, hides self-removal, and keeps the invite action as a child link. *(admin/members.rs + smoke evidence)*
- [x] Invite generation links back to member management. *(admin/members.rs + smoke evidence)*
- [x] Community switcher preserves member-management and invite pages only for destination communities where the current user is an admin. *(community.rs + release gate + smoke evidence)*
- [x] Committed browser smoke verifies the RFC-061 workflow with local Wrangler D1/dev and sandboxed/incognito Chromium without `--no-sandbox`. *(scripts/smoke/member-management.mjs; evidence `.git-exclude/evidence/rfc061/`)*

## Admin role transfer gates (v0.49.0 — RFC-062)

- [x] Member management shows one role-change action per non-self row: promote for members and demote for admins. *(admin/members.rs + smoke evidence)*
- [x] Promote and demote use separate confirmation routes with dedicated form-token purposes; requested role is not accepted from form data. *(community.rs + role_transfer.rs + release gate)*
- [x] Role changes are scoped by membership id, community id, active membership, and current role. *(membership.rs + release gate)*
- [x] Last-admin demotion and admin removal are guarded by conditional SQL writes that re-check active admin count inside the update. *(membership.rs + release gate)*
- [x] Self-demotion direct URLs, non-admin admin routes, and invalid target memberships use generic safe denial. *(role_transfer.rs + smoke evidence)*
- [x] Successful role changes audit direction-specific action names without metadata. *(role_transfer.rs + release gate)*
- [x] Admin invite generation remains member-role only; admin-granting invite UI is not part of this slice. *(admin/members.rs + release gate)*
- [x] Committed browser smoke verifies the RFC-062 workflow with local Wrangler D1/dev and sandboxed/incognito Chromium without `--no-sandbox`. *(scripts/smoke/admin-role-transfer.mjs; evidence `.git-exclude/evidence/rfc062/`)*

## Member lifecycle policy gates (v0.50.0 — RFC-063)

- [x] RFC-063 accepts removal-only policy: re-add means sending a new invite and creating a new membership, not reactivating the old membership. *(RFC-063)*
- [x] Removal copy states access ends and past attendance/notes remain in both EN and JA. *(i18n + release gate)*
- [x] Member-management surfaces expose no restore, reactivate, or suspension controls in this slice. *(release_gates.rs `rfc063_removal_only_policy_is_locked`)*
- [x] Invite redemption creates a fresh random user and membership and does not merge by display name. *(join.rs + release gate)*
- [x] Active member lists and authorization queries continue to exclude removed memberships. *(membership.rs + release gate)*
- [x] Operations docs explain that returning removed members receive a new invite and that past records stay on the old membership. *(docs/src/operations.md)*
- [x] Committed browser smoke verifies removal confirmation copy at 200% text, removal submit, disappearance from the active member list, and absence of restore/suspend controls. *(scripts/smoke/member-management.mjs; evidence `.git-exclude/evidence/rfc063/`)*

## Active-member help-signin gates (v0.51.0 — RFC-024)

- [x] Help-signin codes target active memberships, not display names or bare user ids. *(db/relink.rs + release gate)*
- [x] Redemption re-checks target membership activity and community before minting a session. *(db/relink.rs + release gate)*
- [x] Codes are HMAC-only at rest, short-lived, and single-use. *(migration 0008 + db/relink.rs + release gate)*
- [x] Successful redemption creates a new session and revokes other active sessions for the target `user_id`. *(handlers/relink.rs + db/session.rs + release gate)*
- [x] Failed redemption uses one generic invalid/expired error and is rate-limited without membership audit rows. *(handlers/relink.rs + release gate)*
- [x] Removed-member reactivation, former-member UI, and display-name merge remain out of scope. *(RFC-024/RFC-063 + release gate)*
- [x] Operations docs explain that help-signin is only for active members who lost browser/session access. *(docs/src/operations.md)*
- [x] Committed browser smoke verifies active-only row action, 200% text confirmation copy, code shown once, fresh-context redemption, reused-code generic error, and cross-community non-authorization. *(scripts/smoke/help-signin.mjs; evidence `.git-exclude/evidence/rfc024/`)*

## Rust module boundary cleanup gates (v0.52.0 — RFC-064 Phase 1)

- [x] `workers/ssr/src/handlers/admin/events.rs` is a facade that re-exports only route handler entry points. *(events.rs + implementation review)*
- [x] Admin event workflows are split into focused create, recreate, edit, cancel, attendance, and note-hide modules. *(workers/ssr/src/handlers/admin/events/*.rs)*
- [x] `forms.rs`, `summary.rs`, `policy.rs`, and `support.rs` separate presentation fragments, schedule summary rendering, policy/validation helpers, and small support utilities. *(implementation review boundary checks)*
- [x] No new Cargo crate is introduced in Phase 1; crate extraction remains deferred by RFC-064 trigger criteria. *(Cargo.toml + RFC-064)*
- [x] Admin event source-contract release gates follow the facade plus child modules. *(release_gates.rs `ADMIN_EVENTS_SRC`)*
- [x] All admin event child modules are below the 300 effective-line guideline. *(implementation review line-count evidence)*
- [x] Browser smoke is not required for this slice because no route, form field, rendered-copy, or intended browser behavior changed beyond version/cache-buster alignment. *(RFC-064 + implementation review)*

## Render boundary cleanup gates (v0.53.0 — RFC-064 Phase 2)

- [x] `workers/ssr/src/render.rs` is a facade that preserves the existing `crate::render::*` caller surface. *(render.rs + implementation review)*
- [x] Shared render helpers are split into focused shell, nav, status, notes, event-card, time, participants, and errors modules. *(workers/ssr/src/render/*.rs)*
- [x] `shell.rs` owns normal page construction, and `errors.rs` owns status-coded error response helpers. *(implementation review boundary checks)*
- [x] Non-error render modules remain free of D1, auth, audit, form-token, session, database, `Request`, and `Env` usage. *(implementation review static search)*
- [x] Render source-contract release gates follow the facade plus child modules. *(release_gates.rs `RENDER_SRC`)*
- [x] Render tests remain split from implementation and use explicit imports instead of `use super::*`. *(workers/ssr/src/render/tests.rs)*
- [x] All render child modules are below the 300-line guideline. *(implementation review line-count evidence)*
- [x] Browser smoke is not required for this slice because no route, form field, rendered-copy, or intended browser behavior changed beyond version/cache-buster alignment. *(RFC-064 + implementation review)*

## Operational gates

- [x] `GET /healthz` returns `{"ok":true}`. *(health.rs get_health)*
- [x] `GET /version` returns build version. *(health.rs get_version reads BUILD_VERSION var)*
- [x] Rollback procedure documented and understood. *(docs/src/deployment.md §Rollback: `wrangler rollback --env production`)*
- [x] Log persistence approach documented. *(docs/src/deployment.md §Log persistence: Cloudflare Logpush to R2/S3)*
- [x] Tracked `wrangler.toml` is release-gated to contain only placeholder D1/KV IDs. *(release_gates.rs: `tracked_wrangler_template_contains_only_placeholder_resource_ids`)*
- [ ] D1 migration applied to remote staging and rehearsed. *(operator task: `bun run migrate:staging`, which uses `wrangler d1 migrations apply --remote`)*
- [ ] Production commands use ignored `wrangler.production.local.toml`; staging commands use ignored `wrangler.staging.local.toml`. *(operator task — hosted config isolation)*
- [ ] Production bootstrap invite generated for initial release. *(operator task: `bun run bootstrap:production -- --community "Production Community" --admin "Admin"`; this sets production `HMAC_PEPPER`; keep the printed invite code private)*
- [ ] `SESSION_COOKIE_DOMAIN` is configured as a **`[vars]` binding** in the target environment's ignored local Wrangler config if needed; leave unset for a host-only cookie. *(operator task — RFC-038)*
- [ ] Logpush configured for production. *(operator task: Cloudflare dashboard)*
- [ ] No critical open security issues. *(final security review before go-live)*
