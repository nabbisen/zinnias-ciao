pub struct CardDay<'a> {
    pub starts_at_utc: &'a str,
    pub ends_at_utc: &'a str,
    // Handoff 035 (dead-code sweep) finding: no live formatter reads this
    // field back out (render/time.rs's format_day_time_tz[_localized] only
    // read starts_at_utc/ends_at_utc) — pre-existing, not caused by this
    // sweep's deletions, and out of scope to remove since CardDay itself is
    // explicitly authorized to stay live and unmodified beyond this.
    #[allow(dead_code)]
    pub day_date: &'a str,
}
