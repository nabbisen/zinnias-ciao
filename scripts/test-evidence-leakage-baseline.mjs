#!/usr/bin/env node
// Handoff 068: turns `scan-evidence-leakage` from a script nobody runs into a
// gate that fails on anything new. `scan-evidence-leakage.mjs` alone reports
// findings but never fails a routine run unless someone chooses to invoke
// it — the same "control nobody runs, backlog accumulates unnoticed" shape
// this project has already fixed three times (`LOCALIZATION_EXCEPTIONS`,
// the smoke run set, EN/JA parity). This script pins the known backlog as an
// exact baseline (total and per category) and fails if the measured tree
// differs in EITHER direction — a rise is a new, unreviewed violation; a
// fall means the backlog shrank and the pin is stale and must come down.
// Wired into `bun run smoke:all` (see `scripts/smoke-all.mjs`'s
// `EXTRA_INCLUDED`) so it runs as part of the routine check, not on request.
//
// No detection logic lives here — that's `scripts/lib/evidence-manifest.mjs`
// (Handoff 067 owns it). This script only measures and compares.

import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { scanEvidenceTree } from './scan-evidence-leakage.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const target = join(root, '.git-exclude', 'evidence');

// Measured after Handoff 067 landed (`forbidden_key` narrowed from 110 to 5).
// Every category the scanner's error classes can produce is pinned
// explicitly, including the ones currently at zero — a category going from
// 0 to nonzero is exactly the "new violation type" this gate exists to
// catch, not something a missing key should let through silently.
const PINNED_BASELINE = {
  total: 946,
  categories: {
    raw_resource_id: 484,
    raw_or_hashed_secret: 450,
    forbidden_key: 5,
    sql: 7,
    registered_run_secret: 0,
    malformed_exempt_field: 0,
    cookie: 0,
    d1_error_body: 0,
    har_file: 0,
    unparseable_json: 0,
  },
};

const result = await scanEvidenceTree(target, root);

const measuredCategories = {};
for (const finding of result.findings) {
  measuredCategories[finding.category] = (measuredCategories[finding.category] ?? 0) + 1;
}

const allCategoryNames = new Set([
  ...Object.keys(PINNED_BASELINE.categories),
  ...Object.keys(measuredCategories),
]);

const mismatches = [];
for (const category of [...allCategoryNames].sort()) {
  const pinned = PINNED_BASELINE.categories[category] ?? 0;
  const measured = measuredCategories[category] ?? 0;
  if (pinned !== measured) {
    mismatches.push({
      category,
      pinned,
      measured,
      direction: measured > pinned ? 'rose' : 'fell',
    });
  }
}

const totalMeasured = result.findings.length;
const totalMatches = totalMeasured === PINNED_BASELINE.total;
const categoriesMatch = mismatches.length === 0;
const passed = totalMatches && categoriesMatch;

const report = {
  ok: passed,
  target: result.target,
  pinnedTotal: PINNED_BASELINE.total,
  measuredTotal: totalMeasured,
  pinnedCategories: PINNED_BASELINE.categories,
  measuredCategories,
  mismatches,
};

console.log(JSON.stringify(report, null, 2));

if (!totalMatches) {
  const direction = totalMeasured > PINNED_BASELINE.total ? 'rose' : 'fell';
  console.error(
    `[evidence-leakage-baseline] FAIL: total findings ${direction} from the pinned `
    + `baseline (${PINNED_BASELINE.total} -> ${totalMeasured}).`
    + (direction === 'rose'
      ? ' A new, unreviewed violation appeared — investigate it, do not re-pin upward.'
      : ' The backlog shrank — re-pin PINNED_BASELINE.total downward to the new, lower count.'),
  );
}
for (const mismatch of mismatches) {
  console.error(
    `[evidence-leakage-baseline] FAIL: category "${mismatch.category}" ${mismatch.direction} `
    + `from the pinned baseline (${mismatch.pinned} -> ${mismatch.measured}).`
    + (mismatch.direction === 'rose'
      ? ' A new, unreviewed violation appeared — investigate it, do not re-pin upward.'
      : ' This category shrank — re-pin it downward in PINNED_BASELINE.categories.'),
  );
}
if (passed) {
  console.error('[evidence-leakage-baseline] OK: matches the pinned baseline exactly.');
}

if (!passed) process.exitCode = 1;
