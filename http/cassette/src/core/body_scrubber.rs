//! Apply `config.scrub_body_paths` to a request body before
//! hashing, so SDK-injected non-deterministic fields
//! (request_id, trace_id, timestamps) don't break exact-match
//! on replay.
//!
//! Path syntax: dot-separated object keys, e.g. `"request_id"`
//! or `"metadata.trace_id"`. Each path is interpreted as a
//! chain of object field lookups. Terminal segment's field is
//! removed (`Map::remove`). Non-existent paths are no-ops.
//!
//! ## Limitations
//!
//! - Array indexing is NOT supported — `"results.0.id"` won't
//!   descend into array index 0. If a body has non-deterministic
//!   fields inside an array, either scrub at a higher level
//!   (remove the whole array with `"results"`) or scrub the
//!   entire object containing it.
//! - Only JSON bodies are scrubbed. Non-JSON bodies (binary,
//!   form-encoded, plain text) hash as-is.

/// Apply each path to the raw body and return the resulting
/// bytes. If the body isn't valid JSON, returns the raw bytes
/// unchanged — scrubbing is best-effort and doesn't gate hashing.
pub(crate) fn scrub_body(raw: &[u8], paths: &[String]) -> Vec<u8> {
    if paths.is_empty() {
        return raw.to_vec();
    }
    let mut value: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return raw.to_vec(), // not JSON → no scrub
    };
    for path in paths {
        remove_path(&mut value, path);
    }
    // Re-serialize. `to_vec` preserves object key order as
    // serde_json emits it (sorted by insertion if the value is
    // from parsing; stable enough for hash reproducibility
    // across runs with the same scrub paths).
    serde_json::to_vec(&value).unwrap_or_else(|_| raw.to_vec())
}

/// Descend into `value` following `path` (dot-separated) and
/// remove the terminal field. No-op if the path doesn't exist
/// or any intermediate segment isn't an object.
fn remove_path(value: &mut serde_json::Value, path: &str) {
    let mut segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() {
        return;
    }
    let terminal = segments.pop().unwrap();
    let mut current = value;
    for seg in segments {
        match current {
            serde_json::Value::Object(map) => match map.get_mut(seg) {
                Some(next) => current = next,
                None => return, // path doesn't exist, no-op
            },
            _ => return, // not an object, bail
        }
    }
    if let serde_json::Value::Object(map) = current {
        map.remove(terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: scrub_body
    #[test]
    fn test_empty_paths_returns_raw_unchanged() {
        let body = br#"{"a": 1}"#;
        assert_eq!(scrub_body(body, &[]), body.to_vec());
    }

    /// @covers: scrub_body
    #[test]
    fn test_non_json_body_returns_raw_unchanged() {
        let body = b"not actual json";
        let paths = vec!["some.path".to_string()];
        assert_eq!(scrub_body(body, &paths), body.to_vec());
    }

    /// @covers: scrub_body
    #[test]
    fn test_removes_top_level_field() {
        let body = br#"{"request_id":"abc-123","payload":"data"}"#;
        let paths = vec!["request_id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        assert!(parsed.get("request_id").is_none());
        assert_eq!(parsed.get("payload").and_then(|v| v.as_str()), Some("data"));
    }

    /// @covers: scrub_body
    #[test]
    fn test_removes_nested_field_via_dot_path() {
        let body = br#"{"metadata":{"trace_id":"t-1","version":"v2"},"payload":"ok"}"#;
        let paths = vec!["metadata.trace_id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let meta = parsed.get("metadata").unwrap();
        assert!(meta.get("trace_id").is_none());
        assert_eq!(meta.get("version").and_then(|v| v.as_str()), Some("v2"));
    }

    /// @covers: scrub_body
    #[test]
    fn test_nonexistent_path_is_noop() {
        let body = br#"{"a":1}"#;
        let paths = vec!["nonexistent.field".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        assert_eq!(parsed.get("a").and_then(|v| v.as_i64()), Some(1));
    }

    /// @covers: scrub_body
    #[test]
    fn test_multiple_paths_all_removed() {
        let body = br#"{"request_id":"r","trace_id":"t","keep":"yes"}"#;
        let paths = vec!["request_id".to_string(), "trace_id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        assert!(parsed.get("request_id").is_none());
        assert!(parsed.get("trace_id").is_none());
        assert_eq!(parsed.get("keep").and_then(|v| v.as_str()), Some("yes"));
    }

    /// @covers: scrub_body
    #[test]
    fn test_path_into_array_is_noop_not_a_crash() {
        // "results.0.id" → "results" is an array, not an
        // object; the second segment fails the object check
        // and we bail without touching anything.
        let body = br#"{"results":[{"id":1}]}"#;
        let paths = vec!["results.0.id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        // Raw is preserved (or at least parses back to the same
        // thing — serde may reorder keys but the array is
        // intact).
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let arr = parsed.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(arr.len(), 1);
    }

    /// @covers: scrub_body
    #[test]
    fn test_removing_whole_subtree_by_ancestor_path() {
        // If someone wants to scrub everything under
        // `metadata.*`, they pass "metadata" as the path.
        let body = br#"{"metadata":{"x":1,"y":2},"payload":"ok"}"#;
        let paths = vec!["metadata".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        assert!(parsed.get("metadata").is_none());
        assert_eq!(parsed.get("payload").and_then(|v| v.as_str()), Some("ok"));
    }

    /// @covers: scrub_body
    #[test]
    fn test_scrubbed_bodies_hash_identically_when_scrubbed_fields_differ() {
        // The point of scrubbing: two bodies that differ only in
        // a scrubbed field should produce the same post-scrub
        // bytes (and thus the same hash).
        let a = br#"{"request_id":"first","payload":"same"}"#;
        let b = br#"{"request_id":"second","payload":"same"}"#;
        let paths = vec!["request_id".to_string()];
        let scrubbed_a = scrub_body(a, &paths);
        let scrubbed_b = scrub_body(b, &paths);
        // After scrubbing the request_id, both collapse to
        // the same remaining object.
        let parsed_a: serde_json::Value = serde_json::from_slice(&scrubbed_a).unwrap();
        let parsed_b: serde_json::Value = serde_json::from_slice(&scrubbed_b).unwrap();
        assert_eq!(parsed_a, parsed_b);
    }
}
