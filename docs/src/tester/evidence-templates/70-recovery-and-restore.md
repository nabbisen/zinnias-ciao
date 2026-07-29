# 70 — Recovery and Restore Manual Evidence Template

RFC-050 Tooling Slice 7. Copy this file into the candidate's evidence
directory as `70-recovery-and-restore.md` and fill in every field before the
candidate can pass its recovery/restore manual checks. This file is a
**template** — do not fill in real values here.

**Do not commit real values.** Never paste a cookie, session secret, invite
code, HMAC pepper, raw database id, or Durable Object id into this document.
Run `bun run evidence:scan-leakage <path>` against the filled-in copy before
treating it as complete.

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

### D1 migration ledger application

- **Steps:** apply the exact candidate's complete migration ledger to the
  target environment, through at least the current baseline migration.
- **Expected:** every migration applies cleanly, in order, with no manual
  intervention.
- **Actual:**
- **Pass/fail:**
- **Notes:**

### Rollback/forward recovery rehearsal

- **Steps:**
- **Expected:** a rehearsed rollback and forward-recovery both complete
  without data loss beyond the intentionally-discarded window.
- **Actual:**
- **Pass/fail:**
- **Notes:**

### RFC-079 metadata-reset and backup-sensitivity boundary

- **Steps:**
- **Expected:** a restore correctly resets required-audit metadata per
  RFC-079, and no backup artifact retains a forbidden content class (secret,
  cookie, raw resource id — see Tooling Slice 2).
- **Actual:**
- **Pass/fail:**
- **Notes:**

### Privacy boundary on restore

- **Steps:**
- **Expected:** a restored environment does not expose data across
  community or membership boundaries it should not.
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
