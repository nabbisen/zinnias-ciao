# MVP Release Gate Checklist (RFC-015)

Fill this for every release candidate before promoting to production.

Legend: `[x]` = verified by code inspection or automated test · `[~]` = requires human QA pass · `[ ]` = not yet done

Release review must check security-sensitive changes and every new or
materially changed form against the
[Application Threat Model](../developer/security-threat-model.md).

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
- [x] Invite and relink redemption reserve capacity in the fail-closed `AbuseLimiter` Durable Object (10 authenticated submissions / 5-minute window per canonical client-network subject) before credential lookup; a missing binding, storage failure, or malformed coordinator response denies the operation rather than allowing it. Concurrent bursts admit exactly the policy limit with no lost increments (verified for both the invite/relink and community-creation policies). *(abuse_control.rs, abuse_limiter.rs — RFC-078; native `cargo test` transition/ingress coverage and local `test:abuse-controls` workerd evidence for missing-binding 503, the 10/11th and 3/4th boundaries under sequential *and* concurrent load, and reset-after-valid-credential. RFC-050 exact-candidate hosted evidence — direct-ingress topology and Pseudo IPv4 classification — remains required before public/production B3 closure. **IPv6 client support is not confirmed/implemented for this deployment** — treat this service as IPv4-only in practice. The native IPv6 `/64` hosted-sharing sub-clause is **risk-accepted-open** by explicit owner decision dated 2026-07-28 (RFC-078 § Dated owner risk acceptances; RFC-050 E4a) and must never be represented as hosted-proven, working, or supported in any record; the `/64` grouping code and the `CF-Connecting-IPv6` rejection exist and are natively tested, but that is arithmetic-correctness evidence only, not proof of correct behavior against a real IPv6 client on hosted infrastructure.)*
- [~] Two RFC-078 evidence gaps are explicitly deferred rather than covered by local workerd evidence: (1) a genuinely malformed/duplicated `CF-Connecting-IP` header is proven fail-closed only at the native pure-function level (`abuse_control::tests::rejects_*`), not by sending a real malformed header through local `wrangler dev`, because local Miniflare's own `CF-Connecting-IP` injection behavior was not investigated deeply enough to construct a reliable attacker-controlled-header test locally; (2) a cross-origin tokenless `POST` spending no capacity relies on the pre-existing same-origin form-token behavior rather than a new dedicated cross-origin negative test. Revisit both if RFC-050 hosted evidence surfaces a reason to.
- [x] Session cookies have `HttpOnly; Secure; SameSite=Strict`. Host-only by default (no `Domain` attribute unless `SESSION_COOKIE_DOMAIN` var is set). *(session.rs build_session_cookie — RFC-038)*
- [x] Form token absent/invalid → POST rejected; replay is detected for every purpose, not only some. `form_token.rs::consume_detailed` distinguishes `Proceed` from every `Replay`, including `Replay(None)`, via a conditional UPDATE; every caller — the 20 handler call sites routed through `codlet::consume_token` (now itself `ConsumeResult`-returning) plus the one direct `join.rs` call site — matches the full enum explicitly (`matches!(result, ConsumeResult::Replay(_))`). **Corrected 2026-07-28:** a compatibility wrapper previously collapsed `Proceed` and `Replay(None)` into the same `Option::None`, so every purpose except display-name editing's stored `result_ref` could never detect a replay through it — RFC-050 Tooling Slice 5's concurrency evidence surfaced this as a real, reproducible defect (not a local-testing artifact), confirmed by reading `form_token.rs`, and fixed by removing the wrapper and migrating every call site to match on `ConsumeResult` directly. Subject is the authenticated `user_id`. *(codlet.rs, form_token.rs — RFC-037; remediation 2026-07-28)*
- [x] Admin invite generation reveals plaintext once only in the direct `200` body at a clean URL with `Cache-Control: no-store, private` and `Referrer-Policy: no-referrer`; token replay creates nothing and redirects cleanly; required-audit failure returns generic `503` and rolls back the invite. Legacy `code` query keys redirect before authentication/binding access. *(RFC-076 native/contracts gates and local `smoke:invite`.)*
- [x] Corrected isolated automated RFC-076 evidence passes 22 fail-closed no-JS/network assertions with exact `1/1/0` D1 deltas and native non-empty text selection. The owner also completed bounded human sections A–C: visible/selectable reveal with JavaScript disabled, direct response/network inspection, clean replay and no-store Back/Reload behavior without reconstruction, and legacy-query/referrer containment. Architecture review and owner acceptance close criterion 8 locally. *(`.git-exclude/evidence/rfc076/automated-no-js-browser-evidence.md` and `manual-no-js-browser-evidence.md`; RFC-050 hosted evidence remains open.)*
- [x] Community creation is authenticated, active-admin-only, feature-flagged, token-protected, idempotent, fail-closed rate-limited by user, session, and network in that order (3 / 24h each, via the `AbuseLimiter` Durable Object), audited, and does not auto-generate invite codes. *(release_gates.rs RFC-057 gates; abuse_control.rs/abuse_limiter.rs and `test:abuse-controls` — RFC-078)*
- [x] Cancel-and-recreate source IDs are revalidated on POST and rejected unless same-community and cancelled. *(release_gates.rs RFC-060 gate)*
- [x] Community switcher admin-member targets require an active admin role in the destination community; member-only destinations fall back to Home. *(community.rs + release gate — RFC-061)*
- [x] Script tag in note/title/name renders as text (not executed). *(render.rs escape_html used at every user-content insertion; test: escape_script_tag)*
- [x] Private page cache cleared on logout. *(RFC-042: authenticated HTML is never cached; only static shell assets are stored. No private cache exists to clear — the property holds trivially. PURGE_PRIVATE is retained for defence-in-depth.)*
- [~] New or materially changed forms were reviewed against the form-security baseline: POST repeats authorization, hidden fields are attacker-controlled, token purpose/resource binding is explicit, SQL writes are parameterized and scoped, render output is escaped, replay behavior is defined, audit metadata is minimal, no-JS behavior remains valid, and required evidence is identified. *(Application Threat Model - RFC-071)*

## Auth storage gates (v0.38.6)

- [ ] `HMAC_PEPPER` secret is set in the target environment, either by bootstrap seeding or by `wrangler secret put` with the target environment's ignored local config.
- [x] Root and every named Wrangler environment declare `HMAC_PEPPER` required; corrected disposable hosted evidence binds exact candidate `901855b` to classified deployment rejection, fixed non-mutating runtime `503`, ready valid-secret credential flows, permitted secret deletion, and strict teardown. *(RFC-077 criteria 8–9; architecture-reviewed and owner-accepted 2026-07-22; B2 closed. Real-target secret attachment remains the separate unchecked operator gate above.)*
- [ ] `ABUSE_LIMITER` Durable Object class/binding is provisioned for the target environment (RFC-078; operator task — first exact-candidate deploy provisions the namespace automatically from the inherited `[exports.AbuseLimiter]` declaration).
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
- [x] DST scope limitation documented in `docs/src/maintainer/operations.md` (review P1-2).
- [x] No-JS community switcher has a visible `<noscript>` submit fallback; confirmed in `render.rs` (review P1-4).
- [x] i18n parity test covers all 184 EN/JA string pairs; catches empty strings and copy-paste errors. *(release_gates.rs `i18n_en_ja_parity_count`)*
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

- [x] i18n parity gate covers all 184 EN/JA pairs. *(release_gates.rs `i18n_en_ja_parity_count`)*
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
- [x] Prototype evidence path is documented; the flat manual checklist that used to live alongside it is superseded by the tracked evidence templates and attestation below. *(docs/src/tester/staging-runtime-prototype.md)*
- [x] **Local** RFC-050 Tooling Slices 1–8 completed: version-metadata binding and strict `/version` schema, candidate manifest/redaction utilities, exact-identity smoke mode, E3 authenticated/browser flow collection, E4 concurrency/postcondition tooling, E5 negative-configuration fixtures, manual evidence templates with artifact hashing and leakage scanning, and the tracked per-candidate attestation with gate rules. *(commits `15b9409`, `c55787a`, `50d6310`; local-only — none of this is B4 hosted evidence by itself)*
- [~] Hosted Cloudflare staging smoke executed and evidence attached. *(operator task: deploy staging with `BUILD_VERSION` set to the release label, then `EXPECTED_VERSION=v0.63.0 bun run smoke:runtime -- <deployed-worker-url>`)*
- [~] Hosted staging exposure reviewed: non-production data only, separate staging resources/secrets, short public window, and route disabled/removed or Worker deleted after evidence if no longer needed. *(operator task — RFC-050 staging exposure policy)*
- [~] Hosted staging bootstrap invite generated for authenticated checks. *(operator task: `bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"`; keep the printed invite code private)*
- [~] Seeded authenticated RFC-050 flows, race checks, real-phone 200% scaling, Logpush, and CPU/runtime review completed against the frozen exact candidate. *(operator evidence, recorded per-gate in `docs/src/tester/release-candidates/<candidate-label>.md`; the E3/E4/E5 local collectors above prove the tooling works, not that the hosted candidate does)*

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
- [x] Operations docs explain that returning removed members receive a new invite and that past records stay on the old membership. *(docs/src/maintainer/operations.md)*
- [x] Committed browser smoke verifies removal confirmation copy at 200% text, removal submit, disappearance from the active member list, and absence of restore/suspend controls. *(scripts/smoke/member-management.mjs; evidence `.git-exclude/evidence/rfc063/`)*

## Active-member help-signin gates (v0.51.0 — RFC-024)

- [x] Help-signin codes target active memberships, not display names or bare user ids. *(db/relink.rs + release gate)*
- [x] Redemption re-checks target membership activity and community before minting a session. *(db/relink.rs + release gate)*
- [x] Codes are HMAC-only at rest, short-lived, and single-use. *(migration 0008 + db/relink.rs + release gate)*
- [x] Successful redemption creates a new session and revokes other active sessions for the target `user_id`. *(handlers/relink.rs + db/session.rs + release gate)*
- [x] Failed redemption uses one generic invalid/expired error and is rate-limited without membership audit rows. *(handlers/relink.rs + release gate)*
- [x] Removed-member reactivation, former-member UI, and display-name merge remain out of scope. *(RFC-024/RFC-063 + release gate)*
- [x] Operations docs explain that help-signin is only for active members who lost browser/session access. *(docs/src/maintainer/operations.md)*
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

## Contracts i18n boundary cleanup gates (v0.58.0 — RFC-064 Phase 3)

- [x] `packages/contracts/src/i18n.rs` is a facade with private child modules and facade-level re-exports. *(i18n.rs + implementation review)*
- [x] Existing `zinnias_ciao_contracts::i18n::EN_*` and `JA_*` caller paths remain unchanged. *(implementation review + contracts tests)*
- [x] String values, constant names, routes, forms, static assets, call sites, and rendered output are unchanged. *(implementation review preservation evidence)*
- [x] `packages/contracts/src/i18n/tests.rs` continues validating through the facade, not child modules. *(implementation review)*
- [x] All i18n implementation child modules are below the 300-line guideline. *(implementation review line-count evidence)*
- [x] Crate extraction is explicitly deferred under RFC-064 trigger criteria. *(RFC-064)*
- [x] Browser smoke is not required for this slice because no route, form field, rendered-copy, or intended browser behavior changed beyond version/cache-buster alignment. *(RFC-064 + implementation review)*

## Total community access recovery gates (v0.59.0 — RFC-069)

- [x] Operator recovery is disabled by default in tracked `wrangler.toml`, including production. *(wrangler.toml + release gate)*
- [x] `COMMUNITY_RECOVERY_TOKEN` is read only as a Worker secret and compared with constant-time equality. *(operator.rs + release gate)*
- [x] `POST /operator/recovery/community-access` creates one relink code only for an existing active admin membership in an active community. *(operator.rs + release gate)*
- [x] Disabled, unauthenticated, invalid community, invalid membership, non-admin, and removed-member cases converge on the generic not-found response. *(operator.rs + release gate)*
- [x] Recovery creation audit records `operator_recovery.admin_relink_created` with `operator_label`, `relink_code_id`, `membership_id`, and `community_id`; redemption audit includes the same `relink_code_id`. *(operator.rs + relink.rs + release gate)*
- [x] Maintained recovery script requires explicit target, URL, community id, admin membership id, operator label, `COMMUNITY_RECOVERY_TOKEN`, and production confirmation. *(scripts/recover-community-access.mjs + release gate)*
- [x] Maintainer operations docs describe enabling the temporary window, setting the secret, running the script, disabling the flag, deleting or rotating the secret, redeploying, and verifying generic not-found after closure. *(docs/src/maintainer/operations.md)*
- [ ] Hosted staging recovery drill executed with non-production data and endpoint closed afterward. *(operator task; do not store plaintext relink code in evidence files)*

## Documentation role-structure gates (v0.53.1)

- [x] mdBook navigation is organized by User, Developer, Maintainer, Tester, and Shared sections. *(docs/src/SUMMARY.md)*
- [x] Shared deployment and historical reference material lives under `docs/src/shared/` and is linked from role pages instead of duplicated. *(docs/src/shared/index.md)*
- [x] README and tracked operational comments point to the new role/shared documentation paths. *(README.md, wrangler.toml, workers/ssr/static/sw.js)*
- [x] Active documentation and RFC references no longer point to the removed flat docs paths. *(path scan + mdbook build)*
- [x] Browser smoke is not required for this slice because no route, form field, rendered-copy, or intended browser behavior changed beyond version/cache-buster alignment. *(documentation restructure)*

## Recurrence v2 gates (v0.54.0 — RFC-065)

- [x] Recurrence source of truth is `event_series`; `events.repeat_rule` and `events.repeat_count` remain compatibility summaries. *(migration 0009 + event_write.rs + RFC-065)*
- [x] Legacy migrated series do not derive local recurrence times from UTC clock text. *(migration 0009 + release gate `rfc065_legacy_migration_does_not_treat_utc_clock_as_local_time`)*
- [x] Existing `event_days.id` rows remain the attendance anchor during migration. *(migration 0009 + RFC-065 implementation review)*
- [x] Calendar materialization is bounded to the six-month forward window and far-future month requests do not write rows. *(communities.rs + smoke evidence)*
- [x] Rolling materialization continues after `materialized_through_day_date` and enforces one shared 64-row request budget. *(event_series.rs + domain tests + release gate)*
- [x] The Create Event form has no arbitrary repeat-count default of 8 and supports open-ended, until-date, and count end modes. *(forms.rs + smoke evidence)*
- [x] Occurrence-only cancellation preserves `event_day_id`, writes an exception row, and blocks further status changes for that date. *(occurrence.rs + event_write.rs + event.rs + smoke evidence)*
- [x] Exception rows database-check skip/cancel shape. *(migration 0009 + release gate `rfc065_exception_shape_is_checked_by_database`)*
- [x] Committed browser smoke verifies recurrence creation, Calendar materialization, far-future no-write behavior, and occurrence cancellation with local Wrangler D1/dev and sandboxed/incognito Chromium without `--no-sandbox`. *(scripts/smoke/recurrence-v2.mjs; evidence `.git-exclude/evidence/rfc065/`)*

## Event copy gates (v0.55.0 — RFC-066)

- [x] Event Detail exposes `このイベントをコピー` only to active admins. *(event.rs + browser smoke)*
- [x] Non-admin users do not see the copy action, and direct copy URL access does not reveal source state. *(copy.rs auth + browser smoke)*
- [x] Single-day source copy pre-fills title, location, description, date, start time, end time, and `copy_mode=event_copy`. *(copy.rs/forms.rs + browser smoke)*
- [x] Multi-day non-recurring source copy leaves schedule fields blank and shows the multi-day helper. *(copy.rs + browser smoke)*
- [x] Past recurring source copy copies recurrence frequency and local times but leaves `day_date`, occurrence count, and until date blank. *(copy.rs + browser smoke)*
- [x] Successful copied create produces a new event without copied attendance answers, notes, occurrence exceptions, cancellation state, source event/day/series IDs, or copied audit history. *(create.rs + browser smoke/D1 checks)*
- [x] RFC-060 cancelled-event recreate provenance remains separate from RFC-066 event-copy provenance. *(create.rs + release gate `rfc066_event_copy_is_admin_reviewed_prefill_not_clone`)*
- [x] Community switcher from the copy form lands on normal Create Event without hidden source-copy state. *(copy.rs + browser smoke)*
- [x] Mobile 390px viewport at 200% text scaling keeps helper text, controls, and submit action usable without horizontal overflow. *(scripts/smoke/event-copy.mjs; evidence `.git-exclude/evidence/rfc066/`)*

## Monthly attendance matrix gates (v0.56.0 — RFC-067)

- [x] Calendar keeps the ordinary month grid as the default and exposes matrix mode through `view=matrix`. *(communities.rs + release gate `rfc067_monthly_attendance_matrix_contract_is_guarded`)*
- [x] Active community members and admins can view the matrix; non-members receive the generic not-found response on direct matrix URLs. *(communities.rs + browser smoke)*
- [x] Matrix rows include active members only and use stable `display_name, id` ordering. *(membership.rs + release gate)*
- [x] Matrix cells use the reviewed symbols `○`, `×`, `済`, `?`, and `中`; multi-event cells use `answered/total` with accessible breakdowns. *(matrix/cells.rs + release gate + browser smoke)*
- [x] Matrix mode fetches one event-day row past the 300-row cap so over-cap months render the too-large fallback instead of silently truncated data. *(event.rs + release gate + unit tests)*
- [x] Community switching preserves matrix mode only through the exact reviewed `communities:YYYY-MM[:YYYY-MM-DD][:matrix]` grammar. *(community.rs + release gate + browser smoke)*
- [x] Member-visible matrix HTML omits CSV/export controls and export-only data attributes; admin CSV export is covered by RFC-068. *(release gate + RFC-068 browser smoke)*
- [x] Browser smoke verifies member/admin matrix views, non-member denial, switcher preservation, mobile 200% text scaling, matrix-only horizontal scrolling, and no CSV/export copy. *(scripts/smoke/monthly-attendance-matrix.mjs; evidence `.git-exclude/evidence/rfc067/`)*

## Calendar matrix CSV export gates (v0.57.0 — RFC-068)

- [x] CSV export controls are rendered only for active community admins on matrix mode. *(communities.rs + matrix.rs + release gate `rfc068_calendar_matrix_csv_export_contract_is_guarded`)*
- [x] Active non-admin members can view the matrix but do not receive export controls, `data-date`, `data-member-name`, or `data-export-value` attributes. *(matrix renderer tests + browser smoke)*
- [x] CSV is generated in the browser from rendered admin matrix attributes; no server CSV/data endpoint is added. *(app.js + release gate + browser smoke)*
- [x] The browser sends a metadata-only audit POST before download and does not include matrix contents, member names, statuses, CSV bytes, notes, tokens, invite data, sessions, or filenames. *(communities.rs + implementation review)*
- [x] The audit action is `calendar_matrix_csv.export_requested`, scoped by a dedicated month-bound form token. *(communities.rs + release gate)*
- [x] CSV output uses deterministic filename `ciao-attendance-YYYY-MM.csv`, UTF-8 BOM, CRLF row endings, quoted values, escaped quotes, and formula hardening. *(app.js + browser smoke)*
- [x] Browser smoke verifies member export absence, admin CSV download, formula hardening, audit request, no server CSV endpoint request, and layout overflow checks in sandboxed/incognito Chromium without `--no-sandbox`. *(scripts/smoke/calendar-matrix-csv-export.mjs; evidence `.git-exclude/evidence/rfc068/`)*

## Calendar events list and day-detail gates (RFC-073)

- [x] `CalendarView` parses exactly three variants (`Month`, `List`, `Matrix`) through one closed enum; any unrecognized `view` falls back to Calendar. *(matrix.rs `CalendarView::from_query` + native tests)*
- [x] The Calendar tab renders the month grid and an always-present day-detail section (`id="calendar-day-detail"`) and does not render the monthly events list; with no day selected the section shows a short prompt instead of the full month. *(calendar.rs `render_calendar_day_detail` + native tests)*
- [x] A date-cell click navigates to `...&day=YYYY-MM-DD#calendar-day-detail`, landing on the day's detail rather than the top of the page; the fragment target exists in the DOM on every Calendar render. *(calendar.rs day-cell href + browser smoke)*
- [x] The Events list tab renders the full month regardless of any `day` in the query, and its own tab link omits `day`. *(calendar.rs `render_calendar_list` + native tests + browser smoke)*
- [x] The Attendance table tab and admin CSV export (RFC-067/RFC-068) are unchanged. *(browser smoke re-run unchanged)*
- [x] Community switching preserves `month`, `day`, and `view` — including `view=list` — through the exact reviewed `communities:YYYY-MM[:YYYY-MM-DD][:list|:matrix]` grammar, and emits no fragment. *(community.rs `calendar_next_destination` + native tests + browser smoke)*
- [x] The Events list tab label (`EN_CALENDAR_VIEW_LIST` / `JA_CALENDAR_VIEW_LIST`) passes the i18n EN/JA parity gate. *(release_gates.rs `i18n_en_ja_parity_count`)*
- [x] No new query is introduced per Calendar render: the Events list tab reuses the same month-bounded query the Calendar tab already performs. *(communities.rs `get_communities`)*
- [x] Browser smoke verifies Calendar-tab grid without the list, the day-detail fragment target, admin create-on-day scoping, Events-list full-month behavior with `day` present, Matrix/CSV unchanged, switcher preservation with no fragment, and mobile 200% text scaling. *(scripts/smoke/calendar-views.mjs; evidence `.git-exclude/evidence/rfc073/`)*

## Community switch route preservation gates (RFC-074)

