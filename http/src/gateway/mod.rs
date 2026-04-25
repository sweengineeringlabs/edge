//! Gateway layer — inbound/outbound adapters for the http workspace.
//!
//! `input/`  — deserializes external data into api/ types.
//! `output/` — serializes api/ types into external representations.

pub(crate) mod input;
pub(crate) mod output;
