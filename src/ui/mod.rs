#![allow(dead_code, unused_imports)]
pub mod anim;
pub mod assets;
pub mod components;
pub mod dependency_notify;
pub mod icons;
mod icons_generated;
#[cfg(debug_assertions)]
pub mod inspector;
pub mod page_registry;
pub mod pages;
pub mod shell;
pub mod sidebar;
pub mod theme;
pub mod titlebar;
pub mod toast;
pub mod utils;

pub use shell::Shell;
