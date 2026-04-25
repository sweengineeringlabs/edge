//! State management for lazy initialization and caching of services.

use parking_lot::RwLock;
use std::future::Future;
use std::sync::Arc;

use crate::api::ingress_error::IngressResult;

/// Lazy-initialized service wrapper.
pub struct CachedService<S: ?Sized> {
    inner: RwLock<Option<Arc<S>>>,
}

impl<S: ?Sized> CachedService<S> {
    pub const fn new() -> Self { Self { inner: RwLock::new(None) } }

    pub async fn get_or_init<F, Fut>(&self, init: F) -> IngressResult<Arc<S>>
    where F: FnOnce() -> Fut, Fut: Future<Output = IngressResult<Arc<S>>>,
    {
        { let g = self.inner.read(); if let Some(ref s) = *g { return Ok(Arc::clone(s)); } }
        let new_service = init().await?;
        { let mut g = self.inner.write(); if g.is_none() { *g = Some(Arc::clone(&new_service)); } Ok(Arc::clone(g.as_ref().unwrap())) }
    }

    pub fn is_initialized(&self) -> bool { self.inner.read().is_some() }

    pub fn reset(&self) { *self.inner.write() = None; }

    pub fn get(&self) -> Option<Arc<S>> { self.inner.read().as_ref().map(Arc::clone) }
}

impl<S: ?Sized> Default for CachedService<S> {
    fn default() -> Self { Self::new() }
}

/// Cached service with configuration key.
pub struct ConfiguredCache<S: ?Sized, C: PartialEq + Clone> {
    service: RwLock<Option<Arc<S>>>,
    config: RwLock<Option<C>>,
}

impl<S: ?Sized, C: PartialEq + Clone> ConfiguredCache<S, C> {
    pub const fn new() -> Self { Self { service: RwLock::new(None), config: RwLock::new(None) } }

    pub async fn get_or_init_with_config<F, Fut>(&self, config: &C, init: F) -> IngressResult<Arc<S>>
    where F: FnOnce(C) -> Fut, Fut: Future<Output = IngressResult<Arc<S>>>,
    {
        { let sg = self.service.read(); let cg = self.config.read(); if let (Some(ref s), Some(ref c)) = (&*sg, &*cg) { if c == config { return Ok(Arc::clone(s)); } } }
        let new_service = init(config.clone()).await?;
        { let mut sg = self.service.write(); let mut cg = self.config.write(); *sg = Some(Arc::clone(&new_service)); *cg = Some(config.clone()); }
        Ok(new_service)
    }

    pub fn is_initialized(&self) -> bool { self.service.read().is_some() }

    pub fn current_config(&self) -> Option<C> { self.config.read().clone() }

    pub fn reset(&self) { *self.service.write() = None; *self.config.write() = None; }
}

impl<S: ?Sized, C: PartialEq + Clone> Default for ConfiguredCache<S, C> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Svc { val: i32 }

    /// @covers: get_or_init
    #[tokio::test]
    async fn test_get_or_init_initializes_and_caches_service() {
        let cache: CachedService<Svc> = CachedService::new();
        assert!(!cache.is_initialized());
        let r = cache.get_or_init(|| async { Ok(Arc::new(Svc { val: 42 })) }).await.unwrap();
        assert_eq!(r.val, 42);
        assert!(cache.is_initialized());
    }

    /// @covers: is_initialized
    #[test]
    fn test_is_initialized_returns_false_for_new_cache() {
        let cache: CachedService<Svc> = CachedService::new();
        assert!(!cache.is_initialized());
    }

    /// @covers: reset
    #[tokio::test]
    async fn test_reset_clears_cached_service() {
        let cache: CachedService<Svc> = CachedService::new();
        cache.get_or_init(|| async { Ok(Arc::new(Svc { val: 1 })) }).await.unwrap();
        cache.reset();
        assert!(!cache.is_initialized());
    }

    /// @covers: get
    #[test]
    fn test_get_returns_none_on_empty_cache() {
        let cache: CachedService<Svc> = CachedService::new();
        assert!(cache.get().is_none());
    }

    /// @covers: get_or_init_with_config
    #[tokio::test]
    async fn test_get_or_init_with_config_reinitializes_on_config_change() {
        let cache: ConfiguredCache<Svc, String> = ConfiguredCache::new();
        let first = cache.get_or_init_with_config(&"cfg1".into(), |_| async { Ok(Arc::new(Svc { val: 1 })) }).await.unwrap();
        assert_eq!(first.val, 1);
        let third = cache.get_or_init_with_config(&"cfg2".into(), |_| async { Ok(Arc::new(Svc { val: 3 })) }).await.unwrap();
        assert_eq!(third.val, 3);
    }

    /// @covers: current_config
    #[tokio::test]
    async fn test_current_config_returns_none_before_init() {
        let cache: ConfiguredCache<Svc, String> = ConfiguredCache::new();
        assert!(cache.current_config().is_none());
        cache.get_or_init_with_config(&"my_cfg".into(), |_| async { Ok(Arc::new(Svc { val: 1 })) }).await.unwrap();
        assert_eq!(cache.current_config(), Some("my_cfg".to_string()));
    }
}
