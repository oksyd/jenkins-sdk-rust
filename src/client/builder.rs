use super::config::Config;
use crate::{Auth, Result};
use http::HeaderValue;
use std::time::Duration;
use url::Url;

/// Jenkins-specific configuration around a native reqx builder.
///
/// Create a builder with `Client::builder` or `BlockingClient::builder`.
/// Use [`Self::auth`] for credentials and [`Self::configure_http`] /
/// [`Self::try_configure_http`] for HTTP settings. No environment variables are
/// read by the SDK. Retries and CSRF crumb fetching are disabled by default.
#[must_use]
pub struct Builder<T> {
    pub(super) http: reqx::ClientBuilder<T>,
    pub(super) config: Config,
}

impl<T> Builder<T> {
    pub(super) fn new(base: Url, http: reqx::ClientBuilder<T>) -> Self {
        Self {
            http: http
                .client_name(env!("CARGO_PKG_NAME"))
                .default_header(
                    http::header::USER_AGENT,
                    HeaderValue::from_static(concat!(
                        env!("CARGO_PKG_NAME"),
                        "/",
                        env!("CARGO_PKG_VERSION")
                    )),
                )
                .retry_policy(reqx::prelude::RetryPolicy::disabled()),
            config: Config {
                base_url: base,
                auth: None,
                crumb_ttl: None,
                error_body_limit: Some(4096),
            },
        }
    }

    /// Apply credentials to typed and raw requests; redact the token in captured response snippets.
    /// Credentials are applied at build time, after HTTP builder configuration.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.config.auth = Some(auth);
        self
    }

    /// Enable CSRF crumbs and cache them for this duration across client clones.
    ///
    /// Before POST requests, fetch a crumb and keep the session cookies that
    /// issued it. Cookie domain, path, expiry, and Secure rules are respected.
    /// Concurrent refreshes are serialized; mutation requests are not.
    /// Multipart requests also carry the crumb as a form field.
    ///
    /// API-token authentication normally needs no crumb; core fingerprint file
    /// verification is an exception. Crumb failures propagate. A rejected
    /// mutation is not automatically replayed with a new crumb. Build a new
    /// client after replacing credentials or invalidating its session.
    pub fn crumb_ttl(mut self, ttl: Duration) -> Self {
        self.config.crumb_ttl = Some(ttl);
        self
    }

    /// Bound captured error bodies, or disable capture with `None`.
    ///
    /// The default is 4096 bytes. The configured authentication secret is
    /// redacted before truncation. This setting controls only
    /// [`HttpError::body_snippet`](crate::HttpError::body_snippet); transport and
    /// JSON parser errors keep their original sources.
    pub fn error_body_limit(mut self, bytes: Option<usize>) -> Self {
        self.config.error_body_limit = bytes;
        self
    }

    /// Configure reqx timeouts, retries, TLS, response limits, and other policies.
    ///
    /// ```
    /// # #[cfg(feature = "async")]
    /// # fn example() -> jenkins_sdk::Result<()> {
    /// use jenkins_sdk::{Client, reqx::prelude::RetryPolicy};
    /// use std::time::Duration;
    ///
    /// let client = Client::builder("https://jenkins.example.com/jenkins")?
    ///     .configure_http(|http| http
    ///         .request_timeout(Duration::from_secs(10))
    ///         .total_timeout(Some(Duration::from_secs(30)))
    ///         .retry_policy(RetryPolicy::standard().max_attempts(3))
    ///         .max_response_body_bytes(64 * 1024 * 1024))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Retries are disabled by default. `RetryPolicy::standard()` retries
    /// eligible idempotent requests; it does not replay POST actions.
    /// `max_attempts` includes the first attempt. A total timeout bounds one
    /// HTTP request including retries; a crumb fetch has its own deadline.
    /// Other transport defaults come from reqx.
    ///
    /// SDK requests disable redirect following and check final statuses
    /// themselves; see [`Request`](crate::Request). Native retry policy still
    /// applies. Native requests through `http_client()` honor all reqx settings
    /// and support streaming, but bypass SDK crumb handling and decoding.
    ///
    /// Use [`Self::try_configure_http`] for fallible settings. Configure proxies
    /// explicitly with reqx's `http_proxy`; the SDK does not read proxy
    /// environment variables.
    pub fn configure_http(
        mut self,
        configure: impl FnOnce(reqx::ClientBuilder<T>) -> reqx::ClientBuilder<T>,
    ) -> Self {
        self.http = configure(self.http);
        self
    }

    /// Apply a fallible reqx setting such as a proxy rule or DNS override.
    ///
    /// For example: `.try_configure_http(|http| http.no_proxy(["localhost"]))?`.
    pub fn try_configure_http(
        mut self,
        configure: impl FnOnce(reqx::ClientBuilder<T>) -> reqx::Result<reqx::ClientBuilder<T>>,
    ) -> Result<Self> {
        self.http = configure(self.http)?;
        Ok(self)
    }

    pub(super) fn prepare(mut self) -> Result<Self> {
        if let Some(auth) = &self.config.auth {
            let (name, value) = auth.header()?;
            self.http = self.http.default_header(name, value);
        }
        Ok(self)
    }
}
