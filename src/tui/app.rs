//! Application state and rendering root.
//!
//! `App` is the single source of truth for the TUI.  It owns the conversation
//! state, all widgets, and the current streaming flag.  It does not perform
//! I/O; the event loop in `tui::event_loop` drives state transitions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    client::envelope::RuntimeEnvelope,
    client::envelope_handler,
    state::ConversationState,
    ui::{PromptArea, StatusBar, TranscriptView},
};

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    pub conversation: ConversationState,
    pub transcript_view: TranscriptView,
    pub prompt_area: PromptArea,
    pub status_bar: StatusBar,
    pub session_id: String,
    pub streaming: bool,
    pub error_message: Option<String>,
}

impl App {
    pub fn new(session_id: String) -> Self {
        Self {
            conversation: ConversationState::new(session_id.clone()),
            transcript_view: TranscriptView::new(),
            prompt_area: PromptArea::new(),
            status_bar: StatusBar::new(),
            session_id,
            streaming: false,
            error_message: None,
        }
    }

    /// Applies an incoming envelope to all relevant state components.
    pub fn handle_envelope(&mut self, envelope: RuntimeEnvelope) {
        envelope_handler::apply(self, envelope);
    }

    /// Processes a keypress and returns an `AppEvent` for actions that require
    /// coordination outside the `App` (submit, quit, interrupt).
    /// Returns `None` for local edits (char insertion, backspace, cursor movement).
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    if self.streaming {
                        Some(AppEvent::Interrupt)
                    } else {
                        Some(AppEvent::Quit)
                    }
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppEvent::Quit),
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.error_message = None;
                    None
                }
                _ => None,
            };
        }

        if self.streaming {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let text = self.prompt_area.get_content();
                if text.trim().is_empty() {
                    return None;
                }
                self.prompt_area.clear();
                self.transcript_view.push_user_input(text.clone());
                Some(AppEvent::Submit(text))
            }
            KeyCode::Char(c) => {
                self.prompt_area.insert_char(c);
                None
            }
            KeyCode::Backspace => {
                self.prompt_area.backspace();
                None
            }
            KeyCode::Left => {
                self.prompt_area.move_left();
                None
            }
            KeyCode::Right => {
                self.prompt_area.move_right();
                None
            }
            KeyCode::Esc => {
                self.error_message = None;
                self.prompt_area.clear();
                None
            }
            KeyCode::Up => {
                self.transcript_view.scroll_up(3);
                None
            }
            KeyCode::Down => {
                self.transcript_view.scroll_down(3);
                None
            }
            KeyCode::End => {
                self.transcript_view.resume_auto_scroll();
                None
            }
            _ => None,
        }
    }

    /// Renders the full terminal frame.
    ///
    /// Layout (top → bottom):
    ///   1. Transcript  (fills remaining height)
    ///   2. Prompt area (3 rows)
    ///   3. Status bar  (1 row)
    pub fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.transcript_view.draw(frame, chunks[0]);
        self.prompt_area.draw(frame, chunks[1], !self.streaming);
        self.status_bar.draw(frame, chunks[2], &self.conversation);

        if let Some(ref msg) = self.error_message {
            render_error_overlay(frame, frame.area(), msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Events returned to the event loop
// ---------------------------------------------------------------------------

/// Events produced by key handling that require coordination with I/O.
#[derive(Debug)]
pub enum AppEvent {
    Submit(String),
    Interrupt,
    Quit,
}

// ---------------------------------------------------------------------------
// Error overlay
// ---------------------------------------------------------------------------

fn render_error_overlay(frame: &mut Frame, area: Rect, message: &str) {
    let width = (area.width / 2).max(40).min(area.width.saturating_sub(4));
    let height: u16 = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let popup = Rect { x, y, width, height };

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Error — Ctrl-L to dismiss ");

    let text = Text::from(vec![
        Line::from(Span::styled(message, Style::default().fg(Color::Red))),
    ]);

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        popup,
    );
}