- [x] `/switch` accepts a closed, explicitly enumerated `next` grammar (`home`, `me`, `calendar_feed`, `communities[:...]`, `admin_events_new[:...]`, `admin_members`, `admin_invites`, `admin_export`, `admin_templates`) and constructs every destination from the validated target community id; `next` is never treated as a URL, path, or fragment. *(community.rs `switch_destination` + native tests)*
- [x] Every `admin_*` token requires an active admin role in the **target** community, re-checked independently of any admin role the caller holds elsewhere; every member-level token requires active membership in the target. *(community.rs `is_admin_target` + native privilege-escalation test)*
- [x] An unrecognized top-level token, an absent token, or any value shaped like a path, URL, or fragment (including `%2F` and `../`) falls back to the target's Home without error. A malformed value inside an already-recognized family (`communities:`/`admin_events_new:`) preserves RFC-073's own existing fallback (bare Calendar / bare Create Event), not Home — a deliberate, reviewed distinction. *(community.rs native tests)*
- [x] No community-scoped identifier (member, event, invite, template) is preserved across a switch. *(route-family matrix; help-signin and member-action confirmations pass `admin_members`, never a member id)*
- [x] The switch handler emits no fragment under any input. *(native test over every accepted token + browser smoke)*
- [x] My Page, Calendar feed settings, Export, Templates, member-removal/promote/demote/help-signin confirmations pass their assigned token; Event Detail, note-delete confirmation, and the event admin pages (attendance/cancel/edit) deliberately stay on the default switcher and fall back to Home. *(handler wiring + `rfc074_fallback_family_pages_stay_on_default_switcher` release gate)*
- [x] The no-JS switcher submit path continues to work — the switcher is a `<form method='get' action='/switch'>` with no app JavaScript. *(browser smoke drives the form's own native submit)*
- [x] RFC-067's matrix-preservation contract holds, proven behaviorally (exact destination URL for a matrix token) natively in `ssr`, in addition to the pre-existing `release_gates.rs` source-literal pin. *(community/tests.rs `switch_destination_communities_token_family`)*
- [x] Browser smoke verifies switching from Calendar, My Page, member management, and Create Event (each to both an admin and a member-only target where relevant), plus Event Detail falling back to Home with no event id preserved. *(scripts/smoke/community-switch-route-preservation.mjs; evidence `.git-exclude/evidence/rfc074/`)*

## Locale seam gates (RFC-072 Slice A — not user-visible yet)

The language setting is **not reachable from the UI** in this slice — no link
exists anywhere to `/c/:cid/me/language`, and that page is not yet built (see
the Slice A review request for why). Nothing below is a user-visible
language-switching claim; it establishes the mechanism only.

- [x] `Locale` parses exactly `ja`/`en` and rejects everything else (empty, wrong case, `ja-JP`/`en-US`, `jp`, whitespace); no unvalidated locale reaches a render path. *(packages/contracts/src/locale.rs + native tests)*
- [x] The i18n accessor boundary (`i18n::Localized` + `i18n::t`) resolves a locale-aware pair to the correct language's string, compile-checked (not a runtime key lookup); migrating a page is a single mechanical substitution per string, with no per-call-site `match locale { ... }`. *(packages/contracts/src/i18n.rs + native tests)*
- [x] `migrations/0011_membership_ui_language.sql` adds a nullable `ui_language` to `community_memberships` with a closed `CHECK('ja','en')` set; applied against a disposable local D1 and confirmed to reject an out-of-set value and accept `ja`/`en`; no existing row is written. *(migration + `rfc072_migration_0011_ui_language_check_is_closed` release gate)*
- [x] Locale resolution (active membership preference, else Japanese) reads `ui_language` from the same membership lookup every localized page already performs — no additional D1 query. *(db/membership.rs `find_active` + authz.rs `require_membership`)*
- [x] A stored value outside the allow-list falls back to Japanese at render time rather than panicking; a release gate asserts no `unwrap`/`expect` on the locale read path. *(authz.rs `resolve_locale` + native tests + `rfc072_locale_resolution_never_panics_on_a_bad_stored_value` release gate)*
- [x] My Page is migrated to the accessor as the proof the seam works: `html lang` and every rendered string derive from the same resolved locale, in both languages. *(handlers/me.rs `get_me` + native tests for `html lang` and locale-selected labels)*
- [x] No authorization, validation, or error-classification decision branches on locale anywhere in this slice — locale selects rendered text only. *(handlers/me.rs, authz.rs)*
- [x] Existing i18n parity gate (254 EN/JA pairs) passes unchanged; none of the 254 existing pairs were modified. *(release_gates.rs `i18n_en_ja_parity_count`)*
- [ ] The language settings page (`GET`/`POST /c/:cid/me/language`) — blocked on missing copy for this specific page; see the Slice A review request's escalation. Not routed, not linked, not built yet.

## Locale seam gates (RFC-072 Slice B — member-facing core, still not user-visible)

The language setting is still **not reachable from the UI** in this slice —
the settings page now exists and is routed, but nothing links to it (that is
Slice C's job). Nothing below is a user-visible language-switching claim.

- [x] `GET`/`POST /c/:cid/me/language` is built and routed (superseding the open Slice A item above): requires session + active membership, consumes a `change_ui_language` token bound to `membership_id`, accepts only `ja`/`en`, writes only the active row, is a no-op on the member's current value, and is not linked from anywhere yet. *(handlers/me.rs `get_language`/`post_language` + `rfc072_language_settings_post_is_reject_no_op_replay_and_target_safe` release gate)*
- [x] §7.1's silent-wrong-locale risk is made structurally unrepresentable: only `db::membership::find_active` returns a row carrying a resolved `locale` (as `ActiveMembershipRow`); the plain `MembershipRow` returned by every other membership query has no locale field to reach for. *(db/membership.rs + `rfc072_locale_is_only_ever_read_from_find_active` release gate)*
- [x] Every page named in the RFC's member-facing core — Home, Communities/Calendar (month, list, and matrix modes), and Event Detail (including the note-delete confirmation) — is fully migrated: `html lang` and every rendered string derive from the same resolved locale, with no bare `i18n::JA_*` left behind on any of them. *(release_gates.rs `rfc072_member_facing_core_has_no_half_migrated_page`)* **Corrected 2026-07-30:** this claim was false for Event Detail. The gate above checked handler files only; it never saw that `event.rs` calls three shared render helpers (`render::status_form`, `render::note_form`/`admin_note_hide_form`, `render::participant_list`) that were never locale-migrated, so an English-preference member saw Japanese attendance buttons and note controls on an otherwise-English page. Fixed in Handoff 026 — see the new section below. RFC-072 acceptance criterion 9 was not actually met until that fix landed. **Corrected 2026-07-31:** criterion 9 still wasn't met after Handoff 026 either — see the Handoff 030 section below. The "member-facing core" this bullet names was itself an incomplete list: `handlers/calendar.rs` and `handlers/community_create.rs`, both linked directly from My Page, were never on it and stayed Japanese by omission through three RFC-072 slices, not by documented decision.
- [x] The community switcher's `aria-label` follows locale like every other switcher label, checked as exact attribute text (a bare substring check would false-positive: `EN_NAV_SWITCH_GO` is itself a substring of the English aria-label value). *(render/nav.rs `header_with_switcher_next_localized` + render/tests.rs)*
- [x] Three narrow exceptions are deliberate and reviewed, not oversights: (1) `communities.rs`'s matrix-export-audit 401 branch stays Japanese because it fires before any membership lookup exists — no locale source is available yet, same rationale as `render/errors.rs`; (2) the Japanese calendar-convention date/time formatting (`tz::date_label_ja` and the "{year}年{month}月" grid header) has no English counterpart anywhere in the codebase, so it stays Japanese-only on Calendar, Matrix, and Event Detail regardless of the viewer's locale; (3) `communities/matrix/cells.rs` is untouched — its aria-labels are whole Japanese sentences and its visible cell symbols are already language-neutral. *(release_gates.rs `rfc072_member_facing_core_has_no_half_migrated_page`)*
- [x] `me.rs`'s display-name edit sub-page (`get_display_name`/`post_display_name`) is unchanged from Slice A and still renders bare Japanese; it is not one of this slice's five migrated surfaces.
- [x] Existing i18n parity gate grows to cover every new pair this slice added; every pre-existing pair is unchanged. *(release_gates.rs `i18n_en_ja_parity_count`)*
- [x] No authorization, validation, or error-classification decision branches on locale anywhere in this slice — locale still selects rendered text only.
- [x] Locale resolution adds no additional D1 query to any budgeted route (the migrated pages' existing `require_membership`/`find_active` call already carried `ui_language`).

## Locale seam gates (RFC-072 Slice C — user-visible language switching)

This is the slice where claiming user-visible language switching becomes
true: the setting is linked from My Page, and every page a member can reach
from My Page — including dates, times, and the matrix's screen-reader
labels — honors the language they selected. Out-of-boundary surfaces
(admin, anonymous, error pages) remain Japanese by documented decision, not
by omission.

- [x] `GET`/`POST /c/:cid/me/language` is now linked from My Page, near the display-name action — superseding Slice A/B's "not yet linked" state. *(handlers/me.rs `get_me` + `rfc072_language_setting_is_linked_from_my_page` release gate)*
- [x] Dates and times follow locale everywhere they render: day labels use the RFC's decided English shape (`Mon, 3 Aug` — never all-numeric, the month always spelled or abbreviated) and month headers use the full month name (`August 2026`). Closes the gap Slice B left open on Calendar (month grid, day-cell aria-labels), Matrix (month header), and Event Detail. *(packages/contracts/src/tz.rs `weekday_en`/`month_name_en`/`date_label_en` + native tests; render/time.rs `format_day_time_tz_localized`)*
- [x] The matrix's screen-reader cell labels (date, member name, per-status counts) are now `Localized` templates substituted positionally, not composed Japanese literals. English uses label-value form (`events: 2`, `cancelled: 1`) rather than pluralized counts (never `"1 events"`); a native test asserts every template pair has the same placeholder count on both sides. Visible cell symbols (`○×済?中`, `N/total`) are unchanged. *(handlers/communities/matrix/cells.rs + `cell_label_templates_have_matching_placeholder_counts` native test)*
- [x] The display-name edit sub-page (`get_display_name`/`post_display_name`), linked directly from My Page, is now fully migrated — the gap Slice A left and Slice B didn't own. *(handlers/me.rs + `rfc072_member_facing_core_has_no_half_migrated_page` release gate, extended to cover it)*
- [x] Two copy constants were given more accurate names: `HOME_TODAY` moved to `general.rs` as `TODAY` (it renders on Calendar, not just Home; rename only, values unchanged), and the matrix's member-column header now has its own pair (`CALENDAR_MATRIX_MEMBER_COLUMN`) instead of reusing the `ROLE_MEMBER` badge label.
- [x] The anti-half-migration gate's exception list is reduced to the one that genuinely remains: `communities.rs`'s pre-auth 401 in `post_matrix_export_audit`, which fires before any membership lookup exists. *(release_gates.rs `rfc072_member_facing_core_has_no_half_migrated_page`)* **Corrected 2026-07-30:** this described the gate's handler-file coverage only. Handoff 026 extended the same gate to the shared render helpers those handlers call, adding two further per-file exceptions: `render/errors.rs` (17 refs, documented, unchanged rationale) and `render/event_card.rs` (4 refs, found to be dead code with no live caller — see the new section below).
- [x] Browser smoke verifies, on one member with two communities: switching to English via the plain no-JS settings form: My Page, Home, Calendar (month/list/matrix), Event Detail, and the display-name sub-page all render English with `lang="en"`, including a date label and a month header; switching back to Japanese flips a page back; the second community membership's `ui_language` is unaffected throughout (locale is membership-scoped, not user-scoped). *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc072/`)*
- [x] 200%-text scaling checked on My Page, the settings page, and Calendar's month view while rendering English: no horizontal overflow on any of the three. A pre-existing vertical overlap between the fixed bottom nav and page content in full-page screenshots was observed identically in both languages on My Page and Calendar — not a regression from English text length — and was not modified; see the Slice C review request. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc072/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, and `smoke:display-name` unmodified all still pass — this slice touches surfaces all five exercise.
- [ ] **Corrected 2026-07-31:** this section's own intro claimed "every page a member can reach from My Page ... honors the language they selected. Out-of-boundary surfaces (admin, anonymous, error pages) remain Japanese by documented decision, not by omission." This was false: `handlers/calendar.rs` (the ICS calendar-feed settings page) and `handlers/community_create.rs` (`/communities/new`) are both linked directly from My Page and neither was migrated, nor documented as an exclusion — omission, not decision. Fixed in Handoff 030 — see the new section below.

## RFC-072 residue fix — shared render helpers (Handoff 026)

RFC-072 Slices B and C scoped "fully migrated" by handler file. Event Detail's
handler (`event.rs`) was clean, but it calls shared render helpers —
`render::status_form`, `render::note_form`/`admin_note_hide_form`, and
`render::participant_list` — that were never locale-migrated, so an
English-preference member saw Japanese attendance buttons and note controls
above English-rendered text on the same page. Nothing was deployed, so no
member was actually served this; it was found and fixed before it shipped.
Correctness fix only — no styling change (reserved for RFC-075 Slice 2), no
Japanese wording changed.

- [x] `status_display`, `status_chip`, and `status_form` (`render/status.rs`) take `Locale` directly, not a `_localized` sibling — every live caller (`event.rs`'s `get_event_detail`) already has `locale` in scope. *(render/status.rs; render/tests.rs `status_display_follows_locale`, `status_chip_follows_locale`, `status_form_follows_locale`)*
- [x] `note_form` and `admin_note_hide_form` (`render/notes.rs`) take `Locale` directly, same reasoning — `event.rs` is their only caller anywhere in the codebase. *(render/notes.rs; render/tests.rs `note_form_follows_locale`, `admin_note_hide_form_follows_locale`)*
- [x] `participant_list` (`render/participants.rs`) takes `Locale` directly and threads it into its internal `status_display` call. *(render/participants.rs; render/tests.rs `participant_list_follows_locale`)*
- [x] Seven `Localized` pairs that were missing despite their raw `EN_`/`JA_` constants already existing were added: `STATUS_CLEAR`, `STATUS_CLEAR_LABEL` (`i18n/events.rs`), and `NOTE_SAVE`, `NOTE_SECTION_LABEL`, `NOTE_PLACEHOLDER_LABEL`, `NOTE_CHAR_HINT`, `NOTE_VISIBILITY` (`i18n/notes.rs`). No new copy was drafted — every pair reuses the existing, reviewed EN/JA text unchanged.
- [x] `render/event_card.rs` — investigated per §7.3, not deleted: `grep -rn "event_card("` across `workers/ssr/src` finds exactly one match, the function's own definition, and `git log -S` confirms it has had zero call sites since its introduction in `89eebb7` (RFC-064 Phase 2). Only its separate `CardDay` struct is still used elsewhere. Its `status_display` call site was updated to compile against the new signature (`Locale::Ja`, matching its pre-existing Japanese-only behavior) without migrating its own bare `i18n::JA_*` references or its raw "日間" literal — this file remains untouched otherwise, dead, and undeleted.
- [x] `rfc072_member_facing_core_has_no_half_migrated_page` now also asserts, per file, that `render/status.rs`, `render/notes.rs`, and `render/participants.rs` have zero bare `i18n::JA_` references, with `render/errors.rs` (17, documented) and `render/event_card.rs` (4, documented dead code) pinned as the only two remaining exceptions — asserted at their exact counts, not `> 0`, so a partial edit to either still fails the gate. Docstring corrected to describe what the gate actually checks (a static, file-level proxy for a page-level property) rather than overstating it as page-level. Mutation proof: reintroduced one bare reference into `status.rs`, observed the gate fail with the expected message, restored the file (`cmp` confirmed byte-identical), observed the gate pass again. *(release_gates.rs `rfc072_member_facing_core_has_no_half_migrated_page`)*
- [x] New tests: one per migrated helper (Japanese under `Locale::Ja`, English under `Locale::En`), plus `event_detail_attendance_buttons_and_cancelled_badge_render_in_the_same_language`, which proves the specific defect this package fixes — that `status_form`'s attendance buttons and `event.rs`'s already-locale-aware cancelled-day badge (`i18n::t(locale, i18n::OCCURRENCE_CANCELLED_BADGE)`) resolve to the same language for a given locale. `get_event_detail` itself is async/D1-bound and cannot be unit-tested directly, the same constraint noted by `rfc072_communities_and_event_pages_resolve_locale_and_html_lang_together`. *(render/tests.rs)*
- [x] `smoke:language`'s Event Detail scenario is extended to check the attendance buttons specifically (`form[action*="my-status"] button[name="status"]`), not just page text. The pre-existing `showsEnglishStatusLabels` check (`.text.includes('Going') && .text.includes('No answer')`) passed even against the pre-fix code, because the day's counts line (`"Going 0 · No Go 0 · No answer 3"`) was already locale-aware from an earlier slice and satisfied the same substring independent of what language the buttons rendered — confirmed by running the extended scenario against the pre-fix render code and observing the new `attendanceButtonsAreEnglish`/`attendanceButtonsHaveNoJapanese` checks fail while the old check still passed. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc072-residue/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:community-switch`, and `smoke:display-name` unmodified all still pass.

## RFC-072 boundary correction and a gate that cannot miss (Handoff 030)

Slices B and C's "member-facing core" gate (`rfc072_member_facing_core_has_no_half_migrated_page`) only ever checked files someone remembered to name in a hand-maintained list. Two pages linked directly from My Page — `handlers/calendar.rs` (the ICS calendar-feed settings page) and `handlers/community_create.rs` (`/communities/new`) — were never on that list and stayed Japanese by omission through three RFC-072 slices, not by documented decision. The Slice C entries and the RFC-075 Slice 3 entry above claiming completeness were wrong; both are corrected in place above.

- [x] `handlers/calendar.rs` is fully migrated: `html lang` and every rendered string derive from `require_membership`'s resolved locale. `get_ics_feed` (the unauthenticated bearer-token ICS route with no membership lookup) keeps two bare `i18n::JA_*` references, documented with the same rationale as `render/errors.rs`. *(handlers/calendar.rs; handlers/calendar/tests.rs `calendar_flash_message_follows_locale`)*
- [x] `/communities/new` has no `:cid` and therefore no "current membership" to read a locale from. Resolved per §7.2: from the admin membership that authorized access to the page, not an arbitrary "earliest-joined membership of any role" — the same query `require_active_admin_somewhere` already runs (`find_first_admin_for_user`), extended by one column (`ui_language`), no new D1 round-trip. Given the same row-shape treatment `find_active` got (Slice B): only `find_first_admin_for_user` returns a row carrying a resolved `locale` (`AdminMembershipRow`). §7.2 as written is a knowing extension to the RFC, not a literal reading of it — the RFC never specified a rule for a route with no `:cid`. *(db/membership.rs `AdminMembershipRow`/`find_first_admin_for_user`; authz.rs `require_active_admin_somewhere`; handlers/community_create.rs; handlers/community_create/tests.rs)*
- [x] The hand-maintained gate is replaced with a default-fail, inverted one: every non-test `.rs` file under `handlers/` and `render/` is walked; any file calling bare `render::page(` or containing a bare `i18n::JA_*` reference must appear in an explicit exclusion table with an exact pinned count and a written reason, or the gate fails. This is the version of the check that could have caught `calendar.rs`, which the old gate's list never named. Deriving the exclusion table surfaced two files the handoff's own suggested table missed — `handlers/admin/events/forms.rs` and `handlers/admin/events/summary.rs`, both shared admin form-rendering helpers — added as legitimate Slice-D-adjacent exclusions, reported as a finding rather than silently absorbed. Proven two ways: (1) a currently-clean file mutated to reintroduce a bare reference fails, restored (`cmp`-verified), passes again; (2) a file with real bare references removed from the exclusion table fails — the specific case the old gate could never produce. *(packages/contracts/tests/release_gates.rs `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`)*
- [x] No CSS change: `calendar.rs`'s CSS already landed in RFC-075 Slice 3 (a second, unavoidable touch, per the handoff's own acknowledgement); `community_create.rs`'s CSS remains scoped to a future RFC-075 slice, not bundled into this language fix.
- [x] `smoke:language` inverted its calendar-feed scenario (asserting the page now renders English after switching, not that it stays Japanese) and gained a new `/communities/new` scenario. The fixture's admin-authorizing membership was made the same membership the scenario switches to English, after the first run correctly failed by exposing that the admin membership `require_active_admin_somewhere` resolves and the membership the language switch touches were different rows in the original fixture — proving the switch and §7.2's resolution are both membership-scoped, not user-scoped, the same property Slice C's own membership-scoping check relies on. All six required smokes pass. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc072-boundary/`)*
- [x] `cargo test --workspace`, clippy (`-D warnings`), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors.

## Operational gates

- [x] `GET /healthz` returns `{"ok":true,"ready":true,"service":"ciao.zinnias"}` only with a valid pepper and a generic not-ready `503` otherwise. Corrected exact-candidate hosted evidence also proves classified required-secret rejection, runtime-negative behavior, valid credential flows, secret-deletion failure behavior, bounded non-mutation, and strict teardown. *(RFC-077 criteria 8–9; architecture-reviewed and owner-accepted 2026-07-22; B2 closed.)*
- [x] `GET /version` returns build version. *(health.rs get_version reads BUILD_VERSION var)*
- [x] Rollback procedure documented and understood. *(docs/src/shared/deployment.md §Rollback: `wrangler rollback --env production`)*
- [x] Log persistence approach documented. *(docs/src/shared/deployment.md §Log persistence: Cloudflare Logpush to R2/S3)*
- [x] Tracked `wrangler.toml` is release-gated to contain only placeholder D1/KV IDs. *(release_gates.rs: `tracked_wrangler_template_contains_only_placeholder_resource_ids`)*
- [ ] D1 migration applied to remote staging and rehearsed. *(operator task: `bun run migrate:staging`, which uses `wrangler d1 migrations apply --remote`)*
- [ ] Production commands use ignored `wrangler.production.local.toml`; staging commands use ignored `wrangler.staging.local.toml`. *(operator task — hosted config isolation)*
- [ ] On a proven-fresh, dark production target, bootstrap invite generated; then exact candidate identity and ready `/healthz` verified before `/join` or traffic. *(operator task: `bun run bootstrap:production -- --community "Production Community" --admin "Admin"`; keep the printed invite private)*
- [ ] `SESSION_COOKIE_DOMAIN` is configured as a **`[vars]` binding** in the target environment's ignored local Wrangler config if needed; leave unset for a host-only cookie. *(operator task — RFC-038)*
- [ ] Logpush configured for production. *(operator task: Cloudflare dashboard)*

## Calendar CSS migration gates (RFC-075 Slice 1)

Presentation-only. No handler behavior, form, route, or `data-*` attribute
changed. This slice does **not** change the Content-Security-Policy —
`style-src 'unsafe-inline'` stays until inline `style=` reaches zero, which
is a later, terminal slice of RFC-075, not this one.

- [x] A `cz-*` class layer exists in `app.css`, covering page/section layout, tabs, links, the calendar grid and day cell (with `today`/`selected`/`has-events` state expressed as classes), the day-detail block, the event list, and the matrix view shell. Every new colour/spacing/radius rule references a `--cz-*` token; new tokens were added where no existing one matched the exact pre-migration value, to avoid a visual change. *(workers/ssr/static/app.css)*
- [x] Calendar (`communities/calendar.rs`, `communities/calendar/events.rs`) and `communities.rs` are fully migrated to the class layer; the Matrix view shell and tab links (`communities/matrix.rs`) are migrated, but its per-row/per-column cell rendering stays inline by design, matching the RFC's own scope boundary (same exclusion already applied to `matrix/cells.rs` and `matrix/detail.rs`). *(release_gates.rs `inline_style_count_never_increases`)*
- [x] The Calendar accessibility gate is re-expressed against the rendered class set rather than literal colour/border values: today and selected state come from independent, non-overlapping conditions, and `app.css` gives them a genuinely different treatment (border-width, not colour alone), with a mutation proof confirming the gate still fails when that distinction is removed. *(release_gates.rs `calendar_overview_contract_is_explicit`)*
- [x] Two ratchets exist: total inline `style=` occurrences and total hardcoded hex-colour literals across `workers/ssr/src`, both re-measured and lowered from their pre-slice baseline, and both proven (for the inline-style ratchet) to fail on an increase and pass again after restore. *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest. **Corrected 2026-07-30:** an earlier form of this item claimed the cache key itself moved with the content. It did not, and should not have — a mid-RFC slice re-pins the digest only. The cache key (`sw.js` `CACHE_VERSION` and the `app.js` cache-buster in `render/shell.rs` and `handlers/static_files.rs`) is tied to the workspace version and moves at the next release, which is the only moment it must be correct. *(release_gates.rs `cached_asset_content_matches_pinned_hash`; see ROADMAP.md § Tagging is not deploying)*
- [x] Browser smoke verifies all three Calendar view modes (month, list, matrix) render correctly at mobile width and 200% text, in both Japanese and English, with no horizontal overflow. *(scripts/smoke/rfc075-calendar-css-migration.mjs; evidence `.git-exclude/evidence/rfc075/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, and `smoke:language` unmodified all still pass — this slice touches surfaces all five exercise.

## Event Detail and shared render helper CSS migration gates (RFC-075 Slice 2)

Presentation-only, same non-goals as Slice 1. No handler behaviour, form,
route, or `data-*` attribute changed. No i18n change — locale correctness
from Handoff 026 is untouched. **No CSP change** — `style-src 'unsafe-inline'`
stays until inline `style=` reaches zero, which this slice does not reach.

- [x] Event Detail (`handlers/event.rs`, including the note-delete confirmation sub-page) and the shared render helpers it calls (`render/status.rs`, `render/notes.rs`, `render/participants.rs`) are migrated to `cz-*` classes, along with `render/nav.rs` (bottom nav and both header variants), since every page renders it. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] Status (going/not_going/attended/no_answer) is now expressed as classes (`cz-status-text--*`, `cz-status-btn--*` plus a `--current` modifier), not inline colour triplets — the RFC-permitted "computed colours" exemption is for genuinely open-ended values, and a closed four-value set is exactly where a state class wins. `status_display`'s first return value changed meaning from an fg hex colour to this class suffix; every caller was updated in the same package and the rendered colour is unchanged. `status_triplet` has no caller left anywhere in the tree as a direct result — left in place rather than deleted, the same reasoning as §7.4 below.
- [x] **The WCAG AA contrast tests now read the shipped colour, not a copy.** Before this slice, each test in `token_and_color_regression.rs` called `parse_hex_color` on a hex literal held by the test itself, entirely disconnected from `render/status.rs`'s own constant of the same intended value — changing the shipped colour to something low-contrast left every test passing. The tests now `include_str!` `app.css`, extract the `.cz-status-text--*` class's `color: var(--x)` declaration, resolve `--x` from `:root`, and run the luminance maths on that resolved value. Mutation proof: changed `--cz-status-going-fg` to a known-failing colour, observed the test fail with the real computed contrast ratio, restored (`cmp`-verified), observed it pass again. The old-iOS-colours-fail regression test is unchanged (a literal check is correct for a "these values must never come back" guard). *(packages/contracts/tests/token_and_color_regression.rs)*
- [x] Both ratchets lowered from their Slice 1 values and re-measured with the gate's own counting function: inline `style=` 418 → 355 (63 removed — the sum of this slice's five files' occurrences, re-measured with the gate's own counter rather than trusted from the handoff's own table, which undercounted `nav.rs` at 5 when the actual count was 10); hardcoded hex-colour literals 322 → 283. Both proven (mutation: one inline style + one hex literal reintroduced, both ratchets fail, restored, both pass again). *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move, per the corrected rule from Slice 1. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] Browser smoke captures Event Detail at mobile width and 200% text, in both languages, plus My Page (showing the migrated bottom nav and header) in both languages. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc075-slice2/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, and `smoke:language` (with its Handoff 026 attendance-button checks) all still pass — this slice restructures the files those checks exercise without changing any locale-selected string.
- [x] **Known, pre-existing, not a Slice 2 regression** (originally disclosed here, unchecked; **fixed by Handoff 028, 2026-07-31** — see the new section below): Event Detail's status buttons wrap a single unbreakable word ("Attended" / "参加済み") in a three-column flex row; at 200% text this overflows the mobile viewport. Confirmed present at checkpoint `6415708` (before Slice 2) with the identical CSS properties, then only restructured from inline styles into classes — English overflows substantially and unambiguously (~150-190px) in both the pre- and post-migration measurement; the Japanese case is a marginal ~7px boundary case that did not reproduce in one post-migration measurement, which is ordinary layout/font-rendering variance, not a deliberate change.
- [ ] **§7.4 blocker, on the record:** `render/event_card.rs` remains dead code (confirmed at `ced6ae4`, Handoff 026, zero callers in its entire history) with 10 inline styles and 4 bare `i18n::JA_` references, neither touched by this slice per its explicit non-change scope. Because it is never migrated and never deleted, RFC-075's terminal criterion — inline `style=` reaching zero, so `'unsafe-inline'` can be dropped from `style-src` — is **currently unreachable**. Agreed: this is the blocker, and resolving it (deleting the dead `event_card` function while keeping the `CardDay` struct that shares its file) is a real extraction with its own risk, belonging in its own small, separately reviewed package — not bundled into a CSS migration. The Slice 2 review generalised this further: `status_triplet` (below) is now dead too, so the pre-terminal cleanup is a dead-code sweep, not a single-file deletion.

## Event Detail 200%-text overflow fix (Handoff 028)

Presentation-only, bounded accessibility remediation — not a numbered RFC, the
same handling used for the form-token replay fix. No CSP change; no version
bump or cache-key move; no i18n, route, form, or `data-*` change.

- [x] `.cz-status-form-buttons` gains `flex-wrap: wrap`. `.cz-status-btn` is otherwise unchanged (`flex: 1`, default `min-width: auto`) — the fix relies on, rather than removes, the content-driven minimum width that caused the original overflow: at any scale where all three labels' min-content widths fit inside an equal third of the row, that floor never engages and the row renders exactly as before; at 200% text, a label whose min-content width exceeds its equal-thirds share now wraps to its own row (via `flex-wrap`) instead of forcing the whole row past the viewport, and `flex-grow: 1` lets that wrapped button fill its row rather than sitting at a cramped intrinsic width. *(workers/ssr/static/app.css)*
- [x] An earlier attempt (`flex-basis: calc(50% - gap/2)` plus `min-width: 0` on every button, forcing a fixed 2-up-top layout) was rejected after testing showed it forced the 2+1 wrap **unconditionally, even at normal scale** — a real regression. Recorded here because it is the kind of mistake worth remembering: a flex-basis expressed as a fraction of the container computes independently of content, so three items whose bases sum to more than 100% wrap regardless of whether their actual content needs the room.
- [x] Verified at both scales: at 100% (normal), all three buttons render in one row, same widths (103px each in the Japanese fixture) and same vertical position as before this fix — confirmed via screenshot and a `noHorizontalScroll` check, not assumed. At 200%, both languages clear the viewport with a comfortable margin, not a marginal one: Japanese stays on a single row entirely (row right edge 357px against a 390px viewport, 33px clear); English wraps to two rows (Going/No Go on row one at 158px each, Attended alone on row two at the full 324px row width), also 357px right edge, same 33px margin. *(scripts/smoke/language-preference.mjs `statusButtonRowMeasurement`; evidence `.git-exclude/evidence/overflow-fix/`)*
- [x] Touch targets unaffected: `min-height: var(--cz-touch-min)` (44px) is untouched by this fix; measured button heights at 200% (108px Japanese, 60px English) are well above the floor in both the single-row and wrapped cases.
- [x] No font-size or padding reduction at any scale — the prohibited "shrink text to fit" anti-pattern was not used; the fix is purely a layout (wrapping) change.
- [x] `smoke:language`'s two `noHorizontalScrollAt200Percent` checks (Event Detail, both languages) now pass; the inline comments describing them as a known pre-existing failure are removed since this package closes that finding. A new normal-scale check/screenshot was added alongside them. All six required smokes pass. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/overflow-fix/`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*

## My Page, Home, and calendar-feed settings CSS migration gates (RFC-075 Slice 3)

Presentation-only, same non-goals as Slices 1 and 2. No handler behaviour,
form, route, `data-*`, i18n, or authorization change. **No CSP change** —
`style-src 'unsafe-inline'` stays until inline `style=` reaches zero, which
this slice does not reach.

- [x] `handlers/me.rs` (My Page, display-name edit, language settings — 44 inline styles), `handlers/home.rs` (Home — 21), and `handlers/calendar.rs` (the ICS calendar-**feed** settings page at `/c/:cid/me/calendar` — 14; not `handlers/communities/calendar.rs`, the Calendar page migrated in Slice 1) are fully migrated to `cz-*` classes. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] `handlers/calendar.rs` stays Japanese-only, unchanged — it renders through the plain (non-locale-aware) `render::page`, and was already outside RFC-072's scope before this slice. Its 14 bare `i18n::JA_*` references are unchanged in count and are not checked by `rfc072_member_facing_core_has_no_half_migrated_page` (that gate never named this file), and this package doesn't change that. Verified by browser smoke that the page stays `lang="ja"` even after the test member switches their preference to English. *(scripts/smoke/language-preference.mjs `calendar-feed-page-stays-japanese-only-after-english-switch`)* **Corrected 2026-07-31:** "already outside RFC-072's scope" was wrong — this page is linked directly from My Page and was in scope from Slice B onward; it was simply never caught, because the gate this bullet cites only ever checked files someone remembered to name, and this one was never named through three slices. Fixed in Handoff 030 — see the new section below. The smoke scenario named here (`calendar-feed-page-stays-japanese-only-after-english-switch`) was inverted by that same package, since asserting the page *stays* Japanese was itself asserting the defect.
- [x] Reuse was checked for exact property equivalence, not resemblance, per §7.1 — thirteen direct, byte-identical reuses of existing Slice 1/2 classes were found and used as-is (no shared class was modified to fit a new caller): `cz-page-main` (My Page's, Home's, and the feed page's `<main>`), `cz-hint` (My Page's help text; combined with `cz-hint--gap-top` for Home's first-run invite hint and its empty-community fallback — both already-existing modifier), `cz-event-cancelled-badge`, `cz-event-list-item`, `cz-event-link`, `cz-event-title`, `cz-event-meta`, `cz-event-list`, `cz-section-title` (Home's event cards and community-section heading, reused across pages since Home's event cards are structurally identical to Calendar's), `cz-note-flash` (the calendar-feed page's flash message), `cz-event-back-link`, and `cz-event-title-heading` (the feed page's back link and `<h1>`). One near-miss was caught and correctly *not* reused: Home's per-event location `<span>` (`style="color:#6e6e73"` only) superficially resembles `cz-event-location`, but that class also sets `font-size: .875rem` — a property the original omitted (inheriting instead). Reusing it would have silently changed the location text's size; a new `cz-home-event-location` class (colour only) was added instead.
- [x] Both ratchets lowered and re-measured with the gate's own counting function: inline `style=` 355 → **276** (79 removed — exactly 44+21+14, the three files' own re-measured counts, no discrepancy against the handoff's table this time); hardcoded hex-colour literals 283 → **223**. *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] `count_awaits(HOME_HANDLER_SRC)` (the RFC-029/RFC-044 query-budget gate) is unchanged — no `format!` block restructuring moved an `await`, only attribute strings changed. *(release_gates.rs)*
- [x] Browser smoke captures My Page, Home, and the calendar-feed settings page at mobile width and 200% text, in both languages, with numeric right-edge/overflow measurements (not just a pass/fail boolean) added to every scenario via a new `pageOverflowPx`/`viewportWidth` pair in the smoke's page-state collector. All scenarios clear with `pageOverflowPx: 0`. *(scripts/smoke/language-preference.mjs; evidence `.git-exclude/evidence/rfc075-slice3/`)*
- [x] The pre-existing vertical overlap between the fixed bottom nav and page content in full-page (`captureBeyondViewport`) screenshots — first documented in the RFC-072 Slice C entry above — is present again in this slice's My Page and calendar-feed screenshots, over different content than before (My Page's content grew by one line since Slice C's screenshot, from the calendar-feed link RFC-023 added). Confirmed unchanged in kind, not a regression from this slice, and not modified, per the existing precedent.
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, and `smoke:display-name` all still pass, unmodified.

## Admin event surfaces CSS migration gates (RFC-075 Slice 4)

Presentation-only, same non-goals as Slices 1–3. No handler behaviour, form,
route, `data-*`, i18n, or authorization change. **No CSP change** —
`style-src 'unsafe-inline'` stays until inline `style=` reaches zero, which
this slice does not reach. These pages are admin-only and Japanese-only by
documented decision (RFC-072 Slice D) — no English rendering exists for any
of them, so evidence for this slice is Japanese-only, unlike Slices 1–3.

- [x] `admin/events/forms.rs` (the shared form-rendering helper, 20 inline styles) and its four callers `copy.rs`(7), `create.rs`(5), `recreate.rs`(5), `edit.rs`(9), plus `admin/events/summary.rs`(7, called only from `forms.rs`), `attendance.rs`(12), `cancel.rs`(8), `notes.rs`(7), and `occurrence.rs`(6) — 86 inline styles across 10 files — are fully migrated to `cz-*` classes. `admin/events/notes.rs` needed zero new classes: its confirmation layout reuses `event.rs`'s existing member-facing `cz-confirm-*` family (title/body/actions/keep-link/delete-form/delete-button) byte-for-byte. `recreate.rs` also needed zero new classes, reusing `cz-page-main`, `cz-admin-title--snug`, `cz-admin-back-row`, `cz-admin-back-link`, and `cz-admin-submit-button` in full. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] Three coincidental hex-value matches were given their own new tokens rather than reused, per the Slice 3 `cz-home-event-location` precedent (a value match is not a semantic match): `occurrence.rs`'s helper text (`#3A3A3C`, coincidentally equal to `--cz-color-today-fg`) → new `--cz-color-text-tertiary`; its cancel-occurrence button (`#B42318`, coincidentally equal to `--cz-status-not-going-fg`) → new `--cz-color-danger-strong`; `summary.rs`'s schedule-card background (`#FAFAFB`, coincidentally equal to `--cz-color-today-bg`) → new `--cz-color-surface-subtle`. One genuine, non-coincidental reuse of an established pattern: `attendance.rs`'s "Saved" flash message reuses `--cz-status-attended-fg`, the same token `cz-note-flash`/`cz-me-flash` already use for generic success text (Slices 2/3) — not a new "success" concept, so no new token.
- [x] Reuse was checked for exact property equivalence, including omissions, per §7.1. Beyond the two zero-new-class files above, `cz-page-main` was reused directly on every file's `<main>` except `occurrence.rs`, which needed the same padding plus centering — a new `cz-page-main--narrow` modifier (`max-width: 42rem; margin: 0 auto`) was added rather than adjusting `cz-page-main` itself. `cancel.rs`'s confirmation actions were *almost* a match for the existing member-facing `cz-confirm-*` family but genuinely differ (a pre-existing `flex: 1 1 9rem`/`overflow-wrap: anywhere`/`white-space: normal` wrap-hardening `cz-confirm-*` doesn't have) — a parallel `cz-admin-confirm-*` family was added instead of retrofitting the shared one.
- [x] `forms.rs`'s classes were checked against all four callers, not just one (§14's named risk): `copy.rs`, `create.rs`, `recreate.rs`, and `edit.rs` all render `cz-admin-field`/`cz-admin-field-label`/`cz-admin-field-input` identically; only `copy.rs`/`create.rs`/`recreate.rs` render the repeat-fields row (`cz-admin-repeat-*`), which `edit.rs` never calls — no variant needed.
- [x] `LOCALIZATION_EXCEPTIONS`' pinned `ja_count`/`calls_bare_page` values for all 10 files are unchanged from Handoff 030 — proving no Japanese string was touched. §7.2 predicted no other gate would need re-expression; that held: every gate outside the two ratchets and the digest passed unmodified. *(release_gates.rs `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`)*
- [x] Both ratchets lowered and re-measured with the gate's own counting function: inline `style=` 276 → **190** (86 removed, exactly this slice's own total); hardcoded hex-colour literals 223 → **167** (56 removed). *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] Browser smoke captures the create-event form (`forms.rs`'s densest row — the repeat selector), the attendance page, and the cancel-event confirmation at mobile width and 200% text, Japanese only. All three clear with `pageOverflowPx: 0` and a numeric per-row margin (`marginPx`) reported alongside a boolean, per Handoff 028's precedent: all three surfaces measured a consistent 16px clearance against the 390px viewport. `cancel.rs`'s two-button row was confirmed to wrap into full-width stacked rows at 200% (the same flex-wrap technique Handoff 028 established), both buttons still flush against the same 16px margin with their text intact — the fixed bottom nav visually overlapping mid-page in the stitched full-page screenshot is the same pre-existing, previously-documented artifact from Slice 3, not a regression. *(scripts/smoke/rfc075-slice4-admin-event-forms.mjs; evidence `.git-exclude/evidence/rfc075-slice4/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, and `smoke:language` all still pass, unmodified.
- [ ] **Out-of-scope finding, not acted on:** `create.rs`'s "Use a template" link renders hardcoded English text on this otherwise-Japanese-only admin page — pre-existing, not introduced by this slice (confirmed present before this package's changes), and never caught by the localization gate since it's not an `i18n::JA_*` reference. No i18n change is authorized by this handoff; flagging rather than fixing.

## Admin member management CSS migration gates (RFC-075 Slice 5)

Presentation-only, same non-goals as Slices 1–4. No handler behaviour, form,
route, `data-*`, i18n, or authorization change. **No CSP change** —
`style-src 'unsafe-inline'` stays until inline `style=` reaches zero, which
this slice does not reach. Admin-only, Japanese-only surfaces (RFC-072
Slice D) — evidence for this slice is Japanese-only, same as Slice 4.

- [x] `admin/members.rs` (32 inline styles — invite generation/revocation and the member list), `admin/help_signin.rs` (17), `admin/role_transfer.rs` (10), and `admin/member_remove.rs` (8) — 66 inline styles across 4 files — are fully migrated to `cz-*` classes. Zero new tokens were needed: every hex value in these four files (`#EDFAF0`/`#34C759`/`#167A34`) already maps to the existing attended-status "success" palette, which Slice 4 established as a genuine repeated pattern, not a coincidence to guard against. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] **Terminology correction to this handoff's own description**: §7.3 called `admin/members.rs`'s member list "the member table" and named `<th>` scope/header-association as Preservation Contract concerns. It is not an HTML `<table>` — it is a flex `<ul>/<li>` list, the same idiom used elsewhere in this codebase (Home's event list, the invite-codes list on this same page). There is no table markup or `<th>` to preserve; the corresponding risk — dense per-row content overflowing at 200% on a narrow viewport — was tested against the actual markup instead (see the measurement below).
- [x] Reuse checked for exact property equivalence, including omissions, per §7.1. Two files needed very few new classes: `member_remove.rs` reuses `cz-page-main`, `cz-admin-title`/`cz-admin-title--snug`, `cz-admin-confirm-subtitle` and `cz-admin-error-main` (Slice 4) plus `cz-confirm-delete-form`/`cz-confirm-delete-button` (`event.rs`, Slice 2) byte-for-byte, needing only two new classes (`cz-admin-role-actions`, `cz-admin-role-keep-link`) for its non-wrapping confirm row and keep-link — both close to, but not identical to, the existing `cz-confirm-*`/`cz-admin-confirm-*` families (missing the explicit `min-height`/flex-centering those set, and this row never sets `flex-wrap`, unlike Slice 4's `cancel.rs`). `role_transfer.rs` and `help_signin.rs` share that same new pair plus one more (`cz-admin-role-confirm-button`, a blue-background variant of the pattern without the `font-size`/`margin-top` a stacked-button context would need). A shared `cz-admin-reveal-box` was found to serve both `members.rs`'s invite-code reveal and `help_signin.rs`'s relink-code box — their original inline styles listed the same five properties in different orders, which is CSS-equivalent, not merely similar.
- [x] **`help_signin.rs` (§13): class-only.** Every attribute this handoff named as untouchable — `data-copy-code-value`, `data-copy-code-button`/`hidden`/`data-copy-success`/`data-copy-error`, `data-copy-code-status`/`aria-live="polite"`, and the `aria-label` on the code display — is unchanged, byte-for-byte, in the diff; only `style=` became `class=`. Verified by re-running the smoke's real no-JS POST submit and capturing the actual reveal page (not assumed): the relink code, copy button, and status span all render identically in structure, only restyled.
- [x] `LOCALIZATION_EXCEPTIONS`' pinned `ja_count`/`calls_bare_page` values for all 4 files (26/17/10/8) are unchanged — proving no Japanese string was touched. §7.2 predicted no gate re-expression; that held: every gate outside the two ratchets and the digest passed unmodified. *(release_gates.rs `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`)*
- [x] Both ratchets lowered and re-measured with the gate's own counting function: inline `style=` 190 → **124** (66 removed — exactly this slice's own total: `members.rs`(32) + `help_signin.rs`(17) + `role_transfer.rs`(9) + `member_remove.rs`(8); `role_transfer.rs`'s inline-style count of 9 is distinct from its pinned `LOCALIZATION_EXCEPTIONS` `ja_count` of 10, an unrelated count); hardcoded hex-colour literals 167 → **117** (50 removed). *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] **§7.3 member-row measurement, at both scales, not just 200%:** `admin/members.rs`'s member-list row (`cz-admin-member-row`) was stress-tested with a deliberately long, space-free display name (to force `overflow-wrap: anywhere` to do all the work, not just line-wrap at natural word breaks). At 100% the row clears with a 16px margin against the 390px viewport; at 200% it clears with the same 16px margin — the long name wraps onto multiple lines within its own column instead of overflowing horizontally, and the action-links column stays right-aligned and unclipped. `pageOverflowPx: 0` at both scales. The non-wrapping two-button confirm row (`cz-admin-role-actions`, shared by `role_transfer.rs`/`member_remove.rs`/`help_signin.rs`) was also measured at 200% out of caution, since it — unlike Slice 4's `cancel.rs` — sets no `flex-wrap`; it clears with the same 16px margin, since these pages' actual Japanese button labels are short enough not to need it. Nothing overflowed; no table-reflow fix was needed or attempted. *(scripts/smoke/rfc075-slice5-admin-member-management.mjs; evidence `.git-exclude/evidence/rfc075-slice5/`)*
- [x] Browser smoke captures the member list (100% and 200%), the invites page, the promote confirmation, the remove confirmation, and both `help_signin.rs` renders (the confirm page and the synthetic relink-code reveal, reached via the smoke's own real form submit) — all Japanese, all `pageOverflowPx: 0`. The relink code captured is synthetic test data generated by the smoke run itself, never a real one.
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, `smoke:language`, and `smoke:admin-event-forms` all still pass, unmodified.

## Admin tools and onboarding CSS migration gates (RFC-075 Slice 6)

Presentation-only, same non-goals as Slices 1–5. No handler behaviour, form
`action`/`method`, route, `data-*`, i18n, or authorization change. **No CSP
change** — `style-src 'unsafe-inline'` stays until inline `style=` reaches
zero, which this slice does not reach. `join.rs`/`relink.rs` are anonymous,
Japanese-only by correctness (no session, no membership, no locale to
resolve) and remain untouched by any localization gate; the admin tools
(`templates.rs`, `export.rs`) are Japanese-only by RFC-072 Slice D decision.

- [x] `handlers/templates.rs` (25 inline styles), `handlers/export.rs` (8), `handlers/join.rs` (19), `handlers/community_create.rs` (16), and `handlers/relink.rs` (8) — 76 inline styles across 5 files — are fully migrated to `cz-*` classes. **`community_create.rs` gets its single CSS touch here**, as promised when its language was fixed in Handoff 030 — it stays fully localized and absent from `LOCALIZATION_EXCEPTIONS`, at zero bare `i18n::JA_`. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] **Markup read before describing it, this time.** Every file's structure (`<form>`s, one `<ul>/<li>` list in `templates.rs`, no list or table in `export.rs`) was opened and confirmed before any class was assigned, per Slice 5's corrective precedent. No table markup exists anywhere in this slice's five files.
- [x] **This slice had the highest gate density of any so far**: 38 assertions across two test files (`release_gates.rs` and `rfc078_abuse_controls.rs`) name these five files. Predicted risk was collateral damage from `format!`-block restructuring, not gate re-expression — confirmed correct: both test files pass with every assertion unchanged; only attribute strings (`style="…"` → `class="…"`) were touched anywhere in the diff, verified by reading the full diff for each file, not just running the suite. *(release_gates.rs, rfc078_abuse_controls.rs — both green)*
- [x] **`join.rs` (§7.2/§13): class-only, confirmed by diff.** Form `action`/`method`, every field name, the form-token field, and both failure-path branches (`render_join_form`/`render_profile_form`'s `error_html`) are byte-identical to before this slice except `style=` → `class=`. RFC-076's response-shape indistinguishability is untouched: the same markup renders on success and on every rejection reason (format, form replay, abuse-control block, no valid invite), unchanged from before. Re-running `smoke:invite` end-to-end (fresh invite → profile step → signed-in landing → reused-code generic error) confirms the functional path, not just the markup diff.
- [x] Reuse checked for exact property equivalence, including omissions, per §7.4. `join.rs`'s and `relink.rs`'s shared anonymous-shell classes (`cz-anon-main`, `cz-anon-title`, `cz-anon-form`, etc.) preserve the original `font-family:system-ui,sans-serif` value exactly rather than conforming it to `:root`'s own five-fallback stack — a real value, not rounded to a token. Two genuine near-misses, not reused: `relink.rs`'s submit button omits nothing `join.rs`'s doesn't already have, so it reuses `cz-anon-submit-button` plus an additive `cz-anon-submit-button--sized` modifier for its one extra explicit `min-height`; `community_create.rs`'s two labels share a base class (`cz-community-create-label`) with a `--spaced` modifier overriding `margin` (not `margin-bottom`) for the second label's extra top-margin — the shorthand override works because the modifier rule is declared after the base in `app.css`. Zero coincidental-value token reuse was needed or found this slice — every hex maps to an existing generic token.
- [x] `LOCALIZATION_EXCEPTIONS`' pinned `ja_count`/`calls_bare_page` values for the four excluded files (`templates.rs`=12, `export.rs`=7, `join.rs`=18, `relink.rs`=10) are unchanged. `community_create.rs` remains absent from the table with zero bare `i18n::JA_` references, confirmed by direct count. §7.1 predicted no gate re-expression; that held completely.
- [x] Both ratchets lowered and re-measured with the gate's own counting function: inline `style=` 124 → **48** (76 removed, exactly this slice's own total: 25+8+19+16+8); hardcoded hex-colour literals 117 → **70** (47 removed). *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] Browser smoke captures all five required surfaces at mobile width and 200% text, Japanese only: the join form and relink form (anonymous, no session cookie set — the same as a real first-time visitor), the create-community form, the templates list (seeded with one real row, not just the empty state), and the export page. All five clear with `pageOverflowPx: 0`; the join/relink anonymous shell measures a 32px margin (its own `padding: 2rem` shell, unrelated to `cz-page-main`'s 16px) while the three admin-tool pages measure the familiar 16px. *(scripts/smoke/rfc075-slice6-admin-tools-and-onboarding.mjs; evidence `.git-exclude/evidence/rfc075-slice6/`)*
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, `smoke:language`, `smoke:admin-event-forms`, and `smoke:admin-member-management` all still pass, unmodified. `smoke:invite` (not one of the required eight, but the dedicated functional smoke for the security surface this slice touches) also re-run in full and passes.

## The last migration CSS gates (RFC-075 Slice 7)

Presentation-only, same non-goals as Slices 1–6. No route, form, `data-*`,
authorization, or i18n change. **No CSP change** — the CSP does not move
until the terminal slice. After this slice, every remaining `style=` in
`workers/ssr/src` is either dead code (`render/event_card.rs`, 10, swept in
a later package) or the literal text `style=` inside a CSP-justifying
comment in `lib.rs` (1, untouched here) — **11 total**, not zero.

- [x] `render/errors.rs` (15 inline styles — the six fixed error-page shells, all reusing Slice 6's `cz-anon-*` family), `handlers/communities/matrix.rs` (4 — the sticky table shell, see below), `handlers/communities/matrix/detail.rs` (15 — the day-detail panel, confirmed genuinely static), and `handlers/static_files.rs` (3 — the offline page) — 37 inline styles across 4 files — are fully migrated to `cz-*` classes. `render/errors.rs` is excluded from *localization* (no membership, no locale) but was never excluded from this RFC; it needed migrating for the terminal zero, and now is. *(release_gates.rs `inline_style_count_never_increases`)*
- [x] **Correction to this handoff's own §3.3**: it claimed none of this slice's remaining `style=` attributes interpolated a computed value. True for `matrix/detail.rs`'s 15 and for 2 of `matrix.rs`'s 4 (the sticky row header and its link) — **false** for the other 2: the day-column header's background/border (selected vs. not-selected, 2 states) and, more significantly, the body `<td>`'s colour/background, which `cells.rs` computes per cell across **8** distinct attendance states (empty, cancelled, going, not-going, attended, no-answer, breakdown-complete, breakdown-partial). This is exactly the "computed per-status colours" reason RFC-075 originally deferred matrix per-cell work — except `cells.rs` (Slice 2) already proved the fix: `CellSummary`'s `color`/`background` fields were replaced with one `state: &'static str` field, and `matrix.rs` now renders `class="cz-matrix-cell cz-matrix-cell--{state}"` instead of interpolating hex. No test asserted these fields' values before this change (confirmed by reading `matrix/tests.rs` in full), so nothing needed updating beyond the fields themselves. Same rendered colours throughout — an extraction, not a redesign.
- [x] Two of the eight cell states and the row-header's background reuse existing, non-coincidental tokens (`not-going` matches `--cz-status-not-going-fg`; `cancelled`/`no-answer` text matches `--cz-color-text-secondary`; their backgrounds match `--cz-color-surface`/`--cz-color-bg`). The other six are new tokens (`--cz-matrix-cell-empty-fg/bg`, `--cz-matrix-cell-going-fg/bg`, `--cz-matrix-cell-not-going-bg`, `--cz-matrix-cell-attended-fg/bg`, `--cz-matrix-cell-breakdown-partial-fg`) — matrix cells have never shared the app-wide status palette (e.g. cell "going" is `#0A7F43`, not `--cz-status-going-fg`'s `#005BBB`), and inventing that shared palette now would be a visual redesign, not an extraction, so the existing (arguably inconsistent) values were preserved exactly rather than unified.
- [x] **§7.1 sticky verification, by scrolling, not by reading CSS**, at both 100% and 200% text, with a 14-member fixture tall enough to force page-level vertical scroll and a full month of day columns wide enough to force the scroller's own horizontal scroll: horizontal stickiness is fully verified working — the corner cell and each member-row header hold their left position (≤1px drift) through a full horizontal scroll of the matrix scroller, while an ordinary (non-sticky) day-column header moves left by the scroll delta, confirming the right axis was actually exercised. *(scripts/smoke/rfc075-slice7-final-migration.mjs; evidence `.git-exclude/evidence/rfc075-slice7/`)*
- [x] **Finding, not a regression — verified against the pre-migration code via `git stash`**: vertical "stickiness" does not track the page scroll, and never did. `.cz-matrix-scroller` sets only `overflow-x: auto`, but the CSS Overflow spec forces the *computed* value of `overflow-y` away from `visible` whenever the other axis isn't `visible` — confirmed directly (`getComputedStyle` reports `overflow-y: auto` though only `overflow-x` was ever written). That makes the scroller div itself, not the viewport, the sticky containing block for `top: 0`; since the div is never height-constrained, its own `scrollTop` never moves (confirmed directly, stays `0` throughout), so nothing inside it can visually stick to the page as it scrolls. Re-ran the identical scroll-and-measure check against the pre-migration inline-style code (`git stash` back to `97810a9`, rebuilt, measured, restored): the vertical drift matched the post-migration drift exactly, pixel for pixel, at both scales. This slice's extraction changed nothing about this behaviour — it was already this way, and the corner/column-header's *horizontal* pinning (the axis a 31-day-wide table on a 390px viewport actually needs) is unaffected and fully verified above.
- [x] `handlers/static_files.rs`'s offline page reuses `cz-anon-main`/`cz-anon-title`/`cz-anon-subtitle` (Slice 6) rather than minting new classes — verified, not assumed, that this is safe: `sw.js`'s `SHELL_ASSETS` lists `/static/app.css` and `/offline` together, cached via a single atomic `c.addAll(...)` in the install handler, so a cached offline page always has its stylesheet cached alongside it; the fetch handler's own last-resort inline-HTML fallback (used only if the cache itself is unavailable) carries no styles at all and is unaffected either way. Confirmed by reading `sw.js` directly, not by trusting the handoff's claim. *(scripts/smoke/rfc075-slice7-final-migration.mjs `offline-page`)*
- [x] `LOCALIZATION_EXCEPTIONS`' pinned `ja_count`/`calls_bare_page` for `render/errors.rs` (17, unchanged) is unchanged; `matrix.rs`/`matrix/detail.rs`/`static_files.rs` remain outside the table exactly as before (fully localized via `i18n::t`, or in `static_files.rs`'s case, raw literal Japanese never routed through the i18n system at all — unaffected by this gate either way).
- [x] Both ratchets lowered and re-measured with the gate's own counting function: inline `style=` 48 → **11** (37 removed, exactly this slice's own total: 15+4+15+3), matching the handoff's own prediction exactly — composition confirmed as 10 (`render/event_card.rs`, dead) + 1 (`lib.rs`'s CSP-comment literal `style=` text, a false positive in the counter's plain substring match, left untouched per this handoff's explicit scope); hardcoded hex-colour literals 70 → **32**. *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] `app.css`'s content change is recorded by re-pinning the asset-content-hash digest only — no version bump, no cache-key move. *(release_gates.rs `cached_asset_content_matches_pinned_hash`)*
- [x] Every gate green, including RFC-067's matrix contract and RFC-068's CSV export — neither `data-rfc067-matrix-scroller`, `data-export-value`, `data-member-name`, `data-date`, nor any `scope` attribute changed; both dedicated smokes (`smoke:matrix`, `smoke:matrix-csv`) re-run unmodified and pass. No gate's expected substring changed anywhere in either of this repository's two gate test files.
- [x] Browser smoke captures the matrix (scrolled in both axes, both scales, with the sticky proof above), a day-detail view, one error page (`session_expired`, reached genuinely anonymously — a real bug in this package's own evidence script was caught and fixed here: a second "anonymous" browser page had inherited the first page's session cookie, since cookies are scoped to the browser profile, not the CDP target; `Network.clearBrowserCookies` before each anonymous page fixed it), and the offline page — all Japanese, all `pageOverflowPx: 0`.
- [x] Re-running `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, `smoke:language`, `smoke:admin-event-forms`, and `smoke:admin-member-management` all still pass, unmodified.

## Dead-code sweep (Handoff 035)

Not a numbered RFC — bounded cleanup carried since the Slice 2 review, and a
prerequisite for RFC-075's terminal criterion (inline `style=` reaching
zero). No behaviour, route, form, `data-*`, i18n, CSS, version, or CSP
change.

- [x] Three dead render items deleted, each independently re-verified (not trusted from the handoff) before deletion: the `event_card` function (`render/event_card.rs` — zero callers anywhere in its history back to `89eebb7`; `CardDay`, which shares the file, stays, since four live handlers construct it), `status_triplet` (`render/status.rs` — zero references anywhere), and `status_chip` plus its test (`render/status.rs` / `render/tests.rs` — zero production callers, exercised only by its own now-deleted test, the same "dead code with test coverage" shape the handoff named it for). `CardDay`, `ParticipantEntry`, and `status_display` confirmed live and untouched.
- [x] **`render.rs`'s module-wide `#![allow(dead_code)]` is removed.** `render/` is now compiler-guarded: the next unused item in this module fails the build instead of waiting for someone to notice. Two `pub use` re-export lists were corrected as a direct, mechanical consequence — `status_display` (called internally via `super::status::status_display`, never externally via the crate-level re-export) and `placeholder` (zero callers by any path) were dropped from their respective `pub use` lists; the underlying functions are untouched, only their unused crate-level re-export removed.
- [x] **Removing the allow surfaced substantially more than the three authorized deletions** — reported, not silently deleted, per the handoff's own instruction. Two categories: (1) 12 constants in `render/status.rs` (`CZ_STATUS_*_FG`/`BG`/`BORDER` for all four statuses) became dead as a *direct result* of deleting `status_triplet`, their only caller; (2) a further 12 items were *already* dead before this sweep touched anything, merely hidden by the now-removed allow — 8 more constants in `status.rs` (base design tokens and "raw status colors," never referenced anywhere), `render/errors.rs`'s `placeholder()` function, `render/event_card.rs`'s `CardDay.day_date` field (constructed by all four live callers but never read back by any live formatter), and `render/time.rs`'s `format_day_time`/`apply_offset_display`/`parse_utc_display`/`parse_utc_time` — the last two are the same "dead code with test coverage" shape as `status_chip`, but were not named in this handoff's §3, so were not deleted. All **30** items (24 in `status.rs` as detailed above, plus `errors.rs`'s `placeholder()`, `event_card.rs`'s `day_date` field, and the 4 in `time.rs`) got narrow, item-level `#[allow(dead_code)]` annotations with a comment explaining which category each falls into — not a fresh module-wide allow, and not a silent deletion beyond the three authorized items. **Corrected 2026-08-01**: the review-request's own summary line said "24," undercounting by only citing the `status.rs` subset; the review caught this by diffing the item-level allow count (1 → 31) directly rather than trusting the prose.
- [x] `event_card.rs`'s `LOCALIZATION_EXCEPTIONS` row (pinned at exactly 4 bare `i18n::JA_` refs) was **removed**, not re-pinned to 0 — the gate's own stale-entry check would otherwise fail for a row naming a file with nothing left to except. *(release_gates.rs `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`)*
- [x] Both ratchets lowered to their predicted terminal values: inline `style=` 11 → **1** (the `lib.rs` CSP-comment false positive, confirmed by direct read — the terminal CSP slice will resolve this one); hardcoded hex-colour literals 32 → **25** (removed by the `event_card` function deletion; the now-`#[allow]`ed constants in `status.rs` still contain hex literals in their own definitions and are correctly still counted, since the ratchet counts source text, not reachability). `app.css` was not touched (confirmed via `git diff --stat`); no digest re-pin needed. *(release_gates.rs `inline_style_count_never_increases`, `hardcoded_hex_color_count_never_increases`)*
- [x] Full suite green with the test-count delta explained: the ssr crate's native test count dropped by exactly **1**, from 179 to 178 — `status_chip_follows_locale`, the only `#[test]` function touched anywhere in the diff (confirmed by grepping the full diff for `#[test]`/`fn` additions and removals, not inferred from the count alone). `cargo clippy --workspace --all-targets -- -D warnings` is clean with the allow removed, now doing real enforcement work on `render/`.
- [x] **§7.5 systemic measurement, not a fix**: 14 module-wide `#![allow(dead_code)]` directives remain outside `render/` (the handoff's own count of "16 total, 15 remaining" was off by one — `form_token.rs`'s is an item-level allow, not module-wide, so the true count was 15 total, 14 remaining after this sweep). Measured each in isolation (temporarily disabled one at a time, built, counted, restored — never left disabled): `db.rs` hides **9** dead items (a type alias, two unused DB functions, several never-read struct fields), `abuse_limiter.rs` hides **5** (a never-constructed struct, two unused functions, an unused enum), `errors.rs` hides **2**, `crypto.rs` and `authz.rs` each hide **1**, and the ten `db/*.rs` submodules (`community.rs`, `membership.rs`, `attendance.rs`, `session.rs`, `event_note.rs`, `event.rs`, `event_write.rs`, `relink.rs`, `event_series.rs`) each hide **0** — their own allows are currently protecting nothing. Rough total: **~18** dead items across these 14 files, concentrated in two of them. Measurement only; nothing outside this sweep's three authorized items was deleted or otherwise modified.
- [x] All ten smokes pass with zero rendered change anywhere — `smoke:language`, `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, `smoke:display-name`, `smoke:admin-event-forms`, `smoke:admin-member-management`, `smoke:admin-tools-onboarding`, `smoke:final-migration`, all re-run unmodified.

## RFC-079 audit integrity and redaction

- [x] Production audit actions use the closed 23 Class A + 2 Class B + 1 Class C inventory; no compatibility writer or arbitrary action/JSON entry point remains. *(audit.rs + repository-wide release gate)*
- [x] Class A required business writes and audit evidence use reviewed D1 batches; local forced-audit-failure proofs cover simple, multi-write, one-winner, attendance, occurrence, and token paths. *(local audit atomicity/assertion proofs)*
- [x] Community JSON and matrix-export acknowledgement return generic `503` without disclosure when audit storage fails. *(audit-boundary proof + source ordering gates)*
- [x] Logout is the sole secondary-audit exception: revocation is awaited first, the audit carries no session/subject identifier, and audit failure still clears the cookie. *(audit-boundary proof + ownership gate)*
- [x] Production Rust has one audit INSERT owner (`audit.rs`), no ignored/background required audit, and no raw-ID/Debug error logging path. *(Package 7 removal gates)*
- [x] Migration 0010 locally preserves legacy core chronology, resets every legacy metadata value to `{}`, enforces closed schema bounds, and creates the reviewed assertion table. *(migration proof; local only)*
- [x] Class A construction/storage/cardinality failures centrally emit exactly one bounded `audit.required_batch_failed`; compiled-SSR route proofs cover invite generation, community creation primary-action ownership, join assertion rollback, and a post-trigger success control on disposable local D1. *(local correction architecture-reviewed, owner-accepted, and committed at `71e3ebd`)*
- [~] Packages 0A–8 and the Class A telemetry correction are locally reviewed and committed, but RFC-050 exact-candidate hosted rollback, disclosure, concurrency, migration, and teardown evidence remain open.
- [~] `audit.required_batch_failed`, `audit.pre_disclosure_failed`, and `audit.secondary_write_failed` delivery through an owner-approved persistent sink, with retention/access and canary retrieval observed, remains open. `wrangler tail` is not sufficient.
- [ ] No critical open security issues. *(final security review before go-live)*

## English leak fix and gate (Handoff 036)

Not a numbered RFC — a bounded correction (six named English leaks, five of
them `aria-label` values, so a screen-reader defect, not just cosmetic page
text) plus a gate rebuild. No CSS, version, or CSP change; no admin-surface
localization beyond the two Japanese-only fixes named below; no behaviour,
route, form, `data-*`, or authorization change; existing i18n values
(`LANGUAGE_OPTION_JA`/`_EN`, etc.) untouched.

- [x] Three member-facing leaks fixed with new `Localized` pairs: `NAV_MAIN_ARIA_LABEL` (the bottom nav's `aria-label`, previously the bare literal `"Main"` on every page), and `CALENDAR_MONTH_NAV_ARIA_LABEL`/`CALENDAR_VIEW_NAV_ARIA_LABEL` (previously `"Calendar month"`/`"Calendar view"`, four call sites across `render/nav.rs`, `handlers/communities/calendar.rs` (×2), and `handlers/communities/matrix.rs` (×2)). *(packages/contracts/src/i18n/general.rs, calendar.rs; render/nav.rs; handlers/communities/calendar.rs, matrix.rs)*
- [x] Two admin-only leaks fixed with bare `JA_*` constants, not `Localized` pairs — these RFC-072 Slice D pages are Japanese-only by documented decision, and a pair would incorrectly imply the page has been localized: `admin/events/create.rs`'s `"Use a template"` link text, and `admin/events/attendance.rs`'s `aria-label="Attendance for {name}"`, converted to a `{}`-positional Japanese format template (`substitute_positional` pattern, RFC-072 Slice C) via a local copy of the helper (the original is `pub(super)` to the `matrix` module tree and not reachable from `admin/events`). `LOCALIZATION_EXCEPTIONS` re-pinned for both files (`create.rs` 6→7, `attendance.rs` 12→13) — a deliberate, reviewed increase, confirmed by direct `i18n::JA_` occurrence count, not an approximation. *(packages/contracts/src/i18n/events.rs; workers/ssr/src/handlers/admin/events/create.rs, attendance.rs; packages/contracts/tests/release_gates.rs)*
- [x] The hand-maintained `no_known_english_ui_leaks_in_rendered_text` gate (a fixed forbidden-string list checked against eight hand-picked files) is replaced with a default-fail walk of every non-test file under `handlers/` and `render/`, mirroring `LOCALIZATION_EXCEPTIONS`'s own pattern (pinned exact count, written reason, stale-entry assertion). **User-visible attributes (`aria-label`, `title`, `placeholder`, `alt`) are now covered, not just element text** — the shape of five of this handoff's six leaks, which the old gate's vocabulary could never have matched even if it had scanned the right files. Rust `\`-continued multi-line string literals are collapsed before scanning, closing the exact blind spot that hid all six leaks from a naive single-line search. `{interpolation}` placeholders, numbers, symbols, and non-ASCII (Japanese) text are never flagged. Pre-seeded with the one known-correct exception, the "zinnias" brand name (`render/shell.rs`, `handlers/static_files.rs`, 1 each) — confirmed `LANGUAGE_OPTION_EN` (`packages/contracts/src/i18n/me.rs`) sits outside the walked tree and cannot trigger it. Proven two ways: (1) mutation — reintroduced the bare `"Main"` `aria-label` into `render/nav.rs`, observed the gate fail with the expected finding, restored the file (`cmp`-verified byte-identical), observed the gate pass again; (2) absent-from-table — see the disclosed finding below, a real leak with no exception row, the exact failure shape the old eight-file gate could never produce. *(packages/contracts/tests/release_gates.rs `rfc049_no_english_leaks_in_rendered_text_or_attributes`)*
- [x] **A seventh leak, found before the owner's copy was supplied, is now fixed too**: the new gate also surfaced `handlers/export.rs`'s `"{events} events · {members} active members"` (same admin-only RFC-072 Slice D class as the two fixes above) — a genuine leak this handoff did not name, with no owner-approved Japanese copy at the time. Escalated rather than drafted (per §15); the review supplied `JA_ADMIN_EXPORT_SUMMARY_COUNTS` (`"予定{events}件 · 有効メンバー{members}人"`, substituted by name — the constant is a runtime `&str`, not a compile-time literal, so it can't sit inside the outer `format!`'s own literal). `LOCALIZATION_EXCEPTIONS` re-pinned `export.rs` 7→8, confirmed by direct `i18n::JA_` occurrence count. A placeholder-order test (`admin_export_summary_counts_has_both_named_placeholders_in_order`) pins both `{events}` and `{members}` present, in that order. *(packages/contracts/src/i18n/export.rs; workers/ssr/src/handlers/export.rs; packages/contracts/src/i18n/tests.rs; packages/contracts/tests/release_gates.rs)*
- [x] New tests: a locale-follows test per new pair (`bottom_nav_localized_labels_switch_with_locale` extended; `calendar_month_nav_aria_label_follows_locale`, `matrix_render_month_nav_aria_label_follows_locale`, `render_mode_tabs_aria_label_follows_locale` added), a placeholder-count-stability test for the admin attendance format template (`admin_attend_member_aria_label_has_one_placeholder`), a placeholder-order test for the admin export summary counts (above), and a screen-reader-oriented `smoke:language` assertion proving the bottom nav's **rendered** `aria-label` (not just the source i18n constant) is Japanese under `Locale::Ja` and English under `Locale::En`. *(render/tests.rs; handlers/communities/tests.rs; packages/contracts/src/i18n/tests.rs; scripts/smoke/language-preference.mjs)*
- [x] Full suite green, no disclosed exceptions: `cargo test --workspace` (including `rfc049_no_english_leaks_in_rendered_text_or_attributes`, now passing with zero findings anywhere under `handlers/`/`render/`), clippy (`-D warnings`), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors; `bun run build` succeeds. All ten smokes pass unmodified: `smoke:language`, `smoke:calendar-views`, `smoke:matrix`, `smoke:matrix-csv`, `smoke:community-switch`, `smoke:display-name`, `smoke:admin-event-forms`, `smoke:admin-member-management`, `smoke:admin-tools-onboarding`, `smoke:final-migration`. *(evidence `.git-exclude/evidence/english-leak-fix/`)*
- [~] **Carried, not fixed here** (per review): a wider class of raw-English flash-message literals (`flash=saved`, `flash=Title+required`, etc.) leaks into rendered `<p role="status">` text across five files, including member-facing Event Detail (`event.rs`) — a data-flow leak this gate cannot see by construction, since the source-level template is a bare `{}` placeholder. Scoped as its own package. The orphaned, unused `TEMPLATES_USE_LINK` i18n pair (mismatched Japanese wording vs. the constant actually used for `create.rs`'s fix) is carried with it.

## Flash messages: codes, not prose (Handoff 037)

Not a numbered RFC — closes the data-flow leak class Handoff 036's gate
could not see by construction: raw-English `?flash=` query values echoed
verbatim into rendered text, three of eight sites on member-facing Event
Detail. No CSS, version, or CSP change; `me.rs` and `calendar.rs` (already
correct) untouched; no behaviour/route/form/`data-*`/authorization change —
every redirect keeps its target, only the query value changed.

- [x] All eight `?flash=` sites converted from raw-English prose to lowercase snake_case codes, matched (never echoed) through a per-surface mapper — the pre-existing `calendar_flash_message` pattern extended to `event.rs` (`note_flash_message`, member-facing, `Localized` pairs `NOTE_SAVED_FLASH`/`NOTE_HIDDEN_FLASH`) and three admin-only surfaces (`templates_flash_message`, `attendance_flash_message`, `invites_flash_message`, bare `JA_*` constants — RFC-072 Slice D, no `en` counterpart to pair against). The split is by where the flash **renders**, not which handler redirects: `admin/events/notes.rs` is an admin action but redirects to Event Detail, so its code (`note_hidden`) resolves through the member-facing pair, not a bare Japanese constant. `LOCALIZATION_EXCEPTIONS` re-pinned for the three admin files (`templates.rs` 12→15, `admin/events/attendance.rs` 13→14, `admin/members.rs` 26→28), confirmed by direct `i18n::JA_` occurrence count. *(packages/contracts/src/i18n/notes.rs, templates.rs, events.rs, admin.rs; workers/ssr/src/handlers/event.rs, templates.rs, admin/events/attendance.rs, admin/events/notes.rs, admin/members.rs; packages/contracts/tests/release_gates.rs)*
- [x] A new default-fail gate asserts every `?flash=` value in the tree is a lowercase snake_case code — never containing an uppercase letter, `+`, or `%20` — closing the class at the point a redirect is written, not at the point it renders. Needed zero exceptions, as predicted; `FLASH_CODE_EXCEPTIONS` is empty. Line-comment text is stripped before scanning (the first draft's naive value-extraction matched inside the gate's own doc comment describing this defect — a self-referential trap, the second gate here to hit it). Proven with a mutation proof: reintroduced `?flash=Note+removed`, observed the gate fail, restored the file (`cmp`-verified byte-identical), observed the gate pass again. *(packages/contracts/tests/release_gates.rs `rfc072_flash_query_values_are_lowercase_snake_case_codes_not_prose`)*
- [x] Unknown codes render no flash element at all — every mapper returns `None` for an unrecognized code (never falling back to echoing it), matched by the same `.map(...).unwrap_or_default()` shape `calendar_flash_message` established. Escaping is unchanged at every render site (`escape_html`/`render::escape_html` still wraps the resolved message everywhere); only what gets passed in changed, from the raw query value to a matched, compile-time-known string.
- [x] **A rendered assertion, not just a mapper unit test, is what proves RFC-072 acceptance criterion 9 holds again**: `smoke:language` now submits the real note-save form on Event Detail (a genuine `requestSubmit()`) in both locales and reads the rendered `.cz-note-flash` text back — メモを保存しました。 under `ja`, "Note saved." under `en`. A member saving a note now sees their own language, not the English word "saved" regardless of locale. *(scripts/smoke/language-preference.mjs)*
- [x] `TEMPLATES_USE_LINK` (`EN_TEMPLATES_USE_LINK`/`JA_TEMPLATES_USE_LINK`, zero production usages, carried from the Handoff 036 review) is deleted, along with its `en_ja_parity` list entry. *(packages/contracts/src/i18n/templates.rs, tests.rs)*
- [x] Full suite green, **no disclosed exceptions**: `cargo test --workspace` (513 passed across every crate), clippy (`-D warnings`), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors; `bun run build` succeeds. All ten smokes pass, `smoke:language` 18/18. *(evidence `.git-exclude/evidence/flash-message-localization/`)*
- [~] **Carried to the terminal CSP slice**: two orphaned pairs now in the same shape `TEMPLATES_USE_LINK` was — `ADMIN_INVITES_REVOKED` (orphaned *by* this change: `admin/members.rs` previously matched, not echoed, this value against it; switching to the new `invite_revoked` code left it dead) and `NOTE_SAVED` (pre-existing orphan, unrelated to this package, divergent wording from `NOTE_SAVED_FLASH`). Also carried: applying `strip_line_comments` (new in this package) to `count_inline_styles`, which still lacks comment stripping and is why the inline-style ratchet cannot reach zero without rewording `lib.rs`'s CSP comment.

## RFC-075 terminal slice: `style-src` drops `'unsafe-inline'` (Handoff 038)

**This is the first package permitted to claim a CSP improvement** — every
prior RFC-075 slice was explicitly forbidden from implying one, since the
directive itself hadn't moved yet. Stated plainly: **`style-src` no longer
permits inline styles.** `Content-Security-Policy`'s `style-src` directive
changed from `'self' 'unsafe-inline'` to `'self'` — the one token this
package removes. No other directive changed; no CSS, version, or i18n
wording change; no behaviour, route, form, `data-*`, or authorization
change. Framed per §11 as a strict directive **restored**, not a
vulnerability **fixed** — output was escaped throughout before this, so
`'unsafe-inline'` was a weakened mitigation layer, never an active hole.

- [x] **Violation capture was proven to fire before the header changed, not assumed to work.** Nothing in the smoke suite previously subscribed to console or CSP-violation events — a violation would have logged to console, silently dropped the style, and passed every test. A shared helper (`scripts/lib/csp-violation-capture.mjs`) now subscribes on two independent channels (the CDP `Log` domain, which surfaces the browser's own CSP violation message, and a page-side `securitypolicyviolation` listener injected via `Page.addScriptToEvaluateOnNewDocument` so it survives every navigation) and is wired into all ten required smokes' `newPage()`. Proof: temporarily applied the stricter header and injected a real inline `style=` into a rendered page, observed the capture report the exact browser violation message, then reverted the injected style (`cmp`-verified byte-identical restore) — the header change itself was kept, since that step **was** the real fix being proven safe. *(evidence `.git-exclude/evidence/rfc075-terminal/capture-proof-*`)*
- [x] **All three of `app.js`'s CSSOM style writes exercised under the real stricter header, zero violations.** `ta.style.borderColor` (typing 201 characters into the Event Detail note field, `smoke:language`) — the computed style was read back afterward to confirm the browser actually applied `rgb(255, 59, 48)`, not that CSP silently dropped it while the page kept rendering. `fallbackCopyText`'s `ta.style.position`/`top` and the copy-result `status.style.color` (clicking the one-time code copy button on help-signin, `smoke:admin-member-management`) — reached because `navigator.clipboard.writeText` is unavailable to a script-triggered click in headless Chromium, exercising the fallback path. None were blocked: CSP `style-src` governs `<style>` elements and `style=` markup attributes, not script-driven CSSOM property assignment, exactly as §3 predicted — confirmed with a browser, not assumed from the spec.
- [x] `count_inline_styles` now strips comments first (`strip_line_comments`, built for the flash gate) before counting — it previously treated `lib.rs`'s own CSP comment, which mentioned `style=` while explaining the directive, as a live inline style, which is why the count sat at 1 instead of the true 0. The ratchet (`INLINE_STYLE_RATCHET`, "never increases") is now a hard assertion (`inline_style_count_is_zero`, "must be zero") — a reintroduced inline style is a CSP regression, not an incomplete migration. *(packages/contracts/tests/release_gates.rs)*
- [x] A new gate, `style_src_has_no_unsafe_inline`, asserts `style-src` contains `'self'` and does not contain `'unsafe-inline'` anywhere in `lib.rs`. **Comments are stripped before checking** — the gate's own first draft failed against its own doc comment (which mentioned `'unsafe-inline'` while explaining what the gate checks), the third instance of this project's gates being tripped by text *about* the pattern they hunt, not just the pattern itself. Proven with a mutation proof: reintroduced `'unsafe-inline'` into the header, observed the gate fail with the exact violating text named, restored the file (`cmp`-verified byte-identical), observed the gate pass again. *(packages/contracts/tests/release_gates.rs)*
- [x] Four dead `render/time.rs` functions deleted — `format_day_time`, `apply_offset_display` (zero callers anywhere, production or test), and `parse_utc_display`/`parse_utc_time` (dead in production, exercised only by their own direct tests in `render/tests.rs` — the same "dead code with test coverage" shape as `status_chip`, Handoff 035). Each independently re-verified as caller-free before deletion, not trusted from the handoff. Their narrow `#[allow(dead_code)]` annotations and direct tests (`parse_utc_time_basic`, `parse_utc_display_uses_ja_format`) went with them; `clippy -D warnings` stays clean with no new allow needed.
- [x] Two orphaned pairs deleted, both carried from the Handoff 037 review: `EN_ADMIN_INVITES_REVOKED`/`JA_ADMIN_INVITES_REVOKED` (`i18n/admin.rs`, orphaned *by* the flash package) and `EN_NOTE_SAVED`/`JA_NOTE_SAVED` (`i18n/notes.rs`, orphaned before it) — plus their `en_ja_parity` list entries. `LOCALIZATION_EXCEPTIONS` re-pinned for `admin/members.rs` (28→27: a stale doc-comment reference to the deleted constant was also removed).
- [x] Full suite green, test-count delta named: ssr unit tests **189 → 187** (the two deleted `time.rs` tests, exactly); `release_gates` **88 → 89** (the new `style_src_has_no_unsafe_inline` gate; `inline_style_count_never_increases` renamed to `inline_style_count_is_zero`, not removed). `cargo test --workspace`, clippy (`-D warnings`), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors; `bun run build` succeeds. All ten smokes pass with **zero CSP violations reported on every one**. *(evidence `.git-exclude/evidence/rfc075-terminal/`)*
- [x] **RFC-075 closed** — this package satisfies its terminal criterion. RFC lifecycle movement (moving it to `rfcs/done/`) is the reviewing architect's, not this package's, per §16.

