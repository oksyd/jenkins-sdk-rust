//! Controller log recorders and logger levels.

use super::path;
use crate::{Request, Result, response};
use serde::Serialize;

/// A java.util.logging logger and its numeric level (for example INFO = 800).
#[derive(Clone, Debug, Serialize)]
pub struct Logger {
    pub name: String,
    pub level: i32,
}

pub fn create(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(["log", "newLogRecorder"], response::empty).form_pairs([("name", name)]))
}

/// Replace logger targets; `new_name` also renames the recorder.
pub fn configure(name: &str, new_name: &str, loggers: &[Logger]) -> Result<Request<()>> {
    path::validate_segment(name)?;
    path::validate_segment(new_name)?;
    Request::post(["log", name, "configSubmit"], response::empty)
        .submitted_form(&serde_json::json!({"name": new_name, "loggers": loggers}))
}

pub fn clear(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(["log", name, "clear"], response::empty))
}

pub fn delete(name: &str) -> Result<Request<()>> {
    path::validate_segment(name)?;
    Ok(Request::post(["log", name, "doDelete"], response::empty))
}

/// Read recorded entries as an Atom feed; `.query("level", "WARNING")` filters.
pub fn feed(name: &str) -> Result<Request<Vec<u8>>> {
    path::validate_segment(name)?;
    Ok(Request::get(["log", name, "rss"], response::bytes))
}

pub fn system_feed() -> Request<Vec<u8>> {
    Request::get(["log", "rss"], response::bytes)
}

/// Change a live JVM logger's level, e.g. `INFO`, `FINE`, or `inherit`.
pub fn set_level(name: &str, level: &str) -> Request<()> {
    Request::post(["log", "configLogger"], response::empty)
        .form_pairs([("name", name), ("level", level)])
}
