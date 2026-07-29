# 50 — Browser and Device Manual Evidence Template

RFC-050 Tooling Slice 7. Copy this file into the candidate's evidence
directory as `50-browser-and-device.md` and fill in every field before the
candidate can pass E3's manual browser/device checks. This file is a
**template** — do not fill in real values here.

**Do not commit real values.** Screenshots, videos, and this filled-in file
stay local to the evidence workspace; only the sanitized attestation (Slice
8) is tracked. Never paste a cookie, session secret, invite code, HMAC
pepper, raw database id, or Durable Object id into this document, in a
screenshot filename, or in a note. Run `bun run evidence:scan-leakage
<path>` against the filled-in copy before treating it as complete — see
`docs/src/tester/evidence-templates/index.md`.

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
| Device model | |
| OS + version | |
| Browser + version | |
| Network conditions | (e.g. Wi-Fi, cellular, throttled) |

## Scenarios

Repeat this block once per scenario. Cover at minimum: `/join` at real
device text scale 100% and 200%, `/join` with JavaScript disabled, `/offline`
rendering, and one authenticated flow (e.g. mark attendance) at 200% text
scale. Add more rows for anything specific to this candidate's changes.

### Scenario: `<name>`

- **Steps:**
  1.
  2.
- **Expected:**
- **Actual:**
- **Pass/fail:**
- **Screenshot reference (filename only, not embedded):**
- **Notes:**

## Artifact hashes

Run `bun run evidence:hash-artifacts <path-to-this-candidate's-evidence-dir>`
after every screenshot and this file are in place, and record the resulting
`HASHES.sha256` manifest path here rather than re-listing hashes by hand:

- Manifest: `<path>/HASHES.sha256`

## Reviewer sign-off

| Field | Value |
|---|---|
| Reviewer | |
| Date/time (UTC) | |
| Verdict | |
