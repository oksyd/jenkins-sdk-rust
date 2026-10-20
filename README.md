# jenkins-sdk

[![Crates.io](https://img.shields.io/crates/v/jenkins-sdk.svg)](https://crates.io/crates/jenkins-sdk)
[![Documentation](https://docs.rs/jenkins-sdk/badge.svg)](https://docs.rs/jenkins-sdk)
[![CI](https://github.com/oksyd/jenkins-sdk-rust/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/oksyd/jenkins-sdk-rust/actions/workflows/ci.yaml)
[![Rust 1.99.1+](https://img.shields.io/badge/rust-1.99.1%2B-orange.svg)](Cargo.toml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A Rust client for the Jenkins core HTTP API, with typed requests and responses backed by [reqx](https://docs.rs/reqx).

- **Async and blocking clients** share the same request constructors and response types.
- **Core resources** include jobs, builds, queues, agents, views, users, API tokens, and controller administration.
- **Flexible requests** support JSON field selection, multipart uploads, and custom endpoints.
- **Native HTTP controls** expose reqx configuration for TLS, timeouts, retries, proxies, and streaming.

Requires **Rust 1.99.1+**. The supported Jenkins core baseline is **2.580.1 LTS**; plugin-specific APIs can be accessed through custom requests.

## Getting started

This checkout contains an unreleased breaking API. The examples below use the local checkout; documentation on docs.rs corresponds to published releases.

From a neighboring project, add:

```toml
[dependencies]
jenkins-sdk = { path = "../jenkins-sdk-rust" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use jenkins_sdk::{Auth, Client, api::jobs};

#[tokio::main]
async fn main() -> jenkins_sdk::Result<()> {
    let client = Client::builder("https://jenkins.example.com/jenkins")?
        .auth(Auth::basic("user", "api-token"))
        .build()?;

    let jobs = client.execute(jobs::list()).await?;
    for job in jobs.jobs {
        println!("{}", job.name);
    }

    Ok(())
}
```

Resource functions construct requests; `execute` performs the HTTP call. Response types live in the same resource module. Use `.tree(...)` to select JSON fields and `.query(...)` for other Jenkins parameters.

## Client configuration

The default feature, `async-rustls-ring`, enables the async client with Rustls. For a blocking-only application:

```toml
[dependencies]
jenkins-sdk = { path = "../jenkins-sdk-rust", default-features = false, features = ["blocking-rustls-ring"] }
```

`BlockingClient` executes the same requests without `.await` or a Tokio runtime. To enable both clients, add `blocking-rustls-ring` while keeping default features.

Configure transport policies through `.configure_http(...)` or `.try_configure_http(...)`. Retries are disabled by default. CSRF crumb handling is opt-in through `.crumb_ttl(...)`; API-token authentication normally does not require it.

Choose one TLS backend per enabled client mode. The full TLS and compression feature matrix is documented in rustdoc.

## Documentation and examples

Rustdoc is the primary reference for API contracts, configuration, errors, and migration. Generate documentation for this checkout with both clients enabled:

```sh
cargo doc --open --no-deps --features blocking-rustls-ring
```

Runnable examples cover:

- [Async](examples/async_client.rs) and [blocking](examples/blocking_client.rs) clients.
- [Progressive console logs](examples/progressive_log.rs) and [artifact downloads](examples/artifact_download.rs).
- [Errors and retries](examples/errors_and_retries.rs), [interceptors](examples/interceptor.rs), and [custom CA certificates](examples/custom_ca.rs).