## RFC-075 carried cleanup (Handoff 040)

Not a numbered RFC — three mechanical items deliberately deferred out of
RFC-075 so its migration packages wouldn't absorb unrelated work. No
behaviour, route, form, `data-*`, i18n, CSS value, version, or CSP change.

- [x] Nine module-wide `#![allow(dead_code)]` directives deleted from `workers/ssr/src/db/{attendance,community,event_note,event,event_series,event_write,membership,relink,session}.rs`, each re-measured independently before deletion (comment out, build, count warnings, restore) rather than trusting the prior day's measurement — all nine still guard exactly zero items. The measurement technique itself was validated against `db.rs` (out of scope, known to guard 9 real items) as a control, confirming a real warning would have been caught if one existed. The deletions were correct and the zero-warning measurement was correct. **Corrected by Handoff 044**: the sentence that followed here — "`clippy -D warnings` is clean on all nine with the allows gone — the compiler is now the guard" — was false. `db.rs`'s own `#![allow(dead_code)]` is an inner attribute and therefore already covers its entire module tree, submodules included, so it was suppressing warnings under all nine of these files the whole time; removing the nine redundant per-file allows changed no enforcement, because the module-wide allow one level up was never touched. The compiler is not the guard for these nine modules — `db.rs`'s allow is, and it stayed in place (correctly: it guards real items). Handoff 044's audit measured 31 of its 40 dead-code items living under `db/` as direct confirmation.
- [x] `cz-admin-field-input` renamed to `cz-field-input` across all seven call sites (`admin/events/forms.rs` ×2, `templates.rs` ×3, `join.rs` ×2) plus the `app.css` rule — the class was in use on `join.rs`, the anonymous unauthenticated invite-redemption page, so an "admin" name was actively misleading and risked a future admin-scoped restyle silently reaching the one page a surprise is least welcome. Rename only, confirmed by diff: no CSS declaration and no other markup attribute changed at any of the eight sites.
- [x] Both copies of `css_rule_body` (`release_gates.rs`, `token_and_color_regression.rs` — duplicated because integration test binaries share no code) now tolerate any run of whitespace between a selector and its opening brace, rather than requiring the exact literal `"{selector} {{"`. Proven with both gates that actually read a rule (`calendar_overview_contract_is_explicit` via `.cz-calendar-day--today`; `status_going_fg_passes_wcag_aa` via `.cz-status-text--going`): temporarily reformatted both rules with aligned extra whitespace before the brace in `app.css`, confirmed both gates still passed, restored the file (`cmp`-verified byte-identical), confirmed both gates still pass on the original formatting too.
- [x] `RELEASE_CACHE_ASSET_CONTENT_HASH` re-pinned once, from the gate's own failure message, after the `cz-field-input` rename changed `app.css`'s content — no version bump; the `0.61.0` tag had just been cut and this is mid-cycle.
- [x] Full suite green, test count unchanged (this package adds and removes no tests): `cargo test --workspace`, clippy (`-D warnings`, now doing real enforcement on nine more modules), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors; `bun run build` succeeds. All ten smokes pass with no rendered change and zero CSP violations reported on every one. *(evidence `.git-exclude/evidence/rfc075-carried-cleanup/`)*

