mod app;
mod config;
mod hypr_ipc;
mod input;
mod monitor;
mod state;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::{io::stdout, sync::mpsc, time::Duration};

use app::App;
use hypr_ipc::HyprEvent;
use input::{handle_key, handle_mouse, InputResult};
use state::DialogType;
use state::MainTab;
use ui::{
    render_arrangement_panel, render_confirm_apply_dialog, render_confirm_quit_dialog,
    render_dropdown, render_help_bar, render_input_dialog, render_main_tabs,
    render_saved_arrangement_panel, render_saved_settings_panel, render_settings_panel,
    render_workspace_tabs,
};

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;

    let result = run_app();

    // Cleanup terminal
    stdout().execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_app() -> Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new()?;

    // Start Hyprland IPC listener for monitor events
    let (ipc_tx, ipc_rx) = mpsc::channel::<HyprEvent>();
    if let Err(e) = hypr_ipc::start_listener(ipc_tx) {
        app.message = format!("IPC: {}", e);
    }

    loop {
        // Handle IPC events (non-blocking)
        while let Ok(event) = ipc_rx.try_recv() {
            match event {
                HyprEvent::MonitorAdded(name) => {
                    let _ = app.on_monitor_added(&name);
                }
                HyprEvent::MonitorRemoved(name) => {
                    let _ = app.on_monitor_removed(&name);
                }
            }
        }
        // Handle countdown timer for confirm dialog
        if let DialogType::ConfirmApply { countdown, started } = app.dialog {
            let elapsed = started.elapsed().as_secs() as u8;
            if elapsed >= countdown {
                app.revert_changes();
                let _ = app.save_and_apply();
                app.dialog = DialogType::None;
            }
        }

        // Render UI
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Main tabs (3 rows for box)
                    Constraint::Percentage(38),
                    Constraint::Percentage(43),
                    Constraint::Length(3),
                ])
                .split(area);

            render_main_tabs(frame, chunks[0], &app);

            match app.main_tab {
                MainTab::Live => {
                    render_arrangement_panel(frame, chunks[1], &app);
                    render_settings_panel(frame, chunks[2], &app);
                }
                MainTab::Saved => {
                    // Split arrangement area for workspace tabs
                    let saved_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2), // Workspace tabs
                            Constraint::Min(0),    // Arrangement
                        ])
                        .split(chunks[1]);

                    render_workspace_tabs(frame, saved_chunks[0], &app);
                    render_saved_arrangement_panel(frame, saved_chunks[1], &app);
                    render_saved_settings_panel(frame, chunks[2], &app);
                }
            }

            render_help_bar(frame, chunks[3], &app);

            // Render dialogs on top
            match app.dialog {
                DialogType::EditDropdown => {
                    if app.main_tab == MainTab::Live {
                        render_dropdown(frame, chunks[2], &app);
                    }
                }
                DialogType::ConfirmApply { started, .. } => {
                    let elapsed = started.elapsed().as_secs() as u8;
                    let remaining = 15u8.saturating_sub(elapsed);
                    render_confirm_apply_dialog(frame, remaining);
                }
                DialogType::ConfirmQuit => {
                    render_confirm_quit_dialog(frame);
                }
                DialogType::NewWorkspace => {
                    render_input_dialog(
                        frame,
                        "New Workspace",
                        &app.input_buffer,
                        "Enter workspace name:",
                    );
                }
                DialogType::RenameWorkspace => {
                    render_input_dialog(
                        frame,
                        "Rename Workspace",
                        &app.input_buffer,
                        "Enter new name:",
                    );
                }
                DialogType::DeleteWorkspace => {
                    render_input_dialog(
                        frame,
                        "Delete Workspace",
                        "",
                        &format!(
                            "Delete '{}'? Press Y to confirm",
                            app.current_workspace_name()
                        ),
                    );
                }
                DialogType::None => {}
            }
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let InputResult::Quit = handle_key(&mut app, key.code, key.modifiers) {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    if let InputResult::Quit = handle_mouse(
                        &mut app,
                        mouse.kind,
                        mouse.column,
                        mouse.row,
                        size.width,
                        size.height,
                    ) {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
