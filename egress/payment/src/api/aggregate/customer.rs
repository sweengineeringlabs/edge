//! Customer entity for payment operations.

use serde::{Deserialize, Serialize};

/// A customer in the payment system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub provider_customer_id: Option<String>,
}

impl Customer {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), email: None, name: None, provider_customer_id: None }
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into()); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: new
    #[test]
    fn test_new_creates_customer_with_id() {
        let c = Customer::new("cust-001");
        assert_eq!(c.id, "cust-001");
        assert!(c.email.is_none());
    }

    /// @covers: with_email
    #[test]
    fn test_with_email_sets_email() {
        let c = Customer::new("c").with_email("a@b.com");
        assert_eq!(c.email, Some("a@b.com".to_string()));
    }
}
