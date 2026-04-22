//! Headless widget rendering tests.
//!
//! `ratatui::backend::TestBackend` renders widgets into an in-memory buffer
//! without a real terminal, enabling assertion on character-level output.
//! This validates layout logic and widget draw methods in isolation.

use ratatui::{Terminal, backend::TestBackend, layout::Rect};

use aozcoder::ui::{PromptArea, TranscriptView};

fn make_terminal(cols: u16, rows: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(cols, rows)).unwrap()
}

#[test]
fn prompt_area_renders_without_panic() {
    let mut terminal = make_terminal(80, 5);
    let mut area_widget = PromptArea::new();
    area_widget.insert_char('h');
    area_widget.insert_char('i');

    terminal
        .draw(|f| {
            area_widget.draw(f, Rect { x: 0, y: 0, width: 80, height: 5 }, true);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let rendered: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();

    assert!(rendered.contains('h'), "rendered buffer should contain 'h'");
}

#[test]
fn transcript_view_renders_user_input() {
    let mut terminal = make_terminal(80, 20);
    let mut transcript = TranscriptView::new();
    transcript.push_user_input("explain lifetimes".to_string());

    terminal
        .draw(|f| {
            transcript.draw(f, Rect { x: 0, y: 0, width: 80, height: 20 });
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let rendered: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();

    assert!(
        rendered.contains('e'),
        "rendered buffer should contain part of 'explain lifetimes'"
    );
}

#[test]
fn prompt_area_backspace_removes_last_char() {
    let mut p = PromptArea::new();
    p.insert_char('a');
    p.insert_char('b');
    p.backspace();
    assert_eq!(p.get_content(), "a");
}

#[test]
fn prompt_area_clear_empties_buffer() {
    let mut p = PromptArea::new();
    p.insert_char('x');
    p.clear();
    assert_eq!(p.get_content(), "");
}
