//! Typed Jenkins core requests with async and blocking execution, powered by [`reqx`].
//!
//! Start with a client, construct a [`Request`] using an [`api`] resource module,
//! then execute it. Request construction validates input without performing I/O.
//! Response types live alongside their constructors, such as [`api::jobs::JobInfo`].
//! The core baseline is [`SUPPORTED_JENKINS_LTS`].
//!
//! # Quick start
//!
//! The default feature, `async-rustls-ring`, provides `Client`. Run async calls
//! inside a Tokio runtime.
//!
//! ```no_run
//! # #[cfg(feature = "async")]
//! # async fn example() -> jenkins_sdk::Result<()> {
//! use jenkins_sdk::{Auth, Client, api::{jobs, queue}};
//!
//! let client = Client::builder("https://jenkins.example.com/jenkins")?
//!     .auth(Auth::basic("user", "api-token"))
//!     .build()?;
//!
//! let queue = client.execute(queue::list().tree("items[id,why]")).await?;
//! let job = client.execute(jobs::get("folder/job")?).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Enable `blocking-rustls-ring` for `BlockingClient`, which executes the same
//! requests without a Tokio runtime. Use a dedicated thread or `spawn_blocking`
//! when calling it from an async application.
//!
//! ```no_run
//! # #[cfg(feature = "blocking")]
//! # fn example() -> jenkins_sdk::Result<()> {
//! use jenkins_sdk::{Auth, BlockingClient, api::jobs};
//!
//! let client = BlockingClient::builder("https://jenkins.example.com/jenkins")?
//!     .auth(Auth::basic("user", "api-token"))
//!     .build()?;
//! let job = client.execute(jobs::get("folder/job")?)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! Choose exactly one TLS backend per enabled client mode. Both modes can be
//! enabled together. Disable default features for blocking-only use or to replace
//! the default async TLS backend. Blocking-only dependencies do not include Tokio.
//!
//! | Async | Blocking |
//! |---|---|
//! | `async-rustls-ring` (default) | `blocking-rustls-ring` |
//! | `async-rustls-aws-lc-rs` | `blocking-rustls-aws-lc-rs` |
//! | `async-native-tls` | `blocking-native-tls` |
//!
//! Compression is opt-in: `compression-gzip`, `compression-brotli`, and
//! `compression-zstd`. `--all-features` is unsupported because TLS backends are
//! mutually exclusive.
//!
//! # Usage guide
//!
//! - [`api`]: core coverage, resource paths, and response projections.
//! - [`Builder::configure_http`]: native reqx timeouts, retries, TLS, proxies,
//!   and response limits. Retries are disabled by default.
//! - [`Builder::crumb_ttl`]: opt-in CSRF crumbs and session caching.
//! - [`Request`]: query parameters, request bodies, custom endpoints, and decoders.
//! - [`Multipart`]: buffered file parameters and uploads.
//! - [`Error`] and [`HttpError`]: transport failures, response context, and diagnostics.
//!
//! Complete programs are in the repository's
//! [examples directory](https://github.com/lvillis/jenkins-sdk-rust/tree/main/examples).
//!
//! # Migrating from the service API
//!
//! This request-based API replaces the previous service hierarchy without
//! compatibility aliases:
//!
//! - Replace `client.jobs().get(name, tree)` with
//!   `client.execute(jobs::get(name)?.tree(fields))`, importing [`api::jobs`].
//! - Build operations, logs, and artifacts now live in [`api::builds`].
//!   Use [`api::builds::BuildSelector`] for numeric and symbolic build selectors.
//!   Other names and paths use `&str`; queue IDs use `u64`.
//! - Import response models from their resource modules instead of `types`,
//!   `models`, or the crate root. `ComputerSummary` is now
//!   [`api::computers::ComputerInfo`]; identity is [`api::system::who_am_i`].
//! - Use [`Builder::auth`] with [`Auth`], [`Builder::crumb_ttl`], and
//!   [`Builder::configure_http`] / [`Builder::try_configure_http`]. Transport hooks,
//!   tracing, metrics, TLS, and retry policies use reqx's native APIs.
//! - Match [`Error`] variants and inspect the native reqx error for transport
//!   classification. Executor availability uses
//!   [`api::computers::ExecutorsInfo::idle_executors`], which returns `Option<u32>`.
//! - Invalid legacy routes were removed: use structured controller/user
//!   configuration, [`api::views::configure`] for view changes, and
//!   [`api::computers::launch_slave_agent`] for agent launch. Computer XML creation
//!   uses `computer/createItem`.
//! - Plugin APIs, including Pipeline termination and the extracted People API,
//!   use [`Request::new`] instead of core resource constructors.

// compile-time guard: enable at least one client kind.
#[cfg(not(any(feature = "async", feature = "blocking")))]
compile_error!("Enable at least one of: `async` (default) or `blocking`.");

// compile-time guard: async mode requires exactly one async TLS backend.
#[cfg(all(
    feature = "async",
    not(any(
        feature = "async-rustls-ring",
        feature = "async-rustls-aws-lc-rs",
        feature = "async-native-tls"
    ))
))]
compile_error!(
    "Enable one async TLS backend: `async-rustls-ring` (default), `async-rustls-aws-lc-rs`, or `async-native-tls`."
);

#[cfg(all(
    feature = "async",
    any(
        all(feature = "async-rustls-ring", feature = "async-rustls-aws-lc-rs"),
        all(feature = "async-rustls-ring", feature = "async-native-tls"),
        all(feature = "async-rustls-aws-lc-rs", feature = "async-native-tls")
    )
))]
compile_error!(
    "Async mode accepts exactly one TLS backend: choose only one of `async-rustls-ring`, `async-rustls-aws-lc-rs`, or `async-native-tls`."
);

// compile-time guard: blocking mode requires exactly one blocking TLS backend.
#[cfg(all(
    feature = "blocking",
    not(any(
        feature = "blocking-rustls-ring",
        feature = "blocking-rustls-aws-lc-rs",
        feature = "blocking-native-tls"
    ))
))]
compile_error!(
    "Enable one blocking TLS backend: `blocking-rustls-ring`, `blocking-rustls-aws-lc-rs`, or `blocking-native-tls`."
);

#[cfg(all(
    feature = "blocking",
    any(
        all(
            feature = "blocking-rustls-ring",
            feature = "blocking-rustls-aws-lc-rs"
        ),
        all(feature = "blocking-rustls-ring", feature = "blocking-native-tls"),
        all(feature = "blocking-rustls-aws-lc-rs", feature = "blocking-native-tls")
    )
))]
compile_error!(
    "Blocking mode accepts exactly one TLS backend: choose only one of `blocking-rustls-ring`, `blocking-rustls-aws-lc-rs`, or `blocking-native-tls`."
);

pub mod api;
mod auth;
mod client;
mod error;
mod multipart;
mod request;
mod response;

pub use auth::Auth;
pub use client::Builder;
pub use error::{DecodeError, Error, HttpError, Result};
/// HTTP methods, headers, and status codes used by custom requests.
pub use http;
pub use multipart::Multipart;
pub use request::Request;
/// Native HTTP policies, TLS settings, and extension points.
pub use reqx;

/// Jenkins core version used to audit and integration-test this release.
pub const SUPPORTED_JENKINS_LTS: &str = "2.580.1";

#[cfg(feature = "blocking")]
pub use client::{BlockingClient, BlockingClientBuilder};
#[cfg(feature = "async")]
pub use client::{Client, ClientBuilder};
