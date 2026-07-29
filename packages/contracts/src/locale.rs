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

#[cfg(test)]
mod tests {
    use super::Locale;

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
}
