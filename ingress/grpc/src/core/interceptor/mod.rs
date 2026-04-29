//! Built-in inbound interceptors.

pub(crate) mod trace_context_interceptor;

pub use trace_context_interceptor::{
    TraceContextInterceptor, EXTRACTED_TRACEPARENT, EXTRACTED_TRACESTATE, TRACEPARENT, TRACESTATE,
};
