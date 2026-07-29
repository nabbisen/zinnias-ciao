#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 8): release-candidate
// attestation gate-rule checker. Takes the path to a filled-in copy of
// `docs/src/tester/release-candidates/TEMPLATE.md` and mechanically enforces
// the gate rules that must not be left to prose review alone — see
// `scripts/lib/release-candidate-attestation.mjs` for the exact rules
// (E4a -> E4 dependency, closed verdict vocabulary, the RFC-078 criterion 6
// IPv6 risk-acceptance wording).
//
// Local-only: read-only over the given file. No hosted call, no mutation.

import { readFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkAttestationGates } from './lib/release-candidate-attestation.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetArg = process.argv[2];

if (!targetArg) {
  console.error('Usage: node scripts/check-release-candidate-attestation.mjs <path-to-attestation.md>');
  process.exit(2);
}

const target = resolve(targetArg);
const relTarget = relative(root, target);

let markdown;
try {
  markdown = await readFile(target, 'utf8');
} catch (error) {
  console.log(JSON.stringify({ ok: false, target: relTarget, error: `could not read file: ${error.code ?? error.message}` }));
  process.exit(1);
}

let violations;
try {
  violations = checkAttestationGates(markdown);
} catch (error) {
  console.log(JSON.stringify({ ok: false, target: relTarget, error: error.message }));
  process.exit(1);
}

const passed = violations.length === 0;
console.log(JSON.stringify({ ok: passed, target: relTarget, violations }, null, 2));
if (!passed) process.exitCode = 1;
