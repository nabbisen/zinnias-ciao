#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 8): pins the release-candidate
// attestation gate rules — the E4a -> E4 dependency, the closed verdict
// vocabulary, and the RFC-078 criterion 6 IPv6 risk-acceptance wording — and
// proves the shipped TEMPLATE.md itself validates cleanly out of the box.
// No hosted call, no filesystem mutation.

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  AttestationParseError,
  checkAttestationGates,
  parseGateTable,
  parseRiskAcceptanceTable,
} from './lib/release-candidate-attestation.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const RISK_TABLE = (ipv6Status = 'Risk-Accepted-Open') => `
## Carried risk acceptances

| Risk | Status | Reference |
|---|---|---|
| RFC-078 criterion 6 — IPv6 \`/64\` sub-clause | ${ipv6Status} | owner risk-accepted 2026-07-28 |
`;

function buildFixture({ e4Local = '', e4Hosted = '', e4aLocal = '', e4aHosted = '', ipv6Status = 'Risk-Accepted-Open', omitIpv6Row = false } = {}) {
  const risk = omitIpv6Row
    ? `\n## Carried risk acceptances\n\n| Risk | Status | Reference |\n|---|---|---|\n`
    : RISK_TABLE(ipv6Status);
  return `
## Gate verdicts

| Gate | Local Evidence | Hosted Evidence | Notes |
|---|---|---|---|
| E0 — local candidate freeze | Pass | N/A — local-only gate | |
| E4a — direct-ingress topology and client-identity classification | ${e4aLocal} | ${e4aHosted} | Gates E4 |
| E4 — concurrency and fail-closed controls | ${e4Local} | ${e4Hosted} | Void unless E4a Hosted = Pass |
${risk}`;
}

// -- gate table parsing ------------------------------------------------------

{
  const gates = parseGateTable(buildFixture({ e4Local: 'Pass', e4Hosted: 'Pass', e4aLocal: 'Pass', e4aHosted: 'Pass' }));
  assert.equal(gates.get('E4').hosted.verdict, 'Pass');
  assert.equal(gates.get('E4a').hosted.verdict, 'Pass');
  assert.equal(gates.get('E0').hosted.verdict, 'N/A');
}

assert.throws(
  () => parseGateTable('# no table here at all'),
  (error) => error instanceof AttestationParseError,
  'parseGateTable throws AttestationParseError when the Gate table is absent',
);

assert.throws(
  () => parseRiskAcceptanceTable('# no table here at all'),
  (error) => error instanceof AttestationParseError,
  'parseRiskAcceptanceTable throws AttestationParseError when the Risk table is absent',
);

// -- E4a -> E4 dependency -----------------------------------------------------

{
  // E4 Hosted = Pass, E4a Hosted = Pass for the same candidate: no violation.
  const violations = checkAttestationGates(buildFixture({ e4Hosted: 'Pass', e4aHosted: 'Pass' }));
  assert.deepEqual(
    violations.filter((v) => v.rule === 'e4a_gates_e4'),
    [],
    'E4=Pass is valid when E4a Hosted is also Pass',
  );
}

{
  // E4 Hosted = Pass but E4a Hosted is only Pending: E4's pass is void, must flag.
  const violations = checkAttestationGates(buildFixture({ e4Hosted: 'Pass', e4aHosted: 'Pending' }));
  const hit = violations.find((v) => v.rule === 'e4a_gates_e4');
  assert.ok(hit, 'E4=Pass with E4a Hosted != Pass must be flagged as a dependency violation');
  assert.equal(hit.gate, 'E4');
}

{
  // E4 Hosted correctly recorded as Void (not claiming Pass) while E4a is
  // still Pending: no dependency violation, because no Pass is being claimed.
  const violations = checkAttestationGates(buildFixture({ e4Hosted: 'Void', e4aHosted: 'Pending' }));
  assert.deepEqual(
    violations.filter((v) => v.rule === 'e4a_gates_e4'),
    [],
    'E4=Void with E4a still Pending is not itself a dependency violation',
  );
}

{
  // E4 blank (no claim yet): no dependency violation regardless of E4a.
  const violations = checkAttestationGates(buildFixture({ e4Hosted: '', e4aHosted: '' }));
  assert.deepEqual(
    violations.filter((v) => v.rule === 'e4a_gates_e4'),
    [],
    'a blank E4 Hosted cell makes no claim and cannot violate the dependency',
  );
}

// -- closed vocabulary --------------------------------------------------------

