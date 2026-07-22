#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import * as nodeFs from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  LOCAL_SECRET_FILENAME,
  LocalSecretFileError,
  loadOrCreateLocalPepper,
  parseLocalPepper,
} from './lib/local-secret-file.mjs';
import { runDeveloperSetup } from './lib/setup-core.mjs';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const temporaryParent = join(repositoryRoot, '.git-exclude', 'tmp');
await nodeFs.mkdir(temporaryParent, { recursive: true });
const root = await nodeFs.mkdtemp(join(temporaryParent, 'local-secret-setup-'));
const accessLog = [];

function assertInside(path) {
  const absolute = resolve(String(path));
  assert.ok(
    absolute === root || absolute.startsWith(`${root}/`),
    `filesystem seam rejected path outside isolated root: ${absolute}`,
  );
  accessLog.push(absolute);
}

function isolatedFs(overrides = {}) {
  return {
    async open(path, ...args) {
      assertInside(path);
      return await (overrides.open ?? nodeFs.open)(path, ...args);
    },
    async lstat(path, ...args) {
      assertInside(path);
      return await (overrides.lstat ?? nodeFs.lstat)(path, ...args);
    },
  };
}

const validA = 'a'.repeat(64);
const validB = 'b'.repeat(64);

async function expectCategory(promise, category) {
  await assert.rejects(promise, (error) => {
    assert.ok(error instanceof LocalSecretFileError);
    assert.equal(error.category, category);
    assert.ok(!error.message.includes(validA) && !error.message.includes(validB));
    return true;
  });
}

function wrappedHandle(handle, overrides = {}) {
  return new Proxy(handle, {
    get(target, property) {
      if (property in overrides) return overrides[property];
      const value = target[property];
      return typeof value === 'function' ? value.bind(target) : value;
    },
  });
}

