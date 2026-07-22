import { createHmac } from 'node:crypto';

export const EXPECTED_APPLICATION_TABLES = Object.freeze([
  'attendances',
  'audit_change_assertions',
  'audit_log',
  'calendar_tokens',
  'codlet_codes',
  'codlet_form_tokens',
  'codlet_sessions',
  'communities',
  'community_memberships',
  'event_days',
  'event_notes',
  'event_series',
  'event_series_exceptions',
  'event_templates',
  'events',
  'form_tokens',
  'invite_codes',
  'membership_relink_codes',
  'sessions',
  'users',
]);

const BOOKKEEPING_TABLES = new Set(['d1_migrations']);
const TARGETS = Object.freeze({
  staging: {
    wranglerEnv: 'staging',
    database: 'zinnias-ciao-staging',
    idPrefix: 'stg',
    defaultCommunity: 'Staging Community',
    label: 'hosted staging',
    nextUrl: '<staging-url>',
    configHint: 'wrangler.staging.local.toml',
  },
  production: {
    wranglerEnv: 'production',
    database: 'zinnias-ciao',
    idPrefix: 'prd',
    defaultCommunity: 'Production Community',
    label: 'production',
    nextUrl: '<production-url>',
    configHint: 'wrangler.production.local.toml',
  },
});

const ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
const CODE_LEN = 6;
const INVITE_EXPIRES = '2099-12-31T23:59:59.000Z';
const ROTATION_WARNING =
  'sessions, invites, relink/help-signin codes, form tokens, calendar tokens, and recovery codes';

function get(args, flag) {
  const indexes = args.flatMap((value, index) => (value === flag ? [index] : []));
  if (indexes.length > 1) throw new Error(`Repeated ${flag} is not allowed.`);
  if (indexes.length === 0) return null;
  return args[indexes[0] + 1] ?? null;
}

function has(args, flag) {
  return args.includes(flag);
}

