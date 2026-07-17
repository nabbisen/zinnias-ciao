# RFC 079 — Atomic Required Audits and Recursive Metadata Redaction

**Status.** Accepted — architecture review accepted with notes on 2026-07-14;
explicit owner acceptance recorded on 2026-07-15; local Packages 0A–8 are
implemented and reviewed. A locally implemented Class A failure-telemetry
correction awaits architecture implementation review.

**Priority.** Architect-review remediation B5; blocks every public or production
pilot

**Source finding.** 2026-07-14 architecture preparation review B5

**Tracks.** RFC-002, RFC-003, RFC-009, RFC-014, RFC-024, RFC-041, RFC-045,
RFC-050, RFC-052, RFC-057, RFC-062, RFC-068, RFC-069, RFC-070, RFC-071

**Touches.** `workers/ssr/src/audit.rs`, every current audit-producing handler
and database mutation helper, a new D1 migration, contracts and Worker tests,
audit/threat-model/operations documentation, release gates and exact-candidate
hosted evidence

## Owner Acceptance Record

The owner explicitly accepts the following consequences of this architecture:

- logout revocation is the sole current Class C safety-first exception;
- all legacy `metadata_json` values are permanently reset to `{}` in the live
  audit table;
- the migration preserves core event identity, actor/scope/target, action, and
  timestamp chronology;
- pre-migration backups and D1 recovery history are potentially sensitive and
  remain access-controlled; and
- recovery rolls forward and must not restore the unsafe writer, arbitrary
  metadata, or removed legacy metadata.

Acceptance authorizes implementation; it does not close finding B5 or authorize
a public or production pilot. The reviewed dependency baseline resolves
`worker`, `worker-macros`, and `worker-sys` to worker-rs 0.8.4.

The implementation handoff must also preserve the architecture review notes:

1. choose either explicit legacy-plus-current operator queries or a reviewed,
   deterministic mapping for known legacy `(target_kind, action)` pairs;
   unknown historical actions must not be guessed or silently rewritten;
2. pin one concrete D1/SQLite-compatible zero-winner assertion primitive,
   including statement order, cleanup, transaction-unique keys, concurrent-call
   behavior, and deliberate SQL failure, before converting join or relink;
3. treat delivery to an approved persistent incident sink as an evidence
   prerequisite; structured console emission and `wrangler tail` alone are not
   durable evidence; and
4. include pre/post migration queryability checks and the accepted data-reset,
   backup-sensitivity, and roll-forward consequences in rehearsal and operator
   documentation.

## Summary

Make required audit evidence part of the protected database mutation instead of
a later best-effort side effect, and replace arbitrary metadata JSON with typed,
per-action allowlists plus recursive defense-in-depth redaction.

The current generic writer accepts any `serde_json::Value`, removes only a short
list of exact top-level keys, inserts the row after the business mutation, and
has its error discarded by most callers. The documentation claims broader
redaction—including generic `code`, `hmac`, `session`, and `memo` keys—and does
not disclose that nested values are untouched. Several direct SQL audit inserts
bypass the redactor entirely.

RFC-079 introduces three explicit persistence classes:

1. **Atomic required audit.** Security-, administration-, moderation-, and
   credential-relevant mutations execute with their audit insert in one D1
   `batch()` transaction. An audit failure rolls back the mutation and the
   handler does not return success.
2. **Fail-closed pre-disclosure audit.** Export authorization/request events are
   written before protected data or a client-side download authorization is
   returned. The action name describes authorization/request, not proof that a
   network download completed.
3. **Safety-first secondary audit.** Logout revocation must complete even if its
   audit insert fails. This is the sole current best-effort exception; the
   failure is awaited, classified, and emitted as an incident event rather than
   silently discarded.

All production audit rows are constructed from a closed `AuditAction` and typed
`AuditMetadata` variants. Identifiers belong in dedicated columns whenever
possible. A private recursive sanitizer removes forbidden keys at every object
depth, including objects inside arrays, and enforces depth, node, and serialized
size limits. Arbitrary handler-provided JSON is no longer an audit API.

Legacy metadata is treated as untrusted. Migration preserves the core audit
identity/action/timestamp columns but resets all pre-RFC-079 `metadata_json` to
`{}` because the current implementation cannot prove that retained nested or
misnamed content is safe.

