//! Jenkins jobs: request constructors and response types.

use super::path;
use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// One job entry in `.../api/json` results.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct JobSummary {
    pub name: String,
    pub url: Option<String>,
    pub color: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Job list payload (for example `GET /api/json?tree=jobs[...]`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JobList {
    pub jobs: Vec<JobSummary>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Job-level API payload (for example `GET /job/<name>/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct JobInfo {
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub display_name: Option<String>,
    pub url: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub buildable: Option<bool>,
    pub in_queue: Option<bool>,
    pub next_build_number: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result of triggering a build.
///
/// Submission must return a valid queue `Location` header. A missing or
/// malformed location produces [`crate::Error::Decode`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TriggeredBuild {
    /// Queue item id parsed from the `Location` header.
    pub queue_item_id: u64,
    /// Resolved queue item URL from the `Location` header.
    pub location: url::Url,
}

/// `GET /api/json?tree=jobs[name,url,color]`
pub fn list() -> Request<JobList> {
    Request::get(["api", "json"], response::json).query("tree", "jobs[name,url,color]")
}

/// Retrieve the plain-text job description.
pub fn description(job: &str) -> Result<Request<String>> {
    Ok(Request::get(
        path::job_segments(job, &["description"])?,
        response::text,
    ))
}

/// Schedule a parameterized build with file and text fields.
/// File field names must match the job's core FileParameterDefinition names.
pub fn build_with_files(job: &str, form: crate::Multipart) -> Result<Request<TriggeredBuild>> {
    Ok(Request::post(
        path::job_segments(job, &["buildWithParameters"])?,
        decode_triggered_build,
    )
    .multipart(form))
}

/// Reload the job configuration from disk.
pub fn reload(job: &str) -> Result<Request<()>> {
    Ok(Request::post(
        path::job_segments(job, &["reload"])?,
        response::empty,
    ))
}

/// Cancel queued builds for this job.
pub fn cancel_queue(job: &str) -> Result<Request<()>> {
    Ok(Request::post(
        path::job_segments(job, &["cancelQueue"])?,
        response::empty,
    ))
}

/// Trigger SCM polling on a core AbstractProject.
pub fn poll(job: &str) -> Result<Request<()>> {
    Ok(Request::post(
        path::job_segments(job, &["polling"])?,
        response::empty,
    ))
}

/// Read a core SCMTrigger's rendered polling log page.
/// The job must have that trigger configured; this resource exposes HTML.
pub fn polling_log(job: &str) -> Result<Request<String>> {
    Ok(Request::get(
        path::job_segments(job, &["scmPollLog", ""])?,
        response::text,
    ))
}

/// Wipe the job's workspace.
pub fn wipe_workspace(job: &str) -> Result<Request<()>> {
    Ok(Request::post(
        path::job_segments(job, &["doWipeOutWorkspace"])?,
        response::empty,
    ))
}

/// Download a workspace file. The agent must be online.
pub fn workspace_file(job: &str, file: &str) -> Result<Request<Vec<u8>>> {
    let mut segments = path::job_segments(job, &["ws"])?;
    segments.extend(path::segments(file)?);
    Ok(Request::get(segments, response::bytes))
}

/// Download the workspace as a ZIP archive.
pub fn workspace_archive(job: &str) -> Result<Request<Vec<u8>>> {
    Ok(Request::get(
        path::job_segments(job, &["ws", "*zip*", "workspace.zip"])?,
        response::bytes,
    ))
}

/// `GET /job/<name>/api/json`
pub fn get(job: &str) -> Result<Request<JobInfo>> {
    let segments = path::job_segments(job, &["api", "json"])?;

    Ok(Request::get(segments, response::json))
}

/// `POST /job/<name>/submitDescription`
pub fn set_description(job: &str, description: impl Into<String>) -> Result<Request<()>> {
    let segments = path::job_segments(job, &["submitDescription"])?;
    Ok(Request::post(segments, response::empty).form_pairs([("description", description)]))
}

/// `POST /job/<name>/build`
pub fn build(job: &str) -> Result<Request<TriggeredBuild>> {
    let segments = path::job_segments(job, &["build"])?;
    Ok(Request::post(segments, decode_triggered_build))
}

/// `POST /job/<name>/buildWithParameters`
pub fn build_with_parameters<I, K, V>(job: &str, params: I) -> Result<Request<TriggeredBuild>>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let segments = path::job_segments(job, &["buildWithParameters"])?;
    Ok(Request::post(segments, decode_triggered_build).form_pairs(params))
}

/// `GET /job/<name>/config.xml`
pub fn get_config_xml(job: &str) -> Result<Request<Vec<u8>>> {
    let segments = path::job_segments(job, &["config.xml"])?;
    Ok(Request::get(segments, response::bytes))
}

/// `POST /job/<name>/config.xml` with XML body.
pub fn update_config_xml(job: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    let segments = path::job_segments(job, &["config.xml"])?;
    Ok(Request::post(segments, response::empty).xml(xml))
}

/// `POST /createItem?name=<name>` with XML body.
pub fn create_from_xml(name: &str, xml: impl Into<Vec<u8>>) -> Result<Request<()>> {
    path::validate_segment(name)?;

    Ok(Request::post(["createItem"], response::empty)
        .query("name", name)
        .xml(xml))
}

/// `POST /createItem?name=<new>&mode=copy&from=<from>`
pub fn copy(from: &str, to: &str) -> Result<Request<()>> {
    path::segments(from)?;
    path::validate_segment(to)?;

    Ok(Request::post(["createItem"], response::empty)
        .query("name", to)
        .query("mode", "copy")
        .query("from", from))
}

/// `POST /job/<name>/doDelete`
pub fn delete(job: &str) -> Result<Request<()>> {
    let segments = path::job_segments(job, &["doDelete"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/disable`
pub fn disable(job: &str) -> Result<Request<()>> {
    let segments = path::job_segments(job, &["disable"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/enable`
pub fn enable(job: &str) -> Result<Request<()>> {
    let segments = path::job_segments(job, &["enable"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/confirmRename?newName=<new_name>`
pub fn rename(job: &str, new_name: &str) -> Result<Request<()>> {
    path::validate_segment(new_name)?;

    let segments = path::job_segments(job, &["confirmRename"])?;
    Ok(Request::post(segments, response::empty).query("newName", new_name))
}

fn decode_triggered_build(
    response: &reqx::Response,
    request_url: &url::Url,
) -> std::result::Result<TriggeredBuild, crate::DecodeError> {
    use crate::DecodeError;
    let location = request_url
        .join(response::required_header(response, "location")?)
        .map_err(|_| DecodeError::InvalidHeader("location"))?;
    let path = location.path().strip_suffix('/').unwrap_or(location.path());
    let (queue_path, id) = path
        .rsplit_once('/')
        .ok_or(DecodeError::InvalidHeader("location"))?;
    if !matches!(location.scheme(), "http" | "https")
        || !queue_path.ends_with("/queue/item")
        || location.query().is_some()
        || location.fragment().is_some()
    {
        return Err(DecodeError::InvalidHeader("location"));
    }
    let queue_item_id = id
        .parse()
        .map_err(|_| DecodeError::InvalidHeader("location"))?;
    Ok(TriggeredBuild {
        queue_item_id,
        location,
    })
}
