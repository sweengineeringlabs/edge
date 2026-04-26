use std::time::Duration;

use reqwest_middleware::ClientBuilder;
use swe_edge_egress_tls::TlsApplier;

use crate::core::DefaultHttpOutbound;

pub use crate::api::port::{HttpOutbound, HttpOutboundError, HttpOutboundResult};
pub use crate::api::value_object::{
    FormPart, HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse,
};

/// Aggregate middleware config for assembling a [`DefaultHttpOutbound`].
#[derive(Debug, Clone)]
pub struct HttpOutboundConfig {
    pub http:          HttpConfig,
    pub auth:          swe_edge_egress_auth::AuthConfig,
    pub retry:         swe_edge_egress_retry::RetryConfig,
    pub rate:          swe_edge_egress_rate::RateConfig,
    pub breaker:       swe_edge_egress_breaker::BreakerConfig,
    pub cache:         swe_edge_egress_cache::CacheConfig,
    pub cassette:      swe_edge_egress_cassette::CassetteConfig,
    /// On-disk cassette fixture name (no extension). Maps to
    /// `<cassette_dir>/<cassette_name>.yaml`.
    pub cassette_name: String,
    pub tls:           swe_edge_egress_tls::TlsConfig,
}

/// Error returned when assembling an [`HttpOutbound`] fails at startup.
#[derive(Debug, thiserror::Error)]
pub enum HttpOutboundBuildError {
    #[error("auth: {0}")]
    Auth(#[from] swe_edge_egress_auth::Error),
    #[error("retry: {0}")]
    Retry(#[from] swe_edge_egress_retry::Error),
    #[error("rate: {0}")]
    Rate(#[from] swe_edge_egress_rate::Error),
    #[error("breaker: {0}")]
    Breaker(#[from] swe_edge_egress_breaker::Error),
    #[error("cache: {0}")]
    Cache(#[from] swe_edge_egress_cache::Error),
    #[error("cassette: {0}")]
    Cassette(#[from] swe_edge_egress_cassette::Error),
    #[error("tls: {0}")]
    Tls(#[from] swe_edge_egress_tls::Error),
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
}

/// Build a fully assembled [`HttpOutbound`] from the supplied config.
///
/// Assembly order: TLS → reqwest client → auth → retry → rate →
/// breaker → cache → cassette.
pub fn http_outbound(
    config: HttpOutboundConfig,
) -> Result<impl HttpOutbound, HttpOutboundBuildError> {
    assemble(
        config.http,
        swe_edge_egress_auth::Builder::with_config(config.auth).build()?,
        swe_edge_egress_retry::Builder::with_config(config.retry).build()?,
        swe_edge_egress_rate::Builder::with_config(config.rate).build()?,
        swe_edge_egress_breaker::Builder::with_config(config.breaker).build()?,
        swe_edge_egress_cache::Builder::with_config(config.cache).build()?,
        swe_edge_egress_cassette::Builder::with_config(config.cassette).build(&config.cassette_name)?,
        swe_edge_egress_tls::Builder::with_config(config.tls).build()?,
    )
}

/// Build an [`HttpOutbound`] using the SWE-shipped defaults for every
/// middleware layer (pass-through auth, no TLS, sensible retry/rate/breaker
/// policies from each crate's `config/application.toml`).
pub fn default_http_outbound() -> Result<impl HttpOutbound, HttpOutboundBuildError> {
    assemble(
        HttpConfig::default(),
        swe_edge_egress_auth::builder()?.build()?,
        swe_edge_egress_retry::builder()?.build()?,
        swe_edge_egress_rate::builder()?.build()?,
        swe_edge_egress_breaker::builder()?.build()?,
        swe_edge_egress_cache::builder()?.build()?,
        swe_edge_egress_cassette::builder()?.build("default")?,
        swe_edge_egress_tls::builder()?.build()?,
    )
}

fn assemble(
    http_cfg: HttpConfig,
    auth: swe_edge_egress_auth::AuthMiddleware,
    retry: swe_edge_egress_retry::RetryLayer,
    rate: swe_edge_egress_rate::RateLayer,
    breaker: swe_edge_egress_breaker::BreakerLayer,
    cache: swe_edge_egress_cache::CacheLayer,
    cassette: swe_edge_egress_cassette::CassetteLayer,
    tls: swe_edge_egress_tls::TlsLayer,
) -> Result<DefaultHttpOutbound, HttpOutboundBuildError> {
    let mut cb = reqwest::Client::builder();
    cb = tls.apply_to(cb)?;
    cb = cb.timeout(Duration::from_secs(http_cfg.timeout_secs));
    cb = cb.connect_timeout(Duration::from_secs(http_cfg.connect_timeout_secs));
    if let Some(ua) = &http_cfg.user_agent {
        cb = cb.user_agent(ua);
    }
    if http_cfg.follow_redirects {
        cb = cb.redirect(reqwest::redirect::Policy::limited(
            http_cfg.max_redirects as usize,
        ));
    } else {
        cb = cb.redirect(reqwest::redirect::Policy::none());
    }
    let reqwest_client = cb.build()?;

    let client = ClientBuilder::new(reqwest_client)
        .with(auth)
        .with(retry)
        .with(rate)
        .with(breaker)
        .with(cache)
        .with(cassette)
        .build();

    Ok(DefaultHttpOutbound::new(client, http_cfg.base_url))
}
