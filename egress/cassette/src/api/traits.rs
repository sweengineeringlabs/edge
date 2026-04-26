//! Primary trait re-export hub for `swe_edge_egress_cassette`.

pub(crate) type HttpCassetteTrait = dyn crate::api::http_cassette::HttpCassette;
