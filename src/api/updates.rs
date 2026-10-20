//! Update-center metadata, installation progress, and controller upgrades.

use super::path;
use crate::{Request, Result, response};
use serde_json::Value;

pub fn get() -> Request<Value> {
    Request::get(["updateCenter", "api", "json"], response::json)
}

pub fn job(id: u64) -> Request<Value> {
    Request::get(
        ["updateCenter", "job", &id.to_string(), "api", "json"],
        response::json,
    )
}

pub fn site(id: &str) -> Result<Request<Value>> {
    path::validate_segment(id)?;
    Ok(Request::get(
        ["updateCenter", "site", id, "api", "json"],
        response::json,
    ))
}

/// Plugin metadata advertised by a particular update site.
pub fn plugin(site: &str, name: &str) -> Result<Request<Value>> {
    path::validate_segment(site)?;
    path::validate_segment(name)?;
    Ok(Request::get(
        ["updateCenter", "site", site, "plugin", name, "api", "json"],
        response::json,
    ))
}

/// Install a plugin from a specific site, either dynamically or after restart.
pub fn install_plugin(site: &str, name: &str, dynamic_load: bool) -> Result<Request<()>> {
    path::validate_segment(site)?;
    path::validate_segment(name)?;
    let action = if dynamic_load {
        "installNow"
    } else {
        "install"
    };
    Ok(Request::post(
        ["updateCenter", "site", site, "plugin", name, action],
        response::empty,
    ))
}

/// Restore the installed plugin's backup from before its last upgrade.
pub fn downgrade_plugin(site: &str, name: &str) -> Result<Request<()>> {
    path::validate_segment(site)?;
    path::validate_segment(name)?;
    Ok(Request::post(
        ["updateCenter", "site", site, "plugin", name, "downgrade"],
        response::empty,
    ))
}

pub fn connection_status(site_id: &str) -> Request<Value> {
    Request::get(["updateCenter", "connectionStatus"], response::jenkins_data)
        .query("siteId", site_id)
}

/// Filter by `.query("correlationId", id)` to track a particular installation.
pub fn installation_status() -> Request<Value> {
    Request::get(["updateCenter", "installStatus"], response::jenkins_data)
}

pub fn incomplete_installations() -> Request<Value> {
    Request::get(
        ["updateCenter", "incompleteInstallStatus"],
        response::jenkins_data,
    )
}

pub fn invalidate() -> Request<()> {
    Request::post(["updateCenter", "invalidateData"], response::empty)
}

pub fn invalidate_site(id: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(Request::post(
        ["updateCenter", "site", id, "invalidateData"],
        response::empty,
    ))
}

/// Verify the selected update site's metadata signature; returns validation HTML.
pub fn verify_site_signature(id: &str) -> Result<Request<String>> {
    path::validate_segment(id)?;
    Ok(Request::post(
        ["updateCenter", "site", id, "verifySignature"],
        response::text,
    ))
}

/// Upgrade the controller WAR when the deployment supports in-place upgrades.
pub fn upgrade_core() -> Request<()> {
    Request::post(["updateCenter", "upgrade"], response::empty)
}

pub fn downgrade_core() -> Request<()> {
    Request::post(["updateCenter", "downgrade"], response::empty)
}

pub fn safe_restart() -> Request<()> {
    Request::post(["updateCenter", "safeRestart"], response::empty)
}

pub fn cancel_restart() -> Request<()> {
    Request::post(["updateCenter", "cancelRestart"], response::empty)
}

pub fn restart() -> Request<()> {
    Request::post(["updateCenter", "restart"], response::empty)
}
