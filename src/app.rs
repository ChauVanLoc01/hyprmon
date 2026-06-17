use anyhow::Result;
use std::fs;
use std::time::Instant;

use crate::config::MonitorDatabase;
use crate::monitor::{fetch_monitors, identify_monitors, MonitorConfig, Rotation};
use crate::state::{DialogType, DragState, FocusPanel, MainTab, SettingField};

pub struct App {
    // Live panel state
    pub monitors: Vec<MonitorConfig>,
    pub original_monitors: Vec<MonitorConfig>,
    pub selected_monitor: usize,
    pub focus_panel: FocusPanel,
    pub selected_setting: usize,

    // Saved panel state
    pub saved_monitors: Vec<MonitorConfig>,
    pub saved_selected_monitor: usize,
    pub saved_selected_setting: usize,
    pub selected_workspace: usize,

    // Common state
    pub main_tab: MainTab,
    pub dialog: DialogType,
    pub dropdown_selection: usize,
    pub has_changes: bool,
    pub message: String,
    pub drag_state: DragState,
    pub monitor_db: MonitorDatabase,
    pub input_buffer: String,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut monitor_db = MonitorDatabase::load().unwrap_or_default();
        let mut monitors = fetch_monitors()?;

        // Find best matching workspace for connected monitors
        if let Some(ws_idx) = monitor_db.find_best_workspace(&monitors) {
            monitor_db.active_workspace = ws_idx;
        }

        // Apply saved configs to connected monitors
        for monitor in &mut monitors {
            monitor_db.apply_saved_config(monitor);
        }

        let original = monitors.clone();
        let selected_workspace = monitor_db.active_workspace;
        let saved_monitors = monitor_db.get_workspace_monitors(selected_workspace);

