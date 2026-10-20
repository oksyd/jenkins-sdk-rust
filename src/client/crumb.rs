use crate::{DecodeError, Request, api::system::Crumb, response};
use cookie_store::CookieStore;
use http::{HeaderMap, HeaderName, HeaderValue, header};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct CachedCrumb {
    name: HeaderName,
    field: String,
    value: HeaderValue,
    fetched_at: Instant,
    cookies: CookieStore,
}

impl CachedCrumb {
    pub fn request() -> Request<Self> {
        Request::get(["crumbIssuer", "api", "json"], |response, url| {
            let crumb: Crumb = response::json(response, url)?;
            let name = HeaderName::from_bytes(crumb.crumb_request_field.as_bytes())
                .map_err(|_| DecodeError::InvalidResponse("invalid crumb header name"))?;
            let mut value = HeaderValue::from_str(&crumb.crumb)
                .map_err(|_| DecodeError::InvalidResponse("invalid crumb header value"))?;
            value
                .to_str()
                .map_err(|_| DecodeError::InvalidResponse("invalid crumb header value"))?;
            value.set_sensitive(true);
            let mut cookies = CookieStore::default();
            for cookie in response.headers().get_all(header::SET_COOKIE) {
                let cookie = cookie
                    .to_str()
                    .map_err(|_| DecodeError::InvalidHeader("set-cookie"))?;
                cookies
                    .parse(cookie, url)
                    .map_err(|_| DecodeError::InvalidHeader("set-cookie"))?;
            }
            Ok(Self {
                name,
                field: crumb.crumb_request_field,
                value,
                fetched_at: Instant::now(),
                cookies,
            })
        })
    }

    pub fn headers(&self, url: &url::Url) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(self.name.clone(), self.value.clone());
        let cookies = self
            .cookies
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if !cookies.is_empty() {
            // CookieStore only accepts cookie syntax from valid header values.
            let mut value = HeaderValue::from_str(&cookies)
                .expect("validated cookies form a valid request header");
            value.set_sensitive(true);
            headers.insert(header::COOKIE, value);
        }
        headers
    }
    pub fn valid_for(&self, ttl: Duration) -> bool {
        self.fetched_at.elapsed() < ttl
    }

    pub fn form_field(&self) -> (&str, &str) {
        (
            &self.field,
            self.value.to_str().expect("crumb validated as ASCII"),
        )
    }
}
