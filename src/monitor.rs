use anyhow::Result;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HyprMonitor {
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub make: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub available_modes: Vec<String>,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub resolution: String,
    pub refresh_rate: f64,
    pub position_x: i32,
    pub position_y: i32,
    pub scale: f64,
    pub rotation: Rotation,
    pub is_primary: bool,
    pub available_modes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    Normal,
    Left,
    Right,
    Inverted,
}

impl Rotation {
    pub fn as_str(self) -> &'static str {
        match self {
            Rotation::Normal => "Landscape",
            Rotation::Left => "Portrait Left",
            Rotation::Right => "Portrait Right",
            Rotation::Inverted => "Inverted",
        }
    }

    pub fn transform(self) -> u8 {
        match self {
            Rotation::Normal => 0,
            Rotation::Left => 1,
            Rotation::Right => 3,
            Rotation::Inverted => 2,
        }
    }

    pub fn from_transform<T: Into<u8>>(t: T) -> Self {
        match t.into() {
            1 => Rotation::Left,
            2 => Rotation::Inverted,
            3 => Rotation::Right,
            _ => Rotation::Normal,
        }
    }

    pub fn all() -> Vec<Rotation> {
        vec![
            Rotation::Normal,
            Rotation::Left,
            Rotation::Right,
            Rotation::Inverted,
        ]
    }
}

impl MonitorConfig {
    pub fn display_name(&self) -> String {
        if self.name.starts_with("eDP") {
            "Laptop".to_string()
        } else {
            self.model.clone()
        }
    }
}

pub fn fetch_monitors() -> Result<Vec<MonitorConfig>> {
    let output = Command::new("hyprctl").args(["monitors", "-j"]).output()?;
    parse_monitors(&output.stdout)
}

/// Parse `hyprctl monitors -j` output into sorted [`MonitorConfig`]s. Split from
/// the subprocess call so the mapping/sort/primary-fallback logic is unit-testable.
pub fn parse_monitors(json: &[u8]) -> Result<Vec<MonitorConfig>> {
    let hypr_monitors: Vec<HyprMonitor> = serde_json::from_slice(json)?;

    let mut monitors: Vec<MonitorConfig> = hypr_monitors
        .iter()
        .map(|m| {
            let desc = m
                .description
                .strip_suffix(&format!(" ({})", m.name))
                .unwrap_or(&m.description)
                .to_string();
            MonitorConfig {
                name: m.name.clone(),
                description: desc,
                make: m.make.clone(),
                model: m.model.clone(),
                resolution: format!("{}x{}", m.width, m.height),
                refresh_rate: m.refresh_rate,
                position_x: m.x,
                position_y: m.y,
                scale: m.scale,
                rotation: Rotation::from_transform(m.transform as u8),
                is_primary: m.focused,
                available_modes: m.available_modes.clone(),
            }
        })
        .collect();

    // Sort by x position
    monitors.sort_by_key(|m| m.position_x);

    // Ensure at least one is primary
    if !monitors.iter().any(|m| m.is_primary) && !monitors.is_empty() {
        monitors[0].is_primary = true;
    }

    Ok(monitors)
}

pub fn identify_monitors(monitors: &[MonitorConfig]) {
    for (i, monitor) in monitors.iter().enumerate() {
        let msg = format!("Monitor {}: {}", i + 1, monitor.display_name());

        // Use hyprctl notify with specific formatting
        // Icon types: 0=warning, 1=info, 2=hint, 3=error, 4=confused, 5=ok
        let _ = Command::new("hyprctl")
            .args([
                "notify",
                "5",           // OK icon (checkmark)
                "3000",        // 3 seconds
                "rgb(00ff00)", // Green color
                &format!("fontsize:40 {}", msg),
            ])
            .spawn();
    }
}

#[cfg(test)]
impl MonitorConfig {
    /// Build a representative monitor for tests across modules.
    pub fn for_test(name: &str, make: &str, model: &str, resolution: &str) -> Self {
        Self {
            name: name.into(),
            description: format!("{make} {model}").trim().to_string(),
            make: make.into(),
            model: model.into(),
            resolution: resolution.into(),
            refresh_rate: 60.0,
            position_x: 0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_transform_round_trips() {
        for r in Rotation::all() {
            assert_eq!(Rotation::from_transform(r.transform()), r);
        }
    }

    #[test]
    fn rotation_from_transform_maps_known_and_falls_back() {
        assert_eq!(Rotation::from_transform(0u8), Rotation::Normal);
        assert_eq!(Rotation::from_transform(1u8), Rotation::Left);
        assert_eq!(Rotation::from_transform(2u8), Rotation::Inverted);
        assert_eq!(Rotation::from_transform(3u8), Rotation::Right);
        assert_eq!(Rotation::from_transform(9u8), Rotation::Normal); // unknown -> Normal
    }

    #[test]
    fn rotation_as_str_and_all() {
        assert_eq!(Rotation::Normal.as_str(), "Landscape");
        assert_eq!(Rotation::Left.as_str(), "Portrait Left");
        assert_eq!(Rotation::Right.as_str(), "Portrait Right");
        assert_eq!(Rotation::Inverted.as_str(), "Inverted");
        assert_eq!(Rotation::all().len(), 4);
    }

    fn mc(name: &str, model: &str) -> MonitorConfig {
        MonitorConfig {
            name: name.into(),
            description: String::new(),
            make: String::new(),
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
    fn display_name_uses_laptop_for_edp_else_model() {
        assert_eq!(mc("eDP-1", "0x004D").display_name(), "Laptop");
        assert_eq!(mc("HDMI-A-1", "MSI MP275Q").display_name(), "MSI MP275Q");
    }

    #[test]
    fn parse_monitors_sorts_strips_desc_and_defaults_primary() {
        let json = br#"[
            {"name":"HDMI-A-1","description":"Microstep MSI MP275Q (HDMI-A-1)","make":"Microstep","model":"MSI MP275Q","width":2560,"height":1440,"refreshRate":99.95,"x":1920,"y":0,"scale":1.0,"transform":0,"availableModes":["2560x1440@99.95Hz"],"focused":false},
            {"name":"eDP-1","description":"Najing 0x004D (eDP-1)","make":"Najing","model":"0x004D","width":1920,"height":1080,"refreshRate":144.0,"x":0,"y":0,"scale":1.5,"transform":1,"availableModes":["1920x1080@144Hz"],"focused":false}
        ]"#;
        let monitors = parse_monitors(json).unwrap();
        assert_eq!(monitors.len(), 2);
        // sorted by x: eDP (0) first
        assert_eq!(monitors[0].name, "eDP-1");
        assert_eq!(monitors[1].name, "HDMI-A-1");
        // " (name)" suffix stripped from description
        assert_eq!(monitors[0].description, "Najing 0x004D");
        assert_eq!(monitors[1].description, "Microstep MSI MP275Q");
        // resolution composed, rotation from transform
        assert_eq!(monitors[1].resolution, "2560x1440");
        assert_eq!(monitors[0].rotation, Rotation::Left);
        // none focused -> first becomes primary
        assert!(monitors[0].is_primary);
        assert!(!monitors[1].is_primary);
    }

    #[test]
    fn parse_monitors_keeps_explicit_primary_and_handles_empty() {
        assert!(parse_monitors(b"[]").unwrap().is_empty());
        let json = br#"[{"name":"DP-1","description":"x","make":"","model":"X","width":1920,"height":1080,"refreshRate":60.0,"x":0,"y":0,"scale":1.0,"transform":0,"availableModes":[],"focused":true}]"#;
        let m = parse_monitors(json).unwrap();
        assert!(m[0].is_primary);
    }
}
