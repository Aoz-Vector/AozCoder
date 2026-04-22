//! Canonical RuntimeEnvelope v1 types.
//!
//! Structures correspond 1:1 with `schemas/runtime_envelope_v1.json`.
//! Serde configuration uses `#[serde(tag = "type", rename_all = "snake_case")]`
//! (internally tagged union, RFC 7159 §4) so that discriminant resolution
//! occurs purely on the `"type"` field of the incoming JSON object.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level envelope
// ---------------------------------------------------------------------------

/// A single framed event as emitted by the Vexcoder runtime over SSE.
///
/// Field names mirror the JSON Schema identifiers verbatim so that
/// `serde_json::from_str` requires zero field-renaming configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEnvelope {
    pub version: u32,
    pub task_id: String,
    pub turn: u32,
    pub seq: u32,
    pub event_id: String,
    pub emitted_at: DateTime<Utc>,
    pub source: EnvelopeSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<String>,
    pub event: RuntimeEvent,
}

/// Indicates which subsystem produced the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeSource {
    Model,
    Runtime,
    UserRequest,
    System,
}

// ---------------------------------------------------------------------------
// Runtime event variants
// ---------------------------------------------------------------------------

/// Discriminated union of all event types defined in the schema's `$defs`.
///
/// Serialization uses the internal tag `"type"` with snake_case names.
/// Unknown variants cause a deserialization error; callers should handle
/// `serde_json::Error` and skip unrecognised events rather than aborting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    TurnStart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<String>,
    },
    TranscriptLine {
        line: String,
    },
    TranscriptBlockStart {
        index: u32,
        block: StreamBlock,
    },
    TranscriptBlockDelta {
        index: u32,
        delta: String,
    },
    TranscriptBlockComplete {
        index: u32,
    },
    TranscriptBlockPhaseUpdated {
        index: u32,
        phase: AssistantPhase,
        streaming: bool,
    },
    ToolCallStarted {
        tool_call_id: String,
        tool_name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
        status: ToolStatus,
        started_at: DateTime<Utc>,
    },
    ToolCallArgumentsDelta {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        delta: String,
        status: ToolStatus,
        #[serde(default)]
        invalid_json: bool,
    },
    ToolCallCompleted {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        status: ToolStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        started_at: Option<DateTime<Utc>>,
        completed_at: DateTime<Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        output: String,
    },
    ToolCallFailed {
        tool_call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        status: ToolStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        started_at: Option<DateTime<Utc>>,
        completed_at: DateTime<Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        output: String,
    },
    ToolCallStatusUpdated {
        tool_call_id: String,
        status: ToolStatus,
    },
    ApprovalRequest {
        capability: String,
        scope: ApprovalScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
    },
    ApprovalResolved {
        capability: String,
        scope: ApprovalScope,
        approved: bool,
    },
    ValidationResult {
        passed: bool,
        outputs: Vec<ValidationOutput>,
    },
    TurnEnd {
        status: TurnStatus,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
        changed_files: Vec<String>,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
    MaxTurnsReached {
        max_turns: u32,
    },
}

// ---------------------------------------------------------------------------
// Nested types
// ---------------------------------------------------------------------------

/// A structured block within a streaming transcript segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamBlock {
    Thinking {
        content: String,
        collapsed: bool,
    },
    ToolCall {
        id: String,
        name: String,
        input: serde_json::Map<String, serde_json::Value>,
        status: ToolStatus,
    },
    ToolResult {
        tool_call_id: String,
        output: String,
        is_error: bool,
    },
    FinalText {
        content: String,
    },
}

/// Lifecycle state of a tool invocation.
///
/// Values correspond exactly to the `tool_status` enum in the schema:
/// `pending | waiting_approval | executing | complete | error | cancelled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Pending,
    WaitingApproval,
    Executing,
    Complete,
    Error,
    Cancelled,
}

/// Streaming phase of an assistant turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantPhase {
    Thinking,
    Final,
}

/// Scope of an approval grant: single-use or session-wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalScope {
    Once,
    Session,
}

/// Terminal status of a completed turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Completed,
    Failed,
    Cancelled,
}

/// Aggregate token consumption for a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    #[serde(default)]
    pub estimated: bool,
}

/// Structured output record from a post-turn validation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutput {
    pub label: String,
    pub exit_code: i32,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl RuntimeEvent {
    /// Returns the schema discriminant string for logging and display purposes.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TurnStart { .. } => "turn_start",
            Self::TranscriptLine { .. } => "transcript_line",
            Self::TranscriptBlockStart { .. } => "transcript_block_start",
            Self::TranscriptBlockDelta { .. } => "transcript_block_delta",
            Self::TranscriptBlockComplete { .. } => "transcript_block_complete",
            Self::TranscriptBlockPhaseUpdated { .. } => "transcript_block_phase_updated",
            Self::ToolCallStarted { .. } => "tool_call_started",
            Self::ToolCallArgumentsDelta { .. } => "tool_call_arguments_delta",
            Self::ToolCallCompleted { .. } => "tool_call_completed",
            Self::ToolCallFailed { .. } => "tool_call_failed",
            Self::ToolCallStatusUpdated { .. } => "tool_call_status_updated",
            Self::ApprovalRequest { .. } => "approval_request",
            Self::ApprovalResolved { .. } => "approval_resolved",
            Self::ValidationResult { .. } => "validation_result",
            Self::TurnEnd { .. } => "turn_end",
            Self::Error { .. } => "error",
            Self::MaxTurnsReached { .. } => "max_turns_reached",
        }
    }
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pending => "pending",
            Self::WaitingApproval => "waiting_approval",
            Self::Executing => "executing",
            Self::Complete => "complete",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        })
    }
}
