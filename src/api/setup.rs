//! Initial setup and upgrade wizard actions. Availability depends on install state.

use crate::{Request, response};
use serde_json::Value;

/// Recommended platform plugins, grouped by category.
/// `None` means Jenkins returned no platform data for the current install state.
pub fn platform_plugins() -> Request<Option<Vec<Value>>> {
    Request::get(
        ["setupWizard", "platformPluginList"],
        response::jenkins_optional_data,
    )
}

pub fn restart_status() -> Request<Value> {
    Request::get(["setupWizard", "restartStatus"], response::jenkins_data)
}

/// Set the canonical Jenkins URL during initial setup.
pub fn configure_instance(root_url: &str) -> Request<Value> {
    Request::post(["setupWizard", "configureInstance"], response::jenkins_data)
        .form_pairs([("rootUrl", root_url)])
}

/// Replace the initial administrator during the setup wizard.
/// This changes credentials and the session; build a new client afterwards.
pub fn create_admin(username: &str, password: &str, full_name: &str) -> Request<Value> {
    Request::post(["setupWizard", "createAdminUser"], response::jenkins_data).form_pairs([
        ("username", username),
        ("password1", password),
        ("password2", password),
        ("fullname", full_name),
    ])
}

pub fn complete() -> Request<()> {
    Request::post(
        ["setupWizard", "completeInstall"],
        response::jenkins_success,
    )
}
