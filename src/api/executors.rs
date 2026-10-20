//! Regular and one-off computer executors.

use super::path;
use crate::{Request, Result, response};
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub enum Executor {
    Regular(u32),
    OneOff(u32),
}

fn segments(computer: &str, executor: Executor, suffix: &[&str]) -> Result<Vec<String>> {
    path::validate_segment(computer)?;
    let (kind, index) = match executor {
        Executor::Regular(index) => ("executors", index),
        Executor::OneOff(index) => ("oneOffExecutors", index),
    };
    let mut segments = vec![
        "computer".into(),
        computer.into(),
        kind.into(),
        index.to_string(),
    ];
    segments.extend(suffix.iter().map(|s| (*s).to_owned()));
    Ok(segments)
}

pub fn get(computer: &str, executor: Executor) -> Result<Request<Value>> {
    Ok(Request::get(
        segments(computer, executor, &["api", "json"])?,
        response::json,
    ))
}

/// Stop whichever task currently occupies the executor.
pub fn stop(computer: &str, executor: Executor) -> Result<Request<()>> {
    Ok(Request::post(
        segments(computer, executor, &["stop"])?,
        response::empty,
    ))
}

/// Stop only if the running build matches `full-job-name#build-number`.
pub fn stop_build(computer: &str, executor: Executor, run_id: &str) -> Result<Request<()>> {
    Ok(Request::post(
        segments(computer, executor, &["stopBuild"])?,
        response::empty,
    )
    .form_pairs([("runExtId", run_id)]))
}
