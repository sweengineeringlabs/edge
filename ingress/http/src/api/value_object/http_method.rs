//! HTTP method enum.

use serde::{Deserialize, Serialize};

/// An HTTP request method.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// HTTP GET.
    #[default]
    Get,
    /// HTTP POST.
    Post,
    /// HTTP PUT.
    Put,
    /// HTTP PATCH.
    Patch,
    /// HTTP DELETE.
    Delete,
    /// HTTP HEAD.
    Head,
    /// HTTP OPTIONS.
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"), Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"), Self::Patch => write!(f, "PATCH"),
            Self::Delete => write!(f, "DELETE"), Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_display_formats_correctly() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
    }
}
