//! Configuration for the built-in [`MethodAclPolicy`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Method-level allowlist keyed by the caller's verified subject
/// (CN for mTLS, JWT `sub` for bearer).
///
/// Entries are exact, case-sensitive matches.  Empty value lists
/// mean "allow all methods" for that subject; absent subjects are
/// denied by default.  A wildcard subject `"*"` applies to every
/// authenticated caller.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MethodAclConfig {
    /// `subject -> [method-paths]`.
    #[serde(default)]
    pub by_subject: HashMap<String, Vec<String>>,
}

impl MethodAclConfig {
    /// Build an empty config — denies every authenticated caller.
    pub fn deny_all() -> Self { Self::default() }

    /// Allow `subject` to invoke each listed `method` path.
    pub fn allow(mut self, subject: impl Into<String>, methods: impl IntoIterator<Item = String>) -> Self {
        self.by_subject
            .entry(subject.into())
            .or_default()
            .extend(methods);
        self
    }

    /// Allow EVERY authenticated caller to invoke `method` (wildcard subject).
    pub fn allow_for_any_authenticated(mut self, method: impl Into<String>) -> Self {
        self.by_subject
            .entry("*".to_string())
            .or_default()
            .push(method.into());
        self
    }

    /// Load from TOML.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Returns `true` when `subject` is allowed to call `method`.
    pub(crate) fn allows(&self, subject: Option<&str>, method: &str) -> bool {
        // Wildcard subject — applies ONLY to authenticated callers
        // (subject = Some(_)).  An absent subject means upstream
        // authn never populated identity; wildcards MUST NOT
        // silently elevate that case to "allowed".
        if subject.is_some() {
            if let Some(any) = self.by_subject.get("*") {
                if any.iter().any(|m| m == method) {
                    return true;
                }
            }
        }
        let Some(sub) = subject else { return false };
        match self.by_subject.get(sub) {
            None => false,
            Some(methods) if methods.is_empty() => true,
            Some(methods) => methods.iter().any(|m| m == method),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: MethodAclConfig::deny_all — denies any caller.
    #[test]
    fn test_deny_all_default_rejects_every_subject() {
        let cfg = MethodAclConfig::deny_all();
        assert!(!cfg.allows(Some("alice"), "/svc/M"));
        assert!(!cfg.allows(None, "/svc/M"));
    }

    /// @covers: MethodAclConfig::allow — listed method permitted.
    #[test]
    fn test_allow_grants_only_listed_methods_to_subject() {
        let cfg = MethodAclConfig::deny_all().allow("alice", ["/svc/Read".to_string()]);
        assert!(cfg.allows(Some("alice"), "/svc/Read"));
        assert!(!cfg.allows(Some("alice"), "/svc/Write"));
    }

    /// @covers: MethodAclConfig — empty method list means "any method".
    #[test]
    fn test_empty_method_list_allows_any_method_for_subject() {
        let cfg = MethodAclConfig {
            by_subject: HashMap::from([("alice".to_string(), Vec::new())]),
        };
        assert!(cfg.allows(Some("alice"), "/svc/M"));
        assert!(cfg.allows(Some("alice"), "/svc/Other"));
    }

    /// @covers: wildcard subject — applies to anyone authenticated.
    #[test]
    fn test_wildcard_subject_grants_method_to_any_authenticated_caller() {
        let cfg = MethodAclConfig::deny_all().allow_for_any_authenticated("/health");
        assert!(cfg.allows(Some("alice"), "/health"));
        assert!(cfg.allows(Some("bob"), "/health"));
        // Wildcard does NOT permit unauthenticated callers — `subject = None`
        // is still rejected.
        assert!(!cfg.allows(None, "/health"));
    }

    /// @covers: MethodAclConfig::from_toml — schema round-trip.
    #[test]
    fn test_from_toml_round_trips_by_subject_table() {
        let toml_src = r#"
            [by_subject]
            alice = ["/svc/Read", "/svc/Write"]
            "*"   = ["/health"]
        "#;
        let cfg = MethodAclConfig::from_toml(toml_src).expect("toml parses");
        assert!(cfg.allows(Some("alice"), "/svc/Read"));
        assert!(cfg.allows(Some("bob"),   "/health"));
    }
}
