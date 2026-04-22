//! Conversation state — RFC 1945 §10 (stateless HTTP) analogue for SSE sessions.
//!
//! `ConversationState` holds the ordered history of turns for the current
//! session.  It is updated by `App::handle_envelope` and read by the status bar
//! for token counts and turn number display.

use chrono::{DateTime, Utc};

/// A single completed or in-progress turn.
#[derive(Debug, Clone)]
pub struct Turn {
    pub index: u32,
    pub user_input: Option<String>,
    pub started_at: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Session-scoped conversation history.
#[derive(Debug, Default)]
pub struct ConversationState {
    pub session_id: String,
    pub turns: Vec<Turn>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

impl ConversationState {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            ..Default::default()
        }
    }

    pub fn begin_turn(&mut self, user_input: Option<String>) -> u32 {
        let index = self.turns.len() as u32 + 1;
        self.turns.push(Turn {
            index,
            user_input,
            started_at: Utc::now(),
            input_tokens: 0,
            output_tokens: 0,
        });
        index
    }

    pub fn record_usage(&mut self, input: u64, output: u64) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        if let Some(t) = self.turns.last_mut() {
            t.input_tokens = input;
            t.output_tokens = output;
        }
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }
}
