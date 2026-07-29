#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 7): artifact hashing.
// Computes a `sha256:<hex>` hash for every file under a given evidence
// directory (every kind — JSON, markdown, screenshots, CSV exports — not
// just structured records) and writes a single `HASHES.sha256` manifest at
// that directory's root, in the standard `<hash>  <relative-path>` format
// `sha256sum -c` understands, so every artifact's integrity is independently
// checkable later.
//
// Local-only: read-only over the given directory except for writing the one
// manifest file. No hosted call, no other mutation.

import { readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { hashToArtifactHash } from './lib/evidence-manifest.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetArg = process.argv[2];
const target = resolve(targetArg ?? join(root, '.git-exclude', 'evidence'));
const MANIFEST_NAME = 'HASHES.sha256';

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.name === MANIFEST_NAME) continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(path)));
    } else if (entry.isFile()) {
      files.push(path);
    }
  }
  return files;
}

try {
  await stat(target);
} catch (error) {
  console.log(JSON.stringify({ ok: true, hashed: 0, note: `${relative(root, target)} does not exist yet (${error.code ?? error.message})` }));
  process.exit(0);
}

const files = (await walk(target)).sort();
const lines = [];
for (const filePath of files) {
  const contents = await readFile(filePath);
  const hash = hashToArtifactHash(contents).replace('sha256:', '');
  lines.push(`${hash}  ${relative(target, filePath)}`);
}

const manifestPath = join(target, MANIFEST_NAME);
await writeFile(manifestPath, `${lines.join('\n')}\n`);

console.log(JSON.stringify({
  ok: true,
  target: relative(root, target),
  hashed: files.length,
  manifest: relative(root, manifestPath),
}));