try {
  const createRoot = join(root, 'create');
  await nodeFs.mkdir(createRoot);
  const first = await loadOrCreateLocalPepper({
    projectRoot: createRoot,
    fs: isolatedFs(),
    generate: () => validA,
  });
  assert.equal(first, validA);
  const path = join(createRoot, LOCAL_SECRET_FILENAME);
  const created = await nodeFs.lstat(path);
  assert.equal(created.mode & 0o077, 0, 'created secret must be owner-only');

  const preserved = await loadOrCreateLocalPepper({
    projectRoot: createRoot,
    fs: isolatedFs(),
    generate: () => validB,
  });
  assert.equal(preserved, validA);
  assert.equal((await nodeFs.lstat(path)).ino, created.ino, 'valid secret inode was replaced');

  await nodeFs.chmod(path, 0o644);
  assert.equal(await loadOrCreateLocalPepper({ projectRoot: createRoot, fs: isolatedFs() }), validA);
  assert.equal((await nodeFs.lstat(path)).mode & 0o077, 0, 'handle chmod did not correct mode');

  assert.equal(parseLocalPepper(`export HMAC_PEPPER = "${validA}" # comment\r\n`), validA);
  assert.equal(parseLocalPepper(`HMAC_PEPPER: '${validA}'\n`), validA);
  assert.equal(parseLocalPepper(`IGNORED=x\nHMAC_PEPPER = \`${validA}\`\n`), validA);
  await expectCategory(
    Promise.resolve().then(() => parseLocalPepper(`HMAC_PEPPER=${validA}\nexport HMAC_PEPPER = ${validB}\n`)),
    'duplicate_definition',
  );
  assert.equal(
    parseLocalPepper(`HMAC_PEPPER="${'a'.repeat(32)}\\n${'b'.repeat(32)}"\n`),
    `${'a'.repeat(32)}\n${'b'.repeat(32)}`,
    'double-quoted dotenv newline escape was not interpreted exactly',
  );

  const collisionRoot = join(root, 'collision');
  await nodeFs.mkdir(collisionRoot);
  const collisionPath = join(collisionRoot, LOCAL_SECRET_FILENAME);
  let injectedCollision = false;
  const collisionFs = isolatedFs({
    async open(openPath, flags, mode) {
      if (flags === 'wx' && !injectedCollision) {
        injectedCollision = true;
        await nodeFs.writeFile(openPath, `HMAC_PEPPER=${validB}\n`, { mode: 0o600 });
      }
      return await nodeFs.open(openPath, flags, mode);
    },
  });
  assert.equal(
    await loadOrCreateLocalPepper({ projectRoot: collisionRoot, fs: collisionFs, generate: () => validA }),
    validB,
  );
  assert.ok(injectedCollision, 'exclusive-create collision was not injected');

  const ownerRoot = join(root, 'owner');
  await nodeFs.mkdir(ownerRoot);
  const ownerPath = join(ownerRoot, LOCAL_SECRET_FILENAME);
  await nodeFs.writeFile(ownerPath, `HMAC_PEPPER=${validA}\n`, { mode: 0o600 });
  await expectCategory(
    loadOrCreateLocalPepper({
      projectRoot: ownerRoot,
      fs: isolatedFs({
        async open(openPath, flags, mode) {
          const handle = await nodeFs.open(openPath, flags, mode);
          return wrappedHandle(handle, {
            async stat() {
              const stat = await handle.stat();
              return new Proxy(stat, { get: (target, property) => property === 'uid' ? -1 : target[property] });
            },
          });
        },
      }),
    }),
    'not_owned_by_current_user',
  );

  const directoryRoot = join(root, 'directory');
  await nodeFs.mkdir(join(directoryRoot, LOCAL_SECRET_FILENAME), { recursive: true });
  await expectCategory(loadOrCreateLocalPepper({ projectRoot: directoryRoot, fs: isolatedFs() }), 'non_regular_path');

  const symlinkRoot = join(root, 'symlink');
  await nodeFs.mkdir(symlinkRoot);
  const symlinkTarget = join(symlinkRoot, 'target');
  await nodeFs.writeFile(symlinkTarget, `HMAC_PEPPER=${validA}\n`, { mode: 0o600 });
  await nodeFs.symlink(symlinkTarget, join(symlinkRoot, LOCAL_SECRET_FILENAME));
  await expectCategory(loadOrCreateLocalPepper({ projectRoot: symlinkRoot, fs: isolatedFs() }), 'non_regular_path');

  for (const kind of ['socket', 'device']) {
    const nonRegularRoot = join(root, kind);
    await nodeFs.mkdir(nonRegularRoot);
    await expectCategory(
      loadOrCreateLocalPepper({
        projectRoot: nonRegularRoot,
        fs: isolatedFs({
          async lstat() {
            return {
              isFile: () => false,
              isSymbolicLink: () => false,
              isSocket: () => kind === 'socket',
              isCharacterDevice: () => kind === 'device',
            };
          },
        }),
      }),
      'non_regular_path',
    );
  }

  const permissionSwapRoot = join(root, 'permission-swap');
  await nodeFs.mkdir(permissionSwapRoot);
  const permissionPath = join(permissionSwapRoot, LOCAL_SECRET_FILENAME);
  const permissionOriginal = join(permissionSwapRoot, 'original');
  const permissionTarget = join(permissionSwapRoot, 'target');
  await nodeFs.writeFile(permissionPath, `HMAC_PEPPER=${validA}\n`, { mode: 0o644 });
  await nodeFs.writeFile(permissionTarget, 'target', { mode: 0o644 });
  let permissionSwap = false;
  const permissionFs = isolatedFs({
    async open(openPath, flags, mode) {
      const handle = await nodeFs.open(openPath, flags, mode);
      if (!permissionSwap && openPath === permissionPath) {
        permissionSwap = true;
        await nodeFs.rename(permissionPath, permissionOriginal);
        await nodeFs.symlink(permissionTarget, permissionPath);
      }
      return handle;
    },
  });
  await expectCategory(
    loadOrCreateLocalPepper({ projectRoot: permissionSwapRoot, fs: permissionFs }),
    'path_changed_during_read',
  );
  assert.equal((await nodeFs.lstat(permissionTarget)).mode & 0o077, 0o044, 'replacement target mode changed');
  assert.equal((await nodeFs.lstat(permissionOriginal)).mode & 0o077, 0, 'opened inode was not secured');

  const cleanupSwapRoot = join(root, 'cleanup-swap');
  await nodeFs.mkdir(cleanupSwapRoot);
  const cleanupPath = join(cleanupSwapRoot, LOCAL_SECRET_FILENAME);
  const displacedPath = join(cleanupSwapRoot, 'created-displaced');
  const cleanupFs = isolatedFs({
    async open(openPath, flags, mode) {
      const handle = await nodeFs.open(openPath, flags, mode);
      if (flags !== 'wx') return handle;
      return wrappedHandle(handle, {
        async writeFile() {
          await handle.writeFile('partial', { encoding: 'utf8' });
          await nodeFs.rename(cleanupPath, displacedPath);
          await nodeFs.writeFile(cleanupPath, 'replacement', { mode: 0o600 });
          throw new Error('injected_partial_write_failure');
        },
      });
    },
  });
  await assert.rejects(
    loadOrCreateLocalPepper({ projectRoot: cleanupSwapRoot, fs: cleanupFs, generate: () => validA }),
    /injected_partial_write_failure/u,
  );
  assert.equal(await nodeFs.readFile(cleanupPath, 'utf8'), 'replacement', 'replacement path was modified');
  assert.equal(await nodeFs.readFile(displacedPath, 'utf8'), '', 'created inode retained secret bytes');

  const closeRoot = join(root, 'close-failure');
  await nodeFs.mkdir(closeRoot);
  const closePath = join(closeRoot, LOCAL_SECRET_FILENAME);
  let closeAttempts = 0;
  const closeFs = isolatedFs({
    async open(openPath, flags, mode) {
      const handle = await nodeFs.open(openPath, flags, mode);
      if (flags !== 'wx') return handle;
      return wrappedHandle(handle, {
        async close() {
          closeAttempts += 1;
          if (closeAttempts === 1) throw new Error('injected_close_failure');
          return await handle.close();
        },
      });
    },
  });
  await assert.rejects(
    loadOrCreateLocalPepper({ projectRoot: closeRoot, fs: closeFs, generate: () => validA }),
    /injected_close_failure/u,
  );
  assert.equal(await nodeFs.readFile(closePath, 'utf8'), '', 'close failure retained secret bytes');

  const closeAfterReleaseRoot = join(root, 'close-after-release');
  await nodeFs.mkdir(closeAfterReleaseRoot);
  const closeAfterReleasePath = join(closeAfterReleaseRoot, LOCAL_SECRET_FILENAME);
  let releasedCloseAttempts = 0;
  const closeAfterReleaseFs = isolatedFs({
    async open(openPath, flags, mode) {
      const handle = await nodeFs.open(openPath, flags, mode);
      if (flags !== 'wx') return handle;
      return wrappedHandle(handle, {
        async close() {
          releasedCloseAttempts += 1;
          if (releasedCloseAttempts === 1) {
            await handle.close();
            throw new Error('injected_close_after_release_failure');
          }
          return await handle.close();
        },
      });
    },
  });
  await assert.rejects(
    loadOrCreateLocalPepper({
      projectRoot: closeAfterReleaseRoot,
      fs: closeAfterReleaseFs,
      generate: () => validA,
    }),
    /injected_close_after_release_failure/u,
  );
  assert.equal(
    await nodeFs.readFile(closeAfterReleasePath, 'utf8'),
    '',
    'close-after-release failure retained secret bytes',
  );

  const setupRoot = join(root, 'actual-modes');
  await nodeFs.mkdir(setupRoot);
  const setupEvents = [];
  const adapter = {
    confirm: async (_message, options) => options.yes || true,
    loadPepper: async (projectRoot) => await loadOrCreateLocalPepper({
      projectRoot,
      fs: isolatedFs(),
      generate: () => validA,
    }),
    resetLocalDatabase: async () => setupEvents.push('reset'),
    installDependencies: async () => setupEvents.push('install'),
    applyMigrations: async (options) => setupEvents.push(`migrate:${options.yes}`),
    executeSql: async (statement) => setupEvents.push(statement),
  };
  const modeRuns = [];
  for (const [argv, inviteCode] of [
    [[], 'ABC234'],
    [['-y'], 'DEF567'],
    [['--reset', '-y'], 'GHJ789'],
  ]) {
    modeRuns.push(await runDeveloperSetup({
      argv,
      projectRoot: setupRoot,
      adapter,
      generateCode: () => inviteCode,
      now: () => '2026-07-22T00:00:00.000Z',
    }));
  }
  const setupStat = await nodeFs.lstat(join(setupRoot, LOCAL_SECRET_FILENAME));
  assert.equal(setupStat.mode & 0o077, 0);
  for (const result of modeRuns) {
    const expectedHmac = createHmac('sha256', validA).update(result.inviteCode).digest('hex');
    assert.equal(result.codeHmac, expectedHmac, 'seed HMAC disagreed with loaded local pepper');
    assert.ok(result.statements[3].includes(`'${expectedHmac}'`));
  }
  assert.equal(setupEvents.filter((event) => event === 'reset').length, 1);
  assert.deepEqual(setupEvents.filter((event) => event.startsWith('migrate:')), [
    'migrate:false',
    'migrate:true',
    'migrate:true',
  ]);

  assert.ok(accessLog.length > 0, 'filesystem seam recorded no operations');
  console.log(JSON.stringify({
    ok: true,
    phases: {
      exclusiveCreateAndCollision: true,
      fileHandlePermissionsAndOwner: true,
      dotenvIdentityAndAmbiguity: true,
      nonRegularPaths: true,
      permissionSwapSafety: true,
      cleanupSwapSafety: true,
      closeFailureSanitization: true,
      closeAfterReleaseSanitization: true,
      actualSetupModesAndSeedHmac: true,
      isolatedPathAccess: true,
    },
  }));
} finally {
  await nodeFs.rm(root, { recursive: true, force: true });
}
