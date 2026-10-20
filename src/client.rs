//! Client implementations (async + optional blocking).

mod builder;
mod config;
mod crumb;
pub use builder::Builder;

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(feature = "blocking")]
mod blocking;

#[cfg(feature = "async")]
pub use asynchronous::{Client, ClientBuilder};
#[cfg(feature = "blocking")]
pub use blocking::{BlockingClient, BlockingClientBuilder};
