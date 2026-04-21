//! Cached response entry + associated parsing helpers for
//! RFC 7234 `Vary`, `ETag`, and RFC 5861 `stale-while-revalidate`.
//!
//! This module is deliberately dependency-light (no `reqwest`
//! response types on the parsing helpers) so unit tests can
//! exercise the parsers without constructing full responses.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One cached response entry — the minimal shape needed to
/// reconstruct a `reqwest::Response`, plus the metadata needed
/// for RFC 7234 `Vary` matching, RFC 7234 `ETag`-based
/// revalidation, and RFC 5861 `stale-while-revalidate`.
#[derive(Clone, Debug)]
pub(crate) struct CachedEntry {
    /// HTTP status code captured at store time.
    pub(crate) status: u16,
    /// Response headers captured at store time (snake-case keys
    /// as returned by `HeaderName::as_str`).
    pub(crate) headers: BTreeMap<String, String>,
    /// Response body bytes, shared via `Arc` so we can hand out
    /// cheap clones to the reconstruction path.
    pub(crate) body: Arc<Vec<u8>>,
    /// Freshness deadline. Serve as-fresh while `Instant::now() <
    /// expires_at`. Serve as-stale-but-reusable during the SWR
    /// window. Must revalidate beyond `expires_at + swr`.
    pub(crate) expires_at: Instant,
    /// `ETag` header from the response, if any. Used to send
    /// `If-None-Match` on revalidation.
    pub(crate) etag: Option<String>,
    /// For each name in the response's `Vary` header, the value
    /// of that REQUEST header at store time. Sorted by header
    /// name (lowercase) so equality checks are order-independent.
    /// Empty when the response had no `Vary`.
    pub(crate) vary_headers: Vec<(String, String)>,
    /// RFC 5861 `stale-while-revalidate` window. `None` when the
    /// upstream did not emit the directive.
    pub(crate) stale_while_revalidate: Option<Duration>,
}

/// What the response's `Vary` header means for caching.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum VaryDirective {
    /// Response had no `Vary` header — no per-header keying.
    None,
    /// `Vary: *` — response varies on things we can't observe.
    /// RFC 7234 says: do not cache.
    Star,
    /// Response varies on specific request headers. Names are
    /// normalized to lowercase, sorted, and de-duplicated.
    Names(Vec<String>),
}

/// Parse a response's `Vary` header value.
///
/// `Vary` is a comma-separated list of request-header names, or
/// the single wildcard `*`. `vary_raw` is the raw header value
/// (possibly multiple comma-joined values). Whitespace-tolerant.
///
/// - Empty / all-whitespace → [`VaryDirective::None`].
/// - Any token equal to `*` (even alongside other tokens) →
///   [`VaryDirective::Star`] (conservative — if ANY variant axis
///   is uncacheable, the whole response is).
/// - Else normalizes to lowercase, deduplicates, sorts, and
///   returns [`VaryDirective::Names`].
pub(crate) fn parse_vary(vary_raw: Option<&str>) -> VaryDirective {
    let raw = match vary_raw {
        Some(s) if !s.trim().is_empty() => s,
        _ => return VaryDirective::None,
    };
    let mut names: Vec<String> = Vec::new();
    for part in raw.split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        if token == "*" {
            return VaryDirective::Star;
        }
        names.push(token.to_ascii_lowercase());
    }
    if names.is_empty() {
        return VaryDirective::None;
    }
    names.sort();
    names.dedup();
    VaryDirective::Names(names)
}

/// Extract `max-age=N` from a lowercased Cache-Control value.
/// Malformed numeric values are ignored (returns `None`), NOT
/// propagated as errors — RFC 7234 says unparseable directives
/// are treated as absent.
pub(crate) fn extract_max_age(cc: &str) -> Option<u64> {
    for part in cc.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("max-age=") {
            return v.parse().ok();
        }
    }
    None
}

/// Extract `stale-while-revalidate=N` (RFC 5861) from a
/// lowercased Cache-Control value.
///
/// Returns `None` when the directive is absent, when N is zero,
/// or when N is unparseable. Zero-is-none matches the spec's
/// "no SWR window" semantics — callers can treat `Some` as
/// "SWR applies" without a separate bool.
pub(crate) fn extract_stale_while_revalidate(cc: &str) -> Option<Duration> {
    for part in cc.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("stale-while-revalidate=") {
            let secs: u64 = v.parse().ok()?;
            if secs == 0 {
                return None;
            }
            return Some(Duration::from_secs(secs));
        }
    }
    None
}

