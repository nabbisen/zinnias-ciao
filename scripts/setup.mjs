#!/usr/bin/env node
// ciao.zinnias local developer setup. Hosted resources are never touched.

import { execSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { existsSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { loadOrCreateLocalPepper } from './lib/local-secret-file.mjs';
import { parseSetupArguments, runDeveloperSetup } from './lib/setup-core.mjs';

const argv = process.argv.slice(2);
const preview = parseSetupArguments(argv);

console.log('');
console.log('ciao.zinnias dev setup');
console.log('======================');
if (preview.reset) console.log('  Mode      : RESET (local DB will be wiped)');
console.log(`  Community : ${preview.communityName}`);
console.log(`  Admin     : ${preview.adminName}`);
console.log('  Invite code will be generated automatically.');
console.log('');

async function prompt(message) {
  const readline = createInterface({ input: process.stdin, output: process.stdout });
  return await new Promise((resolveAnswer) => {
    readline.question(`${message} [y/N] `, (answer) => {
      readline.close();
      resolveAnswer(answer.trim().toLowerCase() === 'y');
    });
  });
}

const result = await runDeveloperSetup({
  argv,
  projectRoot: process.cwd(),
  adapter: {
    confirm: async (message, options) => options.yes || await prompt(message),
    loadPepper: async (projectRoot) => await loadOrCreateLocalPepper({ projectRoot }),
    async resetLocalDatabase() {
      console.log('\n[0/3] Wiping local wrangler D1 database...');
      const stateDir = resolve('.wrangler', 'state', 'v3', 'd1');
      if (existsSync(stateDir)) {
        rmSync(stateDir, { recursive: true, force: true });
        console.log(`  Removed ${stateDir}`);
      } else {
        console.log('  (no local DB found — nothing to wipe)');
      }
    },
    async installDependencies() {
      console.log('\n[1/3] Installing dependencies...');
      execSync('bun install', { stdio: 'inherit' });
    },
    async applyMigrations(options) {
      console.log('\n[2/3] Applying migrations...');
      execSync('bunx wrangler d1 migrations apply zinnias-ciao-dev --local --env dev', {
        stdio: options.yes ? ['ignore', 'inherit', 'inherit'] : 'inherit',
      });
    },
    async executeSql(statement) {
      execSync(
        `bunx wrangler d1 execute zinnias-ciao-dev --local --env dev --command ${JSON.stringify(statement)}`,
        { stdio: 'inherit' },
      );
    },
  },
});

if (result.aborted) {
  console.log('Aborted.');
  process.exit(0);
}

const pad = (value, length) => String(value).slice(0, length).padEnd(length);
console.log('');
console.log('┌─────────────────────────────────────────────┐');
console.log('│  Setup complete!                            │');
console.log('│                                             │');
console.log(`│  Invite code : ${pad(result.inviteCode, 29)} │`);
console.log(`│  Community   : ${pad(result.communityName, 29)} │`);
console.log(`│  Admin       : ${pad(result.adminName, 29)} │`);
console.log('│                                             │');
console.log('│  Next steps:                                │');
console.log('│    bun run dev                              │');
console.log('│    open  http://localhost:8787/join         │');
console.log(`│    enter ${pad(result.inviteCode, 36)} │`);
console.log('└─────────────────────────────────────────────┘');
console.log('');
