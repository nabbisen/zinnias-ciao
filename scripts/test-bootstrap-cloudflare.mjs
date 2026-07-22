#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import {
  EXPECTED_APPLICATION_TABLES,
  runBootstrap,
} from './lib/bootstrap-cloudflare-core.mjs';

function jsonRows(rows, mode = 'single') {
  if (mode === 'multiple') return JSON.stringify([{ results: rows }, { results: [{ contradictory: true }] }]);
  if (mode === 'object') return JSON.stringify({ results: rows });
  if (mode === 'duplicate-key') return `[{"results":${JSON.stringify(rows)},"results":[]}]`;
  if (mode === 'escaped-duplicate-second') return `[{"results":${JSON.stringify(rows)},"\\u0072esults":[]}]`;
  if (mode === 'escaped-duplicate-first') return `[{"\\u0072esults":[],"results":${JSON.stringify(rows)}}]`;
  return JSON.stringify([{ results: rows }]);
}

function fakeAdapter({ nonzeroTable = null, tables = EXPECTED_APPLICATION_TABLES, malformed = false, envelope = 'single', rowValue, queryError = false, prompts = [] } = {}) {
  const calls = [];
  const logs = [];
  const answers = [...prompts];
  let randomCalls = 0;
  const adapter = {
    calls,
    logs,
    get randomCalls() {
      return randomCalls;
    },
    log(message = '') {
      logs.push(String(message));
    },
    now() {
      return '2026-07-21T00:00:00.000Z';
    },
    async prompt() {
      return answers.shift() ?? '';
    },
    randomBytes(size) {
      randomCalls += 1;
      return Buffer.alloc(size, randomCalls);
    },
    async runWrangler(args, options = {}) {
      calls.push({ args: [...args], input: options.input });
      const common = ['zinnias-ciao-staging', '--remote', '--env', 'staging', '--config', 'ignored-staging.toml'];
      if (args[0] === 'secret') {
        assert.deepEqual(args, ['secret', 'put', 'HMAC_PEPPER', '--env', 'staging', '--config', 'ignored-staging.toml']);
        assert.equal(typeof options.input, 'string');
        return { stdout: '' };
      }
      if (args[0] === 'd1' && args[1] === 'migrations') {
        assert.deepEqual(args, ['d1', 'migrations', 'apply', ...common]);
        assert.equal(options.input, undefined);
        return { stdout: '' };
      }
      assert.equal(args[0], 'd1', 'fake adapter rejected a non-bootstrap command');
      assert.equal(args[1], 'execute', 'fake adapter rejected an unexpected D1 operation');
      assert.deepEqual(args.slice(2, 8), common);
      assert.equal(options.input, undefined);
      if (queryError) throw new Error('injected_query_error');
      const statement = args[args.indexOf('--command') + 1] ?? '';
      assert.equal(args.filter((value) => value === '--command').length, 1);
      if (statement.includes('sqlite_master')) {
        assert.deepEqual(args.slice(8), [
          '--json',
          '--command',
          "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        ]);
        if (malformed) return { stdout: '{' };
        return {
          stdout: jsonRows([...tables, 'd1_migrations'].sort().map((name) => ({ name })), envelope),
        };
      }
      const table = /FROM "([^"]+)"/u.exec(statement)?.[1];
      if (table) {
        assert.ok(EXPECTED_APPLICATION_TABLES.includes(table), `unexpected freshness table: ${table}`);
        assert.deepEqual(args.slice(8), [
          '--json',
          '--command',
          `SELECT EXISTS(SELECT 1 FROM "${table}" LIMIT 1) AS has_rows`,
        ]);
        return {
          stdout: jsonRows([{ has_rows: rowValue ?? (table === nonzeroTable ? 1 : 0) }], envelope),
        };
      }
      const insert = /^INSERT INTO (communities|users|community_memberships|invite_codes) /u.exec(statement);
      assert.ok(insert, `fake adapter rejected unexpected SQL: ${statement.slice(0, 80)}`);
      assert.deepEqual(args.slice(8, 10), ['--command', statement]);
      return { stdout: '' };
    },
  };
  return adapter;
}

const baseArgs = [
  '--target',
  'staging',
  '--config',
  'ignored-staging.toml',
  '--community',
  'Synthetic Community',
  '--admin',
  'Synthetic Admin',
];

async function expectStopped(adapter, args, pattern) {
  await assert.rejects(runBootstrap({ argv: args, adapter }), pattern);
  assert.equal(adapter.randomCalls, 0, 'stopped path generated secret material');
  assert.equal(adapter.calls.some((call) => call.args[0] === 'secret'), false);
}

await expectStopped(fakeAdapter(), baseArgs.slice(2), /Missing --target/u);

