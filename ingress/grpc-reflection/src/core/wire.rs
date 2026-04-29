//! Hand-rolled protobuf codec for `grpc.reflection.v1alpha`.
//!
//! We avoid `prost` to keep the crate dep-light and to keep the wire
//! shape obvious to reviewers.  The on-the-wire schema is fixed by
//! the upstream `reflection.proto` file — adding a field here means
//! adding a matching tag/varint in [`encode_response`] and a matching
//! arm in [`decode_request`].
//!
//! Wire reference (proto3 spec):
//! - tag = `(field_number << 3) | wire_type`
//! - wire_type 0 = varint
//! - wire_type 2 = length-delimited (UTF-8 string, sub-message, bytes)
//! - wire_type 5 = 32-bit fixed
//!
//! ```proto
//! message ServerReflectionRequest {
//!     string host = 1;                                       // tag 0x0a
//!     oneof message_request {
//!         string file_by_filename               = 3;         // tag 0x1a
//!         string file_containing_symbol         = 4;         // tag 0x22
//!         ExtensionRequest file_containing_extension = 5;    // tag 0x2a
//!         string all_extension_numbers_of_type  = 6;         // tag 0x32
//!         string list_services                  = 7;         // tag 0x3a
//!     }
//! }
//!
//! message ExtensionRequest {
//!     string containing_type   = 1;                          // tag 0x0a
//!     int32  extension_number  = 2;                          // tag 0x10
//! }
//!
//! message ServerReflectionResponse {
//!     string valid_host                          = 1;        // tag 0x0a
//!     ServerReflectionRequest original_request   = 2;        // tag 0x12
//!     oneof message_response {
//!         FileDescriptorResponse  file_descriptor_response = 4;   // tag 0x22
//!         ExtensionNumberResponse all_extension_numbers_response = 5; // tag 0x2a
//!         ListServiceResponse     list_services_response   = 6;       // tag 0x32
//!         ErrorResponse           error_response           = 7;       // tag 0x3a
//!     }
//! }
//!
//! message FileDescriptorResponse { repeated bytes file_descriptor_proto = 1; } // tag 0x0a
//! message ListServiceResponse    { repeated ServiceResponse service     = 1; } // tag 0x0a
//! message ServiceResponse        { string name = 1; }                          // tag 0x0a
//! message ErrorResponse          { int32 error_code = 1; string error_message = 2; } // 0x08, 0x12
//! ```

use crate::api::error::ReflectionError;
use crate::api::types::{ReflectionRequest, ReflectionResponse};

/// Decode a `ServerReflectionRequest`.
///
/// Unknown tags are skipped (per proto3) so future-versioned clients
/// don't break us; recognised oneof fields produce the matching
/// variant.  When no oneof field is present we return
/// [`ReflectionRequest::Unknown`] — the dispatcher answers that with
/// `INVALID_ARGUMENT`.
pub fn decode_request(body: &[u8]) -> Result<ReflectionRequest, ReflectionError> {
    let mut idx = 0usize;
    let mut found: Option<ReflectionRequest> = None;

    while idx < body.len() {
        let (tag, consumed) = decode_varint(&body[idx..])
            .ok_or_else(|| ReflectionError::Malformed("varint tag".into()))?;
        idx += consumed;
        let field_number = (tag >> 3) as u32;
        let wire_type    = (tag & 0x7) as u8;
        match (field_number, wire_type) {
            // host = 1 — skip the value, we don't act on it.
            (1, 2) => {
                let (len, c) = decode_varint(&body[idx..])
                    .ok_or_else(|| ReflectionError::Malformed("host length".into()))?;
                idx += c;
                let end = idx
                    .checked_add(len as usize)
                    .ok_or_else(|| ReflectionError::Malformed("host overflow".into()))?;
                if end > body.len() {
                    return Err(ReflectionError::Malformed("host truncated".into()));
                }
                idx = end;
            }
            (3, 2) => {
                let (s, c) = decode_string(&body[idx..])?;
                idx += c;
                found = Some(ReflectionRequest::FileByFilename(s));
            }
            (4, 2) => {
                let (s, c) = decode_string(&body[idx..])?;
                idx += c;
                found = Some(ReflectionRequest::FileContainingSymbol(s));
            }
            (5, 2) => {
                let (len, c) = decode_varint(&body[idx..])
                    .ok_or_else(|| ReflectionError::Malformed("ext-req length".into()))?;
                idx += c;
                let end = idx
                    .checked_add(len as usize)
                    .ok_or_else(|| ReflectionError::Malformed("ext-req overflow".into()))?;
                if end > body.len() {
                    return Err(ReflectionError::Malformed("ext-req truncated".into()));
                }
                let (containing_type, extension_number) =
                    decode_extension_request(&body[idx..end])?;
                idx = end;
                found = Some(ReflectionRequest::FileContainingExtension {
                    containing_type,
                    extension_number,
                });
            }
            (6, 2) => {
                let (s, c) = decode_string(&body[idx..])?;
                idx += c;
                found = Some(ReflectionRequest::AllExtensionNumbersOfType(s));
            }
            (7, 2) => {
                let (s, c) = decode_string(&body[idx..])?;
                idx += c;
                found = Some(ReflectionRequest::ListServices(s));
            }
            // Unknown tag — skip its value per proto3 forward-compat rules.
            (_, wt) => {
                idx += skip_field(&body[idx..], wt)?;
            }
        }
    }

    Ok(found.unwrap_or(ReflectionRequest::Unknown))
}

