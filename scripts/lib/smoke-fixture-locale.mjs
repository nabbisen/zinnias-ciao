// Handoff 078: ROADMAP.md's English-default flip (RFC-085 reduced it to one
// line, `Locale::PRODUCT_DEFAULT` in `packages/contracts/src/locale.rs`) has
// a large blast radius the smoke suite creates for itself, not the product.
//
// No fixture sets `ui_language`, and no application insert path backfills
// it either (migration 0011 was additive, no backfill) — so every seeded
// membership is `NULL`, and every signed-in page currently resolves through
// `Locale::PRODUCT_DEFAULT` (Japanese today). Flipping that one line would
// flip sixteen-plus smoke scripts' Japanese assertions with it, for a
// reason that has nothing to do with the product: an ambient default the
// suite never pinned. Same disease `smoke-locale.mjs` (Handoff 076) cured
// for `Accept-Language` — pin the input the assertions depend on, don't
// rewrite the assertions.
//
// `WHERE ui_language IS NULL` makes this idempotent and non-destructive:
// safe to call after every step that can create a membership (fixture
// seeding, and application-created ones mid-scenario — `/join` redemption,
// the identity callback's join outcome), and it will never overwrite a
// value a smoke set deliberately. `language-preference.mjs` and
// `rfc075-calendar-css-migration.mjs` both switch `ui_language` as part of
// what they test and must keep working unchanged — this statement can run
// before, after, or interleaved with either and never touch a row they
// have already written a real value into.
export const PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL =
  "UPDATE community_memberships SET ui_language = 'ja' WHERE ui_language IS NULL";
