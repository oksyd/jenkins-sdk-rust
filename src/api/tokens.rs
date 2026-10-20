//! User API tokens, including expiration supported by Jenkins 2.580.1.

use super::path;
use crate::{Request, Result, response};
use serde::Deserialize;

/// Expiration duration or an ISO date (`YYYY-MM-DD`) interpreted by Jenkins.
#[derive(Clone, Debug)]
pub enum Expiration {
    Never,
    Days(u32),
    Date(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NewToken {
    pub token_uuid: String,
    pub token_name: String,
    /// The secret is returned only when the token is created.
    pub token_value: String,
    /// Jenkins-localized date text, or `never`.
    pub expiration_date: String,
    pub about_to_expire: bool,
}

impl std::fmt::Debug for NewToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewToken")
            .field("token_uuid", &self.token_uuid)
            .field("token_name", &self.token_name)
            .field("token_value", &"<redacted>")
            .field("expiration_date", &self.expiration_date)
            .field("about_to_expire", &self.about_to_expire)
            .finish()
    }
}

fn endpoint(user: &str, action: &str) -> Result<Vec<String>> {
    path::validate_segment(user)?;
    Ok([
        "user",
        user,
        "descriptorByName",
        "jenkins.security.ApiTokenProperty",
        action,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect())
}

fn expiration_fields(expiration: Expiration) -> Vec<(&'static str, String)> {
    match expiration {
        Expiration::Never => vec![("expirationDuration", "never".into())],
        Expiration::Days(days) => vec![("expirationDuration", days.to_string())],
        Expiration::Date(date) => vec![
            ("expirationDuration", "custom".into()),
            ("tokenExpiration", date),
        ],
    }
}

/// Generate a token for the authenticated user. Core restricts generating
/// another user's token even for administrators unless explicitly configured.
pub fn generate(user: &str, name: &str, expiration: Expiration) -> Result<Request<NewToken>> {
    let mut fields = expiration_fields(expiration);
    fields.push(("newTokenName", name.into()));
    Ok(
        Request::post(endpoint(user, "generateNewToken")?, response::jenkins_data)
            .form_pairs(fields),
    )
}

/// Register an externally supplied token. Prefer [`generate`] for normal use.
pub fn add_fixed(
    user: &str,
    name: &str,
    secret: &str,
    expiration: Expiration,
) -> Result<Request<serde_json::Value>> {
    if secret.len() != 34
        || !secret.starts_with("11")
        || !secret.as_bytes()[2..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err(crate::Error::InvalidInput {
            field: "fixed API token",
            message: "expected version 11 followed by 32 hexadecimal characters",
        });
    }
    let mut fields = expiration_fields(expiration);
    fields.extend([
        ("newTokenName", name.into()),
        ("newTokenPlainValue", secret.into()),
    ]);
    Ok(Request::post(endpoint(user, "addFixedToken")?, response::jenkins_data).form_pairs(fields))
}

pub fn rename(user: &str, uuid: &str, new_name: &str) -> Result<Request<()>> {
    Ok(
        Request::post(endpoint(user, "rename")?, response::jenkins_success)
            .form_pairs([("tokenUuid", uuid), ("newName", new_name)]),
    )
}

pub fn revoke(user: &str, uuid: &str) -> Result<Request<()>> {
    Ok(Request::post(endpoint(user, "revoke")?, response::empty).form_pairs([("tokenUuid", uuid)]))
}

pub fn revoke_all(user: &str) -> Result<Request<()>> {
    Ok(Request::post(endpoint(user, "revokeAll")?, response::empty))
}

pub fn revoke_all_except(user: &str, uuid: &str) -> Result<Request<()>> {
    Ok(
        Request::post(endpoint(user, "revokeAllExcept")?, response::empty)
            .form_pairs([("tokenUuid", uuid)]),
    )
}
