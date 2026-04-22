//! Block-oriented transcript widget.
//!
//! A transcript is a sequence of `TranscriptEntry` values, each corresponding
//! to a block-level element in the RuntimeEnvelope stream:
//!
//!  - `FinalText`   — assistant text delta, accumulated in-place
//!  - `Thinking`    — extended reasoning, collapsed by default
//!  - `ToolCall`    — structured invocation record
//!  - `ToolResult`  — output of a completed tool call
//!  - `Line`        — raw transcript_line events (tool stderr, system notices)
//!  - `Approval`    — approval_request prompts
//!
//! The index space mirrors the `transcript_block_*` event `index` field.
//! Out-of-order deltas for an unknown index are silently discarded.

use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::client::envelope::{ApprovalScope, AssistantPhase, StreamBlock, ToolStatus, TurnStatus};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub is_error: bool,
}

#[derive(Debug)]
pub enum TranscriptEntry {
    UserInput(String),
    Text { content: String, phase: AssistantPhase },
    Thinking { content: String, collapsed: bool },
    ToolCall(ToolCallRecord),
    Line(String),
    Approval { capability: String, scope: ApprovalScope },
    TurnBoundary(TurnStatus),
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct TranscriptView {
    entries: Vec<TranscriptEntry>,
    /// Maps schema block index → entries index for streaming delta application.
    block_index_map: HashMap<u32, usize>,
    /// Maps tool_call_id → entries index for status updates.
    tool_call_index: HashMap<String, usize>,
    scroll_offset: u16,
    auto_scroll: bool,
    current_phase: AssistantPhase,
}

impl TranscriptView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            block_index_map: HashMap::new(),
            tool_call_index: HashMap::new(),
            scroll_offset: 0,
            auto_scroll: true,
            current_phase: AssistantPhase::Final,
        }
    }

    pub fn begin_turn(&mut self) {
        self.current_phase = AssistantPhase::Final;
    }

    pub fn end_turn(&mut self, status: TurnStatus) {
        self.entries.push(TranscriptEntry::TurnBoundary(status));
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn set_phase(&mut self, phase: AssistantPhase) {
        self.current_phase = phase;
    }

    /// Pushes a user input line before sending.
    pub fn push_user_input(&mut self, text: String) {
        self.entries.push(TranscriptEntry::UserInput(text));
        self.scroll_to_bottom();
    }

    /// Pushes a raw `transcript_line` event.
    pub fn push_line(&mut self, line: String) {
        self.entries.push(TranscriptEntry::Line(line));
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Opens a new block at schema index `index`.
    pub fn start_block(&mut self, index: u32, block: StreamBlock) {
        let entry = match block {
            StreamBlock::FinalText { content } => {
                TranscriptEntry::Text { content, phase: self.current_phase }
            }
            StreamBlock::Thinking { content, collapsed } => {
                TranscriptEntry::Thinking { content, collapsed }
            }
            StreamBlock::ToolCall { id, name, status, .. } => {
                let entry_idx = self.entries.len();
                self.tool_call_index.insert(id.clone(), entry_idx);
                TranscriptEntry::ToolCall(ToolCallRecord {
                    id,
                    name,
                    status,
                    output: None,
                    is_error: false,
                })
            }
            StreamBlock::ToolResult { tool_call_id, output, is_error } => {
                if let Some(&tc_idx) = self.tool_call_index.get(&tool_call_id) {
                    if let Some(TranscriptEntry::ToolCall(ref mut rec)) =
                        self.entries.get_mut(tc_idx)
                    {
                        rec.output = Some(output);
                        rec.is_error = is_error;
                    }
                }
                return;
            }
        };

        let entry_idx = self.entries.len();
        self.block_index_map.insert(index, entry_idx);
        self.entries.push(entry);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Appends `delta` to the block at `index`.
    pub fn append_block_delta(&mut self, index: u32, delta: &str) {
        let Some(&entry_idx) = self.block_index_map.get(&index) else {
            return;
        };
        match self.entries.get_mut(entry_idx) {
            Some(TranscriptEntry::Text { content, .. }) => content.push_str(delta),
            Some(TranscriptEntry::Thinking { content, .. }) => content.push_str(delta),
            _ => {}
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn complete_block(&mut self, _index: u32) {
        // No additional state change required on block completion.
    }

    pub fn start_tool_call(&mut self, tool_call_id: &str, tool_name: &str) {
        let entry_idx = self.entries.len();
        self.tool_call_index.insert(tool_call_id.to_string(), entry_idx);
        self.entries.push(TranscriptEntry::ToolCall(ToolCallRecord {
            id: tool_call_id.to_string(),
            name: tool_name.to_string(),
            status: ToolStatus::Executing,
            output: None,
            is_error: false,
        }));
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn complete_tool_call(&mut self, tool_call_id: &str, output: &str, is_error: bool) {
        let Some(&idx) = self.tool_call_index.get(tool_call_id) else {
            return;
        };
        if let Some(TranscriptEntry::ToolCall(ref mut rec)) = self.entries.get_mut(idx) {
            rec.status = if is_error { ToolStatus::Error } else { ToolStatus::Complete };
            rec.output = Some(output.to_string());
            rec.is_error = is_error;
        }
    }

    pub fn update_tool_status(&mut self, tool_call_id: &str, status: ToolStatus) {
        let Some(&idx) = self.tool_call_index.get(tool_call_id) else {
            return;
        };
        if let Some(TranscriptEntry::ToolCall(ref mut rec)) = self.entries.get_mut(idx) {
            rec.status = status;
        }
    }

    pub fn push_approval_request(&mut self, capability: &str, scope: ApprovalScope) {
        self.entries.push(TranscriptEntry::Approval {
            capability: capability.to_string(),
            scope,
        });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn scroll_up(&mut self, n: u16) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    pub fn resume_auto_scroll(&mut self) {
        self.auto_scroll = true;
        self.scroll_to_bottom();
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.entries.len().saturating_sub(1) as u16;
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(" Transcript ");

        let mut text = Text::default();

        for entry in &self.entries {
            self.render_entry(entry, &mut text);
        }

        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);

        let content_len = self.entries.len();
        if content_len > area.height as usize {
            let mut scrollbar_state =
                ScrollbarState::new(content_len).position(self.scroll_offset as usize);

            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .render(area, frame.buffer_mut(), &mut scrollbar_state);
        }
    }

    fn render_entry<'a>(&'a self, entry: &'a TranscriptEntry, text: &mut Text<'a>) {
        match entry {
            TranscriptEntry::UserInput(s) => {
                text.push_line(Line::from(vec![
                    Span::styled(">>> ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(s.as_str(), Style::default().fg(Color::White)),
                ]));
            }
            TranscriptEntry::Text { content, phase } => {
                let style = match phase {
                    AssistantPhase::Final => Style::default(),
                    AssistantPhase::Thinking => Style::default().fg(Color::DarkGray),
                };
                for line in content.lines() {
                    text.push_line(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(line, style),
                    ]));
                }
            }
            TranscriptEntry::Thinking { content, collapsed } => {
                if *collapsed {
                    text.push_line(Line::from(Span::styled(
                        "    <thinking collapsed>",
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    )));
                } else {
                    text.push_line(Line::from(Span::styled(
                        "    <thinking>",
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    )));
                    for line in content.lines() {
                        text.push_line(Line::from(vec![
                            Span::raw("      "),
                            Span::styled(line, Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
            TranscriptEntry::ToolCall(rec) => {
                let status_color = match rec.status {
                    ToolStatus::Complete => Color::Green,
                    ToolStatus::Error => Color::Red,
                    ToolStatus::Executing => Color::Yellow,
                    ToolStatus::WaitingApproval => Color::Magenta,
                    ToolStatus::Cancelled => Color::DarkGray,
                    ToolStatus::Pending => Color::DarkGray,
                };

                text.push_line(Line::from(vec![
                    Span::styled("[T] ", Style::default().fg(Color::Yellow)),
                    Span::styled(rec.name.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(rec.status.to_string(), Style::default().fg(status_color)),
                ]));

                if let Some(ref output) = rec.output {
                    let preview: String = output.lines().next().unwrap_or("").chars().take(120).collect();
                    text.push_line(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            preview,
                            if rec.is_error {
                                Style::default().fg(Color::Red)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            },
                        ),
                    ]));
                }
            }
            TranscriptEntry::Line(s) => {
                text.push_line(Line::from(vec![
                    Span::styled("[*] ", Style::default().fg(Color::Gray)),
                    Span::styled(s.as_str(), Style::default().fg(Color::Gray)),
                ]));
            }
            TranscriptEntry::Approval { capability, scope } => {
                let scope_label = match scope {
                    ApprovalScope::Once => "once",
                    ApprovalScope::Session => "session",
                };
                text.push_line(Line::from(vec![
                    Span::styled("[?] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("approval required: {capability} (scope: {scope_label})"),
                        Style::default().fg(Color::Magenta),
                    ),
                ]));
            }
            TranscriptEntry::TurnBoundary(status) => {
                let (label, color) = match status {
                    TurnStatus::Completed => ("─── completed ───", Color::DarkGray),
                    TurnStatus::Failed => ("─── failed ───", Color::Red),
                    TurnStatus::Cancelled => ("─── cancelled ───", Color::DarkGray),
                };
                text.push_line(Line::from(
                    Span::styled(label, Style::default().fg(color)),
                ));
            }
        }
    }
}

impl Default for TranscriptView {
    fn default() -> Self {
        Self::new()
    }
}