## The admin-class leak, a gate for it, and two misinstructing comments (Handoff 041)

Not a numbered RFC — three corrections carried out of the Handoff 040
review, all to that package's own prior work, not to the implementer's. No
behaviour, route, form, `data-*`, i18n, CSS value, version, or CSP change.

- [x] `cz-admin-plain-link` renamed to `cz-plain-link` across all four call sites (`join.rs`, `admin/role_transfer.rs`, `admin/help_signin.rs` ×2) plus the `app.css` rule — the identical shape as Handoff 040's `cz-admin-field-input`/`cz-field-input`, found by sweeping every `cz-admin-*` class after that rename rather than assuming it was the only one. Rename only, confirmed by reading every diff hunk: no CSS declaration and no other markup attribute changed at any of the five sites.
- [x] A new default-fail gate, `rfc041_admin_named_classes_stay_inside_the_admin_directory_or_are_excepted`, asserts every `cz-admin-*` class referenced from a file outside `handlers/admin/` is named in a pinned exception list with a written reason — closing the class of leak hand-enumeration missed twice. **What the gate proves, stated explicitly because the two differ**: the property that matters is "an admin-named class is rendered on a non-admin *page*" (page-level); the gate's proxy is "referenced from a file outside `handlers/admin/`" (file-level). `templates.rs`/`export.rs` are themselves admin-only surfaces the proxy flags anyway, hence their six classes are exceptions, not renames; a class referenced only from a shared `render/` helper would be invisible to the page-level question but would still trip this file-level proxy, correctly, since a helper's callers cannot be enumerated by reading the helper alone. Proven in both directions: (1) a temporary `cz-admin-*` reference added to a member page (`home.rs`, not `join.rs`) made the gate fail, removed and restored `cmp`-verified byte-identical, gate passes again; (2) a temporary bogus exception row made the gate object to the stale entry, removed and restored `cmp`-verified byte-identical, gate passes again.
- [x] **The exception list was independently re-derived, not copied from the handoff, and it disagreed with the handoff by one row.** The handoff's own seven-row list included `cz-admin-field` for `templates.rs`; direct grep confirmed `templates.rs` only ever uses `cz-admin-field-label`, and bare `cz-admin-field` exists solely in `handlers/admin/events/forms.rs` (inside the admin directory, needing no exception). The pinned list has six rows. Flagged to the architect rather than resolved unilaterally, per the handoff's own stated purpose for this step: "the whole point of this item is that my enumeration is not trustworthy input."
- [x] The digest gate's comment corrected: it previously instructed re-pinning "in the same commit as the version bump," which Handoff 040 had already correctly contradicted (re-pinning mid-cycle with no version bump). Now states the operative rule — re-pin whenever a cached asset's content changes, in the commit that changes it; the cache key and version move at release, not per package. The `v0.59.0`→`v0.60.0` drift narrative above it is untouched, confirmed by diff.
- [x] Both `css_rule_body` doc comments corrected: they claimed the whitespace-tolerant version has "the same narrowness" as the old exact-string version. It does not — the old version, on a near-miss, let `find` walk on to the next occurrence of the full literal; the new version takes the *first* occurrence of the bare selector and panics if it isn't followed by whitespace-then-`{`, without searching further. Stated why that's acceptable (a loud panic naming the selector, never a silently wrong rule body) and confirmed all six selectors these two files read occur exactly once in `app.css` today. Comment-only; the function bodies were confirmed byte-identical to each other both before and after, via `diff` on the extracted function text.
- [x] `RELEASE_CACHE_ASSET_CONTENT_HASH` re-pinned once, from the gate's own failure message, after the `cz-plain-link` rename changed `app.css`'s content — done after the comment correction above, so the corrected instruction was the one actually followed. No version bump; `0.61.0` was just cut and this is mid-cycle.
- [x] Full suite green, test count moved by exactly +1 (the new §7.4 gate, nothing else): `cargo test --workspace`, clippy (`-D warnings`), fmt, wasm check, and `mdbook build docs` all pass clean; `git diff --check` reports no whitespace errors; `bun run build` succeeds. All ten smokes pass with no rendered change and zero CSP violations reported on every one. *(evidence `.git-exclude/evidence/handoff041-admin-class-leak/`)*

## The language smoke's expired date fixture (Handoff 042)

Not a numbered RFC — a test-fixture repair. `smoke:language`'s
`home-renders-english` check was proven, during the Handoff 041 review, to be
a pre-existing, non-regression failure: the fixture seeded a real calendar
instant (`2026-08-03T00:00:00Z`), and Home lists **upcoming** events
(`starts_at_utc >= now`) against the real clock — once that instant passed,
the event fell off Home and the check went red, with no package change
involved. No production code, route, form, or i18n wording changed.

- [x] `scripts/smoke/language-preference.mjs`'s fixture now seeds the visible
      event at **run time + 3 days, 03:00 UTC**, never a fixed calendar
      instant — it cannot expire the way the old pin did. +3 days keeps it
      comfortably upcoming for the whole run (the old fixture failed by 13
      minutes once real time caught up to it). 03:00 UTC is 12:00 JST the
      same day, so the UTC calendar date and the JST calendar date this app
      renders in are identical for this instant — midnight UTC, the old
      fixture's choice, is exactly what made them diverge.
- [x] Every date-derived expectation (the English date label on Home,
      Calendar list, and Event Detail; both month headers, English and
      Japanese; all three `month=` URL parameters; the all-numeric-date
      negative check) is now computed from that same seeded date, built from
      literal weekday/month-abbreviation arrays and `Date.UTC`/`getUTCDay` —
      not `Intl.DateTimeFormat` (host-ICU-dependent, not this project's
      format decision) and not read back from the rendered page. Agreement
      with `packages/contracts/src/tz.rs`'s `date_label_en` (a differently
      implemented, Zeller's-congruence weekday computation) means something
      precisely because the two are independent.
- [x] A precondition assertion (`assertFixtureStillUpcoming`) runs before
      seeding and fails loudly, naming the fixture and the expired date, if
      the seeded event is ever not in the future relative to the run. With
      the relative seed in place this should never fire — it exists so a
      future re-pin back to a literal date is self-describing instead of an
      opaque content mismatch. It guards this one file only.
