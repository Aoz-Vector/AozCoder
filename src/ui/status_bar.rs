//! One-line status bar rendered at the bottom of the terminal.
//!
//! Displays session metadata: model identifier, current tool activity,
//! cumulative token usage, and turn count.  All fields are optional and
//! degrade gracefully when not yet populated by incoming envelopes.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::ConversationState;

/// Mutable state backing the status bar widget.
pub struct StatusBar {
    pub model: Option<String>,
    pub active_tool: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            model: None,
            active_tool: None,
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    pub fn set_model(&mut self, model: &str) {
        self.model = Some(model.to_string());
    }

    pub fn set_tool_active(&mut self, tool_name: String) {
        self.active_tool = Some(tool_name);
    }

    pub fn clear_tool(&mut self) {
        self.active_tool = None;
    }

    pub fn set_usage(&mut self, input: u64, output: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
    }

    /// Renders a single-line status bar.
    ///
    /// Layout: `[model]  [tool activity]           [tokens]  [turns]`
    pub fn draw(&self, frame: &mut Frame, area: Rect, conversation: &ConversationState) {
        let model_label = self.model.as_deref().unwrap_or("connecting…");

        let tool_label = self
            .active_tool
            .as_deref()
            .map(|t| format!(" [{t}] "))
            .unwrap_or_default();

        let token_label = if self.input_tokens > 0 || self.output_tokens > 0 {
            format!(" in:{} out:{} ", self.input_tokens, self.output_tokens)
        } else {
            String::new()
        };

        let turn_label = format!(" turn:{} ", conversation.turn_count());

        let left = Line::from(vec![
            Span::styled(format!(" {model_label}"), Style::default().fg(Color::Cyan)),
            Span::styled(tool_label, Style::default().fg(Color::Yellow)),
        ]);

        let right = Line::from(vec![
            Span::styled(token_label, Style::default().fg(Color::DarkGray)),
            Span::styled(turn_label, Style::default().fg(Color::DarkGray)),
        ]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        frame.render_widget(Paragraph::new(left), chunks[0]);
        frame.render_widget(
            Paragraph::new(right).alignment(ratatui::layout::Alignment::Right),
            chunks[1],
        );
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
