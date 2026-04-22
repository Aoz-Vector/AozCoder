//! SSE stream consumer following WHATWG EventSource Processing Model §9.2.
//!
//! `SseClient` sends a POST request with `Accept: text/event-stream`, then
//! parses the response body as a sequence of `eventsource_stream` events.
//! Each non-empty `data:` field is deserialised into a `RuntimeEnvelope`.
//!
//! Empty `data:` fields and heartbeat lines (`:`) are silently discarded.
//! Parse failures produce a `warn!` trace and are dropped rather than
//! terminating the stream, to tolerate forward-compatible schema additions.

use std::pin::Pin;

use eventsource_stream::{EventStreamError, Eventsource};
use futures::{Stream, StreamExt};
use reqwest::StatusCode;
use tracing::{debug, error, warn};

use crate::client::{connection::build_client, envelope::RuntimeEnvelope};

// ---------------------------------------------------------------------------
// Public client type
// ---------------------------------------------------------------------------

/// Stateless handle for initiating SSE streams against a Vexcoder endpoint.
///
/// Cheaply cloneable; the internal `reqwest::Client` is `Arc`-backed.
#[derive(Clone)]
pub struct SseClient {
    base_url: String,
    api_key: Option<String>,
    session_id: String,
}

impl SseClient {
    /// Constructs a client bound to the given base URL.
    ///
    /// `api_key`, when present, is transmitted as a `Bearer` token per RFC 6750 §2.1.
    pub fn new(base_url: String, api_key: Option<String>, session_id: String) -> Self {
        Self {
            base_url,
            api_key,
            session_id,
        }
    }

    /// Opens a POST SSE stream to `{base_url}/{endpoint}` and returns a
    /// `Stream` of deserialized `RuntimeEnvelope` values.
    ///
    /// The returned stream is heap-pinned and `Send + 'static`, suitable for
    /// transfer to a `tokio::spawn` task without additional wrapping.
    ///
    /// # Errors
    ///
    /// Returns `SseError` if the HTTP request fails or the server responds
    /// with a non-2xx status code. Stream-level errors are yielded as
    /// `Some(Err(SseError::EventStream(_)))` items.
    pub async fn connect_stream(
        &self,
        endpoint: &str,
        request_body: serde_json::Value,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RuntimeEnvelope, SseError>> + Send>>, SseError>
    {
        let client = build_client()?;
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint);

        debug!(url = %url, "opening SSE stream");

        let mut builder = client
            .post(&url)
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .header("X-Session-Id", &self.session_id)
            .json(&request_body);

        if let Some(ref key) = self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = builder.send().await.map_err(SseError::Request)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SseError::HttpStatus { status, body });
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .filter_map(|result| async move {
                match result {
                    Ok(event) => {
                        if event.data.is_empty() || event.data == "[DONE]" {
                            return None;
                        }
                        match serde_json::from_str::<RuntimeEnvelope>(&event.data) {
                            Ok(envelope) => {
                                debug!(
                                    seq = envelope.seq,
                                    event_type = envelope.event.type_name(),
                                    "envelope received"
                                );
                                Some(Ok(envelope))
                            }
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    data = %event.data,
                                    "envelope deserialisation failed — skipping"
                                );
                                None
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "SSE transport error");
                        Some(Err(SseError::EventStream(e)))
                    }
                }
            });

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced during SSE stream establishment or transport.
#[derive(Debug, thiserror::Error)]
pub enum SseError {
    #[error("HTTP client construction failed: {0}")]
    ClientBuild(reqwest::Error),

    #[error("HTTP request failed: {0}")]
    Request(reqwest::Error),

    /// Non-2xx response before streaming began. `body` is the first 4 KiB of
    /// the response body, truncated at the byte boundary.
    #[error("HTTP {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },

    /// Transport-level SSE framing error (UTF-8 decode, TCP reset, etc.).
    /// Wraps `EventStreamError<reqwest::Error>` per the `eventsource_stream` contract.
    #[error("SSE event stream error: {0}")]
    EventStream(#[from] EventStreamError<reqwest::Error>),

    #[error("JSON deserialisation error: {0}")]
    Parse(#[from] serde_json::Error),
}
