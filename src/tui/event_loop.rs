//! Async event loop implementing the TUI run-to-completion model.
//!
//! `EventLoop::run` drives three concurrent concerns via `tokio::select!`:
//!
//!  1. Terminal input events from `crossterm::event::EventStream` (non-blocking,
//!     backed by the crossterm `event-stream` feature / epoll/kqueue).
//!  2. `RuntimeEnvelope` values forwarded from spawned SSE tasks via an
//!     `mpsc::unbounded_channel`.
//!  3. Frame rendering after each state transition.
//!
//! The loop terminates on `AppEvent::Quit` or when the event stream closes.

use std::pin::Pin;

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::{FutureExt, Stream, StreamExt};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::{
    client::{
        envelope::RuntimeEnvelope,
        sse_parser::{SseClient, SseError},
    },
    tui::app::{App, AppEvent},
};

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type EnvelopeStream = Pin<Box<dyn Stream<Item = Result<RuntimeEnvelope, SseError>> + Send>>;

// ---------------------------------------------------------------------------
// EventLoop
// ---------------------------------------------------------------------------

pub struct EventLoop {
    app: App,
    terminal: DefaultTerminal,
    sse_client: SseClient,
    envelope_tx: mpsc::UnboundedSender<RuntimeEnvelope>,
    envelope_rx: mpsc::UnboundedReceiver<RuntimeEnvelope>,
}

impl EventLoop {
    pub fn new(terminal: DefaultTerminal, sse_client: SseClient, session_id: String) -> Self {
        let (envelope_tx, envelope_rx) = mpsc::unbounded_channel();
        Self {
            app: App::new(session_id),
            terminal,
            sse_client,
            envelope_tx,
            envelope_rx,
        }
    }

    /// Runs the event loop until quit or terminal close.
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("AozCoder event loop started");

        let mut crossterm_events = EventStream::new();

        loop {
            // Render before waiting; ensures the initial frame appears immediately.
            self.terminal.draw(|f| self.app.draw(f))?;

            tokio::select! {
                biased;

                // SSE envelopes forwarded from background stream tasks
                envelope = self.envelope_rx.recv() => {
                    match envelope {
                        Some(env) => self.app.handle_envelope(env),
                        None => break,
                    }
                }

                // Keyboard / resize events from crossterm
                maybe_event = crossterm_events.next().fuse() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            if let Some(action) = self.app.handle_key_event(key) {
                                match action {
                                    AppEvent::Quit => break,
                                    AppEvent::Submit(prompt) => {
                                        self.handle_submit(prompt).await;
                                    }
                                    AppEvent::Interrupt => {
                                        self.app.streaming = false;
                                    }
                                }
                            }
                        }
                        Some(Ok(Event::Resize(_, _))) => {
                            // Terminal redraws automatically on next loop iteration.
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
        }

        info!("AozCoder event loop terminated");
        Ok(())
    }

    /// Spawns a task that opens an SSE stream for `prompt` and forwards
    /// deserialized envelopes to `envelope_tx`.
    async fn handle_submit(&mut self, prompt: String) {
        let endpoint = "v1/run";
        let body = serde_json::json!({
            "task": prompt,
            "stream": true,
            "session_id": self.app.session_id,
        });

        self.app.conversation.begin_turn(Some(prompt.clone()));

        match self.sse_client.connect_stream(endpoint, body).await {
            Ok(stream) => {
                let tx = self.envelope_tx.clone();
                tokio::spawn(forward_stream(stream, tx));
            }
            Err(e) => {
                error!(error = %e, "failed to open SSE stream");
                self.app.error_message = Some(e.to_string());
                self.app.streaming = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stream forwarding task
// ---------------------------------------------------------------------------

async fn forward_stream(mut stream: EnvelopeStream, tx: mpsc::UnboundedSender<RuntimeEnvelope>) {
    while let Some(result) = stream.next().await {
        match result {
            Ok(envelope) => {
                if tx.send(envelope).is_err() {
                    break;
                }
            }
            Err(e) => {
                error!(error = %e, "SSE stream error in background task");
                break;
            }
        }
    }
}
