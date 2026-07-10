# RFC 068 - Calendar Matrix CSV Export

**Status.** Done  
**Target release.** v0.57.0  
**Tracks.** Calendar workflow, admin reporting, privacy boundary, client-side
export.  
**Touches.** `workers/ssr/src/handlers/communities`, Calendar matrix rendering,
static app JavaScript, release gates, browser smoke.

## Summary

RFC-068 adds CSV export for the monthly attendance matrix introduced by
RFC-067.

The export is intentionally narrower than the shared matrix view:

- the matrix view itself remains visible to active community members and
  admins;
- CSV export is visible and usable only by active community admins;
- CSV generation happens in the browser from the rendered matrix table;
- no server-side CSV generation endpoint is added in the first implementation;
- a minimal admin-only audit request records that export was requested without
  uploading matrix contents;
- no member notes, invite data, session data, feed tokens, audit data, or
  hidden export-only payloads are included in the CSV/export payload.

The first export should mirror the visible monthly matrix table: one row per
active member and one column per date in the selected month.

## Background

RFC-067 deliberately excluded CSV export because export creates a stronger
privacy and portability boundary than on-screen viewing. Active members can see
attendance status inside their community, but exporting the whole month creates
a file that can be copied, shared, stored, and processed outside the app.

The project owner requested CSV export for the monthly matrix with two
constraints:

- CSV export should be split from the matrix GUI work because the matrix layout
  needed its own review and adjustment;
- CSV generation should be a client function rather than a server function to
  avoid unnecessary Cloudflare CPU-time risk.

RFC-068 records the admin-only export contract and the client-side generation
strategy before implementation.

## Goals

- Add an admin-only CSV export action to the Calendar matrix view.
- Export the selected month's matrix in a predictable table shape.
- Keep the export bounded to one active community and one visible month.
- Generate CSV in the browser from rendered, admin-visible matrix data.
- Avoid a server CSV endpoint and avoid additional server CPU for formatting.
- Preserve RFC-027's auditable-export policy with a minimal admin-only audit
  request that sends no matrix contents.
- Preserve RFC-067 member visibility for the matrix itself.
- Prevent CSV formula injection in spreadsheet tools.
- Keep Japanese UI copy short and clear.
- Add release gates and browser smoke for permission, content, and download
  behavior.

## Non-Goals

RFC-068 does not add:

- member-accessible CSV export;
- server-generated CSV;
- unaudited admin CSV download through the normal UI;
- asynchronous export jobs;
- ZIP export;
- PDF export;
- print layout;
- export of member notes;
- export of invite codes, feed tokens, sessions, audit rows, or internal-only
  data;
- former-member historical reporting;
- cross-community export;
- arbitrary date-range export;
- inline attendance editing;
- notification/reminder features.

Former-member historical export may be useful later, but it requires a separate
privacy design because it exposes membership history. Arbitrary date-range
export may also be useful later, but the first release should stay aligned with
the already-reviewed monthly matrix.

## Terms

| Term | Meaning |
|------|---------|
| Matrix export | A CSV representation of the monthly attendance matrix. |
| Export admin | An active community member whose role is `admin` in the active community. |
| Export month | The month selected on the Calendar matrix page. |
| Summary CSV | A CSV whose cells mirror the visible matrix symbols or multi-event summaries. |
| Formula injection | Spreadsheet execution risk when a cell begins with characters such as `=`, `+`, `-`, or `@`. |

## External Behavior

### Entry Point

The Calendar matrix page remains:

```text
/c/:community_id/communities?month=YYYY-MM&view=matrix
```

For active community admins only, the matrix view shows a CSV export action
near the matrix heading or month controls.

Japanese reference copy:

```text
CSVを保存
```

English reference copy:

```text
Save CSV
```

The action must not appear in the ordinary Calendar month view unless that view
also links to matrix mode. The export belongs to the matrix, not the generic
Calendar page.

### Authorization

CSV export is community-admin-only.

- Active admins see the export action on matrix mode.
- Active non-admin members can view the matrix but do not see the export
  action.
