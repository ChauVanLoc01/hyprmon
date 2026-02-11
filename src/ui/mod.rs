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