/// Is a cached entry a match for this request, per the entry's
/// captured `Vary` headers?
///
/// Matches when EVERY `(name, expected_value)` pair recorded on
/// the entry at store time matches the corresponding value on
/// the incoming request. Missing request header is treated as
/// the empty string — the entry stores `""` for
/// missing-at-store-time, so missing-at-lookup-time on BOTH
/// sides matches.
pub(crate) fn entry_matches_vary(
    entry: &CachedEntry,
    req_header_values: &dyn Fn(&str) -> String,
) -> bool {
    for (name, expected) in &entry.vary_headers {
        let actual = req_header_values(name);
        if actual != *expected {
            return false;
        }
    }
    true
}

/// Should we revalidate this entry (send `If-None-Match`) instead
/// of serving it directly?
///
/// - Fresh (`now < expires_at`) → false (serve as-is, no revalidation).
/// - Within SWR window (`now < expires_at + swr`) → false (serve
///   stale; background refresh handled separately).
/// - Beyond SWR window → true (must revalidate or refetch).
///
/// When the entry has no SWR, stale means immediately
/// revalidate-eligible.
pub(crate) fn should_revalidate(entry: &CachedEntry, now: Instant) -> bool {
    if now < entry.expires_at {
        return false;
    }
    match entry.stale_while_revalidate {
        None => true,
        Some(swr) => now >= entry.expires_at + swr,
    }
}