function sqlString(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function randomId(adapter, kind, target) {
  return `${kind}_${target.idPrefix}_${adapter.randomBytes(8).toString('hex')}`;
}

function generateCode(adapter) {
  const limit = Math.floor(256 / ALPHABET.length) * ALPHABET.length;
  let code = '';
  while (code.length < CODE_LEN) {
    const byte = adapter.randomBytes(1)[0];
    if (byte < limit) code += ALPHABET[byte % ALPHABET.length];
  }
  return code;
}

function commonD1Args(target, config) {
  return [target.database, '--remote', '--env', target.wranglerEnv, '--config', config];
}

class DuplicateJsonKeyError extends Error {}

function rejectDuplicateJsonKeys(source) {
  let index = 0;

  function malformed() {
    throw new SyntaxError('malformed JSON');
  }

  function whitespace() {
    while (/[\t\n\r ]/u.test(source[index] ?? '')) index += 1;
  }

  function string() {
    if (source[index] !== '"') malformed();
    const start = index;
    index += 1;
    while (index < source.length) {
      const character = source[index];
      if (character === '"') {
        index += 1;
        return JSON.parse(source.slice(start, index));
      }
      if (character === '\\') {
        index += 1;
        const escaped = source[index];
        if (escaped === 'u') {
          if (!/^[0-9a-fA-F]{4}$/u.test(source.slice(index + 1, index + 5))) malformed();
          index += 5;
          continue;
        }
        if (!'"\\/bfnrt'.includes(escaped ?? '')) malformed();
        index += 1;
        continue;
      }
      if (character.charCodeAt(0) <= 0x1f) malformed();
      index += 1;
    }
    malformed();
  }

  function value() {
    whitespace();
    const character = source[index];
    if (character === '{') return object();
    if (character === '[') return array();
    if (character === '"') {
      string();
      return;
    }
    for (const literal of ['true', 'false', 'null']) {
      if (source.startsWith(literal, index)) {
        index += literal.length;
        return;
      }
    }
    const number = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/u.exec(source.slice(index));
    if (!number) malformed();
    index += number[0].length;
  }

  function array() {
    index += 1;
    whitespace();
    if (source[index] === ']') {
      index += 1;
      return;
    }
    while (true) {
      value();
      whitespace();
      if (source[index] === ']') {
        index += 1;
        return;
      }
      if (source[index] !== ',') malformed();
      index += 1;
    }
  }

  function object() {
    index += 1;
    whitespace();
    const keys = new Set();
    if (source[index] === '}') {
      index += 1;
      return;
    }
    while (true) {
      whitespace();
      const key = string();
      if (keys.has(key)) throw new DuplicateJsonKeyError(`duplicate JSON key: ${key}`);
      keys.add(key);
      whitespace();
      if (source[index] !== ':') malformed();
      index += 1;
      value();
      whitespace();
      if (source[index] === '}') {
        index += 1;
        return;
      }
      if (source[index] !== ',') malformed();
      index += 1;
    }
  }

  value();
  whitespace();
  if (index !== source.length) malformed();
}

function parseWranglerRows(stdout) {
  let parsed;
  try {
    rejectDuplicateJsonKeys(stdout);
    parsed = JSON.parse(stdout);
  } catch (error) {
    if (error instanceof DuplicateJsonKeyError) {
      throw new Error('Freshness probe returned duplicate or ambiguous result sets; refusing to continue.');
    }
    throw new Error('Freshness probe returned malformed JSON; refusing to continue.');
  }
  if (!Array.isArray(parsed) || parsed.length !== 1) {
    throw new Error('Freshness probe returned an ambiguous result; refusing to continue.');
  }
  const envelope = parsed[0];
  if (!envelope || typeof envelope !== 'object' || Array.isArray(envelope)) {
    throw new Error('Freshness probe returned an ambiguous result; refusing to continue.');
  }
  const rows = envelope.results;
  if (!Array.isArray(rows)) {
    throw new Error('Freshness probe returned an ambiguous result; refusing to continue.');
  }
  return rows;
}

async function query(adapter, target, config, statement) {
  const result = await adapter.runWrangler([
    'd1',
    'execute',
    ...commonD1Args(target, config),
    '--json',
    '--command',
    statement,
  ]);
  return parseWranglerRows(result.stdout);
}

export async function inspectFreshness(adapter, target, config) {
  const tableRows = await query(
    adapter,
    target,
    config,
    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
  );
  const discovered = tableRows.map((row) => row?.name);
  if (discovered.some((name) => typeof name !== 'string')) {
    throw new Error('Freshness probe returned an invalid table name; refusing to continue.');
  }
  const application = discovered.filter((name) => !BOOKKEEPING_TABLES.has(name)).sort();
  const expected = [...EXPECTED_APPLICATION_TABLES].sort();
  if (JSON.stringify(application) !== JSON.stringify(expected)) {
    throw new Error('Freshness probe found an unexpected or missing application table; refusing to continue.');
  }

  let nonFresh = false;
  for (const table of EXPECTED_APPLICATION_TABLES) {
    const rows = await query(adapter, target, config, `SELECT EXISTS(SELECT 1 FROM "${table}" LIMIT 1) AS has_rows`);
    const hasRows = rows[0]?.has_rows;
    if (rows.length !== 1 || ![0, 1].includes(hasRows)) {
      throw new Error('Freshness probe returned an invalid row result; refusing to continue.');
    }
    if (hasRows === 1) nonFresh = true;
  }
  return { nonFresh };
}

async function authorize(adapter, { targetName, target, nonFresh, rotate, yes, confirmation }) {
  if (!nonFresh) {
    if (rotate) throw new Error('Rotation flag is not valid for a proven-fresh target.');
    if (confirmation !== null) throw new Error('Rotation confirmation is not valid for fresh provisioning.');
    if (yes) return;
    const answer = await adapter.prompt(
      `Provision fresh ${target.wranglerEnv} pepper and seed while the Worker remains dark? [y/N] `,
    );
    if (answer.toLowerCase() !== 'y') throw new Error('Aborted.');
    return;
  }

  if (!rotate) {
    throw new Error('Target is non-fresh; --rotate-hmac-pepper is required for destructive rotation.');
  }
  adapter.log(`WARNING: rotation invalidates ${ROTATION_WARNING}.`);
  const expected = `ROTATE ${targetName}`;
  if (yes) {
    if (confirmation !== expected) {
      throw new Error(`Non-interactive rotation requires --confirm-rotation ${JSON.stringify(expected)}.`);
    }
    return;
  }
  if (confirmation !== null) {
    throw new Error('--confirm-rotation is only valid together with --yes.');
  }
  const answer = await adapter.prompt(`Type exactly "${expected}" to continue: `);
  if (answer !== expected) throw new Error('Target-bound rotation confirmation did not match.');
}

export async function runBootstrap({ argv, adapter }) {
  const targetName = get(argv, '--target');
  if (targetName === null) throw new Error('Missing --target; choose staging or production explicitly.');
  const target = TARGETS[targetName];
  if (!target) throw new Error(`Unknown --target ${JSON.stringify(targetName)}.`);
  const config = get(argv, '--config');
  if (!config) throw new Error(`Missing --config; use ${target.configHint}.`);
  const communityName = get(argv, '--community') ?? target.defaultCommunity;
  const adminName = get(argv, '--admin') ?? 'Admin';
  const yes = has(argv, '-y') || has(argv, '--yes');
  const rotate = has(argv, '--rotate-hmac-pepper');
  const confirmation = get(argv, '--confirm-rotation');

  adapter.log(`ciao.zinnias ${target.label} bootstrap`);
  adapter.log(`Target: ${target.wranglerEnv}`);
  adapter.log('Wrangler secret put can immediately publish a Worker version.');
  adapter.log('Keep this Worker dark: no custom route, public traffic, or user data.');

  await adapter.runWrangler([
    'd1',
    'migrations',
    'apply',
    ...commonD1Args(target, config),
  ]);
  const freshness = await inspectFreshness(adapter, target, config);
  await authorize(adapter, {
    targetName,
    target,
    nonFresh: freshness.nonFresh,
    rotate,
    yes,
    confirmation,
  });

  const pepper = adapter.randomBytes(32).toString('hex');
  const inviteCode = generateCode(adapter);
  const codeHmac = createHmac('sha256', pepper).update(inviteCode).digest('hex');
  const now = adapter.now();
  const ids = {
    community: randomId(adapter, 'com', target),
    user: randomId(adapter, 'usr', target),
    membership: randomId(adapter, 'mem', target),
    invite: randomId(adapter, 'inv', target),
  };

  await adapter.runWrangler(
    ['secret', 'put', 'HMAC_PEPPER', '--env', target.wranglerEnv, '--config', config],
    { input: `${pepper}\n` },
  );

  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES (${sqlString(ids.community)}, ${sqlString(communityName)}, 'Asia/Tokyo', 1, ${sqlString(now)})`,
    `INSERT INTO users (id, created_at) VALUES (${sqlString(ids.user)}, ${sqlString(now)})`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES (${sqlString(ids.membership)}, ${sqlString(ids.community)}, ${sqlString(ids.user)}, 'admin', ${sqlString(adminName)}, ${sqlString(now)})`,
    `INSERT INTO invite_codes (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) VALUES (${sqlString(ids.invite)}, ${sqlString(ids.community)}, ${sqlString(codeHmac)}, ${sqlString(ids.membership)}, '${INVITE_EXPIRES}', 'admin', ${sqlString(now)})`,
  ];
  for (const statement of statements) {
    await adapter.runWrangler([
      'd1',
      'execute',
      ...commonD1Args(target, config),
      '--command',
      statement,
    ]);
  }

  adapter.log('Provisioned, but not ready for traffic.');
  adapter.log('Deploy the exact candidate immediately, verify candidate identity, then require ready /healthz.');
  adapter.log(`Invite code: ${inviteCode}`);
  adapter.log(`Next after readiness: ${target.nextUrl}/join`);
  return { state: 'provisioned-not-ready' };
}
