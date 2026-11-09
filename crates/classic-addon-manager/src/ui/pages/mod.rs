//! The pages listed in the sidebar, one subdirectory each, and the one place where the shell creates their views.
pub mod addons;
pub mod dashboard;
pub mod developer;
pub mod settings;
pub mod troubleshooting;

use cam_ui::icons::{self, Lucide};
use gpui_kit::{AnyView, App, AppContext, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageId {
    Dashboard,
    Addons,
    Developer,
    Troubleshooting,
    Settings,
}

pub struct PageDefinition {
    pub id: PageId,
    pub name: &'static str,
    pub icon: Lucide,
}

/// The sidebar pages, in sidebar order.
pub const PAGE_DEFINITIONS: [PageDefinition; 5] = [
    PageDefinition { id: PageId::Dashboard, name: "Dashboard", icon: icons::HOUSE },
    PageDefinition { id: PageId::Addons, name: "Addons", icon: icons::BLOCKS },
    PageDefinition { id: PageId::Developer, name: "Developer", icon: icons::CODE_XML },
    PageDefinition { id: PageId::Troubleshooting, name: "Troubleshooting", icon: icons::WRENCH },
    PageDefinition { id: PageId::Settings, name: "Settings", icon: icons::SETTINGS_2 },
];

/// Creates a fresh page view. Pages are rebuilt on every switch, so each visit starts from a clean state.
pub fn build_page(id: PageId, window: &mut Window, cx: &mut App) -> AnyView {
    match id {
        PageId::Dashboard => cx.new(|cx| dashboard::DashboardPage::new(window, cx)).into(),
        PageId::Addons => cx.new(|cx| addons::AddonsPage::new(window, cx)).into(),
        PageId::Developer => cx.new(|cx| developer::DeveloperPage::new(window, cx)).into(),
        PageId::Troubleshooting => cx.new(|cx| troubleshooting::TroubleshootingPage::new(window, cx)).into(),
        PageId::Settings => cx.new(|cx| settings::SettingsPage::new(window, cx)).into(),
    }
}
