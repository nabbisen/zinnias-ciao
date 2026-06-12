# RFC 054 — Japanese UX Copy Review

**Status.** Proposed
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Follows RFC-049 (Japanese rendering). Required before public pilot with non-technical users.

## 1. Summary

All UI strings are now `JA_*` constants. This RFC is a human review of the Japanese copy for clarity, politeness, and absence of technical jargon — not a code change.

## 2. Scope

Review all 120 `JA_*` strings in `packages/contracts/src/i18n.rs` against the criteria:

- **Clarity:** would an IT-averse Japanese user understand what to do?
- **Politeness:** ですます style, appropriate register for general community use.
- **No technical jargon:** no セッション (session), トークン (token), 同期 (sync), HMAC, ICS in member-facing copy.
- **Action labels:** Going/No Go/Attended equivalents should match community norms; consider 参加する / 欠席する / 参加済み / 未回答.
- **Error messages:** should say what to do, not what failed technically.

## 3. Architect suggestions for review

| Current tendency | Recommended alternative |
|---|---|
| セッションが期限切れです | もう一度、招待コードを入力してください |
| 同期に失敗しました | 保存できませんでした。電波状況をご確認ください |
| 参加済み (admin tone) | 参加しました (softer, for member-facing history) |

## 4. Blocker

Requires a Japanese native speaker familiar with the target community type. Output: a list of string changes applied to i18n.rs.
