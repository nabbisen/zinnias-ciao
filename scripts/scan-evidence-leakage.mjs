#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 7): leakage scanner. Fails
// the package (nonzero exit) if any forbidden content class from Tooling
// Slice 2 appears anywhere in a given evidence tree — JSON evidence records
// via the field-aware `assertRedacted` sweep, free-text files (manual
// evidence templates, notes) via the narrower text sweep. Binary files
// (screenshots, CSV exports, etc.) are hashed for the record but not
// content-scanned — see `hash-evidence-artifacts.mjs`.
//
// Local-only: read-only over the given directory, no hosted call, no
// mutation of anything outside this process's own stdout.
//
// Handoff 068: the walk-and-scan core is exported (`scanEvidenceTree`) so
// `test-evidence-leakage-baseline.mjs` can call it directly — no subprocess,
// no duplicated walk logic, one source of truth for what counts as scanned
// vs. skipped.

import { readdir, readFile, stat } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { scanJsonValueForLeakage, scanTextForLeakage } from './lib/evidence-manifest.mjs';

export const JSON_EXTENSIONS = new Set(['.json']);
// S7-R1 from the Slice 7 review: `.csv` is text, and this application
// generates CSV exports containing member display names and attendance
// data (RFC-068 matrix export) — if one is ever retained as evidence, it
// must not fall through to the skip-content-scan branch unscanned.
export const TEXT_EXTENSIONS = new Set(['.md', '.txt', '.csv']);

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(path)));
    } else if (entry.isFile()) {
      files.push(path);
    }
  }
  return files;
}

// Scans `target` (an absolute path) for every forbidden content class,
// reporting paths relative to `root`. Returns `{ ok, target, exists,
// jsonFilesScanned, textFilesScanned, filesSkipped, skippedByExtension,
// findings }` — `skippedByExtension` maps each skipped file's lowercased
// extension (or `'(no extension)'`) to how many files were skipped with it,
// per Handoff 068 §3.3's inventory requirement.
export async function scanEvidenceTree(target, root) {
  let files;
  try {
    await stat(target);
    files = await walk(target);
  } catch (error) {
    return {
      ok: true,
      target: relative(root, target),
      exists: false,
      jsonFilesScanned: 0,
      textFilesScanned: 0,
      filesSkipped: 0,
      skippedByExtension: {},
      findings: [],
      note: `${relative(root, target)} does not exist yet — nothing to scan (${error.code ?? error.message})`,
    };
  }

  const findings = [];
  let jsonScanned = 0;
  let textScanned = 0;
  let skipped = 0;
  const skippedByExtension = {};

  for (const filePath of files) {
    const relPath = relative(root, filePath);
    const ext = extname(filePath).toLowerCase();
    if (JSON_EXTENSIONS.has(ext)) {
      jsonScanned += 1;
      try {
        const parsed = JSON.parse(await readFile(filePath, 'utf8'));
        const values = Array.isArray(parsed) ? parsed : [parsed];
        values.forEach((value, index) => {
          const violations = scanJsonValueForLeakage(value, Array.isArray(parsed) ? `${relPath}[${index}]` : relPath);
          for (const violation of violations) {
            findings.push({ path: relPath, category: violation.category, message: violation.message });
          }
        });
      } catch (error) {
        findings.push({ path: relPath, category: 'unparseable_json', message: `${relPath} could not be parsed as JSON: ${error.message}` });
      }
    } else if (TEXT_EXTENSIONS.has(ext)) {
      textScanned += 1;
      const violations = scanTextForLeakage(await readFile(filePath, 'utf8'), relPath);
      for (const violation of violations) {
        findings.push({ path: relPath, category: violation.category, message: violation.message });
      }
    } else {
      skipped += 1;
      const key = ext === '' ? '(no extension)' : ext;
      skippedByExtension[key] = (skippedByExtension[key] ?? 0) + 1;
    }
  }

  return {
    ok: findings.length === 0,
    target: relative(root, target),
    exists: true,
    jsonFilesScanned: jsonScanned,
    textFilesScanned: textScanned,
    filesSkipped: skipped,
    skippedByExtension,
    findings,
  };
}

const isMainModule = import.meta.url === `file://${process.argv[1]}`;

if (isMainModule) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const targetArg = process.argv[2];
  const target = resolve(targetArg ?? join(root, '.git-exclude', 'evidence'));

  const result = await scanEvidenceTree(target, root);
  console.log(JSON.stringify(result, null, 2));
  if (!result.ok) process.exitCode = 1;
}
