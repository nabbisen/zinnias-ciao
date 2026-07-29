# 60 — Observability and Runtime Manual Evidence Template

RFC-050 Tooling Slice 7. Copy this file into the candidate's evidence
directory as `60-observability-and-runtime.md` and fill in every field
before the candidate can pass its observability/runtime manual checks. This
file is a **template** — do not fill in real values here.

**Do not commit real values.** Never paste a cookie, session secret, invite
code, HMAC pepper, raw database id, Durable Object id, or a raw `wrangler
tail`/dashboard log line containing any of those into this document. Record
that a check passed and what was observed in general terms; run `bun run
evidence:scan-leakage <path>` against the filled-in copy before treating it
as complete.

## Candidate identity

| Field | Value |
|---|---|
| Commit | |
| Candidate label | |
| Worker version id | |
| Worker version tag | |
| Deployment | |

## Session metadata

| Field | Value |
|---|---|
| Tester | |
| Date/time (UTC) | |

## Checks

### Workers dashboard — CPU time and error rate

- **Steps:**
- **Expected:** CPU time per request stays within the Workers Free 10ms
  budget (AD-3); error rate is zero for the exercised routes.
- **Actual:**
- **Pass/fail:**
- **Notes:**

### `wrangler tail` review

- **Steps:**
- **Expected:** no unexpected error-level log lines during the exercised
  flows; no secret, cookie, or raw resource id appears in any log line.
- **Actual:**
- **Pass/fail:**
- **Notes:**

### Persistent incident sink delivery and retrieval (RFC-050 Prerequisite 6, gate E7)

- **Steps:**
- **Expected:** a canary incident delivered through the configured sink
  (Logpush → R2 per the 2026-07-28 owner decision) is retrievable, with the
  documented retention (90d production / 30d staging) and access boundary
  intact. E7 requires the canary be "retrievable after the documented
  delivery interval" — record the interval and the retrieval time as named
  fields, not free prose, so a pass cannot be claimed without both being
  present.
- **Canary sent (UTC):**
- **Documented delivery interval:**
- **Earliest permitted retrieval time (UTC):** (sent + interval)
- **Retrieval attempted (UTC):**
- **Retrieval timestamp (UTC):** (when the canary was actually confirmed
  retrievable; must be at or after the earliest permitted retrieval time
  above)
- **Actual:**
- **Pass/fail:**
- **Notes:**

## Artifact hashes

Run `bun run evidence:hash-artifacts <path-to-this-candidate's-evidence-dir>`
after this file is in place, and record the resulting manifest path:

- Manifest: `<path>/HASHES.sha256`

## Reviewer sign-off

| Field | Value |
|---|---|
| Reviewer | |
| Date/time (UTC) | |
| Verdict | |