- [x] **Two-run proof, with a discrepancy flagged to the architect rather
      than resolved unilaterally.** Two independent invocations of the real
      smoke, both 20/20: `generatedAt` `2026-08-03T02:05:26Z` and
      `2026-08-03T02:28:10Z`, both deriving `Thu, 6 Aug` (today + 3 days) on
      Home. Because the seed is quantized to whole UTC calendar days by
      design (§7.1's fixed 03:00 UTC), any two runs on the same UTC day
      necessarily derive the identical date — no faketime/libfaketime was
      available and the system clock was deliberately not touched (invasive,
      affects other processes and TLS validation), and a real day-boundary
      wait was ~22 hours away. In its place: a standalone scratch proof
      (`.git-exclude/tmp/handoff042-proofs/boundary-derivation-proof.mjs`,
      not shipped) exercised the identical derivation formula against three
      contrived boundary instants — a month rollover (Aug→Sep 2026), a year
      rollover (Dec 2026→Jan 2027), and a leap-year month rollover (Feb→Mar
      2028) — and every one of the nine derived values matched a scratch
      `cargo test` call into `tz.rs`'s real `date_label_en` exactly. The
      scratch Rust test file was deleted before commit; nothing under
      `packages/contracts/` changed.
- [x] **Why the other seventeen smokes were left alone.** A hardcoded past
      date is harmless by itself — most of the eighteen smokes that hardcode
      a `2026-MM-DD` literal pin a date already in the past and pass fine. It
      becomes a time bomb only when an assertion *also* depends on the
      page's `>= now` upcoming-events filter. Cross-referencing every smoke
      that visits `/home` against every smoke asserting a date label leaves
      exactly one file: this one. `rfc075-slice7-final-migration.mjs` seeds
      the same kind of date and visits Home too, but asserts no date text
      there, so it was never exposed to the bug.
- [x] Full suite green, test count unchanged at **513** (no Rust touched):
      `cargo test --workspace`, clippy (`-D warnings`), fmt, wasm check, and
      `mdbook build docs` all pass clean; `git diff --check` reports no
      whitespace errors; `bun run build` succeeds. The digest gate
      (`cached_asset_content_matches_pinned_hash`) passed **without a
      re-pin** — nothing under `workers/ssr/static/` or `render/shell.rs`
      changed. The other nine required smokes all pass with zero CSP
      violations reported on every one. *(evidence
      `.git-exclude/evidence/rfc072/`,
      `.git-exclude/tmp/handoff042-proofs/`)*

## Realign the `worker` crate with `worker-build` (Handoff 043)

Not a numbered RFC — one lockfile update, touching a release artifact, so the
owner authorized it explicitly. No `Cargo.toml` edit, no exact pin, no
version bump, no production source, migration, or CSP change.

- [x] **What broke.** `bun run build` failed with
      `error: unexpected argument '--force-enable-abort-handler' found`. The
      host's globally installed `worker-build` binary updated itself to
      **0.8.5** on 2026-08-03, which now passes
      `--force-enable-abort-handler` to the `wasm-bindgen` CLI it fetches — a
      flag the CLI matching this repo's then-pinned `wasm-bindgen 0.2.123`
      does not accept. Proven, during the Handoff 042 review, not caused by
      any repo change: stashed every working-tree change and rebuilt on
      committed `4c7c2b9`, identical failure. `Cargo.lock` had not moved
      since the `0.61.0` release commit `4764c2a`.
- [x] **Why the first two diagnoses published were both wrong** — recorded
      here because the dead end fails silently and looks like a no-op, so
      the next person to hit this needs to be steered around it, not just
      told the fix. First diagnosis: pin `worker-build` back — impossible to
      express in this repo at all, since it is a globally installed binary,
      not a dependency; the breakage would recur on any machine with a
      current one. Second diagnosis: move `wasm-bindgen` forward alone —
      does nothing, because `js-sys 0.3.100` carries
      `wasm-bindgen = "=0.2.123"` as an **upstream exact pin** that isn't
      ours to move; `cargo update -p wasm-bindgen` locks 0 packages and
      exits clean, which is what made it look like a no-op rather than a
      failure. The actual mismatch was `worker` 0.8.4 (library) against
      `worker-build` 0.8.5 (tool) — they ship in lockstep, and bringing the
      library forward releases the upstream pin along with everything else.
- [x] **The fix, one command**: `cargo update -p worker`. Ten packages
      moved: `worker` 0.8.4→0.8.5, `worker-macros` 0.8.4→0.8.5, `worker-sys`
      0.8.4→0.8.5, `js-sys` 0.3.100→0.3.104, `web-sys` 0.3.100→0.3.104,
      `wasm-bindgen` 0.2.123→0.2.127, and the `wasm-bindgen-futures`/
      `-macro`/`-macro-support`/`-shared` family alongside it. `worker`
      resolved to exactly 0.8.5, as expected. `Cargo.toml:20`'s
      `worker = { version = "0.8", ... }` and
      `workers/ssr/Cargo.toml:32`'s `wasm-bindgen = "0.2"` were already the
      semver ranges house style requires and did not need to change — only
      `Cargo.lock` moved, confirmed by `git diff --stat`.
- [x] `bun run build` now succeeds; bundle size **28.4kb**, up from 28.3kb —
      expected from newer generated glue, not a regression.
- [x] Full suite green, test count unchanged at **513**: `cargo test
      --workspace`, clippy (`-D warnings` — no new lint from the regenerated
      bindings), fmt, wasm check, and `mdbook build docs` all pass clean;
      `git diff --check` reports no whitespace errors. The digest gate
      (`cached_asset_content_matches_pinned_hash`) passed **without a
      re-pin** — nothing under `workers/ssr/static/` or `render/shell.rs`
      changed, consistent with this being a toolchain-only update. All ten
      smokes pass with zero CSP violations reported on every one, the
      guard against a behavioural change in the regenerated bindings that
      unit tests alone would not catch. *(evidence
      `.git-exclude/tmp/handoff043-proofs/`)*

## The dead-code audit (Handoff 044)

An audit, not a cleanup — no source file changed. Classifies every item the
five remaining `#![allow(dead_code)]` directives (`db.rs`, `abuse_limiter.rs`,
`errors.rs`, `crypto.rs`, `authz.rs`) suppress. Deletions follow in a later
package. Full per-item table:
`.git-exclude/audits/zinnias-ciao-main-2026-08-08-handoff044-dead-code-audit.md`.

- [x] **Re-measured counts differ from the handoff's**: 40 dead in a plain
      build (handoff claimed 39), 34 with `cfg(test)` on (handoff claimed
      33), 6 rescued by unit tests (exact match, same six items). Verified
      via `--message-format=json` filtered to `dead_code` diagnostics and
      cross-checked with a fully clean `cargo clean -p zinnias-ciao-ssr`
      rebuild. The `db/`-tree count matches the handoff's stated 31
      exactly — the extra item sits outside `db/`, in `authz.rs` or
      `errors.rs`. Flagged to the architect per the handoff's own §15 stop
      condition rather than resolved unilaterally.
- [x] **RFC-078's abuse limiting is enforced**, not missing. The five
      `abuse_limiter.rs` items (`Row`, `is_json_media_type`, `policy_limits`,
      `TransitionOutcome`, `transition`) are called only from Durable-Object
      glue gated `#[cfg(target_arch = "wasm32")]`, invisible to a native
      `cargo build`. Confirmed on the real deployment target: with the
      allow removed, `cargo check -p zinnias-ciao-ssr --target
      wasm32-unknown-unknown` produces zero dead-code warnings in the file.
      Three handlers (`join.rs`, `relink.rs`, `community_create.rs`) call
      `abuse_control::reserve`/`reset`, which crosses the DO boundary via
      the `ABUSE_LIMITER` binding declared in every `wrangler.toml`
      environment. All five: `deliberate`.
- [x] `crypto::hmac_hex_eq`: `duplicate` of `constant_time_eq` (live at
      `handlers/operator.rs:105`), confirmed. Its own test duplicates
      `constant_time_eq`'s. A dedicated search for any hand-rolled `==`
      comparison on an HMAC/digest/token/secret anywhere in production
      found none — every real comparison already routes through
      `constant_time_eq`.
- [x] 16 of the 40 items are `duplicate` — a live, guarded/atomic
      "`*_required`" transaction superseding an older standalone
      insert/revoke/mark-used helper. **None of the 16 pairs are
      byte-equivalent**: every live version adds an `EXISTS` guard (active
      membership, active admin, matching community) the dead version
      lacks, so deleting the dead copy loses no coverage. The two named
      worked examples (`db/session.rs::insert`, `db/membership.rs::insert_user`/
      `insert_membership`) confirmed exactly as the handoff stated.
- [x] 24 items across 12 `SELECT`s are unread struct fields, grouped by
      query per §7.5 — the eventual deletion decision is per query, not
      per field. Three additional `is_active` items in
      `db/membership.rs` are **not** `SELECT` columns at all: Rust-side
      literals hardcoded `true` at every construction site, redundant with
      the row's existence (`removed_at IS NULL` already gates it).
- [x] **No `finding` verdict was needed.** §7.3 — the question the handoff
      called "most likely to change what the project does next" — resolved
      to enforced, not missing.
- [x] `docs/src/tester/release-checklist.md`'s Handoff 040 entry corrected
      (§7.1): the claim that "the compiler is now the guard" for the nine
      deleted per-file allows was false — `db.rs`'s allow is an inner
      attribute covering the whole `db/` tree and was never touched, so it
      was already suppressing those nine files' warnings throughout. The
      nine deletions and the zero-warning measurement were both still
      correct; only the stated reason was wrong.
- [x] No `.rs` file changed (`git diff --stat` confirms). Full suite green
      at 513, unchanged; clippy, fmt, wasm check, `mdbook build docs`,
      `git diff --check` all pass clean; `bun run build` succeeds at
      28.4kb; the digest gate passes without a re-pin. The ten smokes were
      deliberately not run — no source changed, so they could not have
      told us anything. *(evidence `.git-exclude/tmp/handoff044-proofs/`)*

## Delete the unguarded duplicate writers (Handoff 045)

Not a numbered RFC — a deletion driven directly by the Handoff 044 audit.
**This is guard preservation, not tidiness.** None of the 16 `duplicate`
items the audit found was equivalent to its live counterpart: every live
version carries at least one `EXISTS` guard (active membership, active
admin, matching community) the dead one lacked. A future contributor
reaching for the obviously-named dead helper (`session::insert`,
`invite::mark_used`, …) would have bypassed that guard silently — deleting
removes a way to pick the wrong function name, not just unused code.

- [x] **16 items deleted**, each verified against its live counterpart
      immediately before deletion (still present at the audit's named
      `file:line`, still performing the equivalent write; the dead item
      still genuinely unreferenced per the compiler, not a grep):
      `crypto::hmac_hex_eq`; `errors::to_worker_err` and
      `errors::IntoWorkerResult` (plus its impl — this leaves
      `errors.rs` as a doc-comment-only file, `mod errors;` left in place);
      `db/attendance.rs::counts_for_day`; `db/calendar.rs::insert` and
      `::revoke_for_membership`; `db/event.rs::home_upcoming`;
      `db/invite.rs::mark_used` and `::assign_used_membership`;
      `db/membership.rs::insert_user` and `::insert_membership`;
      `db/relink.rs::revoke_unused_for_membership`, `::insert`, and
      `::mark_used`; `db/session.rs::insert` and
      `::revoke_others_for_user`. No pair inverted the audit's
      finding — in every case the **live** version was the better-guarded
      one, never the dead one.
- [x] Fallout cleaned up alongside each deletion: two now-unused imports
      removed (`db/session.rs`'s `SESSION_TTL_SECONDS`/`add_seconds_to_now`,
      `db/membership.rs`'s `now_utc`) — both were used only by a function
      just deleted; `clippy -D warnings` confirms nothing else went stale.
- [x] `crypto::hmac_hex_eq`'s test (`hmac_hex_eq_constant_time`) deleted
      with the function, **not migrated** — it duplicated
      `constant_time_eq_requires_equal_strings`, asserting nothing the
      surviving test doesn't already cover. **Test count: 513 → 512**,
      exactly the one deletion; nothing else moved.
- [x] Allows re-measured per file after deletion, on both targets, by the
      audit's JSON-diagnostic method (not a grep): **`crypto.rs`'s
      `#![allow(dead_code)]` removed** — zero dead items remained, so the
      compiler is now the real guard for this file. **`errors.rs`'s allow
      was already gone** (removed along with the rest of the file's
      content). **`authz.rs`, `abuse_limiter.rs`, and `db.rs` keep their
      allows** — each still has real remaining items this package doesn't
      touch (`authz.rs`'s `user_id`, `abuse_limiter.rs`'s five
      `deliberate` items, and `db.rs`'s own orphaned `Db` alias plus every
      `db/` submodule's still-unread field). **`db.rs`'s allow in
      particular stays because its inner attribute covers the whole `db/`
      tree** — stated explicitly per Handoff 044's own correction, so this
      package doesn't repeat Handoff 040's unstated-mechanism mistake.
- [x] Dead-code counts, both targets, JSON method: native **40 → 24**,
      `wasm32-unknown-unknown` **35 → 19** — both down by exactly 16, and
      the five `abuse_limiter` items remain live-on-wasm32 throughout,
      confirming this package touched none of them.
- [x] Full suite green: `cargo test --workspace` **512 passed** (513 − the
      one accounted-for deletion); clippy (`-D warnings`), fmt, wasm check,
      and `mdbook build docs` all pass clean; `git diff --check` reports no
      whitespace errors; `bun run build` succeeds at 28.4kb. The digest gate
      passes **without** a re-pin — nothing under `workers/ssr/static/` or
      `render/shell.rs` changed. All ten smokes pass, deliberately run in
      full this time (unlike the audit) since this package deletes code
      from the session, invite, relink, membership, and calendar paths.
      *(evidence `.git-exclude/tmp/handoff045-proofs/`)*

## Unread row fields, and the last of the allows (Handoff 046)

Not a numbered RFC — the closing move on `#![allow(dead_code)]`, driven by
what Handoff 045 left behind. Unlike 045, **this package changes queries**:
a field nothing reads corresponds to a column a `SELECT` still fetches,
deserialised into a struct nobody looks at. The point isn't byte savings —
it's restoring the property that the query fetches what the code reads, so
a future unread-field warning means something again.

- [x] **24 fields removed, grouped by the 12 `SELECT`s that fetched them**
      (`db/attendance.rs`'s `AttendanceRow`/`DayCountRow`, `db/calendar.rs`'s
      `CalendarTokenRow`/`IcsEventRow`, `db/community.rs`'s `CommunityRow`,
      `db/event.rs`'s `EventRow`/`EventDayRow`/`HomeEventRow`,
      `db/event_note.rs`'s `NoteRow`, `db/event_template.rs`'s
      `EventTemplateRow`, `db/membership.rs`'s `MembershipRow`). Every
      struct's full constructor list was found first, not assumed single —
      `HomeEventRow` alone has two (`calendar_month_for_community_limited`,
      `home_upcoming_for_communities`); both moved together, in the same
      commit, so no query was left fetching a column its struct no longer
      declares.
- [x] **`invite.rs::is_valid` and the three `membership.rs::is_active`
      items turned out to be Rust-side literal `true`, not `SELECT`
      columns** — set unconditionally at every construction site, redundant
      with the row's existence (each query's own `WHERE` already guarantees
      "used_at IS NULL"/"removed_at IS NULL" before the row is ever built).
      Neither carries authorisation meaning; removing them touched no query
      and no guard. Called out separately from the `SELECT`-column grouping
      above, since there was no projection to edit.
- [x] **A fallout ring the audit's own coordinates didn't reach**: four
      files beyond `db/` and the five allow-holders also constructed these
      structs directly — one in production
      (`handlers/event.rs`'s `DayCountRow` zero-fallback), three in test
      helpers (`handlers/communities/matrix/tests.rs`,
      `handlers/communities/tests.rs`, `handlers/home/tests.rs`,
      `handlers/admin/events/tests.rs`). All four updated in the same
      commits as their structs; `cargo test -p zinnias-ciao-ssr --lib
      --no-run` after each file's edit is what surfaced them — compile
      errors, not warnings, so nothing could be missed silently.
- [x] `db.rs::Db` (orphaned type alias) deleted — zero references anywhere,
      confirmed before and after.
- [x] `workers/ssr/src/errors.rs` deleted along with its `mod errors;` at
      `lib.rs:10` — after Handoff 045 the file was a doc comment describing
      helpers that no longer existed, the same defect this sequence already
      corrected in the digest comment and the Handoff 040 record.
      `render.rs`'s own unrelated `mod errors;` (→ `render/errors.rs`) is
      untouched — confirmed by name, not by proximity.
- [x] **The three surviving `#![allow(dead_code)]` directives, reduced to
      their smallest honest form**:
      - **`db.rs` — removed entirely.** With the 24 fields resolved and
        `Db` gone, its inner attribute (covering the whole `db/` tree)
        suppressed zero remaining items, confirmed by re-measurement before
        deleting it.
      - **`authz.rs` — replaced with an item-level `#[allow(dead_code)]`**
        on the `user_id` field alone, carrying the audit's reason in a
        comment. Its file-header comment had grouped `community_id` with
        `user_id` as both "populated for completeness, not read" — false
        for `community_id`, which `handlers/me.rs` reads twice; corrected
        while converting the allow, since only `user_id` is actually dead.
      - **`abuse_limiter.rs` — `#![cfg_attr(not(target_arch = "wasm32"),
        allow(dead_code))]`.** Tried and confirmed working: zero dead-code
        warnings from the file on *either* target afterward, where a
        blanket allow would have kept claiming the module has dead code
        even on wasm32, where it does not.
      - **`db/session.rs::touch`**, the `deliberate` item `db.rs`'s allow
        used to cover incidentally, now carries its own item-level
        `#[allow(dead_code)]` with the RFC-038 citation.
- [x] **Dead-code counts, both targets, JSON method**: native **24 → 0**,
      `wasm32-unknown-unknown` **19 → 0**. Every remaining item from the
      five original allow-holders is now individually explained by an
      item-level allow or the `cfg_attr` form — nothing is suppressed by a
      module-wide statement that overclaims.
- [x] Full suite green: `cargo test --workspace` **512 passed** — unchanged
      from Handoff 045 (only existing test-constructor call sites were
      edited; no test added or removed). Clippy (`-D warnings`, clean, no
      fallout to delete beyond the four constructor sites already accounted
      for above), fmt, wasm check, `mdbook build docs` all pass clean;
      `git diff --check` reports no whitespace errors; `bun run build`
      succeeds at 28.4kb. The digest gate passes **without** a re-pin. All
      ten smokes pass, with **no rendered-output change** on any of
      them — the guard against a column reaching a page by a route the
      compiler couldn't see. *(evidence
      `.git-exclude/tmp/handoff046-proofs/`)*

## Session provenance and community binding (Handoff 048, RFC-081 §2/§2.1a — external-identity Slice 1)

Closes a live gap, not preparation for future work: `authz.rs` previously
resolved authorization from `(auth.user_id, community_id)` alone, so a
relink- or help-signin-derived session — grantable by a single community's
admin — authorized *every* community that `user_id` belonged to, not just
the granting one. `handlers/community_create.rs` reusing `auth.user_id` for
a signed-in member's second community is what puts one `user_id` in more
than one community and makes the gap reachable. Invite redemption always
mints a fresh `user_id`, so joining never produced this gap.

- [x] **§3 re-enumerated before starting**: exactly two session-minting
      sites (`db/invite.rs` invite redemption, `db/relink.rs` relink
      redemption) and exactly two relink-issuing paths
      (`handlers/admin/help_signin.rs`, `handlers/operator.rs`, RFC-069),
      both landing in the same redemption. No third site found.
- [x] **Migration `0012_session_provenance.sql`**: adds `provenance TEXT`
      (nullable in schema — a `NOT NULL DEFAULT` would stamp legacy rows
      with a provenance they never had) and `scope_community_id TEXT
      REFERENCES communities(id)` to `sessions`, then revokes every
      pre-existing session row outright. Per RFC-081 §11.4: as of this
      migration no real community has used the service (deployment remains
      No-Go), so there is no legacy-assurance session class to preserve —
      the cheap answer applies.
- [x] The two minting sites now set `provenance`: `'invite_redemption'`
      (first-class, `scope_community_id` left `NULL`) and `'relink'`
      (`scope_community_id` set from the redeemed code's own
      `community_id`). A new release gate
      (`rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`)
      pins this to exactly these two sites, default-fail on any new
      unguarded `INSERT INTO sessions`.
- [x] **`authz.rs` fail-closed refusals**: `NULL` provenance is refused
      unconditionally (an assertion in behaviour, since the migration's
      revoke means no session should reach this state — a refusal, not an
      `unwrap`). A present `scope_community_id` that does not match the
      requested community is refused. Both refusals are indistinguishable
      from "no such membership" — no signal that a membership exists
      elsewhere, that the session is scoped, or which community scoped it.
      The operator-issued relink path (RFC-069) gets the identical
      restriction as the admin-issued one.
- [x] **Deliberately deferred, not omitted**: `authenticated_at` (RFC-080
      §6 also wants authentication time recorded; today `created_at` *is*
      the authentication time at both minting sites, so nothing is lost
      until session rotation arrives in Slice 5, which is the named point
      this should be picked back up).
- [x] **Every caller of `list_active_for_user` and `find_first_admin_for_user`
      decided explicitly** (RFC-081 §7.4 — the side-door risk named as
      "most likely to be missed"): `handlers/home.rs::get_home` and
      `redirect_to_home` rebuilt to construct single-community view data
      from data `require_membership` already resolved, instead of calling
      the enumerating queries, for any bound session.
      `handlers/me.rs::can_create_community`'s link visibility now also
      requires an unscoped session. Every other caller in the tree
      (`admin/role_transfer.rs`, `admin/help_signin.rs`,
      `admin/events/create.rs`, `admin/events/attendance.rs`,
      `admin/events/cancel.rs`, `admin/events/edit.rs`,
      `admin/events/recreate.rs`, `admin/events/notes.rs`,
      `admin/events/copy.rs`, `admin/member_remove.rs`,
      `admin/members.rs`, `export.rs`, `templates.rs`, `event.rs`) calls
      `list_communities_for_user` only *after* an `authz::require_membership`
      or `require_admin` check has already gated the same `community_id`,
      to populate the community-switcher dropdown's link list — a
      display-only enumeration leak (a bound session's switcher can list
      communities it cannot reach), left unfixed as a named out-of-scope
      finding rather than expanded into this slice.
- [x] A refused out-of-scope access attempt is now audited
      (`AuditAction::SessionScopeRefused`, Class C/best-effort, no session
      ID or raw identifiers in metadata) — deliberate, since a
      community-bound session reaching for another community is the exact
      misuse this package exists to make visible.
- [x] Six new `authz.rs` unit tests against the pure `decide_membership_scope`
      / `decide_unscoped_admin_access` functions, plus one new release-gate
      test — `cargo test --workspace` moved **512 → 519**, both new tests
      accounted for. Clippy (`-D warnings`), fmt, wasm check, `mdbook build
      docs`, `git diff --check`, and `bun run build` all pass clean.
      Migration proven both ways: applied to a fresh DB, and applied with a
      pre-existing unrevoked session present (confirmed revoked, with
      `provenance`/`scope_community_id` both `NULL`, after migration).
- [ ] **Full ten-smoke regression is not clean.** Seven of the ten
      pre-existing required smokes currently fail against this branch: the
      cause is upstream of this slice's own logic (raw fixture SQL that
      predates the new column), not a defect in the scope check itself.
      Left unresolved pending owner/architect direction — see the Handoff
      048 review request for the full diagnosis, the two routes discovered
      to bypass `authz.rs` entirely, and the smoke-by-smoke breakdown.
      *(evidence `.git-exclude/tmp/handoff048-proofs/`)*

## Slice 1 correction: scope coverage and fixtures (Handoff 049)

Both open items above are corrections to Handoff 048's own review
(`.git-exclude/reviewed/zinnias-ciao-main-2026-08-10-session-provenance-and-community-binding-review.md`),
not new work — the mechanism from Handoff 048 was already correct; two
routes never called into it, and a smoke-fixture assumption in that
handoff's own §9 was never possible.

- [x] `handlers/communities.rs::get_communities` (the calendar/matrix view,
      `/c/{id}/communities`) no longer hand-rolls its membership check —
      it calls `authz::require_membership` like every other page-level
      route, which removed the separate `membership_db::find_active` call
      it used to make (same query resolved once instead of twice).
- [x] `db/membership.rs::list_communities_for_user` takes a **required**
      `scope_community_id: Option<&str>` parameter — every one of its 21
      call sites updated to pass `auth.scope_community_id.as_deref()`, so a
      third caller cannot omit it the way these two did. This also closes
      the community-switcher-dropdown leak named (but left unfixed) in
      Handoff 048's own record above: a bound session's switcher no longer
      lists communities it cannot reach.
- [x] `handlers/community.rs::get_switch` (`/switch?community=`) refuses a
      `NULL`-provenance session explicitly (`authz::has_provenance`, the
      same branch `decide_membership_scope` uses, not a second copy of the
      check) and can no longer pivot a bound session out of scope — its
      membership list is now the same scope-filtered result, so the
      non-member-target fallback (`memberships.first()`) lands on the
      bound community rather than an assumed-safe guess.
- [x] `handlers/home.rs::get_home`'s Handoff 048 bound-session branch
      partially simplified: `community_summaries` now always comes from
      the (now scope-filtered) `list_communities_for_user`, removing the
      hand-built single-element summary and its extra `find_active` call.
      The `list_active_for_user`-derived membership *count* still branches
      explicitly, since that function has no scope parameter (out of this
      package's §4.2) and calling it unfiltered for a bound session would
      leak a second membership's existence through the first-run/no-events
      wording choice.
- [x] All 19 `scripts/smoke/*.mjs` fixtures that seed a session now set
      `provenance = 'invite_redemption'` — not just the seven that were
      red, all nineteen, since the other twelve were latent failures
      waiting for whichever assertion first crossed a gated route.
- [x] A new release gate (`handoff049_smoke_session_fixtures_all_set_a_provenance`)
      walks `scripts/smoke/*.mjs` directly (not a curated filename list —
      a curated list is exactly the shape that let 18 of 19 fixtures go
      stale silently) and fails if any `INSERT INTO sessions` omits
      `provenance`. Proven firing: a temporary provenance-less insert
      added to a fixture failed the gate as expected, then was removed and
      `cmp`-verified byte-identical.
      `rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`
      (Handoff 048's gate, over `workers/ssr/src`) re-confirmed still
      passing — untouched by this package.
      `cargo test --workspace`: **519 → 520**, the one new gate test.
- [x] `smoke:session-scope` extended with four checks proving the
      previously-uncovered routes are fixed: a bound session cannot load
      the other community's calendar/matrix view, cannot pivot to it via
      `/switch`, does not see it listed in its own switcher, and — the
      check that keeps the fix honest — a first-class session with the
      identical two memberships still reaches both communities via both
      routes.
- [x] **All ten pre-existing required smokes green**, plus the extended
      eleventh — the headline result of this package. Zero failed checks,
      zero CSP violations, across all eleven. Digest gate passes without a
      re-pin (bundle unchanged at 28.4kb). *(evidence
      `.git-exclude/tmp/handoff049-proofs/`)*

## Identity schema and namespaces (Handoff 050, external-identity Slice 2)

Purely additive: two new tables nothing reads yet, and a digest helper with
no caller yet. No route, handler, or `authz.rs` change — RFC-081 §2's
authorization boundary (Slices 0–1) is untouched by this package.

- [x] Migration `0013_identity_namespaces.sql`: **no `ALTER TABLE` on any
      existing table.** Creates `identity_namespaces` (RFC-080 §3.2 — an
      immutable record of a reviewed provider registration: provider kind,
      issuer, audience, subject scope, environment, created-at) and
      `user_identities` (RFC-080 §3.3, `UNIQUE(identity_namespace_id,
      subject_lookup)` — not the subject alone, not email). Seeds exactly
      one row: the `local_fake` namespace. No production or staging
      namespace exists.
- [x] `users.idp_subject` untouched — not populated, not referenced, not
      commented on. Per Handoff 050 §3, dropping it moves to Slice 3: SQLite
      refuses `DROP COLUMN` on a `UNIQUE` column, so removing it requires a
      full table rebuild with FK handling for `community_memberships` and
      `sessions` — the same table-rebuild shape Slice 3 already needs for
      `community_memberships`' `UNIQUE(community_id, user_id)`, so both
      rebuilds land in one reviewed package instead of two.
- [x] `crypto::subject_lookup` — a keyed digest (`hmac_hex` with the
      existing pepper, AD-3/RFC-077; no second hashing path). The subject
      is treated as opaque and case-sensitive: no normalisation, no
      lowercasing. Four unit tests: deterministic for the same input;
      different pepper gives a different digest; a case difference gives a
      **different** digest (proves no normalisation); the raw subject
      never appears in the 64-hex-char output.
- [x] `db/identity.rs`: one row type, one lookup by
      `(identity_namespace_id, subject_lookup)` — nothing beyond what
      Slice 4's authentication callback will immediately use. Both it and
      `crypto::subject_lookup` carry an **item-level**
      `#[allow(dead_code)]` naming Slice 4 as the arriving caller. No
      module-wide `#![allow(dead_code)]` added anywhere in the crate —
      confirmed by grep, zero real occurrences (the only three matches are
      comments referencing the *absence* of one, from Handoffs 044–046).
- [x] **§5.4 decided: `list_active_for_user` given the same required
      `scope_community_id` parameter** `list_communities_for_user` gained
      in Handoff 049, not left as a documented exception. It was a second
      enumeration of the same fact, safe only because `home.rs` happened to
      branch around its one scope-sensitive consumer — a future caller
      would have had no such protection. Both of `home.rs`'s call sites
      updated; neither needed behavioural change (`redirect_to_home` and
      `get_home`'s `is_first_run` count both already avoided calling it at
      all for a bound session, which remains the cheaper choice over
      calling the now-scoped version).
- [x] New gate, `rfc080_identity_namespaces_are_never_created_outside_a_migration`:
      unlike the two session-minting gates, this one has **no exceptions
      table** — every `INSERT INTO identity_namespaces` under
      `workers/ssr/src` is unconditionally wrong, since RFC-080 §3.2
      requires namespaces to come from a migration or reviewed
      configuration only. Proven firing: a temporary insert added to
      `db/identity.rs` failed the gate as expected, removed and
      `cmp`-verified byte-identical. Both prior gates
      (`rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`,
      `handoff049_smoke_session_fixtures_all_set_a_provenance`)
      re-confirmed still passing, untouched by this package.
- [x] `cargo test --workspace`: **520 → 525** — one new gate test, four new
      digest unit tests, both accounted for. Clippy (`-D warnings`, no new
      module-wide allow), fmt, wasm check, `mdbook build docs`,
      `git diff --check`, `bun run build` all pass clean. Migration proven
      both ways: applied to a fresh dev database, and applied to one
      already at 0012.
- [x] **All eleven smokes green, unchanged** — the expected result for a
      purely additive package: nothing reads the new tables yet, so nothing
      should have moved. Digest gate passes without a re-pin. *(evidence
      `.git-exclude/tmp/handoff050-proofs/`)*
- [x] **Review correction**: `crypto::subject_lookup` now mixes
      `identity_namespace_id` into the digest input (with a `\u{1f}`
      unit-separator against concatenation collisions), not `subject`
      alone. RFC-080 §3.1 requires that two different namespaces never be
      inferable as the same person; without the namespace in the input,
      the same subject linked under two namespaces (an *expected* state —
      §7 forbids auto-linking, §12 puts merge out of scope) would have
      produced an identical digest. Since the raw subject is never stored,
      this had to be correct before the first `user_identities` row is
      ever linked — the table is still empty, so the fix was free. Fifth
      unit test added: the same subject under two different namespaces
      produces different digests. `cargo test --workspace`: **525 → 526**.
      Clippy, fmt, wasm check, `mdbook build docs`, `git diff --check`,
      `bun run build` re-confirmed clean; digest gate without a re-pin.

## Schema re-baseline: membership continuity, at source (Handoff 052, replaces Handoff 051)

Handoff 051 (rebuilding `community_memberships`/`users` via a forward
migration 0014) escalated as a stop condition: under D1, a table with live
foreign-key dependents cannot be dropped, by any of five mechanisms tested
(`PRAGMA foreign_keys = OFF`, in-file and as a separate call;
`PRAGMA defer_foreign_keys = ON`; renaming the old table out of the way;
explicit `BEGIN`/`COMMIT`). RFC-081 §1.2a records the finding. **Owner
decision, 2026-08-10**: because no database outside a developer's machine
has ever applied these migrations, `migrations/0001_initial.sql` is
corrected at source instead, under a one-time exception recorded in
`ROADMAP.md` ("Migration immutability begins at first deployment") that
expires at first deployment and must not be cited afterward.

- [x] **Three edits to `0001_initial.sql`, nothing else.** Removed the
      table-level `UNIQUE(community_id, user_id)` on `community_memberships`;
      added `idx_memberships_one_active_per_user` (partial unique index,
      `WHERE removed_at IS NULL`) beside the two pre-existing indexes;
      removed `users.idp_subject` and its comment. A dated header comment
      records the exception and points at RFC-081 §1.2a and `ROADMAP.md`.
      No new migration — `0013_identity_namespaces.sql` remains the head;
      the `rfc079_package7_…` migration-filename-list gate needed no
      update, confirmed by re-running it. No migration other than `0001`
      touched — confirmed by `git status`/`git diff --stat`.
- [x] **Every verification run against a genuinely fresh database**
      (`bun run reset:dev`, which deletes `.wrangler/state/v3/d1` outright
      before reapplying every migration — not a database that had
      previously applied the old `0001`). Confirmed by direct query, not
      by reading the SQL: the only autoindex on `community_memberships` is
      `sqlite_autoindex_community_memberships_1`, and
      `pragma_index_info` shows it covers `id` alone (`origin: "pk"`) —
      not `(community_id, user_id)`; `idx_memberships_one_active_per_user`
      confirmed `unique=1, partial=1` via `pragma_index_list`; both
      pre-existing indexes present; `users` has exactly `id`, `created_at`
      via `pragma_table_info`; migrations 0001–0013 all applied; 0013's
      `idns_local_fake` namespace present.
- [x] **All three invariant cases demonstrated** against the same fresh
      database, using the dev-seeded community: two `removed_at IS NULL`
      rows for the same `(community_id, user_id)` — **rejected**
      (`SQLITE_CONSTRAINT_UNIQUE`); one removed plus one new active row for
      the same pair — **accepted** (the case the old constraint made
      impossible, and the entire reason for the change); two removed rows
      for the same pair — **accepted**. Probe rows cleaned up afterward,
      confirmed by row count back to the seed baseline before the smoke run.
- [x] `docs/src/developer/architecture.md`'s AD-2 summary corrected:
      `users.idp_subject` no longer described as nullable/reserved: it was
      rejected by RFC-080 §3.4 and removed; `user_identities` (migration
      0013) is the replacement, keyed on
      `(identity_namespace_id, subject_lookup)`. The frozen v1 historical
      record (`docs/src/shared/ref/roadmap-and-rfcs-v1/ARCHITECTURE-DECISIONS.md`)
      left untouched — it is accurate to what was decided then.
- [x] `cargo test --workspace`: **526, unchanged** — this package adds no
      Rust. Clippy (`-D warnings`), fmt, wasm check, `mdbook build docs`,
      `git diff --check`, `bun run build` (28.4kb, unchanged) all pass
      clean. Digest gate passes without a re-pin.
- [x] **All eleven smokes green from a fresh database** — each smoke seeds
      its own fixtures against the corrected schema end to end, which is
      the strongest evidence the re-baseline is faithful to what the
      application actually needs. Zero failed checks, zero CSP violations.
      *(evidence `.git-exclude/tmp/handoff052-proofs/`)*

## The authentication transaction and the fake issuer (Handoff 053, external-identity Slice 4a)

Pure mechanism: a table, a verification path, a test-only issuer. No route
reaches any of it, no network call exists anywhere in it — confirmed both
by a dedicated gate and by the build itself staying at 28.4kb.

- [x] Migration `0014_auth_transactions.sql`, additive only. Follows AD-3's
      digest-at-rest discipline: `lookup_key_hmac` (the OIDC `state`
      value's HMAC — finding the row by it *is* the state check, so there
      is no separate stored "expected state" to drift from it) and
      `nonce_hmac` are digests; the raw values are never stored.
      `pkce_verifier` is stored raw, deliberately — it must be recoverable
      for the token exchange, and alone it is not a bearer credential
      (the also-required, single-use, provider-issued authorization code
      is never stored at all).
- [x] **JWT handling is hand-rolled, not library-based** — `jsonwebtoken`
      (even restricted to its `rust_crypto` feature) failed to compile to
      `wasm32-unknown-unknown` via a transitive `getrandom` conflict, and
      pulled in RSA/PKCS machinery this package never needs. Built instead
      on the same RustCrypto primitives (`hmac`, `sha2`) already used by
      `crypto::hmac_hex`, plus a new minimal `base64` dependency (confirmed
      wasm32-clean). This makes the algorithm-pinning guarantee airtight
      by construction: there is no library default to research or trust,
      because the caller (never the token) supplies the expected algorithm
      and key source.
- [x] **Algorithm pinning proven with the full negative matrix**: correct
      algorithm accepted; every other algorithm rejected, including one
      the token's own header claims is correct; `alg: none` rejected
      unconditionally; unknown key id rejected; malformed signature
      rejected — each its own distinct rejection reason, its own test, 28
      tests total across the module.
- [x] **§5.6 decided**: a revoked identity authenticates nobody and is
      indistinguishable to the caller from one never linked —
      `identity::identity_lookup_is_authenticatable`, tested, is the one
      place that decision is made; `db::identity::find_by_subject_lookup`
      still returns revoked rows unfiltered, on purpose, so the decision
      stays in the layer that can reason about it.
- [x] Both required gates (no network call anywhere under `identity/`; the
      fake issuer structurally absent from non-test builds via
      `#[cfg(test)] mod fake_issuer;`) proven firing, `cmp`-verified
      restores.
- [x] `cargo test --workspace`: **526 → 556** — 28 identity-module tests
      plus the 2 new gate tests, exactly accounted for. Clippy, fmt, wasm
      check, `mdbook build docs`, `git diff --check`, `bun run build`
      (28.4kb, unchanged) all clean. Digest gate passes without a re-pin.
      Migration proven both ways: fresh database, and one already at 0013
      with a pre-existing row confirmed to survive. Transaction table's
      single-use/expiry/replay guarantees proven directly against a real
      local D1 (no route exists yet to exercise them through).
- [x] **All eleven smokes green, unchanged** — expected for a package that
      adds no reachable surface. *(evidence
      `.git-exclude/tmp/handoff053-proofs/`)*

## The authentication callback route (Handoff 054, external-identity Slice 4b)

4a built a verification path nothing could reach; this package makes it
reachable — the point and the risk of the same package, since every
mistake here is a live authentication path.

- [x] **`dev_fake_issuer` Cargo feature (default off) is the structural
      guarantee §3 requires.** Gates both the fake issuer's HTTP routes
      (`identity::dev_fake_issuer`, entirely absent as a module without
      it) and the namespace-verification-requirements resolver's only
      Some-returning branch — the feature-off variant of
      `resolve_namespace_verification` returns `None` unconditionally, no
      configuration can re-enable it at runtime. Established first,
      empirically, that `wrangler dev` and `bun run build` invoke the
      identical root `[build]` command with no built-in per-environment
      differentiation, and that `scripts/lib/isolated-worker-test.mjs`
      copies the pre-built shared artifact rather than rebuilding — so the
      new smoke builds its own isolated, feature-on artifact into a
      scratch directory and overwrites only its own already-isolated copy,
      never the shared `workers/ssr/build/` the other eleven smokes depend
      on.
- [x] **Both required gates proven firing on every failure branch**, each
      `cmp`-verified byte-identical after restore:
      `identity_dev_fake_issuer_absent_from_production_build`
      (`test:dev-fake-issuer-absent`) — builds both a feature-off and a
      feature-on artifact into scratch directories, asserts four markers
      unique to the gated route paths/issuer constants are absent from the
      former and present in the latter (proving the search itself isn't
      vacuous); fired by flipping the feature to default-on in
      `Cargo.toml`. `rfc081_session_minting_sites_are_enumerated_and_set_a_provenance`,
      strengthened with two new checks (no SQL string literal at any
      minting site; every site's Rust call-site text references
      `SessionProvenance::`) — fired independently on both branches
      (reintroducing a literal; inlining an untyped string past the type).
- [x] **`SessionProvenance` closes the RFC-080 §6 / Handoff 054 §5.4 typo hazard** — a
      compile-time enum with a single serialisation point, converted at
      all three minting sites (`db/invite.rs`, `db/relink.rs`, the new
      `db/auth_transaction.rs`), gate-enforced.
- [x] **Nine-step callback contract (RFC-080 §5.1)**: transaction
      consumed/reserved before the code exchange, not after; only
      `VerifiedExternalIdentity` crosses into identity logic. Safe-return
      allowlist is server-side, resolved from the transaction row, never a
      request parameter — `//evil.example` (the protocol-relative trap)
      and backslash variants explicitly rejected, both by unit test and by
      the smoke seeding a malicious `return_to` directly into the row (the
      only way it could ever arrive there) and confirming the callback
      still redirects to the safe default.
- [x] **One real bug found and fixed by the smoke, not by inspection**:
      `db::auth_transaction::insert_required`'s three `Option<&str>`
      columns were bound via bare `.into()`, which produces a JS
      `undefined` for `None` rather than `null` — D1 rejects `undefined`
      binds outright (`D1_TYPE_ERROR`), so every `/identity/start` call
      failed closed with a generic 500 before this fix. Corrected to the
      same `.map(JsValue::from_str).unwrap_or(JsValue::NULL)` pattern
      already established in `db/event_template.rs`/`db/event_write.rs`.
      Native `cargo test` could not have caught this — it is a D1-runtime
      binding behaviour, not a compile-time type error — which is exactly
      why this slice's own §6 called for an end-to-end proof.
- [x] **Review-054 fix: `mint_malformed_signature` did not reliably
      malform.** It mutated the base64url signature's *last* character —
      but a 32-byte HMAC encodes to 43 characters carrying 258 bits, so
      the last character holds only 4 significant bits (2 are padding);
      4 of the 64 alphabet characters decode to the identical byte there.
      The mutation was a silent no-op on exactly one of those (`'A'`),
      making `malformed_signature_is_rejected` fail (`unwrap_err()`
      panicking on an unexpectedly-valid signature) about 1 run in 16 —
      measured at 3/31 by the reviewer. Fixed to mutate the signature's
      *first* character instead, which always carries a full 6
      significant bits with no padding ambiguity, making the mutation
      unconditional. Re-run 50 times as separate process invocations
      (each with a freshly random HMAC key): 0 failures. Full workspace
      suite, clippy (both feature states), and both wasm checks re-run
      clean after the fix; test count unchanged (566 — a logic fix, not a
      new test).
- [x] **The new identity smoke, all six required scenarios green**:
      successful sign-in issuing a session with provenance
      `external_identity`; a replayed callback rejected; a tampered
      `state` rejected; a wrong `nonce` (tampered before reaching the
      provider) rejected; an out-of-allowlist `return_to` refused; the
      whole flow succeeding end-to-end with a real headless browser,
      application JavaScript fully disabled
      (`Emulation.setScriptExecutionDisabled`), zero CSP violations.
- [x] No CSP change (top-level navigation only, `form-action 'self'`
      unaffected — confirmed by `git diff` on `attach_security_headers`
      showing no touch). No account surface, no `authenticated_at`, no
      version bump.
- [x] `cargo test --workspace`: **556 → 566** — exactly the 10 new tests in
      `handlers/identity/tests.rs` (allowlist rejections, provenance
      exhaustiveness, invite-reference extraction, urlencode); the
      strengthened `release_gates` assertion added no new test. Clippy
      (native and `--features dev_fake_issuer`), fmt, wasm check (both
      feature states), `mdbook build docs`, `git diff --check` all clean.
      `bun run build`: 28.4kb → 28.6kb — expected, not suspicious: this
      slice adds a real reachable route/handler path even with the
      feature off, unlike 4a. Digest/cache-version gate passes without a
      re-pin (no version bump, no cached asset touched).
- [x] **All twelve smokes green** — the eleven pre-existing unmodified,
      plus the new `smoke:identity-callback`. Zero CSP violations, zero
      stray processes left running. *(evidence
      `.git-exclude/evidence/handoff054/`)*

## Session freshness and the account surface (Handoff 055, external-identity Slice 5a)

The first non-community-scoped member route tier — read-only, nothing here
can remove a credential (5b's job).

- [x] **§5.5 investigated before building anything**: a no-membership
      session hitting `/` today gets `render::session_expired()` (401,
      generic Japanese text, no distinct "no communities" disclosure);
      `/switch` redirects to `/join`; `/c/:cid/*` 404s generically;
      `/communities/new` 404s (a separate, pre-existing admin-somewhere
      gate). No leak, no uncaught error — confirmed by seeding a real
      no-membership session against a running local instance and reading
      the actual responses, not by inspection alone.
- [x] Migration `0015_session_authenticated_at.sql`, additive/nullable,
      matching `provenance`'s own precedent — no default, existing rows
      get `NULL`, treated as not-fresh fail-closed. Applied both ways:
      fresh (0001→0015) and onto a database already at 0014, with a
      pre-existing session row confirmed to survive with `authenticated_at
      = NULL`.
- [x] **Every minting site sets `authenticated_at = created_at`** at
      creation (`db/invite.rs`, `db/relink.rs`, both `db/auth_transaction.rs`
      sites) — the existing provenance minting-site gate extended, not
      duplicated, with the same class of check. Proven firing: removed the
      column from one site, confirmed the gate failed naming it, restored,
      `cmp`-verified byte-identical.
- [x] **The step-up predicate** (`authz::is_fresh_for_account_operations`)
      is pure — no D1, no wall-clock read, `freshness_window_start`
      supplied by the caller as an ISO8601 string in `db::now_utc`'s own
      fixed shape, compared lexicographically (no date parsing needed).
      Factored the shared "is this an account-tier session at all"
      condition (provenance present and not `Relink`, unscoped) out into
      `is_account_tier_session`, reused by both the freshness predicate
      and the new `decide_account_surface_access` — one assertion, not two
      that could drift. Exhaustive tests: every provenance × scope ×
      freshness combination via a brute-force cross-product, plus the
      boundary itself pinned both ways (`>=`, inclusive — exactly-900-
      seconds-old is still fresh; one millisecond earlier is not).
- [x] **§5.3's decision recorded, not yet exercised**: first link will not
      require freshness (the fresh OIDC transaction *is* the step-up, and
      an invite-only member has no other way to ever become fresh before
      linking); unlink and recovery-credential changes will. Carried
      forward to Slice 5b's handoff, not overturned here.
- [x] **`/account`**: read-only, no application JavaScript, Japanese-only
      (RFC-072 Slice D — no single community-scoped `ui_language` to
      resolve from an account-tier session, same reasoning as
      `handlers/identity/mod.rs`). Displays linked identities (namespace +
      `linked_at` only — `db::identity::LinkedIdentitySummary` cannot
      structurally carry a subject or digest, since the query never
      selects those columns), the communities the principal belongs to,
      "no recovery credential yet" (always, in 5a), and whether the
      session is fresh enough to manage settings. A `Relink`-provenance
      session is refused the surface entirely
      (`authz::require_account_surface`, audited the same way
      `require_active_admin_somewhere`'s bound-session refusal already
      is). A no-membership principal reaches the page and nothing else —
      RFC-081 §6.
- [x] **`ALLOWED_RETURN_DESTINATIONS` grows for the first time**: `/account`
      added alongside `/`. No live call site produces this value yet — a
      working "re-authenticate, land back on /account" flow needs
      `handlers/identity/mod.rs::get_start`'s existing already-
      authenticated short-circuit to change first (it currently bounces
      *any* valid session straight to `/`, stale or not, before a fresh
      re-auth could ever run), which is session-rotation-adjacent territory
      left to Slice 5b rather than decided here. `resolve_safe_return`
      keeps its `&'static str` return type; the growth is exercised by its
      own unit test, not left dead.
- [x] `cargo test --workspace`: **566 → 594** (+28: 15 in `authz.rs` — the
      account-surface-access decision and the freshness predicate, both
      exhaustive; 1 in `handlers/identity/tests.rs` — the allowlist
      growth; 12 in the new `handlers/account/tests.rs`), exactly
      accounted for. Clippy (native and `--features dev_fake_issuer`),
      fmt, wasm check (both feature states), `mdbook build docs`,
      `git diff --check` all clean. `bun run build`: `index.js` unchanged
      at 28.6kb, `index_bg.wasm` 1,512,220 → 1,521,140 bytes (+~8.7KB) —
      the new authz/handler code, JS glue untouched since no new route
      shape changed at that layer. Digest/cache-version gate passes
      without a re-pin.
- [x] **All thirteen smokes green**, including the new
      `smoke:account-surface` (fresh session's full display exercised
      with JavaScript disabled in a real browser, a stale session's "sign
      in again" display, the RFC-081 §6 no-membership state, and a
      Relink-provenance session refused entirely). Zero CSP violations,
      zero stray processes left running. One transient, unrelated
      `smoke:display-name` network flake reproduced once and passed clean
      on immediate retry — not a regression, noted rather than hidden.
      *(evidence `.git-exclude/evidence/handoff055/`)*

## Handoff 056 — Slice 5b: linking and re-authentication

- [x] **`get_start`'s already-authenticated short-circuit now checks
      freshness, `sign_in` only.** `action=join` is untouched — any valid
      session (fresh or not) still bounces to `/`, since joining is
      invite-driven and has no freshness question. For `sign_in`, a fresh
      account-tier session still bounces to `/`; a valid-but-stale one now
      proceeds through the same nine-step OIDC transaction machinery
      instead of being turned away, with `return_to` set to `/account` and
      `prompt=login` requested.
- [x] **A completed OIDC round trip is not treated as proof of fresh
      authentication.** `should_send_prompt_login(action, caller_has_valid_session)`
      is a pure, exhaustively unit-tested decision (`link` always sends it;
      `sign_in` sends it only when a valid session already exists; `join`
      never does — all three actions crossed with both session states, six
      cases, one test each plus a full cross-product test). The fake
      issuer's `resolve_auth_time(prompted_login, now)` returns `now` when
      prompted and `now - 3600` when not, so a test can tell "re-prompted"
      apart from "SSO reuse" — this claim is not read or checked anywhere
      in `identity::verify_id_token` (deliberately: whether a *real*
      provider honours `prompt=login` is a Stage 3 provider-selection
      criterion, not something this codebase can enforce against a
      provider that lies).
- [x] **Re-authentication rotates, it does not touch `authenticated_at` in
      place.** `db::auth_transaction::reauthenticate_required` mints a new
      session row, revokes the old one, and revokes every other active
      session for that `user_id` (`db::session::revoke_others_statement`,
      factored out of `db/relink.rs`'s existing proven shape and now
      shared by both linking and re-authentication) — atomically with the
      identity touch, in one `execute_asserted_required` batch. Proven via
      a real round trip: same principal, same identity, new session id,
      old session's `revoked_at` set, exactly one active session
      remaining afterward.
- [x] **Linking (RFC-081 §4)**: from an account-tier session only
      (`Relink` and any community-scoped session refused, same boundary as
      the account surface itself), gated by a purpose-bound, user-bound,
      single-use token (`token_purpose::LINK_IDENTITY`, the same
      `codlet`/`form_token` machinery `community_create.rs` already uses)
      distinct from the OIDC transaction itself, with an explicit
      confirmation step before the redirect and `prompt=login`
      unconditionally. `initiating_user_id` (migration 0016, nullable,
      link-only) is pinned onto the transaction row at *creation* time from
      the already-verified session, not re-derived from a live session
      cookie at callback time — closing a cross-session-swap window where
      the session could otherwise change principal between initiation and
      callback. Re-authentication needs no equivalent column: its target
      user always comes from the verified identity itself, so a live-
      session mismatch just falls back to an ordinary, non-rotating
      sign-in rather than any cross-account risk.
- [x] **Collision fails closed, generically, and is audited.** If the
      verified identity already belongs to another principal, the caller
      sees the same generic failure copy as any other rejection — no row
      written, no disclosure that the identity is known elsewhere. Because
      no business mutation happens in this case, the audit write needed a
      new primitive: `audit::execute_required_standalone`, the first
      Class-A-required write in this codebase with no paired mutation
      (distinct from `write_session_scope_refused`'s best-effort Class
      B/C pattern) — added to the Class A executor gate's own inventory.
- [x] **§5.2's additive invariant, established two ways, not asserted.**
      By construction: `link_required`'s only non-`INSERT` statement is
      the revoke-others `UPDATE`, which touches `sessions`, never
      `user_identities` — there is no code path in this function capable of
      removing or deactivating an existing link. By a codebase-wide,
      default-fail gate: `no_unlink_path_exists_for_user_identities` scans
      every `.rs` file under `workers/ssr/src` and forbids
      `"DELETE FROM user_identities"` and
      `"UPDATE user_identities SET status"` unconditionally (deliberately
      not forbidding `UPDATE user_identities SET last_authenticated_at`, a
      different column with no bearing on usability) — any future
      violation, anywhere in the codebase, fails this gate, not just a
      review of this package's diff. Fired: added a matching `UPDATE`
      statement to `db/identity.rs`, confirmed the gate failed naming the
      file, restored (`cmp`-verified byte-identical), reconfirmed green.
- [x] **`/account` is now a produced return destination**, resolving 5a's
      flagged open question: both link's confirmation and `sign_in`
      re-authentication set `return_to = "/account"`.
      `ALLOWED_RETURN_DESTINATIONS` itself is unchanged from 5a (`/`,
      `/account`) — only the producer existed to add.
- [x] **Three new Class A audit actions** — `external_identity.linked`,
      `external_identity.link_rejected`, `session.external_reauthenticated`
      — `AuditAction::ALL` 28 → 31, the Class A inventory gate 24 → 27.
- [x] `cargo test --workspace`: **594 → 600** (+6 under default features: 2
      in the new release gates, 4 in `handlers/identity/tests.rs`'s
      `prompt=login` decision coverage) — plus 3 more
      (`identity/dev_fake_issuer.rs`'s `resolve_auth_time` coverage) that
      only compile under the `dev_fake_issuer` feature (267 → 270 in the
      `ssr` crate alone with that feature on), exactly accounted for.
      Clippy (native and `--features dev_fake_issuer`), fmt, wasm check
      (both feature states), `mdbook build docs`, `git diff --check` all
      clean. `bun run build`: `index.js` unchanged at 28.6kb,
      `index_bg.wasm` 1,521,140 → 1,537,003 bytes (+~15.5KB) — the new
      migration/handler/db/audit code; JS glue untouched, no new route
      shape at that layer. Digest/cache-version gate passes without a
      re-pin. No version bump.
- [x] **Fourteen smokes green**, including the new
      `smoke:account-link-reauth` (six scenarios: `action=join` unchanged
      for a signed-in session; a `Relink`-provenance session refused the
      link entry point entirely; link succeeds with rotation, the new
      identity row, and the old session revoked; a second principal's
      link attempt against the same fake-issuer subject collides
      generically with no row written; that same first principal's own
      rotated session, staled by directly updating `authenticated_at`,
      re-authenticates with a new session id and the old one revoked; and
      the link confirmation flow, through to the generic collision-failure
      page it necessarily reaches on this pass, navigating correctly with
      application JavaScript fully disabled in a real browser). Zero CSP
      violations, zero stray processes left running.

## Handoff 057 — Slice 5c: the recovery credential and unlink

- [x] **The recovery credential**: issued automatically the first time a
      member ever links an identity (`db::recovery::issue_at_first_link_required`,
      called by `handlers/identity/mod.rs::link_outcome` immediately after
      a successful link — a deliberately separate call, not bundled into
      the same batch, so the result doesn't need to be extracted from
      `execute_asserted_required`'s generic batch index). HMAC at rest
      (migration 0017, `code_hmac` only — no raw/plaintext column exists).
      `expires_at` is nullable and this package never sets it: unlike a
      relink code, a recovery credential is meant to remain usable
      indefinitely, not redeemed within minutes. Shown exactly once, in
      the same response that generated or regenerated it — never a
      redirect, since a redirect has nowhere safe to carry a one-time
      plaintext value. Regenerating revokes whatever was previously active
      in the same batch as the new insert
      (`execute_required_tail(vec![revoke_previous, issue_new], ...)`), so
      a member can never hold two.
- [x] **The anonymous consumption route (`/recovery`)** mints an
      account-tier, unscoped, fresh session via a fourth
      `SessionProvenance::AccountRecovery` variant — pinned by a dedicated
      test, not assumed, per the handoff's own instruction. All four
      consumption-failure causes (unknown, consumed, revoked, expired)
      collapse into one generic, identical response: no early
      classification branch exists to leak which cause fired, the same
      "define the invalid state once" discipline `db/relink.rs` already
      uses. Abuse-limited under its own `Scope::Recovery` (`(5, 300_000)`
      — stricter than invite/relink's `(10, 300_000)`, since this route
      authenticates an entire account with a credential that never
      expires), reserved before any credential lookup — a gate proves
      this by source position, not merely presence, after discovering
      (while proving the gate fires) that this file's own module doc
      comment mentioning `abuse_control::reserve` in prose would
      otherwise satisfy a naive presence check regardless of where the
      real call sat; the gate now strips comments first.
- [x] **Unlink — the one legitimate exception to "additive by
      construction."** `db/identity.rs::unlink_required`'s claim is a
      single `UPDATE user_identities SET status='revoked' WHERE id=?1 AND
      user_id=?2 AND status='active' AND {usable-method-check}` — the
      "usable method" definition
      (`db::recovery::usable_method_exists_sql`) is centralized in one
      function and embedded, with independent placeholder numbering, into
      both this claim and the same-batch revoke-others statement, so the
      two can never disagree within one unlink attempt. This — not a
      `SELECT` beforehand — is what makes the required concurrency
      guarantee hold: two requests racing to unlink different identities
      on a two-identity account are serialized by D1's single-writer
      model, and whichever runs second evaluates the usable-method check
      *after* the first has already committed, correctly seeing zero
      remaining methods and declining. `claim` is the batch's tail (not
      `revoke_others`), so the required audit gates on **its** row count;
      `revoke_others` carries the identical guard for a second reason —
      without it, a refused unlink could still silently revoke the
      member's other sessions as a side effect merely because it ran
      earlier in the same batch. Requires a fresh account-tier session
      (redirects to re-authenticate when stale, rather than a dead end);
      refuses generically on decline, row untouched.
- [x] **The unlink gate relaxed, not deleted**: `no_unlink_path_exists_for_user_identities`
      became an exceptions-table (`USER_IDENTITIES_UNLINK_EXCEPTIONS`),
      the same shape as `KNOWN_SESSION_MINTING_SITES` — one named site
      (`db/identity.rs`) with a written reason. `DELETE FROM
      user_identities` stays unconditionally forbidden, no exception ever
      possible. A defence-in-depth assertion on top confirms the named
      site's own statement still references both the `status = 'active'`
      guard and the shared `usable_method_exists_sql` call, catching a
      regression that quietly dropped the guard while leaving the
      forbidden-pattern match intact. Both halves proven firing (a second,
      unnamed file; and a weakened guard on the named site), with
      `cmp`-verified restores.
- [x] **No admin surface reaches any recovery operation** — a default-fail
      gate scans every file under `handlers/admin/` for any reference to
      the recovery/unlink machinery, matching RFC-081 §2's existing
      community-admin-authority boundary.
- [x] `cargo test --workspace`: **600 → 622** (+22: +3 in the new
      `release_gates.rs` gates, +6 in `packages/domain`'s recovery-code
      validation, +13 in the `ssr` crate — `db/recovery.rs`,
      `handlers/account/recovery.rs`'s code-generation tests, the new
      `authz`/`abuse_control`/`abuse_limiter` pinning tests, and the
      account-page rendering tests) — plus `--features dev_fake_issuer`:
      **270 → 283** (same +13 `ssr`-crate delta, the fake-issuer-gated
      tests unaffected). Both deltas reconciled against the diff's new
      `#[test]` functions, including ones in brand-new (therefore
      untracked-at-`git diff`-time) files. Clippy (native and
      `--features dev_fake_issuer`), fmt, wasm check (both feature
      states), `mdbook build docs`, `git diff --check` all clean. `bun run
      build`: `index.js` unchanged at 28.6kb, `index_bg.wasm` 1,537,003 →
      1,574,577 bytes (+~36.7KB) — the new migration/handler/db/audit
      code. Digest/cache-version gate passes without a re-pin. No version
      bump.
- [x] **Fifteen smokes green**, including the new
      `smoke:account-recovery-unlink` (nine scenarios: unlink refused with
      no other usable method, row untouched; unlink succeeding with a
      second identity or a recovery credential as the fallback; the
      concurrency race against real D1 leaving exactly one identity
      active; unlink refused for a `Relink`-provenance session and
      redirected-to-reauthenticate for a stale one; consumption of a
      valid code minting an account-tier fresh session; all four
      consumption failure causes generic and identical apart from their
      own per-render CSRF token; and the refused → generate → succeeds →
      consume sequence navigating correctly with application JavaScript
      fully disabled). Zero CSP violations, zero stray processes left
      running.
- [x] **A smoke-fixture bug found and fixed, not a product bug**: the
      first draft of this smoke's five fetch-based `/recovery` consumption
      attempts plus its no-JS scenario's own final attempt — six total —
      all shared the same local `127.0.0.1` client address, silently
      exhausting `Scope::Recovery`'s own five-per-five-minute abuse-limiter
      budget before the sixth (legitimate) attempt ran, producing a false
      failure that read exactly like a product bug. Root-caused by
      reproducing the same sequence in isolation outside the failing
      smoke until a minimal repro appeared, then confirming the fix (a
      distinct synthetic `CF-Connecting-IP` per scenario, TEST-NET-3
      addresses) empirically before trusting it.
- [x] **A pre-existing smoke updated for new, correct behaviour**:
      `smoke:account-link-reauth`'s "link succeeds" scenario previously
      asserted a 303 redirect to `/account`; a first-ever link now
      correctly reveals the newly-issued recovery credential directly
      (200) instead, since a redirect has nowhere to carry the plaintext
      code. Updated to assert the reveal markup is present, plus a new
      check that the credential was issued exactly once.

## RFC-054 A1: the recovery failure message no longer names an expiry that cannot happen (Handoff 060)

- [x] `JA_RECOVERY_INVALID`/`EN_RECOVERY_INVALID` no longer say "or has
      expired" — recovery credentials never expire
      (`migrations/0017_account_recovery_credentials.sql` leaves
      `expires_at` nullable and nothing ever sets it, deliberately, so a
      member with no other way in can always use it). The old text named a
      cause the system cannot produce; a member reading it could conclude
      their code had aged out and go looking for a new one, when the real
      cause is a mistyped or already-used code. The replacement still
      names two possibilities without saying which applies — the
      generic-failure property (RFC-081 §3.2: unknown, consumed, and
      revoked codes are indistinguishable) is unchanged, confirmed by
      reading every branch of `handlers/recovery.rs::post_recovery` and
      finding all five render this same one constant.

## RFC-054 Slice 1: findings B1–B4, plus copy harmony (Handoff 061)

- [x] **B1** — `JA_RECOVERY_BODY`/`EN_RECOVERY_BODY` now say the code was
      *saved earlier by the member*, distinguishing it from an invite or
      relink code (both admin-issued) — `JA_RELINK_BODY` already told the
      member where to look; recovery did not.
- [x] **B2** — `JA_MEMBERSHIP_SUSPENDED`/`EN_MEMBERSHIP_SUSPENDED` gained a
      middle sentence stating the member's other communities are
      unaffected — true by construction (RFC-082 §1: suspension is
      per-membership) and proven by `smoke:membership-suspension`, not
      reassurance but a fact the member has no other way to learn. States
      only that other communities are unaffected — nothing about why the
      member was suspended or by whom.
- [x] **B3** — the one-time-code reveal warning
      (`*_ACCOUNT_RECOVERY_REVEAL_WARNING` and
      `*_ADMIN_INVITES_REVEAL_WARNING`) now says "write down or copy",
      not just "copy" — these pages have no application JavaScript (AD-1),
      so a copy-only instruction may be one the reader cannot follow. Both
      constants converged onto identical wording in both languages; only
      the instruction changed, not what is revealed or for how long.
- [x] **B4** — `JA_RECOVERY_SUBMIT`/`EN_RECOVERY_SUBMIT` now say
      "サインイン"/"Sign in" instead of "続ける"/"Continue" — consuming a
      recovery code signs the member in, exactly as relink already says.
      `JA_ACCOUNT_LINK_SUBMIT` (linking hands off to a provider, it does
      not sign in) correctly keeps "続ける" — a collision, not a
      contradiction, and left alone.
- [x] **Vocabulary convention established: やめる for dismissing a form.**
      The six `*_CANCEL` button labels (English already uniform: `Cancel`)
      had split three ways to やめる and three to キャンセル. Converged the
      three キャンセル constants (`JA_IDENTITY_SIGN_IN_CANCEL`,
      `JA_ACCOUNT_LINK_CANCEL`, `JA_ACCOUNT_UNLINK_CANCEL`) onto やめる.
      キャンセル stays reserved for cancelling an event
      (`JA_ADMIN_CANCEL_EVENT_TITLE` and siblings), where it already means
      something with real consequences for members — the three converged
      constants were all on identity surfaces built most recently, new
      work that hadn't yet matched established vocabulary.
- [x] **Vocabulary convention established: 管理者 / administrator for the
      role**, not 運営者 / operator. `JA_TZ_ERROR`/`EN_TZ_ERROR` was the
      sole outlier — 管理者 appears fourteen times elsewhere in the
      corpus, 運営者 appeared exactly once, in this one constant, in both
      halves. Now matches `MEMBERSHIP_SUSPENDED`, the product's other
      "something is wrong, go ask someone" message.
- [x] **Independently re-derived, not taken on trust**: all 632 `&str`
      constants across the thirteen i18n modules extracted and compared by
      value. Confirmed every specifically-named collision/contradiction
      group and both substring counts (管理者 ×14, 運営者 ×1) exactly.
      Found and fixed an extraction bug in the first pass (two constants
      using Rust's `\`-newline string continuation weren't matched) before
      trusting the corrected 632 total. Could not exactly reconcile the
      handoff's own "23 other identical-text groups" summary figure to a
      single counting method (variously got 21 or 25 JA-only groups
      depending on how a group already mid-convergence is bucketed) —
      noted in the review request as an unreconciled but non-blocking
      count, since every specifically *named* group and constant checked
      out exactly.
- [x] **No smoke assertion needed updating.** Grepped every `.mjs` for
      every old string (full sentences, and the short shared words
      続ける/キャンセル/やめる/サインイン individually) before making any
      edit — zero hits beyond Handoff 060's own already-fixed one. The
      five smokes named as likely to break
      (`smoke:account-recovery-unlink`, `smoke:membership-suspension`,
      `smoke:admin-tools-onboarding`, `smoke:account-link-reauth`,
      `smoke:identity-callback`) and the remaining eleven all passed
      unmodified.
- [x] `cargo test --workspace`: **628**, unchanged. `--features
      dev_fake_issuer`: **631**, unchanged. Clippy (both feature states),
      fmt, wasm check (both states), `mdbook build docs`, `git diff
      --check` all clean. `bun run build`: `index.js` unchanged at
      28.6kb. No version bump, no digest re-pin (no cached asset
      changed). All sixteen smokes green.
- [x] **Review correction (F1): prose naming a button must move with the
      button's own label, not just duplicate-value strings.**
      `JA_IDENTITY_SIGN_IN_FAILED_BODY` named its cancel button as
      「キャンセルして」 in running prose — a different relationship than a
      shared *value*, and outside the value-equality sweep above, so H1's
      button-label rename (キャンセル → やめる) left this sentence pointing
      at a label no longer on the page. Fixed to 「やめて」;
      `EN_IDENTITY_SIGN_IN_FAILED_BODY` correctly left alone, since its
      button is still `Cancel`.

## RFC-083 Slice D1a: admin event surfaces resolve locale (Handoff 062)

- [x] **Nine of the ten scoped files converted**; `require_admin`'s
      `MembershipContext` is bound and its `.locale` threaded through every
      render site: `attendance.rs`, `cancel.rs`, `copy.rs`, `edit.rs`,
      `notes.rs`, `occurrence.rs`, `recreate.rs`, and the two shared helpers
      `forms.rs`/`summary.rs` (both now take `locale: Locale` as a required
      parameter — no `Option`, per Handoff 049's rule). `create.rs` is
      **deferred**, see below.
- [x] **`create.rs` deferred, not converted.** Its "Use a template" link
      (`JA_ADMIN_USE_TEMPLATE_LINK`) has no English half, and the handoff's
      own §5 claim that the other six JA-only constants "belong to D1b and
      D1c" was wrong for this one — it's used only in this file, squarely
      D1a. Converting the rest of the page while leaving that one link
      Japanese-only would trip RFC-083 §12's own stop condition (correct
      `html lang`, wrong-language body text). Left un-migrated exactly as
      before (still passes `Locale::Ja` explicitly to `forms.rs`, since that
      helper's locale parameter is now required); its exception-table entry
      stays at `ja_count: 7`, with the reason field recording the open
      question. Raised to the architect as a review-request finding rather
      than decided here.
- [x] **Two new English strings written**, proposed wording flagged for
      owner review, not treated as settled:
      `EN_ADMIN_ATTENDANCE_SAVED_FLASH` = "Attendance saved." and
      `EN_ADMIN_ATTEND_MEMBER_ARIA_LABEL` = "Attendance for {}" (pairs
      `JA_ADMIN_ATTENDANCE_SAVED_FLASH`/`JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL`,
      both Handoff 036 leftovers). The other six pre-existing JA-only
      constants in Slice D were **not** given English halves — five belong
      to D1b/D1c as the handoff said; the sixth is `create.rs`'s deferred
      link, above.
- [x] **A genuine defect found and fixed by the conversion, not by
      inspection**: `summary.rs`'s schedule card called the non-locale-aware
      `render::format_day_time_tz`, so its date labels ("7月5日（日）")
      would have stayed Japanese on an English-locale edit page even after
      every other string on the card was threaded. Caught by the new
      both-locales rendered-output test below, which failed on first run
      with the Japanese date label as the only leak — not by manual
      review. Fixed by switching to the already-existing
      `format_day_time_tz_localized(day, tz, locale)`. The non-localized
      `format_day_time_tz` and (separately) `render::header_with_switcher`
      (the two-argument, non-`_localized`, non-`_next` wrapper) both became
      dead code once every caller in this package converted to their
      `_localized`/`_next` siblings — both removed, along with their
      `render.rs` re-exports, to keep clippy `-D warnings` green.
- [x] **A rendered-output assertion, not a source scan** (RFC-083 §6.3):
      two new tests in `workers/ssr/src/handlers/admin/events/tests.rs`
      compose real page content — the edit page's header + hint + details-
      only fields (which itself pulls in `summary.rs`'s schedule card), and
      separately the recurring create-fields form (covering the `REPEAT_*`
      group `render_details_only_event_edit_fields` doesn't touch) — at
      `Locale::En`, then scan the *whole* composed string for any codepoint
      in the ranges Japanese text in this codebase actually uses (Hiragana,
      Katakana, CJK ideographs, CJK punctuation, fullwidth forms), not
      against a fixed list of known constants. Each test also renders the
      same composition at `Locale::Ja` and asserts a Japanese codepoint *is*
      present, so the English-side assertion is proven discriminating, not
      vacuous. This is the check whose absence let RFC-072 claim completion
      twice; a source-file gate does not substitute for it.
- [x] **`LOCALIZATION_EXCEPTIONS` shrunk 27 → 18 entries, 308 → 203 sites**
      (independently summed from the table, not taken from the handoff's
      stated 27→17/308→196 — that arithmetic assumed all ten files
      converted; `create.rs` staying put changes both numbers by exactly
      one entry and seven sites). A new pinned shrink-only test,
      `rfc083_localization_exceptions_table_only_shrinks`, asserts both
      numbers exactly (RFC-083 §6.1) — this is the package's first, since
      none existed before.
      `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
      passes with the nine converted files removed from the table.
- [x] Three source-scanning gates in `release_gates.rs`
      (`rfc051_event_edit_semantics_are_details_only_for_multi_day`,
      `rfc060_cancelled_event_recreate_is_admin_only_and_details_only`,
      `rfc066_event_copy_is_admin_reviewed_prefill_not_clone`) asserted the
      literal substring `"JA_ADMIN_..."` for nine constants this package
      moved behind `i18n::t(locale, i18n::...)` — updated to check for the
      bare constant name instead (still a substring of the new call), same
      property, no gate weakened.
- [x] **Admin event route query counts unchanged.** Every file binds
      `require_admin`'s already-returned `MembershipContext` instead of
      discarding it — no new D1 query added anywhere in this package.
- [x] **`require_admin`'s `?` verified intact on every path** in all nine
      converted files (and `create.rs`) — binding the result to `membership`
      instead of `_membership` does not change control flow; every early
      return this handoff touched already existed before this package.
- [x] `cargo test --workspace`: **628 → 631** (+3: the shrink-only gate
      test, and the two both-locales rendered-output tests).
      `--features dev_fake_issuer`: **631 → 634** (+3, same three tests;
      neither depends on the feature). Clippy (both feature states), fmt,
      wasm check (both states), `mdbook build docs`, `git diff --check` all
      clean. `bun run build`: `index.js` unchanged at 28.6kb — no cached
      asset changed, no re-pin.
- [x] **Smokes**: `smoke:admin-event-forms` green (`html lang="ja"` on all
      three admin pages it captures, matching these admin fixtures' NULL
      `ui_language` — Japanese fallback, unchanged). Of `package.json`'s 21
      `smoke:*` scripts, `smoke:runtime` needs an explicit URL argument and
      isn't a self-contained smoke; of the remaining 20, 19 pass clean and
      `smoke:recurrence` fails identically on this checkpoint's own
      unmodified baseline (`bbf804b`, confirmed via `git stash` + rebuild
      before restoring) — two sub-checks
      (`calendar-materializes-rolling-open-ended-series`,
      `far-future-calendar-month-does-not-materialize`) fail regardless of
      this package, most likely date-fixture drift against real wall-clock
      time. Pre-existing, unrelated, not fixed here; disclosed rather than
      silently skipped.
- [x] **Minor, not fixed**: `scripts/smoke/rfc075-slice4-admin-event-forms.mjs`'s
      header comment still says the three pages it captures are "Japanese
      only, by documented decision (RFC-072 Slice D) — no English rendering
      exists for any of these pages." That's now stale for two of the
      three — the attendance and cancel-confirmation pages it screenshots
      are genuinely locale-aware after this package; only the create-event
      form (`create.rs`, deferred above) still matches the comment as
      written. The smoke still passes, since its fixtures' `ui_language` is
      NULL (Japanese fallback). Noted for whoever next touches that script,
      not corrected here — out of this handoff's authorized scope.

## Smoke coverage: derive the run set, fix the wall-clock break (Handoff 063)

**Smoke evidence must now come from `bun run smoke:all`'s summary output, not a
remembered count.** "Sixteen smokes green" had been quoted across several recent
packages — including reviews of Handoffs 060 and 061 — while the real,
runnable set was twenty (and the real total on disk was twenty-four). A
package's evidence section that states a smoke count without `smoke:all`'s
output behind it is not acceptable evidence.

- [x] **A default-fail coverage gate**
      (`every_smoke_script_is_reachable_by_name_or_documented_exception` in
      `release_gates.rs`), same shape as `LOCALIZATION_EXCEPTIONS`: walks every
      `scripts/smoke/*.mjs` file and fails on anything not referenced by some
      `package.json` script value and not listed in the (currently empty)
      `SMOKE_COVERAGE_EXCEPTIONS` table. Demonstrated failing: temporarily
      removed `smoke:invite` from `package.json`, re-ran the gate, got the
      expected failure naming `invite-redemption.mjs`, restored, re-ran green.
- [x] **`bun run smoke:all`** (`scripts/smoke-all.mjs`) runs every self-contained
      smoke and reports total run / total passed / named failures, derived at
      run time from `package.json`, not hand-maintained. Includes
      `test:abuse-controls` explicitly even though its name doesn't start with
      `smoke:` — omitting it on a naming technicality would recreate the exact
      defect this script exists to fix.
- [x] **Three previously-unreachable scripts given `smoke:*` names**:
      `smoke:admin-role-transfer`, `smoke:help-signin`,
      `smoke:member-management`. All three run and pass in full (no failing
      checks); their subject matter (RFC-062 role transfer, RFC-024 relink,
      member-management workflows) is not obviously duplicated by any other
      current smoke, so they stay as their own scripts rather than being
      folded into anything. Their evidence output was scanned with
      `evidence:scan-leakage`; each flagged a pre-existing raw-resource-id
      finding (a community ID captured in a form field snapshot) — not new,
      not fixed here, and no raw evidence copied into any tracked file.
- [x] **`smoke:recurrence`'s wall-clock break fixed.** The far-future Calendar
      month was hardcoded (`2027-02`); `RECURRENCE_MATERIALIZATION_MONTHS_AHEAD`
      (6 months) meant today (2026-08-16) + 6 months landed exactly on that
      literal, so it silently stopped being "far future" roughly two weeks
      before this package. Now derived as today + (horizon + 2 months margin)
      at run time — always outside the window, permanently. The near-future
      month (`2026-09`, with a hardcoded expected materialized date
      `2026-09-25`) had the mirror-image problem; also re-derived, from the
      (now also relative) series start date + 11 weeks, matching the original
      dates' relationship. A cross-language pin
      (`rfc065_recurrence_smoke_pins_the_materialization_horizon_constant`)
      reads the live Rust `RECURRENCE_MATERIALIZATION_MONTHS_AHEAD` and fails
      if the JS literal drifts from it.
- [x] **A separate, pre-existing, unrelated failure remains in
      `smoke:recurrence`**, confirmed present at the pre-Handoff-063 baseline
      too and left untouched (out of §3.3's authorized scope, which covers
      only date derivation): `calendarShowsSeededTitle` — the Calendar
      month-grid view (`render_calendar_month`) never renders event titles at
      all; only `render_calendar_day_detail`, gated on a `day=` selection this
      smoke's navigation never makes, does. Root-caused, not fixed — this
      looks like a pre-existing smoke-authoring assertion that was never
      actually true of the page it's checking, not a product regression.
      Flagged for the architect rather than silently patched or silently left
      unmentioned; `smoke:recurrence` therefore still exits non-zero, on this
      one unrelated check only.
- [x] **`smoke:language` audited for the same wall-clock shape (§12, carried,
      not fixed)** — it does not have the defect. Handoff 042 §7.1–7.3 already
      derives its fixture's event date relative to real run time (`runAt + 3
      days`) and carries its own `assertFixtureStillUpcoming()` self-guard,
      explicitly anticipating this exact class of drift. Confirmed still
      passing.
- [x] `cargo test --workspace`: **631 → 633** (+2: the coverage gate and the
      cross-language pin). `--features dev_fake_issuer`: **634 → 636** (+2,
      same two tests). Clippy (both feature states), fmt, wasm check,
      `mdbook build docs`, `git diff --check` all clean. `bun run build`:
      `index.js` unchanged at 28.6kb — this package touches no product code.
- [x] **No product code changed** — only `package.json`, `scripts/`, and
      `packages/contracts/tests/release_gates.rs`, per §4's explicit
      non-change scope.

## EN/JA parity is now derived from the constants themselves (Handoff 064)

**`en_ja_parity` never checked parity.** Its entire body asserted a literal
230-element array's length against a literal `230`, and that no literal in the
array was itself empty — it never referenced a single `EN_`/`JA_` identifier, so
it would have passed unchanged if every `JA_` constant in the project were
deleted. Replaced with
`en_ja_parity_is_derived_from_the_constants_themselves` in `release_gates.rs`,
which parses every `packages/contracts/src/i18n/*.rs` file (comments stripped,
`\`-newline string continuation handled by character scanning, not a regex that
can forget `DOTALL`) and checks, project-wide: every `EN_` stem has a `JA_`
counterpart and vice versa (or a pinned, reasoned exception); no value is empty
or whitespace-only; and every paired constant's `{...}` placeholders match
between halves. Independently cross-checked (`grep -c`): 313 `EN_` / 319 `JA_`
constants, matching the parser's derived counts exactly.

- [x] `EN_JA_PARITY_EXCEPTIONS` holds exactly the six genuinely-unpaired stems,
      all RFC-083 Slice D scope (five D1b/D1c, one D1a's `create.rs` follow-up).
      **Expected to shrink to empty** as those sub-slices land — a table growing
      back would mean a new pair went in unpaired, not a documented decision.
- [x] Four failure demonstrations, each captured and each file restored
      byte-identical afterward: an EN constant made structurally absent (its
      declaration line removed, its one caller inlined so the crate still
      builds), a value set to `""`, a `{}` placeholder removed from one half of
      a pair, and a stale exception entry naming a stem that isn't actually
      unpaired. All four produced the expected failure message and nothing
      else.
- [x] `en_ja_parity` and its 230-name array deleted from
      `packages/contracts/src/i18n/tests.rs` entirely — not kept alongside the
      new gate.
- [x] **A second, real, hand-maintained parity mechanism was found that the
      handoff did not know about** — see the review request's own section on
      it. Not touched in this package; flagged for a decision.
- [x] `cargo test --workspace`: **633 → 633, net zero** (contracts lib: 85 → 84,
      the deleted `en_ja_parity`; `release_gates`: 109 → 110, the new gate — one
      test disappears, one appears, the totals happen to net out).
      `--features dev_fake_issuer`: **636 → 636**, same composition. Clippy
      (both feature states), fmt, wasm check, `mdbook build docs`,
      `git diff --check` all clean. `bun run build`: `index.js` unchanged at
      28.6kb — no product code touched.
- [x] `bun run smoke:all`: see the review request for the full run; expected
      23/24 with `smoke:recurrence`'s known, unrelated
      `calendarShowsSeededTitle` failure unchanged.

## The leakage scanner reports every violation, and three smokes stopped capturing whole forms (Handoff 065)

**The scanner used to report at most one violation per document** —
`scanJsonValueForLeakage`/`scanTextForLeakage` threw on the first hit and the
caller stopped there. A control whose purpose is to prove absence cannot do
that: fixing one reported violation and re-running only ever reveals the
*next* one, with no way to know how many remain. Both functions now return
**every** violation found (empty array when clean); `assertRedacted` itself
(used during record construction/parsing, which only ever needs to know a
record is invalid, not enumerate every way it is) keeps its original
throw-on-first behavior, unchanged, for every existing caller.

- [x] **Demonstrated exhaustive**, both as a permanent regression test and as
      a real CLI run against a throwaway fixture (deleted afterward, tree
      confirmed clean): a single document with three unrelated violation
      categories reports all three in one scan, not one.
- [x] `scripts/test-evidence-manifest.mjs` updated: the scan-function tests
      now check the returned array for expected categories instead of
      asserting a thrown error, since that is the contract that changed. Two
      new tests pin exhaustiveness itself (one JSON, one free-text) so a
      future regression to first-match-and-stop would be caught here, not
      discovered by hand again.
- [x] **A non-empty findings list still fails the run.** No `--force`,
      `--allow`, or override flag exists or was added.
- [x] **All 67 top-level entries under `.git-exclude/evidence/` scanned**
      (66 directories plus one loose `.log` file). **A much larger finding
      than the originating review anticipated**: 47 of those 67 have at least
      one violation, categories `raw_resource_id`, `raw_or_hashed_secret`,
      `forbidden_key`, and `sql` — not only the three smoke scripts this
      package fixes. Reported in full in the review request (counts and
      categories per directory, no values). **Not fixed, not deleted, not
      rewritten** — per this package's explicit scope, the contaminated
      directories stay exactly as they are pending an owner decision. This is
      its own, separate, much larger finding.
- [x] `scripts/smoke/admin-role-transfer.mjs`, `help-signin.mjs`, and
      `member-management.mjs`: the blanket `values: Object.fromEntries(...)`
      whole-form capture is removed entirely, not narrowed to an allow-list —
      reading every assertion in all three confirmed none of them reads
      field values *or* field names from the captured object; it was pure
      unused dead weight riding along in the evidence JSON. `_token` (a
      64-character single-use form token) was one of the values silently
      captured this way. Re-ran all three smokes (still pass in full) and
      re-scanned their evidence: **rfc062 (admin-role-transfer) 7 → 0,
      rfc024 (help-signin) 5 → 0, rfc061 (member-management) 14 → 0**
      findings.
- [x] `cargo test --workspace` / `--features dev_fake_issuer`: **633 / 636,
      unchanged** — this package touches no Rust.
- [x] clippy, fmt, wasm check, `mdbook build docs`, `git diff --check`,
      `bun run build` (`index.js` unchanged at 28.6kb): all clean.

## The remaining four smokes stop recording form values (Handoff 066)

**The rule, plainly: a smoke may read a form value for an assertion; it may
never record one.** Handoff 065 fixed three scripts where nothing read the
captured values, so deleting the capture was free. Four more scripts
(`event-copy.mjs`, `self-display-name-editing.mjs`, `recurrence-v2.mjs`,
`language-preference.mjs`) do the same blanket capture, but three of them
*do* read values for real assertions — including `self-display-name-editing.mjs`'s
AD-4 single-use-form-token replay coverage. Deleting the capture outright
would have broken those.

- [x] `collect()` no longer returns `values` in any of the four. Where an
      assertion needs a field, it's read explicitly and separately —
      `readFormValue`/`readFormValues` helpers that query only the named
      field(s), never touch `observed`, and are called out by name at each
      call site so a reader can see exactly what the test depends on. The
      evidence path and the assertion path are now different code paths, not
      a shared object filtered after the fact.
- [x] **`self-display-name-editing.mjs`'s four token-replay tests preserved
      exactly** — same reads, same assertions, same `checks:` results, just
      via the explicit read instead of a captured map. Confirmed by re-running
      the smoke and reading its full `checks:` output, not just a green exit
      code.
- [x] Five raw field-map recordings deleted from `observed` entirely
      (`event-copy.mjs` ×4, `recurrence-v2.mjs` ×1) — the booleans in
      `checks:` were already the diagnosable evidence; the raw values added
      nothing but risk.
- [x] All four smokes re-run in full and pass (`smoke:recurrence` still
      exits non-zero only on the separate, already-decided
      `calendarShowsSeededTitle` finding). Re-scanned all four evidence
      directories: **rfc065 2→0, rfc066 11→0, rfc072 25→0**. **rfc070 9→1** —
      the one remaining finding is unrelated to this package (a
      `hosted-staging-smoke-checklist.md` file containing an example
      `wrangler d1 execute` command for a human tester, not anything this
      smoke's JSON output writes) and untouched, per this package's scope.
- [x] **Whole-tree total: 1097 → 1051**, not the handoff's predicted ~200.
      Investigated rather than accepted or silently corrected: most of the
      `_token`/resource-id findings the handoff was counting live in frozen
      historical evidence bundles (`review-040` through `review-060` and
      similar) that already contain old, pre-fix copies of these same
      scripts' output — fixing the scripts stops *future* captures, it
      cannot retroactively clean snapshots already on disk, and this package
      is explicitly not authorized to touch them. See the review request for
      the full reconciliation.

## `forbidden_key` only fires when the value could carry a string (Handoff 067)

**A forbidden field name is a violation only when its value could contain a
string.** A boolean or number can never carry a secret, no matter what the
field is named — `sessionCookieIssued: true` is an assertion result, not a
cookie. `FORBIDDEN_KEY_PATTERN` itself is unchanged; only *when* it's
consulted changed, identically at both call sites
(`collectRedactionViolations` and `assertRedacted`) via a shared
`valueCanCarryString` gate. `null`/`undefined` are exempted for the same
reason; arrays and plain objects are deliberately **not** exempted — a list
literally named `credentials` could hold real ones.

- [x] **110 → 5.** All 105 boolean/number findings clear. The five
      `validCredentialRow` array findings remain, correctly — a real signal
      about a field name, carried rather than chased (renaming it is a smoke
      change for whoever next touches that script).
- [x] Four permanent tests added to `test-evidence-manifest.mjs`, each
      checking `assertRedacted` and `scanJsonValueForLeakage` agree: a
      string-valued forbidden key still fires (the proof the narrowing isn't
      a hole), an array-valued one still fires, a boolean-valued one and a
      number-valued one no longer do.
- [x] Whole-tree total: **1051 → 946**, exactly matching the handoff's
      predicted figure.
- [x] `cargo test --workspace` / `--features dev_fake_issuer`: **633 / 636,
      unchanged** — no Rust touched. clippy, fmt, wasm check,
      `mdbook build docs`, `git diff --check`, `bun run build` (`index.js`
      unchanged at 28.6kb): all clean.

## The leakage scanner is now a gate, with a shrink-only baseline (Handoff 068)

**`bun run test:evidence-leakage-baseline` now runs automatically as part of
`bun run smoke:all`.** Before this, `scan-evidence-leakage` was invoked by
nothing — not CI, not a release gate, not `smoke:all` — only a manual `bun
run evidence:scan-leakage`, which is exactly how 1051 findings accumulated
without anyone noticing until Handoff 065 happened to run it by hand. The
same "control nobody runs" shape this project has now fixed four times
(`LOCALIZATION_EXCEPTIONS`, the smoke run set, EN/JA parity, and now this).

- [x] **The backlog is pinned, not fixed — and it is local test fixtures, not
      business data.** The service has never been deployed; there is no real
      community, no real member, no real session. A consumed single-use
      form token from a finished local test run, or a `com_…` id a smoke
      script created in its own local D1, has never had authority anywhere.
      The scanner's uniform rule (no "it's only local" exception, so nobody
      has to adjudicate a value one at a time) is correct and unchanged —
      this package makes the existing backlog visible and frozen, not
      acceptable.
- [x] **Baseline pinned as exact totals — total 946, and every category
      individually** (`raw_resource_id` 484, `raw_or_hashed_secret` 450,
      `forbidden_key` 5, `sql` 7, plus every currently-zero category pinned
      at 0 too, so a category going from absent to nonzero is caught the
      same as an existing one rising). **The gate fails in both
      directions** — a rise means a new, unreviewed violation; a fall means
      the backlog shrank and the pin is stale and must come down. Both
      directions demonstrated, plus a same-total category swap (proving the
      per-category pins do real work beyond the total alone).
- [x] **The pin comes down as evidence is naturally replaced** — nothing
      about this gate prevents the baseline shrinking; it only requires that
      a shrink be a deliberate re-pin, not a silent one.
- [x] Skipped-extension inventory reported: `.png` (screenshots, hashed not
      content-scanned by design) and `.log` (not yet in the scanned
      extension set — carried, not added here, since adding an extension
      would move the baseline in the same change that pins it).
- [x] No evidence file touched, moved, deleted, or regenerated. No detection
      logic changed (`FORBIDDEN_KEY_PATTERN` and every other rule are
      Handoff 067's territory, untouched here).

## `.log` evidence is scanned, and a coverage widening can re-pin upward (Handoff 069)

**`.log` joined the scanned set.** Verified first, not assumed: all 105
`.log` files under `.git-exclude/evidence/` start with a `$ node ...`
shell-transcript line and none parse as JSON, so `.log` uses the same text
path as `.md`/`.txt`/`.csv`. `.png` stays out, by design, unchanged.

- [x] **Baseline re-pinned upward: 996 total** (`raw_resource_id` 534, up
      from 484 — every other category unchanged: `raw_or_hashed_secret` 450,
      `forbidden_key` 5, `sql` 7). The +50 delta is 50 previously-unscanned
      `.log` files, one `raw_resource_id` finding each, across six
      directories that had read as fully clean under the old, `.log`-blind
      scan. Same known pattern as the rest of the backlog — local `com_…`
      community ids — nothing new in kind. **This reconciles to 50 files,
      not the 43 the prior review estimated** — re-derived independently by
      two methods (the scanner itself, and a separate `grep`), both agreeing
      exactly; the discrepancy is in the earlier estimate, not this
      measurement.
- [x] **The gate's failure messages now name the one legitimate exception**:
      a rise is still a failure to investigate by default — the exception
      applies only when a run deliberately widened what the scanner scans,
      and requires the reason recorded next to the pinned numbers (done,
      inline, in `test-evidence-leakage-baseline.mjs`). A rise with no
      coverage change remains a failure with no re-pin option, demonstrated.
- [x] All four failure directions re-demonstrated after the re-pin — rise
      with no coverage change, fall, same-total category swap, and (new)
      `.log` content actually being scanned — each verified with a direct
      (non-piped) exit-code check, not just the printed message.
- [x] No evidence file touched, moved, deleted, or regenerated; every
      demonstration mutation reverted and independently verified reverted.
      No detection-rule change; no extension other than `.log` added.

## The recurrence smoke reads the list view for the seeded title (Handoff 070 Part A)

**RFC-073 (`ed549be`, 2026-07-29) moved event titles out of the month grid**
into the day-detail panel and the list view; the month grid's only `title`
is the page heading. `calendarShowsSeededTitle` asserted against the month
view was checking a layout that has not shown a title since — the product
was correct, the smoke was stale.

- [x] **The month-view navigation and its three checks (`noHorizontalScroll`,
      `rowCountIncreased`, `materializedThroughNearMonth`) are unchanged** —
      that visit is what triggers materialization, and it is RFC-011
      accessibility coverage for the hardest layout in the product to keep
      scroll-free at 200% text. It was **not** repointed to `&view=list`.
- [x] **A second, separate navigation** to
      `?month=<nearMaterializeMonth>&view=list` was added, with two
      view-named checks: `listViewShowsSeededTitle` (replaces
      `calendarShowsSeededTitle` — not reused for a different page) and
      `listViewNoHorizontalScroll` (new list-view accessibility coverage,
      free once the page is collected).
- [x] `smoke:recurrence` green, including both new list-view checks.
      `bun run smoke:all` — **25 run, 25 passed**, the first fully green run
      in this series.

## `EN != JA` is now part of the derived gate, and the hand-maintained list is gone (Handoff 070 Part B)

`i18n_en_ja_parity_count` was the last hand-maintained i18n list — a
~301-pair array checking non-emptiness and `EN != JA` via a one-entry
`INTENTIONALLY_IDENTICAL` exemption, covering 301 of 313 EN constants
because a new pair only got checked if someone remembered to add it. Same
shape this project has now derived away four times.

- [x] **`EN != JA` folded into
      `en_ja_parity_is_derived_from_the_constants_themselves`**, with its own
      pinned `EN_JA_IDENTICAL_EXCEPTIONS` table (separate from
      `EN_JA_PARITY_EXCEPTIONS`, which names *unpaired* stems, not identical
      ones), a stale-entry assertion, and structural equality only — no
      assertion on what any string *says* (RFC-054 owns wording; RFC-081
      §3.2 and RFC-082 §4 carry deliberate non-disclosure text this gate
      must not touch).
- [x] Measured independently across the whole corpus: **exactly one pair has
      identical halves**, `JOIN_HEADING` (`"ciao.zinnias"` both sides) — the
      product name, seeded as the sole exception with the same reason the
      old list's one-entry exemption carried. A second identical pair is a
      named stop condition, not a second table row.
- [x] `i18n_en_ja_parity_count` (~301-pair hand-maintained array) and
      `cell_label_templates_have_matching_placeholder_counts` (subsumed —
      independently re-verified that all four `CALENDAR_MATRIX_CELL_*`
      constants use `{}`-only placeholders on both sides, so the derived
      gate's multiset placeholder check already covers them) both deleted.
- [x] `EN != JA` now enforced across all **313** EN constants instead of
      301. `cargo test --workspace`: 631 (was 633; −2 exactly matches the
      two deleted tests, no new `#[test]` fn added). `--features
      dev_fake_issuer`: 634 (was 636).
- [x] Six failure demonstrations run against the derived gate, each verified
      to land, captured, then restored byte-identical: the new identical-pair
      check firing, its stale-entry assertion firing, and all four of
      Handoff 064's original cases (unpaired stem, its stale-entry assertion,
      empty value, placeholder mismatch) confirmed still firing — deleting
      the neighbouring hand-maintained test and cell-label test did not
      disturb them.
- [x] No product code touched; no constant added, removed, renamed, or
      reworded. `node scripts/test-evidence-leakage-baseline.mjs` stays
      green at 996 — this package writes no evidence.

## RFC-083 F1: Slice D1a is closed (Handoff 071)

Handoff 062 converted nine of the ten admin event files and correctly
deferred the tenth: `handlers/admin/events/create.rs`'s "Use a template"
link had no English half, and converting the rest of the page while
leaving that one link Japanese-only would have produced `lang="en"` with
Japanese body text — RFC-083 §12's stop condition verbatim.

- [x] **`EN_ADMIN_USE_TEMPLATE_LINK` added** ("Use a template"), paired with
      the existing `JA_ADMIN_USE_TEMPLATE_LINK` as `ADMIN_USE_TEMPLATE_LINK`.
      The only new constant in this package.
- [x] **Both functions in `create.rs` now resolve locale** from
      `require_admin`'s `MembershipContext` — `get_create_event` (five
      former bare-`JA_` sites, including the template link) and
      `post_create_event` (its invalid-timezone error path, which already
      had `membership` in hand and was discarding the locale at render).
      The Handoff 062 deferral comment is deleted, not left in place —
      it is false the moment this lands.
- [x] **`LOCALIZATION_EXCEPTIONS` is now 17 entries / 196 sites** (was
      18/203) — **no `handlers/admin/events/` entry remains in the table**.
      This is what "Slice D1a is closed" means concretely.
      `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
      passes with the file removed from the table, not re-added to force a
      pass.
- [x] A new rendered-output test,
      `admin_create_page_renders_with_no_japanese_codepoint_in_english_locale`,
      composes the header, title, form fields, and (unlike the pre-existing
      recurring-form test) the template link itself, and asserts no
      Japanese codepoint at `Locale::En`, with a `Locale::Ja` discriminating
      half. Demonstrated failing (a deliberately unconverted template link
      caught, with the offending text in the panic output), then restored
      byte-identical.
- [x] Per-route query counts unchanged: `get_create_event` 5/5 `.await`
      sites, `post_create_event` 6/6, before and after — binding a
      previously-discarded value adds no query.
- [x] `smoke:admin-event-forms` unaffected (still asserts `htmlLangJa: true`
      — this smoke intentionally still captures Japanese evidence; only its
      stale header comment and evidence `note` were corrected, not its
      assertions). `bun run smoke:all` at 25/25.
- [x] **Deviation from §4's literal text, disclosed**: adding
      `EN_ADMIN_USE_TEMPLATE_LINK` mechanically pairs the
      `ADMIN_USE_TEMPLATE_LINK` stem Handoff 070 had pinned in
      `EN_JA_PARITY_EXCEPTIONS` as unpaired. The derived gate's own
      stale-entry assertion then fails exactly as designed ("it was paired
      — delete the entry, the table is meant to shrink"). §4 said not to
      touch that table; leaving the now-stale entry in place is not a
      coherent alternative — it fails a gate for a reason unrelated to any
      new wording, as a direct, unavoidable consequence of the work §3.1
      explicitly required. The entry was removed; see the review request
      for the full disclosure.

## RFC-083 Slice D1b: member-administration surfaces resolve locale (Handoff 072)

Five files converted: `handlers/admin/members.rs` (invite generation/revocation,
member list — 30 sites), `help_signin.rs` (17), `role_transfer.rs`
(promote/demote — 10), `suspension.rs` (RFC-082 suspend/unsuspend — 10),
`member_remove.rs` (8). `handlers/templates.rs` and `handlers/export.rs`
(D1c) deliberately untouched.

- [x] **All five files resolve locale** from `require_admin`'s
      `MembershipContext`; no `_membership` remains in any of them.
      `role_transfer.rs`/`suspension.rs` thread it through a `Localized`
      pair carried in their confirm-page config structs (`RoleChangeConfirm`,
      `SuspensionConfirm`), resolved once the locale is known — the
      title/consequence/confirm strings can't be resolved at the call site,
      since `require_admin` hasn't run yet there.
- [x] **The locale-blind-helper trap (§3) was checked explicitly, not just
      the obvious `i18n::JA_` sites**: every `render::bottom_nav(` and
      `render::header_with_switcher_next(` call (both present in all five
      files) converted to its `_localized` sibling. Neither helper was
      deleted — both still have D1c callers (`templates.rs`, `export.rs`);
      clippy confirms no dead-code warning.
- [x] `render::not_found` / `service_unavailable` / `session_expired`
      untouched — no locale threaded into any of them.
- [x] `EN_ADMIN_INVITE_REVOKED_FLASH` added ("Invite code revoked.",
      proposed wording flagged for owner review); `invites_flash_message`
      takes a **required** `Locale`, not `Option<Locale>` — a missed call
      site is a compile error. No other constant added.
- [x] **`LOCALIZATION_EXCEPTIONS` now 12 entries / 121 sites** (was 17/196)
      — the table holds no D1b file. **`EN_JA_PARITY_EXCEPTIONS` now 4**
      (was 5) — `ADMIN_INVITE_REVOKED_FLASH` removed now that it's paired,
      leaving D1c's four stems.
      `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
      passes with all five files genuinely removed.
- [x] Two rendered-output tests added (`admin_members_page_...`,
      `admin_help_signin_page_...`), each composing the **header and nav**
      specifically — the exact leak class §3 warns about — plus row/body
      labels, asserting no Japanese codepoint at `Locale::En` with a
      `Locale::Ja` discriminating half. Demonstrated failing **via a
      helper** (temporarily swapped `bottom_nav_localized` for the
      locale-blind `bottom_nav` in the test), catching the leaked Japanese
      nav bar; restored byte-identical.
- [x] **The ROADMAP English-default tripwire gate added**
      (`roadmap_english_default_tripwire_fires_when_slice_d_completes`):
      fails when `LOCALIZATION_EXCEPTIONS` narrows to exactly the three
      structurally-unresolvable entries ROADMAP.md names. A set-equality
      check against fixed path strings, not a count — not satisfiable by
      re-pinning a number. Demonstrated both ways: passes today (12
      entries, nine more than the trigger), and fires when the table is
      temporarily reduced to exactly the trigger set; restored
      byte-identical.
- [x] Per-function `.await` counts unchanged across all eleven converted
      routes (verified against the checkpoint, function by function) — no
      new query anywhere.
- [x] `smoke:admin-member-management`, `smoke:admin-tools-onboarding`, and
      `bun run smoke:all` all green — **25/25**, the standing figure.
- [x] `cargo test --workspace`: 635 (was 632, +3 — two render-assertion
      tests plus the tripwire gate). `--features dev_fake_issuer`: 638 (was
      635, +3).
- [x] No Japanese wording changed; `node scripts/test-evidence-leakage-baseline.mjs`
      stays green at 996.

## The localization gate now catches a locale-blind helper, not just bare `i18n::JA_` (Handoff 073)

D1b's review (F1) found that nothing stopped a converted file from calling
`render::bottom_nav`/`render::header_with_switcher_next` directly — a page
with the correct `html lang`, an English body, and a **Japanese navigation
bar**, with every gate (including D1b's own new rendered-output tests,
which compose the page themselves rather than invoking the handler) green.

- [x] **A converted file may not call a locale-blind render helper** — any
      `render::<name>(` where `<name>_localized` also exists. The helper
      set is **derived**, not listed: scans `workers/ssr/src/render.rs`/
      `render/*.rs` for `pub fn <name>_localized` with a bare `pub fn
      <name>` sibling, comments stripped first. Today that derives
      `{bottom_nav, header_with_switcher_next, page}` —
      `header_with_switcher_localized`/`format_day_time_tz_localized` are
      correctly excluded (no bare sibling), not by name.
- [x] `LocalizationException`'s `calls_bare_page: bool` is gone, replaced
      by `bare_helper_calls: usize` — an exact pinned count, same
      discipline as `ja_count`. Non-excepted files must be 0; excepted
      files pin their exact count (`export.rs`/`templates.rs`: 3 each; the
      other seven table entries: 0–2).
- [x] Confirmed bare `render::page(` in a non-excepted file still fails —
      the property this check has always caught is unchanged by the
      rewrite, verified with its own mutation.
- [x] Five demonstrations, all mutated/verified/restored byte-identical:
      the exact §2.1 defect (`bottom_nav_localized` → `bottom_nav` in
      `members.rs`); a different helper
      (`header_with_switcher_next_localized` → `header_with_switcher_next`
      in `help_signin.rs`) — proving the derived set, not one hard-coded
      name; the pinned-count assertion firing on a partial edit to an
      excepted file (`export.rs`, 3→4); the derivation being live (a
      throwaway `zz_demo`/`zz_demo_localized` pair entering the checked
      set, confirmed via instrumented output, then removed); and the
      ROADMAP tripwire's subset refinement firing when the table is
      reduced to a **proper subset** of the trigger (two of three), which
      the prior equality-only check would have missed.
- [x] The tripwire (`roadmap_english_default_tripwire_fires_when_slice_d_completes`)
      now fires on `remaining ⊆ trigger`, not `remaining == trigger` — a
      future RFC threading `render/errors.rs` (RFC-083 §4.4 considered and
      rejected this, but did not forbid it) would otherwise shrink the
      table to a proper subset and leave the decision silently overdue.
      Everything else unchanged: no numeric literal, the ROADMAP
      reference, "not a gate to re-pin."
- [x] No product code, no conversion work — this package touches only
      `packages/contracts/tests/release_gates.rs`. `bottom_nav` and
      `header_with_switcher_next` were not removed (D1c still needs them);
      confirmed no clippy dead-code warning.
- [x] `cargo test --workspace`/`--features dev_fake_issuer`: unchanged at
      635/638 — this package strengthens two existing gates, it adds no
      new `#[test]` fn. `bun run smoke:all` 25/25; evidence baseline 996.

## RFC-083 Slice D1 is closed: templates and export resolve locale (Handoff 074)

The last two admin surfaces converted: `handlers/templates.rs` (15 sites)
and `handlers/export.rs` (8 sites). **This was the first slice enforced end
to end** by Handoff 073's gate — both a file's `i18n::JA_` count and its
locale-blind-helper count had to reach zero independently.

- [x] Both files resolve locale from `require_admin`'s `MembershipContext`;
      no `_membership` remains in either.
- [x] **Four new English strings added**, all proposed wording flagged for
      owner review: `EN_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH` ("Enter a
      title."), `EN_ADMIN_TEMPLATE_SAVED_FLASH` ("Template saved."),
      `EN_ADMIN_TEMPLATE_DELETED_FLASH` ("Template deleted."), and
      `EN_ADMIN_EXPORT_SUMMARY_COUNTS` ("{events} events · {members} active
      members" — restores the exact pre-Japanese-only English phrasing the
      Handoff 036 comment already documented, rather than inventing new
      copy). **These were the corpus's last four JA-only constants.**
- [x] `ADMIN_EXPORT_SUMMARY_COUNTS`'s English half keeps both named
      placeholders (`{events}`, `{members}`) — substitution is by name, so
      English word order was free; demonstrated: dropping the `{members}`
      substitution step leaves a literal `{members}` surviving into
      rendered output, caught and restored.
- [x] `templates_flash_message` takes a **required** `Locale`, following
      `calendar_flash_message`/D1b's `invites_flash_message`; both stale
      "no locale to resolve" doc comments corrected.
- [x] **All three dimensions re-pinned**: `LOCALIZATION_EXCEPTIONS` 12→10
      entries, 121→98 `ja_count` sites, 14→8 `bare_helper_calls` — the
      last of which never had its own table-level shrink-only total before
      this package; added one. **`EN_JA_PARITY_EXCEPTIONS` is now empty**
      — confirmed inert (the stale-entry loop iterates zero times; no
      other assertion requires non-emptiness) and left in place as the
      table Slice D2/D3's next unpaired stem will land in.
- [x] **`render::bottom_nav` and `render::header_with_switcher_next`
      removed** (D1b was told to keep them for exactly this moment) along
      with their `render.rs` re-exports, after confirming zero remaining
      callers repo-wide. The derived helper set (Handoff 073) shrank to
      `{page}` **with no gate edit** — confirmed live via instrumented
      output, not assumed. The remaining 8 pinned `bare_helper_calls`
      sites are, verified directly, all bare `render::page` calls in D2's
      seven files.
- [x] Rendered-output tests added for both pages, each covering header,
      nav, and body at `Locale::En` with a `Locale::Ja` discriminating
      half on header/nav; the export test additionally asserts neither
      placeholder survives literally. Demonstrated failing in both
      classes Handoff 073 made separate: a bare `i18n::JA_` site
      (temporarily substituted into a test's own composition), and a
      locale-blind helper (the two removed helpers were temporarily
      reintroduced solely to prove the assertion still catches this class,
      then removed again — byte-identical to the post-removal state).
- [x] Export-token security (§9) verified by diff, not assumed: the
      entire change to `export.rs` is confined to `get_export_page`
      (the landing page). `get_export_json` — token consumption,
      `require_admin`, the pre-disclosure audit write, and `build_export`'s
      payload — has zero diff.
- [x] Per-function `.await` counts unchanged across all 7 routes touched
      (verified against the checkpoint, function by function).
- [x] `smoke:admin-tools-onboarding` and `bun run smoke:all` green —
      **25/25**. `cargo test --workspace`: 638 (was 635, +3 — two
      rendered-output tests plus the placeholder test). `--features
      dev_fake_issuer`: 641 (was 638, +3). Evidence baseline unchanged at
      996.
- [x] **RFC-083 Slice D1 is now finished.** `LOCALIZATION_EXCEPTIONS`
      holds exactly the three structurally-unresolvable files and D2's
      seven — every admin surface honours the member's language
      preference. Remaining convertible work: D2a (four anonymous routes)
      and D2b (account surfaces, its own RFC).

## RFC-083 Slice D2a: anonymous routes gain a header-negotiated locale (Handoff 075)

The four anonymous/redemption routes — `join.rs`, `relink.rs`,
`recovery.rs`, `identity/mod.rs` — have no membership to read a stored
preference from (rung 1 never applies here). RFC-083 §8.1's ladder's rung
2, `Accept-Language` negotiation, is new to this package.

- [x] **`negotiate_accept_language` added to `packages/contracts`**,
      deliberately separate from `Locale::parse`: the parse function stays
      a closed two-code allow-list for *stored* values (must fail closed);
      the new function is a lenient RFC 7231/9110-shaped negotiator for a
      *header*, where rejecting `en-US`/`ja-JP`/`en-GB` is the ordinary
      case. Algorithm: split on `,`, examine at most **10** entries
      (`ACCEPT_LANGUAGE_MAX_ENTRIES` — generous headroom against an
      attacker-controlled unbounded header; real browsers send one to
      three); missing `q` defaults to `1.0`; a malformed or out-of-range
      weight discards only its own entry, never defaults it; `q<=0` is
      never selected; stable descending-weight sort preserves header order
      on ties; primary subtag (before the first `-`), lowercased, tried
      against `Locale::parse`; first match wins; no match returns `None`
      and the caller falls to the Japanese floor (rung 3).
- [x] `authz::resolve_anonymous_locale(&req)` reads `Accept-Language`,
      negotiates, and falls back to `Locale::default()` — never returns
      unresolved. Called at the top of every handler in the four files
      (`identity/mod.rs`'s `get_start`/`get_callback` resolve it
      unconditionally: this file never calls `require_membership`, so a
      raw `AuthContext` from `require_auth` never carries a resolvable
      stored preference — rung 2 is unconditionally correct throughout).
- [x] All `i18n::JA_*` bare sites in the four files converted to
      `i18n::t(locale, i18n::*)`; every `render::page` call converted to
      `render::page_localized`. `LOCALIZATION_EXCEPTIONS` re-pinned on all
      three dimensions: 10→**6** entries, 98→**54** `ja_count`, 8→**3**
      `bare_helper_calls`. The tripwire gate still passes (6 entries is
      not a subset of the 3-element trigger).
- [x] `account/link.rs` (D2b, out of scope) adapts to
      `start_oidc_transaction`'s new trailing `locale: Locale` parameter
      with a literal `Locale::Ja` — its unchanged current behaviour, not a
      new D2b resolution decision; the explanatory comment sits above the
      call, not inline within the argument list (an inline comment there
      breaks `prompt_login_is_sent_for_link_and_reauthentication`'s raw
      argument-list parsing).
- [x] **§6.2 oracle re-verification, checked not assumed**: no `if
      locale`/`match locale` branch exists anywhere in the four files —
      locale only ever flows into rendering, never into a control-flow
      decision, so the set of possible response shapes for a given
      `Accept-Language` is identical regardless of a code's validity.
      RFC-081 §3.2's generic-failure property holds in English too: every
      failure branch in `relink.rs`/`recovery.rs`/`join.rs` (format,
      form-replay, abuse-control block, no-valid-code, claim-lost) routes
      through the *same* single generic constant
      (`RELINK_INVALID`/`RECOVERY_INVALID`/`JOIN_CODE_HINT`), and each
      constant's English half is a pre-existing, equally generic message
      (never naming consumed/revoked/expired), not a new per-cause string.
      RFC-076's `/join` response-isolation property (the same markup
      renders on success and on every rejection reason) is unaffected —
      only which language populates that markup changed, not its shape.
- [x] **§6.1 verified**: the raw `Accept-Language` value is consumed
      inline inside `resolve_anonymous_locale` and never escapes it —
      confirmed by grep that no `console_log!`/`audit::*` call in any of
      the four files references the header or the resolved locale.
- [x] **§6.3 — new gate**:
      `anonymous_routes_rely_on_the_default_no_store_cache_control` asserts
      none of the four files ever writes its own `Cache-Control` header,
      relying instead on `lib.rs:281`'s default `no-store`. Demonstrated
      failing by adding `Cache-Control: public, max-age=60` to
      `relink.rs`'s `redirect` helper; restored, byte-identical.
- [x] Rendered-output tests added for two pages (`join.rs`: code form and
      profile form; `relink.rs`: the form) at `Locale::En` with a
      `Locale::Ja` discriminating half, following the established
      Response-avoidance precedent (compose body pieces via direct
      `i18n::t` calls, never invoke the `Result<Response>`-returning
      function itself). Both failure classes demonstrated: a bare
      `i18n::JA_` site (substituted into `join.rs`'s own test
      composition); a locale-blind helper — since these four pages have no
      nav/header helper to swap within a unit test (their only helper is
      `render::page`/`render::page_localized`), demonstrated instead via
      the source-scanning gate
      `rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
      by temporarily reverting `relink.rs`'s final line to bare
      `render::page`; both restorations confirmed byte-identical.
- [x] `negotiate_accept_language`'s own tests (14 total: 4 pre-existing
      `Locale::parse` tests plus 10 new) demonstrated failing: loosened the
      `q<=0.0` rejection to `q<0.0`, confirmed
      `q_zero_on_an_otherwise_matching_tag_is_never_selected` caught the
      regression, restored byte-identical.
- [x] Per-function `.await` counts unchanged across all five touched files
      (`join.rs`, `relink.rs`, `recovery.rs`, `identity/mod.rs`,
      `account/link.rs`), verified against the checkpoint.
- [x] `cargo test --workspace`: 652 (was 638, +14 — 10 negotiation tests,
      2 `join.rs` render tests, 1 `relink.rs` render test, 1 no-store gate
      test). `--features dev_fake_issuer`: 655 (was 641, +14). Evidence
      baseline unchanged at 996.
- [x] **§13 stop condition triggered: `bun run smoke:all` was 21/25.**
      `smoke:invite`, `smoke:help-signin`, and `smoke:account-link-reauth`
      each failed one scenario that asserts a literal Japanese substring on
      a generic-failure page; `smoke:admin-tools-onboarding` failed its
      `htmlLangJa` check on `/join` and `/relink`. Root cause: the
      sandboxed headless Chromium these smokes launch sent no explicit
      `Accept-Language`, so Chromium's own en-US-derived default negotiated
      these routes to English — rung 2 working as designed, but four smoke
      assertions written before negotiation existed hardcoded the old
      Japanese-always behavior. Correctly reported rather than resolved
      unilaterally — see Handoff 076 immediately below, landed in the same
      commit, which fixes the underlying ambient-state dependence.

## Pin the smoke `Accept-Language`, and prove rung 2 end to end (Handoff 076)

Handoff 075's stop condition (`smoke:all` 21/25, above) traced to a defect
one level under the four failing assertions: **no smoke pinned
`Accept-Language` anywhere**, so the suite's result depended on the
developer machine's `LANG` — a developer with `LANG=ja_JP.UTF-8` would have
seen 25/25 and never learned rung 2 existed. Fixing the four assertions
without fixing that would have left the same ambient-state class of defect
Handoffs 063–070 spent five packages eliminating from the recurrence smoke.

- [x] **`scripts/lib/smoke-locale.mjs` added** — a single exported
      constant, `SMOKE_ACCEPT_LANGUAGE = 'ja'`, the one source of truth for
      every smoke's pinned header. Mechanism chosen: CDP
      `Network.setExtraHTTPHeaders`, not a Chromium `--lang` flag —
      empirically confirmed (`--lang=ja` against a local echo server) that
      `--lang` does **not** change the `Accept-Language` header on this
      headless Chromium build, while `setExtraHTTPHeaders` does and is
      already how every smoke sets its session `Cookie` header, so the two
      are merged into one call rather than added as a second one.
- [x] **All 23 smoke scripts that launch Chromium updated** to send the
      pinned header on every page they open — merged into an existing
      `Cookie` header call where one existed (13 scripts), added to an
      `if`/`else` anonymous-page branch (5 scripts: `help-signin.mjs`,
      `invite-redemption.mjs`, `session-provenance-and-community-binding.mjs`,
      `rfc075-slice6-admin-tools-and-onboarding.mjs`,
      `rfc075-slice7-final-migration.mjs` — the latter two had no `else`
      branch at all, so one was added), or added fresh where no header call
      existed at all (5 no-JS single-page scripts:
      `account-link-and-reauthentication.mjs`, `account-surface.mjs`,
      `external-identity-callback.mjs`, `membership-suspension.mjs`,
      `account-recovery-and-unlink.mjs`). Every smoke now depends on this
      input, not just the four that failed.
- [x] **F1 of this handoff's own review: the first sweep missed
      `account-recovery-and-unlink.mjs`.** It launches Chromium and
      navigates to `/recovery` (one of the four routes Handoff 075 just
      localized) but was left unpinned — latent, not benign, since a stale
      Japanese-literal assertion on that script's *account* surface
      (`account/unlink`, still D2b/hardcoded-Japanese) only happens to hold
      today; it would fail on an `en_US` machine the moment D2b makes that
      surface locale-aware too. This is the sixth time in this series a
      hand-executed sweep missed a member of its own population
      (`LOCALIZATION_EXCEPTIONS`, the smoke run set, the parity stem list,
      the identical-pair array, the locale-blind helper check, now this).
      Fixed by adding the import/merge to the missed script, **and** by
      deriving the population instead of sweeping it by hand: a new gate,
      `every_chromium_smoke_pins_accept_language_or_documented_exception`
      (mirroring Handoff 063's `every_smoke_script_is_reachable_by_name_or_documented_exception`
      shape exactly — a directory walk plus a pinned, empty-by-design
      exception table), fails on any `scripts/smoke/*.mjs` that launches
      Chromium without importing `scripts/lib/smoke-locale.mjs`.
      Demonstrated failing: removed the import from
      `account-recovery-and-unlink.mjs`, confirmed the gate named that
      exact file, restored byte-identical. Re-ran
      `smoke:account-recovery-unlink` under both `LANG=en_US.UTF-8` and
      `LANG=ja_JP.UTF-8` — identical passing result both times.
- [x] **Ambient-independence demonstrated, not assumed**: ran
      `smoke:invite`, `smoke:help-signin`, `smoke:account-link-reauth`, and
      `smoke:admin-tools-onboarding` under both `LANG=en_US.UTF-8` (this
      machine's default) and `LANG=ja_JP.UTF-8` — identical passing result
      both times, for all four.
- [x] **Rung 2 proven end to end** — nothing before this exercised header
      → rendered page; the unit tests proved the negotiation function
      alone, the render tests proved composition alone. Added
      `join-negotiates-english-via-accept-language` to
      `scripts/smoke/invite-redemption.mjs` (a scenario in an existing
      script, not a new one — `smoke:all` stays 25/25): pins
      `Accept-Language: en` on a fresh anonymous page, navigates to
      `/join`, and asserts the body contains no Japanese codepoint,
      `html lang="en"`, and — making RFC-083 §6.2's oracle property
      observable rather than argued — the **same form shape** (action,
      method, sorted field names) as the Japanese render captured earlier
      in the same run. Demonstrated failing: temporarily pinned
      `Accept-Language: ja` for this scenario instead (simulating rung 2
      resolving to Japanese regardless of the header), confirmed
      `htmlLangEn`/`noJapaneseCodepoint` both caught it, restored
      byte-identical.
- [x] **A trap found and avoided**: the new scenario's first draft included
      an `observed: {...}` diagnostic field, which
      `rfc076_one_time_invite_response_isolation_is_pinned` explicitly
      forbids as a source-level marker in this file (a past incident
      pattern for plaintext-sensitive leakage) — caught immediately by
      `cargo test --workspace`, removed; the `checks` booleans and
      screenshot already carry everything needed.
- [x] `scripts/smoke/rfc075-slice6-admin-tools-and-onboarding.mjs`'s stale
      comment and evidence `note:` (*"anonymous with no locale to
      resolve"*) corrected to describe the pinned-header arrangement; the
      other three affected scripts checked for the same claim — none
      found.
- [x] **No assertion weakened** — every fix pins the input the assertions
      already depended on; none was changed to accept either language.
      **No product code touched** — every change this handoff made is
      under `scripts/`; Handoff 075's `workers/ssr/src/` and
      `packages/contracts/src/` changes stand exactly as approved.
- [x] `cargo test --workspace`: 653 (was 652, +1 — the new derived gate).
      `--features dev_fake_issuer`: 656 (was 655, +1). Evidence baseline
      unchanged at 996. `bun run smoke:all`: **25 run, 25 passed.**

## RFC-084: the account tier resolves a locale — the localization programme's last convertible work (Handoff 084)

RFC-083 §4.2 deferred D2b — `account/mod.rs`, `account/link.rs`,
`account/unlink.rs` — because the account tier is authenticated but never
community-scoped: a signed-in member reaches it holding zero, one, or
several `ui_language` values, possibly disagreeing. RFC-084 (accepted
2026-08-16) resolved this: rung 1 (a stored preference) outranks rung 2
only when it *resolves* — a member with no expressed preference, or
disagreeing ones, has not made a choice for rung 2 to override.

- [x] **The resolution rule (RFC-084 §3/§3.1)**: rung 1 resolves only when
      every present membership's `ui_language` collapses to exactly one
      distinct, valid [`Locale`] — collect the parseable values (`NULL` or
      malformed treated as no expressed preference, same fail-closed
      contract as `Locale::parse` and `db::membership::resolve_locale`'s
      single-membership precedent), resolve if that set has exactly one
      member, else fall through. Implements the **distinct-set** rule, not
      RFC-084 §4A's literal "every membership agrees" wording — a member
      with `en` in one community and no preference in another has made one
      unambiguous choice, and a strict reading would ignore it:

      | Memberships | Resolves to |
      |---|---|
      | none | rung 2 |
      | all `NULL` | rung 2 |
      | `en`, `NULL` | **`en`** |
      | `en`, `en` | `en` |
      | `en`, `ja` | rung 2 (disagreement) |

      `resolve_account_locale_from_memberships` (`packages/contracts/src/locale.rs`)
      implements this purely; `authz::resolve_account_locale` composes it
      with rung 2/3 via the existing `resolve_anonymous_locale`. Six new
      unit tests cover every row above plus a malformed-value case.
      Demonstrated failing: removed the disagreement early-return, confirmed
      the `en`/`ja` test caught it, restored byte-identical.
- [x] **The query design (RFC-084 §4/§7)**: `CommunitySummary` — read by
      23 other call sites through `list_communities_for_user` — is
      **unchanged**, never widened to carry a language value none of them
      want. A sibling, `db::membership::list_communities_with_locale_for_user`,
      wraps it in a new `CommunityLocaleRow { summary, ui_language }`
      instead of duplicating its fields. `account/mod.rs` calls the
      sibling **instead of** `list_communities_for_user` — one query,
      swapped, not added, serving both the community list and rung 1 at no
      extra cost. `account/link.rs` and `account/unlink.rs`, which made
      zero membership queries before, each pay one new query per route
      (RFC-084 §10 decision 2 — internal consistency preferred over the
      cheaper rung-2-only alternative).
- [x] **Query counts, verified against the checkpoint**: `account/mod.rs`
      **unchanged** (6→6 `.await`, whole file) — the sibling genuinely
      replaced the old call. `account/link.rs` **+2** (6→8: `get_link` and
      `post_link` each +1). `account/unlink.rs` **+2** (7→9: `get_unlink`
      and `post_unlink` each +1). `account/recovery.rs` and
      `handlers/identity/mod.rs` **unchanged** (5→5, 23→23) — both only
      thread a `&Request` through to `render_account_page`'s own internal
      resolution, no query of their own. No other file's count moved.
- [x] `account/link.rs::post_link`'s `Locale::Ja` D2b placeholder (Handoff
      075) replaced with the resolved value; `prompt_login_is_sent_for_link_and_reauthentication`
      updated from asserting the literal to asserting a real resolution
      call is present and neither literal survives.
- [x] All `i18n::JA_ACCOUNT_*` bare sites in the three files converted to
      `i18n::t(locale, i18n::ACCOUNT_*)`; every remaining `render::page`
      call in the corpus converted to `render::page_localized`. No new
      copy — every `EN_ACCOUNT_*` half already existed, unpaired, since the
      original RFC-072 Slice D deferral; only `Localized` pairs were added
      (plus one pre-existing unpaired stem outside `account.rs`,
      `IDENTITY_SIGN_IN_LINK`, needed by the freshness banner's re-sign-in
      link).
- [x] **`LOCALIZATION_EXCEPTIONS` re-pinned on all three dimensions**:
      6→**3** entries, 54→**23** `ja_count`, 3→**0** `bare_helper_calls` —
      independently re-derived against the checkpoint before touching the
      table, matching exactly. The three remaining entries are precisely
      the structurally-unresolvable set (RFC-083 §4.4); nothing left is a
      deferred decision. `EN_JA_PARITY_EXCEPTIONS` untouched — no new
      constant was introduced.
- [x] **Rendered-output test added for `account/mod.rs`** (the required
      "at least two," satisfied per this and Handoff 075's join.rs/
      relink.rs render tests) at `Locale::En` with a `Locale::Ja`
      discriminating half — composes the real `render_body` function
      directly (sync, no DB, unlike `render_account_page`), not a
      hand-rolled copy. Both failure classes demonstrated: a bare
      `i18n::JA_` site (temporarily reintroduced into `render_freshness`'s
      production code, not just the test — this page has no nav/header
      helper a unit test could swap, so a real production-code mutation is
      the closer proof); a locale-blind helper — via the source-scanning
      gate, temporarily reverting `render_account_page`'s final line to
      bare `render::page` (same precedent as Handoff 075's `relink.rs`
      demonstration, for the same no-nav/header page shape). Both restored,
      confirmed byte-identical.
- [x] **§10 security properties verified against the implementation**:
      the last-credential guard's refusal (`identity_db::unlink_required`
      returning `false` for *any* cause — not found, wrong user, or the
      usable-method guard) funnels through the single call site rendering
      `i18n::t(locale, i18n::ACCOUNT_UNLINK_REFUSED)` — the only refusal
      message in the file, so no cause can produce a different one in
      either language; both halves read directly, both pre-existing and
      equally generic. The one-time reveal's condition
      (`reveal.map(render_recovery_reveal)`, called only when
      `Some(code)`, and the `Cache-Control: no-store, private —
      Referrer-Policy: no-referrer` headers gated on `reveal.is_some()`)
      is untouched — only a `locale` parameter was threaded through the
      text it renders, never the condition governing whether or how long
      it renders.
- [x] **A genuine finding, not anticipated by the handoff**: this package
      converted the *last* remaining caller of the bare `render::page`
      helper anywhere in the codebase (confirmed by a repo-wide grep
      finding none) — `render::page` and its private `shell` helper are
      now dead code. Kept, not removed (`#[allow(dead_code)]`, with a
      comment explaining why), since the handoff's own scope said "no
      helper is removed by this package"; whether to delete them is left
      for an explicit future decision. This also touched
      `render/shell.rs`'s tracked content (comments only), tripping
      `cached_asset_content_matches_pinned_hash` — re-pinned the hash
      alone, per that gate's own documented precedent (Handoff 040 §7.3:
      "re-pin whenever content changes; the cache key and version move at
      release, not per package") — no version bump, cache-buster, or
      `sw.js` `CACHE_VERSION` change, since none was warranted for a
      comment-only, non-functional change mid-cycle.
- [x] `smoke:account-surface`, `smoke:account-link-reauth`,
      `smoke:account-recovery-unlink` each pass standalone; `bun run
      smoke:all`: **25 run, 25 passed** — no pinned Japanese assertion
      flipped language.
- [x] `cargo test --workspace`: **660** (was 653, +7 — six
      `resolve_account_locale_from_memberships` unit tests, one
      `account/mod.rs` render test), of which **659 pass and 1 fails**.
      `--features dev_fake_issuer`: **663** (was 656, +7), 662 pass / 1
      fails. **The one failure in both is
      `roadmap_english_default_tripwire_fires_when_slice_d_completes`,
      firing exactly as this handoff's §6 requires** — reported as the
      expected outcome below, not a defect.
- [ ] **The tripwire has fired — ROADMAP.md's English-default decision is
      now due.** `LOCALIZATION_EXCEPTIONS` reached exactly the three
      structurally-unresolvable entries, and the tripwire's full message:

      > LOCALIZATION_EXCEPTIONS now holds only entries within the three
      > structurally-unresolvable paths (["render/errors.rs",
      > "handlers/calendar.rs", "handlers/communities.rs"]) — RFC-083
      > Slice D has reached completion. ROADMAP.md's "The default language
      > flips to English when Slice D completes" (owner decision
      > 2026-08-16, RFC-083 §8.2) is now due: flip `Locale::default()` to
      > English and update migration 0011_membership_ui_language.sql's
      > comment. This is not a gate to re-pin — resolve the ROADMAP
      > decision, then delete or rewrite this test to reflect the new
      > default.

      Per §6/§15: **not re-pinned, not deleted, no exception added.** The
      owner resolves the ROADMAP decision; that resolution — and deleting
      or rewriting this test — is its own package, not this one.

## RFC-085: the locale fallbacks are now three named answers, not one (Handoff 085)

`impl Default for Locale` answered three different questions with one
value: a member's unexpressed preference, a corrupt stored `ui_language`,
and an unmatched `Accept-Language`. All three happened to be Japanese, so
the conflation was invisible — until ROADMAP.md's now-due English-default
flip, which would have moved the fail-closed answer too, as a side effect
of a product decision. This package separates them before that flip is
taken.

- [x] **`impl Default for Locale` deleted.** Two named associated
      constants replace it, both still `Locale::Ja` — **no value changed**:
      - `Locale::PRODUCT_DEFAULT` — the answer when nothing was expressed
        (a `NULL` `ui_language`, or `Accept-Language` matching nothing).
        This is the one ROADMAP.md's decision moves.
      - `Locale::FAIL_CLOSED` — the answer for a stored value outside
        migration `0011`'s `CHECK` allow-list, reachable only by manual
        repair. Must never move when `PRODUCT_DEFAULT` does — that is the
        package's whole purpose.
- [x] `db::membership::resolve_locale` rewritten from
      `stored.and_then(Locale::parse).unwrap_or_default()` to a `match`
      naming which answer each of the three inputs gets:
      `None → PRODUCT_DEFAULT`, `Some(valid) → that locale`,
      `Some(corrupt) → FAIL_CLOSED`.
- [x] `authz::resolve_anonymous_locale`'s rung 3
      (`.unwrap_or_default()`) renamed to `.unwrap_or(Locale::PRODUCT_DEFAULT)`;
      its doc comment updated from "falls through to `Locale::default()`"
      to name the constant explicitly.
- [x] **A new gate, `locale_never_regains_a_default_impl`**, asserts
      `impl Default for Locale` does not exist in `locale.rs` — comments
      stripped first, matched on the specific `impl Default for Locale`
      shape rather than the bare word `Default`, so `Locale`'s own
      `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` and every unrelated
      `#[derive(Default)]` elsewhere cannot trip it. Demonstrated failing:
      temporarily reintroduced the impl, confirmed the gate caught it,
      restored byte-identical.
- [x] **The re-merge guard** (§6.4's central assertion,
      `resolve_locale_corrupt_value_arm_references_fail_closed_not_product_default`):
      a source-scanning gate, not a behavioral one — both constants are
      `Locale::Ja` today, so `resolve_locale(None)` and
      `resolve_locale(Some("corrupt"))` currently produce identical
      *output* regardless of which named constant the corrupt-value arm
      actually references. This gate scans which one it references,
      catching the exact mistake a value-only comparison cannot: someone
      later pointing the corrupt-value arm at `Locale::PRODUCT_DEFAULT`
      instead of `Locale::FAIL_CLOSED`. `resolve_locale`'s own unit tests
      updated to assert against the named constants
      (`Locale::PRODUCT_DEFAULT`/`Locale::FAIL_CLOSED`) rather than the
      bare `Locale::Ja` they used before, for the same reason.
- [x] A pre-existing gate,
      `rfc072_locale_resolution_never_panics_on_a_bad_stored_value`,
      checked for the literal string `unwrap_or_default()` that this
      package removed — updated to check for `Locale::FAIL_CLOSED`
      instead, preserving its actual purpose (parse-or-fall-back, never
      assume a valid stored value) rather than the now-stale pattern.
- [x] **No panic path introduced** (SEC-5): confirmed by diff that this
      package added zero `.unwrap()`/`.expect()` calls anywhere;
      `resolve_locale`'s corrupt-value arm uses `.unwrap_or(...)`, a safe
      combinator, not a panicking one.
- [x] `cargo test --workspace --no-fail-fast`: **662 total** (was 660,
      +2 — the two new gates; `default_is_japanese` was renamed to assert
      both named constants rather than added), **661 passing, 1 failing**.
      `--features dev_fake_issuer`: **665 total** (was 663, +2), 664
      passing, 1 failing. **The one failure in both is the ROADMAP
      tripwire, untouched by this package, exactly as required** — not a
      regression.
- [x] `bun run smoke:all`: **25/25** — this package changes no rendered
      text (only which named constant produces the same Japanese value),
      so this confirms nothing shifted.
- [x] Evidence baseline unchanged at 996.

## Handoff 078: the smoke fixtures pin `ui_language`, proven by a temporary flip

No fixture sets `ui_language`, and no application insert path backfills it
either — so every seeded membership was `NULL`, and every signed-in smoke
page resolved through `Locale::PRODUCT_DEFAULT` (Japanese today, but
ROADMAP.md's now-due English-default decision moves it). Flipping that one
line would have flipped every Japanese-asserting smoke with it, for a
reason with nothing to do with the product — the same ambient-state
dependence Handoff 076 removed for `Accept-Language`.

- [x] **A shared, idempotent pin**: `scripts/lib/smoke-fixture-locale.mjs`
      exports `PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL` —
      `UPDATE community_memberships SET ui_language = 'ja' WHERE
      ui_language IS NULL`. Safe to call repeatedly; never overwrites a
      value a smoke set deliberately.
- [x] **Placement, not just seeding**: called after fixture seeding in
      every membership-creating smoke (22 of 23 — `abuse-controls.mjs`
      excluded, since it never navigates a browser or checks rendered
      text at all), **and** after `invite-redemption.mjs`'s real `/join`
      HTTP redemption, which creates a membership through application
      code, not a fixture `INSERT` — that row is `NULL` exactly like an
      unseeded one.
- [x] **A derived gate**,
      `every_japanese_asserting_smoke_pins_fixture_ui_language_or_documented_exception`,
      requires the pin of every smoke that asserts either a literal
      Japanese codepoint **or** a hardcoded `htmlLang === 'ja'` comparison
      — the second signal mattered: four scripts
      (`rfc075-slice4/5/6/7-*.mjs`) assert `htmlLangJa` with **zero**
      Japanese codepoints anywhere in the file, so a codepoint-only
      derivation would have missed a real dependency (confirmed by the
      temporary flip below, which is exactly how this was caught).
      Comments stripped first. Demonstrated failing: removed the import
      from a pinned script, confirmed the gate named it, restored
      byte-identical.
- [x] **One documented exception**: `language-preference.mjs` manages
      `ui_language` directly with its own scoped `UPDATE` (its own
      membership under test, by id) rather than the shared blanket pin —
      its `otherMembershipUnaffectedThroughout` check proves the language
      switch is membership-scoped precisely by asserting a *second*,
      deliberately-untouched membership's `ui_language` stays `NULL`
      forever. The blanket `WHERE ui_language IS NULL` pin would have set
      that row to `'ja'` too, making the check pass by construction and
      proving nothing — found by running the temporary flip, exactly the
      failure mode §10 named in advance.
- [x] **A second, related gap found the same way**: several raw Node
      `fetch()` calls (not the CDP-driven browser Handoff 076 already
      pins) carry no `Accept-Language` header at all — harmless while
      `PRODUCT_DEFAULT` was Japanese (rung 3 coincidentally matched rung
      2's pinned value), but for a request with no membership to resolve
      rung 1 from either (a zero-membership account principal, an
      anonymous identity-callback failure), rung 2 has nothing to
      negotiate and falls straight to rung 3 — this affected
      `account-surface.mjs`, `account-link-and-reauthentication.mjs`,
      `account-recovery-and-unlink.mjs`, and `external-identity-callback.mjs`.
      Fixed by pinning `Accept-Language: SMOKE_ACCEPT_LANGUAGE` on each
      affected `fetch()` (or its shared `cookieHeader` helper), reusing
      Handoff 076's existing constant — not a new mechanism.
- [x] **The temporary-flip proof (§5), the package's central verification**:
      temporarily set `Locale::PRODUCT_DEFAULT` to `Locale::En`, rebuilt,
      ran `bun run smoke:all` — first pass surfaced the four gaps above;
      after fixing them, a second flipped run was **25/25**. Reverted;
      `diff` confirmed `locale.rs` byte-identical; rebuilt; a final
      unflipped run was **25/25** again (after ruling out two transient,
      unrelated `UND_ERR_SOCKET` failures on `smoke:membership-suspension`
      — the same class of environment flake this project's history has
      already named twice — both clean on immediate retry).
- [x] **Not one assertion changed** — every fix pins an input an existing
      assertion already depended on. **No product code touched** —
      `workers/`, `packages/` untouched; `Locale::PRODUCT_DEFAULT` is
      unchanged in the committed tree. **The ROADMAP tripwire untouched.**
- [x] `cargo test --workspace --no-fail-fast`: **663 total** (was 662,
      +1 — the new gate), 662 passing, 1 failing. `--features
      dev_fake_issuer`: **666 total** (was 665, +1), 665 passing, 1
      failing. The one failure in both remains the ROADMAP tripwire.
- [x] Evidence baseline unchanged at 996.

## Handoff 079: the default language flips to English — RFC-085's separation made visible

ROADMAP.md's decision was due once RFC-084 closed Slice D: `Locale::PRODUCT_DEFAULT`
moves from Japanese to English. `Locale::FAIL_CLOSED` — the separate,
unmoving answer for a corrupt stored `ui_language` value — does not move
with it. This is the flip RFC-085 and Handoff 078 prepared for.

- [x] **The flip itself, one line**: `Locale::PRODUCT_DEFAULT: Self = Self::Ja`
      → `Self::En` in `packages/contracts/src/locale.rs`.
      `Locale::FAIL_CLOSED` is unchanged, still `Self::Ja`.
- [x] **Confirmed, not assumed**: the RFC-085 re-merge gate
      (`resolve_locale_corrupt_value_arm_references_fail_closed_not_product_default`)
      and `locale_never_regains_a_default_impl` were re-run in isolation
      after the flip — both still pass. They check constant *names* and
      structural shape, not values, so they survive the flip by
      construction, but this was verified rather than inferred from the
      full-suite pass.
      `resolve_locale(None)` now resolves `Locale::En`;
      `resolve_locale(Some("fr"))` (an out-of-allow-list stored value)
      still resolves `Locale::FAIL_CLOSED` (`Locale::Ja`), confirmed by a
      new test — the two rungs now visibly diverge, where before the flip
      they coincided and a value-only comparison could not have told a
      correct separation from a silent re-merge.
- [x] **Migration `0011` is schema-unchanged**: only its top comment was
      corrected to stop saying every existing row renders "Japanese" —
      `ADD COLUMN`/`CHECK` are byte-for-byte unchanged. No installed base
      to disrupt; the service has never been deployed.
- [x] **The ROADMAP tripwire retired**: `roadmap_english_default_tripwire_fires_when_slice_d_completes`
      (added by Handoff 072, the sole intentional failure since `cf3baba`)
      is deleted, not left failing-by-design or converted to pass — the
      decision it existed to surface has been taken. Its `STRUCTURALLY_UNRESOLVABLE`
      exception-tracking logic is deleted with it; confirmed via grep that
      no other file references the function name.
- [x] **The RFC-085 test that used to assert coincidence now asserts
      divergence**: renamed to `product_default_and_fail_closed_now_hold_different_values`,
      asserting `Locale::PRODUCT_DEFAULT == Locale::En`,
      `Locale::FAIL_CLOSED == Locale::Ja`, and the two are `!=` each
      other — impossible to state meaningfully before this handoff, since
      both held the same value.
- [x] **Doc comments corrected, named individually, not mass-replaced**:
      `locale.rs`'s module doc, `PRODUCT_DEFAULT`/`FAIL_CLOSED`'s own doc
      comments, `negotiate_accept_language`, and
      `resolve_account_locale_from_memberships` in `locale.rs`;
      `authz::resolve_anonymous_locale` in `authz.rs`; a stale comment on
      `db::membership`'s corrupt-value test; and the one release-gate doc
      comment and panic message in `release_gates.rs` that named "Japanese"
      as if it were still the product default. Historical sections above
      this one, and every `rfcs/` document, are deliberately left
      describing what was true when they were written.
- [x] **No panic path introduced** (SEC-5): confirmed by diff across every
      touched file — zero new `.unwrap()`/`.expect()` calls.
- [x] `cargo test --workspace --no-fail-fast`: **fully green for the
      first time since `cf3baba`** — 663 total, 663 passing, 0 failing
      (was 662 passing, 1 failing). `--features dev_fake_issuer`: 666
      total, 666 passing, 0 failing (was 665 passing, 1 failing). The
      ROADMAP tripwire's removal accounts for the entire delta; nothing
      else broke or was added to the count.
- [x] `bun run smoke:all`: **25/25**. `node scripts/test-evidence-leakage-baseline.mjs`:
      unchanged at 996.
- [x] `cargo clippy` (both feature states, `-D warnings`), `cargo fmt --all
      -- --check`, `cargo check --target wasm32-unknown-unknown -p
      zinnias-ciao-ssr`, `mdbook build docs`, `git diff --check`, and `bun
      run build` all pass clean.
- [x] ROADMAP.md's own entry for this decision is rewritten in place to
      record it as taken (dated, sequenced against RFC-085 and Handoff
      078's commits) — the one deliberate exception to this document's
      own rule of leaving past sections as historical record, since that
      entry described an open decision this handoff resolves.

## Handoff 081: every version artifact now derives from one authority

The `0.63.0` release (Handoff 080) had to hand-edit a hardcoded version
literal inside a release gate to pass — the exact pattern this project has
spent a week removing elsewhere. Of the five version-bearing artifacts a
release touches, only `sw.js`'s `CACHE_VERSION` was actually checked against
`Cargo.toml`; `package.json`'s version was ungated entirely, and the
`app.js` cache-buster was pinned by a literal sitting inside an unrelated
test (`rfc056_calendar_page_owns_calendar_and_switcher`, a test about the
calendar page owning its switcher).

- [x] **A shared `workspace_version()` helper**, extracted from
      `sw_cache_version_matches_workspace_version`'s inline
      `[workspace.package]` parser with its behaviour unchanged (including
      the loud `.expect` on an unparseable authority) — every derivation
      gate below now calls it instead of re-deriving the value.
- [x] **The cache-buster assertion moved and derived.** It no longer lives
      inside `rfc056_...`; it is now its own test,
      `cache_buster_matches_workspace_version`, beside
      `sw_cache_version_matches_workspace_version`, and builds its expected
      string (`/static/app.js?v={workspace_version()}`) instead of
      hardcoding it. `rfc056_...`'s other assertions — all genuinely about
      the calendar page — are untouched.
- [x] **`package.json`'s version is now gated**, in a new
      `package_json_version_matches_workspace_version` test. Parsed with
      `serde_json` as a JSON field, not substring-matched, so a `"version"`
      key appearing anywhere else in the file could not satisfy it by
      accident.
- [x] **The `v` prefix stays exactly where it was**: `CACHE_VERSION` reads
      `v0.63.0` (the gate strips the leading `v` before comparing); the
      cache-buster and `package.json` read bare `0.63.0` (compared with no
      prefix added or stripped). Confirmed by construction — neither gate
      normalises the other's format.
- [x] **All four mutation points demonstrated failing, individually, then
      restored byte-identical**: `sw.js`'s `CACHE_VERSION`; `render/shell.rs`'s
      cache-buster alone (proving `handlers/static_files.rs` is checked
      independently); `handlers/static_files.rs`'s cache-buster alone
      (proving the reverse); and `package.json`'s version alone. Each
      mutation was confirmed landed via grep before running the gate, and
      each file was confirmed byte-identical via `git diff --stat` after
      reverting.
- [x] **The single-authority property demonstrated**: bumped
      `Cargo.toml`'s workspace version alone, with nothing else touched,
      and all three derivation gates failed together in one run
      (`sw_cache_version_matches_workspace_version`,
      `cache_buster_matches_workspace_version` — which itself covers both
      source-file mutation points in one assertion — and
      `package_json_version_matches_workspace_version`). Reverted; `Cargo.lock`
      was regenerated back to `0.63.0` by the same `cargo check` that
      confirmed it, and the full suite is green again.
- [x] `cargo test --workspace --no-fail-fast`: 665 total, 665 passing, 0
      failing (was 663, +2 — the two new gates; no gate was removed, only
      moved and derived). `--features dev_fake_issuer`: 668 total, 668
      passing, 0 failing (was 666, +2).
- [x] `bun run smoke:all`: 25/25. `node scripts/test-evidence-leakage-baseline.mjs`:
      unchanged at 996. No product code touched — `workers/ssr/src/` and
      `packages/contracts/src/` untouched; this is a test-layer change
      only, and no version value changed anywhere.

**What a release no longer has to do by hand:** bump `Cargo.toml`'s
workspace version, and `sw.js`'s `CACHE_VERSION`, the `app.js` cache-buster
in both `render/shell.rs` and `handlers/static_files.rs`, and `package.json`'s
version all now fail loudly on their own if left behind — no release gate
needs its own literal hand-edited to reflect the new version.

## RFC-054 Slice 2: admin destructive-action copy

Slice 1 was about the member reading at their worst moment; Slice 2 is about
the volunteer administrator, about to act on someone else, usually once,
without practice. Three `Localized` pairs in `packages/contracts/src/i18n/admin.rs`
changed, both halves each — no other constant touched.

- [x] **`ADMIN_REMOVE_CONSEQUENCE` now states removal cannot be undone.**
      `removed_at` is only ever `SET` (`db/membership.rs:641`); no path
      clears it. Suspension already said 「この操作は取り消せます。」;
      removal said nothing, and silence next to an explicit reassurance
      read as "probably similar." Also merged in the fact that other
      communities are unaffected, and that re-inviting the same person
      later creates a **new** membership — their prior role and display
      name do not carry over. Verified against migration `0001`'s partial
      unique index, which is what makes the re-invitation claim accurate:
      the person can be invited again; the membership cannot be restored.
- [x] **All three last-admin refusals now name the same remedy.** Two of
      the three already said "transfer the admin role first";
      `ADMIN_LAST_ADMIN_DEMOTE` stopped at the refusal. Its second sentence
      is copied verbatim from its siblings — convergence, not new wording.
- [x] **`ADMIN_DEMOTE_CONSEQUENCE`'s subject converged on "this member."**
      Five of six sibling constants already said このメンバー/"This
      member"; this one said この人/"This person" for the same person in
      the same table. `REMOVE_CONSEQUENCE`'s English subject (`They` →
      `This member`) converged in the same package, since that string was
      already changing for the irreversibility fix above.
- [x] `ADMIN_SUSPEND_CONSEQUENCE` is untouched — its reversibility
      statement is true and useful; the asymmetry was fixed by making the
      silent constant speak, not by quieting the informative one.
- [x] **Cross-check re-run, not taken on trust**: grepped
      「メンバーから外す」、「メンバーに戻す」、「管理者権限を移譲」、「この人」、
      and 「この操作は取り消せ」 across `workers/ssr/src` and
      `packages/contracts/src` before editing — all five counts matched
      the decision list's own prior measurement exactly. No prose in
      running text names either changed button's label, so Slice 1's F1
      class (prose naming a button whose label had since changed) does not
      recur here.
- [x] **A pre-existing gate caught a real conflict**:
      `rfc063_removal_only_policy_is_locked` asserts
      `JA_ADMIN_REMOVE_CONSEQUENCE.contains("残ります")`. The first draft's
      continuative phrasing ("…残り、他のコミュニティ…") dropped that exact
      substring while preserving the same meaning. Fixed with a
      punctuation-only change (splitting into two sentences,
      "…残ります。他のコミュニティ…") — no content lost, no gate touched.
- [x] `cargo test --workspace --no-fail-fast`: unchanged at 665 total, 665
      passing, 0 failing (the derived EN/JA parity gate and the
      identical-pair check inside it both re-confirmed passing without any
      table edit — these three pairs stay paired, and none became
      identical to another constant). `--features dev_fake_issuer`:
      unchanged at 668/668.
- [x] `smoke:admin-member-management` and `smoke:admin-tools-onboarding` —
      the two smokes that render these pages — both clean, plus
      `smoke:admin-role-transfer`, which asserts the last-admin-demote
      copy is absent from an unrelated denial page (a negative substring
      check that stays valid since the new text still starts with the old
      substring). `bun run smoke:all`: 25/25.
      `node scripts/test-evidence-leakage-baseline.mjs`: unchanged at 996.
