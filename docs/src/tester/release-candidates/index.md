# RFC-050 Release Candidate Attestations

RFC-050 Tooling Slice 8. This directory holds the **tracked, sanitized**
record of every candidate that has gone through (or is going through) an
RFC-050 hosted evidence campaign. It is the only piece of that campaign
ever committed — the raw evidence package stays local at
`.git-exclude/evidence/<candidate-label>/` and is git-ignored.

- [TEMPLATE](TEMPLATE.md) — copy this to start a new candidate's record.

## Workflow

1. Run the RFC-050 local tooling (Tooling Slices 1–7) against the frozen
   exact candidate, producing its evidence package under
   `.git-exclude/evidence/<candidate-label>/`.
2. Copy `TEMPLATE.md` to `docs/src/tester/release-candidates/<candidate-label>.md`
   and fill in every field: candidate identity, the evidence-package digest
   manifest path, and a verdict for every gate — with separate Local
   Evidence and Hosted Evidence citations. A local citation never
   discharges a Hosted Evidence cell; see the template's own explanation.
3. Scan **this specific candidate's** evidence directory for leakage before
   treating the record as complete:

   ```sh
   bun run evidence:scan-leakage .git-exclude/evidence/<candidate-label>
   ```

   Never run the scanner with no argument for this purpose — its default
   scope covers the whole evidence tree, including historical directories
   unrelated to this candidate, and is known to exit nonzero for reasons
   that have nothing to do with this candidate's record. There is no
   `--force` flag; a nonzero exit means fix the filled-in evidence and
   re-scan.
4. Hash the same directory and record the resulting manifest path in the
   candidate's record:

   ```sh
   bun run evidence:hash-artifacts .git-exclude/evidence/<candidate-label>
   ```

5. Run the gate-rule checker against the filled-in candidate record itself
   (not the evidence package):

   ```sh
   bun run evidence:check-attestation docs/src/tester/release-candidates/<candidate-label>.md
   ```

   This mechanically enforces the E4a → E4 dependency, the closed verdict
   vocabulary, and the RFC-078 criterion 6 IPv6 `/64` risk-acceptance
   wording. A nonzero exit means the record itself is inconsistent; it does
   not and cannot verify that a cited hosted observation is genuine — that
   remains the reviewer's judgment at sign-off.
6. Only once every gate cell, the reviewer sign-off, and the E9 staging
   closure result are filled in does the candidate record commit.

## Rules

- **One record per candidate.** A failed candidate's record stays recorded
  as failed — it is never edited to pass after a different version
  deploys. A new commit, a new Worker version, or a re-run after a fix is a
  new candidate and gets a new file.
- **No credentials, cookies, tokens, peppers, recovery codes, raw resource
  ids, subject identifiers, screenshots, or user/community data** in any
  candidate record — see `scripts/lib/evidence-manifest.mjs`'s redaction
  rules (RFC-050 Tooling Slice 2). Those live only in the local, git-ignored
  evidence package.
