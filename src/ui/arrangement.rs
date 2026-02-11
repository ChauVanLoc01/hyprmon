use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use super::{BOX_GAP, BOX_HEIGHT, BOX_WIDTH};
use crate::app::App;
use crate::state::{DragState, FocusPanel};

pub fn render_arrangement_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus_panel == FocusPanel::Arrangement;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Monitor Arrangement ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.monitors.is_empty() {
        let text = Paragraph::new("No monitors detected.").alignment(Alignment::Center);
        frame.render_widget(text, inner);
        return;
    }

    let total_monitors = app.monitors.len();
    let total_width = (BOX_WIDTH * total_monitors as u16) + (BOX_GAP * (total_monitors as u16 - 1));
    let base_start_x = inner.x + (inner.width.saturating_sub(total_width)) / 2;
    let base_start_y = inner.y + (inner.height.saturating_sub(BOX_HEIGHT)) / 2;

    // Calculate drag offsets
    let (drag_offset_x, drag_offset_y): (i16, i16) = match app.drag_state {
        DragState::Dragging {
            current_x,
            start_x,
            current_y,
            start_y,
            ..
        } => (
            current_x as i16 - start_x as i16,
            current_y as i16 - start_y as i16,
        ),
        DragState::None => (0, 0),
    };

    for (i, monitor) in app.monitors.iter().enumerate() {
        let base_x = base_start_x + (i as u16 * (BOX_WIDTH + BOX_GAP));

        let (x, y) = match app.drag_state {
            DragState::Dragging { monitor_idx, .. } if monitor_idx == i => {
                let new_x = (base_x as i16 + drag_offset_x).max(inner.x as i16) as u16;
                let new_y = (base_start_y as i16 + drag_offset_y).max(inner.y as i16) as u16;
                (
                    new_x.min(inner.x + inner.width - BOX_WIDTH),
                    new_y.min(inner.y + inner.height - BOX_HEIGHT),
                )
            }
            _ => (base_x, base_start_y),
        };

        let is_selected = i == app.selected_monitor;
        let is_dragging =
            matches!(app.drag_state, DragState::Dragging { monitor_idx, .. } if monitor_idx == i);
        let monitor_area = Rect::new(x, y, BOX_WIDTH, BOX_HEIGHT);

        let border_type = if is_selected {
            symbols::border::DOUBLE
        } else {
            symbols::border::PLAIN
        };

        let style = if is_dragging {
            Style::default().fg(Color::Green).bold()
        } else if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border_type)
            .border_style(style);

        frame.render_widget(block, monitor_area);

        // Monitor number + primary indicator
        let primary_mark = if monitor.is_primary { "*" } else { " " };
        let number_label = format!("{}{}", primary_mark, i + 1);
        let number_area = Rect::new(x + 1, y + 1, BOX_WIDTH - 2, 1);

        let label_style = if is_dragging {
            Style::default().fg(Color::Green).bold()
        } else if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default()
        };

        frame.render_widget(
            Paragraph::new(number_label)
                .style(label_style)
                .alignment(Alignment::Center),
            number_area,
        );

        // Monitor name
        let name = monitor.display_name();
        let display_name = if name.len() > (BOX_WIDTH - 2) as usize {
            format!("{}…", &name[..(BOX_WIDTH as usize - 3)])
        } else {
            name
        };

        let name_area = Rect::new(x + 1, y + 2, BOX_WIDTH - 2, 1);
        frame.render_widget(
            Paragraph::new(display_name)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center),
            name_area,
        );

        // Resolution
        let res_area = Rect::new(x + 1, y + 3, BOX_WIDTH - 2, 1);
        frame.render_widget(
            Paragraph::new(monitor.resolution.as_str())
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            res_area,
        );
    }

    // Help text
    let help = if matches!(app.drag_state, DragState::Dragging { .. }) {
        "Dragging... Release to set new position."
    } else {
        "Drag to move | ←→/hl Select | Shift+←→/HL Reorder | P Primary | I Identify"
    };
    let help_area = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        help_area,
    );
}
