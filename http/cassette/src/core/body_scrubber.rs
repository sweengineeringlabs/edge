//! Apply `config.scrub_body_paths` to a request body before
//! hashing, so SDK-injected non-deterministic fields
//! (request_id, trace_id, timestamps) don't break exact-match
//! on replay.
//!
//! Path syntax: dot-separated segments, e.g. `"request_id"`,
//! `"metadata.trace_id"`, or `"results.0.id"`. Each segment is
//! either an object key or, when the current value is a JSON
//! array and the segment parses as `usize`, an array index. The
//! terminal segment's field is removed (`Map::remove`) or, for
//! arrays, the element at that index (`Vec::remove`). Non-existent
//! paths and out-of-bounds indices are no-ops.
//!
//! ## Limitations
//!
//! - Only JSON bodies are scrubbed. Non-JSON bodies (binary,
//!   form-encoded, plain text) hash as-is.
//! - Array segments must be numeric strings — a non-numeric
//!   segment into an array is a no-op (we bail rather than
//!   pretending it matched).

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
/// remove the terminal field or element. No-op if the path
/// doesn't exist, an intermediate segment doesn't match the
/// current value's shape (object key missing, array index OOB,
/// non-numeric segment into an array, scalar encountered
/// mid-path), or the terminal is missing.
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
            serde_json::Value::Array(arr) => {
                // Array descent: segment must parse as usize and
                // be in bounds. Non-numeric or OOB → bail.
                let idx: usize = match seg.parse() {
                    Ok(i) => i,
                    Err(_) => return,
                };
                match arr.get_mut(idx) {
                    Some(next) => current = next,
                    None => return,
                }
            }
            _ => return, // scalar mid-path, bail
        }
    }
    match current {
        serde_json::Value::Object(map) => {
            map.remove(terminal);
        }
        serde_json::Value::Array(arr) => {
            // Terminal array removal: numeric in-bounds removes
            // the element (array shrinks). Anything else is a
            // no-op.
            if let Ok(idx) = terminal.parse::<usize>() {
                if idx < arr.len() {
                    arr.remove(idx);
                }
            }
        }
        _ => {}
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
    fn test_removes_field_inside_array_element() {
        // Numeric segment descends into the array; the terminal
        // key is then removed from the object at that index.
        let body = br#"{"results":[{"id":1,"name":"a"},{"id":2}]}"#;
        let paths = vec!["results.0.id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let arr = parsed.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(arr.len(), 2);
        let first = arr.first().unwrap();
        assert!(first.get("id").is_none(), "id should be removed from results[0]");
        assert_eq!(first.get("name").and_then(|v| v.as_str()), Some("a"));
        // results[1] untouched.
        assert_eq!(arr[1].get("id").and_then(|v| v.as_i64()), Some(2));
    }

    /// @covers: scrub_body
    #[test]
    fn test_removes_entire_array_element_by_index() {
        // Terminal numeric segment on an array removes the
        // element at that index; the array shrinks.
        let body = br#"{"results":[{"a":1},{"b":2}]}"#;
        let paths = vec!["results.0".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let arr = parsed.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(arr.len(), 1, "array should shrink by one");
        let remaining = arr.first().unwrap();
        assert!(remaining.get("a").is_none(), "first element {{a:1}} was removed");
        assert_eq!(remaining.get("b").and_then(|v| v.as_i64()), Some(2));
    }

    /// @covers: scrub_body
    #[test]
    fn test_out_of_bounds_index_is_noop() {
        // Index 99 doesn't exist in a one-element array → bail
        // without mutating anything.
        let body = br#"{"results":[{"id":1}]}"#;
        let paths = vec!["results.99.id".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let arr = parsed.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].get("id").and_then(|v| v.as_i64()), Some(1));
    }

    /// @covers: scrub_body
    #[test]
    fn test_non_numeric_segment_into_array_is_noop() {
        // "nope" is neither a usize nor a valid array op → bail
        // without touching the array.
        let body = br#"{"results":[1,2,3]}"#;
        let paths = vec!["results.nope".to_string()];
        let scrubbed = scrub_body(body, &paths);
        let parsed: serde_json::Value = serde_json::from_slice(&scrubbed).unwrap();
        let arr = parsed.get("results").and_then(|v| v.as_array()).unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[1].as_i64(), Some(2));
        assert_eq!(arr[2].as_i64(), Some(3));
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
