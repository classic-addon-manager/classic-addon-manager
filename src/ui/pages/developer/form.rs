//! State of the declaration form shared by the publish form and the edit panel: values, dirty check, validation and
//! submit flow.
use gpui_kit::{Context, EventEmitter};

use super::{
    api::{self, SubmitAddonResult, ValidateResult},
    queries::DevQueries,
    types::*,
};
use crate::{
    backend::api::ApiError,
    state::{run_blocking, toast},
    ui::icons,
};

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

/// Form state for stored declaration values, without an icon change.
pub fn publish_form_from_payload(payload: DeclarationValues) -> PublishFormState {
    PublishFormState { values: payload, icon_asset_id: None, icon_url: None }
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

/// The fields in the order they appear on the form.
pub const FORM_FIELD_ORDER: [&str; 12] = [
    "name",
    "alias",
    "icon",
    "description",
    "author",
    "repo",
    "branch",
    "tags",
    "keywords",
    "dependencies",
    "library",
    "kofi",
];

pub fn has_error(errors: &FieldErrors, key: &str) -> bool {
    errors.get(key).is_some_and(|e| !e.is_empty())
}

/// First field (in form order) with errors.
pub fn first_error_key(errors: &FieldErrors) -> Option<&'static str> {
    FORM_FIELD_ORDER.iter().copied().find(|k| has_error(errors, k))
}

