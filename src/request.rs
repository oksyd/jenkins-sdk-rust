//! Transport-independent typed request construction.

use crate::{DecodeError, Result, response};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use reqx::Response;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt;
use url::Url;

pub(crate) enum Body {
    Empty,
    Form(Vec<(String, String)>),
    Bytes(Vec<u8>),
    Multipart(crate::Multipart),
}

/// A typed Jenkins request, independent of the I/O mode.
///
/// Construct requests with functions in [`crate::api`], then pass them to
/// `Client::execute` or `BlockingClient::execute`. Only execution performs I/O.
///
/// Use [`Self::tree`] for field selection, [`Self::query`] to replace a query
/// parameter, and [`Self::append_query`] to preserve repeated parameters.
/// Body methods select the endpoint's wire format; Jenkins structured form
/// submissions use [`Self::submitted_form`] rather than [`Self::json_body`].
///
/// Execution accepts 2xx statuses. Named POST actions also accept 302/303
/// redirects without following them, preserving queue `Location` headers.
/// Custom requests opt in with [`Self::allow_action_redirects`]. Other statuses
/// produce [`crate::Error::Http`]. Core management decoders also reject JSON
/// envelopes with `status: "error"`, even when the HTTP status is 200.
///
/// Decoders can be changed with [`Self::decode_json`], [`Self::decode_text`],
/// [`Self::decode_bytes`], [`Self::discard_body`], or [`Self::raw_response`].
/// Responses are buffered and subject to reqx's configured size limit; use the
/// client's `http_client()` for native streaming.
#[must_use = "a request must be passed to a client's execute method"]
pub struct Request<T> {
    pub(crate) method: Method,
    pub(crate) segments: Vec<String>,
    pub(crate) query: Vec<(String, String)>,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Body,
    pub(crate) action_redirects: bool,
    pub(crate) decode: fn(&Response, &Url) -> std::result::Result<T, DecodeError>,
}

impl<T> fmt::Debug for Request<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("segments", &self.segments)
            .finish_non_exhaustive()
    }
}

impl Request<Response> {
    /// Construct a request to a path relative to the Jenkins context root.
    ///
    /// Pass **unescaped segments**, not a URL. Each segment is encoded exactly
    /// once. Credentials, crumb handling, status checking, and transport limits
    /// are the same as for named endpoints. The response retains headers and body.
    /// An empty iterator selects the context root; a final empty segment requests
    /// a trailing slash. Segments containing `/`, `\`, or control characters,
    /// and segments equal to `.` or `..`, are rejected.
    ///
    /// ```
    /// use jenkins_sdk::{Request, http::Method};
    /// let request = Request::new(Method::GET, ["label", "linux", "api", "json"])?
    ///     .decode_json::<serde_json::Value>()
    ///     .tree("name,nodes[nodeName]");
    /// # Ok::<(), jenkins_sdk::Error>(())
    /// ```
    pub fn new<I, S>(method: Method, segments: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut request = Self::get(segments, |response, _| Ok(response.clone()));
        for (index, segment) in request.segments.iter().enumerate() {
            // A final empty segment explicitly requests a trailing slash.
            if index + 1 != request.segments.len() || !segment.is_empty() {
                crate::api::path::validate_segment(segment)?;
            }
        }
        request.method = method;
        Ok(request)
    }
}

impl<T> Request<T> {
    pub(crate) fn get<I, S>(
        segments: I,
        decode: fn(&Response, &Url) -> std::result::Result<T, DecodeError>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            method: Method::GET,
            segments: segments.into_iter().map(Into::into).collect(),
            query: Vec::new(),
            headers: HeaderMap::new(),
            body: Body::Empty,
            action_redirects: false,
            decode,
        }
    }

    pub(crate) fn post<I, S>(
        segments: I,
        decode: fn(&Response, &Url) -> std::result::Result<T, DecodeError>,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            method: Method::POST,
            action_redirects: true,
            ..Self::get(segments, decode)
        }
    }

    /// Add a Jenkins query parameter, replacing an existing value of that name.
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.query.retain(|(key, _)| key != &name);
        self.query.push((name, value.into()));
        self
    }

    /// Append a parameter without removing earlier values (for example XML `exclude`).
    pub fn append_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((name.into(), value.into()));
        self
    }

    /// Select Jenkins JSON fields, for example `items[id,why]`.
    pub fn tree(self, fields: impl Into<String>) -> Self {
        self.query("tree", fields)
    }

    /// Send URL-encoded form fields, preserving repeated names.
    pub fn form_pairs<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.body = Body::Form(
            pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }

    /// Send an XML document.
    pub fn xml(self, xml: impl Into<Vec<u8>>) -> Self {
        self.body(HeaderValue::from_static("application/xml"), xml)
    }

    /// Send an ordered multipart form, including file parts.
    pub fn multipart(mut self, form: crate::Multipart) -> Self {
        self.body = Body::Multipart(form);
        self
    }

    /// Send a JSON document. Jenkins `configSubmit` actions instead expect
    /// a URL-encoded `json` field; use [`Self::submitted_form`] for those.
    pub fn json_body(self, value: &impl Serialize) -> Result<Self> {
        Ok(self.body(
            HeaderValue::from_static("application/json"),
            serde_json::to_vec(value).map_err(crate::Error::Serialize)?,
        ))
    }

    /// Encode Jenkins' structured form submission protocol.
    pub fn submitted_form(self, value: &impl Serialize) -> Result<Self> {
        Ok(self.form_pairs([(
            "json",
            serde_json::to_string(value).map_err(crate::Error::Serialize)?,
        )]))
    }

    /// Send bytes with an explicit media type.
    pub fn body(mut self, content_type: HeaderValue, bytes: impl Into<Vec<u8>>) -> Self {
        self.headers
            .insert(http::header::CONTENT_TYPE, content_type);
        self.body = Body::Bytes(bytes.into());
        self
    }

    /// Set a request header. Client-managed crumb/session headers take precedence.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Accept Jenkins action redirects (302 and 303) without following them.
    /// Named POST actions already enable this.
    pub fn allow_action_redirects(mut self) -> Self {
        self.action_redirects = true;
        self
    }

    /// Decode successful responses using Serde, including caller-defined models.
    pub fn decode_json<U: DeserializeOwned>(self) -> Request<U> {
        self.with_decoder(response::json)
    }

    /// Decode successful responses as strict UTF-8.
    pub fn decode_text(self) -> Request<String> {
        self.with_decoder(response::text)
    }

    /// Return the buffered response bytes.
    pub fn decode_bytes(self) -> Request<Vec<u8>> {
        self.with_decoder(response::bytes)
    }

    /// Discard a successful response body.
    pub fn discard_body(self) -> Request<()> {
        self.with_decoder(response::empty)
    }

    /// Retain status, headers, and body instead of the endpoint's default decoder.
    pub fn raw_response(self) -> Request<Response> {
        self.with_decoder(|response, _| Ok(response.clone()))
    }

    fn with_decoder<U>(
        self,
        decode: fn(&Response, &Url) -> std::result::Result<U, DecodeError>,
    ) -> Request<U> {
        Request {
            method: self.method,
            segments: self.segments,
            query: self.query,
            headers: self.headers,
            body: self.body,
            action_redirects: self.action_redirects,
            decode,
        }
    }
}
