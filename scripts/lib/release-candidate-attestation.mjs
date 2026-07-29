// RFC-050 local evidence tooling (Tooling Slice 8): release-candidate
// attestation gate rules. Parses the two pipe-tables in a filled-in copy of
// `docs/src/tester/release-candidates/TEMPLATE.md` (the "Gate verdicts"
// table and the "Carried risk acceptances" table) and mechanically enforces
// the gate rules the RFC-050 disposition checkpoint required be enforced
// rather than left to prose:
//
//   1. E4a -> E4 dependency: E4's Hosted Evidence verdict may never be
//      `Pass` unless E4a's Hosted Evidence verdict is also a current `Pass`
//      for the same candidate (RFC-050 E4a: "E4's capacity results are void
//      unless E4a passed against the same candidate").
//   2. Closed verdict vocabulary in every Local/Hosted cell, so a typo or a
//      prose sentence can't silently stand in for a verdict.
//   3. The RFC-078 criterion 6 IPv6 `/64` risk acceptance must read exactly
//      `Risk-Accepted-Open` and can never be recorded as `Pass` or any other
//      word — it is carried forward as accepted risk, not hosted-proven.
//
// Pure text parsing. No hosted call, no filesystem mutation.

export const VERDICT_WORDS = ['Pass', 'Fail', 'Pending', 'Void', 'Risk-Accepted-Open', 'N/A'];

const GATE_TABLE_HEADER = /^\|\s*Gate\s*\|/i;
const RISK_TABLE_HEADER = /^\|\s*Risk\s*\|/i;
const TABLE_SEPARATOR_ROW = /^\|[\s-]*\|[\s-:|]*$/;

export class AttestationParseError extends Error {}

