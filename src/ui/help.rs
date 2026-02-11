use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::MainTab;

pub fn render_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let spans = match app.main_tab {
        MainTab::Live => create_live_help(),
        MainTab::Saved => create_saved_help(),
    };

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
}

fn key_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn desc_style() -> Style {
    Style::default().fg(Color::White)
}

fn sep_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn create_live_help() -> Vec<Span<'static>> {
    vec![
        Span::styled("1", key_style()),
        Span::styled("/", sep_style()),
        Span::styled("2", key_style()),
        Span::styled(" Tab", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("←→", key_style()),
        Span::styled(" Select", desc_style()),
        Span::styled("  ", sep_style()),
        Span::styled("⇧←→", key_style()),
        Span::styled(" Move", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("P", key_style()),
        Span::styled(" Primary", desc_style()),
        Span::styled("  ", sep_style()),
        Span::styled("I", key_style()),
        Span::styled(" Identify", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("R", key_style()),
        Span::styled(" Refresh", desc_style()),
        Span::styled("  ", sep_style()),
        Span::styled("A", key_style()),
        Span::styled(" Apply", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("Q", key_style()),
        Span::styled(" Quit", desc_style()),
    ]
}

fn create_saved_help() -> Vec<Span<'static>> {
    vec![
        Span::styled("1", key_style()),
        Span::styled("/", sep_style()),
        Span::styled("2", key_style()),
        Span::styled(" Tab", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("[", key_style()),
        Span::styled("/", sep_style()),
        Span::styled("]", key_style()),
        Span::styled(" Workspace", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("N", key_style()),
        Span::styled(" New", desc_style()),
        Span::styled("  ", sep_style()),
        Span::styled("R", key_style()),
        Span::styled(" Rename", desc_style()),
        Span::styled("  ", sep_style()),
        Span::styled("D", key_style()),
        Span::styled(" Delete", desc_style()),
        Span::styled("  │  ", sep_style()),
        Span::styled("Q", key_style()),
        Span::styled(" Quit", desc_style()),
    ]
}