This accepted remediation design authorizes implementation. Completion and
pilot readiness still require the local real-D1 and exact-candidate hosted
evidence specified below.

## Problem and Security Invariants

The architecture review found best-effort audit writes after invite generation
and revocation, join/relink, member removal and role transfer, event lifecycle
changes, attendance override, note moderation, calendar-token changes,
templates, exports, and logout. A successful business response can therefore
exist without the audit row required by the requirements and threat model.

Awaiting the existing writer is not enough. Once the mutation has committed, a
later insert error cannot undo it. Correctness requires the mutation and audit
statement to share a database transaction.

The current redactor also provides a false sense of safety:

- it visits only the root JSON object;
- it does not remove documented keys such as `code`, `hmac`, `session`, or
  `memo`;
- it does not normalize case;
- it does not constrain unknown keys, nesting, or size; and
- direct `INSERT INTO audit_log` call sites do not invoke it.

RFC-079 establishes two invariants:

> A mutation classified as requiring audit may be reported successful only if
> the business state and exactly the required audit row committed in the same
> D1 transaction. Failure of either side commits neither side.

> Audit metadata reaches storage only through a closed per-action schema and a
> recursive sanitizer. Bearer material, secrets, session identity, user content,
> and unknown metadata are rejected or removed before statement construction;
> audit and operational logs never become a second content store.

## Goals

- Classify every current audit event by explicit failure semantics.
- Eliminate silently discarded audit errors.
- Atomically couple required mutations and audit inserts with D1 `batch()`.
- Preserve safe logout revocation even during audit-table failure.
- Fail closed before returning protected exports when their audit is unavailable.
- Centralize audit action names, statement construction, metadata schemas, and
  logging.
- Recursively redact forbidden keys in nested objects and arrays.
- Reject unknown, oversized, excessively deep, or excessively complex metadata.
- Remove unproven legacy metadata without destroying the useful audit timeline.
- Keep note bodies, event descriptions/titles, display names, credentials,
  cookies, session IDs, and bearer URLs out of audit and Workers logs.
- Prove rollback behavior with real D1 transactions locally and on isolated
  hosted negative-test infrastructure under RFC-050.

## Non-Goals

- No member/admin audit-log UI.
- No audit delivery to a second database, queue, SIEM, or external service.
- No user-behavior analytics or read-access history.
- No audit row for ordinary member attendance changes, own-note edits/deletes,
  GET requests, or failed credential guesses.
- No attempt to make a disconnected HTTP download itself transactional with D1.
- No preservation of arbitrary legacy metadata at the cost of leakage risk.
- No generic transaction framework for unrelated features.
- No background `waitUntil()` or Queue substitute for required audit durability.
- No production fault-injection route or runtime fail-open switch.
- No B1–B4 implementation except coordinated tests and dependencies.

## Current Inventory and Classification

### Class A — atomic required audit

The following successful mutations must commit with their audit row:

| Surface | Canonical action | Protected state |
|---|---|---|
| Community creation | `community.created` | Community row |
| First-admin creation | `membership.created_first_admin` | Initial membership |
| Display-name update | `membership.display_name_updated` | Membership label |
| Invite generation | `invite_code.generated` | HMAC-only invite row |
| Invite revocation | `invite_code.revoked` | Invite revocation timestamp |
| Invite redemption/join | `invite_code.redeemed` | Invite claim, user, membership, session |
| Help-signin creation | `membership.relink_code_created` | Old-code revocation and new HMAC-only code |
| Help-signin redemption | `membership.relink_redeemed` | Code claim, new session, old-session revocation |
| Operator recovery creation | `operator_recovery.admin_relink_created` | Recovery code mutation |
| Member removal | `membership.removed` | Soft-removal transition |
| Role promotion | `membership.promoted_to_admin` | Role transition |
| Role demotion | `membership.demoted_to_member` | Guarded role transition |
| Event creation | `event.created` | Event, optional series, and day rows |
| Event edit | `event.edited` | Event and optional day update |
| Event cancellation | `event.cancelled` | Event lifecycle transition |
| Occurrence cancellation | `event.occurrence_cancelled` | Day and exception row |
| Bulk admin attendance | `attendance.admin_override` | Set of attendance upserts |
| Admin mark-attended | `attendance.admin_set_attended` | One attendance upsert |
| Admin note moderation | `event_note.admin_hidden` | Moderation timestamp |
| Calendar feed generation | `calendar_feed.token_generated` | Old-token revocation and new HMAC-only token |
| Calendar feed revocation | `calendar_feed.token_revoked` | Token revocation transition |
| Event-template creation | `event_template.created` | Template row |
| Event-template deletion | `event_template.deleted` | Template deactivation |

