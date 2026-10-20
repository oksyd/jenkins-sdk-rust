//! Jenkins views: request constructors and response types.

use super::{jobs::JobSummary, path};
use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// One view summary entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ViewSummary {
    pub name: String,
    pub url: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// View list payload (for example `GET /api/json?tree=views[name,url]`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ViewList {
    pub views: Vec<ViewSummary>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// View details payload (`GET /view/<name>/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ViewInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub property: Option<Vec<Value>>,
    pub jobs: Option<Vec<JobSummary>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// `GET /api/json?tree=views[name,url]`
pub fn list() -> Request<ViewList> {
    Request::get(["api", "json"], response::json).query("tree", "views[name,url]")
}

/// Update a view's description.
pub fn set_description(name: &str, description: impl Into<String>) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(
        Request::post(["view", name, "submitDescription"], response::empty)
            .form_pairs([("description", description)]),
    )
}

/// Submit a complete view configuration. Renaming uses the `name` field.
/// XML updates preserve the current name; renaming requires this form action.
pub fn configure(name: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(name)?;
    let new_name = form
        .get("name")
        .and_then(Value::as_str)
        .ok_or(crate::Error::InvalidInput {
            field: "view configuration",
            message: "requires a string name field",
        })?;
    path::validate_segment(new_name)?;
    Request::post(["view", name, "configSubmit"], response::empty)
        .query("name", new_name)
        .submitted_form(form)
}

/// `GET /view/<name>/api/json`
pub fn get(name: &str) -> Result<Request<ViewInfo>> {
    path::validate_segment(name)?;

    Ok(Request::get(["view", name, "api", "json"], response::json))
}

/// `POST /createView?name=<name>` with XML body.
pub fn create_from_xml(name: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["createView"], response::empty)
        .query("name", name)
        .xml(xml))
}

/// `POST /view/<name>/doDelete`
pub fn delete(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["view", name, "doDelete"], response::empty))
}

/// `POST /view/<view>/addJobToView?name=<job>`
pub fn add_job(view: &str, job: &str) -> Result<Request<()>> {
    path::validate_segment(view)?;
    path::segments(job)?;

    Ok(Request::post(["view", view, "addJobToView"], response::empty).query("name", job))
}

/// `POST /view/<view>/removeJobFromView?name=<job>`
pub fn remove_job(view: &str, job: &str) -> Result<Request<()>> {
    path::validate_segment(view)?;
    path::segments(job)?;

    Ok(Request::post(["view", view, "removeJobFromView"], response::empty).query("name", job))
}

/// `GET /view/<name>/config.xml`
pub fn get_config_xml(name: &str) -> Result<Request<Vec<u8>>> {
    path::validate_segment(name)?;

    Ok(Request::get(["view", name, "config.xml"], response::bytes))
}

/// `POST /view/<name>/config.xml` with XML body.
pub fn update_config_xml(name: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["view", name, "config.xml"], response::empty).xml(xml))
}
