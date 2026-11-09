//! The eight accent colours of the Settings page, keyed by the `accentcolor` value in config.toml.
use cam_core::config::AccentColorId;
use cam_ui::theme::Accent;

#[derive(Clone, Copy, Debug)]
pub struct AccentPreset {
    pub id: AccentColorId,
    pub label: &'static str,
    pub accent: Accent,
}

const fn preset(id: AccentColorId, label: &'static str, l: f32, c: f32, h: f32) -> AccentPreset {
    AccentPreset { id, label, accent: Accent { l, c, h } }
}

/// In the order the Settings page shows them.
pub const ACCENT_PRESETS: [AccentPreset; 8] = [
    preset(AccentColorId::Gold, "Gold", 0.83, 0.095, 80.0),
    preset(AccentColorId::Amber, "Amber", 0.8, 0.14, 65.0),
    preset(AccentColorId::Rose, "Rose", 0.72, 0.16, 15.0),
    preset(AccentColorId::Violet, "Violet", 0.7, 0.17, 295.0),
    preset(AccentColorId::Blue, "Blue", 0.7, 0.14, 250.0),
    preset(AccentColorId::Cyan, "Cyan", 0.78, 0.11, 195.0),
    preset(AccentColorId::Emerald, "Emerald", 0.78, 0.14, 155.0),
    preset(AccentColorId::Slate, "Slate", 0.8, 0.02, 260.0),
];

/// The theme accent of the configured colour `id`.
pub fn accent(id: AccentColorId) -> Accent {
    ACCENT_PRESETS.iter().find(|p| p.id == id).map(|p| p.accent).expect("every accent colour has a preset")
}
