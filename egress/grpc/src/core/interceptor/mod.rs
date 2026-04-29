//! Built-in outbound interceptors.

pub(crate) mod trace_context_interceptor;

pub use trace_context_interceptor::TraceContextInterceptor;
