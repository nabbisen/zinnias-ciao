//! Member-facing UI locale (RFC-072).
//!
//! Stored and transmitted as the stable code (`"ja"`/`"en"`), never a
//! display label. Parsing is a closed allow-list: anything outside the
//! reviewed set is a rejection, not a silent default — callers decide the
//! fallback (Japanese, per RFC-072's resolution order), this type never
//! guesses one on their behalf.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Ja,
    En,
}

impl Locale {
    /// Parses a stable locale code. Only the two reviewed codes are
    /// accepted; an empty string, wrong case (`"EN"`), a BCP-47-shaped tag
    /// (`"ja-JP"`, `"en-US"`), a near-miss (`"jp"`), or a value carrying
    /// whitespace are all rejected outright, never coerced.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ja" => Some(Self::Ja),
            "en" => Some(Self::En),
            _ => None,
        }
    }

    /// The stable code to store or transmit — never a display label.
    pub fn code(self) -> &'static str {
        match self {
            Self::Ja => "ja",
            Self::En => "en",
        }
    }
}

impl Default for Locale {
    /// Japanese is the fallback when no membership preference is set, and
    /// the safe fallback when a stored value is outside the allow-list
    /// (RFC-072 §Locale Resolution; a bad stored value must fail safe, never
    /// panic).
    fn default() -> Self {
        Self::Ja
    }
}

/// RFC-083 §8.1 rung 2 / Handoff 075 §4.2: the header is attacker-controlled
/// and unbounded — a real browser sends well under ten entries (Chrome and
/// Firefox both default to one to three), so this is generous headroom
/// against a pathological header, not a tight fit to real traffic. Entries
/// beyond this bound are ignored outright, not allocated for.
const ACCEPT_LANGUAGE_MAX_ENTRIES: usize = 10;

/// Negotiates an `Accept-Language` header into a supported [`Locale`]
/// (RFC-083 §8.1 rung 2, Handoff 075). **Not** [`Locale::parse`]: that is
/// the strict allow-list for a *stored* value, where an unexpected shape
/// must fail closed; this is a lenient negotiator for a *header*, where
/// rejecting almost every real-world tag (`en-US`, `ja-JP`, `en-GB`, ...)
/// is the ordinary case, not a fault. The two must never merge — loosening
/// `Locale::parse` to satisfy this caller would weaken the control that
/// protects stored preferences from a malformed database value.
///
/// Algorithm:
/// - Split on `,`, examine at most [`ACCEPT_LANGUAGE_MAX_ENTRIES`] entries.
/// - Each entry is `<tag>` or `<tag>;q=<weight>` (any further `;param=...`
///   segment is ignored, not rejected). A missing weight is `1.0`. A
///   weight that fails to parse as a finite number in `[0, 1]` makes the
///   *entry* ignored entirely — never defaulted to a guessed weight.
/// - `q=0` (or any non-positive weight) means "explicitly not acceptable"
///   and that entry is never selected.
/// - The surviving entries are ordered by descending weight; `sort_by` is
///   stable, so entries of equal weight keep their original header order.
/// - For each entry in that order, the primary subtag (before the first
///   `-`), lowercased, is tried against [`Locale::parse`]. The first
///   match wins.
/// - No match anywhere returns `None`, and the caller falls through to
///   the Japanese floor (RFC-083 §8.1 rung 3) — this function never
///   guesses a default itself, matching [`Locale::parse`]'s own contract.
pub fn negotiate_accept_language(header: &str) -> Option<Locale> {
    let mut entries: Vec<(f32, &str)> = header
        .split(',')
        .take(ACCEPT_LANGUAGE_MAX_ENTRIES)
        .filter_map(|raw| {
            let mut parts = raw.split(';').map(str::trim);
            let tag = parts.next()?;
            if tag.is_empty() {
                return None;
            }
            let mut weight = 1.0f32;
            for param in parts {
                if let Some(q) = param.strip_prefix("q=") {
                    match q.trim().parse::<f32>() {
                        Ok(w) if w.is_finite() && (0.0..=1.0).contains(&w) => weight = w,
                        // Malformed or out-of-range weight: ignore this
                        // whole entry rather than inventing a weight for it.
                        _ => return None,
                    }
                }
            }
            if weight <= 0.0 {
                // q=0 (or a rounding-to-nonpositive weight): explicitly
                // not acceptable.
                return None;
            }
            Some((weight, tag))
        })
        .collect();

    entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    entries.into_iter().find_map(|(_, tag)| {
        let primary = tag.split('-').next().unwrap_or(tag).to_ascii_lowercase();
        Locale::parse(&primary)
    })
}

#[cfg(test)]
mod tests {
    use super::{Locale, negotiate_accept_language};

