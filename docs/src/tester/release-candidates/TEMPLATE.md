# Release Candidate Attestation — TEMPLATE

RFC-050 Tooling Slice 8. This is the **only** per-candidate RFC-050 artifact
ever committed. Everything else produced during a hosted evidence campaign
— the numbered evidence-package files, screenshots, filled-in copies of
`docs/src/tester/evidence-templates/`, `HASHES.sha256` — stays in
`.git-exclude/evidence/<candidate-label>/`, local and git-ignored.

**Do not commit real values.** This document records verdicts and pointers
only. Never paste a credential, cookie, session/form token, HMAC pepper,
recovery code, raw database id, Durable Object id, subject identifier, raw
resource id, screenshot, or any user/community data into a copy of this
file. Run `bun run evidence:scan-leakage <candidate-evidence-dir>` against
the underlying evidence package (not this file) before signing off — see
the "Evidence package integrity" section below.

**One record per candidate. Never edited to pass after a different version
deploys.** A failed candidate's record stays recorded as failed. A new
candidate — a new commit, a new Worker version, a re-run after a fix — gets
a new file at `docs/src/tester/release-candidates/<candidate-label>.md`.
Copy this template to create it.

## Candidate identity

| Field | Value |
|---|---|
| Candidate label | |
| Commit | |
| Worker version id | |
| Worker version tag | |
| Deployment | |
| Evidence package directory (local, not committed) | `.git-exclude/evidence/<candidate-label>/` |

## Evidence package integrity

| Field | Value |
|---|---|
| Evidence-package digest manifest | `<path>/HASHES.sha256`, produced by `bun run evidence:hash-artifacts .git-exclude/evidence/<candidate-label>` |
| External verification | Independently verifiable via `sha256sum -c HASHES.sha256` (run from inside the evidence directory), without relying on any of this project's own tooling |
| Leakage scan target directory | `.git-exclude/evidence/<candidate-label>/` — must be this specific candidate directory. Never invoke `scan-evidence-leakage.mjs` with no argument for this purpose: its default scope is the whole `.git-exclude/evidence` tree, which includes historical pre-RFC-050 directories and is known to exit nonzero for reasons unrelated to this candidate. There is no `--force` or override flag, and none should be added — a nonzero exit here means fix the document and re-scan. |
| Leakage scan result | (exit code and finding count from the scoped scan above) |

## Gate verdicts

Verdict vocabulary (closed): `Pass`, `Fail`, `Pending`, `Void`, `Risk-Accepted-Open`, `N/A`. A cell that doesn't start with one of these words fails the mechanical gate check below. `Void` marks a verdict invalidated by a gate dependency (see Gate rules), not an independently assessed failure. `N/A` is only for a gate that structurally has no Hosted requirement (E0).

**A Local Evidence citation never satisfies a Hosted Evidence cell, structurally.** These are two separate columns for exactly that reason: citing a local command's output, a local-mode `runtime-smoke.mjs` run, or any other local gate in the Hosted column does not discharge that gate's hosted-evidence requirement. Every Hosted cell other than E0's must cite evidence produced during this candidate's actual hosted campaign — a path under this candidate's `.git-exclude/evidence/<candidate-label>/` directory, a hosted dashboard observation, or similar. A commit hash, a local gate name, or `git diff --check` output is not a valid Hosted citation for any gate.

| Gate | Local Evidence | Hosted Evidence | Notes |
|---|---|---|---|
| E0 — local candidate freeze | | N/A — local-only gate | |
| E1 — identity/deployment/bindings/migrations | | | |
| E2 — public routes/headers/cookies/cache/offline | | | |
| E3 — authenticated core + timezone/export flows | | | |
| E4a — direct-ingress topology and client-identity classification | | | Gates E4; see Gate rules |
| E4 — concurrency and fail-closed controls | | | Void unless E4a Hosted = Pass for this same candidate |
| E5 — isolated negative configuration | | | |
| E6 — no-JS and real-device accessibility | | | |
| E7 — persistent logs/audit/incident visibility | | | Canary delivery-interval/retrieval-timestamp fields: see `evidence-templates/60-observability-and-runtime.md` |
| E8 — CPU/query/error/plan behavior | | | |
| E9 — migration/restore/recovery/closure | | | |

### Gate rules (mechanically enforced)

Run `node scripts/check-release-candidate-attestation.mjs <this-file>` (or
`bun run evidence:check-attestation <this-file>`) before treating this
document as complete. It enforces:

1. **E4a → E4 dependency.** A `Pass` recorded in E4's Hosted Evidence cell
   is invalid unless E4a's Hosted Evidence cell is also a current `Pass`
   for this same candidate (RFC-050 E4a: "E4's capacity results are void
   unless E4a passed against the same candidate"). If E4a is not `Pass`,
   E4's Hosted cell must read `Void`.
2. **Closed vocabulary.** Every Local/Hosted cell must be blank or begin
   with one of the six verdict words above.
3. **No known-local citation in a Hosted cell.** A Hosted Evidence cell
   citing a `bun run test`/`cargo test`/`cargo clippy`/`cargo fmt`/`cargo
   check`/`git diff --check` invocation, or a `docs/`/`scripts/` path, is
   rejected — those are recognizable local-repo markers, never a hosted
   observation. This is a negative check for known-local shapes, not a
   positive shape requirement: hosted evidence legitimately takes forms
   (a dashboard observation, a version-id confirmation) that don't reduce
   to one path pattern, so nothing narrower than this is required.
4. **The IPv6 risk acceptance below must read exactly `Risk-Accepted-Open`.**

These are mechanical checks on this document's own text; they do not and
cannot verify that a cited hosted observation is genuine. That judgment
remains the reviewer's, at sign-off below.

## Carried risk acceptances

| Risk | Status | Reference |
|---|---|---|
| RFC-078 criterion 6 — IPv6 `/64` sub-clause | Risk-Accepted-Open | IPv6 client support is not confirmed/implemented for this deployment (RFC-050 §E4a). Owner risk-accepted 2026-07-28. This status may never be recorded as `Pass`, `Hosted-Proven`, or any word other than `Risk-Accepted-Open` for any candidate under this RFC. |

## Open exceptions

| Exception | Gate | Disposition |
|---|---|---|
| | | |

## Reviewer decision

| Field | Value |
|---|---|
| Reviewer | |
| Date/time (UTC) | |
| Outcome (Approved / Conditionally Approved / Corrections Required / Design Revision Required / Requirements Clarification Required / Human Owner Decision Required) | |
| Rationale | |

## Staging closure result (gate E9)

| Field | Value |
|---|---|
| E9 closure confirmed (recovery flag disabled, temporary secret rotated/deleted, negative-test and recovery Workers/routes closed including the E4a disposable upstream-Worker negative test and its teardown, canonical staging Worker removed or placed behind reviewed access control) | |
| Teardown inventory reference (resource kind, privacy-safe fingerprint, action, result, time, operator — recorded in the local evidence package, not here) | |
| Retired pre-RFC-078 `RATE_LIMIT` KV namespace disposition (if one exists for this environment) | retained, not deleted, pending separate owner-authorized deletion |