“Exactly the required audit row” means one row for a transition that actually
changes state. An already-applied/replayed/no-op transition must not manufacture
a second success audit. Community creation intentionally produces two events in
the same batch because it creates two independently significant objects.

### Class B — fail-closed before disclosure

| Surface | Canonical action | Rule |
|---|---|---|
| Community JSON export | `community.export_authorized` | Insert audit before returning the payload; `503` on failure. |
| Calendar matrix CSV | `calendar_matrix_csv.export_requested` | Insert audit before returning the browser authorization acknowledgement. |

The server can prove that it authorized or acknowledged a request; it cannot
prove that every response byte reached disk. The old `community.exported`
action is therefore renamed for future rows. An audit row may remain if response
construction or the client connection later fails; that is truthful
authorization-attempt evidence, not an orphaned mutation.

### Class C — safety-first secondary audit

| Surface | Canonical action | Rule |
|---|---|---|
| Logout | `session.logout` | Await server-side session revocation first. Attempt an audit row without session ID. Audit failure must not restore a credential or prevent cookie clearing. |

Logout is the only current exception to atomic required audit. Rolling back a
security revocation because the audit table is unavailable would preserve a
potentially stolen session and is less safe. The handler must:

1. await and require the session revocation result;
2. attempt the audit insert and await it;
3. on audit failure, emit a structured `audit.secondary_write_failed` incident
   containing request ID and action only;
4. clear the cookie and complete logout; and
5. never place the session ID in the audit target or log.

The exception is explicit, narrow, tested, and documented. It is not permission
for another handler to discard an audit result.

### Intentionally unaudited activity

- Ordinary member attendance/no-answer changes.
- A member's own note save/delete.
- Read-only GET requests and calendar-feed reads.
- Invalid invite/relink guesses; RFC-078 provides bounded aggregate events
  without credential or subject material.
- Authorization denials unless a separate incident rule emits a privacy-safe
  structured operational event.

## Decision

### Closed audit domain model

Replace the stringly typed writer with a closed model resembling:

```rust
pub enum AuditAction {
    InviteCodeGenerated,
    InviteCodeRedeemed,
    MembershipRemoved,
    EventEdited,
    // every reviewed action
}

pub enum AuditMetadata {
    None,
    ChangedFields(&'static [&'static str]),
    EventCreated { mode: EventCreationMode, source_event_id: Option<String> },
    EventEdited { scope: EventEditScope },
    AttendanceOverride { changed_count: u32 },
    OccurrenceCancelled { series_id: String, day_date: String },
    RelinkCorrelation { relink_code_id: String },
    MatrixExportRequested { month: String },
    OperatorRecovery { operator_label: String, relink_code_id: String },
}

pub struct AuditRecord { /* private validated fields */ }
```

Exact Rust names may vary. Required properties do not:

- `AuditAction` maps to one canonical globally namespaced string.
- Handlers cannot supply an arbitrary action string.
- Each action accepts only its matching metadata variant.
- Metadata fields have explicit types, formats, enums, and length bounds.
- `AuditRecord` validates target kind, optional identifiers, request ID, and
  action/metadata compatibility before preparing SQL.
- `AuditRecord` and metadata do not implement unrestricted debug output.
- The only production `INSERT INTO audit_log` SQL lives in the audit module.
- Database helpers receive a prepared audit record/statement; they do not
  reconstruct JSON or action strings.

Existing direct inserts in community creation, operator recovery, and display-
name update must migrate to this central statement builder. This ensures every
path shares validation, serialization, schema columns, and future migrations.

### Metadata allowlist

Identifiers already represented by `community_id`, `actor_membership_id`,
`target_kind`, or `target_id` are not duplicated in metadata.

Allowed metadata is deliberately small:

