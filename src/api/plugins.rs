//! Jenkins core's plugin manager. Plugin-contributed APIs are separate.

use super::path;
use crate::{Multipart, Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Plugin {
    pub short_name: String,
    pub version: Option<String>,
    pub long_name: Option<String>,
    pub active: Option<bool>,
    pub enabled: Option<bool>,
    pub has_update: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PluginList {
    pub plugins: Vec<Plugin>,
}

pub fn list() -> Request<PluginList> {
    Request::get(["pluginManager", "api", "json"], response::json).query("depth", "1")
}

pub fn get(name: &str) -> Result<Request<Plugin>> {
    path::validate_segment(name)?;
    Ok(Request::get(
        ["pluginManager", "plugin", name, "api", "json"],
        response::json,
    ))
}

/// Installed and available plugins, using the management action's data schema.
pub fn available() -> Request<Vec<Value>> {
    Request::get(["pluginManager", "plugins"], response::jenkins_data)
}

pub fn search(query: &str, limit: u32) -> Request<Vec<Value>> {
    Request::get(["pluginManager", "pluginsSearch"], response::jenkins_data)
        .query("query", query)
        .query("limit", limit.to_string())
}

/// Install plugins from configured update sites. Entries use `shortName` or
/// `shortName.siteId`. Returns the correlation ID in the management response data.
pub fn install(names: &[&str], dynamic_load: bool) -> Result<Request<Value>> {
    Request::post(["pluginManager", "installPlugins"], response::jenkins_data)
        .json_body(&json!({"plugins": names, "dynamicLoad": dynamic_load}))
}

/// Install dependencies specified by `plugin="name@version"` attributes in XML.
pub fn install_required(xml: impl Into<Vec<u8>>) -> Request<()> {
    Request::post(
        ["pluginManager", "installNecessaryPlugins"],
        response::empty,
    )
    .xml(xml)
}

/// Inspect missing/outdated plugin dependencies before submitting a configuration.
pub fn prevalidate_config(xml: impl Into<Vec<u8>>) -> Request<Vec<Value>> {
    Request::post(["pluginManager", "prevalidateConfig"], response::json).xml(xml)
}

/// Upload an HPI/JPI archive. Jenkins expects the file followed by the URL field.
pub fn upload(filename: &str, archive: impl Into<Vec<u8>>) -> Result<Request<()>> {
    let form = Multipart::new()
        .file("file", filename, archive)?
        .text("url", "")?;
    Ok(Request::post(["pluginManager", "uploadPlugin"], response::empty).multipart(form))
}

/// Ask Jenkins to download an HPI/JPI from the supplied URL.
pub fn upload_from_url(url: &str) -> Result<Request<()>> {
    let form = Multipart::new()
        .file("file", "unused.jpi", Vec::new())?
        .text("url", url)?;
    Ok(Request::post(["pluginManager", "uploadPlugin"], response::empty).multipart(form))
}

pub fn enable(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(
        ["pluginManager", "plugin", name, "makeEnabled"],
        response::empty,
    ))
}

pub fn disable(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(
        ["pluginManager", "plugin", name, "makeDisabled"],
        response::empty,
    ))
}

pub fn uninstall(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(
        ["pluginManager", "plugin", name, "doUninstall"],
        response::empty,
    ))
}

pub fn check_updates() -> Request<()> {
    Request::post(["pluginManager", "checkUpdatesServer"], response::empty)
}

/// Configure the default update-site URL.
pub fn configure_site(url: &str) -> Request<()> {
    Request::post(["pluginManager", "siteConfigure"], response::empty).form_pairs([("site", url)])
}

/// Remove update sites by ID.
pub fn remove_sites(ids: &[&str]) -> Request<()> {
    Request::post(["pluginManager", "updateSources"], response::empty).form_pairs(
        std::iter::once(("remove", "true")).chain(ids.iter().map(|id| ("sources", *id))),
    )
}

/// Configure Jenkins' outbound proxy using its structured form schema.
pub fn configure_proxy(form: &Value) -> Result<Request<()>> {
    Request::post(["pluginManager", "proxyConfigure"], response::empty).submitted_form(form)
}

pub fn finish_installation() -> Request<()> {
    Request::post(["pluginManager", "installPluginsDone"], response::empty)
}
