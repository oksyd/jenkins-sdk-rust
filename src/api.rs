//! Jenkins resources: request constructors and their response types.
//!
//! Functions construct typed requests without performing I/O. Names and paths
//! are validated before a request is returned.
//!
//! # Resource paths and projections
//!
//! Nested job names use `folder/job`; constructors insert Jenkins' `/job/`
//! segments and encode each name. Artifact paths use the same segment validation:
//! empty segments, `.` / `..`, backslashes, and control characters are rejected.
//! Supply unescaped names.
//!
//! Use [`Request::tree`](crate::Request::tree) for JSON field selection and
//! [`Request::query`](crate::Request::query) for other Jenkins parameters.
//! Omitted optional fields remain `None`. List envelopes require their collection
//! field and any non-optional entry fields, such as a job's `name`; include these
//! in projections. Unknown fields are retained by models with an `extra` map.
//!
//! # Core coverage
//!
//! Resource modules cover jobs/builds, queues, agents/executors, views, users,
//! tokens, labels, fingerprints, logging, maintenance, setup, and core plugin,
//! cloud, update-center, and controller management. The baseline is
//! [`SUPPORTED_JENKINS_LTS`](crate::SUPPORTED_JENKINS_LTS).
//!
//! Jenkins exposes a distributed Remote Access API. Use [`remote`] to read any
//! exported resource as JSON, XML, schema, or Python text; JSON can use a
//! caller-defined model. Descriptor-dependent forms use `serde_json::Value`.
//! [`Request::new`](crate::Request::new) supports additional HTTP routes.
//!
//! Plugin-contributed APIs, such as Pipeline `wfapi`, `term`/`kill`, credentials,
//! folder management, and People, are outside the core baseline. CLI and agent
//! remoting/WebSocket protocols are also outside this HTTP API; their JARs and
//! launch descriptors are downloadable through core constructors.
//!
//! The repository's `tests/jenkins-lts.sh` exercises both clients against an
//! isolated controller. Restart, shutdown, WAR replacement, first-run setup
//! changes, online-agent execution, and concrete cloud implementations have
//! source-reviewed constructors but are not executed by that lifecycle test.
//! Network-dependent update downloads are not part of the offline fixture.

pub(crate) mod path;

pub mod builds;
pub mod clouds;
pub mod computers;
pub mod executors;
pub mod fingerprints;
pub mod jobs;
pub mod labels;
pub mod logs;
pub mod monitors;
pub mod plugins;
pub mod queue;
pub mod remote;
pub mod setup;
pub mod system;
pub mod tokens;
pub mod updates;
pub mod users;
pub mod views;