- Anonymous, removed, or non-member users follow existing Calendar access
  behavior and cannot reach matrix data.

Because the first implementation has no server CSV endpoint, the primary
authorization boundary is rendered UI plus server-rendered export metadata:
only admin responses should include the export action and any export-specific
attributes.

The design must not include a hidden export payload in member-visible matrix
HTML. If a future implementation adds a server endpoint or hidden export model,
that endpoint/model must re-check admin authorization server-side.

The admin export action must also be auditable. The first implementation should
use a tiny admin-only audit request before starting the browser download. That
request re-checks active admin authorization and writes an audit event, but it
does not upload matrix contents or ask the server to format CSV.

### Export Shape

The first CSV is a summary CSV that mirrors the visible matrix table.

Columns:

1. `member_name`
2. one column per date in the selected month, formatted as `YYYY-MM-DD`

Rows:

- one row per active community member;
- row order matches the matrix row order;
- removed members are not included.

Cell values:

- empty string for dates with no event days;
- `○` for going;
- `×` for not going;
- `済` for attended;
- `?` for no reply;
- `中` for all-cancelled or single cancelled date;
- `answered/total` for multiple non-cancelled event days on the same date.

The CSV does not include event titles or selected-date detail in v0.57.0. Those
are intentionally omitted to keep the export exactly aligned with the visible
matrix grid and avoid defining a larger reporting data contract in the first
export release.

### Filename

The downloaded filename should be deterministic and avoid user-controlled raw
text:

```text
ciao-attendance-YYYY-MM.csv
```

The community name should not be included in the filename in the first
implementation. Community names can contain user-provided text and are not
needed for a single-community in-app export action.

### Encoding and CSV Format

The CSV should use:

- UTF-8;
- a UTF-8 BOM if needed for common spreadsheet compatibility;
- comma separator;
- CRLF row endings;
- RFC-4180-style double-quote escaping;
- a header row.

All values should be quoted or escaped by one shared CSV encoder rather than
assembled with string concatenation.

### Formula Injection

Every exported cell must be spreadsheet-safe.

If a cell value begins, after leading whitespace, with any of:

```text
=
+
-
@
```

the CSV encoder should prefix a single quote (`'`) before CSV quoting. This
must apply to member display names and any future text cells. Current status
symbols are not formula risks, but the encoder should be generic so later text
fields do not bypass it.

### JavaScript and No-JS Behavior

CSV generation is a progressive enhancement.

- With JavaScript enabled, admins can save the CSV after the audit request
  succeeds.
- With JavaScript disabled, the export action may be hidden or disabled with
  plain copy.
- No server fallback is required in v0.57.0.

This is acceptable because RFC-068 is an export convenience, not the primary
Calendar viewing workflow.

## Internal Design

### Rendering Contract

The matrix table should expose export-readable attributes only on admin
responses.

Recommended shape:

- the table or scroller has a stable attribute such as
  `data-calendar-matrix-export="true"`;
- header cells expose date values through `data-date="YYYY-MM-DD"`;
- member header cells expose the display name through text content or a
  `data-member-name` attribute;
- matrix cells expose the export value through `data-export-value`.

The implementation must use `data-export-value` for matrix cells. Reading
visible cell text is not acceptable because empty dates and layout-only content
can render as non-breaking spaces or incidental whitespace rather than the CSV
contract's empty string.

The implementation should prefer data attributes for other values where visible
text may contain spacing or accessibility-only content.

These export attributes must not be emitted for non-admin matrix responses.
Non-admin members should receive only the normal visible matrix and accessible
labels from RFC-067.

### Client-Side Export Module

The existing `static/app.js` should own the browser behavior.

Recommended behavior:

1. find an admin-visible export button;
2. find the associated exportable matrix table;
3. read header dates and row/cell export values;
4. build CSV with the shared encoder;
5. send an admin-only audit request containing only export type and month;
6. if the audit succeeds, create a `Blob` with `text/csv;charset=utf-8`;
7. trigger a temporary object URL download;
8. revoke the object URL after use.

