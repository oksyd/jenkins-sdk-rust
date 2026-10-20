//! Blocking Jenkins request execution.
use super::{Builder, config::Config, crumb::CachedCrumb};
use crate::{Request, Result, request::Body, response};
use http::Method;
use reqx::prelude::{RedirectPolicy, StatusPolicy};
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};

/// Blocking client configuration. Start with [`BlockingClient::builder`].
pub type BlockingClientBuilder = Builder<reqx::advanced::BlockingTransportOptions>;

impl BlockingClientBuilder {
    /// Build a client with the configured authentication and transport.
    pub fn build(self) -> Result<BlockingClient> {
        let builder = self.prepare()?;
        Ok(BlockingClient {
            inner: Arc::new(Inner {
                http: builder.http.build()?,
                config: builder.config,
                crumb: Mutex::new(None),
            }),
        })
    }
}

/// Executes typed requests synchronously without a Tokio runtime.
///
/// Clones share the HTTP pool and crumb cache. Construct requests with [`crate::api`]
/// or [`Request::new`], then pass them to [`Self::execute`].
/// In async applications, call this client on a dedicated thread or use
/// `spawn_blocking` to avoid blocking the executor.
#[derive(Clone)]
pub struct BlockingClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqx::blocking::Client,
    config: Config,
    crumb: Mutex<Option<CachedCrumb>>,
}

impl BlockingClient {
    /// Configure a client for an absolute HTTP(S) Jenkins URL.
    ///
    /// Context paths such as `/jenkins` are supported; a trailing slash is
    /// optional. Credentials, query parameters, and fragments in the URL are
    /// rejected. Supply credentials through [`Builder::auth`].
    pub fn builder(base: impl AsRef<str>) -> Result<BlockingClientBuilder> {
        let base = Config::parse_base_url(base.as_ref())?;
        let http = reqx::blocking::Client::builder(base.as_str());
        Ok(Builder::new(base, http))
    }

    /// Access native reqx requests, streams, and metrics. Raw requests share
    /// credentials and defaults but bypass SDK crumb handling and decoding.
    ///
    /// Use streaming for large artifacts, or increase the buffered response
    /// limit through [`Builder::configure_http`].
    pub fn http_client(&self) -> &reqx::blocking::Client {
        &self.inner.http
    }
    /// The configured Jenkins context root, normalized with a trailing slash.
    pub fn base_url(&self) -> &url::Url {
        &self.inner.config.base_url
    }

    /// Execute a request built by [`crate::api`]. HTTP errors and invalid
    /// response data retain the endpoint context. Text decoding is strict UTF-8.
    pub fn execute<T>(&self, request: Request<T>) -> Result<T> {
        let crumb = match self.inner.config.crumb_ttl {
            Some(ttl) if request.method == Method::POST => Some(self.crumb(ttl)?),
            _ => None,
        };
        self.send(request, crumb)
    }

    fn crumb(&self, ttl: Duration) -> Result<CachedCrumb> {
        // Serialize only refreshes. Mutation I/O occurs after this lock is released.
        let mut cache = self
            .inner
            .crumb
            .lock()
            .map_err(|_| crate::Error::CrumbCachePoisoned)?;
        if let Some(crumb) = cache.as_ref().filter(|crumb| crumb.valid_for(ttl)) {
            return Ok(crumb.clone());
        }
        let crumb = self.send(CachedCrumb::request(), None)?;
        let result = crumb.clone();
        *cache = Some(crumb);
        Ok(result)
    }

    fn send<T>(&self, mut request: Request<T>, crumb: Option<CachedCrumb>) -> Result<T> {
        let url = self.inner.config.request_url(&request);
        let mut http = self
            .inner
            .http
            .request(request.method.clone(), url.as_str())
            .query_pairs(request.query.iter().map(|(key, value)| (key, value)))
            .status_policy(StatusPolicy::Response)
            .redirect_policy(RedirectPolicy::none());
        for (name, value) in &request.headers {
            http = http.header(name.clone(), value.clone());
        }
        if let Some(crumb) = &crumb {
            for (name, value) in &crumb.headers(&url) {
                http = http.header(name.clone(), value.clone());
            }
        }
        match std::mem::replace(&mut request.body, Body::Empty) {
            Body::Empty => {}
            Body::Form(fields) => {
                http = http.form(&fields)?;
            }
            Body::Bytes(bytes) => {
                http = http.body(bytes);
            }
            Body::Multipart(mut form) => {
                if let Some(crumb) = &crumb {
                    let (name, value) = crumb.form_field();
                    form = form.text(name, value)?;
                }
                let (content_type, bytes) = form.encode();
                http = http
                    .header(http::header::CONTENT_TYPE, content_type)
                    .body(bytes);
            }
        }
        let response = http.send()?;
        response::decode(
            &request,
            url,
            response,
            self.inner.config.error_body_limit,
            self.inner.config.auth.as_ref(),
        )
    }
}
