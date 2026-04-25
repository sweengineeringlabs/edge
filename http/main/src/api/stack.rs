//! HTTP middleware stack abstraction — counterpart for `core::stack`.
//!
//! The concrete `DefaultStack` assembler lives in `core::stack`;
//! it implements [`StackAssembler`](crate::api::traits::StackAssembler)
//! and wires the full middleware chain from a resolved
//! [`StackConfig`](crate::api::stack_config::StackConfig).