| Action family | Allowed metadata |
|---|---|
| Invite generation/revocation, membership removal/role, event cancel, template, calendar-token | `{}` |
| Invite redemption | `role_granted`, if needed; membership is in actor/target columns |
| Relink creation/redemption | `relink_code_id` only when required for RFC-069 recovery correlation; this is a random database record ID, never plaintext/HMAC |
| Operator recovery | fixed-format pseudonymous `operator_label`, `relink_code_id` |
| Event creation | enum `creation_mode`; optional `source_event_id` |
| Event edit | enum `edit_scope` or fixed `changed_fields`; never old/new values |
| Occurrence cancellation | `series_id`, ISO local `day_date`; event/day identifiers remain columns where possible |
| Attendance override | bounded integer `changed_count`; no member/status map |
| Admin note moderation | target membership ID only if no dedicated target column can represent it; never note content |
| Display-name update | `changed_fields: ["display_name"]`; never old/new names |
| Matrix export request | validated `YYYY-MM` month |
| Community JSON export | `{}` |

Event title is removed from audit metadata despite the old policy allowing it.
Titles, descriptions, display names, note text, and exported content already
exist in their business tables and must not become retained historical copies
inside `audit_log`.

`operator_label` is not free text or a person's name. It must match a bounded
ASCII identifier grammar such as `[a-z0-9][a-z0-9._-]{0,31}` and represent an
operator role/runbook identity. A separate incident reference belongs in the
operator's access-controlled incident record, not arbitrary audit metadata.

### Recursive defense-in-depth sanitizer

Serialization passes through a private recursive function before statement
construction. It traverses:

- every object value;
- every array element; and
- objects/arrays nested within arrays or objects.

Key comparison is ASCII case-insensitive. At minimum remove exact keys:

```text
password token secret code hmac session note memo body pepper cookie
authorization content description display_name email phone
```

Also remove credential-bearing patterns such as:

```text
*_token *_secret *_hmac *_hash *_password *_cookie
session_* authorization_*
```

`relink_code_id` is not removed because it is an explicitly allowed random
record identifier, not a code value. It is emitted only by the typed correlation
variant. No other arbitrary key containing `code` receives an exemption.

The sanitizer enforces:

- root value is an object;
- maximum depth 8;
- maximum 128 visited nodes;
- maximum 2,048 serialized UTF-8 bytes after sanitation; and
- valid JSON without non-finite numeric values.

Exceeding a bound or encountering action/metadata mismatch returns a typed
construction error. For Class A this prevents the mutation; for Class B it
prevents disclosure. The sanitizer never substitutes the unsanitized input and
never logs the rejected value.

The closed metadata model is the primary boundary. Recursive redaction is
defense in depth and a migration/testing guarantee, not justification for
accepting arbitrary handler JSON under benign-looking keys.

### D1 audit integrity migration

Add migration `0010_audit_integrity.sql`. Rebuild the audit table because
SQLite cannot add all required constraints in place. The target shape is
conceptually:

```sql
CREATE TABLE audit_log_v2 (
    id                   TEXT PRIMARY KEY,
    request_id           TEXT NOT NULL,
    community_id         TEXT,
    actor_membership_id  TEXT,
    target_kind          TEXT NOT NULL,
    target_id            TEXT,
    action               TEXT NOT NULL,
    metadata_json        TEXT NOT NULL DEFAULT '{}'
      CHECK (json_valid(metadata_json)
             AND json_type(metadata_json) = 'object'
             AND length(CAST(metadata_json AS BLOB)) <= 2048),
    created_at           TEXT NOT NULL,
    CHECK (length(id) BETWEEN 8 AND 64),
    CHECK (length(request_id) BETWEEN 1 AND 96),
    CHECK (length(target_kind) BETWEEN 1 AND 64),
    CHECK (length(action) BETWEEN 3 AND 96)
) STRICT;
```

The exact D1-compatible SQL must be proven locally and in the remote migration
rehearsal. Migration behavior:

1. preserve `id`, scope/actor/target columns, action, and `created_at`;
2. set `request_id = 'legacy'` for old rows;
3. set every legacy `metadata_json = '{}'` without inspecting or exporting its
   content;
