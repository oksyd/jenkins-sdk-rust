//! Client-wide Jenkins settings and base URL validation.

use crate::{Auth, Error, Request, Result};
use std::time::Duration;
use url::Url;

pub(super) struct Config {
    pub(super) base_url: Url,
    pub(super) auth: Option<Auth>,
    pub(super) crumb_ttl: Option<Duration>,
    pub(super) error_body_limit: Option<usize>,
}

impl Config {
    pub(super) fn parse_base_url(raw: &str) -> Result<Url> {
        let mut url = Url::parse(raw)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(Error::InvalidInput {
                field: "base URL",
                message: "expected an absolute HTTP(S) URL",
            });
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(Error::InvalidInput {
                field: "base URL",
                message: "credentials, query parameters, and fragments are not allowed",
            });
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        Ok(url)
    }

    pub(super) fn request_url<T>(&self, request: &Request<T>) -> Url {
        let mut url = self.base_url.clone();
        if request.segments.is_empty() {
            return url;
        }
        // The constructor guarantees a hierarchical HTTP(S) base URL.
        url.path_segments_mut()
            .expect("HTTP(S) base URL")
            .pop_if_empty()
            .extend(&request.segments);
        url
    }
}
