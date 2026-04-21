//! Per-scheme strategy impls + the factory that turns an
//! `AuthConfig` into the right concrete strategy.

pub(crate) mod aws_sigv4_strategy;
pub(crate) mod basic_strategy;
pub(crate) mod bearer_strategy;
pub(crate) mod digest_strategy;
pub(crate) mod header_strategy;
pub(crate) mod noop_strategy;
pub(crate) mod strategy_factory;

pub(crate) use strategy_factory::build_strategy;
