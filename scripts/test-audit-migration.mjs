// Disposable local-only audit integrity migration rehearsal.
// Never add --remote, hosted configuration, credentials, or production data.

import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const sourceMigrations = join(root, 'migrations');
const wranglerBin = join(root, 'node_modules/.bin/wrangler');
const wranglerPackage = join(root, 'node_modules/wrangler/package.json');
const database = 'zinnias-ciao-audit-migration-local';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function run(command, args, { cwd, env, expectFailure = false } = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('exit', (code) => {
      if ((!expectFailure && code === 0) || (expectFailure && code !== 0)) {
        accept({ stdout, stderr });
        return;
      }
      const diagnostic = `${stdout}\n${stderr}`.trim().slice(-2000);
      reject(new Error(`${command} exited ${code}: ${diagnostic}`));
    });
  });
}

function d1Args(config, persistTo, actionArgs) {
  return [
    'd1', ...actionArgs,
    '--local',
    '--persist-to', persistTo,
    '--config', config,
  ];
}

function queryRows(result) {
  const payload = result.stdout.trim() || result.stderr.trim();
  assert(payload.startsWith('[') || payload.startsWith('{'), 'Wrangler returned no JSON query result');
  const parsed = JSON.parse(payload);
  const queryResult = Array.isArray(parsed) ? parsed[0] : parsed;
  assert(queryResult?.success === true && Array.isArray(queryResult.results), 'unexpected Wrangler JSON result');
  return queryResult.results;
}

function digestCore(rows) {
  return createHash('sha256').update(JSON.stringify(rows)).digest('hex');
}

const disposableRoot = join(root, '.git-exclude/tmp');
await mkdir(disposableRoot, { recursive: true });
const tempRoot = await mkdtemp(join(disposableRoot, 'audit-migration-'));
const migrationsDir = join(tempRoot, 'migrations');
const persistTo = join(tempRoot, 'state');
const xdgConfig = join(tempRoot, 'xdg');
const config = join(tempRoot, 'wrangler.toml');
const seed = join(tempRoot, 'synthetic-legacy.sql');
const wranglerLog = join(tempRoot, 'wrangler.log');
await mkdir(migrationsDir, { recursive: true });
await mkdir(persistTo, { recursive: true });
await mkdir(xdgConfig, { recursive: true });

const cloudflareAuthorityKey = /^(?:CLOUDFLARE_|CF_(?:API_TOKEN|API_KEY|EMAIL|ACCOUNT_ID|ZONE_ID)$|WRANGLER_(?:API_TOKEN|OAUTH_TOKEN)$)/u;

function inheritNonAuthorityEnvironment(source) {
  const allowed = {};
  for (const key of Object.keys(source)) {
    if (cloudflareAuthorityKey.test(key)) continue;
    const value = source[key];
    if (value !== undefined) allowed[key] = value;
  }
  return allowed;
}

const sentinelEnvironment = { AUDIT_MIGRATION_LOCAL_SENTINEL: 'retained' };
Object.defineProperty(sentinelEnvironment, 'CLOUDFLARE_API_TOKEN', {
  enumerable: true,
  get() {
    throw new Error('sentinel authority value was read');
  },
});
const sentinelResult = inheritNonAuthorityEnvironment(sentinelEnvironment);
assert(
  sentinelResult.AUDIT_MIGRATION_LOCAL_SENTINEL === 'retained'
    && !Object.hasOwn(sentinelResult, 'CLOUDFLARE_API_TOKEN'),
  'authority-key filtering regression',
);

const inheritedLocalEnv = inheritNonAuthorityEnvironment(process.env);
const env = {
  ...inheritedLocalEnv,
  XDG_CONFIG_HOME: xdgConfig,
  WRANGLER_LOG_PATH: wranglerLog,
  NO_COLOR: '1',
};

const coreQuery = [
  'SELECT id, community_id, actor_membership_id, target_kind, target_id, action, created_at',
  'FROM audit_log ORDER BY id',
].join(' ');

async function execute(command, { expectFailure = false } = {}) {
  return run(
    wranglerBin,
    d1Args(config, persistTo, [
      'execute', database,
      '--yes',
      '--command', command,
      '--json',
    ]),
    { cwd: tempRoot, env, expectFailure },
  );
}

