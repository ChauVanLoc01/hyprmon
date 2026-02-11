use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use super::centered_rect;
use crate::app::App;

pub fn render_dropdown(frame: &mut Frame, area: Rect, app: &App) {
    let options = app.get_dropdown_options();
    if options.is_empty() {
        return;
    }

    let height = (options.len() + 2).min(10) as u16;
    let width = options.iter().map(|s| s.len()).max().unwrap_or(10) as u16 + 6;

    // Position dropdown BELOW the selected setting row, aligned with value column
    let x = area.x + 18; // Align with value column (after label)
    let y = area.y + 3 + app.selected_setting as u16; // One row below the setting

    let dropdown_area = Rect::new(
        x.min(area.x + area.width - width),
        y.min(area.y + area.height - height),
        width.max(20), // Minimum width for readability
        height,
    );

    // Only clear the exact dropdown area
    frame.render_widget(Clear, dropdown_area);

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == app.dropdown_selection {
                Style::default().bg(Color::Cyan).fg(Color::Black)
            } else {
                Style::default()
            };
            ListItem::new(format!(" {} ", opt)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Select "),
    );

    let mut state = ListState::default();
    state.select(Some(app.dropdown_selection));
    frame.render_stateful_widget(list, dropdown_area, &mut state);
}

pub fn render_confirm_apply_dialog(frame: &mut Frame, countdown: u8) {
    let area = centered_rect(50, 7, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!(
        "Do you want to keep these changes?\n\n[Y] Yes    [N] No\n\nAuto-revert in {} seconds",
        countdown
    );

    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White)),
        inner,
    );
}

pub fn render_confirm_quit_dialog(frame: &mut Frame) {
    let area = centered_rect(50, 6, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Warning ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = "You have unsaved changes.\nAre you sure you want to quit?\n\n[Y] Yes    [N] No";

    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White)),
        inner,
    );
}

pub fn render_input_dialog(frame: &mut Frame, title: &str, input: &str, hint: &str) {
    let area = centered_rect(50, 5, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", title));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!(
        "{}\n\n> {}█\n\n{}",
        hint, input, "Enter to confirm | Esc to cancel"
    );

    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White)),
        inner,
    );
}
