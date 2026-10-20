//! Core administrative monitor actions.
//!
//! Monitor IDs are their exported `id`, usually a fully qualified Java class name.
//! Individual monitor action parameters are described by the corresponding core
//! class; generic HTTP requests can also access descriptor-specific validation.

use super::path;
use crate::{Request, Result, response};

pub fn disable(id: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(Request::post(
        ["administrativeMonitor", id, "disable"],
        response::empty,
    ))
}

/// Submit a monitor's action. Add its parameters with `.form_pairs(...)`.
pub fn act(id: &str) -> Result<Request<()>> {
    path::validate_segment(id)?;
    Ok(Request::post(
        ["administrativeMonitor", id, "act"],
        response::empty,
    ))
}

/// Upgrade persisted data up to the requested Jenkins version.
pub fn upgrade_old_data(version: &str) -> Request<()> {
    Request::post(
        ["administrativeMonitor", "OldData", "upgrade"],
        response::empty,
    )
    .form_pairs([("thruVer", version)])
}

pub fn discard_unreadable_data() -> Request<()> {
    Request::post(
        ["administrativeMonitor", "OldData", "discard"],
        response::empty,
    )
}

pub fn clear_slow_triggers() -> Request<()> {
    Request::post(
        [
            "administrativeMonitor",
            "hudson.triggers.SlowTriggerAdminMonitor",
            "clear",
        ],
        response::empty,
    )
}
