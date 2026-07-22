#!/usr/bin/env node

import { randomBytes } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { runBootstrap } from './lib/bootstrap-cloudflare-core.mjs';

function prompt(message) {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(message, (answer) => {
      rl.close();
      resolve(answer.trim());
    });
  });
}

const adapter = {
  log(message = '') {
    console.log(message);
  },
  now() {
    return new Date().toISOString();
  },
  prompt,
  randomBytes,
  runWrangler(args, { input } = {}) {
    if (input !== undefined) {
      const result = spawnSync('bunx', ['wrangler', ...args], {
        input,
        stdio: ['pipe', 'inherit', 'inherit'],
        encoding: 'utf8',
      });
      if (result.status !== 0) throw new Error(`wrangler exited ${result.status ?? 1}`);
      return { stdout: result.stdout ?? '' };
    }
    return {
      stdout: execFileSync('bunx', ['wrangler', ...args], {
        stdio: ['ignore', 'pipe', 'inherit'],
        encoding: 'utf8',
      }),
    };
  },
};

try {
  await runBootstrap({ argv: process.argv.slice(2), adapter });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