        Ok(Self {
            monitors,
            original_monitors: original,
            selected_monitor: 0,
            focus_panel: FocusPanel::Arrangement,
            selected_setting: 0,

            saved_monitors,
            saved_selected_monitor: 0,
            saved_selected_setting: 0,
            selected_workspace,

            main_tab: MainTab::Live,
            dialog: DialogType::None,
            dropdown_selection: 0,
            has_changes: false,
            message: String::new(),
            drag_state: DragState::None,
            monitor_db,
            input_buffer: String::new(),
        })
    }

    /// Switch main tab
    pub fn switch_tab(&mut self, tab: MainTab) {
        // Dropdown is only valid in Live panel; close it when changing tabs.
        if self.main_tab != tab && matches!(self.dialog, DialogType::EditDropdown) {
            self.dialog = DialogType::None;
        }
        self.main_tab = tab;
        if tab == MainTab::Saved {
            self.refresh_saved_monitors();
        }
    }

    /// Refresh saved monitors from current workspace
    pub fn refresh_saved_monitors(&mut self) {
        self.saved_monitors = self
            .monitor_db
            .get_workspace_monitors(self.selected_workspace);
        self.saved_selected_monitor = self
            .saved_selected_monitor
            .min(self.saved_monitors.len().saturating_sub(1));
    }

    /// Select next workspace
    pub fn next_workspace(&mut self) {
        if self.selected_workspace < self.monitor_db.workspaces.len() - 1 {
            self.selected_workspace += 1;
            self.monitor_db.active_workspace = self.selected_workspace;
            self.refresh_saved_monitors();
        }
    }

    /// Select previous workspace
    pub fn prev_workspace(&mut self) {
        if self.selected_workspace > 0 {
            self.selected_workspace -= 1;
            self.monitor_db.active_workspace = self.selected_workspace;
            self.refresh_saved_monitors();
        }
    }

    /// Create new workspace
    pub fn create_workspace(&mut self, name: &str) {
        let idx = self.monitor_db.add_workspace(name);
        self.selected_workspace = idx;
        self.monitor_db.active_workspace = idx;
        let _ = self.monitor_db.save();
        self.refresh_saved_monitors();
        self.message = format!("Created workspace: {}", name);
    }

    /// Delete current workspace
    pub fn delete_current_workspace(&mut self) -> bool {
        if self.monitor_db.delete_workspace(self.selected_workspace) {
            self.selected_workspace = self
                .selected_workspace
                .min(self.monitor_db.workspaces.len().saturating_sub(1));
            self.monitor_db.active_workspace = self.selected_workspace;
            let _ = self.monitor_db.save();
            self.refresh_saved_monitors();
            self.message = "Workspace deleted".to_string();
            true
        } else {
            self.message = "Cannot delete last workspace".to_string();
            false
        }
    }

    /// Rename current workspace
    pub fn rename_current_workspace(&mut self, name: &str) {
        self.monitor_db
            .rename_workspace(self.selected_workspace, name);
        let _ = self.monitor_db.save();
        self.message = format!("Renamed to: {}", name);
    }

    /// Get current workspace name
    pub fn current_workspace_name(&self) -> String {
        self.monitor_db
            .workspaces
            .get(self.selected_workspace)
            .map(|ws| ws.name.clone())
            .unwrap_or_default()
    }

    pub fn current_monitor(&self) -> Option<&MonitorConfig> {
        self.monitors.get(self.selected_monitor)
    }

    pub fn current_monitor_mut(&mut self) -> Option<&mut MonitorConfig> {
        self.monitors.get_mut(self.selected_monitor)
    }

    pub fn select_next_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.selected_monitor = (self.selected_monitor + 1) % self.monitors.len();
        }
    }

    pub fn select_prev_monitor(&mut self) {
        if !self.monitors.is_empty() {
            if self.selected_monitor == 0 {
                self.selected_monitor = self.monitors.len() - 1;
            } else {
                self.selected_monitor -= 1;
            }
        }
    }

    pub fn move_monitor_left(&mut self) {
        if self.selected_monitor == 0 || self.monitors.len() < 2 {
            return;
        }
        self.monitors
            .swap(self.selected_monitor, self.selected_monitor - 1);
        self.selected_monitor -= 1;
        self.recalculate_positions();
        self.has_changes = true;
    }

    pub fn move_monitor_right(&mut self) {
        if self.selected_monitor >= self.monitors.len() - 1 {
            return;
        }
        self.monitors
            .swap(self.selected_monitor, self.selected_monitor + 1);
        self.selected_monitor += 1;
        self.recalculate_positions();
        self.has_changes = true;
    }

    pub fn recalculate_positions(&mut self) {
        let mut x = 0i32;
        for monitor in &mut self.monitors {
            monitor.position_x = x;
            monitor.position_y = 0;

            if let Some((w, _)) = monitor.resolution.split_once('x') {
                if let Ok(width) = w.parse::<i32>() {
                    x += (width as f64 / monitor.scale) as i32;
                }
            }
        }
    }

    pub fn set_primary(&mut self, index: usize) {
        for (i, m) in self.monitors.iter_mut().enumerate() {
            m.is_primary = i == index;
        }
        self.has_changes = true;
    }

    pub fn toggle_primary(&mut self) {
        let idx = self.selected_monitor;
        if idx < self.monitors.len() {
            let is_currently_primary = self.monitors[idx].is_primary;

            if is_currently_primary {
                // If unchecking, make the first other monitor primary
                if self.monitors.len() > 1 {
                    let new_primary = if idx == 0 { 1 } else { 0 };
                    self.set_primary(new_primary);
                }
                // If only one monitor, keep it primary
            } else {
                self.set_primary(idx);
            }
        }
    }

    pub fn get_dropdown_options(&self) -> Vec<String> {
        let field = SettingField::all()[self.selected_setting];
        let monitor = match self.current_monitor() {
            Some(m) => m,
            None => return vec![],
        };

        match field {
            SettingField::Resolution => {
                let mut resolutions: Vec<String> = monitor
                    .available_modes
                    .iter()
                    .filter_map(|m| m.split_once('@').map(|(res, _)| res.trim().to_string()))
                    .collect();
                resolutions.sort();
                resolutions.dedup();
                // Sort descending by total pixels (width * height)
                resolutions.sort_by(|a, b| {
                    let pixels_a = a.split('x')
                        .filter_map(|s| s.parse::<u64>().ok())
                        .product::<u64>();
                    let pixels_b = b.split('x')
                        .filter_map(|s| s.parse::<u64>().ok())
                        .product::<u64>();
                    pixels_b.cmp(&pixels_a)
                });
                resolutions
            }
            SettingField::RefreshRate => {
                let current_res = monitor.resolution.trim();
                let mut refresh_rates: Vec<u32> = monitor
                    .available_modes
                    .iter()
                    .filter_map(|m| m.split_once('@'))
                    .filter(|(res, _)| res.trim() == current_res)
                    .filter_map(|(_, rate)| {
                        // Parse and round the refresh rate
                        rate.trim()
                            .trim_end_matches("Hz")
                            .trim()
                            .parse::<f64>()
                            .ok()
                            .map(|r| r.round() as u32)
                    })
                    .collect();

                refresh_rates.sort_by(|a, b| b.cmp(a)); // Sort descending
                refresh_rates.dedup();

                let result: Vec<String> = refresh_rates
                    .into_iter()
                    .map(|r| format!("{}Hz", r))
                    .collect();

                if result.is_empty() {
                    vec![format!("{}Hz", monitor.refresh_rate.round() as u32)]
                } else {
                    result
                }
            }
            SettingField::Scale => vec!["100%", "125%", "150%", "175%", "200%"]
                .into_iter()
                .map(String::from)
                .collect(),
            SettingField::Rotation => Rotation::all()
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            SettingField::Primary => vec![],
        }
    }

    pub fn apply_dropdown_selection(&mut self) {
        let field = SettingField::all()[self.selected_setting];
        let options = self.get_dropdown_options();
        let dropdown_idx = self.dropdown_selection;

        if dropdown_idx >= options.len() {
            return;
        }

        let selected_value = options[dropdown_idx].clone();

        if let Some(monitor) = self.current_monitor_mut() {
            match field {
                SettingField::Resolution => {
                    monitor.resolution = selected_value;
                }
                SettingField::RefreshRate => {
                    if let Ok(rate) = selected_value.trim_end_matches("Hz").parse::<f64>() {
                        monitor.refresh_rate = rate;
                    }
                }
                SettingField::Scale => {
                    if let Ok(scale) = selected_value.trim_end_matches('%').parse::<f64>() {
                        monitor.scale = scale / 100.0;
                    }
                }
                SettingField::Rotation => {
                    monitor.rotation = match dropdown_idx {
                        0 => Rotation::Normal,
                        1 => Rotation::Left,
                        2 => Rotation::Right,
                        3 => Rotation::Inverted,
                        _ => Rotation::Normal,
                    };
                }
                SettingField::Primary => {}
            }
            self.has_changes = true;
        }

        self.recalculate_positions();
    }

    #[allow(dead_code)]
    pub fn generate_config(&self) -> String {
        let mut config =
            String::from("# Hyprland Monitor Configuration\n# Generated by hyprmon\n\n");

        for monitor in &self.monitors {
            let is_laptop = monitor.name.starts_with("eDP");
            let identifier = if is_laptop {
                monitor.name.clone()
            } else {
                format!("desc:{}", monitor.description)
            };

            let transform = monitor.rotation.transform();
            let scale = if monitor.scale.fract() == 0.0 {
                format!("{}", monitor.scale as i32)
            } else {
                format!("{:.2}", monitor.scale)
            };

            if transform == 0 {
                config.push_str(&format!(
                    "# {}\nmonitor={},{}@{:.2},{}x{},{}\n\n",
                    monitor.model,
                    identifier,
                    monitor.resolution,
                    monitor.refresh_rate,
                    monitor.position_x,
                    monitor.position_y,
                    scale
                ));
            } else {
                config.push_str(&format!(
                    "# {}\nmonitor={},{}@{:.2},{}x{},{},transform,{}\n\n",
                    monitor.model,
                    identifier,
                    monitor.resolution,
                    monitor.refresh_rate,
                    monitor.position_x,
                    monitor.position_y,
                    scale,
                    transform
                ));
            }
        }

        config.push_str("# Fallback\nmonitor=,preferred,auto,1\n");
        config
    }

    pub fn save_and_apply(&mut self) -> Result<()> {
        // Sync workspace selection before saving
        self.monitor_db.active_workspace = self.selected_workspace;

        // Update database with current monitor configs
        for monitor in &self.monitors {
            self.monitor_db.update_monitor(monitor);
        }
        self.monitor_db.save()?;

        // Refresh saved monitors view
        self.refresh_saved_monitors();

        let config_path = dirs::home_dir().unwrap().join(".config/hypr/monitors.conf");

        let existing = if config_path.exists() {
            let backup = config_path.with_extension("conf.bak");
            fs::copy(&config_path, &backup)?;
            fs::read_to_string(&config_path).unwrap_or_default()
        } else {
            String::new()
        };

        // Rewrite only hyprmon's managed block so any user-authored lines in
        // monitors.conf survive regeneration.
        let block = self.monitor_db.generate_full_config();
        let config = crate::config::splice_managed_block(&existing, &block);
        fs::write(&config_path, &config)?;

        // Reload Hyprland to apply changes
        std::process::Command::new("hyprctl")
            .arg("reload")
            .output()
            .ok();

        self.message = "Applied! Check your monitors.".to_string();
        self.dialog = DialogType::ConfirmApply {
            countdown: 15,
            started: Instant::now(),
        };

        Ok(())
    }

    pub fn revert_changes(&mut self) {
        self.monitors = self.original_monitors.clone();
        self.has_changes = false;
        self.message = "Changes reverted.".to_string();
    }

    pub fn confirm_changes(&mut self) {
        self.original_monitors = self.monitors.clone();
        self.has_changes = false;
        self.dialog = DialogType::None;
        self.message = "Configuration saved!".to_string();
    }

    pub fn identify(&self) {
        identify_monitors(&self.monitors);
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.monitor_db = MonitorDatabase::load().unwrap_or_default();
        self.monitors = fetch_monitors()?;

        // Apply saved configs to connected monitors
        for monitor in &mut self.monitors {
            self.monitor_db.apply_saved_config(monitor);
        }

        self.original_monitors = self.monitors.clone();
        self.selected_monitor = self
            .selected_monitor
            .min(self.monitors.len().saturating_sub(1));
        self.has_changes = false;
        self.message = "Monitors refreshed.".to_string();
        Ok(())
    }

    /// Called when a monitor is added via IPC
    pub fn on_monitor_added(&mut self, _name: &str) -> Result<()> {
        self.refresh()?;

        // Auto-apply if we have saved config
        let has_saved = self
            .monitors
            .iter()
            .any(|m| self.monitor_db.get_saved_config(m).is_some());
        if has_saved {
            self.message = "Monitor connected - applying saved config...".to_string();
            self.save_and_apply()?;
        } else {
            self.message = "New monitor detected!".to_string();
        }
        Ok(())
    }

    /// Called when a monitor is removed via IPC
    pub fn on_monitor_removed(&mut self, _name: &str) -> Result<()> {
        self.refresh()?;
        self.message = "Monitor disconnected.".to_string();
        Ok(())
    }
}