4. swap tables only after the copy succeeds;
5. recreate indexes for `(community_id, created_at)` and `(action, created_at)`;
6. verify row count and core-column checksums/counts before dropping the old
   table within the migration; and
7. never print legacy metadata during rehearsal or evidence capture.

Resetting metadata is intentional data minimization. Existing metadata cannot
be proven safe because the old redactor was shallow and direct inserts bypassed
it. Core event identity and chronology remain available for incident review.

Backups made before RFC-079 must be classified as potentially containing
sensitive audit metadata. Access remains operator-only; retention/deletion is
reviewed without opening or copying metadata into evidence.

### Atomic D1 mutation pattern

Cloudflare documents D1 `batch()` statements as a SQL transaction: statements
execute sequentially and a statement error aborts or rolls back the sequence.
Class A helpers therefore prepare every business statement plus the central
audit statement and execute one `db.batch(...)`.

The audit statement is last unless a specific foreign-key dependency requires
otherwise. It must be a real statement in the same batch, not an awaited call
afterward and not `waitUntil()`.

Conditional mutations require care:

- mutation SQL repeats the narrow community, resource, actor/role, active-state,
  and expected-current-state predicates; a preceding handler authorization
  lookup is not the mutation boundary;
- audit insert is conditional on the exact successful post-state;
- no-op/replay results insert no audit row;
- handlers inspect returned `D1Result` changes to choose the response;
- a SQL error in the audit insert rolls back an earlier changed row; and
- application checks performed after a successful batch are not described as a
  rollback mechanism.

Flows whose correctness depends on “exactly one row changed” before later
statements—especially invite/relink claims—must include a database-enforced
transaction assertion. A small CHECK-constrained guard table or another
reviewed D1/SQLite statement may deliberately abort the batch when the claim
did not win. Merely checking `meta().changes` after `batch()` is too late to
roll back already committed statements.

No automatic retry wraps a mutating batch unless the whole operation has a
reviewed idempotency key and postcondition. A user whose form token was consumed
before an atomic batch failure receives a generic temporary-unavailable page
and may obtain a fresh form/token; the system must not return a success redirect.

### Multi-row flow requirements

The implementation must refactor these existing multi-write paths:

- **Join:** invite claim, user, membership, used-membership link, session, and
  audit in one guarded batch. A lost claim rolls back all candidate rows.
- **Relink redemption:** code claim, session insert, other-session revocation,
  and audit in one guarded batch.
- **Relink/calendar generation:** revoke active predecessors, insert replacement,
  and audit in one batch.
- **Event creation:** event, optional series, bounded day rows, and audit in one
  batch. The 64-occurrence cap remains; statement/query budget is verified with
  RFC-044.
- **Event edit/occurrence cancellation:** all related updates/inserts plus audit
  in one batch.
- **Bulk attendance override:** replace sequential per-cell commits with one
  bounded set-based statement or a proven bounded batch, followed by one audit
  statement. Reject an oversized request before mutation.

Community creation and operator recovery already demonstrate the intended D1
batch direction, but must use the central audit statement builder. Display-name
update already batches its writes; implementation review must ensure a failed
audit statement—not only a post-batch `changes` check—cannot leave an unaudited
reported success.

### Failure behavior and observability

Class A or B construction/D1 failure returns a generic Japanese `503 Service
Unavailable` with normal security headers, request ID, and
`Cache-Control: no-store`. It returns no success cookie, generated code,
download, or success redirect and reveals no SQL/action/storage detail.

Emit structured operational events:

```text
audit.required_batch_failed
audit.pre_disclosure_failed
audit.secondary_write_failed
audit.metadata_rejected
```

Allowed log fields:

- request ID;
- canonical action;
- failure category;
- route class;
- environment/build/Worker version.

Forbidden log fields:

- community, actor, membership, session, credential, event, note, or target IDs;
- metadata JSON or rejected key/value;
- SQL, bind values, D1 response bodies, cookies, tokens, codes, HMACs, or
  peppers; and
- event titles/descriptions, display names, note content, or export data.

Successful audit logs use the same structured shape with outcome only. Remove
the current console format that prints raw actor/community/target identifiers.
Required audit work is awaited because the response depends on it; it is never a
floating promise or background task.

Any required-audit failure in hosted staging or production is a security-
control incident and keeps RFC-050 E7/B5 evidence open.