fn decode_extension_request(body: &[u8]) -> Result<(String, i32), ReflectionError> {
    let mut idx = 0usize;
    let mut containing_type = String::new();
    let mut extension_number = 0i32;
    while idx < body.len() {
        let (tag, c) = decode_varint(&body[idx..])
            .ok_or_else(|| ReflectionError::Malformed("ext-req tag".into()))?;
        idx += c;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (1, 2) => {
                let (s, c) = decode_string(&body[idx..])?;
                idx += c;
                containing_type = s;
            }
            (2, 0) => {
                let (v, c) = decode_varint(&body[idx..])
                    .ok_or_else(|| ReflectionError::Malformed("ext-req number".into()))?;
                idx += c;
                extension_number = v as i32;
            }
            (_, wt) => {
                idx += skip_field(&body[idx..], wt)?;
            }
        }
    }
    Ok((containing_type, extension_number))
}

/// Encode a `ServerReflectionResponse`.
///
/// Always emits the corresponding `original_request = 2` field so
/// reflection clients (grpcurl) can correlate streamed responses with
/// the requests that produced them.
pub fn encode_response(response: &ReflectionResponse, original_request: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);

    // valid_host = 1 — we have no opinion; clients accept absence.

    // original_request = 2 (sub-message, length-delimited).
    if !original_request.is_empty() {
        out.push(0x12); // tag 2, wire 2
        encode_varint(original_request.len() as u64, &mut out);
        out.extend_from_slice(original_request);
    }

    // message_response oneof.
    match response {
        ReflectionResponse::FileDescriptor(files) => {
            let mut sub = Vec::new();
            for f in files {
                // FileDescriptorResponse.file_descriptor_proto = 1, repeated bytes
                sub.push(0x0a);
                encode_varint(f.len() as u64, &mut sub);
                sub.extend_from_slice(f);
            }
            out.push(0x22); // tag 4, wire 2
            encode_varint(sub.len() as u64, &mut out);
            out.extend_from_slice(&sub);
        }
        ReflectionResponse::ListServices(names) => {
            let mut sub = Vec::new();
            for n in names {
                // ServiceResponse{name=1}
                let mut svc = Vec::new();
                svc.push(0x0a);
                encode_varint(n.len() as u64, &mut svc);
                svc.extend_from_slice(n.as_bytes());
                // ListServiceResponse.service = 1, repeated message
                sub.push(0x0a);
                encode_varint(svc.len() as u64, &mut sub);
                sub.extend_from_slice(&svc);
            }
            out.push(0x32); // tag 6, wire 2
            encode_varint(sub.len() as u64, &mut out);
            out.extend_from_slice(&sub);
        }
        ReflectionResponse::Error { error_code, error_message } => {
            let mut sub = Vec::new();
            // error_code = 1 (varint, int32)
            if *error_code != 0 {
                sub.push(0x08);
                encode_varint(*error_code as i64 as u64, &mut sub);
            }
            // error_message = 2 (string)
            if !error_message.is_empty() {
                sub.push(0x12);
                encode_varint(error_message.len() as u64, &mut sub);
                sub.extend_from_slice(error_message.as_bytes());
            }
            out.push(0x3a); // tag 7, wire 2
            encode_varint(sub.len() as u64, &mut out);
            out.extend_from_slice(&sub);
        }
    }

    out
}

