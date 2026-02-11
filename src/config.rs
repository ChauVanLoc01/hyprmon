use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::monitor::{MonitorConfig, Rotation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMonitor {
    pub resolution: String,
    pub refresh_rate: f64,
    pub scale: f64,
    pub rotation: u8,
    pub position_x: i32,
    pub position_y: i32,
    #[serde(default)]
    pub is_primary: bool,
}

/// A workspace represents a saved monitor configuration for a specific location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub monitors: HashMap<String, SavedMonitor>,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            monitors: HashMap::new(),
        }
    }

    /// Get monitor keys in this workspace
    #[allow(dead_code)]
    pub fn monitor_keys(&self) -> Vec<String> {
        self.monitors.keys().cloned().collect()
    }

    /// Check if workspace matches current connected monitors
    pub fn matches_monitors(&self, connected: &[MonitorConfig]) -> usize {
        connected
            .iter()
            .filter(|m| {
                let key = MonitorDatabase::get_monitor_key(m);
                self.monitors.contains_key(&key)
            })
            .count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorDatabase {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub active_workspace: usize,
}

impl Default for MonitorDatabase {
    fn default() -> Self {
        Self {
            workspaces: vec![Workspace::new("Default")],
            active_workspace: 0,
        }
    }
}

impl MonitorDatabase {
    pub fn config_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".config/hypr/monitors.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let mut db: MonitorDatabase = serde_json::from_str(&content)?;
            if db.workspaces.is_empty() {
                db.workspaces.push(Workspace::new("Default"));
            }
            Ok(db)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get the identifier key for a monitor (desc:Description or eDP-1 for laptops)
    pub fn get_monitor_key(monitor: &MonitorConfig) -> String {
        if monitor.name.starts_with("eDP") {
            monitor.name.clone()
        } else {
            format!("desc:{}", monitor.description)
        }
    }

    /// Get current active workspace
    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.active_workspace)
    }

    /// Get current active workspace mutably
    pub fn current_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.active_workspace)
    }

    /// Update current workspace with monitor config
    pub fn update_monitor(&mut self, monitor: &MonitorConfig) {
        let key = Self::get_monitor_key(monitor);
        let saved = SavedMonitor {
            resolution: monitor.resolution.clone(),
            refresh_rate: monitor.refresh_rate,
            scale: monitor.scale,
            rotation: monitor.rotation.transform(),
            position_x: monitor.position_x,
            position_y: monitor.position_y,
            is_primary: monitor.is_primary,
        };

        if let Some(ws) = self.current_workspace_mut() {
            ws.monitors.insert(key, saved);
        }
    }

    /// Get saved config for a monitor from current workspace
    pub fn get_saved_config(&self, monitor: &MonitorConfig) -> Option<&SavedMonitor> {
        let key = Self::get_monitor_key(monitor);
        self.current_workspace()?.monitors.get(&key)
    }

    /// Apply saved config to a monitor
    pub fn apply_saved_config(&self, monitor: &mut MonitorConfig) -> bool {
        if let Some(saved) = self.get_saved_config(monitor) {
            monitor.resolution = saved.resolution.clone();
            monitor.refresh_rate = saved.refresh_rate;
            monitor.scale = saved.scale;
            monitor.rotation = Rotation::from_transform(saved.rotation);
            monitor.position_x = saved.position_x;
            monitor.position_y = saved.position_y;
            monitor.is_primary = saved.is_primary;
            true
        } else {
            false
        }
    }

    /// Find best matching workspace for connected monitors
    pub fn find_best_workspace(&self, connected: &[MonitorConfig]) -> Option<usize> {
        let mut best_idx = None;
        let mut best_score = 0;

        for (idx, ws) in self.workspaces.iter().enumerate() {
            let score = ws.matches_monitors(connected);
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }

        if best_score > 0 {
            best_idx
        } else {
            None
        }
    }

    /// Add a new workspace
    pub fn add_workspace(&mut self, name: &str) -> usize {
        self.workspaces.push(Workspace::new(name));
        self.workspaces.len() - 1
    }

    /// Delete workspace at index
    pub fn delete_workspace(&mut self, idx: usize) -> bool {
        if self.workspaces.len() <= 1 || idx >= self.workspaces.len() {
            return false;
        }
        self.workspaces.remove(idx);
        if self.active_workspace >= self.workspaces.len() {
            self.active_workspace = self.workspaces.len() - 1;
        }
        true
    }

    /// Rename workspace
    pub fn rename_workspace(&mut self, idx: usize, name: &str) {
        if let Some(ws) = self.workspaces.get_mut(idx) {
            ws.name = name.to_string();
        }
    }

    /// Generate full Hyprland config from ALL workspaces
    pub fn generate_full_config(&self) -> String {
        let mut config = String::from(
            "# Hyprland Monitor Configuration\n# Generated by hyprmon\n# Contains ALL configured monitors from all workspaces\n\n",
        );

        // Collect all unique monitors across all workspaces
        let mut all_monitors: HashMap<String, &SavedMonitor> = HashMap::new();

        for ws in &self.workspaces {
            for (key, saved) in &ws.monitors {
                // Later workspaces override earlier ones
                all_monitors.insert(key.clone(), saved);
            }
        }

        for (key, saved) in &all_monitors {
            let transform = saved.rotation;
            let scale = if saved.scale.fract() == 0.0 {
                format!("{}", saved.scale as i32)
            } else {
                format!("{:.2}", saved.scale)
            };
            if transform == 0 {
                config.push_str(&format!(
                    "monitor={},{}@{:.2},{}x{},{}\n",
                    key,
                    saved.resolution,
                    saved.refresh_rate,
                    saved.position_x,
                    saved.position_y,
                    scale
                ));
            } else {
                config.push_str(&format!(
                    "monitor={},{}@{:.2},{}x{},{},transform,{}\n",
                    key,
                    saved.resolution,
                    saved.refresh_rate,
                    saved.position_x,
                    saved.position_y,
                    scale,
                    transform
                ));
            }
        }

        config.push_str("\n# Fallback for unknown monitors\nmonitor=,preferred,auto,1\n");
        config
    }

    /// Get monitors from a specific workspace as MonitorConfig
    pub fn get_workspace_monitors(&self, ws_idx: usize) -> Vec<MonitorConfig> {
        let Some(ws) = self.workspaces.get(ws_idx) else {
            return vec![];
        };

        ws.monitors
            .iter()
            .map(|(key, saved)| {
                let (name, description, make, model) = if key.starts_with("desc:") {
                    let desc = key.strip_prefix("desc:").unwrap_or(key).to_string();
                    let parts: Vec<&str> = desc.rsplitn(2, ' ').collect();
                    let model = parts.first().unwrap_or(&"").to_string();
                    let make = parts.get(1).unwrap_or(&"").to_string();
                    (key.clone(), desc, make, model)
                } else {
                    (key.clone(), String::new(), String::new(), key.clone())
                };

                MonitorConfig {
                    name,
                    description,
                    make,
                    model,
                    resolution: saved.resolution.clone(),
                    refresh_rate: saved.refresh_rate,
                    position_x: saved.position_x,
                    position_y: saved.position_y,
                    scale: saved.scale,
                    rotation: Rotation::from_transform(saved.rotation),
                    is_primary: saved.is_primary,
                    available_modes: vec![format!(
                        "{}@{:.0}Hz",
                        saved.resolution, saved.refresh_rate
                    )],
                }
            })
            .collect()
    }
}
