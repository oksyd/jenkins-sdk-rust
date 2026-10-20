//! Jenkins builds: request constructors and response types.
//!
//! ```
//! use jenkins_sdk::{Request, api::builds::{self, BuildInfo, BuildSelector}};
//!
//! let request: Request<BuildInfo> = builds::get("folder/job", BuildSelector::LastSuccessful)?
//!     .tree("number,result");
//! # Ok::<(), jenkins_sdk::Error>(())
//! ```

use super::path;
use crate::{Request, Result, response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A build number or one of Jenkins' symbolic build selectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildSelector {
    Number(u64),
    Last,
    LastCompleted,
    LastSuccessful,
    LastFailed,
    LastStable,
    LastUnstable,
    LastUnsuccessful,
}

impl std::fmt::Display for BuildSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let selector = match self {
            Self::Number(number) => return number.fmt(f),
            Self::Last => "lastBuild",
            Self::LastCompleted => "lastCompletedBuild",
            Self::LastSuccessful => "lastSuccessfulBuild",
            Self::LastFailed => "lastFailedBuild",
            Self::LastStable => "lastStableBuild",
            Self::LastUnstable => "lastUnstableBuild",
            Self::LastUnsuccessful => "lastUnsuccessfulBuild",
        };
        f.write_str(selector)
    }
}

impl std::str::FromStr for BuildSelector {
    type Err = crate::Error;
    fn from_str(value: &str) -> Result<Self> {
        Ok(match value {
            "lastBuild" => Self::Last,
            "lastCompletedBuild" => Self::LastCompleted,
            "lastSuccessfulBuild" => Self::LastSuccessful,
            "lastFailedBuild" => Self::LastFailed,
            "lastStableBuild" => Self::LastStable,
            "lastUnstableBuild" => Self::LastUnstable,
            "lastUnsuccessfulBuild" => Self::LastUnsuccessful,
            _ => Self::Number(value.parse().map_err(|_| crate::Error::InvalidInput {
                field: "build",
                message: "expected a build number or Jenkins build selector",
            })?),
        })
    }
}

/// Build-level API payload (for example `GET /job/<name>/<build>/api/json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BuildInfo {
    pub number: Option<u64>,
    pub result: Option<String>,
    pub building: Option<bool>,
    pub display_name: Option<String>,
    pub full_display_name: Option<String>,
    pub url: Option<String>,
    pub duration: Option<u64>,
    pub timestamp: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result of `.../logText/progressiveText`.
///
/// Text must be UTF-8 and `X-Text-Size` must contain a valid byte offset.
/// A missing `X-More-Data` means the log is complete.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProgressiveText {
    /// Newly appended console output starting from the requested offset.
    pub text: String,
    /// Next byte offset (`X-Text-Size`) for the following request.
    pub next_start: u64,
    /// Whether the server indicates more data is available (`X-More-Data: true`).
    pub more_data: bool,
}

pub(super) fn build_segments(
    job: &str,
    build: BuildSelector,
    suffix: &[&str],
) -> Result<Vec<String>> {
    let mut segments = path::job_segments(job, &[])?;
    segments.push(build.to_string());
    segments.extend(suffix.iter().map(|part| (*part).to_owned()));
    Ok(segments)
}

/// `GET /job/<name>/<build>/logText/progressiveText?start=<offset>`
pub fn progressive_console_text(
    job: &str,
    build: BuildSelector,
    start: u64,
) -> Result<Request<ProgressiveText>> {
    let segments = build_segments(job, build, &["logText", "progressiveText"])?;
    Ok(Request::get(segments, decode_progressive_text).query("start", start.to_string()))
}

/// Read annotated HTML. Use `.raw_response()` to retain `X-ConsoleAnnotator`
/// and pass that header back on subsequent requests when retaining annotation state.
pub fn progressive_console_html(
    job: &str,
    build: BuildSelector,
    start: u64,
) -> Result<Request<ProgressiveText>> {
    Ok(Request::get(
        build_segments(job, build, &["logText", "progressiveHtml"])?,
        decode_progressive_text,
    )
    .query("start", start.to_string()))
}