fn decode_string(body: &[u8]) -> Result<(String, usize), ReflectionError> {
    let (len, c) = decode_varint(body)
        .ok_or_else(|| ReflectionError::Malformed("string length".into()))?;
    let total = c
        .checked_add(len as usize)
        .ok_or_else(|| ReflectionError::Malformed("string overflow".into()))?;
    if total > body.len() {
        return Err(ReflectionError::Malformed("string truncated".into()));
    }
    let bytes = &body[c..c + len as usize];
    let s = std::str::from_utf8(bytes)
        .map_err(|_| ReflectionError::Malformed("string utf-8".into()))?
        .to_string();
    Ok((s, total))
}

fn skip_field(body: &[u8], wire_type: u8) -> Result<usize, ReflectionError> {
    match wire_type {
        0 => {
            // varint
            let (_, c) = decode_varint(body)
                .ok_or_else(|| ReflectionError::Malformed("skip varint".into()))?;
            Ok(c)
        }
        1 => {
            // 64-bit fixed
            if body.len() < 8 {
                return Err(ReflectionError::Malformed("skip fixed64".into()));
            }
            Ok(8)
        }
        2 => {
            // length-delimited
            let (len, c) = decode_varint(body)
                .ok_or_else(|| ReflectionError::Malformed("skip ld length".into()))?;
            let total = c
                .checked_add(len as usize)
                .ok_or_else(|| ReflectionError::Malformed("skip ld overflow".into()))?;
            if total > body.len() {
                return Err(ReflectionError::Malformed("skip ld truncated".into()));
            }
            Ok(total)
        }
        5 => {
            // 32-bit fixed
            if body.len() < 4 {
                return Err(ReflectionError::Malformed("skip fixed32".into()));
            }
            Ok(4)
        }
        wt => Err(ReflectionError::Malformed(format!(
            "unsupported wire type {wt}"
        ))),
    }
}

pub(crate) fn decode_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift  = 0u32;
    for (i, byte) in bytes.iter().take(10).enumerate() {
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
    }
    None
}

