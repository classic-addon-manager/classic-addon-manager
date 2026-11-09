//! The window: the shell around the pages, the pages themselves, the chat dialog and the parts they share. Widgets that
//! know nothing about addons come from `cam_ui`.
mod accent;
mod addon_filter;
mod addon_icon;
mod chat;
mod dependency_notify;
mod format;
#[cfg(debug_assertions)]
pub mod inspector;
mod pages;
mod rating_buttons;
mod shell;

pub use accent::accent;
pub use shell::Shell;
