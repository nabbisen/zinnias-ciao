import { randomBytes } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { rmSync } from 'node:fs';
import { chmod, cp, mkdir, mkdtemp, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(scriptsDir, '..');

// R-N3 (2026-07-28 form-token-replay-detection remediation review): this
// harness copies the pre-built `workers/ssr/build/` artifact rather than
// rebuilding from source (see `bun run build`). A Rust source change with no
// rebuild in between silently produces confidently wrong evidence — every
// test using this harness would keep exercising the old compiled behavior.
// Warn (not fail — some workflows may intentionally pin an older artifact)
// whenever the artifact predates the newest file under `workers/ssr/src/`.
async function newestMtimeUnder(dir) {
  let newest = 0;
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      newest = Math.max(newest, await newestMtimeUnder(path));
    } else if (entry.isFile()) {
      const info = await stat(path);
      newest = Math.max(newest, info.mtimeMs);
    }
  }
  return newest;
}

async function warnIfWorkerArtifactIsStale() {
  const sourceDir = join(repositoryRoot, 'workers', 'ssr', 'src');
  const artifactPath = join(repositoryRoot, 'workers', 'ssr', 'build', 'index.js');
  let sourceNewest;
  let artifactMtime;
  try {
    [sourceNewest, artifactMtime] = await Promise.all([
      newestMtimeUnder(sourceDir),
      stat(artifactPath).then((info) => info.mtimeMs),
    ]);
  } catch {
    // Either path is missing; the artifact copy step just below will fail
    // loudly with a clearer message than a staleness warning would give.
    return;
  }
  if (artifactMtime < sourceNewest) {
    console.warn(
      '\n⚠️  workers/ssr/build/index.js is older than the newest file under '
      + 'workers/ssr/src/ — this isolated Worker test copies that pre-built '
      + 'artifact rather than rebuilding, so it is about to exercise stale '
      + 'compiled behavior. Run `bun run build` first if you changed Rust '
      + 'source and expect this run to reflect it.\n',
    );
  }
}

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

function configText(
  main,
  migrations,
  { includeD1, includeAbuseLimiter, recoveryEnabled, requiredSecrets, communityCreationEnabled },
) {
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
  // RFC-078: the compiled Worker always exports the `AbuseLimiter` Durable
  // Object class. Declaring the binding here by default keeps every
  // isolated-harness consumer working; omitting it is a deliberate negative
  // phase for a test that wants to prove the missing-binding fail-closed
  // path (`abuse-controls.mjs`), never an accident.
  const rootAbuseLimiter = includeAbuseLimiter
    ? `
[exports.AbuseLimiter]
type = "durable-object"
storage = "sqlite"

[[durable_objects.bindings]]
name = "ABUSE_LIMITER"
class_name = "AbuseLimiter"
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
  const devAbuseLimiter = includeAbuseLimiter
    ? `
[[env.dev.durable_objects.bindings]]
name = "ABUSE_LIMITER"
class_name = "AbuseLimiter"
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
COMMUNITY_CREATION_ENABLED = ${quoteToml(communityCreationEnabled ? 'true' : 'false')}
COMMUNITY_RECOVERY_ENABLED = ${quoteToml(recoveryEnabled ? 'true' : 'false')}
${rootD1}${rootAbuseLimiter}

[env.dev]
name = "zinnias-ciao-isolated-test-dev"

[env.dev.secrets]
required = [${required}]

[env.dev.vars]
BUILD_VERSION = "isolated-test"
LOG_LEVEL = "debug"
COMMUNITY_CREATION_ENABLED = ${quoteToml(communityCreationEnabled ? 'true' : 'false')}
COMMUNITY_RECOVERY_ENABLED = ${quoteToml(recoveryEnabled ? 'true' : 'false')}
${devD1}${devAbuseLimiter}
`;
}

export async function prepareIsolatedWorkerTest(
  label,
  {
    includeD1 = true,
    // Defaults true for every consumer of this shared harness, not only
    // RFC-078's own tests: the compiled Worker always exports the
    // `AbuseLimiter` class, so any caller that omitted this binding would
    // silently exercise a different Worker shape than production. Pass
    // `false` only for a deliberate negative phase proving the missing-
    // binding fail-closed path (see `scripts/smoke/abuse-controls.mjs`).
    includeAbuseLimiter = true,
    includeSecretFile = true,
    pepper: suppliedPepper,
    recoveryEnabled = false,
    recoveryToken,
    requiredSecrets = ['HMAC_PEPPER'],
    secretContents,
    // Defaults true to match every existing consumer's expectation (the
    // isolated harness is normally a "feature on" environment). Pass `false`
    // only for a deliberate negative-configuration phase proving the
    // disabled-flag path (RFC-050 Tooling Slice 6).
    communityCreationEnabled = true,
    // Escape hatch for negative-configuration fixtures this harness has no
    // named option for (a misnamed binding, a malformed section, etc.):
    // `(text) => text` receives the fully generated `wrangler.toml` body and
    // returns the text actually written. Use sparingly and only for a
    // deliberate negative phase — prefer a named option (like
    // `communityCreationEnabled` above) when the same negative shape will be
    // reused.
    configOverride,
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

  await warnIfWorkerArtifactIsStale();
  const workerArtifacts = join(root, 'worker-build');
  const isolatedMigrations = join(root, 'migrations');
  await cp(join(repositoryRoot, 'workers', 'ssr', 'build'), workerArtifacts, {
    recursive: true,
  });
  await cp(join(repositoryRoot, 'migrations'), isolatedMigrations, { recursive: true });

  const configPath = join(root, 'wrangler.toml');
  const workerMain = join(workerArtifacts, 'index.js');
  const migrations = isolatedMigrations;
  const generatedConfigText = configText(workerMain, migrations, {
    includeD1,
    includeAbuseLimiter,
    recoveryEnabled,
    requiredSecrets,
    communityCreationEnabled,
  });
  await writeFile(
    configPath,
    configOverride ? configOverride(generatedConfigText) : generatedConfigText,
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
