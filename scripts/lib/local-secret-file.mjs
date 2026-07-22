import { randomBytes } from 'node:crypto';
import { constants as fsConstants } from 'node:fs';
import * as nodeFs from 'node:fs/promises';
import { resolve } from 'node:path';

export const LOCAL_SECRET_FILENAME = '.dev.vars.dev';
const MIN_BYTES = 32;
const MAX_BYTES = 4096;
const LEGACY_SENTINELS = new Set(['dev-pepper-change-in-production', 'dev-pepper']);

// Keep this expression and the value transformations in sync with the dotenv
// parser bundled by the repository's pinned Wrangler. Counting every matching
// definition before selecting a value prevents dotenv's normal last-value-wins
// behavior from making setup and the Worker disagree about ambiguous files.
const DOTENV_LINE = /(?:^|^)\s*(?:export\s+)?([\w.-]+)(?:\s*=\s*?|:\s+?)(\s*'(?:\\'|[^'])*'|\s*"(?:\\"|[^"])*"|\s*`(?:\\`|[^`])*`|[^#\r\n]+)?\s*(?:#.*)?(?:$|$)/gmu;

export class LocalSecretFileError extends Error {
  constructor(category) {
    super(`local secret setup failed: ${category}`);
    this.name = 'LocalSecretFileError';
    this.category = category;
  }
}

export function validateLocalPepper(value) {
  if (value.length === 0 || value.trim().length === 0) {
    throw new LocalSecretFileError('empty');
  }
  if (value.trim() !== value) throw new LocalSecretFileError('surrounding_whitespace');
  if (LEGACY_SENTINELS.has(value)) throw new LocalSecretFileError('legacy_sentinel');
  const bytes = Buffer.byteLength(value, 'utf8');
  if (bytes < MIN_BYTES) throw new LocalSecretFileError('too_short');
  if (bytes > MAX_BYTES) throw new LocalSecretFileError('too_long');
  return value;
}

function dotenvValue(rawValue) {
  let value = (rawValue ?? '').trim();
  const quote = value[0];
  value = value.replace(/^(['"`])([\s\S]*)\1$/gmu, '$2');
  if (quote === '"') {
    value = value.replace(/\\n/gu, '\n').replace(/\\r/gu, '\r');
  }
  return value;
}

export function parseLocalPepper(contents) {
  const source = String(contents).replace(/\r\n?/gmu, '\n');
  const definitions = [];
  DOTENV_LINE.lastIndex = 0;
  let match;
  while ((match = DOTENV_LINE.exec(source)) !== null) {
    if (match[1] === 'HMAC_PEPPER') definitions.push(dotenvValue(match[2]));
  }
  if (definitions.length === 0) throw new LocalSecretFileError('missing_definition');
  if (definitions.length !== 1) throw new LocalSecretFileError('duplicate_definition');
  return validateLocalPepper(definitions[0]);
}

function sameInode(left, right) {
  return left.dev === right.dev && left.ino === right.ino;
}

function assertSafeOpenedFile(metadata, category) {
  if (!metadata.isFile()) throw new LocalSecretFileError(category);
  if (typeof process.getuid === 'function' && metadata.uid !== process.getuid()) {
    throw new LocalSecretFileError('not_owned_by_current_user');
  }
}

async function sanitizeCreatedInode(fs, path, createdStat, originalHandle) {
  async function sanitizeHandle(handle) {
    const opened = await handle.stat();
    if (!sameInode(createdStat, opened)) return false;
    await handle.truncate(0);
    if (typeof handle.sync === 'function') await handle.sync();
    return true;
  }

  if (originalHandle) {
    try {
      if (await sanitizeHandle(originalHandle)) return;
    } catch {
      // close() may reject after releasing the descriptor. Fall through to a
      // non-following reopen and compare the reopened device/inode identity.
    }
  }

  let reopened;
  try {
    const noFollow = fsConstants.O_NOFOLLOW ?? 0;
    const nonBlock = fsConstants.O_NONBLOCK ?? 0;
    reopened = await fs.open(path, fsConstants.O_WRONLY | noFollow | nonBlock);
    await sanitizeHandle(reopened);
  } catch (error) {
    // A missing or replaced path is safe: this invocation never follows or
    // removes it. Other cleanup failures are surfaced without secret content.
    if (!['ENOENT', 'ELOOP', 'ENXIO'].includes(error?.code)) {
      throw new LocalSecretFileError('created_inode_sanitization_failed');
    }
  } finally {
    if (reopened) {
      try {
        await reopened.close();
      } catch {
        throw new LocalSecretFileError('created_inode_sanitization_close_failed');
      }
    }
  }
}

async function createSecret(fs, path, generate) {
  const pepper = validateLocalPepper(generate());
  let handle;
  let createdStat;
  try {
    handle = await fs.open(path, 'wx', 0o600);
    createdStat = await handle.stat();
    assertSafeOpenedFile(createdStat, 'created_path_not_regular');
    await handle.chmod(0o600);
    const secured = await handle.stat();
    if (!sameInode(createdStat, secured) || (secured.mode & 0o077) !== 0) {
      throw new LocalSecretFileError('created_permissions_not_owner_only');
    }
    await handle.writeFile(`HMAC_PEPPER=${pepper}\n`, { encoding: 'utf8' });
    if (typeof handle.sync === 'function') await handle.sync();
    const currentPath = await fs.lstat(path);
    if (!sameInode(createdStat, currentPath)) {
      throw new LocalSecretFileError('created_path_replaced');
    }
    await handle.close();
    handle = undefined;
    return pepper;
  } catch (error) {
    if (error?.code === 'EEXIST') return null;
    let cleanupError;
    if (createdStat) {
      try {
        await sanitizeCreatedInode(fs, path, createdStat, handle);
      } catch (failure) {
        cleanupError = failure;
      }
    }
    if (handle) {
      try {
        await handle.close();
      } catch {
        // Failure cleanup has already sanitized the created inode. A second
        // close can legitimately report EBADF after close-after-release.
      }
    }
    throw cleanupError ?? error;
  }
}

async function readExistingSecret(fs, path) {
  let metadata;
  try {
    metadata = await fs.lstat(path);
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new LocalSecretFileError('non_regular_path');
  }

  let handle;
  try {
    const noFollow = fsConstants.O_NOFOLLOW ?? 0;
    const nonBlock = fsConstants.O_NONBLOCK ?? 0;
    handle = await fs.open(path, fsConstants.O_RDONLY | noFollow | nonBlock);
    const opened = await handle.stat();
    if (!sameInode(metadata, opened)) {
      throw new LocalSecretFileError('path_changed_before_read');
    }
    assertSafeOpenedFile(opened, 'non_regular_path');
    if ((opened.mode & 0o077) !== 0) {
      await handle.chmod(0o600);
      const corrected = await handle.stat();
      if (!sameInode(opened, corrected) || (corrected.mode & 0o077) !== 0) {
        throw new LocalSecretFileError('permissions_not_owner_only');
      }
    }
    const contents = await handle.readFile({ encoding: 'utf8' });
    const currentPath = await fs.lstat(path);
    if (!sameInode(opened, currentPath)) {
      throw new LocalSecretFileError('path_changed_during_read');
    }
    return parseLocalPepper(contents);
  } finally {
    if (handle) await handle.close();
  }
}

export async function loadOrCreateLocalPepper({
  projectRoot,
  fs = nodeFs,
  generate = () => randomBytes(32).toString('hex'),
} = {}) {
  if (!projectRoot) throw new LocalSecretFileError('missing_project_root');
  const root = resolve(projectRoot);
  const path = resolve(root, LOCAL_SECRET_FILENAME);
  if (!path.startsWith(`${root}/`)) throw new LocalSecretFileError('path_escaped_project_root');

  const existing = await readExistingSecret(fs, path);
  if (existing !== null) return existing;
  const created = await createSecret(fs, path, generate);
  if (created !== null) return created;
  const raced = await readExistingSecret(fs, path);
  if (raced === null) throw new LocalSecretFileError('exclusive_create_race');
  return raced;
}
