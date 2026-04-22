//! HTTP client construction following RFC 9113 (HTTP/2) connection semantics.
//!
//! A single `reqwest::Client` is intended to be shared across all SSE
//! connections for a given process lifetime (connection pool reuse).
//! `http2_prior_knowledge` bypasses the HTTP/1.1 upgrade round-trip when
//! the server is known to support HTTP/2, as Vexcoder does on all endpoints.

use std::time::Duration;

use reqwest::{Client, ClientBuilder};

use crate::client::sse_parser::SseError;

/// Pool capacity per host. Tuned for a single-server TUI client.
const POOL_IDLE_PER_HOST: usize = 4;

/// Absolute request timeout (SSE streams excluded — they use connection-level
/// keepalive instead). Applies to the initial HTTP response header receipt.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Constructs a shared `reqwest::Client` configured for SSE workloads.
///
/// - HTTP/2 prior knowledge: avoids the `Upgrade` round-trip (RFC 7540 §3.4).
/// - `rustls-tls`: statically linked TLS; no dependency on the system OpenSSL ABI.
/// - `pool_max_idle_per_host`: bounds idle connection growth for long-lived processes.
pub fn build_client() -> Result<Client, SseError> {
    ClientBuilder::new()
        .http2_prior_knowledge()
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_max_idle_per_host(POOL_IDLE_PER_HOST)
        .user_agent(concat!("aozcoder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(SseError::ClientBuild)
}
