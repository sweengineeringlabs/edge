use std::collections::HashMap;

use futures::future::BoxFuture;
use reqwest_middleware::ClientWithMiddleware;

use crate::api::port::http_outbound::{HttpOutbound, HttpOutboundError, HttpOutboundResult};
use crate::api::value_object::{HttpBody, HttpRequest, HttpResponse};

pub(crate) struct DefaultHttpOutbound {
    client: ClientWithMiddleware,
    base_url: Option<String>,
}

impl DefaultHttpOutbound {
    pub(crate) fn new(client: ClientWithMiddleware, base_url: Option<String>) -> Self {
        Self { client, base_url }
    }

    fn resolve_url(&self, url: &str) -> String {
        match &self.base_url {
            Some(base) if !url.starts_with("http://") && !url.starts_with("https://") => {
                format!("{}/{}", base.trim_end_matches('/'), url.trim_start_matches('/'))
            }
            _ => url.to_string(),
        }
    }
}

impl HttpOutbound for DefaultHttpOutbound {
    fn send(&self, request: HttpRequest) -> BoxFuture<'_, HttpOutboundResult<HttpResponse>> {
        Box::pin(async move {
            let url = self.resolve_url(&request.url);

            let method = reqwest::Method::from_bytes(request.method.to_string().as_bytes())
                .map_err(|e| HttpOutboundError::InvalidRequest(e.to_string()))?;

            let mut builder = self.client.request(method, &url);

            for (k, v) in &request.headers {
                builder = builder.header(k, v);
            }

            if !request.query.is_empty() {
                let pairs: Vec<(&str, &str)> = request
                    .query
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                builder = builder.query(&pairs);
            }

            if let Some(body) = request.body {
                builder = match body {
                    HttpBody::Json(v) => builder.json(&v),
                    HttpBody::Raw(b) => builder.body(b),
                    HttpBody::Form(f) => {
                        let pairs: Vec<(String, String)> = f.into_iter().collect();
                        builder.form(&pairs)
                    }
                    HttpBody::Multipart(parts) => {
                        let mut form = reqwest::multipart::Form::new();
                        for part in parts {
                            let mut mp = reqwest::multipart::Part::bytes(part.data);
                            if let Some(filename) = part.filename {
                                mp = mp.file_name(filename);
                            }
                            if let Some(ct) = part.content_type {
                                mp = mp.mime_str(&ct).map_err(|e| {
                                    HttpOutboundError::InvalidRequest(e.to_string())
                                })?;
                            }
                            form = form.part(part.name, mp);
                        }
                        builder.multipart(form)
                    }
                };
            }

            if let Some(timeout) = request.timeout {
                builder = builder.timeout(timeout);
            }

            let response = builder
                .send()
                .await
                .map_err(|e| HttpOutboundError::ConnectionFailed(e.to_string()))?;

            let status = response.status().as_u16();
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();
            let body = response
                .bytes()
                .await
                .map_err(|e| HttpOutboundError::Internal(e.to_string()))?
                .to_vec();

            Ok(HttpResponse { status, headers, body })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>> {
        Box::pin(async move { Ok(()) })
    }
}