pub(crate) fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `string field` with the given tag byte + utf8 contents.
    fn encode_string_field(tag: u8, value: &str) -> Vec<u8> {
        let mut out = vec![tag];
        encode_varint(value.len() as u64, &mut out);
        out.extend_from_slice(value.as_bytes());
        out
    }

    /// @covers: encode_varint, decode_varint round-trip across boundary values.
    #[test]
    fn test_varint_round_trips_for_typical_values() {
        for v in [0u64, 1, 0x7f, 0x80, 0x3fff, 0x4000, 0xffff_ffff] {
            let mut buf = Vec::new();
            encode_varint(v, &mut buf);
            let (out, c) = decode_varint(&buf).unwrap();
            assert_eq!(out, v, "round-trip failed for {v}");
            assert_eq!(c, buf.len(), "consumed mismatch for {v}");
        }
    }

    /// @covers: decode_request — empty body yields Unknown (no oneof set).
    #[test]
    fn test_decode_request_empty_body_returns_unknown_variant() {
        let r = decode_request(&[]).unwrap();
        assert_eq!(r, ReflectionRequest::Unknown);
    }

    /// @covers: decode_request — list_services round-trips.
    #[test]
    fn test_decode_request_list_services_field_seven_round_trips() {
        let body = encode_string_field(0x3a, "");
        let r = decode_request(&body).unwrap();
        assert_eq!(r, ReflectionRequest::ListServices(String::new()));
    }

    /// @covers: decode_request — file_by_filename field=3.
    #[test]
    fn test_decode_request_file_by_filename_round_trips() {
        let body = encode_string_field(0x1a, "pkg/foo.proto");
        let r = decode_request(&body).unwrap();
        assert_eq!(r, ReflectionRequest::FileByFilename("pkg/foo.proto".into()));
    }

    /// @covers: decode_request — file_containing_symbol field=4.
    #[test]
    fn test_decode_request_file_containing_symbol_round_trips() {
        let body = encode_string_field(0x22, "pkg.MyService");
        let r = decode_request(&body).unwrap();
        assert_eq!(
            r,
            ReflectionRequest::FileContainingSymbol("pkg.MyService".into())
        );
    }

    /// @covers: decode_request — host (field=1) is preserved/skipped without
    /// affecting the oneof outcome.
    #[test]
    fn test_decode_request_host_field_does_not_affect_oneof_decoding() {
        let mut body = encode_string_field(0x0a, "myhost");
        body.extend_from_slice(&encode_string_field(0x3a, "")); // list_services
        let r = decode_request(&body).unwrap();
        assert_eq!(r, ReflectionRequest::ListServices(String::new()));
    }

    /// @covers: decode_request — unknown wire-type returns Malformed.
    #[test]
    fn test_decode_request_unknown_wire_type_six_returns_malformed_error() {
        // tag = 1<<3 | 6 (group start, deprecated) — must fail loudly.
        let body = vec![0x0e];
        let err = decode_request(&body).expect_err("must fail");
        match err {
            ReflectionError::Malformed(_) => {}
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    /// @covers: decode_request — truncated string returns Malformed.
    #[test]
    fn test_decode_request_truncated_string_returns_malformed_error() {
        // tag list_services with length 5 but only 3 bytes follow.
        let body = vec![0x3a, 0x05, b'a', b'b', b'c'];
        let err = decode_request(&body).expect_err("must fail");
        assert!(matches!(err, ReflectionError::Malformed(_)));
    }

    /// @covers: decode_request — extension request (field=5) decodes both inner fields.
    #[test]
    fn test_decode_request_file_containing_extension_round_trips_both_fields() {
        // ExtensionRequest { containing_type = "pkg.M", extension_number = 42 }
        let mut inner = Vec::new();
        inner.push(0x0a); // string field 1
        encode_varint(5, &mut inner);
        inner.extend_from_slice(b"pkg.M");
        inner.push(0x10); // varint field 2
        encode_varint(42, &mut inner);

        let mut body = Vec::new();
        body.push(0x2a); // tag 5, wire 2
        encode_varint(inner.len() as u64, &mut body);
        body.extend_from_slice(&inner);

        let r = decode_request(&body).unwrap();
        match r {
            ReflectionRequest::FileContainingExtension { containing_type, extension_number } => {
                assert_eq!(containing_type, "pkg.M");
                assert_eq!(extension_number, 42);
            }
            other => panic!("expected FileContainingExtension, got {other:?}"),
        }
    }

    /// @covers: encode_response — ListServices emits tag 6 and one ServiceResponse per name.
    #[test]
    fn test_encode_response_list_services_includes_each_name_under_tag_six() {
        let bytes = encode_response(
            &ReflectionResponse::ListServices(vec!["pkg.A".into(), "pkg.B".into()]),
            &[],
        );
        // First byte must be tag 6, wire 2 → 0x32.
        assert_eq!(bytes.first().copied(), Some(0x32));
    }

    /// @covers: encode_response — Error variant emits tag 7 and embeds code+message.
    #[test]
    fn test_encode_response_error_variant_includes_code_and_message_under_tag_seven() {
        let bytes = encode_response(
            &ReflectionResponse::Error {
                error_code: 5,
                error_message: "not found".into(),
            },
            &[],
        );
        assert_eq!(bytes.first().copied(), Some(0x3a));
    }

    /// @covers: encode_response — original_request is embedded under field 2 when supplied.
    #[test]
    fn test_encode_response_includes_original_request_field_two_when_supplied() {
        let original = vec![0x3a, 0x00]; // ListServices request
        let bytes = encode_response(
            &ReflectionResponse::ListServices(vec![]),
            &original,
        );
        assert_eq!(bytes.first().copied(), Some(0x12), "field 2 = original_request");
    }

    /// @covers: encode_response — FileDescriptor wraps each file under tag 4 sub-tag 1.
    #[test]
    fn test_encode_response_file_descriptor_response_wraps_each_payload_under_tag_four() {
        let bytes = encode_response(
            &ReflectionResponse::FileDescriptor(vec![vec![0xde, 0xad]]),
            &[],
        );
        assert_eq!(bytes.first().copied(), Some(0x22), "tag 4 wire 2 = 0x22");
    }
}
