//! Built-in [`AuthzPolicy`] backed by a [`MethodAclConfig`].

use swe_edge_ingress_grpc::PeerIdentity;

use crate::api::{AuthzPolicy, MethodAclConfig};

/// Policy that consults a [`MethodAclConfig`] keyed on the caller's
/// CN.  See [`MethodAclConfig`] for the matching rules.
#[derive(Debug, Clone)]
pub struct MethodAclPolicy {
    config: MethodAclConfig,
}

impl MethodAclPolicy {
    /// Construct from an ACL config.
    pub fn from_config(config: MethodAclConfig) -> Self { Self { config } }
}

impl AuthzPolicy for MethodAclPolicy {
    fn allows(&self, identity: &PeerIdentity, method: &str) -> bool {
        let subject = identity.cn.as_deref();
        self.config.allows(subject, method)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: MethodAclPolicy — delegates allow decision to config.
    #[test]
    fn test_allows_returns_true_when_config_grants_subject_method() {
        let cfg = MethodAclConfig::deny_all().allow("alice", ["/svc/Read".into()]);
        let policy = MethodAclPolicy::from_config(cfg);
        let alice  = PeerIdentity { cn: Some("alice".into()), ..Default::default() };
        assert!(policy.allows(&alice, "/svc/Read"));
    }

    /// @covers: MethodAclPolicy — denies subjects absent from the ACL.
    #[test]
    fn test_allows_returns_false_when_subject_not_in_acl() {
        let cfg = MethodAclConfig::deny_all().allow("alice", ["/svc/Read".into()]);
        let policy = MethodAclPolicy::from_config(cfg);
        let bob   = PeerIdentity { cn: Some("bob".into()), ..Default::default() };
        assert!(!policy.allows(&bob, "/svc/Read"));
    }

    /// @covers: MethodAclPolicy — identity with no CN is denied.
    #[test]
    fn test_allows_returns_false_when_identity_has_no_cn() {
        let cfg = MethodAclConfig::deny_all().allow("alice", ["/svc/Read".into()]);
        let policy = MethodAclPolicy::from_config(cfg);
        let unknown = PeerIdentity::default();
        assert!(!policy.allows(&unknown, "/svc/Read"));
    }
}
