//! The declaration form: every field of an addon's declaration, including the keywords input and the dependencies
//! combobox.
//!
//! [`AddonDeclarationFields`] is a view over a [`DeclarationForm`] entity (the single source of truth: `form.form.values`).
//! It renders `form.schema` grouped by `field.section` (`Section` cards), with the repo row (`github.com/` prefix + branch),
//! the icon row after `alias`, the ko-fi hint link, the dependencies combobox (shared `Combobox`, filled from the
//! app-wide addon catalog), the keywords `TagsInput`, toggle chips for multi-choice fields (capped by `max_items`),
//! checkboxes and text inputs/areas.
//! Without a schema it renders the muted paragraph (`schema_error` or `Loading…`).
//!
//! * Text inputs / text areas / tag inputs are stateful entities created once, when the schema arrives, and keyed by field key.
//!   Typing pushes `WireValue`s into the form. Whenever the form changes (resume, loading) the entities are updated only when
//!   their content differs, so there is no feedback loop and an unchanged value never clears the validation result.
//! * `Field` is a label, a control and a hint with the error popover. In focus mode ([`ErrorMode::Focus`]) the popover
//!   shows above the control while focus is inside the field. In label mode ([`ErrorMode::Label`]) an alert icon next
//!   to the label shows it on hover. The popover is a pointer-transparent `Floating` layer, so it never steals focus.
//! * `scroll_to_field(key)` centres the field (`addon-<key>`, its bounds are recorded every frame) in the
//!   `ScrollHandle` passed with `set_scroll_handle` unless it is already fully visible.
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, AppContext as _, Bounds, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, ObjectFit, ParentElement, Pixels, Render, ScrollHandle, SharedString, Size,
    StatefulInteractiveElement as _, Styled, StyledImage as _, Subscription, Window, canvas, div, img,
    prelude::FluentBuilder as _, px, transparent_black,
};

use super::{
    form::{DeclarationForm, bool_of, has_error, list_of, text_of},
    icon_picker::{self, IconChange},
    types::*,
};
use crate::{
    state::{self, Load},
    ui::{
        components::{
            Align, Button, ButtonSize, ButtonVariant, Checkbox, Combobox, ComboboxSide, CommandItem, Input, InputEvent,
            InputState, Label, Section, Side, TagsChanged, TagsInput, Textarea, TextareaState, Toggle, ToggleSize,
            ToggleVariant,
            floating::{Floating, Placement},
            text_area, text_input_with_value,
        },
        icons,
        theme::{Tokens, TokensExt, TwText, alpha, radius, shadow_xs},
    },
};

/// Description shown under a section title, if the section has one.
fn section_description(section: &str) -> Option<&'static str> {
    (section == "Repository").then_some("GitHub user/repo and branch.")
}

/// Where the validation message of a [`field`](AddonDeclarationFields::field) shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ErrorMode {
    /// Popover above the control while it has focus.
    Focus,
    /// Alert icon next to the label, popover on hover.
    Label,
}

/// Per-field UI state: focus tracking (focus mode), popover anchor/measure cells and the icon hover (label mode).
struct FieldUi {
    focus: FocusHandle,
    anchor: Rc<Cell<Bounds<Pixels>>>,
    measured: Rc<Cell<Option<Size<Pixels>>>>,
    hovered: bool,
    _subs: [Subscription; 2],
}

/// Everything `render` reads from the form, copied once so the view can be borrowed mutably while building.
struct Snapshot {
    values: DeclarationValues,
    errors: FieldErrors,
    busy: bool,
    icon_url: Option<String>,
}

impl Snapshot {
    fn errs(&self, key: &str) -> Vec<String> {
        self.errors.get(key).cloned().unwrap_or_default()
    }
}

struct FieldSpec {
    key: SharedString,
    label: SharedString,
    hint: Option<AnyElement>,
    errors: Vec<String>,
    mode: ErrorMode,
    /// Clicking the label focuses this input.
    label_target: Option<FocusHandle>,
}

type BoundsMap = Rc<RefCell<HashMap<SharedString, Bounds<Pixels>>>>;

