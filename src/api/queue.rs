//! Jenkins queue: request constructors and response types.

use crate::{Request, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Queue list payload (`GET /queue/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QueueList {
    pub items: Vec<QueueItem>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Queue item payload (`GET /queue/item/<id>/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct QueueItem {
    pub id: Option<u64>,
    pub blocked: Option<bool>,
    pub buildable: Option<bool>,
    pub stuck: Option<bool>,
    pub cancelled: Option<bool>,
    pub why: Option<String>,
    pub params: Option<String>,
    pub in_queue_since: Option<u64>,
    pub task: Option<QueueTask>,
    pub executable: Option<QueueExecutable>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Minimal task shape nested in queue items.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct QueueTask {
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "_class")]
    pub class_name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Build pointer nested in queue items when executable already started.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct QueueExecutable {
    pub number: Option<u64>,
    pub url: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// `GET /queue/api/json`
pub fn list() -> Request<QueueList> {
    Request::get(["queue", "api", "json"], response::json)
}

/// `GET /queue/item/<id>/api/json`
pub fn item(id: u64) -> Request<QueueItem> {
    Request::get(
        ["queue", "item", &id.to_string(), "api", "json"],
        response::json,
    )
}

/// `POST /queue/cancelItem?id=<id>`
pub fn cancel(id: u64) -> Request<()> {
    Request::post(["queue", "cancelItem"], response::empty).query("id", id.to_string())
}
