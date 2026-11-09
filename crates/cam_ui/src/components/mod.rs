//! The widgets: buttons, form fields, cards, tabs, dialogs, menus, tooltips, empty and error states and the markdown
//! renderer. Each widget lives in its own submodule and is re-exported from here.

// --- basic (button, badge, card, label, skeleton, progress, separator, collapsible, accordion, tabs,
//     toggle, toggle_group, scroll_area, section, empty_state, error_box)
pub mod accordion;
pub mod badge;
pub mod button;
pub mod card;
pub mod collapsible;
pub mod empty_state;
pub mod error_box;
pub mod label;
pub mod progress;
pub mod scroll_area;
pub mod section;
pub mod separator;
pub mod skeleton;
pub mod spinner;
pub mod style_ext;
pub mod tabs;
pub mod toggle;
pub mod toggle_group;
// --- inputs (input, textarea, checkbox, switch, select, command/combobox)
pub mod checkbox;
pub mod command;
pub mod input;
pub mod select;
pub mod switch;
pub mod textarea;
// --- overlays (dialog, alert_dialog, error_dialog, external_link_dialog, dropdown_menu, context_menu, popover, tooltip)
pub mod alert_dialog;
pub mod context_menu;
pub mod dialog;
pub mod dropdown_menu;
pub mod error_dialog;
pub mod external_link_dialog;
pub mod floating;
pub mod menu;
pub mod popover;
pub mod tooltip;
// --- markdown (readme)
pub mod markdown_ast;
pub mod markdown_html;
pub mod readme;

pub use accordion::*;
pub use alert_dialog::*;
pub use badge::*;
pub use button::*;
pub use card::*;
pub use checkbox::*;
pub use collapsible::*;
pub use command::*;
pub use context_menu::*;
pub use dialog::*;
pub use dropdown_menu::*;
pub use empty_state::*;
pub use error_box::error_box;
pub use error_dialog::ErrorItem;
pub use floating::{Align, Side};
pub use input::*;
pub use label::*;
pub use menu::{Menu, MenuItem, MenuItemVariant, MenuView};
pub use popover::*;
pub use progress::*;
pub use readme::*;
pub use scroll_area::*;
pub use section::*;
pub use select::*;
pub use separator::*;
pub use skeleton::*;
pub use spinner::*;
pub use style_ext::*;
pub use switch::*;
pub use tabs::*;
pub use textarea::*;
pub use toggle::*;
pub use toggle_group::*;
pub use tooltip::*;