/// `GET /job/<name>/<build>/api/json`
pub fn get(job: &str, build: BuildSelector) -> Result<Request<BuildInfo>> {
    let segments = build_segments(job, build, &["api", "json"])?;

    Ok(Request::get(segments, response::json))
}

/// `GET /job/<name>/<build>/consoleText`
pub fn console_text(job: &str, build: BuildSelector) -> Result<Request<String>> {
    let segments = build_segments(job, build, &["consoleText"])?;
    Ok(Request::get(segments, response::text))
}

/// Download all archived artifacts as a ZIP file.
pub fn artifact_archive(job: &str, build: BuildSelector) -> Result<Request<Vec<u8>>> {
    Ok(Request::get(
        build_segments(job, build, &["artifact", "*zip*", "archive.zip"])?,
        response::bytes,
    ))
}

/// Read the build number as plain text.
pub fn number(job: &str, build: BuildSelector) -> Result<Request<String>> {
    Ok(Request::get(
        build_segments(job, build, &["buildNumber"])?,
        response::text,
    ))
}

/// Read the timestamp. Use `.query("format", "...")` for a Java date format.
pub fn timestamp(job: &str, build: BuildSelector) -> Result<Request<String>> {
    Ok(Request::get(
        build_segments(job, build, &["buildTimestamp"])?,
        response::text,
    ))
}

/// Read the SCM polling output attached to a build triggered by SCM changes.
pub fn polling_log(job: &str, build: BuildSelector) -> Result<Request<String>> {
    Ok(Request::get(
        build_segments(job, build, &["pollingLog", "pollingLog"])?,
        response::text,
    ))
}

/// Update both editable core build properties.
pub fn configure(
    job: &str,
    build: BuildSelector,
    display_name: &str,
    description: &str,
) -> Result<Request<()>> {
    Request::post(
        build_segments(job, build, &["configSubmit"])?,
        response::empty,
    )
    .submitted_form(&serde_json::json!({"displayName": display_name, "description": description}))
}

/// `GET /job/<name>/<build>/artifact/<path>`
pub fn download_artifact(job: &str, build: BuildSelector, path: &str) -> Result<Request<Vec<u8>>> {
    let mut segments = build_segments(job, build, &["artifact"])?;
    segments.extend(path::segments(path)?);
    Ok(Request::get(segments, response::bytes))
}

/// `POST /job/<name>/<build>/stop`
pub fn stop(job: &str, build: BuildSelector) -> Result<Request<()>> {
    let segments = build_segments(job, build, &["stop"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/<build>/doDelete`
pub fn delete(job: &str, build: BuildSelector) -> Result<Request<()>> {
    let segments = build_segments(job, build, &["doDelete"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/<build>/toggleLogKeep`
pub fn toggle_keep_log(job: &str, build: BuildSelector) -> Result<Request<()>> {
    let segments = build_segments(job, build, &["toggleLogKeep"])?;
    Ok(Request::post(segments, response::empty))
}

/// `POST /job/<name>/<build>/submitDescription`
pub fn set_description(
    job: &str,
    build: BuildSelector,
    description: impl Into<String>,
) -> Result<Request<()>> {
    let segments = build_segments(job, build, &["submitDescription"])?;
    Ok(Request::post(segments, response::empty).form_pairs([("description", description)]))
}

pub(super) fn decode_progressive_text(
    response: &reqx::Response,
    url: &url::Url,
) -> std::result::Result<ProgressiveText, crate::DecodeError> {
    use crate::DecodeError;
    let next_start = response::required_header(response, "x-text-size")?
        .parse()
        .map_err(|_| DecodeError::InvalidHeader("x-text-size"))?;
    // Jenkins omits X-More-Data when the log is complete.
    let more_data = match response.headers().get("x-more-data") {
        None => false,
        Some(value) if value == "true" => true,
        Some(value) if value == "false" => false,
        Some(_) => return Err(DecodeError::InvalidHeader("x-more-data")),
    };
    Ok(ProgressiveText {
        text: response::text(response, url)?,
        next_start,
        more_data,
    })
}
