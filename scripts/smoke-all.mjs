#!/usr/bin/env node
// Handoff 063 (F2 of the RFC-083 Slice D1a review): run every self-contained
// smoke by name and report one derived summary. Before this script existed,
// "how many smokes are green" was a number carried by hand from package to
// package — and it drifted: "sixteen smokes" was quoted in several recent
// review requests while the real, runnable set was twenty. A package's
// evidence section must quote this script's output, not a remembered count.
//
// `smoke:runtime` is excluded — it requires an explicit URL argument and
// isn't a self-contained smoke. `test:abuse-controls` is included even
// though its own package.json name doesn't start with `smoke:` — leaving it
// out on a naming technicality would silently recreate the exact defect
// this script exists to fix.

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));

const EXCLUDED = new Set(['smoke:runtime', 'smoke:all']);
const EXTRA_INCLUDED = ['test:abuse-controls'];

const names = [
  ...Object.keys(pkg.scripts).filter((name) => name.startsWith('smoke:') && !EXCLUDED.has(name)),
  ...EXTRA_INCLUDED,
].sort();

console.log(`[smoke-all] running ${names.length} smokes: ${names.join(', ')}`);

const results = [];
for (const name of names) {
  const startedAt = Date.now();
  console.log(`\n[smoke-all] === ${name} ===`);
  const proc = spawnSync('bun', ['run', name], {
    cwd: repoRoot,
    stdio: 'inherit',
  });
  const durationMs = Date.now() - startedAt;
  const passed = proc.status === 0;
  results.push({ name, passed, exitCode: proc.status, durationMs });
  console.log(
    `[smoke-all] ${name}: ${passed ? 'PASS' : 'FAIL'} (exit ${proc.status}, ${durationMs}ms)`,
  );
}

const passedCount = results.filter((r) => r.passed).length;
const failed = results.filter((r) => !r.passed);

console.log('\n[smoke-all] ==================== summary ====================');
console.log(`[smoke-all] total run: ${results.length}`);
console.log(`[smoke-all] total passed: ${passedCount}`);
if (failed.length > 0) {
  console.log(`[smoke-all] failures (${failed.length}):`);
  for (const r of failed) {
    console.log(`[smoke-all]   - ${r.name} (exit ${r.exitCode})`);
  }
} else {
  console.log('[smoke-all] failures: none');
}

console.log(
  JSON.stringify(
    {
      totalRun: results.length,
      totalPassed: passedCount,
      failures: failed.map((r) => r.name),
      results,
    },
    null,
    2,
  ),
);

if (failed.length > 0) process.exitCode = 1;
