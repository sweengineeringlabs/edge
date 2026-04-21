//! Pluggable HTTP auth strategy contract.
//!
//! Async trait (per `async_trait` macro) so strategies that need
//! async setup — currently Digest, for fetching a nonce via a
//! side-channel request — fit the same shape as the synchronous
//! schemes (Bearer / Basic / Header / AWS SigV4).
//!
//! `pub(crate)` on purpose — consumers never implement this
//! trait. Plug-in extension is scoped to new variants on
//! [`AuthConfig`](crate::api::auth_config::AuthConfig), not to
//! arbitrary external impls.

use async_trait::async_trait;

use crate::api::error::Error;

/// Attaches configured credentials to an outbound HTTP request.
///
/// Two-phase contract:
///
/// 1. [`prepare`](AuthStrategy::prepare) — optional async
///    setup. Called once per request before `authorize`. Most
///    strategies leave this at the no-op default; Digest uses
///    it to fetch a fresh nonce for the target host via a
///    side-channel `reqwest::Client` it owns.
///
/// 2. [`authorize`](AuthStrategy::authorize) — sync header
///    attachment. Called after `prepare` completes. Strategies
///    hold any pre-computed state they need so the hot path on
///    every request is a trivial header insert.
#[async_trait]
pub(crate) trait AuthStrategy: Send + Sync + std::fmt::Debug {
    /// Optional async setup step.
    ///
    /// Default: no-op. Strategies that need to fetch or refresh
    /// per-host state (Digest's nonce cache) override this.
    /// `host` is the URL host of the outbound request — `None`
    /// when the URL is hostless (unlikely in practice).
    async fn prepare(&self, _host: Option<&str>) -> Result<(), Error> {
        Ok(())
    }

    /// Apply the strategy to `req` in place. Called once per
    /// outbound request AFTER `prepare` completes.
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    #[derive(Debug)]
    struct StubStrategy;
    #[async_trait]
    impl AuthStrategy for StubStrategy {
        fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
            req.headers_mut()
                .insert("x-stub", "applied".parse().unwrap());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct PrepareCountingStrategy {
        calls: std::sync::atomic::AtomicUsize,
    }
    #[async_trait]
    impl AuthStrategy for PrepareCountingStrategy {
        async fn prepare(&self, _host: Option<&str>) -> Result<(), Error> {
            self.calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        fn authorize(&self, _req: &mut reqwest::Request) -> Result<(), Error> {
            Ok(())
        }
    }

    /// @covers: AuthStrategy
    #[tokio::test]
    async fn test_default_prepare_is_noop() {
        let s = StubStrategy;
        s.prepare(Some("example.test")).await.unwrap();
        // No assertion on a side effect — the point is that the
        // default impl exists and returns Ok without panicking.
    }

    /// @covers: AuthStrategy
    #[tokio::test]
    async fn test_authorize_mutates_request() {
        let s: Box<dyn AuthStrategy> = Box::new(StubStrategy);
        let mut req = reqwest::Request::new(
            Method::GET,
            Url::parse("http://example.test/").unwrap(),
        );
        s.prepare(Some("example.test")).await.unwrap();
        s.authorize(&mut req).unwrap();
        assert_eq!(req.headers().get("x-stub").unwrap(), "applied");
    }

    /// @covers: AuthStrategy
    #[tokio::test]
    async fn test_prepare_can_be_called_once_per_request() {
        let s = PrepareCountingStrategy {
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        s.prepare(Some("host-1")).await.unwrap();
        s.prepare(Some("host-1")).await.unwrap();
        assert_eq!(s.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
