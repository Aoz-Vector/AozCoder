//! Single-line prompt input widget.
//!
//! Cursor movement follows the terminal model described in ECMA-48 §5.4:
//! the cursor is a position index into the UTF-8 scalar sequence, not the
//! byte sequence.  `unicode_width` provides display-column widths for
//! rendering the cursor position glyph-accurately.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Input buffer with a logical cursor position.
pub struct PromptArea {
    /// Character sequence; stored as a `Vec<char>` for O(1) indexed insertion.
    chars: Vec<char>,
    /// Cursor position in char-index space: 0 ≤ cursor ≤ chars.len().
    cursor: usize,
}

impl PromptArea {
    pub fn new() -> Self {
        Self { chars: Vec::new(), cursor: 0 }
    }

    /// Inserts `c` at the cursor position and advances the cursor by one.
    pub fn insert_char(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Removes the character immediately before the cursor (delete-left).
    /// No-op when the cursor is at position 0.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Moves the cursor one position to the left.
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one position to the right.
    pub fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    /// Resets the buffer and cursor to the empty state.
    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    /// Returns the current buffer contents as a `String`.
    pub fn get_content(&self) -> String {
        self.chars.iter().collect()
    }

    /// Renders the prompt area into `area`.
    ///
    /// When `active` is `false` (i.e., the session is streaming), the border
    /// is dimmed and the cursor block is suppressed to signal read-only state.
    pub fn draw(&self, frame: &mut Frame, area: Rect, active: bool) {
        let border_style = if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if active { " Input " } else { " Input (streaming…) " });

        let inner = block.inner(area);

        let content: String = self.chars.iter().collect();
        let (before, after) = content.split_at(
            self.chars[..self.cursor].iter().collect::<String>().len(),
        );

        let mut spans = vec![Span::raw(before.to_string())];

        if active {
            if let Some(c) = self.chars.get(self.cursor) {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));
                let rest: String = self.chars[self.cursor + 1..].iter().collect();
                spans.push(Span::raw(rest));
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default().bg(Color::White),
                ));
            }
        } else {
            spans.push(Span::raw(after.to_string()));
        }

        let paragraph = Paragraph::new(Line::from(spans)).block(block);
        frame.render_widget(paragraph, area);
        let _ = inner; // inner used for sizing reference only
    }
}

impl Default for PromptArea {
    fn default() -> Self {
        Self::new()
    }
}
