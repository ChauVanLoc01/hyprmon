mod arrangement;
mod dialogs;
mod help;
mod saved;
pub mod settings;
mod tabs;

pub use arrangement::render_arrangement_panel;
pub use dialogs::{
    render_confirm_apply_dialog, render_confirm_quit_dialog, render_dropdown, render_input_dialog,
};
pub use help::render_help_bar;
pub use saved::{render_saved_arrangement_panel, render_saved_settings_panel};
pub use settings::render_settings_panel;
pub use tabs::{render_main_tabs, render_workspace_tabs};

use ratatui::prelude::*;

pub fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let width = r.width * percent_x / 100;
    let x = r.x + (r.width - width) / 2;
    let y = r.y + (r.height - height) / 2;
    Rect::new(x, y, width, height)
}

// Layout constants
pub const BOX_WIDTH: u16 = 18;
pub const BOX_HEIGHT: u16 = 6;
pub const BOX_GAP: u16 = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::monitor::MonitorConfig;
    use crate::state::{DialogType, FocusPanel, MainTab};
    use ratatui::{backend::TestBackend, Terminal};

    const W: u16 = 140;
    const H: u16 = 44;

    fn area() -> Rect {
        Rect::new(0, 0, W, H)
    }

    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(W, H)).unwrap()
    }

    fn app() -> App {
        App::for_test(vec![
            MonitorConfig::for_test("eDP-1", "Najing", "0x004D", "1920x1080"),
            MonitorConfig::for_test("HDMI-A-1", "MSI", "MP275Q", "2560x1440"),
        ])
    }

    #[test]
    fn renders_every_panel_without_panicking() {
        let mut a = app();
        a.message = "Applied!".into();
        let r = area();
        terminal().draw(|f| render_arrangement_panel(f, r, &a)).unwrap();
        terminal().draw(|f| render_settings_panel(f, r, &a)).unwrap();
        terminal().draw(|f| render_main_tabs(f, r, &a)).unwrap();
        terminal().draw(|f| render_workspace_tabs(f, r, &a)).unwrap();
        terminal().draw(|f| render_help_bar(f, r, &a)).unwrap();
    }

    #[test]
    fn renders_focused_and_empty_states() {
        let r = area();
        let mut a = app();
        a.focus_panel = FocusPanel::Settings;
        a.main_tab = MainTab::Saved;
        terminal().draw(|f| render_settings_panel(f, r, &a)).unwrap();

        let empty = App::for_test(vec![]);
        terminal().draw(|f| render_settings_panel(f, r, &empty)).unwrap();
        terminal().draw(|f| render_arrangement_panel(f, r, &empty)).unwrap();
    }

    #[test]
    fn renders_saved_panels_with_and_without_monitors() {
        let r = area();
        let empty = App::for_test(vec![]);
        terminal().draw(|f| render_saved_arrangement_panel(f, r, &empty)).unwrap();
        terminal().draw(|f| render_saved_settings_panel(f, r, &empty)).unwrap();

        let mut a = app();
        a.saved_monitors = a.monitors.clone();
        terminal().draw(|f| render_saved_arrangement_panel(f, r, &a)).unwrap();
        terminal().draw(|f| render_saved_settings_panel(f, r, &a)).unwrap();
    }

    #[test]
    fn renders_dialogs() {
        let r = area();
        let mut a = app();
        a.dialog = DialogType::EditDropdown;
        a.selected_setting = 0;
        terminal().draw(|f| render_dropdown(f, r, &a)).unwrap();
        terminal().draw(|f| render_confirm_apply_dialog(f, 10)).unwrap();
        terminal().draw(render_confirm_quit_dialog).unwrap();
        terminal()
            .draw(|f| render_input_dialog(f, "New Workspace", "typed", "Enter to confirm"))
            .unwrap();
    }

    #[test]
    fn centered_rect_stays_within_bounds() {
        let r = centered_rect(50, 10, Rect::new(0, 0, 100, 40));
        assert_eq!(r.height, 10);
        assert!(r.x + r.width <= 100);
        assert!(r.y + r.height <= 40);
    }
}