pub struct AddonDeclarationFields {
    form: Entity<DeclarationForm>,
    locked_fields: Vec<String>,
    scroll: Option<ScrollHandle>,
    built: bool,
    inputs: HashMap<String, Entity<InputState>>,
    areas: HashMap<String, Entity<TextareaState>>,
    tags: HashMap<String, Entity<TagsInput>>,
    /// Last `(disabled, invalid)` pushed into each `TagsInput` (its setters notify, so only call them on change).
    tags_props: HashMap<String, (bool, bool)>,
    ui: HashMap<SharedString, FieldUi>,
    bounds: BoundsMap,
    _subs: Vec<Subscription>,
}

impl AddonDeclarationFields {
    pub fn new(
        form: Entity<DeclarationForm>,
        locked_fields: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let observe = cx.observe_in(&form, window, |this, _form, window, cx| {
            this.ensure_widgets(window, cx);
            this.sync_from_form(window, cx);
            cx.notify();
        });
        let catalog = cx.observe(&state::catalog(cx), |_, _, cx| cx.notify());
        let mut this = Self {
            form,
            locked_fields,
            scroll: None,
            built: false,
            inputs: HashMap::new(),
            areas: HashMap::new(),
            tags: HashMap::new(),
            tags_props: HashMap::new(),
            ui: HashMap::new(),
            bounds: Rc::default(),
            _subs: vec![observe, catalog],
        };
        this.ensure_widgets(window, cx);
        state::catalog::ensure(cx);
        this
    }

    /// Immutable schema fields listed here are disabled with the "can't be changed after publish" hint.
    pub fn set_locked_fields(&mut self, locked_fields: Vec<String>, cx: &mut Context<Self>) {
        if self.locked_fields != locked_fields {
            self.locked_fields = locked_fields;
            self.apply_props(cx);
            cx.notify();
        }
    }

    /// The scroll container holding this view (needed by [`Self::scroll_to_field`]).
    pub fn set_scroll_handle(&mut self, handle: ScrollHandle) {
        self.scroll = Some(handle);
    }

    /// Scrolls the field with this key (`addon-<key>`) to the centre of the scroll container unless fully visible.
    pub fn scroll_to_field(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(handle) = &self.scroll else { return };
        let Some(target) = self.bounds.borrow().get(key).copied() else { return };
        let viewport = handle.bounds();
        if target.top() >= viewport.top() && target.bottom() <= viewport.bottom() {
            return;
        }
        let mut offset = handle.offset();
        let max = handle.max_offset();
        offset.y = (offset.y + viewport.center().y - target.center().y).clamp(-max.y, px(0.0));
        handle.set_offset(offset);
        cx.notify();
        window.refresh();
    }

    // -----------------------------------------------------------------------------------------------------------------
    // Model sync
    // -----------------------------------------------------------------------------------------------------------------