{
  const violations = checkAttestationGates(buildFixture({ e4Hosted: 'Looks good to me', e4aHosted: 'Pass' }));
  const hit = violations.find((v) => v.rule === 'closed_vocabulary' && v.gate === 'E4');
  assert.ok(hit, 'a Hosted cell that does not start with a verdict word must be flagged');
}

// -- R2: no known-local citation in a Hosted cell -----------------------------

{
  const violations = checkAttestationGates(buildFixture({ e4Hosted: 'Pass — cargo test output', e4aHosted: 'Pass' }));
  const hit = violations.find((v) => v.rule === 'hosted_cites_local_evidence' && v.gate === 'E4');
  assert.ok(hit, 'a Hosted cell citing "cargo test" must be flagged as a local citation');
}

{
  const violations = checkAttestationGates(
    buildFixture({ e4Hosted: 'Pass — see docs/src/tester/evidence-templates/60-observability-and-runtime.md', e4aHosted: 'Pass' }),
  );
  const hit = violations.find((v) => v.rule === 'hosted_cites_local_evidence' && v.gate === 'E4');
  assert.ok(hit, 'a Hosted cell citing a docs/ path must be flagged as a local citation');
}

{
  const violations = checkAttestationGates(
    buildFixture({ e4Hosted: 'Pass — scripts/collect-evidence-e4-concurrency.mjs local run', e4aHosted: 'Pass' }),
  );
  const hit = violations.find((v) => v.rule === 'hosted_cites_local_evidence' && v.gate === 'E4');
  assert.ok(hit, 'a Hosted cell citing a scripts/ path must be flagged as a local citation');
}

{
  const violations = checkAttestationGates(
    buildFixture({
      e4Hosted: 'Pass — hosted run 2026-08-01, see .git-exclude/evidence/rc1/30-concurrency.json',
      e4aHosted: 'Pass',
    }),
  );
  assert.deepEqual(
    violations.filter((v) => v.rule === 'hosted_cites_local_evidence'),
    [],
    'a genuine hosted-evidence-directory citation must not be flagged as local',
  );
}

{
  // N/A is exempt (E0 structurally has no hosted requirement, and its
  // template text itself says "local-only gate").
  const violations = checkAttestationGates(buildFixture());
  assert.deepEqual(
    violations.filter((v) => v.rule === 'hosted_cites_local_evidence'),
    [],
    'the N/A cell for a local-only gate must not itself be flagged as a local citation',
  );
}

// -- IPv6 risk acceptance -----------------------------------------------------

{
  const violations = checkAttestationGates(buildFixture({ ipv6Status: 'Pass' }));
  const hit = violations.find((v) => v.rule === 'ipv6_risk_acceptance_status');
  assert.ok(hit, 'the IPv6 risk acceptance must never be recorded as Pass');
}

{
  const violations = checkAttestationGates(buildFixture({ ipv6Status: 'Hosted-Proven' }));
  const hit = violations.find((v) => v.rule === 'ipv6_risk_acceptance_status');
  assert.ok(hit, 'the IPv6 risk acceptance must never be recorded as hosted-proven');
}

{
  const violations = checkAttestationGates(buildFixture({ omitIpv6Row: true }));
  const hit = violations.find((v) => v.rule === 'ipv6_risk_acceptance_missing');
  assert.ok(hit, 'a missing IPv6 risk-acceptance row must be flagged, not silently accepted');
}

{
  const violations = checkAttestationGates(buildFixture({ ipv6Status: 'Risk-Accepted-Open' }));
  assert.deepEqual(
    violations.filter((v) => v.rule.startsWith('ipv6')),
    [],
    'the correct IPv6 risk-acceptance wording produces no violation',
  );
}

// -- the shipped TEMPLATE.md itself validates cleanly out of the box ---------

{
  const templatePath = resolve(root, 'docs/src/tester/release-candidates/TEMPLATE.md');
  const templateText = await readFile(templatePath, 'utf8');
  const violations = checkAttestationGates(templateText);
  assert.deepEqual(violations, [], `TEMPLATE.md must validate with zero violations out of the box, got: ${JSON.stringify(violations)}`);
}

console.log(JSON.stringify({
  ok: true,
  phases: {
    gateTableParsing: true,
    missingTableThrowsParseError: true,
    e4aGatesE4Dependency: true,
    closedVocabulary: true,
    hostedCitesLocalEvidenceRejected: true,
    ipv6RiskAcceptanceWording: true,
    shippedTemplateValidatesCleanly: true,
  },
}));
