//! Label expressions, label atoms, and their load statistics.

use super::path;
use crate::{Request, Result, response};
use serde_json::Value;

/// Read a label expression, for example `linux && amd64`.
pub fn get(expression: &str) -> Result<Request<Value>> {
    path::validate_segment(expression)?;
    Ok(Request::get(
        ["label", expression, "api", "json"],
        response::json,
    ))
}

pub fn load_statistics(expression: &str) -> Result<Request<Value>> {
    path::validate_segment(expression)?;
    Ok(Request::get(
        ["label", expression, "loadStatistics", "api", "json"],
        response::json,
    ))
}

/// Update a single label atom's description.
pub fn set_description(atom: &str, description: impl Into<String>) -> Result<Request<()>> {
    path::validate_segment(atom)?;
    Ok(
        Request::post(["label", atom, "submitDescription"], response::empty)
            .form_pairs([("description", description)]),
    )
}

/// Configure a label atom's description and properties.
pub fn configure(atom: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(atom)?;
    Request::post(["label", atom, "configSubmit"], response::empty).submitted_form(form)
}
