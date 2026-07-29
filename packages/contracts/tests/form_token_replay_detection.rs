// Guards against the form-token replay-detection defect recurring
// (remediation 2026-07-28, surfaced by RFC-050 Tooling Slice 5's local
// concurrency evidence). The defect: `form_token::consume` — a
// compatibility wrapper called by `codlet::consume_token` — collapsed
// `ConsumeResult::Proceed` and `ConsumeResult::Replay(None)` into the same
// `Option::None`. `result_ref` is written in exactly one place in the whole
// codebase (`me.rs`, display-name editing), so for every other purpose both
// branches returned `None`, and the 20 call sites checking `.is_some()`
// could never detect a replay. Fixed by removing the wrapper and migrating
// `codlet::consume_token` (and the one direct caller in `join.rs`) to
// return/match `ConsumeResult` directly.
//
// These are text-level pins, not a general static analyzer: they check that
// the specific removed function does not reappear, that the specific
// call-site pattern does not reappear, and that `codlet::consume_token`'s
// signature stays `ConsumeResult`-returning rather than collapsing back to
// an `Option`.

const CODLET_SRC: &str = include_str!("../../../workers/ssr/src/codlet.rs");
const FORM_TOKEN_SRC: &str = include_str!("../../../workers/ssr/src/form_token.rs");

const HANDLER_SOURCES: &[(&str, &str)] = &[
    (
        "templates.rs",
        include_str!("../../../workers/ssr/src/handlers/templates.rs"),
    ),
    (
        "admin/events/attendance.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
    ),
    (
        "auth.rs",
        include_str!("../../../workers/ssr/src/handlers/auth.rs"),
    ),
    (
        "calendar.rs",
        include_str!("../../../workers/ssr/src/handlers/calendar.rs"),
    ),
    (
        "communities.rs",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
    ),
    (
        "admin/help_signin.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
    ),
    (
        "admin/events/create.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
    ),
    (
        "admin/member_remove.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs"),
    ),
    (
        "admin/members.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
    ),
    (
        "export.rs",
        include_str!("../../../workers/ssr/src/handlers/export.rs"),
    ),
    (
        "admin/events/cancel.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
    ),
    (
        "admin/events/edit.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
    ),
    (
        "admin/events/notes.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
    ),
    (
        "admin/role_transfer.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs"),
    ),
    (
        "admin/events/occurrence.rs",
        include_str!("../../../workers/ssr/src/handlers/admin/events/occurrence.rs"),
    ),
    (
        "event.rs",
        include_str!("../../../workers/ssr/src/handlers/event.rs"),
    ),
    (
        "join.rs",
        include_str!("../../../workers/ssr/src/handlers/join.rs"),
    ),
    (
        "relink.rs",
        include_str!("../../../workers/ssr/src/handlers/relink.rs"),
    ),
    (
        "community_create.rs",
        include_str!("../../../workers/ssr/src/handlers/community_create.rs"),
    ),
    (
        "me.rs",
        include_str!("../../../workers/ssr/src/handlers/me.rs"),
    ),
];

#[test]
fn the_removed_option_collapsing_wrapper_does_not_reappear() {
    assert!(
        !FORM_TOKEN_SRC.contains("pub async fn consume("),
        "form_token.rs must not reintroduce a `consume` wrapper that collapses \
         ConsumeResult to an Option — only `consume_detailed` may exist, and every \
         caller must match on the full ConsumeResult",
    );
    assert!(
        FORM_TOKEN_SRC.contains("pub async fn consume_detailed("),
        "form_token.rs must still expose consume_detailed",
    );
}

#[test]
fn codlet_consume_token_returns_consume_result_not_option() {
    assert!(
        CODLET_SRC.contains("-> Result<ConsumeResult>"),
        "codlet::consume_token must return Result<ConsumeResult>, not a collapsed Option — \
         an Option-returning signature is exactly what let 20 call sites' `.is_some()` \
         checks silently fail to detect a replay",
    );
    assert!(
        !CODLET_SRC.contains("Result<Option<String>>"),
        "codlet::consume_token must not collapse back to Result<Option<String>>",
    );
}

#[test]
fn no_handler_calls_the_removed_option_returning_wrapper() {
    for (name, src) in HANDLER_SOURCES {
        assert!(
            !src.contains("form_token::consume("),
            "{name} must call form_token::consume_detailed, not the removed \
             Option-collapsing form_token::consume",
        );
    }
}

/// Whether a `//` line-comment marker appears before `index` on the same
/// line as `index`. A plain substring check, not a lexer: a `//` inside a
/// string literal would still count, but none of these call sites have one
/// nearby, and a false negative here (missing a real defect) is worse than
/// a rare false positive.
fn line_before_index_has_comment_marker(text: &str, index: usize) -> bool {
    let line_start = text[..index].rfind('\n').map(|i| i + 1).unwrap_or(0);
    text[line_start..index].contains("//")
}