    /// Creates the input entities once the schema is there.
    fn ensure_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.built {
            return;
        }
        let Some(schema) = self.form.read(cx).schema.clone() else { return };
        self.built = true;
        let has_repo = schema.fields.iter().any(|f| f.key == "repo");
        for field in &schema.fields {
            let key = field.key.clone();
            let initial = text_of(&self.form.read(cx).form.values, &key);
            match field.key.as_str() {
                "dependencies" => {}
                "repo" | "kofi" => {
                    self.create_input(&key, false, if key == "repo" { "user/repo" } else { "" }, initial, window, cx)
                }
                "branch" if has_repo => self.create_input(&key, false, "main", initial, window, cx),
                _ => match field.widget {
                    Widget::Text => self.create_input(&key, false, "", initial, window, cx),
                    Widget::Textarea => self.create_input(&key, true, "", initial, window, cx),
                    Widget::StringList => self.create_tags(&key, window, cx),
                    Widget::EnumMulti | Widget::Checkbox => {}
                },
            }
        }
        self.apply_props(cx);
    }

    fn create_input(
        &mut self,
        key: &str,
        multiline: bool,
        placeholder: &'static str,
        initial: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key_owned = key.to_owned();
        if multiline {
            let state = text_area(window, cx, placeholder);
            if !initial.is_empty() {
                state.update(cx, |s, cx| s.set_value(initial, window, cx));
            }
            let sub = cx.subscribe_in(&state, window, move |this, input, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let text = input.read(cx).value().to_string();
                    this.push_text(&key_owned, text, cx);
                }
            });
            self._subs.push(sub);
            self.areas.insert(key.to_owned(), state);
        } else {
            let state = text_input_with_value(window, cx, placeholder, initial);
            let sub = cx.subscribe_in(&state, window, move |this, input, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let text = input.read(cx).value().to_string();
                    this.push_text(&key_owned, text, cx);
                }
            });
            self._subs.push(sub);
            self.inputs.insert(key.to_owned(), state);
        }
    }

    fn create_tags(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let initial: Vec<SharedString> =
            list_of(&self.form.read(cx).form.values, key).into_iter().map(Into::into).collect();
        let tags = cx.new(|cx| TagsInput::new(window, cx, ""));
        tags.update(cx, |t, cx| t.set_values(initial, cx));
        let key_owned = key.to_owned();
        let sub = cx.subscribe(&tags, move |this, _, ev: &TagsChanged, cx| {
            this.push_list(&key_owned, ev.0.iter().map(|s| s.to_string()).collect(), cx);
        });
        self._subs.push(sub);
        self.tags.insert(key.to_owned(), tags);
    }

    /// Input -> form (no-op when the form already holds this text, e.g. after a form -> input push).
    fn push_text(&mut self, key: &str, text: String, cx: &mut Context<Self>) {
        if text_of(&self.form.read(cx).form.values, key) == text {
            return;
        }
        self.form.update(cx, |f, cx| f.set_value(key, WireValue::Text(text), cx));
    }

    fn push_list(&mut self, key: &str, list: Vec<String>, cx: &mut Context<Self>) {
        if list_of(&self.form.read(cx).form.values, key) == list {
            return;
        }
        self.form.update(cx, |f, cx| f.set_value(key, WireValue::List(list), cx));
    }

    /// Form -> inputs, only where the content differs.
    fn sync_from_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let values = self.form.read(cx).form.values.clone();
        let inputs: Vec<(String, Entity<InputState>)> =
            self.inputs.iter().map(|(k, s)| (k.clone(), s.clone())).collect();
        for (key, state) in inputs {
            let want = text_of(&values, &key);
            if state.read(cx).value().as_ref() != want.as_str() {
                state.update(cx, |s, cx| s.set_value(want, window, cx));
            }
        }
        let areas: Vec<(String, Entity<TextareaState>)> =
            self.areas.iter().map(|(k, s)| (k.clone(), s.clone())).collect();
        for (key, state) in areas {
            let want = text_of(&values, &key);
            if state.read(cx).value().as_ref() != want.as_str() {
                state.update(cx, |s, cx| s.set_value(want, window, cx));
            }
        }
        let tags: Vec<(String, Entity<TagsInput>)> = self.tags.iter().map(|(k, s)| (k.clone(), s.clone())).collect();
        for (key, entity) in tags {
            let want = list_of(&values, &key);
            let differs = {
                let have = entity.read(cx).values();
                have.len() != want.len() || have.iter().zip(&want).any(|(a, b)| a.as_ref() != b.as_str())
            };
            if differs {
                entity.update(cx, |t, cx| t.set_values(want.into_iter().map(Into::into).collect(), cx));
            }
        }
        self.apply_props(cx);
    }

    fn is_locked(&self, field: &SchemaField) -> bool {
        field.immutable && self.locked_fields.contains(&field.key)
    }

    /// Pushes `disabled` / `invalid` into the `TagsInput` entities (the other widgets get them at render time).
    fn apply_props(&mut self, cx: &mut Context<Self>) {
        let (busy, schema, errors) = {
            let f = self.form.read(cx);
            (f.busy(), f.schema.clone(), f.field_errors.clone())
        };
        let Some(schema) = schema else { return };
        for field in &schema.fields {
            let Some(entity) = self.tags.get(&field.key).cloned() else { continue };
            let props = (busy || self.is_locked(field), has_error(&errors, &field.key));
            let last = self.tags_props.get(&field.key).copied();
            if last.map(|l| l.0) != Some(props.0) {
                entity.update(cx, |t, cx| t.set_disabled(props.0, cx));
            }
            if last.map(|l| l.1) != Some(props.1) {
                entity.update(cx, |t, cx| t.set_invalid(props.1, cx));
            }
            self.tags_props.insert(field.key.clone(), props);
        }
    }

    // -----------------------------------------------------------------------------------------------------------------
    // Shared chrome
    // -----------------------------------------------------------------------------------------------------------------

    fn ensure_ui(&mut self, key: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.ui.contains_key(key) {
            return;
        }
        let focus = cx.focus_handle();
        let focus_in = cx.on_focus_in(&focus, window, |_, _, cx| cx.notify());
        let focus_out = cx.on_focus_out(&focus, window, |_, _, _, cx| cx.notify());
        self.ui.insert(
            key.clone(),
            FieldUi {
                focus,
                anchor: Rc::default(),
                measured: Rc::default(),
                hovered: false,
                _subs: [focus_in, focus_out],
            },
        );
    }

    /// Records the bounds of its (relative) parent every frame: the `addon-<key>` element for `scroll_to_field` and
    /// optionally the popover anchor.
    fn probe(&self, key: SharedString, anchor: Option<Rc<Cell<Bounds<Pixels>>>>) -> AnyElement {
        let map = self.bounds.clone();
        canvas(
            move |bounds, _, _| {
                map.borrow_mut().insert(key, bounds);
                if let Some(anchor) = anchor {
                    anchor.set(bounds);
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
        .into_any_element()
    }

    /// A control that is not wrapped in a [`Field`](Self::field) but is a scroll target.
    fn tracked(&self, key: &str, control: impl IntoElement) -> AnyElement {
        div().relative().child(control).child(self.probe(SharedString::from(key.to_owned()), None)).into_any_element()
    }

    /// The field wrapper: label (+ alert icon in label mode), control, hint, error popover.
    fn field(
        &mut self,
        spec: FieldSpec,
        control: AnyElement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.tokens();
        self.ensure_ui(&spec.key, window, cx);
        let (focus, anchor, measured, hovered) = {
            let ui = &self.ui[&spec.key];
            (ui.focus.clone(), ui.anchor.clone(), ui.measured.clone(), ui.hovered)
        };
        let has_error = !spec.errors.is_empty();
        let label_mode = spec.mode == ErrorMode::Label;
        let active = if label_mode { hovered } else { focus.contains_focused(window, cx) };
        let popover = (has_error && active).then(|| error_popover(&t, &spec.errors, anchor.get(), measured));

        let key = spec.key.clone();
        let label = div()
            .id(SharedString::from(format!("label-{key}")))
            .child(Label::new(spec.label))
            .when_some(spec.label_target, |d, handle| d.on_click(move |_, window, cx| window.focus(&handle, cx)));

        let mut alert = None;
        let mut focus_popover = None;
        if label_mode {
            if has_error {
                let hover_key = key.clone();
                alert = Some(
                    div()
                        .id(SharedString::from(format!("alert-{key}")))
                        .relative()
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .text_color(t.destructive)
                        .child(Icon::new(icons::CIRCLE_ALERT).size(px(14.0)))
                        .child(self.probe(SharedString::from(format!("alert-{key}")), Some(anchor.clone())))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if let Some(ui) = this.ui.get_mut(&hover_key)
                                && ui.hovered != *hovered
                            {
                                ui.hovered = *hovered;
                                cx.notify();
                            }
                        }))
                        .children(popover),
                );
            }
        } else {
            focus_popover = popover;
        }

        let wrapper = div()
            .relative()
            .when(!label_mode, |d| d.track_focus(&focus))
            .child(control)
            .child(self.probe(key.clone(), (!label_mode).then(|| anchor.clone())))
            .children(focus_popover);

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().flex().items_center().gap(px(6.0)).child(label).children(alert))
            .child(wrapper)
            .children(spec.hint.map(|hint| div().text_xs_tw().text_color(t.muted_foreground).child(hint)))
            .into_any_element()
    }

    // -----------------------------------------------------------------------------------------------------------------
    // Fields
    // -----------------------------------------------------------------------------------------------------------------

    fn render_field(
        &mut self,
        field: &SchemaField,
        branch: Option<&SchemaField>,
        snap: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match field.key.as_str() {
            "repo" => self.repo_row(field, branch, snap, window, cx),
            "dependencies" => self.dependencies_field(field, snap, window, cx),
            "kofi" => self.kofi_field(field, snap, window, cx),
            _ => self.generic_field(field, snap, window, cx),
        }
    }

    fn generic_field(
        &mut self,
        field: &SchemaField,
        snap: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.tokens();
        let key = field.key.as_str();
        let errors = snap.errs(key);
        let invalid = !errors.is_empty();
        let locked = self.is_locked(field);
        let disabled = snap.busy || locked;
        let hint =
            if locked { Some(format!("{} can't be changed after publish.", field.label)) } else { field.hint.clone() };
        let spec = |mode, label_target| FieldSpec {
            key: field.key.clone().into(),
            label: field.label.clone().into(),
            hint: hint.clone().map(|h| h.into_any_element()),
            errors: errors.clone(),
            mode,
            label_target,
        };

        match field.widget {
            Widget::Checkbox => {
                let form = self.form.clone();
                let key_owned = field.key.clone();
                let label = div()
                    .text_sm_tw()
                    .line_height(px(14.0))
                    .when(!disabled, |d| d.cursor_pointer())
                    .when(disabled, |d| d.opacity(0.5))
                    .child(field.label.clone());
                let checkbox = Checkbox::new(SharedString::from(format!("addon-{key}")))
                    .checked(bool_of(&snap.values, key))
                    .disabled(disabled)
                    .invalid(invalid)
                    .on_change(move |checked, _, cx| {
                        form.update(cx, |f, cx| f.set_value(&key_owned, WireValue::Bool(*checked), cx))
                    })
                    .child(label);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(self.tracked(key, div().flex().child(checkbox)))
                    .children(hint.map(|h| div().text_xs_tw().text_color(t.muted_foreground).child(h)))
                    .children(
                        errors.into_iter().map(|m| {
                            div().text_xs_tw().font_weight(FontWeight::MEDIUM).text_color(t.destructive).child(m)
                        }),
                    )
                    .into_any_element()
            }
            Widget::EnumMulti => {
                let selected = list_of(&snap.values, key);
                let max = field.max_items.filter(|&m| m > 0);
                let chips = field.enum_values.iter().cloned().enumerate().map(|(ix, option)| {
                    let on = selected.contains(&option);
                    let off = disabled || (!on && max.is_some_and(|m| selected.len() >= m));
                    let (form, key_owned, opt) = (self.form.clone(), field.key.clone(), option.clone());
                    let mut toggle = Toggle::new(SharedString::from(format!("enum-{key}-{ix}")))
                        .size(ToggleSize::Sm)
                        .variant(ToggleVariant::Outline)
                        .pressed(on)
                        .disabled(off)
                        .px(px(10.0))
                        .on_pressed_change(move |pressed, _, cx| {
                            let opt = opt.clone();
                            form.update(cx, |f, cx| {
                                f.update_value(
                                    &key_owned,
                                    move |prev| {
                                        let mut list = match prev {
                                            Some(WireValue::List(l)) => l.clone(),
                                            _ => Vec::new(),
                                        };
                                        if pressed {
                                            if !(list.contains(&opt) || max.is_some_and(|m| list.len() >= m)) {
                                                list.push(opt);
                                            }
                                        } else {
                                            list.retain(|item| *item != opt);
                                        }
                                        WireValue::List(list)
                                    },
                                    cx,
                                )
                            });
                        })
                        .label(option);
                    if on {
                        toggle = toggle.bg(t.primary).text_color(t.primary_foreground);
                    }
                    toggle
                });
                let group = div().flex().flex_wrap().gap(px(6.0)).children(chips);
                self.field(spec(ErrorMode::Label, None), group.into_any_element(), window, cx)
            }
            Widget::StringList => {
                let Some(tags) = self.tags.get(key).cloned() else { return div().into_any_element() };
                self.field(spec(ErrorMode::Focus, None), tags.into_any_element(), window, cx)
            }
            Widget::Text | Widget::Textarea => {
                let control_state = if field.widget == Widget::Textarea {
                    self.areas.get(key).cloned().map(|state| {
                        (
                            state.focus_handle(cx),
                            Textarea::new(&state).disabled(disabled).invalid(invalid).into_any_element(),
                        )
                    })
                } else {
                    self.inputs.get(key).cloned().map(|state| {
                        (
                            state.focus_handle(cx),
                            Input::new(&state).disabled(disabled).invalid(invalid).into_any_element(),
                        )
                    })
                };
                let Some((handle, control)) = control_state else { return div().into_any_element() };
                self.field(spec(ErrorMode::Focus, Some(handle)), control, window, cx)
            }
        }
    }

    fn repo_row(
        &mut self,
        field: &SchemaField,
        branch: Option<&SchemaField>,
        snap: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.tokens();
        let repo_errors = snap.errs("repo");
        let invalid = !repo_errors.is_empty();
        let mut repo_field = div().flex_1().min_w_0();
        if let Some(state) = self.inputs.get("repo").cloned() {
            let focused = state.focus_handle(cx).is_focused(window);
            let (border, shadows) = match (invalid, focused && !snap.busy) {
                (true, _) => (t.destructive, shadow_xs()),
                (false, true) => (t.ring, shadow_xs()),
                (false, false) => (t.input, shadow_xs()),
            };
            let container = div()
                .flex()
                .h(px(36.0))
                .overflow_hidden()
                .rounded(radius::MD)
                .border_1()
                .border_color(border)
                .bg(alpha(t.input, 0.3))
                .shadow(shadows)
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.0))
                        .border_r_1()
                        .border_color(t.input)
                        .px(px(10.0))
                        .text_sm_tw()
                        .text_color(t.muted_foreground)
                        .child(Icon::new(icons::GITHUB).size(px(14.0)))
                        .child("github.com/"),
                )
                .child(
                    Input::new(&state)
                        .disabled(snap.busy)
                        .invalid(invalid)
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .rounded(px(0.0))
                        .border_0()
                        .bg(transparent_black())
                        .shadow(Vec::new()),
                );
            let spec = FieldSpec {
                key: "repo".into(),
                label: field.label.clone().into(),
                hint: field.hint.clone().map(|h| h.into_any_element()),
                errors: repo_errors,
                mode: ErrorMode::Focus,
                label_target: Some(state.focus_handle(cx)),
            };
            repo_field = repo_field.child(self.field(spec, container.into_any_element(), window, cx));
        }
        let mut branch_cell = div().w(px(160.0)).flex_none();
        if let (Some(branch), Some(state)) = (branch, self.inputs.get("branch").cloned()) {
            let errors = snap.errs("branch");
            let invalid = !errors.is_empty();
            let spec = FieldSpec {
                key: "branch".into(),
                label: branch.label.clone().into(),
                hint: None,
                errors,
                mode: ErrorMode::Focus,
                label_target: Some(state.focus_handle(cx)),
            };
            let control = Input::new(&state).disabled(snap.busy).invalid(invalid).into_any_element();
            branch_cell = branch_cell.child(self.field(spec, control, window, cx));
        }
        div().flex().gap(px(16.0)).child(repo_field).child(branch_cell).into_any_element()
    }

    fn kofi_field(
        &mut self,
        field: &SchemaField,
        snap: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.tokens();
        let Some(state) = self.inputs.get("kofi").cloned() else { return div().into_any_element() };
        let errors = snap.errs("kofi");
        let invalid = !errors.is_empty();
        let kofi = text_of(&snap.values, "kofi");
        let kofi = kofi.trim();
        let (link_text, url) = if kofi.is_empty() {
            ("ko-fi.com".to_owned(), "https://ko-fi.com".to_owned())
        } else {
            (format!("ko-fi.com/{kofi}"), format!("https://ko-fi.com/{kofi}"))
        };
        let primary = t.primary;
        let hint = div()
            .flex()
            .flex_wrap()
            .child("Players can support you at")
            .child(div().w(px(3.0)))
            .child(
                div()
                    .id("kofi-link")
                    .cursor_pointer()
                    .text_color(primary)
                    .hover(move |s| s.text_color(primary).underline())
                    .on_click(move |_, _, cx| cx.open_url(&url))
                    .child(link_text),
            )
            .child(".")
            .into_any_element();
        let spec = FieldSpec {
            key: "kofi".into(),
            label: format!("{} username", field.label).into(),
            hint: Some(hint),
            errors,
            mode: ErrorMode::Focus,
            label_target: Some(state.focus_handle(cx)),
        };
        let control = Input::new(&state).disabled(snap.busy).invalid(invalid).into_any_element();
        self.field(spec, control, window, cx)
    }

    fn dependencies_field(
        &mut self,
        field: &SchemaField,
        snap: &Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let errors = snap.errs("dependencies");
        let selected: Vec<SharedString> = list_of(&snap.values, "dependencies").into_iter().map(Into::into).collect();
        let (form, busy) = (self.form.clone(), snap.busy);
        let mut combobox = Combobox::new("addon-dependencies")
            .multiple(true)
            .selected(selected)
            .placeholder("Select addons")
            .search_placeholder("Search addons…")
            .empty_text("")
            .side(ComboboxSide::Top)
            .invalid(!errors.is_empty())
            .disabled(busy)
            .on_change(move |names, _, cx| {
                let list = names.iter().map(|n| n.to_string()).collect();
                form.update(cx, |f, cx| f.set_value("dependencies", WireValue::List(list), cx));
            });
        combobox = match state::catalog::load(cx) {
            Load::Idle | Load::Pending => combobox.status("Loading addons…"),
            Load::Failed(_) => combobox.status("Couldn't load addons.").status_action("Retry", move |_, cx| {
                if !busy {
                    state::catalog::refresh(cx).detach();
                }
            }),
            Load::Ready(catalog) if catalog.addons().is_empty() => combobox.status("No addons available."),
            Load::Ready(catalog) => combobox.items(
                catalog
                    .addons()
                    .iter()
                    .map(|m| CommandItem::new(m.name.clone(), m.alias.clone()).keywords(m.name.clone())),
            ),
        };
        let spec = FieldSpec {
            key: "dependencies".into(),
            label: field.label.clone().into(),
            hint: field.hint.clone().map(|h| h.into_any_element()),
            errors,
            mode: ErrorMode::Focus,
            label_target: None,
        };
        self.field(spec, combobox.into_any_element(), window, cx)
    }

    fn icon_row(&mut self, snap: &Snapshot, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.tokens();
        let preview = match snap.icon_url.as_deref().filter(|u| !u.is_empty()) {
            Some(url) => div()
                .flex_none()
                .size(px(48.0))
                .rounded(radius::MD)
                .border_1()
                .border_color(t.border)
                .overflow_hidden()
                .child(img(url.to_owned()).size(px(46.0)).object_fit(ObjectFit::Cover))
                .into_any_element(),
            None => div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(48.0))
                .rounded(radius::MD)
                .border_1()
                .border_color(t.border)
                .bg(t.card)
                .text_color(t.muted_foreground)
                .child(Icon::new(icons::IMAGE).size(px(20.0)))
                .into_any_element(),
        };
        let (form, current) = (self.form.clone(), snap.icon_url.clone());
        let change = Button::new("addon-icon-change")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .label("Change icon")
            .disabled(snap.busy)
            .on_click(move |_, window, cx| {
                let form = form.clone();
                icon_picker::open(window, cx, current.clone(), move |change, _window, cx| {
                    form.update(cx, |f, cx| match change {
                        IconChange::Removed => f.set_icon(Some(String::new()), None, cx),
                        IconChange::Uploaded { asset_id, url } => f.set_icon(Some(asset_id), Some(url), cx),
                    })
                });
            });
        let spec = FieldSpec {
            key: "icon".into(),
            label: "Icon".into(),
            hint: Some("PNG only · 1:1 · min 50×50 · max 3 MB".into_any_element()),
            errors: snap.errs("icon"),
            mode: ErrorMode::Label,
            label_target: None,
        };
        let control = div().flex().items_center().gap(px(12.0)).child(preview).child(change).into_any_element();
        self.field(spec, control, window, cx)
    }
}

