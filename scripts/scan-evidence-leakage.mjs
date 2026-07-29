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

import { readdir, readFile, stat } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  EvidenceLeakageError,
  EvidenceRedactionError,
  scanJsonValueForLeakage,
  scanTextForLeakage,
} from './lib/evidence-manifest.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const targetArg = process.argv[2];
const target = resolve(targetArg ?? join(root, '.git-exclude', 'evidence'));

const JSON_EXTENSIONS = new Set(['.json']);
// S7-R1 from the Slice 7 review: `.csv` is text, and this application
// generates CSV exports containing member display names and attendance
// data (RFC-068 matrix export) — if one is ever retained as evidence, it
// must not fall through to the skip-content-scan branch unscanned.
const TEXT_EXTENSIONS = new Set(['.md', '.txt', '.csv']);

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

let files;
try {
  await stat(target);
  files = await walk(target);
} catch (error) {
  console.log(JSON.stringify({
    ok: true,
    scanned: 0,
    findings: [],
    note: `${relative(root, target)} does not exist yet — nothing to scan (${error.code ?? error.message})`,
  }));
  process.exit(0);
}

const findings = [];
let jsonScanned = 0;
let textScanned = 0;
let skipped = 0;

for (const filePath of files) {
  const relPath = relative(root, filePath);
  const ext = extname(filePath).toLowerCase();
  if (JSON_EXTENSIONS.has(ext)) {
    jsonScanned += 1;
    try {
      const parsed = JSON.parse(await readFile(filePath, 'utf8'));
      const values = Array.isArray(parsed) ? parsed : [parsed];
      values.forEach((value, index) => {
        try {
          scanJsonValueForLeakage(value, Array.isArray(parsed) ? `${relPath}[${index}]` : relPath);
        } catch (error) {
          if (error instanceof EvidenceRedactionError) {
            findings.push({ path: relPath, category: error.category, message: error.message });
          } else {
            throw error;
          }
        }
      });
    } catch (error) {
      findings.push({ path: relPath, category: 'unparseable_json', message: `${relPath} could not be parsed as JSON: ${error.message}` });
    }
  } else if (TEXT_EXTENSIONS.has(ext)) {
    textScanned += 1;
    try {
      scanTextForLeakage(await readFile(filePath, 'utf8'), relPath);
    } catch (error) {
      if (error instanceof EvidenceLeakageError) {
        findings.push({ path: relPath, category: error.category, message: error.message });
      } else {
        throw error;
      }
    }
  } else {
    skipped += 1;
  }
}

const passed = findings.length === 0;
console.log(JSON.stringify({
  ok: passed,
  target: relative(root, target),
  jsonFilesScanned: jsonScanned,
  textFilesScanned: textScanned,
  filesSkipped: skipped,
  findings,
}, null, 2));

if (!passed) process.exitCode = 1;
