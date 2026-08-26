// Handoff 083 Part C: `smoke-all.mjs` runs each smoke via `spawnSync`, which
// waits only for the smoke script's own top-level process to exit — not for
// grandchildren (`wrangler dev`/`workerd`, headless Chromium) that were only
// signaled to terminate, never confirmed to have. Sending SIGTERM and
// immediately letting the script's own process end is exactly the "unawaited
// close" pattern the harness had already worked around once, independently,
// in `abuse-controls.mjs`'s own `stop()` helper (kill, then race the child's
// `exit` event against a timeout) — this generalizes that same pattern so
// every smoke script uses it, instead of firing a signal and moving on.
//
// A grandchild still holding a port or socket when the next smoke starts is
// a plausible, previously-undiagnosed contributor to the intermittent
// UND_ERR_SOCKET/timeout failures documented across Handoffs 064, 069, 076,
// and 078 — investigated, not confirmed as the sole cause; see this
// package's review request.
export async function killAndWait(child, { signal = 'SIGTERM', timeoutMs = 5000 } = {}) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  child.kill(signal);
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    new Promise((resolve) => setTimeout(resolve, timeoutMs)),
  ]);
}