const fresh = fakeAdapter();
const result = await runBootstrap({ argv: [...baseArgs, '--yes'], adapter: fresh });
assert.equal(result.state, 'provisioned-not-ready');
const secretCall = fresh.calls.find((call) => call.args[0] === 'secret');
assert.ok(secretCall && typeof secretCall.input === 'string');
const pepper = secretCall.input.trim();
const inviteInsert = fresh.calls.find((call) =>
  String(call.args[call.args.indexOf('--command') + 1] ?? '').includes('INSERT INTO invite_codes'),
);
assert.ok(inviteInsert, 'fresh flow did not seed an invite');
const code = fresh.logs.find((line) => line.startsWith('Invite code: '))?.slice(13) ?? '';
const expectedHmac = createHmac('sha256', pepper).update(code).digest('hex');
assert.ok(inviteInsert.args.join(' ').includes(expectedHmac), 'seed HMAC did not use secret stdin');
assert.equal(fresh.logs.join('\n').includes(pepper), false, 'bootstrap printed the pepper');
assert.equal(
  fresh.calls.some((call) => call.args.includes('deploy') || call.args.includes('route')),
  false,
);

for (const table of EXPECTED_APPLICATION_TABLES) {
  const adapter = fakeAdapter({ nonzeroTable: table });
  await expectStopped(adapter, [...baseArgs, '--yes'], /--rotate-hmac-pepper/u);
}

await expectStopped(
  fakeAdapter({ tables: EXPECTED_APPLICATION_TABLES.slice(1) }),
  [...baseArgs, '--yes'],
  /unexpected or missing/u,
);
await expectStopped(
  fakeAdapter({ tables: [...EXPECTED_APPLICATION_TABLES, 'unknown_table'] }),
  [...baseArgs, '--yes'],
  /unexpected or missing/u,
);
await expectStopped(fakeAdapter({ malformed: true }), [...baseArgs, '--yes'], /malformed JSON/u);
await expectStopped(fakeAdapter({ envelope: 'multiple' }), [...baseArgs, '--yes'], /ambiguous/u);
await expectStopped(fakeAdapter({ envelope: 'object' }), [...baseArgs, '--yes'], /ambiguous/u);
await expectStopped(fakeAdapter({ envelope: 'duplicate-key' }), [...baseArgs, '--yes'], /duplicate or ambiguous/u);
await expectStopped(fakeAdapter({ envelope: 'escaped-duplicate-second' }), [...baseArgs, '--yes'], /duplicate or ambiguous/u);
await expectStopped(fakeAdapter({ envelope: 'escaped-duplicate-first' }), [...baseArgs, '--yes'], /duplicate or ambiguous/u);
await expectStopped(fakeAdapter({ rowValue: '0' }), [...baseArgs, '--yes'], /invalid row result/u);
await expectStopped(fakeAdapter({ queryError: true }), [...baseArgs, '--yes'], /injected_query_error/u);

const nonFresh = { nonzeroTable: EXPECTED_APPLICATION_TABLES[0] };
await expectStopped(
  fakeAdapter(nonFresh),
  [...baseArgs, '--yes', '--rotate-hmac-pepper'],
  /--confirm-rotation/u,
);
await expectStopped(
  fakeAdapter(nonFresh),
  [
    ...baseArgs,
    '--yes',
    '--rotate-hmac-pepper',
    '--confirm-rotation',
    'ROTATE staging',
    '--confirm-rotation',
    'ROTATE staging',
  ],
  /Repeated --confirm-rotation/u,
);
await expectStopped(
  fakeAdapter(nonFresh),
  [...baseArgs, '--yes', '--rotate-hmac-pepper', '--confirm-rotation', 'ROTATE production'],
  /--confirm-rotation/u,
);
await expectStopped(
  fakeAdapter({ ...nonFresh, prompts: ['wrong'] }),
  [...baseArgs, '--rotate-hmac-pepper'],
  /did not match/u,
);

const interactiveRotation = fakeAdapter({ ...nonFresh, prompts: ['ROTATE staging'] });
assert.equal(
  (await runBootstrap({ argv: [...baseArgs, '--rotate-hmac-pepper'], adapter: interactiveRotation })).state,
  'provisioned-not-ready',
);
const nonInteractiveRotation = fakeAdapter(nonFresh);
assert.equal(
  (
    await runBootstrap({
      argv: [
        ...baseArgs,
        '--yes',
        '--rotate-hmac-pepper',
        '--confirm-rotation',
        'ROTATE staging',
      ],
      adapter: nonInteractiveRotation,
    })
  ).state,
  'provisioned-not-ready',
);
for (const phrase of [
  'sessions',
  'invites',
  'relink/help-signin codes',
  'form tokens',
  'calendar tokens',
  'recovery codes',
]) {
  assert.ok(nonInteractiveRotation.logs.join('\n').includes(phrase));
}

console.log(
  JSON.stringify({
    ok: true,
    phases: {
      missingTargetStops: true,
      strictSingleResultEnvelope: true,
      duplicateResultKeysStop: true,
      exactCommandAdapter: true,
      fresh: true,
      everyKnownNonzeroStopsWithoutRotation: true,
      unknownAndErrorStop: true,
      interactiveTargetConfirmation: true,
      nonInteractiveTargetConfirmation: true,
      noSecretOutput: true,
      sameSecretForSeed: true,
      provisionedNotReady: true,
    },
  }),
);
