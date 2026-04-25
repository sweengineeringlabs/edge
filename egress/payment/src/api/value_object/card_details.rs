//! Card payment details (tokenised — no raw PAN).

use serde::{Deserialize, Serialize};

/// Tokenised card details (no raw card numbers stored).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetails {
    pub token: String,
    pub last_four: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<u8>,
    pub exp_year: Option<u16>,
}

impl CardDetails {
    pub fn from_token(token: impl Into<String>) -> Self {
        Self { token: token.into(), last_four: None, brand: None, exp_month: None, exp_year: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_token
    #[test]
    fn test_from_token_creates_card_details_with_token() {
        let c = CardDetails::from_token("tok_visa_123");
        assert_eq!(c.token, "tok_visa_123");
        assert!(c.last_four.is_none());
    }
}
