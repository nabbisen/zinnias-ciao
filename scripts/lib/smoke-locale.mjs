// RFC-083 §8.1 rung 2 (Handoff 076 — review finding F1 of Handoff 075's
// `.git-exclude/reviewed/zinnias-ciao-main-2026-08-16-rfc083-slice-d2a-anonymous-routes-review.md`):
// before rung 2 existed, the anonymous/redemption routes ignored
// `Accept-Language` entirely, so the sandboxed Chromium's ambient default
// (derived from the developer machine's `LANG`) was harmless. It is not any
// more — every smoke that opens a page must send a fixed value, or the
// suite's result depends on state it does not control.
//
// Pinned to Japanese: that is the language every pre-existing smoke
// assertion on an anonymous route was written to exercise, so pinning it
// restores their original intent rather than changing what they check.
// `--lang=<code>` was tried first and does not affect the header on this
// headless Chromium build (confirmed empirically); `Network.setExtraHTTPHeaders`
// does and is already how every smoke sets its session `Cookie`, so the two
// are merged into one call per page rather than added as a second one.
export const SMOKE_ACCEPT_LANGUAGE = 'ja';
