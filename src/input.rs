use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::App;
use crate::state::{DialogType, DragState, FocusPanel, MainTab, SettingField};
use crate::ui::{settings::row_to_setting, BOX_GAP, BOX_WIDTH};

pub enum InputResult {
    Continue,
    Quit,
}

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> InputResult {
    match app.dialog {
        DialogType::ConfirmApply { .. } => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm_changes();
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.revert_changes();
                let _ = app.save_and_apply();
                app.dialog = DialogType::None;
            }
            _ => {}
        },
        DialogType::ConfirmQuit => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                return InputResult::Quit;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.dialog = DialogType::None;
            }
            _ => {}
        },
        DialogType::EditDropdown => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if app.dropdown_selection > 0 {
                    app.dropdown_selection -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = app.get_dropdown_options().len().saturating_sub(1);
                if app.dropdown_selection < max {
                    app.dropdown_selection += 1;
                }
            }
            KeyCode::Enter => {
                app.apply_dropdown_selection();
                app.dialog = DialogType::None;
            }
            KeyCode::Esc => {
                app.dialog = DialogType::None;
            }
            _ => {}
        },
        DialogType::NewWorkspace => match code {
            KeyCode::Enter => {
                if !app.input_buffer.is_empty() {
                    let name = app.input_buffer.clone();
                    app.create_workspace(&name);
                    app.input_buffer.clear();
                    app.dialog = DialogType::None;
                }
            }
            KeyCode::Esc => {
                app.input_buffer.clear();
                app.dialog = DialogType::None;
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                if app.input_buffer.len() < 20 {
                    app.input_buffer.push(c);
                }
            }
            _ => {}
        },
        DialogType::RenameWorkspace => match code {
            KeyCode::Enter => {
                if !app.input_buffer.is_empty() {
                    let name = app.input_buffer.clone();
                    app.rename_current_workspace(&name);
                    app.input_buffer.clear();
                    app.dialog = DialogType::None;
                }
            }
            KeyCode::Esc => {
                app.input_buffer.clear();
                app.dialog = DialogType::None;
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                if app.input_buffer.len() < 20 {
                    app.input_buffer.push(c);
                }
            }
            _ => {}
        },
        DialogType::DeleteWorkspace => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.delete_current_workspace();
                app.dialog = DialogType::None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.dialog = DialogType::None;
            }
            _ => {}
        },
        DialogType::None => {
            match code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    if app.has_changes {
                        app.dialog = DialogType::ConfirmQuit;
                    } else {
                        return InputResult::Quit;
                    }
                }
                // Main tab switching
                KeyCode::Char('1') => {
                    app.switch_tab(MainTab::Live);
                }
                KeyCode::Char('2') => {
                    app.switch_tab(MainTab::Saved);
                }
                // Workspace navigation (in Saved panel)
                KeyCode::Char('[') => {
                    if app.main_tab == MainTab::Saved {
                        app.prev_workspace();
                    }
                }
                KeyCode::Char(']') => {
                    if app.main_tab == MainTab::Saved {
                        app.next_workspace();
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if app.main_tab == MainTab::Saved {
                        app.input_buffer.clear();
                        app.dialog = DialogType::NewWorkspace;
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if app.main_tab == MainTab::Saved {
                        app.dialog = DialogType::DeleteWorkspace;
                    }
                }
                KeyCode::Tab => {
                    if modifiers.contains(KeyModifiers::SHIFT) {
                        app.select_next_monitor();
                    } else {
                        app.focus_panel = match app.focus_panel {
                            FocusPanel::Arrangement => FocusPanel::Settings,
                            FocusPanel::Settings => FocusPanel::Arrangement,
                        };
                    }
                }
                KeyCode::BackTab => {
                    app.select_next_monitor();
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    if app.main_tab == MainTab::Live {
                        app.toggle_primary();
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if app.main_tab == MainTab::Live {
                        if let Err(e) = app.save_and_apply() {
                            app.message = format!("Error: {}", e);
                        }
                    }
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if app.main_tab == MainTab::Live {
                        app.identify();
                        app.message = "Identifying monitors... Check your displays!".to_string();
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    if app.main_tab == MainTab::Live {
                        if let Err(e) = app.refresh() {
                            app.message = format!("Error: {}", e);
                        }
                    } else if app.main_tab == MainTab::Saved {
                        // R for Rename in Saved panel
                        app.input_buffer = app.current_workspace_name();
                        app.dialog = DialogType::RenameWorkspace;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.focus_panel == FocusPanel::Arrangement {
                        match app.main_tab {
                            MainTab::Live => {
                                if modifiers.contains(KeyModifiers::SHIFT) {
                                    app.move_monitor_left();
                                } else {
                                    app.select_prev_monitor();
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_monitor > 0 {
                                    app.saved_selected_monitor -= 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if app.focus_panel == FocusPanel::Arrangement {
                        match app.main_tab {
                            MainTab::Live => {
                                if modifiers.contains(KeyModifiers::SHIFT) {
                                    app.move_monitor_right();
                                } else {
                                    app.select_next_monitor();
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_monitor
                                    < app.saved_monitors.len().saturating_sub(1)
                                {
                                    app.saved_selected_monitor += 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.focus_panel == FocusPanel::Settings {
                        match app.main_tab {
                            MainTab::Live => {
                                if app.selected_setting > 0 {
                                    app.selected_setting -= 1;
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_setting > 0 {
                                    app.saved_selected_setting -= 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.focus_panel == FocusPanel::Settings {
                        let max = SettingField::all().len() - 1;
                        match app.main_tab {
                            MainTab::Live => {
                                if app.selected_setting < max {
                                    app.selected_setting += 1;
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_setting < max {
                                    app.saved_selected_setting += 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('H') => {
                    if app.focus_panel == FocusPanel::Arrangement && app.main_tab == MainTab::Live {
                        app.move_monitor_left();
                    }
                }
                KeyCode::Char('L') => {
                    if app.focus_panel == FocusPanel::Arrangement && app.main_tab == MainTab::Live {
                        app.move_monitor_right();
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if app.focus_panel == FocusPanel::Settings && app.main_tab == MainTab::Live {
                        let field = SettingField::all()[app.selected_setting];
                        if field == SettingField::Primary {
                            app.toggle_primary();
                        } else {
                            app.dropdown_selection = 0;
                            app.dialog = DialogType::EditDropdown;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    InputResult::Continue
}

pub fn handle_mouse(
    app: &mut App,
    kind: MouseEventKind,
    col: u16,
    row: u16,
    terminal_width: u16,
    terminal_height: u16,
) -> InputResult {
    let col = col as usize;
    let row = row as usize;
    let height = terminal_height as usize;
    let width = terminal_width as usize;

    // Use ratatui's Layout to compute exact same areas as main.rs render
    let rect = Rect::new(0, 0, terminal_width, terminal_height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Main tabs (3 rows for box)
            Constraint::Percentage(38), // Arrangement
            Constraint::Percentage(43), // Settings
            Constraint::Length(3),      // Help bar
        ])
        .split(rect);

    let tabs_area = chunks[0];
    let arrangement_area = chunks[1];
    let settings_area = chunks[2];

    let tabs_start = tabs_area.y as usize;
    let tabs_end = (tabs_area.y + tabs_area.height) as usize;
    let arrangement_start = arrangement_area.y as usize;
    let arrangement_end = (arrangement_area.y + arrangement_area.height) as usize;
    let settings_start = settings_area.y as usize;
    let settings_end = (settings_area.y + settings_area.height) as usize;

    match app.dialog {
        DialogType::EditDropdown => {
            if app.main_tab != MainTab::Live {
                app.dialog = DialogType::None;
                return InputResult::Continue;
            }
            match kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let options = app.get_dropdown_options();
                    if options.is_empty() {
                        app.dialog = DialogType::None;
                        return InputResult::Continue;
                    }

                    // Calculate dropdown bounds - MUST match dialogs.rs render_dropdown exactly
                    let area_x = settings_area.x as usize;
                    let area_y = settings_area.y as usize;
                    let area_width = settings_area.width as usize;
                    let area_height = settings_area.height as usize;

                    let dropdown_height = (options.len() + 2).min(10);
                    let dropdown_width = options.iter().map(|s| s.len()).max().unwrap_or(10) + 6;
                    let dropdown_width = dropdown_width.max(20); // Match dialogs.rs minimum

                    // Position: BELOW the setting row, aligned with value column
                    let raw_x = area_x + 18;
                    let raw_y = area_y + 3 + app.selected_setting; // One row below

                    // Clamp to area bounds (matching dialogs.rs clamping)
                    let dropdown_x = raw_x.min(area_x + area_width - dropdown_width);
                    let dropdown_y = raw_y.min(area_y + area_height - dropdown_height);

                    // Check if click is inside dropdown area (including border)
                    if col >= dropdown_x
                        && col < dropdown_x + dropdown_width
                        && row >= dropdown_y
                        && row < dropdown_y + dropdown_height
                    {
                        // Inside dropdown - check if on an option (skip border rows)
                        if row > dropdown_y && row < dropdown_y + dropdown_height - 1 {
                            let clicked_idx = row - dropdown_y - 1;
                            if clicked_idx < options.len() {
                                app.dropdown_selection = clicked_idx;
                                app.apply_dropdown_selection();
                                app.dialog = DialogType::None;
                            }
                        }
                        // Click on border does nothing, stays open
                    } else {
                        // Click outside dropdown closes it
                        app.dialog = DialogType::None;
                    }
                }
                MouseEventKind::ScrollUp => {
                    if app.dropdown_selection > 0 {
                        app.dropdown_selection -= 1;
                    }
                }
                MouseEventKind::ScrollDown => {
                    let max = app.get_dropdown_options().len().saturating_sub(1);
                    if app.dropdown_selection < max {
                        app.dropdown_selection += 1;
                    }
                }
                _ => {}
            }
        }
        DialogType::ConfirmApply { .. } | DialogType::ConfirmQuit => {
            if let MouseEventKind::Down(MouseButton::Left) = kind {
                let center_y = height / 2;
                let center_x = width / 2;
                if row >= center_y && row <= center_y + 2 {
                    if col >= center_x.saturating_sub(12) && col <= center_x.saturating_sub(6) {
                        // [Y] Yes
                        if matches!(app.dialog, DialogType::ConfirmQuit) {
                            return InputResult::Quit;
                        } else {
                            app.confirm_changes();
                        }
                    } else if col >= center_x.saturating_sub(2) && col <= center_x + 4 {
                        // [N] No
                        if matches!(app.dialog, DialogType::ConfirmApply { .. }) {
                            app.revert_changes();
                            let _ = app.save_and_apply();
                        }
                        app.dialog = DialogType::None;
                    }
                }
            }
        }
        DialogType::NewWorkspace | DialogType::RenameWorkspace | DialogType::DeleteWorkspace => {
            // Input dialogs - ignore mouse, use keyboard
        }
        DialogType::None => {
            match kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Check if click is on main tabs area
                    if row >= tabs_start && row < tabs_end {
                        // Tabs are boxed and centered
                        let center = width / 2;
                        if col < center {
                            app.switch_tab(MainTab::Live);
                        } else {
                            app.switch_tab(MainTab::Saved);
                        }
                    } else if row >= arrangement_start && row < arrangement_end {
                        // Click in arrangement panel
                        app.focus_panel = FocusPanel::Arrangement;

                        // Calculate which monitor was clicked
                        let num_monitors = if app.main_tab == MainTab::Live {
                            app.monitors.len()
                        } else {
                            app.saved_monitors.len()
                        };

                        if num_monitors > 0 {
                            let box_width = BOX_WIDTH as usize;
                            let gap = BOX_GAP as usize;
                            let total_width =
                                (box_width * num_monitors) + (gap * (num_monitors - 1));
                            let start_x = width.saturating_sub(total_width) / 2;

                            for i in 0..num_monitors {
                                let box_start = start_x + i * (box_width + gap);
                                let box_end = box_start + box_width;
                                if col >= box_start && col < box_end {
                                    if app.main_tab == MainTab::Live {
                                        app.selected_monitor = i;
                                        // Start dragging only in Live
                                        app.drag_state = DragState::Dragging {
                                            monitor_idx: i,
                                            start_x: col as u16,
                                            start_y: row as u16,
                                            current_x: col as u16,
                                            current_y: row as u16,
                                        };
                                    } else {
                                        app.saved_selected_monitor = i;
                                        app.drag_state = DragState::None;
                                    }
                                    break;
                                }
                            }
                        }
                    } else if row >= settings_start && row < settings_end {
                        // Click in settings panel
                        app.focus_panel = FocusPanel::Settings;

                        if let Some(idx) = row_to_setting(row, settings_start) {
                            if app.main_tab == MainTab::Live {
                                app.selected_setting = idx;
                                let field = SettingField::all()[idx];

                                // Convert to panel-local x for robust hit testing.
                                let rel_col = col.saturating_sub(settings_area.x as usize);
                                if field == SettingField::Primary {
                                    // Checkbox is around column 4-7
                                    if (3..=8).contains(&rel_col) {
                                        app.toggle_primary();
                                    }
                                } else {
                                    // Value area is around column 18-35, [Change] is after
                                    if rel_col >= 17 {
                                        app.dropdown_selection = 0;
                                        app.dialog = DialogType::EditDropdown;
                                    }
                                }
                            } else {
                                // Saved panel is read-only; only update highlight.
                                app.saved_selected_setting = idx;
                            }
                        }
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if let DragState::Dragging {
                        monitor_idx,
                        start_x,
                        start_y,
                        ..
                    } = app.drag_state
                    {
                        app.drag_state = DragState::Dragging {
                            monitor_idx,
                            start_x,
                            start_y,
                            current_x: col as u16,
                            current_y: row as u16,
                        };
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    if let DragState::Dragging {
                        start_x, current_x, ..
                    } = app.drag_state
                    {
                        let drag_distance = current_x as i16 - start_x as i16;
                        let box_width = BOX_WIDTH as i16;
                        let gap = BOX_GAP as i16;
                        let threshold = (box_width + gap) / 2;

                        if drag_distance.abs() > threshold {
                            let positions_moved =
                                (drag_distance.abs() + threshold) / (box_width + gap);

                            if drag_distance > 0 {
                                for _ in 0..positions_moved {
                                    if app.selected_monitor < app.monitors.len() - 1 {
                                        app.monitors
                                            .swap(app.selected_monitor, app.selected_monitor + 1);
                                        app.selected_monitor += 1;
                                    }
                                }
                            } else {
                                for _ in 0..positions_moved {
                                    if app.selected_monitor > 0 {
                                        app.monitors
                                            .swap(app.selected_monitor, app.selected_monitor - 1);
                                        app.selected_monitor -= 1;
                                    }
                                }
                            }

                            app.recalculate_positions();
                            app.has_changes = true;
                        }

                        app.drag_state = DragState::None;
                    }
                }
                MouseEventKind::ScrollUp => {
                    if row >= arrangement_start && row < arrangement_end {
                        match app.main_tab {
                            MainTab::Live => app.select_prev_monitor(),
                            MainTab::Saved => {
                                if app.saved_selected_monitor > 0 {
                                    app.saved_selected_monitor -= 1;
                                }
                            }
                        }
                    } else if row >= settings_start && row < settings_end {
                        match app.main_tab {
                            MainTab::Live => {
                                if app.selected_setting > 0 {
                                    app.selected_setting -= 1;
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_setting > 0 {
                                    app.saved_selected_setting -= 1;
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    if row >= arrangement_start && row < arrangement_end {
                        match app.main_tab {
                            MainTab::Live => app.select_next_monitor(),
                            MainTab::Saved => {
                                if app.saved_selected_monitor
                                    < app.saved_monitors.len().saturating_sub(1)
                                {
                                    app.saved_selected_monitor += 1;
                                }
                            }
                        }
                    } else if row >= settings_start && row < settings_end {
                        let max = SettingField::all().len().saturating_sub(1);
                        match app.main_tab {
                            MainTab::Live => {
                                if app.selected_setting < max {
                                    app.selected_setting += 1;
                                }
                            }
                            MainTab::Saved => {
                                if app.saved_selected_setting < max {
                                    app.saved_selected_setting += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    InputResult::Continue
}
