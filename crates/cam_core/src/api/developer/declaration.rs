//! A declaration being edited: its field values plus the pending icon change, the dirty check against the stored
//! values, and the conversion to and from what the endpoints send and return.
use super::types::{DeclarationValues, WireValue};

/// The field values of a declaration plus the pending icon change.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishFormState {
    /// Wire-typed values keyed by schema field key.
    pub values: DeclarationValues,
    /// Opaque temporary asset ID from upload; `None` means preserve the current icon, `Some("")` removes it.
    pub icon_asset_id: Option<String>,
    /// Server-projected preview URL; display only, never submitted or reconstructed.
    pub icon_url: Option<String>,
}

/// Whether `form` differs from `baseline`. Empty text, `false` and an absent value count as equal, and so do an empty
/// list and an absent one.
pub fn is_publish_form_dirty(form: &PublishFormState, baseline: &PublishFormState) -> bool {
    let keys: std::collections::BTreeSet<&String> = form.values.keys().chain(baseline.values.keys()).collect();
    for key in keys {
        let a = form.values.get(key);
        let b = baseline.values.get(key);
        if matches!(a, Some(WireValue::List(_))) || matches!(b, Some(WireValue::List(_))) {
            let empty = Vec::new();
            let list_a = if let Some(WireValue::List(l)) = a { l } else { &empty };
            let list_b = if let Some(WireValue::List(l)) = b { l } else { &empty };
            if list_a != list_b {
                return true;
            }
            continue;
        }
        let blank = |v: Option<&WireValue>| {
            matches!(v, Some(WireValue::Text(s)) if s.is_empty()) || matches!(v, Some(WireValue::Bool(false)))
        };
        if (a.is_none() && blank(b)) || (b.is_none() && blank(a)) {
            continue;
        }
        if a != b {
            return true;
        }
    }
    form.icon_asset_id != baseline.icon_asset_id
}

/// The values to submit: an absent `icon_asset_id` preserves the server's icon, `''` removes it, a UUID replaces it.
pub fn form_to_values(form: PublishFormState) -> DeclarationValues {
    let mut values = form.values;
    if let Some(id) = form.icon_asset_id {
        values.insert("icon_asset_id".into(), WireValue::Text(id));
    }
    values
}

/// Form state for stored declaration values: `icon_url` moves out of the values into the preview URL.
pub fn values_to_form(values: &DeclarationValues) -> PublishFormState {
    let mut rest = values.clone();
    let icon_url = rest.remove("icon_url");
    PublishFormState {
        values: rest,
        icon_asset_id: None,
        icon_url: match icon_url {
            Some(WireValue::Text(s)) if !s.is_empty() => Some(s),
            _ => None,
        },
    }
}

pub fn text_of(values: &DeclarationValues, key: &str) -> String {
    match values.get(key) {
        Some(WireValue::Text(s)) => s.clone(),
        _ => String::new(),
    }
}

pub fn list_of(values: &DeclarationValues, key: &str) -> Vec<String> {
    match values.get(key) {
        Some(WireValue::List(l)) => l.clone(),
        _ => Vec::new(),
    }
}

pub fn bool_of(values: &DeclarationValues, key: &str) -> bool {
    matches!(values.get(key), Some(WireValue::Bool(true)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn form(pairs: &[(&str, WireValue)]) -> PublishFormState {
        PublishFormState {
            values: pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            ..Default::default()
        }
    }

    fn t(s: &str) -> WireValue {
        WireValue::Text(s.into())
    }

    #[test]
    fn dirty_rules() {
        let base = PublishFormState::default();
        assert!(!is_publish_form_dirty(&base, &base));
        // schema zero values merged in after load are not a change
        let zeros = form(&[("name", t("")), ("tags", WireValue::List(vec![])), ("library", WireValue::Bool(false))]);
        assert!(!is_publish_form_dirty(&zeros, &base));
        assert!(!is_publish_form_dirty(&base, &zeros));
        assert!(is_publish_form_dirty(&form(&[("name", t("x"))]), &zeros));
        assert!(is_publish_form_dirty(&form(&[("library", WireValue::Bool(true))]), &zeros));
        assert!(is_publish_form_dirty(&form(&[("tags", WireValue::List(vec!["a".into()]))]), &zeros));
        assert!(!is_publish_form_dirty(
            &form(&[("tags", WireValue::List(vec!["a".into()]))]),
            &form(&[("tags", WireValue::List(vec!["a".into()]))])
        ));
        assert!(is_publish_form_dirty(
            &form(&[("tags", WireValue::List(vec!["a".into()]))]),
            &form(&[("tags", WireValue::List(vec!["b".into()]))])
        ));
        let mut icon = zeros.clone();
        icon.icon_asset_id = Some("x".into());
        assert!(is_publish_form_dirty(&icon, &zeros));
        // an undefined key vs a non-empty value is dirty
        assert!(is_publish_form_dirty(&form(&[("name", t("x"))]), &base));
    }

    #[test]
    fn values_and_form_roundtrip() {
        let mut values = DeclarationValues::new();
        values.insert("name".into(), t("Foo"));
        values.insert("icon_url".into(), t("https://cdn/x.png"));
        let f = values_to_form(&values);
        assert_eq!(f.icon_url.as_deref(), Some("https://cdn/x.png"));
        assert!(!f.values.contains_key("icon_url"));
        assert!(!form_to_values(f.clone()).contains_key("icon_asset_id"));
        let mut removed = f;
        removed.icon_asset_id = Some(String::new());
        assert_eq!(form_to_values(removed)["icon_asset_id"], t(""));
        values.insert("icon_url".into(), t(""));
        assert_eq!(values_to_form(&values).icon_url, None);
    }

    #[test]
    fn helpers() {
        let f = form(&[("a", t("x")), ("b", WireValue::List(vec!["y".into()])), ("c", WireValue::Bool(true))]);
        assert_eq!(text_of(&f.values, "a"), "x");
        assert_eq!(text_of(&f.values, "b"), "");
        assert_eq!(list_of(&f.values, "b"), vec!["y"]);
        assert!(bool_of(&f.values, "c"));
        assert!(!bool_of(&f.values, "a"));
    }
}
