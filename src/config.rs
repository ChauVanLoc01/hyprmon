use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Where `save()` writes. Set by `load_from`; `None` falls back to the real
    /// config path. Skipped during (de)serialization so the JSON format is
    /// unchanged and tests can redirect persistence to a temp file.
    #[serde(skip)]
    config_path: Option<PathBuf>,
}

impl Default for MonitorDatabase {
    fn default() -> Self {
        Self {
            workspaces: vec![Workspace::new("Default")],
            active_workspace: 0,
            config_path: None,
        }
    }
}

impl MonitorDatabase {
    pub fn config_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".config/hypr/monitors.json")
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&Self::config_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let mut db: MonitorDatabase = serde_json::from_str(&content)?;
            if db.workspaces.is_empty() {
                db.workspaces.push(Workspace::new("Default"));
            }
            // Heal legacy entries where the same physical panel was stored twice
            // under serial-bearing and serial-free desc keys (Hyprland reports the
            // description inconsistently). Without this, both rules match the live
            // monitor and Hyprland's last-wins clobbers the intended position.
            for ws in &mut db.workspaces {
                dedup_prefix_keys(ws);
            }
            db.config_path = Some(path.to_path_buf());
            Ok(db)
        } else {
            Ok(MonitorDatabase {
                config_path: Some(path.to_path_buf()),
                ..Self::default()
            })
        }
    }

    /// Redirect where `save()` persists. Lets tests target a path other than the
    /// real `~/.config/hypr/monitors.json`.
    #[cfg(test)]
    pub fn set_config_path(&mut self, path: PathBuf) {
        self.config_path = Some(path);
    }

    pub fn save(&self) -> Result<()> {
        let path = self.config_path.clone().unwrap_or_else(Self::config_path);
        let content = serde_json::to_string_pretty(&self)?;
        atomic_write(&path, &content)?;
        Ok(())
    }

    /// Stable identity key for a monitor.
    ///
    /// Laptops keep their connector name (`eDP-1`). External monitors are keyed by
    /// `make + model` and deliberately drop the serial: Hyprland reports the serial
    /// in the description only intermittently, so including it splits one physical
    /// panel across multiple entries. Hyprland matches `desc:` as a prefix, so the
    /// serial-free form still resolves to the connected monitor.
    pub fn get_monitor_key(monitor: &MonitorConfig) -> String {
        if monitor.name.starts_with("eDP") {
            return monitor.name.clone();
        }
        let identity = format!("{} {}", monitor.make.trim(), monitor.model.trim());
        let identity = identity.trim();
        if identity.is_empty() {
            format!("desc:{}", monitor.description.trim())
        } else {
            format!("desc:{identity}")
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

    /// Generate the managed monitor block (monitor= rules + fallback) from ALL
    /// workspaces. The result is spliced between the hyprmon markers by
    /// [`splice_managed_block`]; it carries no file-level header of its own so it
    /// can be rewritten in place without accumulating duplicate headers.
    pub fn generate_full_config(&self) -> String {
        let mut config = String::new();

        // Collect all unique monitors across all workspaces (later workspaces win).
        let mut merged: HashMap<String, SavedMonitor> = HashMap::new();
        for ws in &self.workspaces {
            for (key, saved) in &ws.monitors {
                merged.insert(key.clone(), saved.clone());
            }
        }

        // Stable left-to-right order so the deconflict pass is deterministic
        // (HashMap iteration order is otherwise random across runs).
        let mut all_monitors: Vec<(String, SavedMonitor)> = merged.into_iter().collect();
        all_monitors.sort_by(|a, b| {
            a.1.position_x
                .cmp(&b.1.position_x)
                .then_with(|| a.1.position_y.cmp(&b.1.position_y))
                .then_with(|| a.0.cmp(&b.0))
        });

        // Deconflict horizontally: any monitor whose left edge falls inside the
        // running right edge is pushed to abut its neighbor. Hyprland's
        // directional monitor focus/move (movecurrentworkspacetomonitor l/r,
        // focusmonitor) silently fails ("Monitor not found") when connected
        // monitors overlap, so overlapping geometry must never reach the conf.
        let mut right_edge = i32::MIN;
        for (_key, saved) in all_monitors.iter_mut() {
            if saved.position_x < right_edge {
                saved.position_x = right_edge;
            }
            right_edge = saved
                .position_x
                .saturating_add(monitor_logical_width(saved));
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

/// Write `content` to `path` atomically: serialize into a sibling temp file and
/// `rename(2)` it over the target. rename is atomic within a filesystem, so a
/// crash or power loss mid-write can never leave a truncated/corrupt JSON file —
/// the old file stays intact until the new one is fully written.
fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

/// Markers delimiting the region of `monitors.conf` that hyprmon owns. Anything
/// outside these markers is authored by the user and must survive regeneration.
pub const BLOCK_BEGIN: &str = "# >>> hyprmon:begin >>> (auto-generated — do not edit between markers)";
pub const BLOCK_END: &str = "# <<< hyprmon:end <<<";

/// Splice the generated monitor `block` into `existing` between [`BLOCK_BEGIN`]
/// and [`BLOCK_END`], replacing only that region and preserving everything
/// outside it. When the markers are absent the file is treated as a legacy,
/// fully hyprmon-owned `monitors.conf` and replaced wholesale — otherwise the
/// previously generated `monitor=` lines would survive as conflicting rules.
/// Custom lines a user wants to keep must live outside the markers, which exist
/// after the first write.
pub fn splice_managed_block(existing: &str, block: &str) -> String {
    let managed = format!("{BLOCK_BEGIN}\n{}\n{BLOCK_END}\n", block.trim_end());

    if let (Some(start), Some(end_pos)) = (existing.find(BLOCK_BEGIN), existing.find(BLOCK_END)) {
        if start < end_pos {
            let end = end_pos + BLOCK_END.len();
            let after = existing[end..].strip_prefix('\n').unwrap_or(&existing[end..]);
            return format!("{}{managed}{after}", &existing[..start]);
        }
    }

    managed
}

/// Returns true when `short` is a `desc:` key that is a space-delimited prefix of
/// `long` — i.e. they identify the same panel, with `long` carrying an extra
/// serial token Hyprland sometimes appends.
fn is_desc_prefix(short: &str, long: &str) -> bool {
    if short == long || !short.starts_with("desc:") {
        return false;
    }
    long.strip_prefix(short)
        .map(|rest| rest.starts_with(' '))
        .unwrap_or(false)
}

/// Collapse duplicate entries for one physical panel, keeping the shortest
/// (serial-free) key. Hyprland's intermittent serial reporting otherwise leaves
/// two `monitor=` rules matching the same display with conflicting positions.
fn dedup_prefix_keys(ws: &mut Workspace) {
    let mut keys: Vec<String> = ws.monitors.keys().cloned().collect();
    keys.sort_by_key(|k| k.len()); // shortest (serial-free) first

    let mut kept: Vec<String> = Vec::new();
    for key in keys {
        if kept.iter().any(|k| is_desc_prefix(k, &key)) {
            ws.monitors.remove(&key);
        } else {
            kept.push(key);
        }
    }
}

/// Logical horizontal footprint of a monitor in the Hyprland layout, accounting
/// for scale and rotation. Portrait orientations (transform 1/3 = 90°/270°) swap
/// the panel's width and height, so the laid-out width becomes the panel height.
fn monitor_logical_width(saved: &SavedMonitor) -> i32 {
    let (w, h) = saved
        .resolution
        .trim()
        .split_once('x')
        .and_then(|(w, h)| Some((w.trim().parse::<i32>().ok()?, h.trim().parse::<i32>().ok()?)))
        .unwrap_or((0, 0));

    let footprint = if saved.rotation == 1 || saved.rotation == 3 {
        h
    } else {
        w
    };

    let scale = if saved.scale > 0.0 { saved.scale } else { 1.0 };
    ((footprint as f64) / scale).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(res: &str, scale: f64, x: i32) -> SavedMonitor {
        SavedMonitor {
            resolution: res.into(),
            refresh_rate: 60.0,
            scale,
            rotation: 0,
            position_x: x,
            position_y: 0,
            is_primary: false,
        }
    }

    fn db_with(monitors: Vec<(&str, SavedMonitor)>) -> MonitorDatabase {
        let mut ws = Workspace::new("Default");
        for (key, m) in monitors {
            ws.monitors.insert(key.to_string(), m);
        }
        MonitorDatabase {
            workspaces: vec![ws],
            active_workspace: 0,
            config_path: None,
        }
    }

    #[test]
    fn deconflicts_overlapping_monitors() {
        // eDP 0..1920 (scale 1.0); MSI stored at 1280 overlaps it.
        let db = db_with(vec![
            ("eDP-1", saved("1920x1080", 1.0, 0)),
            ("desc:MSI", saved("2560x1440", 1.0, 1280)),
        ]);
        let conf = db.generate_full_config();
        assert!(conf.contains("eDP-1,1920x1080@60.00,0x0,1"), "conf:\n{conf}");
        // MSI pushed to eDP's right edge -> no overlap.
        assert!(
            conf.contains("desc:MSI,2560x1440@60.00,1920x0,1"),
            "conf:\n{conf}"
        );
    }

    #[test]
    fn preserves_valid_fractional_scale_layout() {
        // eDP @1.5 -> logical width 1280; MSI at 1280 already abuts and must stay.
        let db = db_with(vec![
            ("eDP-1", saved("1920x1080", 1.5, 0)),
            ("desc:MSI", saved("2560x1440", 1.0, 1280)),
        ]);
        let conf = db.generate_full_config();
        assert!(
            conf.contains("eDP-1,1920x1080@60.00,0x0,1.50"),
            "conf:\n{conf}"
        );
        assert!(
            conf.contains("desc:MSI,2560x1440@60.00,1280x0,1"),
            "conf:\n{conf}"
        );
    }

    #[test]
    fn portrait_rotation_uses_height_as_footprint() {
        // Rotated panel (transform 1): footprint width = panel height = 1080.
        let mut left = saved("1920x1080", 1.0, 0);
        left.rotation = 1;
        let db = db_with(vec![
            ("eDP-1", left),
            ("desc:MSI", saved("2560x1440", 1.0, 500)),
        ]);
        let conf = db.generate_full_config();
        // eDP footprint 1080 -> MSI must land at 1080, not overlap.
        assert!(
            conf.contains("desc:MSI,2560x1440@60.00,1080x0,1"),
            "conf:\n{conf}"
        );
    }

    fn monitor(name: &str, make: &str, model: &str, desc: &str) -> MonitorConfig {
        MonitorConfig {
            name: name.into(),
            description: desc.into(),
            make: make.into(),
            model: model.into(),
            resolution: "1920x1080".into(),
            refresh_rate: 60.0,
            position_x: 0,
            position_y: 0,
            scale: 1.0,
            rotation: Rotation::Normal,
            is_primary: false,
            available_modes: vec![],
        }
    }

    #[test]
    fn stable_key_drops_serial() {
        let m = monitor(
            "HDMI-A-1",
            "Microstep",
            "MSI MP275Q",
            "Microstep MSI MP275Q PC3M265601090",
        );
        assert_eq!(
            MonitorDatabase::get_monitor_key(&m),
            "desc:Microstep MSI MP275Q"
        );
    }

    #[test]
    fn laptop_keeps_connector_name() {
        let m = monitor("eDP-1", "Najing", "0x004D", "Najing CEC 0x004D");
        assert_eq!(MonitorDatabase::get_monitor_key(&m), "eDP-1");
    }

    #[test]
    fn dedup_collapses_serial_duplicate() {
        let mut ws = Workspace::new("Default");
        ws.monitors.insert(
            "desc:ASUSTek COMPUTER INC VY249HF".into(),
            saved("1920x1080", 1.0, 0),
        );
        ws.monitors.insert(
            "desc:ASUSTek COMPUTER INC VY249HF R9LMRS025371".into(),
            saved("1920x1080", 1.0, 1920),
        );
        dedup_prefix_keys(&mut ws);
        assert_eq!(ws.monitors.len(), 1);
        assert!(ws.monitors.contains_key("desc:ASUSTek COMPUTER INC VY249HF"));
    }

    #[test]
    fn splice_replaces_legacy_then_preserves_external_edits() {
        // A legacy, markerless hyprmon file is replaced wholesale so old
        // monitor= rules cannot linger as conflicting duplicates.
        let legacy = "# Generated by hyprmon\nmonitor=eDP-1,1920x1080@60.00,0x0,1\n";
        let out1 = splice_managed_block(legacy, "monitor=eDP-1,1920x1080@60.00,1920x0,1.50\n");
        assert_eq!(out1.matches(BLOCK_BEGIN).count(), 1);
        assert!(out1.contains(",1920x0,1.50"));
        assert!(
            !out1.contains("monitor=eDP-1,1920x1080@60.00,0x0,1\n"),
            "legacy line must be gone:\n{out1}"
        );

        // Once markers exist, lines the user adds outside them survive regen.
        let edited = format!("{out1}workspace = 1, monitor:eDP-1\n");
        let out2 = splice_managed_block(&edited, "monitor=eDP-1,1920x1080@60.00,100x0,1.50\n");
        assert_eq!(out2.matches(BLOCK_BEGIN).count(), 1);
        assert_eq!(out2.matches(BLOCK_END).count(), 1);
        assert!(out2.contains(",100x0,1.50"));
        assert!(!out2.contains(",1920x0,1.50"));
        assert!(out2.contains("workspace = 1, monitor:eDP-1"));
    }

    #[test]
    fn atomic_write_persists_and_overwrites() {
        let mut path = std::env::temp_dir();
        path.push(format!("hyprmon_atomic_test_{}.json", std::process::id()));

        atomic_write(&path, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        // Overwriting leaves no temp residue and replaces content.
        atomic_write(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        assert!(!path.with_extension("json.tmp").exists());

        let _ = std::fs::remove_file(&path);
    }

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("hyprmon_{tag}_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn save_then_load_round_trips() {
        let p = temp_path("save_rt");
        let mut db = db_with(vec![("eDP-1", saved("1920x1080", 1.5, 0))]);
        db.set_config_path(p.clone());
        db.save().unwrap();

        let loaded = MonitorDatabase::load_from(&p).unwrap();
        assert_eq!(loaded.workspaces[0].monitors.len(), 1);
        assert!(loaded.workspaces[0].monitors.contains_key("eDP-1"));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_from_missing_returns_seeded_default() {
        let db = MonitorDatabase::load_from(&temp_path("missing")).unwrap();
        assert_eq!(db.workspaces.len(), 1);
        assert_eq!(db.workspaces[0].name, "Default");
    }

    #[test]
    fn load_from_empty_workspaces_seeds_default() {
        let p = temp_path("empty");
        std::fs::write(&p, r#"{"workspaces":[],"active_workspace":0}"#).unwrap();
        let db = MonitorDatabase::load_from(&p).unwrap();
        assert_eq!(db.workspaces.len(), 1);
        assert_eq!(db.workspaces[0].name, "Default");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_from_dedups_serial_duplicates() {
        let p = temp_path("dedup");
        let json = r#"{"workspaces":[{"name":"Default","monitors":{
            "desc:ASUSTek COMPUTER INC VY249HF":{"resolution":"1920x1080","refresh_rate":60.0,"scale":1.0,"rotation":0,"position_x":0,"position_y":0,"is_primary":false},
            "desc:ASUSTek COMPUTER INC VY249HF R9LMRS025371":{"resolution":"1920x1080","refresh_rate":60.0,"scale":1.0,"rotation":0,"position_x":1920,"position_y":0,"is_primary":false}
        }}],"active_workspace":0}"#;
        std::fs::write(&p, json).unwrap();
        let db = MonitorDatabase::load_from(&p).unwrap();
        assert_eq!(db.workspaces[0].monitors.len(), 1);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_path_points_at_hypr_dir() {
        assert!(MonitorDatabase::config_path().ends_with(".config/hypr/monitors.json"));
    }

    #[test]
    fn workspace_keys_and_match_count() {
        let mut ws = Workspace::new("Desk");
        ws.monitors.insert("eDP-1".into(), saved("1920x1080", 1.0, 0));
        ws.monitors.insert("desc:MSI".into(), saved("2560x1440", 1.0, 1920));
        assert_eq!(ws.name, "Desk");
        assert_eq!(ws.monitor_keys().len(), 2);

        let connected = vec![
            monitor("eDP-1", "Najing", "0x004D", "d"),  // key eDP-1 -> present
            monitor("HDMI-A-1", "MSI", "X", "d"),        // key desc:MSI X -> absent
        ];
        assert_eq!(ws.matches_monitors(&connected), 1);
    }

    #[test]
    fn current_workspace_accessors_and_out_of_range() {
        let mut db = db_with(vec![]);
        assert_eq!(db.current_workspace().unwrap().name, "Default");
        db.current_workspace_mut().unwrap().name = "Renamed".into();
        assert_eq!(db.current_workspace().unwrap().name, "Renamed");
        db.active_workspace = 99;
        assert!(db.current_workspace().is_none());
        assert!(db.current_workspace_mut().is_none());
    }

    #[test]
    fn update_monitor_upserts_and_apply_round_trips() {
        let mut db = db_with(vec![]);
        let mut m = monitor("HDMI-A-1", "MSI", "MP275Q", "MSI MP275Q");
        m.resolution = "2560x1440".into();
        m.scale = 1.25;
        m.position_x = 100;
        db.update_monitor(&m);
        m.position_x = 200; // same key -> overwrite
        db.update_monitor(&m);
        assert_eq!(db.current_workspace().unwrap().monitors.len(), 1);

        assert!(db.get_saved_config(&m).is_some());
        let mut fresh = monitor("HDMI-A-1", "MSI", "MP275Q", "MSI MP275Q");
        assert!(db.apply_saved_config(&mut fresh));
        assert_eq!(fresh.position_x, 200);
        assert_eq!(fresh.scale, 1.25);

        let mut other = monitor("DP-1", "Dell", "U2419", "Dell U2419");
        assert!(db.get_saved_config(&other).is_none());
        assert!(!db.apply_saved_config(&mut other));
    }

    #[test]
    fn find_best_workspace_picks_match_else_none() {
        let mut db = db_with(vec![("eDP-1", saved("1920x1080", 1.0, 0))]);
        db.add_workspace("Two");
        assert_eq!(
            db.find_best_workspace(&[monitor("eDP-1", "N", "M", "d")]),
            Some(0)
        );
        assert_eq!(db.find_best_workspace(&[monitor("DP-9", "Z", "Z", "d")]), None);
    }

    #[test]
    fn workspace_crud_and_active_clamp() {
        let mut db = db_with(vec![]);
        assert_eq!(db.add_workspace("Two"), 1);
        db.rename_workspace(1, "Renamed");
        assert_eq!(db.workspaces[1].name, "Renamed");
        assert!(db.delete_workspace(1));
        assert!(!db.delete_workspace(0)); // cannot delete the last one
        db.add_workspace("X");
        assert!(!db.delete_workspace(99)); // out of range

        let mut db2 = db_with(vec![]);
        db2.add_workspace("B");
        db2.active_workspace = 1;
        assert!(db2.delete_workspace(1));
        assert_eq!(db2.active_workspace, 0); // active clamped after delete
    }

    #[test]
    fn get_monitor_key_falls_back_to_description() {
        let mut m = monitor("HDMI-A-1", "", "", "Some Desc 123");
        assert_eq!(MonitorDatabase::get_monitor_key(&m), "desc:Some Desc 123");
        m.make = "MSI".into();
        m.model = "MP275Q".into();
        assert_eq!(MonitorDatabase::get_monitor_key(&m), "desc:MSI MP275Q");
    }

    #[test]
    fn get_workspace_monitors_reconstructs_and_out_of_range() {
        let db = db_with(vec![
            ("eDP-1", saved("1920x1080", 1.5, 0)),
            ("desc:ASUSTek COMPUTER INC VY249HF", saved("1920x1080", 1.0, 1920)),
        ]);
        assert_eq!(db.get_workspace_monitors(0).len(), 2);
        assert!(db.get_workspace_monitors(9).is_empty());
    }

    #[test]
    fn generate_writes_transform_for_rotated_monitor() {
        let mut m = saved("1920x1080", 1.0, 0);
        m.rotation = 1;
        let conf = db_with(vec![("eDP-1", m)]).generate_full_config();
        assert!(conf.contains(",transform,1"), "conf:\n{conf}");
    }

    #[test]
    fn logical_width_handles_bad_resolution_and_zero_scale() {
        let mut m = saved("garbage", 1.0, 0);
        assert_eq!(monitor_logical_width(&m), 0);
        m.resolution = "1920x1080".into();
        m.scale = 0.0; // guarded -> treated as 1.0
        assert_eq!(monitor_logical_width(&m), 1920);
    }

    #[test]
    fn is_desc_prefix_requires_space_boundary_and_desc_scheme() {
        assert!(is_desc_prefix("desc:MSI", "desc:MSI MP275Q"));
        assert!(!is_desc_prefix("desc:MSI", "desc:MSIX")); // no space boundary
        assert!(!is_desc_prefix("eDP-1", "eDP-1 x")); // not a desc: key
        assert!(!is_desc_prefix("desc:MSI", "desc:MSI")); // identical
    }
}
