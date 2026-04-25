//! # swe-gateway
//!
//! A gateway abstraction layer with inbound + outbound support for common integrations.
//!
//! ## Overview
//!
//! This crate provides a hexagonal architecture pattern for abstracting external dependencies
//! behind clean interfaces. Each gateway type has three traits:
//!
//! - **Inbound**: Read/query operations
//! - **Outbound**: Write/mutation operations
//! - **Gateway**: Combined trait (extends both Inbound and Outbound)
//!
//! ## Gateway Types
//!
//! | Gateway | Abstracts | Default Impl |
//! |---------|-----------|--------------|
//! | DatabaseGateway | SQL, NoSQL, in-memory | MemoryDatabase |
//! | FileGateway | Local FS, S3, GCS | LocalFileGateway |
//! | HttpGateway | REST, GraphQL, gRPC | RestClient |
//! | NotificationGateway | Email, SMS, Push | ConsoleNotifier |
//! | PaymentGateway | Stripe, PayPal, Square | MockPaymentGateway |
//!
//! ## Quick Start
//!
//! ```rust
//! use edge_gateway::prelude::*;
//! use edge_gateway::saf;
//!
//! // Create a memory database
//! let db = saf::memory_database();
//!
//! // Create a local file gateway
//! let files = saf::local_file_gateway("./data");
//!
//! // Create a mock payment gateway
//! let payments = saf::mock_payment_gateway();
//! ```
//!
//! ## Feature Flags
//!
//! Additional backends can be enabled via feature flags:
//!
//! - `postgres` - PostgreSQL database support
//! - `mysql` - MySQL/MariaDB database support
//! - `s3` - Amazon S3 file storage
//! - `email` - Email notifications via SMTP
//! - `stripe` - Stripe payment processing
//! - `full` - Enable postgres, s3, email, and stripe
//!

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// ── Private layer modules (SEA: all layers are private) ──
mod api;
pub(crate) mod core;

// ── Public modules ──
pub mod saf;

// ── Public surface delegated via saf (SEA rule §7) ──
pub use saf::*;

/// Prelude module with commonly used imports.
pub mod prelude {
    pub use crate::saf::*;
}
