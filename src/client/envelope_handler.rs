//! Dispatch layer between incoming `RuntimeEnvelope` values and `App` state.
//!
//! `apply` is a pure function — it takes a mutable reference to `App` and
//! a `RuntimeEnvelope` and updates state accordingly.  No I/O occurs here;
//! all side-effectful work (rendering, network) remains in the calling layer.

use crate::{
    client::envelope::{RuntimeEnvelope, RuntimeEvent},
    tui::app::App,
};

/// Applies a received envelope to the application state.
///
/// Variants not handled below are logged at the `debug` level and ignored;
/// this preserves forward-compatibility with schema additions.
pub fn apply(app: &mut App, envelope: RuntimeEnvelope) {
    tracing::debug!(
        seq = envelope.seq,
        turn = envelope.turn,
        event_type = envelope.event.type_name(),
        "applying envelope"
    );

    match envelope.event {
        RuntimeEvent::TurnStart { .. } => {
            app.streaming = true;
            app.error_message = None;
            app.transcript_view.begin_turn();
        }

        RuntimeEvent::TranscriptLine { line } => {
            app.transcript_view.push_line(line);
        }

        RuntimeEvent::TranscriptBlockStart { index, block } => {
            app.transcript_view.start_block(index, block);
        }

        RuntimeEvent::TranscriptBlockDelta { index, delta } => {
            app.transcript_view.append_block_delta(index, &delta);
        }

        RuntimeEvent::TranscriptBlockComplete { index } => {
            app.transcript_view.complete_block(index);
        }

        RuntimeEvent::TranscriptBlockPhaseUpdated { phase, .. } => {
            app.transcript_view.set_phase(phase);
        }

        RuntimeEvent::ToolCallStarted {
            tool_call_id,
            tool_name,
            ..
        } => {
            app.transcript_view
                .start_tool_call(&tool_call_id, &tool_name);
            app.status_bar.set_tool_active(tool_name);
        }

        RuntimeEvent::ToolCallCompleted {
            tool_call_id,
            output,
            duration_ms,
            ..
        } => {
            app.transcript_view
                .complete_tool_call(&tool_call_id, &output, false);
            app.status_bar.clear_tool();
            if let Some(ms) = duration_ms {
                tracing::debug!(tool_call_id = %tool_call_id, duration_ms = ms, "tool completed");
            }
        }

        RuntimeEvent::ToolCallFailed {
            tool_call_id,
            output,
            ..
        } => {
            app.transcript_view
                .complete_tool_call(&tool_call_id, &output, true);
            app.status_bar.clear_tool();
        }

        RuntimeEvent::ToolCallStatusUpdated {
            tool_call_id,
            status,
        } => {
            app.transcript_view
                .update_tool_status(&tool_call_id, status);
        }

        RuntimeEvent::ToolCallArgumentsDelta { .. } => {
            // Argument streaming is not rendered in the transcript;
            // the completed arguments appear on ToolCallCompleted.
        }

        RuntimeEvent::ApprovalRequest {
            capability, scope, ..
        } => {
            app.transcript_view
                .push_approval_request(&capability, scope);
        }

        RuntimeEvent::TurnEnd { status, usage, .. } => {
            app.streaming = false;
            app.transcript_view.end_turn(status);
            if let Some(u) = usage {
                app.status_bar.set_usage(u.input, u.output);
            }
        }

        RuntimeEvent::Error {
            message,
            recoverable,
            code,
        } => {
            tracing::error!(code = %code, recoverable = recoverable, "runtime error");
            if !recoverable {
                app.streaming = false;
                app.error_message = Some(message);
            } else {
                app.transcript_view
                    .push_line(format!("[error:{code}] {message}"));
            }
        }

        RuntimeEvent::MaxTurnsReached { max_turns } => {
            app.streaming = false;
            app.transcript_view
                .push_line(format!("Max turns ({max_turns}) reached — session ended."));
        }

        RuntimeEvent::ApprovalResolved { .. } | RuntimeEvent::ValidationResult { .. } => {
            tracing::debug!("envelope handled passively");
        }
    }
}