#[cfg(test)]
impl App {
    /// Build an `App` around `monitors` with a fresh in-memory DB, bypassing the
    /// `new()` path that shells out to `hyprctl`. Used by UI render tests.
    pub fn for_test(monitors: Vec<MonitorConfig>) -> Self {
        let db = MonitorDatabase::default();
        let saved = db.get_workspace_monitors(db.active_workspace);
        Self {
            monitors: monitors.clone(),
            original_monitors: monitors,
            selected_monitor: 0,
            focus_panel: FocusPanel::Arrangement,
            selected_setting: 0,
            saved_monitors: saved,
            saved_selected_monitor: 0,
            saved_selected_setting: 0,
            selected_workspace: 0,
            main_tab: MainTab::Live,
            dialog: DialogType::None,
            dropdown_selection: 0,
            has_changes: false,
            message: String::new(),
            drag_state: DragState::None,
            monitor_db: db,
            input_buffer: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mc(name: &str, make: &str, model: &str, res: &str, x: i32) -> MonitorConfig {
        MonitorConfig {
            name: name.into(),
            description: format!("{make} {model}"),
            make: make.into(),
            model: model.into(),
            resolution: res.into(),
            refresh_rate: 60.0,
            position_x: x,
            position_y: 0,
            scale: 1.0,
            rotation: Rotation::Normal,
            is_primary: false,
            available_modes: vec![
                "1920x1080@60.00Hz".into(),
                "1920x1080@144.00Hz".into(),
                "2560x1440@60.00Hz".into(),
            ],
        }
    }

    fn app_with(monitors: Vec<MonitorConfig>, db: MonitorDatabase) -> App {
        let aw = db.active_workspace;
        let saved = db.get_workspace_monitors(aw);
        App {
            monitors: monitors.clone(),
            original_monitors: monitors,
            selected_monitor: 0,
            focus_panel: FocusPanel::Arrangement,
            selected_setting: 0,
            saved_monitors: saved,
            saved_selected_monitor: 0,
            saved_selected_setting: 0,
            selected_workspace: aw,
            main_tab: MainTab::Live,
            dialog: DialogType::None,
            dropdown_selection: 0,
            has_changes: false,
            message: String::new(),
            drag_state: DragState::None,
            monitor_db: db,
            input_buffer: String::new(),
        }
    }

    #[test]
    fn switch_tab_closes_dropdown_and_refreshes_saved() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "1920x1080", 0)], MonitorDatabase::default());
        app.dialog = DialogType::EditDropdown;
        app.switch_tab(MainTab::Saved);
        assert_eq!(app.main_tab, MainTab::Saved);
        assert!(matches!(app.dialog, DialogType::None));
    }

    #[test]
    fn refresh_saved_monitors_clamps_selection() {
        let mut app = app_with(vec![], MonitorDatabase::default());
        app.saved_selected_monitor = 5;
        app.refresh_saved_monitors();
        assert_eq!(app.saved_selected_monitor, 0);
    }

    #[test]
    fn workspace_navigation_respects_bounds() {
        let mut db = MonitorDatabase::default();
        db.add_workspace("Two");
        let mut app = app_with(vec![], db);
        app.next_workspace();
        assert_eq!(app.selected_workspace, 1);
        app.next_workspace(); // already last
        assert_eq!(app.selected_workspace, 1);
        app.prev_workspace();
        assert_eq!(app.selected_workspace, 0);
        app.prev_workspace(); // already first
        assert_eq!(app.selected_workspace, 0);
    }

    #[test]
    fn workspace_create_rename_delete_persists_to_temp() {
        let mut p = std::env::temp_dir();
        p.push(format!("hyprmon_app_ws_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let mut db = MonitorDatabase::default();
        db.set_config_path(p.clone());
        let mut app = app_with(vec![], db);

        app.create_workspace("New");
        assert_eq!(app.monitor_db.workspaces.len(), 2);
        assert!(app.message.contains("New"));
        assert!(p.exists());

        app.rename_current_workspace("Renamed");
        assert_eq!(app.current_workspace_name(), "Renamed");

        assert!(app.delete_current_workspace());
        assert!(!app.delete_current_workspace()); // cannot delete the last
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn current_workspace_name_out_of_range_is_empty() {
        let mut app = app_with(vec![], MonitorDatabase::default());
        app.selected_workspace = 99;
        assert_eq!(app.current_workspace_name(), "");
    }

    #[test]
    fn current_monitor_accessors() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "1920x1080", 0)], MonitorDatabase::default());
        assert!(app.current_monitor().is_some());
        app.current_monitor_mut().unwrap().scale = 2.0;
        assert_eq!(app.current_monitor().unwrap().scale, 2.0);
        app.selected_monitor = 99;
        assert!(app.current_monitor().is_none());
        assert!(app.current_monitor_mut().is_none());
    }

    #[test]
    fn monitor_selection_wraps() {
        let mut app = app_with(
            vec![mc("A", "x", "x", "1920x1080", 0), mc("B", "y", "y", "1920x1080", 1920)],
            MonitorDatabase::default(),
        );
        app.select_next_monitor();
        assert_eq!(app.selected_monitor, 1);
        app.select_next_monitor(); // wrap
        assert_eq!(app.selected_monitor, 0);
        app.select_prev_monitor(); // wrap back
        assert_eq!(app.selected_monitor, 1);
        app.select_prev_monitor();
        assert_eq!(app.selected_monitor, 0);
    }

    #[test]
    fn move_monitor_swaps_and_recalculates() {
        let mut app = app_with(
            vec![mc("A", "x", "x", "1920x1080", 0), mc("B", "y", "y", "1920x1080", 1920)],
            MonitorDatabase::default(),
        );
        app.selected_monitor = 1;
        app.move_monitor_left();
        assert_eq!(app.monitors[0].name, "B");
        assert_eq!(app.selected_monitor, 0);
        assert!(app.has_changes);
        app.move_monitor_left(); // already leftmost
        assert_eq!(app.selected_monitor, 0);
        app.move_monitor_right();
        assert_eq!(app.monitors[1].name, "B");
        assert_eq!(app.selected_monitor, 1);
        app.move_monitor_right(); // already rightmost
        assert_eq!(app.selected_monitor, 1);
    }

    #[test]
    fn recalculate_positions_lays_edge_to_edge_with_scale() {
        let mut app = app_with(
            vec![mc("A", "x", "x", "1920x1080", 0), mc("B", "y", "y", "2560x1440", 0)],
            MonitorDatabase::default(),
        );
        app.monitors[0].scale = 1.5; // logical width 1280
        app.recalculate_positions();
        assert_eq!(app.monitors[0].position_x, 0);
        assert_eq!(app.monitors[1].position_x, 1280);
    }

    #[test]
    fn set_and_toggle_primary() {
        let mut app = app_with(
            vec![mc("A", "x", "x", "1920x1080", 0), mc("B", "y", "y", "1920x1080", 1920)],
            MonitorDatabase::default(),
        );
        app.set_primary(1);
        assert!(app.monitors[1].is_primary && !app.monitors[0].is_primary);

        app.selected_monitor = 1;
        app.toggle_primary(); // currently primary -> hand off to the other
        assert!(app.monitors[0].is_primary);
        app.toggle_primary(); // selected(1) not primary -> becomes primary
        assert!(app.monitors[1].is_primary);
    }

    #[test]
    fn toggle_primary_single_monitor_stays_primary() {
        let mut app = app_with(vec![mc("A", "x", "x", "1920x1080", 0)], MonitorDatabase::default());
        app.set_primary(0);
        app.toggle_primary();
        assert!(app.monitors[0].is_primary);
    }

    #[test]
    fn dropdown_options_per_field() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "1920x1080", 0)], MonitorDatabase::default());

        app.selected_setting = 0; // Resolution -> highest pixel first
        let res = app.get_dropdown_options();
        assert_eq!(res.first().map(String::as_str), Some("2560x1440"));
        assert!(res.contains(&"1920x1080".to_string()));

        app.selected_setting = 1; // RefreshRate for 1920x1080 -> 144,60
        let rates = app.get_dropdown_options();
        assert_eq!(rates, vec!["144Hz", "60Hz"]);

        app.selected_setting = 2; // Scale fixed list
        assert_eq!(app.get_dropdown_options().len(), 5);

        app.selected_setting = 3; // Rotation
        assert_eq!(app.get_dropdown_options().len(), 4);

        app.selected_setting = 4; // Primary has no dropdown
        assert!(app.get_dropdown_options().is_empty());
    }

    #[test]
    fn refresh_rate_options_fall_back_when_no_modes_match() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "3840x2160", 0)], MonitorDatabase::default());
        app.selected_setting = 1; // no mode matches 3840x2160 -> fallback to current
        assert_eq!(app.get_dropdown_options(), vec!["60Hz"]);
    }

    #[test]
    fn apply_dropdown_sets_each_field() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "1920x1080", 0)], MonitorDatabase::default());

        app.selected_setting = 2; // Scale -> "150%"
        app.dropdown_selection = 2;
        app.apply_dropdown_selection();
        assert_eq!(app.current_monitor().unwrap().scale, 1.5);

        app.selected_setting = 3; // Rotation -> Left
        app.dropdown_selection = 1;
        app.apply_dropdown_selection();
        assert_eq!(app.current_monitor().unwrap().rotation, Rotation::Left);

        app.selected_setting = 1; // RefreshRate -> 144
        app.dropdown_selection = 0;
        app.apply_dropdown_selection();
        assert_eq!(app.current_monitor().unwrap().refresh_rate, 144.0);

        app.selected_setting = 0; // Resolution -> top option
        let top = app.get_dropdown_options()[0].clone();
        app.dropdown_selection = 0;
        app.apply_dropdown_selection();
        assert_eq!(app.current_monitor().unwrap().resolution, top);
    }

    #[test]
    fn apply_dropdown_out_of_range_is_noop() {
        let mut app = app_with(vec![mc("eDP-1", "N", "M", "1920x1080", 0)], MonitorDatabase::default());
        app.selected_setting = 2;
        app.dropdown_selection = 99;
        app.apply_dropdown_selection();
        assert_eq!(app.current_monitor().unwrap().scale, 1.0); // unchanged
    }

    #[test]
    fn generate_config_writes_lines_and_transform() {
        let mut app = app_with(
            vec![mc("eDP-1", "N", "M", "1920x1080", 0), mc("HDMI-A-1", "MSI", "MP", "2560x1440", 1920)],
            MonitorDatabase::default(),
        );
        app.monitors[1].rotation = Rotation::Left;
        let cfg = app.generate_config();
        assert!(cfg.contains("monitor=eDP-1,"));
        assert!(cfg.contains("desc:MSI MP"));
        assert!(cfg.contains(",transform,1"));
    }

    #[test]
    fn revert_and_confirm_changes() {
        let mut app = app_with(vec![mc("A", "x", "x", "1920x1080", 0)], MonitorDatabase::default());
        app.monitors[0].scale = 9.0;
        app.has_changes = true;
        app.revert_changes();
        assert_eq!(app.monitors[0].scale, 1.0);
        assert!(!app.has_changes);

        app.monitors[0].scale = 3.0;
        app.dialog = DialogType::ConfirmQuit;
        app.confirm_changes();
        assert_eq!(app.original_monitors[0].scale, 3.0);
        assert!(!app.has_changes);
        assert!(matches!(app.dialog, DialogType::None));
    }
}