The local correction centralizes `audit.required_batch_failed` ownership in
`audit.rs`. A failed Class A operation emits exactly one line containing only a
validated-or-sentinel request ID, canonical action, closed failure category,
and `route_class=class_a`. Invalid request IDs are replaced wholesale with
`invalid_request_id`. Class B and C keep their own event ownership and do not
also emit the Class A event. This local console/Worker emission is not
persistent delivery and does not close B5.

## Implementation Slices

After acceptance, implementation should remain reviewable:

1. Add the closed action/metadata model, validators, recursive sanitizer, and
   pure tests without changing callers.
2. Add/rehearse migration 0010, including destructive legacy-metadata reset and
   old-backup handling documentation.
3. Add central audit statement construction and structured success/failure
   logging; remove generic public `write(..., Value)`.
4. Convert simple single-row Class A mutations to mutation-plus-audit batches.
5. Convert guarded role/removal/invite actions with conditional audit semantics.
6. Convert join, relink, event, calendar-token, and bulk-attendance multi-row
   flows with database-enforced assertions and bounded batches.
7. Convert Class B exports and the Class C logout exception.
8. Remove every discarded audit result and every direct audit SQL outside the
   audit module.
9. Update RFC-014/RFC-052 policy, threat model, architecture, operations,
   release checklist, RFC-050 matrix, and contract gates.
10. Capture local real-D1 rollback evidence and exact-candidate isolated hosted
    evidence.

RFC-079 is large enough for a developer handoff after acceptance. The handoff
should split the work by domain helper and pin the inventory table so no caller
is lost during migration.

## Test and Evidence Plan

### Pure/native tests

- Every `AuditAction` has one canonical namespaced string and compatible
  metadata variant.
- Unknown/mismatched metadata fails construction.
- Allowed enum, date, month, identifier, count, and operator-label boundaries.
- Root, nested-object, nested-array, mixed-case, prefix, and suffix forbidden
  keys are removed recursively.
- `code`, `hmac`, `session`, and `memo` are explicitly covered at every depth.
- `relink_code_id` survives only through its typed variant; plaintext/HMAC
  fields cannot be represented.
- Depth 9, node 129, oversized JSON, non-object root, and invalid numbers fail.
- Event title/description, display name, note body, cookie, and session ID have
  no production metadata representation.
- Structured log formatter emits only allowed fields.

### Local real-D1 integration tests

Use a workerd/Wrangler-compatible D1 path; native mocks alone cannot prove
transaction rollback.

- Migration preserves row count/core columns and resets all legacy metadata.
- New table rejects invalid/non-object/oversized metadata.
- Install a test-only D1 trigger that aborts audit insertion; each representative
  Class A helper returns failure and leaves business state unchanged.
- Successful mutation produces exactly one action row.
- No-op/replay produces no new audit row.
- Join claim loss leaves no user, membership, session, or audit candidate rows.
- Relink claim loss leaves sessions unchanged.
- Event create failure leaves no event/series/day rows.
- Occurrence failure leaves neither day status nor exception partially changed.
- Bulk attendance failure rolls back every cell.
- Class B audit failure returns no export/acknowledgement.
- Class C audit failure still revokes the session and clears the cookie while
  producing the bounded incident event.
- Concurrent guarded mutations preserve one-winner and one-audit cardinality.

### Source/contract gates

- No `let _ = audit...`, ignored result, floating promise, or `waitUntil()` audit.
- `INSERT INTO audit_log` appears only in the central audit module/migration.
- No production audit API accepts arbitrary `serde_json::Value`.
- Every Class A action appears in the inventory and one mutation helper.
- Logout is the only declared secondary-audit exception.
- No audit console event includes raw actor/community/target/session IDs or
  metadata.
- Documentation denylist and typed allowlist match implementation constants.

### Hosted RFC-050 evidence

Against isolated negative-test D1/resources and then the exact candidate:

- force audit insertion failure without a deployable runtime flag;
- prove representative invite, relink, member moderation, event, attendance,
  note, and token mutations roll back;
