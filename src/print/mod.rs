//! Non-interactive output for CI/CD pipelines and batch invocations.
//!
//! When a TTY is not attached (e.g., `cargo run -- run "…" | jq`), the TUI
//! is bypassed and output is emitted directly to stdout in the requested
//! format.  This module provides the formatter and the streaming collector.

use std::io::{self, Write};

use crate::client::envelope::{RuntimeEnvelope, RuntimeEvent};

/// Output format variants corresponding to `--format` on the `run` subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Markdown,
}

/// Collects `RuntimeEnvelope` events into a final text buffer.
///
/// Designed for use with `run_batch`: callers feed envelopes until
/// `TurnEnd` is received, then call `flush` to write the result.
pub struct BatchOutput {
    format: OutputFormat,
    buffer: String,
}

impl BatchOutput {
    pub fn new(format: OutputFormat) -> Self {
        Self { format, buffer: String::new() }
    }

    /// Ingests one envelope.  Returns `true` when the turn has ended
    /// and no further envelopes are expected.
    pub fn ingest(&mut self, envelope: RuntimeEnvelope) -> bool {
        match envelope.event {
            RuntimeEvent::TranscriptBlockDelta { delta, .. } => {
                self.buffer.push_str(&delta);
                false
            }
            RuntimeEvent::TranscriptLine { line } => {
                self.buffer.push_str(&line);
                self.buffer.push('\n');
                false
            }
            RuntimeEvent::TurnEnd { .. } => true,
            RuntimeEvent::Error { message, .. } => {
                eprintln!("error: {message}");
                true
            }
            _ => false,
        }
    }

    /// Writes the collected output to `stdout`.
    pub fn flush(self) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        match self.format {
            OutputFormat::Text | OutputFormat::Markdown => {
                writeln!(stdout, "{}", self.buffer)
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&self.buffer)
                    .expect("String is always valid JSON string");
                writeln!(stdout, "{json}")
            }
        }
    }
}
