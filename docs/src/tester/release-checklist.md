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
- [~] Hosted Cloudflare staging smoke executed and evidence attached. *(operator task: deploy staging with `BUILD_VERSION` set to the release label, then `EXPECTED_VERSION=v0.60.0 bun run smoke:runtime -- <deployed-worker-url>`)*
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
- [x] Every page named in the RFC's member-facing core — Home, Communities/Calendar (month, list, and matrix modes), and Event Detail (including the note-delete confirmation) — is fully migrated: `html lang` and every rendered string derive from the same resolved locale, with no bare `i18n::JA_*` left behind on any of them. *(release_gates.rs `rfc072_member_facing_core_has_no_half_migrated_page`)* **Corrected 2026-07-30:** this claim was false for Event Detail. The gate above checked handler files only; it never saw that `event.rs` calls three shared render helpers (`render::status_form`, `render::note_form`/`admin_note_hide_form`, `render::participant_list`) that were never locale-migrated, so an English-preference member saw Japanese attendance buttons and note controls on an otherwise-English page. Fixed in Handoff 026 — see the new section below. RFC-072 acceptance criterion 9 was not actually met until that fix landed.
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
- [ ] **Known, pre-existing, not a Slice 2 regression:** Event Detail's status buttons wrap a single unbreakable word ("Attended" / "参加済み") in a three-column flex row; at 200% text this overflows the mobile viewport. Confirmed present at checkpoint `6415708` (before this slice) with the identical CSS properties, then only restructured from inline styles into classes — English overflows substantially and unambiguously (~150-190px) in both the pre- and post-migration measurement; the Japanese case is a marginal ~7px boundary case that did not reproduce in one post-migration measurement, which is ordinary layout/font-rendering variance, not a deliberate change. Not fixed in this presentation-migration-only package — a real layout fix (e.g. permitting label wrap) is a behaviour change outside its scope. Needs its own small, reviewed accessibility fix.
- [ ] **§7.4 blocker, on the record:** `render/event_card.rs` remains dead code (confirmed at `ced6ae4`, Handoff 026, zero callers in its entire history) with 10 inline styles and 4 bare `i18n::JA_` references, neither touched by this slice per its explicit non-change scope. Because it is never migrated and never deleted, RFC-075's terminal criterion — inline `style=` reaching zero, so `'unsafe-inline'` can be dropped from `style-src` — is **currently unreachable**. Agreed: this is the blocker, and resolving it (deleting the dead `event_card` function while keeping the `CardDay` struct that shares its file) is a real extraction with its own risk, belonging in its own small, separately reviewed package — not bundled into a CSS migration.

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
