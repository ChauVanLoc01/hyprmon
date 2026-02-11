use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::MainTab;

pub fn render_main_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let is_live = app.main_tab == MainTab::Live;
    
    // Calculate positions for centered tabs
    let live_text = " 1 Live ";
    let saved_text = " 2 Saved ";
    let gap = 2;
    let total_width = live_text.len() + saved_text.len() + gap + 4; // +4 for borders
    let start_x = area.x + (area.width.saturating_sub(total_width as u16)) / 2;
    
    // Live tab box
    let live_width = live_text.len() as u16 + 2;
    let live_area = Rect::new(start_x, area.y, live_width, 3);
    
    if is_live {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(block, live_area);
        frame.render_widget(
            Paragraph::new(live_text)
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center),
            Rect::new(live_area.x + 1, live_area.y + 1, live_area.width - 2, 1),
        );
    } else {
        frame.render_widget(
            Paragraph::new(live_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            Rect::new(live_area.x + 1, live_area.y + 1, live_area.width - 2, 1),
        );
    }
    
    // Saved tab box
    let saved_start = start_x + live_width + gap as u16;
    let saved_width = saved_text.len() as u16 + 2;
    let saved_area = Rect::new(saved_start, area.y, saved_width, 3);
    
    if !is_live {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, saved_area);
        frame.render_widget(
            Paragraph::new(saved_text)
                .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center),
            Rect::new(saved_area.x + 1, saved_area.y + 1, saved_area.width - 2, 1),
        );
    } else {
        frame.render_widget(
            Paragraph::new(saved_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            Rect::new(saved_area.x + 1, saved_area.y + 1, saved_area.width - 2, 1),
        );
    }
}

pub fn render_workspace_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = vec![Span::styled(
        " Workspaces: ",
        Style::default().fg(Color::DarkGray),
    )];

    for (i, ws) in app.monitor_db.workspaces.iter().enumerate() {
        let is_selected = i == app.selected_workspace;

        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }

        if is_selected {
            spans.push(Span::styled("▸ ", Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(
                ws.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                ws.name.clone(),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // Add [+] button
    spans.push(Span::styled("   ", Style::default()));
    spans.push(Span::styled("[", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        "+",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));

    let line = Line::from(spans);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(line), inner);
}
