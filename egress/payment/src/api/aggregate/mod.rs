//! Payment aggregates — types with identity.
pub mod customer;
#[allow(clippy::module_inception)]
pub mod payment;

pub use customer::Customer;
pub use payment::Payment;
