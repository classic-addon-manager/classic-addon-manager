//! Page registry: the pages listed in the sidebar and the one place where page views are created for the shell.
use gpui_kit::{
    AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
};

use super::{
    icons::{self, Lucide},
    theme::{TokensExt, TwText},
};

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
        PageId::Dashboard => dashboard(window, cx),
        PageId::Addons => addons(window, cx),
        PageId::Developer => developer(window, cx),
        PageId::Troubleshooting => troubleshooting(window, cx),
        PageId::Settings => settings(window, cx),
    }
}

fn dashboard(window: &mut Window, cx: &mut App) -> AnyView {
    let _ = &window;
    cx.new(|cx| crate::ui::pages::dashboard::DashboardPage::new(window, cx)).into()
}

fn addons(window: &mut Window, cx: &mut App) -> AnyView {
    let _ = &window;
    cx.new(|cx| crate::ui::pages::addons::AddonsPage::new(window, cx)).into()
}

fn developer(window: &mut Window, cx: &mut App) -> AnyView {
    let _ = &window;
    cx.new(|cx| crate::ui::pages::developer::DeveloperPage::new(window, cx)).into()
}

fn troubleshooting(window: &mut Window, cx: &mut App) -> AnyView {
    let _ = &window;
    cx.new(|cx| crate::ui::pages::troubleshooting::TroubleshootingPage::new(window, cx)).into()
}

fn settings(window: &mut Window, cx: &mut App) -> AnyView {
    let _ = &window;
    cx.new(|cx| crate::ui::pages::settings::SettingsPage::new(window, cx)).into()
}

// ---- placeholders -------------------------------------------------------------------------------------------------

struct Placeholder(SharedString);

impl Render for Placeholder {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(t.muted_foreground)
            .text_sm_tw()
            .child(self.0.clone())
    }
}

fn placeholder(name: &str, cx: &mut App) -> AnyView {
    let label = SharedString::from(format!("{name} page: not built yet"));
    cx.new(|_| Placeholder(label)).into()
}
