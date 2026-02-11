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
                format!("desc:{} {}", monitor.make, monitor.model)
            };

            let transform = monitor.rotation.transform();

            config.push_str(&format!(
                "# {}\nmonitor={},{}@{:.2},{}x{},{:.2},transform,{}\n\n",
                monitor.model,
                identifier,
                monitor.resolution,
                monitor.refresh_rate,
                monitor.position_x,
                monitor.position_y,
                monitor.scale,
                transform
            ));
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

        if config_path.exists() {
            let backup = config_path.with_extension("conf.bak");
            fs::copy(&config_path, &backup)?;
        }

        // Generate config from ALL saved monitors (not just connected ones)
        let config = self.monitor_db.generate_full_config();
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