- prove Class B data is not disclosed;
- prove logout revocation remains effective;
- inspect action cardinality under concurrent one-winner flows;
- query only action/count/core identifiers needed for verification;
- prove nested hostile metadata cannot be retained;
- observe privacy-safe structured failure events through persistent logging;
- verify migration/restore behavior and legacy metadata reset; and
- remove test triggers/resources and restore canonical exact-candidate state.

Evidence must not contain legacy metadata, submitted codes, HMACs, sessions,
cookies, notes, event content, personal labels, or raw resource IDs.

## Acceptance Criteria

RFC-079 implementation is complete only when:

1. Every current audit event is classified as Class A, B, C, or intentionally
   unaudited in durable documentation.
2. Every Class A mutation and its required audit row commit in one D1 batch; an
   audit SQL failure rolls back the complete mutation.
3. Class B returns no protected disclosure/authorization acknowledgement when
   its audit insert fails.
4. Logout revocation is awaited and remains effective during audit failure;
   that exception cannot spread to other actions.
5. No caller discards an audit result, backgrounds required audit, or directly
   inserts an audit row outside the central module.
6. Production metadata uses a closed action-specific schema and recursive
   defense-in-depth sanitation with the approved limits.
7. Legacy metadata is reset safely, core audit chronology is preserved, and old
   backups are treated as potentially sensitive.
8. Audit and Workers logs contain no bearer material, session identity, user
   content, unnecessary personal data, or raw actor/community/target IDs.
9. Pure, real-D1 rollback, concurrency, migration, source-gate, and hosted
   negative evidence passes.
10. RFC-014, RFC-052, RFC-071, RFC-050, threat model, operations, and release
    checklist claims match the implementation exactly.

Moving the implementation to `done/` establishes local source/config/migration
completion. B5 remains open for public/production pilot until RFC-050 captures
exact-candidate hosted rollback, redaction, logging, migration, and teardown
evidence.

## Rollout and Rollback

Rollout order:

1. approve the action inventory, logout exception, metadata allowlist, and
   legacy-metadata reset;
2. implement and locally prove the closed audit module;
3. rehearse migration 0010 against a synthetic/controlled copy without printing
   metadata;
4. convert and test every Class A/B/C call site;
5. deploy to isolated staging and apply migration;
6. run RFC-050 negative rollback/redaction evidence;
7. verify persistent incident events and audit query usability; and
8. proceed toward a pilot only with final security approval.

Migration rollback must not restore unsafe legacy metadata or best-effort
required audit behavior. If the new path fails, protected mutations/exports
return `503` while the operator rolls forward. Logout continues its safety-first
revocation behavior.

D1 backups before migration remain access-controlled. After Package 7, code
rollback retains the new table schema and must not restore the removed
compatibility adapter, arbitrary metadata, shallow redaction, ignored audit
results, or raw-identifier logging. Recovery rolls forward from the closed
boundary.

## Alternatives Rejected

### Await the current audit call after mutation

Rejected. Awaiting detects failure but cannot roll back a business mutation
that already committed.

### Keep all audit writes best-effort for availability

Rejected. It contradicts the required audit trail and lets successful
administrative/security changes become untraceable. The only current exception
is logout because rolling back revocation is less safe.

### Put required audit in `waitUntil()` or a Queue

Rejected. Background/at-least-once work is not atomic with D1 business state and
can be dropped, duplicated, delayed, or fail after success is returned.

### Use only a larger recursive denylist

Rejected as the primary boundary. Sensitive values can hide behind benign keys,
and arbitrary metadata grows without review. Typed per-action allowlists are the
primary control; recursive redaction remains defense in depth.

### Preserve and recursively clean all legacy metadata

Rejected. The old API allowed arbitrary shapes and direct inserts. A generic
cleaner cannot prove that a value under an innocent key is non-sensitive.
Resetting metadata to `{}` preserves the reliable event skeleton with lower
privacy risk.

### Audit exports after returning the response

Rejected. A failed insert could accompany a successful disclosure. Audit before
authorization/response and name the event honestly as authorized/requested.

## Current Platform References

- [Cloudflare D1 Database `batch()` documentation](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch).
- [Cloudflare D1 Workers Binding API](https://developers.cloudflare.com/d1/worker-api/).
- [Cloudflare Workers best practices](https://developers.cloudflare.com/workers/best-practices/workers-best-practices/).
- Installed worker-rs 0.8.4 D1 binding API.
