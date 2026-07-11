#!/usr/bin/env node
// RFC-069 operator-assisted community access recovery. Prints one plaintext
// relink code; do not redirect output into evidence logs.

import { createInterface } from 'node:readline';

const args = process.argv.slice(2);
const get = (flag) => {
  const i = args.indexOf(flag);
  return i !== -1 ? args[i + 1] ?? null : null;
};
const has = (flag) => args.includes(flag);

function usage() {
  console.error(`Usage:
COMMUNITY_RECOVERY_TOKEN="<operator-token>" \\
node scripts/recover-community-access.mjs \\
  --target staging|production \\
  --url https://<worker-host> \\
  --community-id com_... \\
  --admin-membership-id mem_... \\
  --operator-label INC-1234 \\
  [--confirm-production]`);
}

function requireArg(flag) {
  const value = get(flag);
  if (!value || value.trim() === '') {
    console.error(`Missing required ${flag}.`);
    usage();
    process.exit(2);
  }
  return value.trim();
}

function requireOperatorLabel(label) {
  if (label.length > 80 || /[\u0000-\u001f\u007f]/u.test(label)) {
    console.error('--operator-label must be short plain text with no control characters.');
    process.exit(2);
  }
}

async function confirmProduction() {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question('Type "production" to create a production recovery relink code: ', (answer) => {
      rl.close();
      resolve(answer.trim() === 'production');
    });
  });
}

const target = requireArg('--target');
if (!['staging', 'production'].includes(target)) {
  console.error('Unknown --target. Expected "staging" or "production".');
  process.exit(2);
}

const baseUrl = requireArg('--url').replace(/\/+$/u, '');
let endpoint;
try {
  endpoint = new URL('/operator/recovery/community-access', baseUrl);
} catch {
  console.error('--url must be an absolute Worker URL.');
  process.exit(2);
}

const communityId = requireArg('--community-id');
const adminMembershipId = requireArg('--admin-membership-id');
const operatorLabel = requireArg('--operator-label');
requireOperatorLabel(operatorLabel);

const token = process.env.COMMUNITY_RECOVERY_TOKEN;
if (!token || token.trim() === '') {
  console.error('Missing COMMUNITY_RECOVERY_TOKEN environment variable.');
  process.exit(2);
}

if (target === 'production') {
  if (!has('--confirm-production')) {
    console.error('Production recovery requires --confirm-production.');
    process.exit(2);
  }
  const confirmed = await confirmProduction();
  if (!confirmed) {
    console.error('Aborted.');
    process.exit(1);
  }
}

const response = await fetch(endpoint, {
  method: 'POST',
  headers: {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
    Accept: 'application/json',
  },
  body: JSON.stringify({
    community_id: communityId,
    admin_membership_id: adminMembershipId,
    operator_label: operatorLabel,
  }),
});

if (!response.ok) {
  console.error(`Recovery request failed with HTTP ${response.status}.`);
  process.exit(1);
}

let payload;
try {
  payload = await response.json();
} catch {
  console.error('Recovery endpoint returned a non-JSON response.');
  process.exit(1);
}

if (!payload?.ok || !payload.relink_code || !payload.expires_at) {
  console.error('Recovery endpoint returned an unexpected response.');
  process.exit(1);
}

console.log('');
console.log('Community access recovery relink code');
console.log('=====================================');
console.log(`Target       : ${target}`);
console.log(`Community    : ${payload.community_id}`);
console.log(`Membership   : ${payload.admin_membership_id}`);
console.log(`Expires at   : ${payload.expires_at}`);
console.log(`Relink URL   : ${baseUrl}/relink`);
console.log(`Relink code  : ${payload.relink_code}`);
console.log('');
console.log('Give the relink code only to the intended existing active admin.');
console.log('After recovery, disable COMMUNITY_RECOVERY_ENABLED, rotate or delete');
console.log('COMMUNITY_RECOVERY_TOKEN, redeploy, and verify the endpoint is closed.');