If the audit request fails, the UI should show the short export-unavailable
copy and must not trigger the download through the normal button flow.

The code should not depend on a framework and should remain compatible with
the existing static asset/service-worker setup.

### Admin Audit Request

The first implementation should add a small server route for audit only, not
CSV generation.

Recommended shape:

```text
POST /c/:community_id/admin/calendar/matrix-export/audit
```

The request body should include only:

- form token;
- selected month in `YYYY-MM`;
- export type, fixed to a reviewed value such as `calendar_matrix_csv`.

The route must:

- require an authenticated active admin in the active community;
- validate the form token with a dedicated purpose;
- validate the month shape;
- write an audit row such as `calendar_matrix_csv.export_requested`;
- avoid writing matrix contents, member names, attendance values, notes, CSV
  bytes, or user-controlled filenames to the audit row;
- return a small success/failure response suitable for `fetch`.

This route is not a CSV endpoint. It does not return CSV and it does not query
or format the matrix data.

### Server Data

No new D1 query is required for matrix data in the first implementation.

The server already renders the matrix from:

- active members;
- visible-month event days;
- batched attendance rows.

The export reuses the same server-rendered matrix values. It should not fetch
additional matrix data over the network. The only network request during export
is the admin audit request described above.

### Caps and Fallback

RFC-067 caps remain unchanged:

- active member rows render up to 100;
- visible-month event-day rows render up to 300;
- over-cap months show the too-large fallback.

If the matrix is too large and the table is not rendered, CSV export should not
be available. The fallback copy should continue to direct users to Calendar
view rather than promising an export.

### Route and Switcher State

No new route mode is required.

The export action is available only on:

```text
view=matrix
```

Community switching continues to use the RFC-067 matrix `next` grammar. CSV
export state is not preserved as a separate mode because export is an action,
not a view.

## Options Considered

### Option A - Client-side summary CSV from rendered matrix

This RFC's recommended first implementation.

Benefits:

- avoids server-side CSV CPU work;
- reuses the already-reviewed matrix data contract;
- needs no server CSV route or matrix D1 query;
- preserves export auditability with a small metadata-only request;
- simple to smoke-test;
- keeps non-admin responses free of export metadata.

Tradeoffs:

- exports compact summaries, not per-event detail;
- depends on JavaScript;
- adds one small audit request before download;
- a future detailed export may need a separate data contract.

### Option B - Server-generated admin CSV endpoint

Potential later option.

Benefits:

- can work without JavaScript;
- can stream or construct a richer data contract;
- can centralize authorization on the server endpoint.

Tradeoffs:

- adds Cloudflare CPU/subrequest pressure;
- adds a new route and authorization surface;
- duplicates matrix data shaping or creates a second reporting model;
- higher security review cost.

### Option C - Detailed client-side CSV with per-event columns

Potential later option.

Benefits:

- more informative for admins;
- can answer exactly which event caused an `answered/total` value.

Tradeoffs:

- no longer mirrors the visible monthly table;
- greatly increases column count for busy months;
- needs stable event labels and duplicate-title handling;
- risks turning v0.57.0 into a reporting redesign rather than export of the
  existing matrix.

## Security, Privacy, and Safety

- CSV export is admin-only.
- The export must remain scoped to the active community.
- Non-admin matrix responses must not include export buttons or hidden export
  data attributes.
- Removed members are excluded by default.
- Former-member historical reporting is out of scope.
- Member notes are excluded.
- Invite codes are excluded.
- Calendar feed tokens are excluded.
- Session data is excluded.
- Audit data is excluded from the CSV.
- Internal IDs are excluded from CSV values.
- Export cells are formula-injection hardened.
- The export action must not send matrix data to a server or third party.
- The export action must write an audit record through a metadata-only
  admin-checked request before the normal UI download starts.

The generated CSV is a portable file. The UI copy should be modest and should
not imply that the app can control the file after download.

## Accessibility and UX

