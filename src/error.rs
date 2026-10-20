use http::{Method, StatusCode};
use std::{fmt, time::Duration};
use thiserror::Error;
use url::Url;

/// Result of request construction, client configuration, or Jenkins execution.
pub type Result<T> = std::result::Result<T, Error>;

/// Context shared by HTTP failures and invalid Jenkins responses.
#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub method: Method,
    /// Request URL without query parameters or credentials.
    pub url: Url,
    pub request_id: Option<String>,
    pub retry_after: Option<Duration>,
    /// Bounded response text with the configured authentication secret redacted.
    /// Defaults to at most 4096 bytes; configure with
    /// [`Builder::error_body_limit`](crate::Builder::error_body_limit).
    pub body_snippet: Option<String>,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HTTP {} during {} {}",
            self.status,
            self.method,
            self.url.path()
        )
    }
}

impl std::error::Error for HttpError {}

/// Why a successful HTTP response could not be interpreted.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DecodeError {
    #[error("Jenkins rejected the action: {0}")]
    Rejected(String),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("missing required response header {0}")]
    MissingHeader(&'static str),
    #[error("invalid response header {0}")]
    InvalidHeader(&'static str),
    #[error("{0}")]
    InvalidResponse(&'static str),
}

/// SDK errors keep HTTP status, response decoding, and transport failures distinct.
/// Retry eligibility and transport error classification belong to reqx.
///
/// - [`Self::Transport`] retains the native reqx error, including timeout phases
///   and response-size failures.
/// - [`Self::Http`] reports a status rejected by the request's status policy.
/// - [`Self::Decode`] reports malformed JSON, invalid UTF-8, missing metadata, or
///   a rejected Jenkins management action, even when its HTTP status is 200.
/// - [`Self::Serialize`] reports request-serialization failures.
///
/// [`Self::response`] returns status, method, URL without query parameters,
/// request ID, Retry-After, and an optional body snippet when available.
/// [`Builder::error_body_limit`](crate::Builder::error_body_limit) controls only
/// captured snippets; transport and parser errors retain their original sources.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("could not serialize request: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error(transparent)]
    Transport(#[from] Box<reqx::Error>),
    #[error(transparent)]
    Http(#[from] Box<HttpError>),
    #[error("{response}: {source}")]
    Decode {
        response: Box<HttpError>,
        #[source]
        source: DecodeError,
    },
    #[error("invalid {field}: {message}")]
    InvalidInput {
        field: &'static str,
        message: &'static str,
    },
    #[error("invalid base URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("invalid authorization header: {0}")]
    InvalidAuth(#[from] http::header::InvalidHeaderValue),
    #[error("crumb cache lock was poisoned by a panic")]
    CrumbCachePoisoned,
}

impl From<reqx::Error> for Error {
    fn from(error: reqx::Error) -> Self {
        Self::Transport(Box::new(error))
    }
}

impl Error {
    /// The response context, when an HTTP response was received.
    pub fn response(&self) -> Option<&HttpError> {
        match self {
            Self::Http(response) | Self::Decode { response, .. } => Some(response),
            _ => None,
        }
    }
}
