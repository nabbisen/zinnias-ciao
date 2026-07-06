#!/usr/bin/env node
// Bootstrap a hosted Cloudflare Worker environment with one community, one seed
// admin, and one plaintext invite code. The invite code is printed once.

import { createHmac, randomBytes } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';

const args = process.argv.slice(2);
const get = (flag) => {
  const i = args.indexOf(flag);
  return i !== -1 ? args[i + 1] ?? null : null;
};
const has = (flag) => args.includes(flag);

const TARGETS = {
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
};

const targetName = get('--target') ?? 'staging';
const target = TARGETS[targetName];
if (!target) {
  console.error(`Unknown --target "${targetName}". Expected "staging" or "production".`);
  process.exit(2);
}

const communityName = get('--community') ?? target.defaultCommunity;
const adminName = get('--admin') ?? 'Admin';
const wranglerConfig = get('--config');
if (!wranglerConfig) {
  console.error(
    `Missing required --config. For ${target.wranglerEnv}, pass --config ${target.configHint}.`,
  );
  process.exit(2);
}
const yes = has('-y') || has('--yes');

const ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
const CODE_LEN = 6;
const INVITE_EXPIRES = '2099-12-31T23:59:59.000Z';

function randomId(kind) {
  return `${kind}_${target.idPrefix}_${randomBytes(8).toString('hex')}`;
}

function generateCode() {
  const limit = Math.floor(256 / ALPHABET.length) * ALPHABET.length;
  let code = '';
  while (code.length < CODE_LEN) {
    const b = randomBytes(1)[0];
    if (b < limit) {
      code += ALPHABET[b % ALPHABET.length];
    }
  }
  return code;
}

function esc(value) {
  return String(value).replace(/'/g, "''");
}

async function confirm(message) {
  if (yes) return true;
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(`${message} [y/N] `, (answer) => {
      rl.close();
      resolve(answer.trim().toLowerCase() === 'y');
    });
  });
}

function run(command, args, options = {}) {
  execFileSync(command, args, { stdio: 'inherit', ...options });
}

function putPepper(pepper) {
  const result = spawnSync(
    'bunx',
    [
      'wrangler',
      'secret',
      'put',
      'HMAC_PEPPER',
      '--env',
      target.wranglerEnv,
      '--config',
      wranglerConfig,
    ],
    {
      input: `${pepper}\n`,
      stdio: ['pipe', 'inherit', 'inherit'],
      encoding: 'utf8',
    },
  );
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const pepper = randomBytes(32).toString('hex');
const inviteCode = generateCode();
const codeHmac = createHmac('sha256', pepper).update(inviteCode).digest('hex');
const now = new Date().toISOString();

const ids = {
  community: randomId('com'),
  user: randomId('usr'),
  membership: randomId('mem'),
  invite: randomId('inv'),
};

console.log('');
console.log(`ciao.zinnias ${target.label} bootstrap`);
console.log('====================================');
console.log(`  Community : ${communityName}`);
console.log(`  Admin     : ${adminName}`);
console.log(`  Target    : Cloudflare [env.${target.wranglerEnv}] remote D1`);
console.log(`  Config    : ${wranglerConfig}`);
console.log('');
console.log(`This rotates ${target.wranglerEnv} HMAC_PEPPER. Existing ${target.wranglerEnv}`);
console.log('sessions, invite codes, and form tokens issued with the previous pepper will');
console.log('no longer validate. The pepper value will be sent to Wrangler and will not be');
console.log('printed.');
console.log('');

if (targetName === 'production') {
  console.log('Production bootstrap is for initial release setup only. Do not run it on an');
  console.log('active production database unless a planned credential rotation is approved.');
  console.log('');
}

const ok = await confirm(
  `Apply remote ${target.wranglerEnv} migrations, rotate ${target.wranglerEnv} pepper, and seed?`,
);
if (!ok) {
  console.log('Aborted.');
  process.exit(0);
}

console.log(`\n[1/4] Applying remote ${target.wranglerEnv} migrations...`);
run('bunx', [
  'wrangler',
  'd1',
  'migrations',
  'apply',
  target.database,
  '--remote',
  '--env',
  target.wranglerEnv,
  '--config',
  wranglerConfig,
]);

console.log(`\n[2/4] Setting ${target.wranglerEnv} HMAC_PEPPER...`);
putPepper(pepper);

console.log(`\n[3/4] Seeding ${target.wranglerEnv} community, admin, and invite code...`);
const statements = [
  `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${ids.community}', '${esc(communityName)}', 'Asia/Tokyo', 1, '${now}')`,
  `INSERT INTO users (id, created_at) VALUES ('${ids.user}', '${now}')`,
  `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${ids.membership}', '${ids.community}', '${ids.user}', 'admin', '${esc(adminName)}', '${now}')`,
  `INSERT INTO invite_codes (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) VALUES ('${ids.invite}', '${ids.community}', '${codeHmac}', '${ids.membership}', '${INVITE_EXPIRES}', 'admin', '${now}')`,
];

for (const statement of statements) {
  run('bunx', [
    'wrangler',
    'd1',
    'execute',
    target.database,
    '--remote',
    '--env',
    target.wranglerEnv,
    '--config',
    wranglerConfig,
    '--command',
    statement,
  ]);
}

console.log(`\n[4/4] Verifying remote ${target.wranglerEnv} form-token table...`);
run('bunx', [
  'wrangler',
  'd1',
  'execute',
  target.database,
  '--remote',
  '--env',
  target.wranglerEnv,
  '--config',
  wranglerConfig,
  '--command',
  "SELECT name FROM sqlite_master WHERE type='table' AND name='form_tokens'",
]);

const pad = (value, width) => String(value).slice(0, width).padEnd(width);

console.log('');
console.log('+-------------------------------------------------+');
console.log(`|  ${pad(`${target.label} bootstrap complete.`, 45)} |`);
console.log('|                                                 |');
console.log(`|  Invite code : ${pad(inviteCode, 31)} |`);
console.log(`|  Community   : ${pad(communityName, 31)} |`);
console.log(`|  Admin seed  : ${pad(adminName, 31)} |`);
console.log('|                                                 |');
console.log('|  Next steps:                                    |');
console.log(`|    open ${pad(`${target.nextUrl}/join`, 35)} |`);
console.log(`|    enter ${pad(inviteCode, 38)} |`);
console.log('+-------------------------------------------------+');
console.log('');
