//! Jenkins users: request constructors and response types.

use super::path;
use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// User payload from `GET /user/<id>/api/json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UserInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub description: Option<String>,
    pub absolute_url: Option<String>,
    pub url: Option<String>,
    pub authorities: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// `GET /user/<id>/api/json`
pub fn get(id: &str) -> Result<Request<UserInfo>> {
    path::validate_segment(id)?;

    Ok(Request::get(["user", id, "api", "json"], response::json))
}

pub fn set_description(id: &str, description: impl Into<String>) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(
        Request::post(["user", id, "submitDescription"], response::empty)
            .form_pairs([("description", description)]),
    )
}

/// Delete a user. Jenkins rejects deletion of the currently authenticated user.
pub fn delete(id: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(Request::post(["user", id, "doDelete"], response::empty))
}

/// Configure a user's account, including `fullName` and `description`.
pub fn configure(id: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Request::post(["user", id, "account", "configSubmit"], response::empty).submitted_form(form)
}

/// Configure another core user property category (for example `appearance`
/// or `security`) with that category's full structured form.
pub fn configure_category(id: &str, category: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(id)?;
    path::validate_segment(category)?;
    Request::post(["user", id, category, "configSubmit"], response::empty).submitted_form(form)
}

/// Create an account in Jenkins' built-in user database as an administrator.
/// Other security realms do not expose this action.
pub fn create_local(id: &str, password: &str, full_name: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(
        Request::post(["securityRealm", "createAccountByAdmin"], |response, _| {
            if response.status() == http::StatusCode::FOUND {
                Ok(())
            } else {
                Err(crate::DecodeError::InvalidResponse(
                    "Jenkins returned an account validation form",
                ))
            }
        })
        .form_pairs([
            ("username", id),
            ("password1", password),
            ("password2", password),
            ("fullname", full_name),
        ]),
    )
}

/// Invalidate this user's existing browser sessions.
pub fn renew_session_seed(id: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(Request::post(
        [
            "user",
            id,
            "descriptorByName",
            "jenkins.security.seed.UserSeedProperty",
            "renewSessionSeed",
        ],
        response::empty,
    ))
}
