#!/usr/bin/env node
// ciao.zinnias dev setup script
//
// Usage:
//   node scripts/setup.mjs                                    (interactive)
//   node scripts/setup.mjs -y                                 (non-interactive)
//   node scripts/setup.mjs --community "Zinnia Club" --admin "Aya" -y
//   node scripts/setup.mjs --reset -y                         (wipe local DB first)
//
// What it does:
//   1. (--reset only) Deletes the local wrangler D1 file so migrations apply fresh.
//   2. Runs `wrangler d1 migrations apply` for all pending migrations.
//   3. Seeds: one community, one admin user+membership, one randomly generated
//      invite code stored as HMAC-SHA256(dev-pepper, code) — plaintext never in DB.
//   4. Prints the invite code to use at /join.

import { createHmac, randomBytes } from 'node:crypto';
import { execSync }                from 'node:child_process';
import { createInterface }         from 'node:readline';
import { rmSync, existsSync }      from 'node:fs';
import { resolve }                 from 'node:path';

// ── Argument parsing ──────────────────────────────────────────────────────
const args = process.argv.slice(2);
const get  = (flag) => { const i = args.indexOf(flag); return i !== -1 ? args[i + 1] ?? null : null; };
const has  = (flag) => args.includes(flag);

const communityName = get('--community') ?? 'My Community';
const adminName     = get('--admin')     ?? 'Admin';
const yes           = has('-y') || has('--yes');
const reset         = has('--reset');

// ── Generate invite code ──────────────────────────────────────────────────
// Same alphabet as Rust INVITE_CODE_ALPHABET (no ambiguous chars 0/O/1/I/L).
const ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
const CODE_LEN = 6;

function generateCode() {
  const buf = randomBytes(CODE_LEN);
  return Array.from(buf).map(b => ALPHABET[b % ALPHABET.length]).join('');
}

const inviteCode = generateCode();

// ── Constants (must match Worker fallbacks) ───────────────────────────────
const DEV_PEPPER     = 'dev-pepper-change-in-production';
const COMMUNITY_ID   = 'com_dev_seed_001';
const USER_ID        = 'usr_dev_seed_001';
const MEMBERSHIP_ID  = 'mem_dev_seed_001';
const INVITE_ID      = 'inv_dev_seed_001';
const INVITE_EXPIRES = '2099-12-31T23:59:59.000Z'; // never expires in dev

// HMAC-SHA256(pepper, code) — identical to Rust crypto::hmac_hex
const codeHmac = createHmac('sha256', DEV_PEPPER).update(inviteCode).digest('hex');
const now      = new Date().toISOString();

// ── Confirmation helper ───────────────────────────────────────────────────
async function confirm(msg) {
  if (yes) return true;
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise(res => {
    rl.question(`${msg} [y/N] `, ans => { rl.close(); res(ans.trim().toLowerCase() === 'y'); });
  });
}

// ── Print plan ────────────────────────────────────────────────────────────
console.log('');
console.log('ciao.zinnias dev setup');
console.log('======================');
if (reset) console.log('  Mode      : RESET (local DB will be wiped)');
console.log(`  Community : ${communityName}`);
console.log(`  Admin     : ${adminName}`);
console.log(`  Invite code will be generated automatically.`);
console.log('');

const ok = await confirm(
  reset ? 'Wipe local DB, apply migrations, and seed?' : 'Apply migrations and seed?'
);
if (!ok) { console.log('Aborted.'); process.exit(0); }

// ── Step 0: reset (optional) ──────────────────────────────────────────────
if (reset) {
  console.log('\n[0/3] Wiping local wrangler D1 database...');
  const stateDir = resolve('.wrangler', 'state', 'v3', 'd1');
  if (existsSync(stateDir)) {
    rmSync(stateDir, { recursive: true, force: true });
    console.log(`  Removed ${stateDir}`);
  } else {
    console.log('  (no local DB found — nothing to wipe)');
  }
}

// ── Step 1: install ───────────────────────────────────────────────────────
console.log('\n[1/3] Installing dependencies...');
execSync('bun install', { stdio: 'inherit' });

// ── Step 2: migrations ────────────────────────────────────────────────────
console.log('\n[2/3] Applying migrations...');
// When -y is set, detach stdin so wrangler sees a non-TTY and skips its own
// confirmation prompt.  stdout/stderr still stream to the terminal.
execSync(
  'bunx wrangler d1 migrations apply zinnias-ciao-dev --local --env dev',
  { stdio: yes ? ['ignore', 'inherit', 'inherit'] : 'inherit' }
);

// ── Step 3: seed ─────────────────────────────────────────────────────────
console.log('\n[3/3] Seeding community, admin, and invite code...');

const statements = [
  `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${COMMUNITY_ID}', '${esc(communityName)}', 'Asia/Tokyo', 1, '${now}')`,
  `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${USER_ID}', '${now}')`,
  `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${MEMBERSHIP_ID}', '${COMMUNITY_ID}', '${USER_ID}', 'admin', '${esc(adminName)}', '${now}')`,
  `INSERT OR IGNORE INTO invite_codes (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) VALUES ('${INVITE_ID}', '${COMMUNITY_ID}', '${codeHmac}', '${MEMBERSHIP_ID}', '${INVITE_EXPIRES}', 'admin', '${now}')`,
];

for (const stmt of statements) {
  execSync(
    `bunx wrangler d1 execute zinnias-ciao-dev --local --env dev --command ${JSON.stringify(stmt)}`,
    { stdio: 'inherit' }
  );
}

// ── Done ──────────────────────────────────────────────────────────────────
const pad = (s, n) => String(s).slice(0, n).padEnd(n);

console.log('');
console.log('┌─────────────────────────────────────────────┐');
console.log('│  Setup complete!                            │');
console.log('│                                             │');
console.log(`│  Invite code : ${pad(inviteCode, 29)} │`);
console.log(`│  Community   : ${pad(communityName, 29)} │`);
console.log(`│  Admin       : ${pad(adminName, 29)} │`);
console.log('│                                             │');
console.log('│  Next steps:                                │');
console.log('│    bun run dev                              │');
console.log('│    open  http://localhost:8787/join         │');
console.log(`│    enter ${pad(inviteCode, 36)} │`);
console.log('└─────────────────────────────────────────────┘');
console.log('');

// ── Helpers ───────────────────────────────────────────────────────────────
function esc(s) { return String(s).replace(/'/g, "''"); }