try {
  const wranglerMetadata = JSON.parse(await readFile(wranglerPackage, 'utf8'));
  const wranglerVersion = String(wranglerMetadata.version ?? '');
  assert(/^4\./u.test(wranglerVersion), `Audit migration rehearsal requires Wrangler 4.x, found ${wranglerVersion || 'unknown'}`);

  await writeFile(config, [
    'name = "zinnias-ciao-audit-migration-local"',
    'main = "worker.mjs"',
    'compatibility_date = "2026-07-15"',
    '',
    '[[d1_databases]]',
    'binding = "DB"',
    `database_name = "${database}"`,
    'database_id = "local"',
    'migrations_dir = "migrations"',
    '',
  ].join('\n'));
  await writeFile(join(tempRoot, 'worker.mjs'), 'export default { fetch() { return new Response("local-only"); } };\n');

  for (let version = 1; version <= 9; version += 1) {
    const prefix = String(version).padStart(4, '0');
    const names = {
      '0001': '0001_initial.sql',
      '0002': '0002_form_tokens_nullable_user.sql',
      '0003': '0003_invite_grants_role.sql',
      '0004': '0004_calendar_tokens.sql',
      '0005': '0005_event_templates.sql',
      '0006': '0006_event_recurrence.sql',
      '0007': '0007_codlet_tables.sql',
      '0008': '0008_membership_relink_codes.sql',
      '0009': '0009_recurrence_v2.sql',
    };
    await copyFile(join(sourceMigrations, names[prefix]), join(migrationsDir, names[prefix]));
  }

  await run(
    wranglerBin,
    d1Args(config, persistTo, ['migrations', 'apply', database]),
    { cwd: tempRoot, env },
  );

  await writeFile(seed, `
INSERT INTO audit_log
  (id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
VALUES
  ('aud_legacy_0001', 'com_alpha', 'mem_admin', 'event_day', 'day_alpha', 'occurrence_cancelled', '{"Password":"synthetic","nested":{"token":"synthetic"}}', '2026-01-01T00:00:00Z'),
  ('aud_legacy_0002', 'com_alpha', 'mem_admin', 'calendar_feed', NULL, 'calendar_token_generated', '{"safe":"synthetic"}', '2026-01-02T00:00:00Z'),
  ('aud_legacy_0003', 'com_alpha', 'mem_admin', 'calendar_feed', NULL, 'calendar_token_revoked', NULL, '2026-01-03T00:00:00Z'),
  ('aud_legacy_0004', 'com_alpha', 'mem_admin', 'community', 'com_alpha', 'exported', '{}', '2026-01-04T00:00:00Z'),
  ('aud_legacy_0005', NULL, NULL, 'custom', NULL, 'mystery', '{"unknown":"synthetic"}', '2026-01-05T00:00:00Z');
`);
  await run(
    wranglerBin,
    d1Args(config, persistTo, ['execute', database, '--yes', '--file', seed]),
    { cwd: tempRoot, env },
  );

  const before = queryRows(await execute(coreQuery));
  const beforeDigest = digestCore(before);
  assert(before.length === 5, 'synthetic legacy row count changed');

  await copyFile(
    join(sourceMigrations, '0010_audit_integrity.sql'),
    join(migrationsDir, '0010_audit_integrity.sql'),
  );
  await run(
    wranglerBin,
    d1Args(config, persistTo, ['migrations', 'apply', database]),
    { cwd: tempRoot, env },
  );

  const after = queryRows(await execute(coreQuery));
  assert(after.length === before.length, 'migration changed the core row count');
  assert(digestCore(after) === beforeDigest, 'migration changed a core audit column');

  const reset = queryRows(await execute([
    'SELECT COUNT(*) AS total,',
    "SUM(request_id = 'legacy') AS legacy_requests,",
    "SUM(metadata_json = '{}') AS empty_metadata",
    'FROM audit_log',
  ].join(' ')))[0];
  assert(reset.total === 5 && reset.legacy_requests === 5 && reset.empty_metadata === 5, 'legacy reset invariant failed');

  const schema = queryRows(await execute([
    'SELECT',
    "SUM(name = 'audit_log') AS audit_log,",
    "SUM(name = 'audit_change_assertions') AS assertion_table,",
    "SUM(name = 'audit_log_legacy_0010') AS legacy_table,",
    "SUM(name = 'audit_migration_0010_guard') AS guard_table,",
    "SUM(name = 'idx_audit_log_community_created_at') AS community_index,",
    "SUM(name = 'idx_audit_log_action_created_at') AS action_index",
    'FROM sqlite_master',
  ].join(' ')))[0];
  assert(
    schema.audit_log === 1
      && schema.assertion_table === 1
      && schema.legacy_table === 0
      && schema.guard_table === 0
      && schema.community_index === 1
      && schema.action_index === 1,
    'post-migration schema boundary failed',
  );

  const compatibility = queryRows(await execute(`
    SELECT
      SUM(logical_action = 'event.occurrence_cancelled') AS occurrence_alias,
      SUM(logical_action = 'calendar_feed.token_generated') AS calendar_generated_alias,
      SUM(logical_action = 'calendar_feed.token_revoked') AS calendar_revoked_alias,
      SUM(logical_action = 'community.exported') AS community_export_distinct,
      SUM(logical_action = 'custom.mystery') AS unknown_fallback
    FROM (
      SELECT CASE
        WHEN target_kind = 'event_day' AND action = 'occurrence_cancelled'
          THEN 'event.occurrence_cancelled'
        WHEN target_kind = 'calendar_feed' AND action = 'calendar_token_generated'
          THEN 'calendar_feed.token_generated'
        WHEN target_kind = 'calendar_feed' AND action = 'calendar_token_revoked'
          THEN 'calendar_feed.token_revoked'
        WHEN instr(action, '.') > 0 THEN action
        ELSE target_kind || '.' || action
      END AS logical_action
      FROM audit_log
    )
  `))[0];
  assert(Object.values(compatibility).every((count) => count === 1), 'logical-action compatibility mapping failed');

  await execute(`
    INSERT INTO audit_log
      (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
    VALUES
      ('aud_valid_0010', 'req_0010', 'com_alpha', 'mem_admin', 'attendance', 'evt_alpha',
       'attendance.admin_override', '{"changed_count":1}', '2026-02-01T00:00:00Z')
  `);
  await execute(`
    INSERT INTO audit_change_assertions (operation_id, changed_count)
    VALUES ('ast_abcdefghijklmnopqrstuv', 1)
  `);
  await execute("DELETE FROM audit_change_assertions WHERE operation_id = 'ast_abcdefghijklmnopqrstuv'");

  const rejectedStatements = [
    `INSERT INTO audit_log (id, request_id, target_kind, action, metadata_json, created_at)
     VALUES ('aud_bad_req1', '', 'event', 'event.created', '{}', '2026-02-01T00:00:00Z')`,
    `INSERT INTO audit_log (id, request_id, target_kind, action, metadata_json, created_at)
     VALUES ('aud_bad_meta', 'req_bad', 'event', 'event.created', '[]', '2026-02-01T00:00:00Z')`,
    `INSERT INTO audit_log (id, request_id, target_kind, action, metadata_json, created_at)
     VALUES ('aud_bad_size', 'req_bad', 'event', 'event.created', json_object('safe', printf('%02049d', 0)), '2026-02-01T00:00:00Z')`,
    "INSERT INTO audit_change_assertions (operation_id, changed_count) VALUES ('ast_abcdefghijklmnopqrstuv', 0)",
    "INSERT INTO audit_change_assertions (operation_id, changed_count) VALUES ('invalid', 1)",
  ];
  for (const statement of rejectedStatements) {
    await execute(statement, { expectFailure: true });
  }

  const ledger = queryRows(await execute("SELECT COUNT(*) AS count FROM d1_migrations WHERE name = '0010_audit_integrity.sql'"))[0];
  assert(ledger.count === 1, 'migration 0010 is not recorded exactly once');

  process.stdout.write(`${JSON.stringify({
    wrangler: wranglerVersion,
    mode: 'local-only',
    migration: '0010_audit_integrity.sql',
    legacyRows: before.length,
    coreColumnsPreserved: true,
    legacyRequestIdsAssigned: reset.legacy_requests,
    legacyMetadataReset: reset.empty_metadata,
    indexesCreated: schema.community_index + schema.action_index,
    assertionTableCreated: schema.assertion_table === 1,
    compatibilityMappingsVerified: Object.values(compatibility).length,
    constraintRejections: rejectedStatements.length,
    privacy: { legacyMetadataSelected: false, legacyMetadataPrinted: false },
    cleanup: { legacyTable: schema.legacy_table, migrationGuardTable: schema.guard_table },
  }, null, 2)}\n`);
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}
