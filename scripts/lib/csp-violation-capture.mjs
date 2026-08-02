// Handoff 038 §7.1: Content-Security-Policy violation capture for browser
// smokes. Before this, nothing in the smoke suite subscribed to console or
// violation events — `Runtime.enable` was already on, so `Runtime.consoleAPICalled`
// events already fired, but nothing listened. A CSP violation would log to
// console, drop the style silently, and pass every test the suite had.
//
// Two independent channels, since either catching a violation is enough but
// neither alone is guaranteed to see every shape of violation:
//   - the CDP Log domain, which surfaces a browser-generated CSP violation
//     message as a console-visible error entry;
//   - a page-side `securitypolicyviolation` event listener, injected via
//     `Page.addScriptToEvaluateOnNewDocument` so it is present before any
//     page script runs and survives every navigation, not just the first.
//
// Call `attachCspViolationCapture(cdp)` once per `Cdp` instance right after
// `Page.enable`/`Runtime.enable` (before any `navigate`), then
// `readCspViolations(cdp)` at the end of the scenario to collect from both
// channels combined.

const PAGE_SIDE_INIT_SCRIPT = `
  window.__cspViolations = window.__cspViolations || [];
  document.addEventListener('securitypolicyviolation', (e) => {
    window.__cspViolations.push({
      channel: 'securitypolicyviolation',
      violatedDirective: e.violatedDirective,
      blockedURI: e.blockedURI,
      sourceFile: e.sourceFile,
      lineNumber: e.lineNumber,
    });
  });
`;

export async function attachCspViolationCapture(cdp) {
  const logViolations = [];
  await cdp.send('Log.enable');
  cdp.on('Log.entryAdded', ({ entry }) => {
    const text = `${entry.source ?? ''} ${entry.text ?? ''}`;
    if (/content security policy|csp/i.test(text)) {
      logViolations.push({
        channel: 'Log.entryAdded',
        level: entry.level,
        source: entry.source,
        text: entry.text,
      });
    }
  });
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: PAGE_SIDE_INIT_SCRIPT });
  cdp.__cspLogViolations = logViolations;
}

export async function readCspViolations(cdp) {
  const result = await cdp.send('Runtime.evaluate', {
    expression: 'window.__cspViolations || []',
    returnByValue: true,
  });
  const pageSide = result.result?.value ?? [];
  const logSide = cdp.__cspLogViolations ?? [];
  return [...logSide, ...pageSide];
}
