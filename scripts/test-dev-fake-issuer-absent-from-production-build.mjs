#!/usr/bin/env node
// Handoff 054 §3/§7: a production build must be STRUCTURALLY incapable of
// verifying a token against `idns_local_fake` — not merely unlikely to, not
// prevented by configuration alone. The mechanism is the `dev_fake_issuer`
// Cargo feature (default off): with it off, `workers/ssr/src/identity/mod.rs`
// compiles only the `resolve_namespace_verification` variant that returns
// `None` unconditionally, and `workers/ssr/src/identity/dev_fake_issuer.rs`
// (the module holding the issuer's signing key, JWKS-equivalent constants,
// and its two HTTP routes) is entirely absent — `#[cfg(feature =
// "dev_fake_issuer")]`-gated at its declaration in `identity/mod.rs`, so
// there is no code, not just no reachable code.
//
// Bundle size alone is suggestive, not proof (Handoff 054 §7) — this script
// proves absence directly from the compiled artifact: it builds both
// variants into scratch directories (never touching the shared
// `workers/ssr/build/` the other smokes depend on), then greps the
// feature-off `.wasm` for four strings that exist only inside the gated
// module or its gated call sites. Building the feature-on variant too and
// confirming the same markers ARE found there is what proves the search
// methodology itself isn't vacuously passing.
//
// Two markers deliberately excluded: `idns_local_fake` and
// `zinnias-ciao-dev-fake-client`. Both are also held in unconditional
// `const` bindings in `workers/ssr/src/handlers/identity/mod.rs` (namespace
// and client identifiers used to build a request that is never reachable in
// production, since `resolve_authorize_endpoint`/`resolve_namespace_verification`
// return `None` before either constant's value can have any effect) — so
// they appear as inert string data in BOTH builds and would falsely fail
// this gate. The four markers below are unique to the gated module/call
// sites and appear in neither constant's dead-in-production code path.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = resolve(dirname(fileURLToPath(import.meta.url)));
const repositoryRoot = resolve(scriptsDir, '..');

const MARKERS = [
  '/dev/identity/fake-issuer/authorize',
  '/dev/identity/fake-issuer/token',
  'https://fake-issuer.local.test',
  'dev-fake-key-1',
];

function buildVariant(outDir, extraArgs) {
  execFileSync(
    'worker-build',
    ['--release', '-d', outDir, 'workers/ssr', ...extraArgs],
    { cwd: repositoryRoot, stdio: 'pipe' },
  );
}

function findMarkers(buffer) {
  return MARKERS.filter((marker) => buffer.includes(Buffer.from(marker, 'utf8')));
}

async function main() {
  const offDir = await mkdtemp(join(tmpdir(), 'handoff054-build-off-'));
  const onDir = await mkdtemp(join(tmpdir(), 'handoff054-build-on-'));
  try {
    buildVariant(offDir, []);
    buildVariant(onDir, ['--features', 'dev_fake_issuer']);

    const offWasm = await readFile(join(offDir, 'index_bg.wasm'));
    const onWasm = await readFile(join(onDir, 'index_bg.wasm'));

    const foundInOff = findMarkers(offWasm);
    const foundInOn = findMarkers(onWasm);

    assert.deepEqual(
      foundInOff,
      [],
      `production-shape build (feature off) contains fake-issuer marker(s): ${foundInOff.join(', ')} — ` +
        'this is a complete authentication-bypass risk (Handoff 054 §11): a production build must not ' +
        'be able to resolve idns_local_fake verification requirements at all.',
    );

    assert.deepEqual(
      foundInOn.slice().sort(),
      MARKERS.slice().sort(),
      `smoke-shape build (feature on) is missing marker(s) that should be present: ` +
        `${MARKERS.filter((m) => !foundInOn.includes(m)).join(', ')} — the search methodology may be ` +
        'broken (e.g. wasm-opt renamed/dropped the strings), which would make the feature-off check above ' +
        'vacuous rather than a real proof.',
    );

    console.log(
      JSON.stringify({
        ok: true,
        markersChecked: MARKERS,
        productionBuild: { featureOff: true, bytes: offWasm.length, markersFound: foundInOff },
        smokeBuild: { featureOn: true, bytes: onWasm.length, markersFound: foundInOn },
      }),
    );
  } finally {
    await rm(offDir, { recursive: true, force: true });
    await rm(onDir, { recursive: true, force: true });
  }
}

await main();
