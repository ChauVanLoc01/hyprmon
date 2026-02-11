use anyhow::Result;
use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum HyprEvent {
    MonitorAdded(String),
    MonitorRemoved(String),
}

fn get_socket_path() -> Result<PathBuf> {
    let instance_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
    let xdg_runtime = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

    // Try new path first (Hyprland 0.40+)
    let new_path = PathBuf::from(&xdg_runtime)
        .join("hypr")
        .join(&instance_sig)
        .join(".socket2.sock");

    if new_path.exists() {
        return Ok(new_path);
    }

    // Fall back to old path
    let old_path = PathBuf::from("/tmp/hypr")
        .join(&instance_sig)
        .join(".socket2.sock");

    Ok(old_path)
}

pub fn start_listener(tx: Sender<HyprEvent>) -> Result<()> {
    let socket_path = get_socket_path()?;
    let stream = UnixStream::connect(&socket_path)?;
    let reader = BufReader::new(stream);

    std::thread::spawn(move || {
        for line in reader.lines().map_while(Result::ok) {
            if let Some(event) = parse_event(&line) {
                let _ = tx.send(event);
            }
        }
    });

    Ok(())
}

fn parse_event(line: &str) -> Option<HyprEvent> {
    let parts: Vec<&str> = line.splitn(2, ">>").collect();
    if parts.len() != 2 {
        return None;
    }

    let event_type = parts[0];
    let data = parts[1];

    match event_type {
        "monitoradded" | "monitoraddedv2" => Some(HyprEvent::MonitorAdded(data.to_string())),
        "monitorremoved" => Some(HyprEvent::MonitorRemoved(data.to_string())),
        _ => None,
    }
}
