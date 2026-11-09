//! What the addon lists filter and mark by: the search text and the "New" badge.
use chrono::{DateTime, Utc};

use super::format::days_ago;

/// Days after `added_at` during which an addon counts as new.
pub const NEW_ADDON_DAYS: i64 = 32;

/// True while the addon was added less than [`NEW_ADDON_DAYS`] days ago. An addon without an added time is not new.
pub fn is_new_addon(added_at: Option<DateTime<Utc>>) -> bool {
    added_at.is_some_and(|t| days_ago(&t) < NEW_ADDON_DAYS)
}

/// Search text of an addon list, trimmed and lower-cased once. An addon matches when the text is empty or found in its
/// name, alias, description or author, compared without case.
pub struct AddonSearch(String);

impl AddonSearch {
    pub fn new(text: &str) -> Self {
        Self(text.trim().to_lowercase())
    }

    pub fn matches(&self, name: &str, alias: &str, description: &str, author: &str) -> bool {
        self.0.is_empty()
            || [name, alias, description, author].iter().any(|field| field.to_lowercase().contains(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addon_search() {
        let search = |text: &str| AddonSearch::new(text).matches("bagpack", "BagPack", "Inventory helper", "Madpeter");
        assert!(search(""));
        assert!(search("   "));
        assert!(search(" MADPE "));
        assert!(search("inventory"));
        assert!(!search("zzz"));
    }
}