// RFC-050 Tooling Slice 8 review (2026-07-29), recommendation R2: reject
// Hosted Evidence cells that cite a known-local marker instead of a hosted
// observation. Deliberately a *negative* check for recognizable local
// command/path shapes, not a positive shape requirement — hosted evidence
// legitimately takes forms (a dashboard observation, a version-id
// confirmation) that don't reduce to any one path pattern.
const LOCAL_CITATION_MARKERS = [
  /\bbun run test\b/i,
  /\bcargo (?:test|clippy|fmt|check)\b/i,
  /\bgit diff --check\b/i,
  /(?:^|[\s(`'"])docs\//i,
  /(?:^|[\s(`'"])scripts\//i,
];

function splitRow(line) {
  const trimmed = line.trim();
  const withoutEdges = trimmed.replace(/^\|/, '').replace(/\|$/, '');
  return withoutEdges.split('|').map((cell) => cell.trim());
}

function readTable(lines, headerTest) {
  const headerIndex = lines.findIndex((line) => headerTest(line.trim()));
  if (headerIndex === -1) return null;
  const rows = [];
  for (let i = headerIndex + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (!line.trim().startsWith('|')) break;
    if (TABLE_SEPARATOR_ROW.test(line.trim())) continue;
    rows.push(splitRow(line));
  }
  return rows;
}

function extractGateId(cell) {
  const match = /^(E\d+a?)\b/.exec(cell);
  return match ? match[1] : null;
}

function extractVerdict(cell) {
  if (!cell) return { verdict: null, raw: cell };
  for (const word of VERDICT_WORDS) {
    if (cell === word || cell.startsWith(`${word} `) || cell.startsWith(`${word}—`) || cell.startsWith(`${word}–`)) {
      return { verdict: word, raw: cell };
    }
  }
  return { verdict: 'invalid', raw: cell };
}

export function parseGateTable(markdown) {
  const lines = markdown.split('\n');
  const rows = readTable(lines, (line) => GATE_TABLE_HEADER.test(line));
  if (!rows) {
    throw new AttestationParseError('No "| Gate | ..." table found in attestation document.');
  }
  const gates = new Map();
  for (const row of rows) {
    const [gateCell, localCell, hostedCell] = row;
    const gateId = extractGateId(gateCell ?? '');
    if (!gateId) continue;
    gates.set(gateId, {
      local: extractVerdict(localCell ?? ''),
      hosted: extractVerdict(hostedCell ?? ''),
    });
  }
  return gates;
}

export function parseRiskAcceptanceTable(markdown) {
  const lines = markdown.split('\n');
  const rows = readTable(lines, (line) => RISK_TABLE_HEADER.test(line));
  if (!rows) {
    throw new AttestationParseError('No "| Risk | ..." table found in attestation document.');
  }
  return rows
    .map(([riskCell, statusCell]) => ({ risk: riskCell ?? '', status: (statusCell ?? '').trim() }))
    .filter((entry) => entry.risk.length > 0);
}

/**
 * Returns an array of violation objects (empty if the attestation passes all
 * mechanically-enforced gate rules). Never throws on a filled-in template —
 * only `AttestationParseError` on a document missing the required tables at
 * all, which indicates the wrong file was passed in.
 */
export function checkAttestationGates(markdown) {
  const violations = [];
  const gates = parseGateTable(markdown);

  for (const [gateId, { local, hosted }] of gates) {
    if (local.verdict === 'invalid') {
      violations.push({
        rule: 'closed_vocabulary',
        gate: gateId,
        message: `Gate ${gateId} Local Evidence cell does not start with a valid verdict word (${VERDICT_WORDS.join(', ')}): "${local.raw}"`,
      });
    }
    if (hosted.verdict === 'invalid') {
      violations.push({
        rule: 'closed_vocabulary',
        gate: gateId,
        message: `Gate ${gateId} Hosted Evidence cell does not start with a valid verdict word (${VERDICT_WORDS.join(', ')}): "${hosted.raw}"`,
      });
    }
    if (hosted.verdict && hosted.verdict !== 'invalid' && hosted.verdict !== 'N/A') {
      const marker = LOCAL_CITATION_MARKERS.find((pattern) => pattern.test(hosted.raw));
      if (marker) {
        violations.push({
          rule: 'hosted_cites_local_evidence',
          gate: gateId,
          message: `Gate ${gateId} Hosted Evidence cell cites a local-only marker (matched ${marker}), which cannot discharge a hosted-evidence requirement: "${hosted.raw}"`,
        });
      }
    }
  }

  const e4 = gates.get('E4');
  const e4a = gates.get('E4a');
  if (e4 && e4.hosted.verdict === 'Pass') {
    if (!e4a || e4a.hosted.verdict !== 'Pass') {
      violations.push({
        rule: 'e4a_gates_e4',
        gate: 'E4',
        message:
          'Gate E4 Hosted Evidence is recorded as Pass, but E4 is void unless E4a Hosted Evidence is also a current Pass for the same candidate (RFC-050 E4a dependency).',
      });
    }
  }

  const riskAcceptances = parseRiskAcceptanceTable(markdown);
  const ipv6Row = riskAcceptances.find((entry) => /ipv6/i.test(entry.risk));
  if (!ipv6Row) {
    violations.push({
      rule: 'ipv6_risk_acceptance_missing',
      gate: 'E4a',
      message: 'No RFC-078 criterion 6 IPv6 /64 risk-acceptance row found in the Carried risk acceptances table.',
    });
  } else if (ipv6Row.status !== 'Risk-Accepted-Open') {
    violations.push({
      rule: 'ipv6_risk_acceptance_status',
      gate: 'E4a',
      message: `RFC-078 criterion 6 IPv6 /64 risk acceptance must read exactly "Risk-Accepted-Open", found "${ipv6Row.status}". IPv6 client support is not confirmed/implemented for this deployment and can never be recorded as hosted-proven.`,
    });
  }

  return violations;
}

export function assertAttestationGatesValid(markdown) {
  const violations = checkAttestationGates(markdown);
  if (violations.length > 0) {
    const error = new Error(
      `Attestation gate rules failed (${violations.length}):\n${violations.map((v) => `- [${v.rule}] ${v.message}`).join('\n')}`,
    );
    error.violations = violations;
    throw error;
  }
}
