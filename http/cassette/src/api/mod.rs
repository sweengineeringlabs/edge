//! API layer — public schema + trait contracts + public types.
pub mod builder;
pub(crate) mod body_scrubber;
pub(crate) mod cassette_config;
pub(crate) mod cassette_layer;
pub(crate) mod default_http_cassette;
pub(crate) mod error;
pub(crate) mod http_cassette;
pub(crate) mod recorded_interaction;
pub(crate) mod traits;