- The export action should be a normal button, not a link to a fake route.
- The button should have a clear accessible name.
- Export should not disturb the selected matrix date.
- Export should not require opening a modal in v0.57.0.
- If export fails because the table is missing or unsupported, show a plain
  short error near the button or use a non-disruptive status message.
- The button should remain reachable at mobile width and 200% text scaling.
- The export action should not create page-wide horizontal overflow.

## Copy Contract

Japanese copy should stay short.

| Surface | Japanese copy | English reference |
|---------|---------------|-------------------|
| Export button | `CSVを保存` | Save CSV |
| Export unavailable | `CSVを保存できません。` | CSV cannot be saved. |
| Admin-only note, if needed | `CSV保存は管理者のみです。` | CSV export is admin-only. |

The admin-only note is optional. It should not be shown to ordinary members if
that would clutter the shared matrix; hiding the button for non-admin members
is preferred.

## Acceptance Criteria

- Active community admins can save a CSV from Calendar matrix mode.
- Active non-admin members can view the matrix but do not see or receive export
  controls or export-only attributes.
- Non-members cannot use export to access matrix data.
- CSV generation is client-side and does not call a server CSV/data endpoint.
- The normal export button flow records a metadata-only audit event before
  triggering the download.
- The CSV mirrors the visible matrix table with `member_name` plus one column
  per date in the selected month.
- Cell values follow RFC-067 matrix summary values.
- CSV values are UTF-8 and spreadsheet-formula hardened.
- Over-cap months do not expose export because no complete matrix is rendered.
- The export button is usable at mobile width and 200% text scaling.
- Release gates prevent member-visible hidden export payloads.
- Browser smoke verifies admin export, non-admin absence, CSV content, formula
  hardening, no network call to a CSV/data endpoint, and one metadata-only
  audit request.

## Test Plan

- Unit tests for the CSV encoder in JavaScript, if the project test structure
  supports it without adding heavy tooling.
- CSV encoder tests should cover:
  - member names beginning directly with `=`, `+`, `-`, and `@`;
  - member names with leading spaces or tabs before `=`, `+`, `-`, and `@`;
  - normal symbols and `answered/total` values remaining unchanged;
  - double quotes, commas, CR, and LF encoded by the same CSV encoder.
- Source/release gates:
  - export button/copy appears only in admin-conditioned rendering;
  - non-admin matrix source path does not include export-only attributes;
  - matrix cells use `data-export-value` and the client does not read visible
    cell text for CSV values;
  - no server route named `export_matrix` or similar CSV/data endpoint is
    added for RFC-068;
  - a metadata-only audit route exists and re-checks active admin access;
  - `app.js` contains formula-injection hardening for CSV cells;
  - i18n EN/JA parity includes export copy if copy constants are added.
- Rendering tests:
  - admin matrix HTML includes export control and data attributes;
  - member matrix HTML excludes export control and export attributes;
  - too-large matrix fallback excludes export control.
- Browser smoke:
  - admin opens matrix, clicks export, and a CSV download is produced;
  - CSV header contains `member_name` and selected-month dates;
  - CSV rows match visible member order and cell summaries;
  - a member display name beginning with a formula character is hardened;
  - leading whitespace before a formula character is hardened;
  - non-admin member sees the matrix but no export action;
  - smoke confirms no request to a CSV/data endpoint during export;
  - smoke confirms one metadata-only audit request before download;
  - mobile 390px with 200% text scaling keeps export action usable.
- Source gates:
  - `cargo fmt --all -- --check`
  - `git diff --check`
  - `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace`
  - `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
  - `mdbook build docs`

## Rollout Notes

RFC-068 can ship as a normal source release after review and smoke evidence.
No migration is expected.

Hosted staging smoke is recommended before production use because download
behavior and browser file handling can vary by environment and device.

## Deferred Decisions

- Detailed per-event CSV export.
- Former-member historical export.
- Server-generated export endpoint.
- Arbitrary date range export.
- Including community name in filename.
- Locale-specific spreadsheet formats beyond UTF-8 CSV.
