//! Artifact fingerprints recorded by Jenkins core.

use crate::{Request, Result, response};
use serde_json::Value;

pub fn get(md5: &str) -> Result<Request<Value>> {
    if md5.len() != 32 || !md5.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(crate::Error::InvalidInput {
            field: "fingerprint",
            message: "expected a 32-digit hexadecimal MD5 hash",
        });
    }
    Ok(Request::get(
        ["fingerprint", &md5.to_ascii_lowercase(), "api", "json"],
        response::json,
    ))
}

/// Upload a file and retain the response's redirect to its fingerprint page.
/// Enable `crumb_ttl` when CSRF protection is active, including API-token auth:
/// this core action explicitly validates a crumb inside the multipart form.
pub fn check(filename: &str, bytes: impl Into<Vec<u8>>) -> Result<Request<reqx::Response>> {
    let form = crate::Multipart::new().file("name", filename, bytes)?;
    Ok(Request::post(["doFingerprintCheck"], |response, _| Ok(response.clone())).multipart(form))
}