/// Is this entry within its stale-while-revalidate window?
/// True only when stale AND SWR is set AND SWR window not
/// exceeded.
pub(crate) fn in_swr_window(entry: &CachedEntry, now: Instant) -> bool {
    if now < entry.expires_at {
        return false; // fresh, not stale
    }
    match entry.stale_while_revalidate {
        None => false,
        Some(swr) => now < entry.expires_at + swr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(expires_at: Instant, swr: Option<Duration>) -> CachedEntry {
        CachedEntry {
            status: 200,
            headers: BTreeMap::new(),
            body: Arc::new(Vec::new()),
            expires_at,
            etag: None,
            vary_headers: Vec::new(),
            stale_while_revalidate: swr,
        }
    }

    /// @covers: parse_vary
    #[test]
    fn test_parse_vary_absent_returns_none() {
        assert_eq!(parse_vary(None), VaryDirective::None);
        assert_eq!(parse_vary(Some("")), VaryDirective::None);
        assert_eq!(parse_vary(Some("   ")), VaryDirective::None);
    }

    /// @covers: parse_vary
    #[test]
    fn test_parse_vary_star_alone_returns_star() {
        assert_eq!(parse_vary(Some("*")), VaryDirective::Star);
        assert_eq!(parse_vary(Some("  *  ")), VaryDirective::Star);
    }

    /// @covers: parse_vary
    #[test]
    fn test_parse_vary_star_among_names_returns_star() {
        // Conservative: any star axis makes the whole thing uncacheable.
        assert_eq!(
            parse_vary(Some("Accept-Encoding, *")),
            VaryDirective::Star
        );
    }

    /// @covers: parse_vary
    #[test]
    fn test_parse_vary_names_normalized_sorted_deduped() {
        assert_eq!(
            parse_vary(Some("Accept-Language, Accept-Encoding, accept-language")),
            VaryDirective::Names(vec![
                "accept-encoding".to_string(),
                "accept-language".to_string(),
            ])
        );
    }

    /// @covers: parse_vary
    #[test]
    fn test_parse_vary_empty_tokens_ignored() {
        assert_eq!(
            parse_vary(Some("Accept-Encoding,, , Accept-Language")),
            VaryDirective::Names(vec![
                "accept-encoding".to_string(),
                "accept-language".to_string(),
            ])
        );
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_from_simple_directive() {
        assert_eq!(extract_max_age("max-age=600"), Some(600));
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_from_mixed_directives() {
        assert_eq!(
            extract_max_age("public, max-age=300, must-revalidate"),
            Some(300)
        );
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_absent_returns_none() {
        assert!(extract_max_age("no-cache").is_none());
        assert!(extract_max_age("").is_none());
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_malformed_returns_none() {
        // Non-numeric value — per RFC 7234 we must not panic.
        assert_eq!(extract_max_age("max-age=abc"), None);
        assert_eq!(extract_max_age("max-age="), None);
    }

    /// @covers: extract_stale_while_revalidate
    #[test]
    fn test_extract_swr_parses_nonzero() {
        assert_eq!(
            extract_stale_while_revalidate("max-age=60, stale-while-revalidate=300"),
            Some(Duration::from_secs(300))
        );
    }

    /// @covers: extract_stale_while_revalidate
    #[test]
    fn test_extract_swr_zero_is_none() {
        // RFC: "stale-while-revalidate=0" means no SWR window.
        assert_eq!(
            extract_stale_while_revalidate("max-age=60, stale-while-revalidate=0"),
            None
        );
    }

    /// @covers: extract_stale_while_revalidate
    #[test]
    fn test_extract_swr_absent_is_none() {
        assert_eq!(extract_stale_while_revalidate("max-age=60"), None);
        assert_eq!(extract_stale_while_revalidate("public"), None);
        assert_eq!(extract_stale_while_revalidate(""), None);
    }

    /// @covers: extract_stale_while_revalidate
    #[test]
    fn test_extract_swr_malformed_returns_none() {
        // Non-numeric tail — log-and-ignore, never panic.
        assert_eq!(
            extract_stale_while_revalidate("stale-while-revalidate=abc"),
            None
        );
    }

    /// @covers: entry_matches_vary
    #[test]
    fn test_entry_matches_vary_no_vary_always_matches() {
        let entry = sample_entry(
            Instant::now() + Duration::from_secs(60),
            None,
        );
        // No vary_headers recorded → any request matches.
        let req = |_: &str| "whatever".to_string();
        assert!(entry_matches_vary(&entry, &req));
    }

    /// @covers: entry_matches_vary
    #[test]
    fn test_entry_matches_vary_exact_match() {
        let mut entry = sample_entry(
            Instant::now() + Duration::from_secs(60),
            None,
        );
        entry.vary_headers = vec![
            ("accept-encoding".to_string(), "gzip".to_string()),
            ("accept-language".to_string(), "en".to_string()),
        ];
        let req = |name: &str| match name {
            "accept-encoding" => "gzip".to_string(),
            "accept-language" => "en".to_string(),
            _ => String::new(),
        };
        assert!(entry_matches_vary(&entry, &req));
    }

    /// @covers: entry_matches_vary
    #[test]
    fn test_entry_matches_vary_mismatch_rejects() {
        let mut entry = sample_entry(
            Instant::now() + Duration::from_secs(60),
            None,
        );
        entry.vary_headers =
            vec![("accept-encoding".to_string(), "gzip".to_string())];
        let req = |_: &str| "br".to_string();
        assert!(!entry_matches_vary(&entry, &req));
    }

    /// @covers: should_revalidate
    #[test]
    fn test_should_revalidate_fresh_returns_false() {
        let now = Instant::now();
        let entry = sample_entry(now + Duration::from_secs(60), None);
        assert!(!should_revalidate(&entry, now));
    }

    /// @covers: should_revalidate
    #[test]
    fn test_should_revalidate_stale_no_swr_returns_true() {
        let now = Instant::now();
        // Make the entry definitively stale: expires_at is in the past.
        let entry =
            sample_entry(now - Duration::from_secs(1), None);
        assert!(should_revalidate(&entry, now));
    }

    /// @covers: should_revalidate
    #[test]
    fn test_should_revalidate_stale_within_swr_returns_false() {
        let now = Instant::now();
        // expired 1s ago, SWR window = 60s → still reusable.
        let entry = sample_entry(
            now - Duration::from_secs(1),
            Some(Duration::from_secs(60)),
        );
        assert!(!should_revalidate(&entry, now));
    }

    /// @covers: should_revalidate
    #[test]
    fn test_should_revalidate_stale_beyond_swr_returns_true() {
        let now = Instant::now();
        // expired 120s ago, SWR = 60s → past the SWR window.
        let entry = sample_entry(
            now - Duration::from_secs(120),
            Some(Duration::from_secs(60)),
        );
        assert!(should_revalidate(&entry, now));
    }

    /// @covers: in_swr_window
    #[test]
    fn test_cached_entry_in_swr_window_is_still_reusable() {
        let now = Instant::now();
        // Stale (expired 5s ago) but within 30s SWR window.
        let entry = sample_entry(
            now - Duration::from_secs(5),
            Some(Duration::from_secs(30)),
        );
        assert!(in_swr_window(&entry, now));
    }

    /// @covers: in_swr_window
    #[test]
    fn test_in_swr_window_fresh_returns_false() {
        // Fresh entries are not "in SWR window" — they're just fresh.
        let now = Instant::now();
        let entry = sample_entry(
            now + Duration::from_secs(60),
            Some(Duration::from_secs(30)),
        );
        assert!(!in_swr_window(&entry, now));
    }

    /// @covers: in_swr_window
    #[test]
    fn test_in_swr_window_no_swr_returns_false() {
        let now = Instant::now();
        let entry = sample_entry(now - Duration::from_secs(1), None);
        assert!(!in_swr_window(&entry, now));
    }
}
