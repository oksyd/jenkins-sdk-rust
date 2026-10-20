//! Shared HTTP status handling, response decoding, and error diagnostics.

use crate::{Auth, DecodeError, Error, HttpError, Request, Result};
use http::StatusCode;
use reqx::Response;
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime};
use url::Url;

pub(crate) fn json<T: DeserializeOwned>(
    response: &Response,
    _: &Url,
) -> std::result::Result<T, DecodeError> {
    Ok(serde_json::from_slice(response.body())?)
}

/// Core management actions use a separate JSON status envelope, even at HTTP 200.
pub(crate) fn jenkins_data<T: DeserializeOwned>(
    response: &Response,
    url: &Url,
) -> std::result::Result<T, DecodeError> {
    let mut value = jenkins_envelope(response, url)?;
    Ok(serde_json::from_value(
        value
            .get_mut("data")
            .map(serde_json::Value::take)
            .ok_or(DecodeError::InvalidResponse("missing Jenkins action data"))?,
    )?)
}

pub(crate) fn jenkins_optional_data<T: DeserializeOwned>(
    response: &Response,
    url: &Url,
) -> std::result::Result<Option<T>, DecodeError> {
    let mut value = jenkins_envelope(response, url)?;
    Ok(value
        .get_mut("data")
        .map(serde_json::Value::take)
        .map(serde_json::from_value)
        .transpose()?)
}

pub(crate) fn jenkins_success(
    response: &Response,
    url: &Url,
) -> std::result::Result<(), DecodeError> {
    jenkins_envelope(response, url)?;
    Ok(())
}

fn jenkins_envelope(
    response: &Response,
    url: &Url,
) -> std::result::Result<serde_json::Value, DecodeError> {
    let value: serde_json::Value = json(response, url)?;
    match value.get("status").and_then(serde_json::Value::as_str) {
        Some("ok") => Ok(value),
        Some("error") => Err(DecodeError::Rejected(
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .ok_or(DecodeError::InvalidResponse(
                    "missing Jenkins error message",
                ))?
                .to_owned(),
        )),
        _ => Err(DecodeError::InvalidResponse(
            "invalid Jenkins action status",
        )),
    }
}

pub(crate) fn text(response: &Response, _: &Url) -> std::result::Result<String, DecodeError> {
    Ok(std::str::from_utf8(response.body())?.to_owned())
}

pub(crate) fn bytes(response: &Response, _: &Url) -> std::result::Result<Vec<u8>, DecodeError> {
    Ok(response.body().to_vec())
}

pub(crate) fn empty(_: &Response, _: &Url) -> std::result::Result<(), DecodeError> {
    Ok(())
}

pub(crate) fn required_header<'a>(
    response: &'a reqx::Response,
    name: &'static str,
) -> std::result::Result<&'a str, crate::DecodeError> {
    response
        .headers()
        .get(name)
        .ok_or(crate::DecodeError::MissingHeader(name))?
        .to_str()
        .map_err(|_| crate::DecodeError::InvalidHeader(name))
}

pub(crate) fn decode<T>(
    request: &Request<T>,
    url: Url,
    response: reqx::Response,
    error_body_limit: Option<usize>,
    auth: Option<&Auth>,
) -> Result<T> {
    let status = response.status();
    // Jenkins POST actions commonly return 302/303 with a Location header.
    let redirect =
        request.action_redirects && matches!(status, StatusCode::FOUND | StatusCode::SEE_OTHER);
    if !status.is_success() && !redirect {
        return Err(Error::Http(Box::new(error_context(
            request,
            url,
            &response,
            error_body_limit,
            auth,
        ))));
    }
    (request.decode)(&response, &url).map_err(|source| Error::Decode {
        response: Box::new(error_context(
            request,
            url,
            &response,
            error_body_limit,
            auth,
        )),
        source,
    })
}

fn error_context<T>(
    request: &Request<T>,
    url: Url,
    response: &reqx::Response,
    error_body_limit: Option<usize>,
    auth: Option<&Auth>,
) -> HttpError {
    let body_snippet = error_body_limit.map(|limit| {
        let mut body = String::from_utf8_lossy(response.body()).into_owned();
        if let Some(auth) = auth
            && !auth.secret().is_empty()
        {
            body = body.replace(auth.secret(), "<redacted>");
        }
        body.truncate(body.floor_char_boundary(limit.min(body.len())));
        body
    });
    let headers = response.headers();
    HttpError {
        status: response.status(),
        method: request.method.clone(),
        url,
        request_id: headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        retry_after: headers
            .get(http::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                let value = value.trim();
                if let Ok(seconds) = value.parse::<u64>() {
                    return Some(Duration::from_secs(seconds));
                }
                httpdate::parse_http_date(value).ok().map(|at| {
                    at.duration_since(SystemTime::now())
                        .unwrap_or(Duration::ZERO)
                })
            }),
        body_snippet,
    }
}
