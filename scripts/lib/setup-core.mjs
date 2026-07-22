import { createHmac, randomBytes } from 'node:crypto';

const ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
const CODE_LEN = 6;

const COMMUNITY_ID = 'com_dev_seed_001';
const USER_ID = 'usr_dev_seed_001';
const MEMBERSHIP_ID = 'mem_dev_seed_001';
const INVITE_ID = 'inv_dev_seed_001';
const INVITE_EXPIRES = '2099-12-31T23:59:59.000Z';

export function parseSetupArguments(argv) {
  const get = (flag) => {
    const index = argv.indexOf(flag);
    return index === -1 ? null : (argv[index + 1] ?? null);
  };
  const has = (flag) => argv.includes(flag);
  return {
    communityName: get('--community') ?? 'My Community',
    adminName: get('--admin') ?? 'Admin',
    yes: has('-y') || has('--yes'),
    reset: has('--reset'),
  };
}

export function generateInviteCode() {
  const bytes = randomBytes(CODE_LEN);
  return Array.from(bytes, (byte) => ALPHABET[byte % ALPHABET.length]).join('');
}

function escapeSql(value) {
  return String(value).replace(/'/gu, "''");
}

export async function runDeveloperSetup({
  argv,
  projectRoot,
  adapter,
  generateCode = generateInviteCode,
  now = () => new Date().toISOString(),
}) {
  const options = parseSetupArguments(argv);
  const confirmed = await adapter.confirm(
    options.reset ? 'Wipe local DB, apply migrations, and seed?' : 'Apply migrations and seed?',
    options,
  );
  if (!confirmed) return { aborted: true, ...options };

  // This call is shared by normal, reset, and non-interactive operation. It
  // occurs before any reset or command so unsafe secret state fails closed.
  const pepper = await adapter.loadPepper(projectRoot);
  const inviteCode = generateCode();
  const codeHmac = createHmac('sha256', pepper).update(inviteCode).digest('hex');
  const timestamp = now();

  if (options.reset) await adapter.resetLocalDatabase();
  await adapter.installDependencies();
  await adapter.applyMigrations(options);

  const statements = [
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${COMMUNITY_ID}', '${escapeSql(options.communityName)}', 'Asia/Tokyo', 1, '${timestamp}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${USER_ID}', '${timestamp}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${MEMBERSHIP_ID}', '${COMMUNITY_ID}', '${USER_ID}', 'admin', '${escapeSql(options.adminName)}', '${timestamp}')`,
    `INSERT OR IGNORE INTO invite_codes (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at) VALUES ('${INVITE_ID}', '${COMMUNITY_ID}', '${codeHmac}', '${MEMBERSHIP_ID}', '${INVITE_EXPIRES}', 'admin', '${timestamp}')`,
  ];
  for (const statement of statements) await adapter.executeSql(statement);

  return { aborted: false, ...options, inviteCode, codeHmac, statements };
}
