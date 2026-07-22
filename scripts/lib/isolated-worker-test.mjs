import { randomBytes } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { rmSync } from 'node:fs';
import { chmod, cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(scriptsDir, '..');

const CONTROL_ENVIRONMENT = new Set([
  'CLOUDFLARE_INCLUDE_PROCESS_ENV',
  'CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV',
]);
const FORBIDDEN_BINDINGS = new Set(['HMAC_PEPPER', 'COMMUNITY_RECOVERY_TOKEN']);

function safeLabel(value) {
  if (!/^[a-z0-9-]+$/u.test(value)) throw new Error('isolated test label must be kebab-case');
  return value;
}

function quoteToml(value) {
  return JSON.stringify(String(value));
}

function childEnvironment(root) {
  const environment = {
    PATH: process.env.PATH ?? '/usr/local/bin:/usr/bin:/bin',
    HOME: join(root, 'home'),
    XDG_CONFIG_HOME: join(root, 'xdg'),
    TMPDIR: join(root, 'tmp'),
    NO_COLOR: '1',
    CLOUDFLARE_INCLUDE_PROCESS_ENV: 'false',
    CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV: 'false',
  };
  for (const key of ['LANG', 'LC_ALL', 'LC_CTYPE', 'TERM']) {
    if (process.env[key] !== undefined) environment[key] = process.env[key];
  }
  assertIsolatedChildEnvironment(environment);
  return Object.freeze(environment);
}

export function assertIsolatedChildEnvironment(environment) {
  for (const key of Object.keys(environment)) {
    const authority =
      (key.startsWith('CLOUDFLARE_') && !CONTROL_ENVIRONMENT.has(key)) ||
      key.startsWith('CF_') ||
      key.startsWith('WRANGLER_') ||
      key.startsWith('AWS_') ||
      key.startsWith('GOOGLE_') ||
      key.startsWith('AZURE_') ||
      FORBIDDEN_BINDINGS.has(key);
    if (authority) throw new Error(`forbidden inherited Worker environment key: ${key}`);
  }
  if (environment.CLOUDFLARE_INCLUDE_PROCESS_ENV !== 'false') {
    throw new Error('CLOUDFLARE_INCLUDE_PROCESS_ENV must be fixed false');
  }
  if (environment.CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV !== 'false') {
    throw new Error('CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV must be fixed false');
  }
}

function configText(main, migrations, { includeD1, includeKv, recoveryEnabled, requiredSecrets }) {
  const required = requiredSecrets.map(quoteToml).join(', ');
  const rootD1 = includeD1
    ? `
[[d1_databases]]
binding = "DB"
database_name = "zinnias-ciao-dev"
database_id = "local"
migrations_dir = ${quoteToml(migrations)}
`
    : '';
  const rootKv = includeKv
    ? `
[[kv_namespaces]]
binding = "RATE_LIMIT"
id = "local"
`
    : '';
  const devD1 = includeD1
    ? `
[[env.dev.d1_databases]]
binding = "DB"
database_name = "zinnias-ciao-dev"
database_id = "local"
migrations_dir = ${quoteToml(migrations)}
`
    : '';
  const devKv = includeKv
    ? `
[[env.dev.kv_namespaces]]
binding = "RATE_LIMIT"
id = "local"
`
    : '';
  return `name = "zinnias-ciao-isolated-test"
main = ${quoteToml(main)}
compatibility_date = "2025-01-01"
compatibility_flags = ["nodejs_compat"]

[secrets]
required = [${required}]

[vars]
BUILD_VERSION = "isolated-test"
LOG_LEVEL = "debug"
COMMUNITY_CREATION_ENABLED = "true"
COMMUNITY_RECOVERY_ENABLED = ${quoteToml(recoveryEnabled ? 'true' : 'false')}
${rootD1}${rootKv}

[env.dev]
name = "zinnias-ciao-isolated-test-dev"

[env.dev.secrets]
required = [${required}]

[env.dev.vars]
BUILD_VERSION = "isolated-test"
LOG_LEVEL = "debug"
COMMUNITY_CREATION_ENABLED = "true"
COMMUNITY_RECOVERY_ENABLED = ${quoteToml(recoveryEnabled ? 'true' : 'false')}
${devD1}${devKv}
`;
}

export async function prepareIsolatedWorkerTest(
  label,
  {
    includeD1 = true,
    includeKv = true,
    includeSecretFile = true,
    pepper: suppliedPepper,
    recoveryEnabled = false,
    recoveryToken,
    requiredSecrets = ['HMAC_PEPPER'],
    secretContents,
  } = {},
) {
  safeLabel(label);
  const temporaryParent = join(repositoryRoot, '.git-exclude', 'tmp');
  await mkdir(temporaryParent, { recursive: true });
  const container = await mkdtemp(join(temporaryParent, `${label}-container-`));
  const root = join(container, 'worker-root');
  const home = join(root, 'home');
  const xdg = join(root, 'xdg');
  const tmp = join(root, 'tmp');
  const persistTo = join(root, 'state');
  await Promise.all([
    mkdir(root, { recursive: true }),
    mkdir(home, { recursive: true }),
    mkdir(xdg, { recursive: true }),
    mkdir(tmp, { recursive: true }),
    mkdir(persistTo, { recursive: true }),
  ]);

  // A valid secret in an ancestor outside the config/CWD root must never be
  // discovered. Missing-secret phases prove this canary cannot satisfy the
  // Worker's required binding.
  const canaryPath = join(container, '.dev.vars.dev');
  await writeFile(canaryPath, `HMAC_PEPPER=${randomBytes(32).toString('hex')}\n`, {
    flag: 'wx',
    mode: 0o600,
  });
  await chmod(canaryPath, 0o600);

  const workerArtifacts = join(root, 'worker-build');
  const isolatedMigrations = join(root, 'migrations');
  await cp(join(repositoryRoot, 'workers', 'ssr', 'build'), workerArtifacts, {
    recursive: true,
  });
  await cp(join(repositoryRoot, 'migrations'), isolatedMigrations, { recursive: true });

  const configPath = join(root, 'wrangler.toml');
  const workerMain = join(workerArtifacts, 'index.js');
  const migrations = isolatedMigrations;
  await writeFile(
    configPath,
    configText(workerMain, migrations, {
      includeD1,
      includeKv,
      recoveryEnabled,
      requiredSecrets,
    }),
    { mode: 0o600 },
  );
  await chmod(configPath, 0o600);

  const pepper = suppliedPepper ?? randomBytes(32).toString('hex');
  const secretPath = join(root, '.dev.vars.dev');
  if (includeSecretFile) {
    const contents =
      secretContents ??
      `HMAC_PEPPER=${pepper}\n${recoveryToken === undefined ? '' : `COMMUNITY_RECOVERY_TOKEN=${recoveryToken}\n`}`;
    await writeFile(secretPath, contents, { flag: 'wx', mode: 0o600 });
    await chmod(secretPath, 0o600);
  }

  const env = childEnvironment(root);
  const wranglerBin = join(repositoryRoot, 'node_modules', '.bin', 'wrangler');
  const environmentAuditPath = join(root, 'child-environment-keys.json');
  const wrapperPath = join(root, 'wrangler-child-wrapper.mjs');
  await writeFile(
    wrapperPath,
    `import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';

const control = new Set(['CLOUDFLARE_INCLUDE_PROCESS_ENV', 'CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV']);
const forbiddenBindings = new Set(['HMAC_PEPPER', 'COMMUNITY_RECOVERY_TOKEN']);
const keys = Object.keys(process.env).sort();
writeFileSync(${JSON.stringify(environmentAuditPath)}, JSON.stringify(keys));
for (const key of keys) {
  const forbidden =
    (key.startsWith('CLOUDFLARE_') && !control.has(key)) ||
    key.startsWith('CF_') || key.startsWith('WRANGLER_') ||
    key.startsWith('AWS_') || key.startsWith('GOOGLE_') || key.startsWith('AZURE_') ||
    forbiddenBindings.has(key);
  if (forbidden) throw new Error('forbidden child environment key: ' + key);
}
if (process.env.CLOUDFLARE_INCLUDE_PROCESS_ENV !== 'false' ||
    process.env.CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV !== 'false') {
  throw new Error('child environment controls are not fixed false');
}
const child = spawn(${JSON.stringify(wranglerBin)}, process.argv.slice(2), {
  env: process.env,
  stdio: 'inherit',
});
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => child.kill(signal));
}
child.on('error', (error) => { throw error; });
child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});
`,
    { mode: 0o700 },
  );
  await chmod(wrapperPath, 0o700);
  let cleaned = false;
  const cleanupOnExit = () => {
    if (!cleaned) rmSync(container, { recursive: true, force: true });
  };
  process.once('exit', cleanupOnExit);

  function argsWithIsolation(args) {
    const result = [...args];
    if (!result.includes('--config')) result.push('--config', configPath);
    if (!result.includes('--persist-to')) result.push('--persist-to', persistTo);
    return result;
  }

  function runWranglerSync(args, options = {}) {
    assertIsolatedChildEnvironment(env);
    return execFileSync(process.execPath, [wrapperPath, ...argsWithIsolation(args)], {
      cwd: root,
      env,
      stdio: options.stdio ?? ['ignore', 'pipe', 'pipe'],
      encoding: options.encoding,
    });
  }

  function spawnDev(port) {
    assertIsolatedChildEnvironment(env);
    return spawn(
      process.execPath,
      [wrapperPath, ...argsWithIsolation([
        'dev',
        '--env',
        'dev',
        '--local',
        '--ip',
        '127.0.0.1',
        '--port',
        String(port),
      ])],
      { cwd: root, env, stdio: ['ignore', 'ignore', 'pipe'] },
    );
  }

  async function cleanup() {
    if (cleaned) return;
    cleaned = true;
    process.removeListener('exit', cleanupOnExit);
    await rm(container, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }

  async function assertChildEnvironmentAudit() {
    const keys = JSON.parse(await readFile(environmentAuditPath, 'utf8'));
    assertIsolatedChildEnvironment(Object.fromEntries(keys.map((key) => [key, env[key]])));
    return true;
  }

  return Object.freeze({
    assertChildEnvironmentAudit,
    canaryPath,
    cleanup,
    configPath,
    env,
    environmentAuditPath,
    pepper,
    persistTo,
    repositoryRoot,
    root,
    runWranglerSync,
    secretPath,
    spawnDev,
    workerArtifacts,
    wranglerBin,
  });
}