/// Error popover above the anchor, aligned to its start and 8 px away, at most 320 px wide. It is pointer-transparent.
fn error_popover(
    t: &Tokens,
    errors: &[String],
    anchor: Bounds<Pixels>,
    measured: Rc<Cell<Option<Size<Pixels>>>>,
) -> AnyElement {
    let destructive = t.destructive;
    let body = div()
        .flex()
        .gap(px(8.0))
        .max_w(px(320.0))
        .p(px(12.0))
        .rounded(radius::MD)
        .border_1()
        .border_color(t.border)
        .bg(t.popover)
        .text_color(t.popover_foreground)
        .shadow_md()
        .child(
            div().mt(px(2.0)).flex_none().text_color(destructive).child(Icon::new(icons::CIRCLE_ALERT).size(px(14.0))),
        )
        .child(div().flex().flex_col().gap(px(4.0)).min_w_0().children(errors.iter().map(|message| {
            div().text_xs_tw().font_weight(FontWeight::MEDIUM).text_color(destructive).child(message.clone())
        })));
    Floating::new(anchor, measured, Placement::new(Side::Top, Align::Start, 8.0, 0.0), body)
        .occlude(false)
        .into_any_element()
}

impl Render for AddonDeclarationFields {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let (schema, snap, schema_error) = {
            let f = self.form.read(cx);
            (
                f.schema.clone(),
                Snapshot {
                    values: f.form.values.clone(),
                    errors: f.field_errors.clone(),
                    busy: f.busy(),
                    icon_url: f.form.icon_url.clone(),
                },
                f.schema_error.clone(),
            )
        };
        let Some(schema) = schema else {
            return div()
                .text_sm_tw()
                .text_color(t.muted_foreground)
                .child(schema_error.unwrap_or_else(|| "Loading…".to_owned()))
                .into_any_element();
        };
        self.ensure_widgets(window, cx);

