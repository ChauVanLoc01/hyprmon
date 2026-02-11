use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use super::{BOX_GAP, BOX_HEIGHT, BOX_WIDTH};
use crate::app::App;
use crate::state::{FocusPanel, MainTab, SettingField};

pub fn render_saved_arrangement_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus_panel == FocusPanel::Arrangement && app.main_tab == MainTab::Saved;
    let border_style = if is_focused {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let ws_name = app.current_workspace_name();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Saved Monitors - {} ", ws_name));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.saved_monitors.is_empty() {
        let text = Paragraph::new("No monitors saved in this workspace.\nSwitch to Live panel and Apply to save current monitors.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, inner);
        return;
    }

    let total_monitors = app.saved_monitors.len();
    let total_width =
        (BOX_WIDTH * total_monitors as u16) + (BOX_GAP * (total_monitors as u16).saturating_sub(1));
    let start_x = inner.x + (inner.width.saturating_sub(total_width)) / 2;
    let start_y = inner.y + (inner.height.saturating_sub(BOX_HEIGHT)) / 2;

    for (i, monitor) in app.saved_monitors.iter().enumerate() {
        let x = start_x + (i as u16 * (BOX_WIDTH + BOX_GAP));
        let y = start_y;

        let is_selected = i == app.saved_selected_monitor;
        let monitor_area = Rect::new(x, y, BOX_WIDTH, BOX_HEIGHT);

        let border_type = if is_selected {
            symbols::border::DOUBLE
        } else {
            symbols::border::PLAIN
        };

        let style = if is_selected {
            Style::default().fg(Color::Magenta)
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

        let label_style = if is_selected {
            Style::default().fg(Color::Magenta).bold()
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
    let help = "←→/hl Select | Edit settings below";
    let help_area = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        help_area,
    );
}

pub fn render_saved_settings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus_panel == FocusPanel::Settings && app.main_tab == MainTab::Saved;
    let border_style = if is_focused {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let monitor = match app.saved_monitors.get(app.saved_selected_monitor) {
        Some(m) => m,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Saved Settings ");
            frame.render_widget(block, area);
            return;
        }
    };

    let title = format!(" Saved Settings - {} ", monitor.display_name());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fields = SettingField::all();
    let mut y = inner.y + 1;

    for (i, field) in fields.iter().enumerate() {
        let is_selected = i == app.saved_selected_setting && is_focused;
        let cursor = if is_selected { ">" } else { " " };

        let style = if is_selected {
            Style::default().fg(Color::Magenta).bold()
        } else {
            Style::default()
        };

        match field {
            SettingField::Primary => {
                y += 1;
                let checkbox = if monitor.is_primary { "[x]" } else { "[ ]" };
                let line = format!(" {} {} Primary monitor", cursor, checkbox);
                frame.render_widget(
                    Paragraph::new(line).style(style),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }
            _ => {
                let value = match field {
                    SettingField::Resolution => monitor.resolution.clone(),
                    SettingField::RefreshRate => format!("{:.0} Hz", monitor.refresh_rate),
                    SettingField::Scale => format!("{:.0}%", monitor.scale * 100.0),
                    SettingField::Rotation => monitor.rotation.as_str().to_string(),
                    _ => String::new(),
                };

                let line = format!(" {} {:<14} {:<14}", cursor, field.label(), value);
                frame.render_widget(
                    Paragraph::new(line).style(style),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }
        }
        y += 1;
    }

    // Note about editing
    y += 1;
    frame.render_widget(
        Paragraph::new(" Note: Saved configs are read-only. Edit in Live panel.")
            .style(Style::default().fg(Color::DarkGray).italic()),
        Rect::new(inner.x, y, inner.width, 1),
    );
}
