//! Jenkins system: request constructors and response types.

use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// JSON payload of `GET /crumbIssuer/api/json`.
#[derive(Clone, Debug, Deserialize)]
#[non_exhaustive]
pub struct Crumb {
    #[serde(rename = "crumbRequestField")]
    pub crumb_request_field: String,
    pub crumb: String,
}

/// Root payload of `GET /api/json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SystemRoot {
    pub mode: Option<String>,
    pub node_name: Option<String>,
    pub node_description: Option<String>,
    pub num_executors: Option<u32>,
    pub quieting_down: Option<bool>,
    pub use_crumbs: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Payload of `GET /whoAmI/api/json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct WhoAmI {
    pub id: Option<String>,
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub anonymous: Option<bool>,
    pub authenticated: Option<bool>,
    pub authorities: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// `GET /api/json`
pub fn root() -> Request<SystemRoot> {
    Request::get(["api", "json"], response::json)
}

/// `GET /overallLoad/api/json`
pub fn overall_load() -> Request<Value> {
    Request::get(["overallLoad", "api", "json"], response::json)
}

/// `GET /unlabeledLoad/api/json`
pub fn unlabeled_load() -> Request<Value> {
    Request::get(["unlabeledLoad", "api", "json"], response::json)
}

/// `GET /whoAmI/api/json`
pub fn who_am_i() -> Request<WhoAmI> {
    Request::get(["whoAmI", "api", "json"], response::json)
}

/// `GET /crumbIssuer/api/json`
pub fn crumb() -> Request<Crumb> {
    Request::get(["crumbIssuer", "api", "json"], response::json)
}

/// `GET /jnlpJars/agent.jar`
pub fn agent_jar() -> Request<Vec<u8>> {
    Request::get(["jnlpJars", "agent.jar"], response::bytes)
}

/// `GET /jnlpJars/jenkins-cli.jar`
/// The current JAR exceeds reqx's default buffered response limit; configure
/// `max_response_body_bytes` explicitly before downloading it.
pub fn cli_jar() -> Request<Vec<u8>> {
    Request::get(["jnlpJars", "jenkins-cli.jar"], response::bytes)
}

/// `POST /quietDown`
pub fn quiet_down() -> Request<()> {
    Request::post(["quietDown"], response::empty)
}

/// `POST /cancelQuietDown`
pub fn cancel_quiet_down() -> Request<()> {
    Request::post(["cancelQuietDown"], response::empty)
}

/// `POST /reload`
pub fn reload_configuration() -> Request<()> {
    Request::post(["reload"], response::empty)
}

/// `POST /safeRestart`
pub fn safe_restart() -> Request<()> {
    Request::post(["safeRestart"], response::empty)
}

/// `POST /restart`
pub fn restart() -> Request<()> {
    Request::post(["restart"], response::empty)
}

/// `POST /exit`
pub fn exit() -> Request<()> {
    Request::post(["exit"], response::empty)
}

/// Wait for running builds to finish, then shut down the controller.
pub fn safe_exit() -> Request<()> {
    Request::post(["safeExit"], response::empty)
}

/// Execute Groovy on the controller and return its console output.
pub fn script(script: impl Into<String>) -> Request<String> {
    Request::post(["scriptText"], response::text).form_pairs([("script", script)])
}

/// Submit the complete global configuration as Jenkins structured form data.
/// The schema depends on the enabled core descriptors.
pub fn configure(form: &Value) -> Result<Request<()>> {
    Request::post(["configSubmit"], response::empty).submitted_form(form)
}

/// Submit a core global configuration section, for example
/// `configureSecurity`, `configureTools`, or `appearance`.
pub fn configure_section(section: &str, form: &Value) -> Result<Request<()>> {
    super::path::validate_segment(section)?;
    Request::post([section, "configure"], response::empty).submitted_form(form)
}

/// Read `X-Jenkins` from the root resource without downloading its body.
pub fn version() -> Request<String> {
    let mut request = Request::get(["api", "json"], |response, _| {
        Ok(response::required_header(response, "x-jenkins")?.to_owned())
    });
    request.method = http::Method::HEAD;
    request
}

/// Core health check (`GET /health/`), available without authentication.
/// Jenkins returns HTTP 503 when a check fails.
pub fn health() -> Request<Value> {
    Request::get(["health", ""], response::json)
}

/// Start the fingerprint cleanup task.
pub fn cleanup_fingerprints() -> Request<()> {
    Request::post(["fingerprintCleanup"], response::empty)
}

/// Start the unused-workspace cleanup task.
pub fn cleanup_workspaces() -> Request<()> {
    Request::post(["workspaceCleanup"], response::empty)
}

/// Request JVM garbage collection.
pub fn collect_garbage() -> Request<()> {
    Request::post(["gc"], response::empty)
}

/// Download a file from the controller's userContent directory.
pub fn user_content(file: &str) -> Result<Request<Vec<u8>>> {
    let mut segments = vec!["userContent".to_owned()];
    segments.extend(super::path::segments(file)?);
    Ok(Request::get(segments, response::bytes))
}