/// Finds every `let <name> = ` binding immediately preceding one of the given
/// call substrings (e.g. `"consume_token("`), scanning backward from each
/// occurrence to the nearest `let ` that is not itself inside a `//`
/// comment (P2-N1 from the P-N1 review: an explanatory comment naming the
/// anti-pattern — exactly the documentation this whole gate family exists to
/// encourage — must never be mistaken for a real binding). Not a parser —
/// bounded, textual, and only meant to catch the one shape every real call
/// site uses.
fn bound_variable_names(src: &str, call_pattern: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_from = 0;
    while let Some(relative) = src[search_from..].find(call_pattern) {
        let call_index = search_from + relative;
        let before = &src[..call_index];
        let mut search_end = before.len();
        while let Some(let_index) = before[..search_end].rfind("let ") {
            if line_before_index_has_comment_marker(before, let_index) {
                search_end = let_index;
                continue;
            }
            let after_let = &before[let_index + 4..];
            let name: String = after_let
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
            break;
        }
        search_from = call_index + call_pattern.len();
    }
    names
}

#[test]
fn bound_variable_names_skips_commented_out_candidates() {
    // Pins the P2-N1 fix directly: a comment naming the anti-pattern must
    // not be mistaken for a real binding, and the real binding on the next
    // line must still be found.
    let src = "// let _ = crate::codlet::consume_token(a, b, c, d, e).await?;\n\
               let replay = crate::codlet::consume_token(a, b, c, d, e).await?;\n";
    assert_eq!(
        bound_variable_names(src, "consume_token("),
        vec!["replay".to_string()],
        "a commented-out candidate must be skipped and the real binding still found",
    );
}

#[test]
fn bound_variable_names_reports_nothing_when_the_only_candidate_is_commented() {
    // P3-N1 from the scanner-hardening review: no real binding exists at
    // all, only a commented-out one. Must report zero bindings cleanly —
    // never panic, never loop — not merely "avoid a false positive."
    let src = "// let replay = crate::codlet::consume_token(a, b, c, d, e).await?;\n\
               crate::codlet::consume_token(a, b, c, d, e).await?;\n";
    assert_eq!(
        bound_variable_names(src, "consume_token("),
        Vec::<String>::new(),
        "with only a commented-out candidate, no binding should be reported",
    );
}

#[test]
fn bound_variable_names_finds_a_real_binding_with_a_trailing_comment_after_it() {
    // P3-N1: the more likely real-world shape — a real binding followed by
    // a trailing `// TODO`-style comment on the same or a later line. A
    // comment *after* the binding must never suppress it; only a comment
    // that precedes the `let ` itself on its own line should.
    let src = "let replay = crate::codlet::consume_token(a, b, c, d, e).await?; // TODO: review\n";
    assert_eq!(
        bound_variable_names(src, "consume_token("),
        vec!["replay".to_string()],
        "a trailing comment after a real binding must not suppress it",
    );
}

#[test]
fn no_handler_discards_a_consume_result_with_let_underscore() {
    // P-N1 from the remediation follow-ups review: `let _ = ...consume_token(...)`
    // (or `consume_detailed`) discards the result entirely — invisible to the
    // `.is_some()` scanner below, and visually indistinguishable from the
    // defect just fixed at the other 21 call sites. A deliberate "replay is
    // acceptable here" decision (see auth.rs::post_logout) must say so by
    // matching the full `ConsumeResult` explicitly — even if every arm is a
    // no-op — never by silent discard.
    for (name, src) in HANDLER_SOURCES {
        let mut bound = bound_variable_names(src, "consume_token(");
        bound.extend(bound_variable_names(src, "consume_detailed("));
        assert!(
            !bound.iter().any(|bound_name| bound_name == "_"),
            "{name}: a form-token consume result is discarded with `let _ = ...` — \
             match the full ConsumeResult explicitly instead, even if every branch is a no-op",
        );
    }
}

#[test]
fn no_handler_classifies_any_consume_result_binding_with_is_some() {
    // R-N2 from the remediation review: broadened beyond the one variable
    // name (`replay`) every original call site happened to use, to any name
    // bound from a `consume_token(...)` or `consume_detailed(...)` call.
    // `ConsumeResult` has no `.is_some()` method, so this is the primary
    // control regardless — this pin catches a *new* Option-returning helper
    // wrapped around the binding before that helper itself could be pinned.
    for (name, src) in HANDLER_SOURCES {
        let mut bound = bound_variable_names(src, "consume_token(");
        bound.extend(bound_variable_names(src, "consume_detailed("));
        for var in bound {
            let pattern = format!("{var}.is_some()");
            assert!(
                !src.contains(pattern.as_str()),
                "{name}: `{var}`, bound from a form-token consume call, must not be \
                 classified with .is_some() — match on ConsumeResult explicitly instead",
            );
        }
    }
}
