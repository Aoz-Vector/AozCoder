//! Integration tests for the SSE client against a mock HTTP server.
//!
//! Uses `wiremock` to stand up a local server that returns a pre-formed SSE
//! response body.  The test verifies that `SseClient::connect_stream` correctly
//! deserialises `RuntimeEnvelope` values from the byte stream.

use aozcoder::client::{
    SseClient,
    envelope::{RuntimeEvent, TurnStatus},
};
use futures::StreamExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

fn make_sse_body(events: &[&str]) -> String {
    events.iter().map(|e| format!("data: {e}\n\n")).collect()
}

#[tokio::test]
async fn connect_stream_parses_turn_end() {
    let server = MockServer::start().await;

    let turn_start = serde_json::json!({
        "version": 1,
        "task_id": "t1",
        "turn": 1,
        "seq": 1,
        "event_id": "e1",
        "emitted_at": "2026-04-22T00:00:00Z",
        "source": "runtime",
        "event": { "type": "turn_start" }
    });

    let turn_end = serde_json::json!({
        "version": 1,
        "task_id": "t1",
        "turn": 1,
        "seq": 2,
        "event_id": "e2",
        "emitted_at": "2026-04-22T00:00:01Z",
        "source": "runtime",
        "event": {
            "type": "turn_end",
            "status": "completed",
            "changed_files": []
        }
    });

    let body = make_sse_body(&[&turn_start.to_string(), &turn_end.to_string()]);

    Mock::given(method("POST"))
        .and(path("/v1/run"))
        .and(header("accept", "text/event-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_raw(body, "text/event-stream"),
        )
        .mount(&server)
        .await;

    let client = SseClient::new(server.uri(), None, "test-session".to_string());
    let body = serde_json::json!({ "task": "hello", "stream": true });

    let mut stream = client.connect_stream("v1/run", body).await.unwrap();

    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.seq, 1);
    assert!(matches!(first.event, RuntimeEvent::TurnStart { .. }));

    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(second.seq, 2);
    assert!(matches!(
        second.event,
        RuntimeEvent::TurnEnd {
            status: TurnStatus::Completed,
            ..
        }
    ));

    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn connect_stream_http_error_returns_err() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/run"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("unavailable", "text/plain"))
        .mount(&server)
        .await;

    let client = SseClient::new(server.uri(), None, "s".to_string());
    let body = serde_json::json!({});

    let result = client.connect_stream("v1/run", body).await;
    assert!(result.is_err());
}
