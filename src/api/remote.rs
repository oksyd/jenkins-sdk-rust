//! Jenkins' resource-oriented Remote Access API and build feeds.
//!
//! Resource paths are relative to the Jenkins root, for example `job/demo/42`.
//! An empty path selects the controller. Query parameters (`tree`, `depth`,
//! `xpath`, `wrapper`, repeated `exclude`) use the regular request builder.
//!
//! ```
//! use jenkins_sdk::api::remote;
//!
//! let request = remote::json::<serde_json::Value>("label/linux")?
//!     .tree("name,nodes[nodeName]");
//! let xml = remote::xml("job/demo")?
//!     .query("depth", "2")
//!     .append_query("exclude", "//build")
//!     .append_query("exclude", "//action");
//! # Ok::<(), jenkins_sdk::Error>(())
//! ```

use super::path;
use crate::{Request, Result, response};
use serde::de::DeserializeOwned;

fn resource_path(resource: &str, suffix: &[&str]) -> Result<Vec<String>> {
    let mut segments = if resource.is_empty() {
        Vec::new()
    } else {
        path::segments(resource)?
    };
    segments.extend(suffix.iter().map(|s| (*s).to_owned()));
    Ok(segments)
}

/// Read any exported core resource, using a caller-defined model or `serde_json::Value`.
pub fn json<T: DeserializeOwned>(resource: &str) -> Result<Request<T>> {
    Ok(Request::get(
        resource_path(resource, &["api", "json"])?,
        response::json,
    ))
}

/// Read XML. XPath selection and exclusions are evaluated by Jenkins.
pub fn xml(resource: &str) -> Result<Request<Vec<u8>>> {
    Ok(Request::get(
        resource_path(resource, &["api", "xml"])?,
        response::bytes,
    ))
}

/// Read the resource's XML schema.
pub fn schema(resource: &str) -> Result<Request<Vec<u8>>> {
    Ok(Request::get(
        resource_path(resource, &["api", "schema"])?,
        response::bytes,
    ))
}

/// Read Jenkins' Python-literal representation as text; it is never executed.
pub fn python(resource: &str) -> Result<Request<String>> {
    Ok(Request::get(
        resource_path(resource, &["api", "python"])?,
        response::text,
    ))
}

/// Build feed selection. Available feeds depend on the resource.
#[derive(Clone, Copy, Debug)]
pub enum Feed {
    All,
    Failed,
    Latest,
    Changelog,
}

/// Read an Atom feed from a view, job, computer, or user.
/// `Latest` is supported on views/computers/users; `Changelog` on jobs.
pub fn feed(resource: &str, feed: Feed) -> Result<Request<Vec<u8>>> {
    let action = match feed {
        Feed::All => "rssAll",
        Feed::Failed => "rssFailed",
        Feed::Latest => "rssLatest",
        Feed::Changelog => "rssChangelog",
    };
    Ok(Request::get(
        resource_path(resource, &[action])?,
        response::bytes,
    ))
}