/// Events for the owner of a [`DeclarationForm`].
#[derive(Clone, Debug, PartialEq)]
pub enum FormEvent {
    /// The submission was created or updated, and the success toast was already shown.
    Submitted,
    /// An existing submission was loaded into the form.
    Resumed(DeclarationKind),
    /// Validation failed: bring this field, the first one in `FORM_FIELD_ORDER` with errors, into view.
    ScrollToError(&'static str),
}

/// All state and actions of the declaration editor. Own it in an `Entity` and observe it.
pub struct DeclarationForm {
    pub form: PublishFormState,
    /// Baseline for the dirty check.
    initial: PublishFormState,
    initial_submission_id: Option<i64>,
    /// Submission being revised, `None` creates a new submission.
    pub submission_id: Option<i64>,
    pub already_open_id: Option<i64>,
    pub validating: bool,
    pub validated: bool,
    pub submitting: bool,
    pub resuming: bool,
    pub editable: bool,
    pub field_errors: FieldErrors,
    pub validation_error: bool,
    pub publish_error: Option<String>,
    pub schema: Option<AddonSchema>,
    pub schema_error: Option<String>,
}

impl EventEmitter<FormEvent> for DeclarationForm {}

impl DeclarationForm {
    pub fn new(initial: PublishFormState, initial_submission_id: Option<i64>, cx: &mut Context<Self>) -> Self {
        let this = Self {
            form: initial.clone(),
            initial,
            initial_submission_id,
            submission_id: initial_submission_id,
            already_open_id: None,
            validating: false,
            validated: false,
            submitting: false,
            resuming: false,
            editable: true,
            field_errors: FieldErrors::new(),
            validation_error: false,
            publish_error: None,
            schema: None,
            schema_error: None,
        };
        let schema = DevQueries::fetch_schema(cx);
        cx.spawn(async move |this, cx| {
            let result = schema.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(SchemaDoc::Supported(schema)) => {
                        let mut values = api::schema_zero_values(&schema);
                        values.extend(std::mem::take(&mut this.form.values));
                        this.form.values = values;
                        this.schema = Some(schema);
                    }
                    Ok(SchemaDoc::Unsupported) => this.schema_error = Some(api::UNSUPPORTED_SCHEMA.into()),
                    Err(e) => this.schema_error = Some(e.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        this
    }

    pub fn busy(&self) -> bool {
        self.validating || self.submitting || self.resuming || !self.editable
    }

    pub fn dirty(&self) -> bool {
        self.submission_id != self.initial_submission_id || is_publish_form_dirty(&self.form, &self.initial)
    }

    pub fn has_field_errors(&self) -> bool {
        FORM_FIELD_ORDER.iter().any(|k| has_error(&self.field_errors, k))
    }

    /// The message to show after a failed attempt, `None` when nothing failed.
    pub fn fail_copy(&self) -> Option<String> {
        if self.has_field_errors() {
            Some("Fix the highlighted fields.".into())
        } else if let Some(e) = &self.publish_error {
            Some(e.clone())
        } else if self.validation_error {
            Some("Couldn't validate this addon.".into())
        } else {
            None
        }
    }

    fn clear_result(&mut self) {
        self.validated = false;
        self.validation_error = false;
        self.publish_error = None;
    }

    fn clear_field_error(&mut self, key: &str) {
        self.field_errors.remove(key);
    }

    /// Sets a field's value, then clears the validation result and that field's error.
    pub fn set_value(&mut self, key: &str, value: WireValue, cx: &mut Context<Self>) {
        if self.form.values.get(key) != Some(&value) {
            self.form.values.insert(key.to_owned(), value);
        }
        self.clear_result();
        self.clear_field_error(key);
        cx.notify();
    }

    /// Like [`Self::set_value`], with the new value computed from the current one.
    pub fn update_value(&mut self, key: &str, f: impl FnOnce(Option<&WireValue>) -> WireValue, cx: &mut Context<Self>) {
        let next = f(self.form.values.get(key));
        self.set_value(key, next, cx);
    }

    /// Sets the icon asset id and preview URL. Both changes clear the validation result and the icon error.
    pub fn set_icon(&mut self, asset_id: Option<String>, url: Option<String>, cx: &mut Context<Self>) {
        self.form.icon_asset_id = asset_id;
        self.clear_result();
        self.clear_field_error("icon");
        self.form.icon_url = url;
        self.clear_result();
        self.clear_field_error("icon");
        cx.notify();
    }

    fn apply_invalid_result(&mut self, fields: FieldErrors, cx: &mut Context<Self>) {
        if let Some(first) = first_error_key(&fields) {
            self.field_errors = fields;
            cx.emit(FormEvent::ScrollToError(first));
            return;
        }
        self.validation_error = true;
    }

    fn mark_not_open(&mut self) {
        self.editable = false;
        self.submission_id = None;
        self.publish_error = Some("This submission is no longer open.".into());
    }

    fn resume(&mut self, id: i64, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        // Values can't be read without the schema.
        let Some(schema) = self.schema.clone() else {
            self.publish_error = Some(api::VALUES.message(&ApiError::NotFound));
            cx.notify();
            return;
        };
        self.resuming = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = run_blocking(cx, move || api::values(&EditorSource::Submission { id }, &schema)).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Err(e) => this.publish_error = Some(api::VALUES.message(&e)),
                    Ok(SourceValues { kind, values }) => {
                        this.submission_id = Some(id);
                        this.form = values_to_form(&values);
                        this.clear_result();
                        this.field_errors.clear();
                        this.editable = true;
                        cx.emit(FormEvent::Resumed(kind));
                    }
                }
                this.resuming = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub fn validate(&mut self, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        self.validating = true;
        self.publish_error = None;
        self.field_errors.clear();
        self.validation_error = false;
        cx.notify();
        let form = self.form.clone();
        let submission_id = self.submission_id;
        cx.spawn(async move |this, cx| {
            let result = run_blocking(cx, move || api::validate_addon(form, submission_id)).await;
            let _ = this.update(cx, |this, cx| {
                let mut passed = false;
                match result {
                    ValidateResult::NotOpen(_) => this.mark_not_open(),
                    ValidateResult::AlreadyOpen(id) => this.already_open_id = Some(id),
                    ValidateResult::Failed(_) => this.validation_error = true,
                    ValidateResult::Valid => passed = true,
                    ValidateResult::Invalid { fields, .. } => this.apply_invalid_result(fields, cx),
                }
                this.validated = passed;
                this.validating = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub fn submit(&mut self, cx: &mut Context<Self>) {
        if self.busy() || !self.validated {
            return;
        }
        self.submitting = true;
        self.publish_error = None;
        self.field_errors.clear();
        cx.notify();
        let form = self.form.clone();
        let submission_id = self.submission_id;
        cx.spawn(async move |this, cx| {
            let result = run_blocking(cx, move || api::submit_addon(form, submission_id)).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    SubmitAddonResult::Submitted(_) => {
                        let created = this.submission_id.is_none();
                        let (title, description) = if created {
                            ("Addon submitted", "It's now in review.")
                        } else {
                            ("Submission updated", "Your changes were saved.")
                        };
                        toast::show(cx, toast::Toast::new(title, description).icon(icons::CHECK));
                        cx.emit(FormEvent::Submitted);
                    }
                    SubmitAddonResult::AlreadyOpen(id) => this.already_open_id = Some(id),
                    SubmitAddonResult::Invalid { fields, .. } => {
                        this.validated = false;
                        this.apply_invalid_result(fields, cx);
                    }
                    SubmitAddonResult::NotOpen(_) => this.mark_not_open(),
                    SubmitAddonResult::Error(message) => this.publish_error = Some(message),
                }
                this.submitting = false;
                cx.notify();
            });
        })
        .detach();
    }

    /// Validates first, then submits once the declaration has passed validation.
    pub fn primary_action(&mut self, cx: &mut Context<Self>) {
        if self.validated {
            self.submit(cx);
        } else {
            self.validate(cx);
        }
    }

    pub fn dismiss_already_open(&mut self, cx: &mut Context<Self>) {
        self.already_open_id = None;
        cx.notify();
    }

    pub fn resume_already_open(&mut self, cx: &mut Context<Self>) {
        let id = self.already_open_id.take();
        cx.notify();
        if let Some(id) = id {
            self.resume(id, cx);
        }
    }
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
    fn first_error_follows_form_order() {
        let mut errors = FieldErrors::new();
        errors.insert("kofi".into(), vec!["x".into()]);
        errors.insert("repo".into(), vec!["y".into()]);
        errors.insert("empty".into(), vec![]);
        assert_eq!(first_error_key(&errors), Some("repo"));
        assert_eq!(first_error_key(&FieldErrors::new()), None);
        errors.insert("icon".into(), vec!["z".into()]);
        assert_eq!(first_error_key(&errors), Some("icon"));
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
