//! Jenkins computers: request constructors and response types.

use super::path;
use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Computers list payload (`GET /computer/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ComputerList {
    pub display_name: Option<String>,
    pub busy_executors: Option<u32>,
    pub total_executors: Option<u32>,
    pub computer: Vec<ComputerInfo>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Computer details payload (`GET /computer/<name>/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ComputerInfo {
    pub display_name: Option<String>,
    pub offline: Option<bool>,
    pub temporarily_offline: Option<bool>,
    pub idle: Option<bool>,
    pub num_executors: Option<u32>,
    pub monitor_data: Option<Value>,
    #[serde(rename = "_class")]
    pub class_name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Executor counters from `GET /computer/api/json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ExecutorsInfo {
    pub total_executors: u32,
    pub busy_executors: u32,
}

impl ExecutorsInfo {
    /// Returns None if the server's snapshot reports more busy than total executors.
    pub fn idle_executors(&self) -> Option<u32> {
        self.total_executors.checked_sub(self.busy_executors)
    }
}

/// `GET /computer/api/json`
pub fn list() -> Request<ComputerList> {
    Request::get(["computer", "api", "json"], response::json)
}

/// `POST /computer/createItem?name=<name>` with XML body.
pub fn create_from_xml(name: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["computer", "createItem"], response::empty)
        .query("name", name)
        .xml(xml))
}

/// `POST /computer/createItem?name=<to>&mode=copy&from=<from>`
pub fn copy(from: &str, to: &str) -> Result<Request<()>> {
    path::validate_segment(from)?;
    path::validate_segment(to)?;

    Ok(Request::post(["computer", "createItem"], response::empty)
        .query("name", to)
        .query("mode", "copy")
        .query("from", from))
}

/// `GET /computer/api/json` (typed subset).
pub fn executors_info() -> Request<ExecutorsInfo> {
    Request::get(["computer", "api", "json"], response::json)
}

/// `GET /computer/<name>/api/json`
pub fn get(name: &str) -> Result<Request<ComputerInfo>> {
    path::validate_segment(name)?;

    Ok(Request::get(
        ["computer", name, "api", "json"],
        response::json,
    ))
}

/// `POST /computer/<name>/toggleOffline`
pub fn toggle_offline(name: &str, offline_message: Option<&str>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    let mut req = Request::post(["computer", name, "toggleOffline"], response::empty);
    if let Some(message) = offline_message {
        req = req.query("offlineMessage", message);
    }
    Ok(req)
}

/// `POST /computer/<name>/doDelete`
pub fn delete(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(
        ["computer", name, "doDelete"],
        response::empty,
    ))
}

/// `GET /computer/<name>/config.xml`
pub fn get_config_xml(name: &str) -> Result<Request<Vec<u8>>> {
    path::validate_segment(name)?;

    Ok(Request::get(
        ["computer", name, "config.xml"],
        response::bytes,
    ))
}

/// `POST /computer/<name>/config.xml` with XML body.
pub fn update_config_xml(name: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["computer", name, "config.xml"], response::empty).xml(xml))
}

/// `POST /computer/<name>/doDisconnect`
pub fn disconnect(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(
        ["computer", name, "doDisconnect"],
        response::empty,
    ))
}

/// `POST /computer/<name>/launchSlaveAgent`
pub fn launch_slave_agent(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(
        ["computer", name, "launchSlaveAgent"],
        response::empty,
    ))
}

/// Change the reason associated with a temporarily offline node.
pub fn change_offline_cause(name: &str, message: impl Into<String>) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(
        Request::post(["computer", name, "changeOfflineCause"], response::empty)
            .form_pairs([("offlineMessage", message)]),
    )
}

/// Update an agent's description.
pub fn set_description(name: &str, description: impl Into<String>) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(
        Request::post(["computer", name, "submitDescription"], response::empty)
            .form_pairs([("description", description)]),
    )
}

/// Execute Groovy on an online agent and return its output.
pub fn script(name: &str, script: impl Into<String>) -> Result<Request<String>> {
    path::validate_segment(name)?;
    Ok(
        Request::post(["computer", name, "scriptText"], response::text)
            .form_pairs([("script", script)]),
    )
}

/// Read the agent connection log starting at a byte offset.
pub fn progressive_log(name: &str, start: u64) -> Result<Request<super::builds::ProgressiveText>> {
    path::validate_segment(name)?;
    Ok(Request::get(
        ["computer", name, "logText", "progressiveText"],
        super::builds::decode_progressive_text,
    )
    .query("start", start.to_string()))
}

/// Refresh disk, response-time, and other node monitors.
pub fn update_monitors() -> Request<()> {
    Request::post(["computer", "updateNow"], response::empty)
}

/// Download an inbound agent's JNLP launch descriptor.
pub fn agent_jnlp(name: &str) -> Result<Request<Vec<u8>>> {
    path::validate_segment(name)?;
    Ok(Request::get(
        ["computer", name, "jenkins-agent.jnlp"],
        response::bytes,
    ))
}

/// Dump the remoting export table for diagnostics.
pub fn export_table(name: &str) -> Result<Request<String>> {
    path::validate_segment(name)?;
    Ok(Request::get(
        ["computer", name, "dumpExportTable"],
        response::text,
    ))
}
