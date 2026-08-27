// Handoff (RFC-037 replay-test restoration): a literal flash-code string in
// a script is the same defect one rename later — Handoff 037 converted
// `?flash=Note+removed` to `?flash=note_hidden` in the handler and never
// reached the two scripts that verify AD-4, which kept asserting the old
// prose value. This reads the Rust handler source directly and extracts the
// code the named function actually emits, so a future rename is caught by
// re-deriving, not by someone remembering to grep every script.
//
// No existing precedent for this in scripts/: `smoke-fixture-locale.mjs` and
// `language-preference.mjs` only reference Rust files in comments. Kept
// deliberately small and loud on failure — a wrong or missing match throws
// immediately rather than returning something that would silently mismatch
// downstream.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

// Matches a Rust fn declaration start, used only to find where the NEXT
// function begins so the search for `fnName`'s body does not run past it
// into a sibling function's own ?flash= literal.
const NEXT_FN_PATTERN = /\n\s*(?:pub(?:\([^)]*\))? )?(?:async )?fn \w+\(/;

/// Read `handlerRelPath` (relative to the repo root) and return the
/// `?flash=<code>` value the function named `fnName` emits. Throws if the
/// function cannot be found, or if it contains no `?flash=` literal at all —
/// both are loud failures, never a silent empty/undefined result.
export function flashCodeEmittedBy(handlerRelPath, fnName) {
  const absPath = join(repoRoot, handlerRelPath);
  const source = readFileSync(absPath, 'utf8');

  const fnMarker = `fn ${fnName}(`;
  const fnStart = source.indexOf(fnMarker);
  if (fnStart === -1) {
    throw new Error(
      `flashCodeEmittedBy: function "${fnName}" not found in ${handlerRelPath}`,
    );
  }

  const afterFnStart = source.slice(fnStart + fnMarker.length);
  const nextFnMatch = afterFnStart.match(NEXT_FN_PATTERN);
  const fnBody = nextFnMatch ? afterFnStart.slice(0, nextFnMatch.index) : afterFnStart;

  const flashMatch = fnBody.match(/\?flash=([a-z0-9_]+)/);
  if (!flashMatch) {
    throw new Error(
      `flashCodeEmittedBy: function "${fnName}" in ${handlerRelPath} emits no ?flash=<code> value — ` +
        'has it stopped setting a flash on success, or was the code moved to a helper this function calls?',
    );
  }
  return flashMatch[1];
}
