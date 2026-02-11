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
    let hypr_monitors: Vec<HyprMonitor> = serde_json::from_slice(&output.stdout)?;

    let mut monitors: Vec<MonitorConfig> = hypr_monitors
        .iter()
        .map(|m| MonitorConfig {
            name: m.name.clone(),
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