    #[test]
    fn parse_accepts_exactly_the_reviewed_codes() {
        assert_eq!(Locale::parse("ja"), Some(Locale::Ja));
        assert_eq!(Locale::parse("en"), Some(Locale::En));
    }

    #[test]
    fn parse_rejects_everything_else() {
        for bad in [
            "", "EN", "JA", "ja-JP", "en-US", "jp", "en ", " en", "ja\n", "english", "japanese",
            "ja,en",
        ] {
            assert_eq!(Locale::parse(bad), None, "{bad:?} must be rejected");
        }
    }

    #[test]
    fn code_round_trips_through_parse() {
        assert_eq!(Locale::parse(Locale::Ja.code()), Some(Locale::Ja));
        assert_eq!(Locale::parse(Locale::En.code()), Some(Locale::En));
    }

    #[test]
    fn default_is_japanese() {
        assert_eq!(Locale::default(), Locale::Ja);
    }

    // ── negotiate_accept_language (RFC-083 §8.1 rung 2, Handoff 075) ─────

    #[test]
    fn negotiates_a_plain_ja() {
        assert_eq!(negotiate_accept_language("ja"), Some(Locale::Ja));
    }

    #[test]
    fn negotiates_a_regional_tag_by_its_primary_subtag() {
        assert_eq!(negotiate_accept_language("en-US"), Some(Locale::En));
    }

    #[test]
    fn q_ordering_picks_the_highest_weight_even_when_not_first_in_header() {
        // fr (unsupported, implicit q=1.0), then en at q=0.9, then ja at
        // q=0.8 — en must win despite ja appearing nowhere near first and
        // fr appearing before either.
        assert_eq!(
            negotiate_accept_language("fr,ja;q=0.8,en;q=0.9"),
            Some(Locale::En)
        );
    }

    #[test]
    fn q_zero_on_an_otherwise_matching_tag_is_never_selected() {
        // ja is explicitly refused (q=0); en is the only acceptable match.
        assert_eq!(
            negotiate_accept_language("ja;q=0,en;q=0.5"),
            Some(Locale::En)
        );
        // q=0 on the only tag present: nothing is acceptable.
        assert_eq!(negotiate_accept_language("ja;q=0"), None);
    }

    #[test]
    fn a_malformed_q_discards_only_its_own_entry_not_the_whole_header() {
        // "q=abc" doesn't parse as a weight — that entry is ignored
        // entirely (not defaulted to q=1.0), but en later in the header
        // still matches.
        assert_eq!(
            negotiate_accept_language("ja;q=abc,en;q=0.5"),
            Some(Locale::En)
        );
        // Also malformed: a weight outside [0, 1], and non-finite weights
        // (Rust's f32 parser accepts "nan"/"inf" as valid floats, so the
        // range check — not just parse success — is what rejects them).
        assert_eq!(negotiate_accept_language("ja;q=1.5"), None);
        assert_eq!(negotiate_accept_language("ja;q=nan"), None);
        assert_eq!(negotiate_accept_language("ja;q=inf"), None);
    }

    #[test]
    fn an_unsupported_language_only_negotiates_to_none() {
        assert_eq!(negotiate_accept_language("fr-FR,de;q=0.8"), None);
    }

    #[test]
    fn empty_header_negotiates_to_none() {
        assert_eq!(negotiate_accept_language(""), None);
    }

    #[test]
    fn entries_beyond_the_bound_are_never_examined() {
        // Handoff 075 §4.2: exactly ACCEPT_LANGUAGE_MAX_ENTRIES (10)
        // unsupported entries, then the 11th is the only one that would
        // ever match — it must be ignored, proving the bound is real
        // rather than merely documented.
        let mut header = "fr;q=0.99".to_string();
        for _ in 0..9 {
            header.push_str(",fr;q=0.99");
        }
        header.push_str(",ja;q=0.01");
        assert_eq!(
            negotiate_accept_language(&header),
            None,
            "the 11th entry (a matching ja) must be beyond the bound and never examined"
        );
    }

    #[test]
    fn a_missing_weight_defaults_to_one_not_zero() {
        // ja has no explicit q (defaults to 1.0); en explicitly has a
        // lower weight — ja must win.
        assert_eq!(negotiate_accept_language("en;q=0.5,ja"), Some(Locale::Ja));
    }

    #[test]
    fn equal_weights_keep_header_order() {
        // Both explicit q=0.8: the first one in header order (en) wins,
        // proving the sort is stable rather than incidentally so.
        assert_eq!(
            negotiate_accept_language("en;q=0.8,ja;q=0.8"),
            Some(Locale::En)
        );
        assert_eq!(
            negotiate_accept_language("ja;q=0.8,en;q=0.8"),
            Some(Locale::Ja)
        );
    }
}
