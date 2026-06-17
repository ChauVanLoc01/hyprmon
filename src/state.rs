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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_field_all_lists_every_variant_in_order() {
        let all = SettingField::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], SettingField::Resolution);
        assert_eq!(all[4], SettingField::Primary);
    }

    #[test]
    fn setting_field_labels_are_distinct_and_nonempty() {
        let labels: Vec<&str> = SettingField::all().iter().map(|f| f.label()).collect();
        assert_eq!(labels.len(), 5);
        assert!(labels.iter().all(|l| l.ends_with(':')));
        // all distinct
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len());
    }
}
