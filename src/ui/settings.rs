use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::state::{FocusPanel, SettingField};

pub fn render_settings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus_panel == FocusPanel::Settings;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let monitor = match app.current_monitor() {
        Some(m) => m,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Settings ");
            frame.render_widget(block, area);
            return;
        }
    };

    let title = format!(
        " Settings for Monitor {} ({}) ",
        app.selected_monitor + 1,
        monitor.display_name()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fields = SettingField::all();
    let mut y = inner.y + 1;

    for (i, field) in fields.iter().enumerate() {
        let is_selected = i == app.selected_setting && is_focused;
        let cursor = if is_selected { ">" } else { " " };

        let style = if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default()
        };

        match field {
            SettingField::Primary => {
                y += 1; // Extra spacing
                let checkbox = if monitor.is_primary { "[x]" } else { "[ ]" };
                let line = format!(" {} {} Set as primary monitor", cursor, checkbox);
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

                // Format: " > Label:          Value          [Change]"
                let label = field.label();
                let line = format!(" {} {:<14} {:<14} [Change]", cursor, label, value);
                frame.render_widget(
                    Paragraph::new(line).style(style),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }
        }
        y += 1;
    }

    // Status message
    if !app.message.is_empty() {
        y += 1;
        frame.render_widget(
            Paragraph::new(app.message.as_str()).style(Style::default().fg(Color::Green)),
            Rect::new(inner.x + 1, y, inner.width - 2, 1),
        );
    }
}

/// Returns the row index for each setting field (for mouse click detection)
#[allow(dead_code)]
pub fn get_setting_row(setting_index: usize, panel_start_y: usize) -> usize {
    // Row calculation: panel_start_y + 2 (border + padding) + setting_index
    // Primary has an extra row of spacing before it
    if setting_index == 4 {
        // Primary field
        panel_start_y + 2 + setting_index + 1
    } else {
        panel_start_y + 2 + setting_index
    }
}

/// Converts a row position to a setting index, returns None if not on a setting
pub fn row_to_setting(row: usize, panel_start_y: usize) -> Option<usize> {
    if row < panel_start_y + 2 {
        return None;
    }

    let relative_row = row - panel_start_y - 2;

    // Account for extra spacing before Primary checkbox
    if relative_row <= 3 {
        Some(relative_row)
    } else if relative_row == 5 {
        // Primary checkbox (after spacing)
        Some(4)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::row_to_setting;

    #[test]
    fn maps_regular_setting_rows() {
        let panel_start = 10;
        assert_eq!(row_to_setting(panel_start + 2, panel_start), Some(0)); // Resolution
        assert_eq!(row_to_setting(panel_start + 3, panel_start), Some(1)); // Refresh Rate
        assert_eq!(row_to_setting(panel_start + 4, panel_start), Some(2)); // Scale
        assert_eq!(row_to_setting(panel_start + 5, panel_start), Some(3)); // Rotation
    }

    #[test]
    fn skips_spacing_and_maps_primary_row() {
        let panel_start = 10;
        assert_eq!(row_to_setting(panel_start + 6, panel_start), None); // spacing row
        assert_eq!(row_to_setting(panel_start + 7, panel_start), Some(4)); // Primary
    }
}