        let has_repo = schema.fields.iter().any(|f| f.key == "repo");
        let branch_field = schema.fields.iter().find(|f| f.key == "branch");
        let mut sections: Vec<(&str, Vec<&SchemaField>)> = Vec::new();
        for field in &schema.fields {
            if field.key == "branch" && has_repo {
                continue;
            }
            match sections.iter_mut().find(|(name, _)| *name == field.section) {
                Some((_, list)) => list.push(field),
                None => sections.push((field.section.as_str(), vec![field])),
            }
        }

        let mut section_els = Vec::new();
        for (title, fields) in sections {
            let mut rows: Vec<AnyElement> = Vec::new();
            let mut i = 0;
            while i < fields.len() {
                let field = fields[i];
                let next = fields.get(i + 1).copied();
                if let Some(next) = next.filter(|n| field.key != "repo" && field.half && n.half) {
                    let left = self.render_field(field, branch_field, &snap, window, cx);
                    let right = self.render_field(next, branch_field, &snap, window, cx);
                    rows.push(
                        div()
                            .flex()
                            .gap(px(16.0))
                            .child(div().flex_1().min_w_0().child(left))
                            .child(div().flex_1().min_w_0().child(right))
                            .into_any_element(),
                    );
                    if field.key == "alias" || next.key == "alias" {
                        rows.push(self.icon_row(&snap, window, cx));
                    }
                    i += 2;
                    continue;
                }
                rows.push(div().child(self.render_field(field, branch_field, &snap, window, cx)).into_any_element());
                if field.key == "alias" {
                    rows.push(self.icon_row(&snap, window, cx));
                }
                i += 1;
            }
            let mut section = Section::new(title.to_owned())
                .child(div().flex().flex_col().gap(px(16.0)).px(px(16.0)).py(px(16.0)).children(rows));
            if let Some(description) = section_description(title) {
                section = section.description(description);
            }
            section_els.push(section);
        }
        div().flex().flex_col().gap(px(32.0)).children(section_els).into_any_element()
    }
}
