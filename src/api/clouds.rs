//! Core cloud-management actions. Cloud implementations are supplied by plugins;
//! these routes and their lifecycle are implemented by Jenkins itself.

use super::path;
use crate::{Request, Result, response};
use serde_json::Value;

/// Create a cloud using its descriptor's full structured configuration.
pub fn create(descriptor: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(descriptor)?;
    Request::post(["cloud", "doCreate"], response::empty)
        .query("cloudDescriptorName", descriptor)
        .submitted_form(form)
}

pub fn copy(from: &str, to: &str) -> Result<Request<()>> {
    path::validate_segment(from)?;
    path::validate_segment(to)?;
    Ok(
        Request::post(["cloud", "create"], response::empty).form_pairs([
            ("mode", "copy"),
            ("from", from),
            ("name", to),
        ]),
    )
}

/// Reconfigure or rename a cloud using the descriptor's complete form.
pub fn configure(name: &str, form: &Value) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Request::post(["cloud", name, "configSubmit"], response::empty).submitted_form(form)
}

pub fn delete(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(["cloud", name, "doDelete"], response::empty))
}

/// Set provisioning preference by cloud name. Names omitted here follow the list.
pub fn reorder(names: &[&str]) -> Result<Request<()>> {
    if names.is_empty() {
        return Err(crate::Error::InvalidInput {
            field: "cloud order",
            message: "requires at least one name",
        });
    }
    for name in names {
        path::validate_segment(name)?;
    }
    Ok(Request::post(["cloud", "reorder"], response::empty)
        .form_pairs(names.iter().map(|name| ("name", *name))))
}
