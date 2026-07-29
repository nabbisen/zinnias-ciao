# RFC-050 Manual Evidence Templates

These three templates (RFC-050 Tooling Slice 7) are copied into a
candidate's local evidence directory and filled in by hand during a hosted
evidence campaign. They are templates, not evidence — do not fill in real
values in these tracked copies.

- [50 — Browser and Device](50-browser-and-device.md)
- [60 — Observability and Runtime](60-observability-and-runtime.md)
- [70 — Recovery and Restore](70-recovery-and-restore.md)

## Workflow

1. Copy the three templates into the candidate's evidence directory
   (`.git-exclude/evidence/<candidate-label>/`, alongside the automated
   `00-manifest.json`, `01-local-gates.json`, and the Tooling Slice 3–6
   records).
2. Fill in every field. Never paste a cookie, session secret, invite code,
   HMAC pepper, raw database id, or Durable Object id — see
   `scripts/lib/evidence-manifest.mjs`'s redaction rules
   (RFC-050 Tooling Slice 2).
3. Run the leakage scanner against the filled-in copies before treating
   them as complete:

   ```sh
   bun run evidence:scan-leakage .git-exclude/evidence/<candidate-label>
   ```

   A nonzero exit means a forbidden content class was found; fix the
   document and re-run before proceeding. JSON evidence records get the
   full field-aware sweep; these markdown templates get the narrower
   free-text sweep (cookies, D1 error bodies, SQL-shaped text, raw
   resource-id prefixes, bare hex values at this project's actual secret
   length, and any run-scoped registered value) — see the known
   limitation below.
4. Run the artifact-hashing utility over the same directory and keep the
   resulting `HASHES.sha256` manifest with the evidence package:

   ```sh
   bun run evidence:hash-artifacts .git-exclude/evidence/<candidate-label>
   ```

5. Only the sanitized, tracked attestation (RFC-050 Tooling Slice 8, not yet
   built) is ever committed — these filled-in templates and their
   screenshots stay local and ignored.

## Known limitation

The free-text leakage scan flags a bare hex string at this project's actual
secret/digest/token/Durable-Object-id length — 56, 64, 96, or 128 hex
characters, matching `random_token()`, HMAC-SHA256 subject digests, and
Durable Object ids — the same way the JSON-record scan does. It deliberately
excludes 32- and 40-character hex runs, because a git commit sha is 7–40
hex characters and a manual template legitimately contains one as prose,
with no structured field to exempt it by the way a JSON record's `commit`
field can be. Those two lengths remain unscannable in free text; reviewers
filling in these templates must still avoid pasting a raw 32- or 40-character
hex value by hand for that residual case specifically. Every other forbidden
class (cookies, D1 error bodies, SQL-shaped text, raw resource-id prefixes,
and any run-scoped registered value) is caught the same as in JSON.
