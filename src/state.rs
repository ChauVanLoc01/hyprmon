use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainTab {
    Live,
    Saved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPanel {
    Arrangement,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingField {
    Resolution,
    RefreshRate,
    Scale,
    Rotation,
    Primary,
}

impl SettingField {
    pub fn all() -> Vec<SettingField> {
        vec![
            SettingField::Resolution,
            SettingField::RefreshRate,
            SettingField::Scale,
            SettingField::Rotation,
            SettingField::Primary,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingField::Resolution => "Resolution:",
            SettingField::RefreshRate => "Refresh Rate:",
            SettingField::Scale => "Scale:",
            SettingField::Rotation => "Rotation:",
            SettingField::Primary => "Primary:",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogType {
    None,
    ConfirmApply { countdown: u8, started: Instant },
    ConfirmQuit,
    EditDropdown,
    NewWorkspace,
    RenameWorkspace,
    DeleteWorkspace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    Dragging {
        monitor_idx: usize,
        start_x: u16,
        start_y: u16,
        current_x: u16,
        current_y: u16,
    },
}
