# ciao.zinnias Roadmap

## Status

**Current release:** 0.57.0.

The RFC folder is the source of truth for implementation state:

- Detailed RFC index: [rfcs/README.md](./rfcs/README.md)
- Implemented RFCs: [rfcs/done/](./rfcs/done/)
- Proposed RFCs: [rfcs/proposed/](./rfcs/proposed/)

Recent workflow releases focused on calendar-centered use, community bootstrap,
member administration, admin role transfer, member lifecycle policy,
admin-mediated help sign-in, Rust module boundary cleanup, recurrence v2 for
Calendar workflows, admin event-copy creation assistance, the monthly
attendance matrix, and admin-only matrix CSV export.

## Proposed Work

The active proposed backlog is:

| RFC | Theme | Current note |
|-----|-------|--------------|
| 020 | Design assets, prototype, and handoff | Non-code design deliverable remains. |
| 021 | Notification strategy and reminder digests | Requires product and infrastructure design before implementation. |
| 031 | Consentful contact channels and privacy-safe messaging | Depends on consent and notification policy decisions. |
| 033 | Subgroups, event visibility, and boundary safety | High-impact authorization and privacy design work. |
| 034 | Notification-free quiet mode and attention design | Should be considered with RFC-021. |
| 044 | D1 query-budget gate and integration test harness | Runtime/integration hardening candidate. |
| 045 | Pre-pilot runtime verification matrix | Runtime evidence and operator verification candidate. |
| 050 | Staging runtime verification evidence pack | Prototype exists; full evidence workflow remains. |
| 054 | Japanese UX copy review | Needs native-speaker review and copy-quality pass. |
| 064 | Rust module and crate boundary cleanup | Phase 1 admin-events split ships in v0.52.0; Phase 2 render split ships in v0.53.0; Phase 3+ remains proposed. |

## Near-Term Candidates

Recommended next candidates, in practical order:

1. **RFC-064: Rust Module and Crate Boundary Cleanup**
   Phase 1 and Phase 2 have split admin event handlers and shared render
   helpers. Next steps should decide whether to continue with contracts/i18n
   structure or explicitly defer crate extraction.

2. **RFC-054: Japanese UX Copy Review**
   Recent releases added sensitive recovery and member-management flows. Copy
   quality is now part of usability and safety.

3. **RFC-021 and RFC-034: Notifications and Quiet Mode**
   These should be designed together to avoid adding reminders without a clear
   attention and opt-out policy.

4. **RFC-031: Consentful Contact Channels**
   Useful after notification policy is clear. This should remain privacy-first
   and consent-bound.

5. **RFC-033: Subgroups and Event Visibility**
   Large feature area touching authorization, event visibility, and community
   boundaries. It should start with design review, not direct implementation.

6. **RFC-044, RFC-045, RFC-050: Runtime Evidence and Hardening**
   These are good candidates when the project priority shifts from product
   workflow to deployment confidence and Cloudflare-hosted evidence.

## Before First Pilot Deployment

These are the remaining gates before the first real community can use the
service.

### Operator Tasks

- [ ] Apply all D1 migrations through `0009_recurrence_v2.sql` to the target environment; rehearse rollback.
- [ ] Set required secrets per environment without printing or committing real values.
- [ ] Configure required KV/D1 bindings per environment.
- [ ] Configure `SESSION_COOKIE_DOMAIN` as a non-secret variable only when a shared cookie domain is required.
- [ ] Configure Logpush for production if production audit retention requires it.
- [ ] Run security review against the release checklist.

### Browser and Device QA

- [ ] Core join-and-mark-attendance flow under 2 minutes on a real phone.
- [ ] Calendar, event detail, and admin member flows remain usable at 200% system text scaling.
- [ ] No-JS destructive confirmations work without scripting.
- [ ] Recovery/help-signin flow works on the target hosted environment.

### Release Gate

- **Do not deploy to production** or tag a public pilot release without explicit confirmation from nabbisen.

## After First Pilot

Once a pilot community has been running for at least 4 weeks, revisit:

1. **RFC-033: Subgroups** if privacy or visibility boundaries emerge from real usage.
2. **RFC-021: Notifications** if sync-based checking proves insufficient.
3. **RFC-031: Contact channels** if admins need direct member communication and consent rules are clear.

The guiding principle remains: add only what is needed. Every feature added is a
feature that must be maintained, explained, and trusted.
