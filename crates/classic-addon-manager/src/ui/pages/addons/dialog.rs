//! Dialog showing one catalog addon: banner and header, warning, stats, description, changelog, dependencies, support
//! link and the install / uninstall bar.
//!
//! One view entity holds the shown manifest and the install / uninstall actions. The release, README, dependency
//! and installed lookups come from the app's stores. Following a dependency chip swaps the manifest inside the same
//! dialog.
use std::{collections::HashSet, sync::Arc, time::Duration};

use cam_core::{
    addon::{DependencyInfo, DependencyResolutionResult, InstallError, InstallProgress, resolve_dependencies_with},
    api::{AddonManifest, Release},
};
use cam_ui::{
    components::{
        Button, ButtonSize, ButtonVariant, Collapsible, DialogSpec, Readme, ScrollbarAxis, Side, Skeleton, Tooltip,
        dialog, floating::window_area, scrollbar_overlay,
    },
    icons,
    theme::{Tokens, TokensExt, Tw, TwText, alpha, radius, shadow_md, shadow_sm, tw},
    toast::{self, Toast},
};
use gpui_kit::component::Icon;
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, AnyWindowHandle, App, AppContext as _, Context, ElementId, Entity,
    FontWeight, ImageAssetLoader, ImageSource, InteractiveElement as _, IntoElement, MouseButton, ObjectFit,
    ParentElement, Render, RenderImage, Resource, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled,
    StyledImage as _, Subscription, Window, canvas, div, ease_out_quint, img, linear_color_stop, linear_gradient,
    point, prelude::FluentBuilder as _, px, relative,
};

use crate::{
    state::{self, Load, Operation, catalog, ratings::RatingModel, tasks::error_message},
    ui::{
        addon_icon::{AddonIcon, AddonIconSize},
        dependency_notify::notify_dependency_result,
        format::{days_ago, format_number, format_to_local_date},
        rating_buttons::RatingButtons,
    },
};

const COLLAPSED_MAX_HEIGHT: f32 = 280.0;
const RECENT_RELEASE_DAYS: i64 = 14;
const SHOW_SCROLL_TOP_AFTER: f32 = 240.0;

/// Text of the changelog block.
pub fn changelog_text(release: &Load<Release>) -> String {
    match release {
        Load::Idle | Load::Pending => String::new(),
        Load::Failed(_) => "Error loading change log".to_string(),
        Load::Ready(r) if r.body.is_empty() => "No change log was provided".to_string(),
        Load::Ready(r) => r.body.clone(),
    }
}

/// Text of the description section: the README when it loaded and is not empty, otherwise the manifest description,
/// otherwise "No description provided."
pub fn readme_text(readme: &Load<Option<String>>, manifest: &AddonManifest) -> String {
    let from_readme = match readme {
        Load::Ready(Some(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    };
    from_readme
        .or_else(|| (!manifest.description.is_empty()).then(|| manifest.description.clone()))
        .unwrap_or_else(|| "No description provided.".to_string())
}

/// Toast text for an install that failed before the store recorded a per-addon result.
pub fn install_error_message(manifest: &AddonManifest, error: &anyhow::Error) -> String {
    if InstallError::from_error(error) == InstallError::NoRelease {
        format!("No release found for {}", manifest.name)
    } else {
        format!("Failed to install {}: {}", manifest.alias, error_message(error))
    }
}

/// Button text and fill for a running install: `Downloading 49%`, then `Installing 30%` while files are extracted, or
/// `LibCommon 49%` for a dependency. `None` between the download and the extraction.
pub fn progress_label(addon_name: &str, progress: &InstallProgress) -> Option<(String, f32)> {
    let fraction = progress.fraction()?;
    let percent = (fraction * 100.0).floor() as u32;
    let label = if progress.name != addon_name {
        format!("{} {percent}%", progress.alias)
    } else if progress.verified {
        format!("Installing {percent}%")
    } else {
        format!("Downloading {percent}%")
    };
    Some((label, fraction as f32))
}

// ---------------------------------------------------------------------------------------------------------
// dialog view
// ---------------------------------------------------------------------------------------------------------

pub struct RemoteAddonDialog {
    manifest: AddonManifest,
    window_handle: AnyWindowHandle,
    rating: Entity<RatingModel>,
    _rating_sub: Subscription,
    /// Re-render on changes to the release, README and catalog lookups and to the addon store, which holds the
    /// installed addons and running actions.
    _subs: Vec<Subscription>,
    scroll: ScrollHandle,
    show_scroll_top: bool,
    // description section
    expanded: bool,
    overflowing: bool,
    // changelog section
    changelog_open: bool,
    previous_recent: bool,
}

impl RemoteAddonDialog {
    /// Opens the dialog for `manifest`.
    pub fn open(manifest: AddonManifest, window: &mut Window, cx: &mut App) {
        let handle = window.window_handle();
        let view = cx.new(|cx| Self::new(manifest, handle, cx));
        // Size: at least 650x500 and at most 90% of the window height. The widest it may get is 850px in windows 1024px
        // or wider, 70% of the window width from 768px, and below that nothing beyond the 650px minimum.
        let vw: f32 = window_area(window).size.width.into();
        let mut spec = DialogSpec::new().padded(false).min_width(px(650.0)).min_height(px(500.0)).max_height_frac(0.9);
        spec = if vw >= 1024.0 {
            spec.max_width(px(850.0))
        } else if vw >= 768.0 {
            spec.max_width(px(100_000.0)).max_width_frac(0.7)
        } else {
            spec.max_width(px(0.0))
        };
        dialog::open(window, cx, spec.content_view(view));
    }

    fn new(manifest: AddonManifest, window_handle: AnyWindowHandle, cx: &mut Context<Self>) -> Self {
        let rating = cx.new(|cx| RatingModel::new(manifest.name.clone(), manifest.alias.clone(), true, cx));
        let rating_sub = cx.observe(&rating, |_, _, cx| cx.notify());
        let subs = vec![
            cx.observe(&state::queries::release(cx), |_, _, cx| cx.notify()),
            cx.observe(&state::queries::readme(cx), |_, _, cx| cx.notify()),
            cx.observe(&state::catalog(cx), |_, _, cx| cx.notify()),
            cx.observe(&state::addons(cx), |_, _, cx| cx.notify()),
        ];
        let mut this = Self {
            manifest,
            window_handle,
            rating,
            _rating_sub: rating_sub,
            _subs: subs,
            scroll: ScrollHandle::default(),
            show_scroll_top: false,
            expanded: false,
            overflowing: false,
            changelog_open: false,
            previous_recent: false,
        };
        this.start_queries(cx);
        this
    }

    /// Shows another addon (a dependency) in the same dialog.
    fn set_manifest(&mut self, manifest: AddonManifest, cx: &mut Context<Self>) {
        self.rating = cx.new(|cx| RatingModel::new(manifest.name.clone(), manifest.alias.clone(), true, cx));
        self._rating_sub = cx.observe(&self.rating, |_, _, cx| cx.notify());
        self.manifest = manifest;
        self.expanded = false;
        self.overflowing = false;
        self.start_queries(cx);
        cx.notify();
    }

    /// Starts the release, README and catalog lookups of the shown addon. A failed release or dependency lookup is
    /// toasted here, while the addon is still shown, so list hover prefetching stays silent.
    fn start_queries(&mut self, cx: &mut Context<Self>) {
        let manifest = self.manifest.clone();
        let release = state::queries::release(cx).update(cx, |q, cx| q.fetch(manifest.name.clone(), cx));
        if !manifest.repo.is_empty() {
            let key = (manifest.repo.clone(), manifest.branch.clone());
            state::queries::readme(cx).update(cx, |q, cx| q.ensure(key, cx));
        }
        let catalog = (!manifest.dependencies.is_empty()).then(|| catalog::fetch(cx));
        cx.spawn(async move |this, cx| {
            let release_failed = release.await.is_err();
            let catalog = match catalog {
                Some(task) => Some(task.await),
                None => None,
            };
            let _ = this.update(cx, |this, cx| {
                if this.manifest.name != manifest.name {
                    return;
                }
                if release_failed {
                    toast::show(
                        cx,
                        Toast::new("Error", format!("Failed to fetch release information for {}", manifest.name))
                            .icon(icons::TRIANGLE_ALERT),
                    );
                }
                match catalog {
                    None => {}
                    Some(Err(_)) => {
                        toast::show(
                            cx,
                            Toast::new("Error", format!("Failed to resolve dependencies for {}", manifest.name))
                                .icon(icons::TRIANGLE_ALERT),
                        );
                    }
                    Some(Ok(catalog)) => {
                        let installed = installed_names(cx);
                        let errors = resolve_dependencies_with(&manifest, catalog.addons(), &installed).errors;
                        if let Some(first) = errors.first() {
                            log::warn!("Dependency resolution errors: {errors:?}");
                            toast::show(
                                cx,
                                Toast::new("Dependency resolution warning", first.clone()).icon(icons::TRIANGLE_ALERT),
                            );
                        }
                    }
                }
            });
        })
        .detach();
    }

    fn release<'a>(&self, cx: &'a App) -> &'a Load<Release> {
        state::queries::release(cx).read(cx).get(&self.manifest.name)
    }

    /// `Idle` for an addon without a repository.
    fn readme<'a>(&self, cx: &'a App) -> &'a Load<Option<String>> {
        state::queries::readme(cx).read(cx).get(&(self.manifest.repo.clone(), self.manifest.branch.clone()))
    }

    /// The dependency tree against the catalog and the installed addons, `Idle` for an addon without dependencies.
    fn dependencies(&self, cx: &App) -> Load<DependencyResolutionResult> {
        if self.manifest.dependencies.is_empty() {
            return Load::Idle;
        }
        let installed = installed_names(cx);
        catalog::load(cx).as_ref().map(|c| resolve_dependencies_with(&self.manifest, c.addons(), &installed))
    }

    fn installed(&self, cx: &App) -> bool {
        state::addons(cx).read(cx).is_installed(&self.manifest.name)
    }

    /// The install or uninstall running on the shown addon, wherever it was started.
    fn operation<'a>(&self, cx: &'a App) -> Option<&'a Operation> {
        state::addons(cx).read(cx).operation(&self.manifest.name)
    }

    fn is_processing(&self, cx: &App) -> bool {
        self.operation(cx).is_some()
    }

    // -- install / uninstall -------------------------------------------------------------------------------

    /// Installs the latest release with its dependencies.
    fn install(&mut self, cx: &mut Context<Self>) {
        if self.is_processing(cx) {
            return;
        }
        if self.installed(cx) {
            toast::show(cx, Toast::new("Already installed", format!("{} is already installed.", self.manifest.alias)));
            return;
        }
        if self.release(cx).ready().is_none() {
            toast::show(cx, Toast::new("Error", "Cannot install addon without a release.").icon(icons::TRIANGLE_ALERT));
            return;
        }
        let manifest = self.manifest.clone();
        let handle = self.window_handle;
        let install =
            state::addons(cx).update(cx, |store, cx| store.install(manifest.clone(), "latest".to_string(), cx));
        cx.spawn(async move |_, cx| {
            let res = install.await;
            cx.update(|cx| match res {
                Err(e) => {
                    state::log_failure(&format!("Installing {} failed:", manifest.name), &e);
                    let message = install_error_message(&manifest, &e);
                    toast::show(cx, Toast::new("Error", message).icon(icons::TRIANGLE_ALERT));
                }
                Ok(result) => {
                    // the store already refreshed the installed list and the update state
                    if !notify_dependency_result(&result, &format!("Failed to install {}", manifest.alias), cx) {
                        return;
                    }
                    let message = format!("{} was installed successfully.", manifest.alias);
                    toast::show(cx, Toast::new("Addon installed", message));
                    close_dialog(handle, cx);
                }
            })
        })
        .detach();
    }

    /// Uninstalls the addon.
    fn uninstall(&mut self, cx: &mut Context<Self>) {
        if self.is_processing(cx) {
            return;
        }
        let manifest = self.manifest.clone();
        let handle = self.window_handle;
        let uninstall = state::addons(cx).update(cx, |store, cx| store.uninstall(manifest.name.clone(), cx));
        cx.spawn(async move |_, cx| {
            let res = uninstall.await;
            cx.update(|cx| match res {
                Ok(()) => {
                    toast::show(cx, Toast::new("Uninstalled", format!("{} uninstalled successfully", manifest.alias)));
                    close_dialog(handle, cx);
                }
                Err(e) => {
                    log::error!("Uninstalling {} failed: {e:#}", manifest.name);
                    toast::show(
                        cx,
                        Toast::new("Error", format!("Failed to uninstall {}: {}", manifest.alias, error_message(&e)))
                            .icon(icons::TRIANGLE_ALERT),
                    );
                }
            })
        })
        .detach();
    }

    // -- measuring / scrolling -----------------------------------------------------------------------------

    fn set_description_height(&mut self, height: f32, cx: &mut Context<Self>) {
        let overflowing = height > COLLAPSED_MAX_HEIGHT;
        if overflowing != self.overflowing {
            self.overflowing = overflowing;
            cx.notify();
        }
    }

    fn sync_scroll_top(&mut self, cx: &mut Context<Self>) {
        let y: f32 = (-self.scroll.offset().y).into();
        let visible = y > SHOW_SCROLL_TOP_AFTER;
        if visible != self.show_scroll_top {
            self.show_scroll_top = visible;
            cx.notify();
        }
    }
}

/// Names of the installed addons.
fn installed_names(cx: &App) -> HashSet<String> {
    state::addons(cx).read(cx).installed_addons.iter().map(|a| a.name.clone()).collect()
}

fn close_dialog(handle: AnyWindowHandle, cx: &mut App) {
    let _ = handle.update(cx, |_, window, cx| dialog::close(window, cx));
}

// ---------------------------------------------------------------------------------------------------------
// sections
// ---------------------------------------------------------------------------------------------------------

/// A dialog section: a divider on top, padding and an uppercase caption in the primary colour.
fn section(label: impl Into<SharedString>, t: &Tokens) -> gpui_kit::Div {
    div().border_t_1().border_color(alpha(t.border, 0.6)).px(px(24.0)).py(px(20.0)).child(
        div()
            .mb(px(12.0))
            .text_size(px(11.0))
            .line_height(px(16.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(t.primary, 0.8))
            .child(SharedString::from(label.into().to_uppercase())),
    )
}

fn icon_button(
    id: impl Into<ElementId>,
    icon: icons::Lucide,
    tooltip: &'static str,
    url: String,
    t: &Tokens,
) -> impl IntoElement {
    let primary = t.primary;
    let accent = alpha(t.accent, 0.5);
    let id = id.into();
    Tooltip::new(tooltip).id((id.clone(), "tooltip")).side(Side::Top).child(
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .size(px(32.0))
            .rounded(radius::MD)
            .cursor_pointer()
            .text_color(t.muted_foreground)
            .hover(move |s| s.bg(accent).text_color(primary))
            .on_click(move |_, _, cx| cx.open_url(&url))
            .child(Icon::new(icon).size(px(16.0))),
    )
}

impl RemoteAddonDialog {
    fn banner_image(&self, window: &mut Window, cx: &mut Context<Self>) -> (String, Option<Arc<RenderImage>>) {
        let url =
            format!("https://raw.githubusercontent.com/{}/{}/banner.png", self.manifest.repo, self.manifest.branch);
        let loaded = window.use_asset::<ImageAssetLoader>(&Resource::Uri(url.clone().into()), cx);
        (url, loaded.and_then(Result::ok))
    }

    /// Banner, icon, name, author, tags, version and the code / issue links.
    fn header(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        t: &Tokens,
        release: &Load<Release>,
    ) -> AnyElement {
        let manifest = &self.manifest;
        let (banner_url, banner) = self.banner_image(window, cx);
        let has_banner = banner.is_some();
        let icon_url = crate::ui::addon_icon::addon_icon_url(manifest.icon.as_deref(), None, None, false);

        let banner_block = {
            let el = div().relative().w_full().overflow_hidden().h(if has_banner { px(160.0) } else { px(0.0) });
            match banner {
                Some(image) => el
                    .child(
                        img(ImageSource::Render(image)).absolute().inset_0().size_full().object_fit(ObjectFit::Cover),
                    )
                    // fade from transparent into the background over the bottom 96px of the banner
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .h(px(96.0))
                            .flex()
                            .flex_col()
                            .child(div().h(px(48.0)).w_full().bg(linear_gradient(
                                0.0,
                                linear_color_stop(alpha(t.background, 0.7), 0.0),
                                linear_color_stop(alpha(t.background, 0.0), 1.0),
                            )))
                            .child(div().h(px(48.0)).w_full().bg(linear_gradient(
                                0.0,
                                linear_color_stop(t.background, 0.0),
                                linear_color_stop(alpha(t.background, 0.7), 1.0),
                            ))),
                    )
                    .with_animation(
                        ElementId::Name(format!("banner-{banner_url}").into()),
                        Animation::new(Duration::from_millis(300)).with_easing(ease_out_quint()),
                        |el, delta| el.h(px(160.0 * delta)),
                    )
                    .into_any_element(),
                None => el.into_any_element(),
            }
        };

        let icon_wrap = {
            let wrap = div().relative().ml(px(24.0)).w(px(56.0)).rounded(radius::XL).child(
                div()
                    .overflow_hidden()
                    .rounded(radius::XL)
                    .child(AddonIcon::new(&manifest.alias, icon_url).size(AddonIconSize::Lg)),
            );
            if has_banner { wrap.mt(px(-32.0)) } else { wrap.mt(px(24.0)) }
        };

        let tag_chip = |tag: &str| {
            div()
                .rounded_full()
                .border_1()
                .border_color(alpha(t.border, 0.6))
                .bg(alpha(t.muted, 0.6))
                .px(px(10.0))
                .py(px(2.0))
                .text_size(px(11.0))
                .line_height(px(16.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.muted_foreground)
                .child(tag.to_string())
        };

        div()
            .flex_none()
            .child(banner_block)
            .child(icon_wrap)
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(16.0))
                    .px(px(24.0))
                    .pt(px(12.0))
                    .pb(px(16.0))
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_2xl_tw()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(manifest.alias.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(4.0))
                                    .truncate()
                                    .text_sm_tw()
                                    .text_color(t.muted_foreground)
                                    .child(format!("by {}", manifest.author)),
                            )
                            .when(!manifest.tags.is_empty(), |d| {
                                d.child(
                                    div()
                                        .mt(px(10.0))
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(6.0))
                                        .children(manifest.tags.iter().map(|tag| tag_chip(tag))),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(6.0))
                            .children(release.ready().map(|release| {
                                div()
                                    .rounded(radius::MD)
                                    .border_1()
                                    .border_color(alpha(t.primary, 0.2))
                                    .bg(alpha(t.primary, 0.1))
                                    .px(px(10.0))
                                    .py(px(4.0))
                                    .font_mono_tw()
                                    .text_xs_tw()
                                    .text_color(t.primary)
                                    .child(release.tag_name.clone())
                            }))
                            .child(icon_button(
                                "dialog-github",
                                icons::GITHUB,
                                "View code",
                                format!("https://github.com/{}", manifest.repo),
                                t,
                            ))
                            .child(icon_button(
                                "dialog-issue",
                                icons::BUG,
                                "Report issue",
                                format!("https://github.com/{}/issues/new", manifest.repo),
                                t,
                            )),
                    ),
            )
            .into_any_element()
    }

    /// Amber bar with the manifest's warning text, if it has one.
    fn warning(&self) -> Option<AnyElement> {
        let text = self.manifest.warning.as_deref().filter(|w| !w.is_empty())?;
        let orange = tw(Tw::Orange, 300);
        Some(
            div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .border_y_1()
                .border_color(gpui_kit::transparent_black())
                .bg(orange)
                .px(px(24.0))
                .py(px(12.0))
                .text_sm_tw()
                .text_color(gpui_kit::black())
                .shadow(shadow_sm())
                .child(div().mt(px(2.0)).flex_none().child(Icon::new(icons::TRIANGLE_ALERT).size(px(16.0))))
                .child(div().flex_1().min_w_0().child(text.to_string()))
                .into_any_element(),
        )
    }

    /// Row of stats: downloads, liked percentage, version and update date.
    fn stat_bar(&self, t: &Tokens, release: &Load<Release>) -> AnyElement {
        let loading = release.is_pending();
        let release_value = |f: &dyn Fn(&Release) -> String| -> AnyElement {
            if loading {
                return div().flex().justify_center().child(Skeleton::new().h(px(16.0)).w(px(80.0))).into_any_element();
            }
            match release.ready() {
                None => "—".into_any_element(),
                Some(r) => f(r).into_any_element(),
            }
        };
        let mut stats: Vec<(icons::Lucide, AnyElement, &'static str)> =
            vec![(icons::DOWNLOAD, format_number(self.manifest.downloads).into_any_element(), "Downloads")];
        if let Some(pct) = self.manifest.like_percent {
            stats.push((icons::HEART, format!("{pct}%").into_any_element(), "Liked"));
        }
        stats.push((icons::TAG, release_value(&|r| r.tag_name.clone()), "Version"));
        stats.push((
            icons::CALENDAR_DAYS,
            release_value(&|r| r.published_at.map(|t| format_to_local_date(&t, true)).unwrap_or_default()),
            "Updated",
        ));
        let border = alpha(t.border, 0.6);
        div()
            .flex()
            .border_y_1()
            .border_color(border)
            .children(stats.into_iter().enumerate().map(|(i, (icon, value, label))| {
                div()
                    .flex_1()
                    .min_w_0()
                    .px(px(16.0))
                    .py(px(12.0))
                    .text_center()
                    .when(i > 0, |d| d.border_l_1().border_color(border))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .mb(px(6.0))
                            .child(Icon::new(icon).size(px(14.0)).text_color(t.primary)),
                    )
                    .child(div().truncate().text_sm_tw().font_weight(FontWeight::SEMIBOLD).child(value))
                    .child(div().mt(px(2.0)).text_xs_tw().text_color(t.muted_foreground).child(label))
            }))
            .into_any_element()
    }

    /// Description section: the README (or manifest description), collapsed when it is long.
    fn description_section(&mut self, cx: &mut Context<Self>, t: &Tokens, readme: &Load<Option<String>>) -> AnyElement {
        if readme.is_pending() {
            return section("Description", t)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(Skeleton::new().h(px(16.0)).w(gpui_kit::relative(0.6667)))
                        .child(Skeleton::new().h(px(16.0)).w_full())
                        .child(Skeleton::new().h(px(16.0)).w_full())
                        .child(Skeleton::new().h(px(16.0)).w(gpui_kit::relative(0.8))),
                )
                .into_any_element();
        }
        let text = readme_text(readme, &self.manifest);
        let this = cx.entity();
        let measure = canvas(
            move |bounds, _, cx| {
                let height: f32 = bounds.size.height.into();
                this.update(cx, |d, cx| d.set_description_height(height, cx));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0();
        let expanded = self.expanded;
        let overflowing = self.overflowing;
        let background = t.background;
        let manifest = &self.manifest;
        section("Description", t)
            .child(
                div()
                    .relative()
                    .when(!expanded, |d| d.max_h(px(COLLAPSED_MAX_HEIGHT)).overflow_hidden())
                    .child(
                        div()
                            .relative()
                            .text_sm_tw()
                            .child(
                                Readme::new(SharedString::from(format!("dialog-readme-{}", manifest.name)), text)
                                    .repo(manifest.repo.clone())
                                    .branch(manifest.branch.clone()),
                            )
                            .child(measure),
                    )
                    .when(!expanded && overflowing, |d| {
                        d.child(div().absolute().left_0().right_0().bottom_0().h(px(80.0)).bg(linear_gradient(
                            0.0,
                            linear_color_stop(background, 0.0),
                            linear_color_stop(alpha(background, 0.0), 1.0),
                        )))
                    }),
            )
            .when(overflowing, |d| {
                let primary = t.primary;
                d.child(
                    div()
                        .id("description-toggle")
                        .mt(px(8.0))
                        .cursor_pointer()
                        .text_xs_tw()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(primary)
                        .hover(move |s| s.text_color(alpha(primary, 0.8)).text_xs_tw().font_weight(FontWeight::MEDIUM))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.expanded = !this.expanded;
                            cx.notify();
                        }))
                        .child(if expanded { "Show less" } else { "Show more" }),
                )
            })
            .into_any_element()
    }

    /// Changelog section: the latest release notes, open by default when the release is recent.
    fn changelog_section(&mut self, cx: &mut Context<Self>, t: &Tokens, release: &Load<Release>) -> AnyElement {
        let is_recent =
            release.ready().and_then(|r| r.published_at).is_some_and(|t| days_ago(&t) < RECENT_RELEASE_DAYS);
        if is_recent != self.previous_recent {
            self.previous_recent = is_recent;
            self.changelog_open = is_recent;
        }
        let changelog = changelog_text(release);
        if release.is_pending() {
            return section("What's New", t)
                .child(Skeleton::new().h(px(24.0)).w(gpui_kit::relative(0.5)))
                .into_any_element();
        }
        let Some(release) = release.ready() else {
            let (no_notes, load_failed) =
                (changelog == "No change log was provided", changelog == "Error loading change log");
            let text = if changelog.is_empty() { "No release available".to_string() } else { changelog };
            return section("What's New", t)
                .child(div().text_sm_tw().font_weight(FontWeight::MEDIUM).text_color(t.muted_foreground).child(text))
                .when(no_notes, |d| {
                    d.child(
                        div()
                            .mt(px(4.0))
                            .text_xs_tw()
                            .text_color(t.muted_foreground)
                            .child("The author hasn't provided release notes."),
                    )
                })
                .when(load_failed, |d| {
                    d.child(
                        div()
                            .mt(px(4.0))
                            .text_xs_tw()
                            .text_color(t.destructive)
                            .child("Could not fetch release details."),
                    )
                })
                .into_any_element();
        };
        let open = self.changelog_open;
        let this = cx.entity();
        let (muted, primary, foreground, muted_bg) = (t.muted_foreground, t.primary, t.foreground, t.muted);
        let tag = release.tag_name.clone();
        let readme_id = format!("dialog-changelog-{}-{}", self.manifest.name, release.tag_name);
        section("What's New", t)
            .child(
                Collapsible::new("changelog")
                    .open(open)
                    .on_open_change(move |next, _, cx| {
                        this.update(cx, |d, cx| {
                            d.changelog_open = next;
                            cx.notify();
                        })
                    })
                    .trigger_with(move |open| {
                        div()
                            .flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .text_sm_tw()
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("What's new in {tag}"))
                            .child(div().flex_none().text_color(muted).child(
                                Icon::new(if open { icons::CHEVRON_UP } else { icons::CHEVRON_DOWN }).size(px(16.0)),
                            ))
                            .into_any_element()
                    })
                    .trigger_hover(move |s| s.text_color(primary).text_sm_tw().font_weight(FontWeight::MEDIUM))
                    .content_with(move || {
                        div()
                            .mt(px(12.0))
                            .rounded(radius::LG)
                            .bg(alpha(muted_bg, 0.4))
                            .p(px(16.0))
                            .text_sm_tw()
                            .text_color(alpha(foreground, 0.9))
                            .child(Readme::new(SharedString::from(readme_id), changelog))
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }

    /// Chips of the addon's dependencies. Clicking one shows that addon in this dialog.
    fn requires_section(
        &self,
        cx: &mut Context<Self>,
        t: &Tokens,
        deps: &Load<DependencyResolutionResult>,
    ) -> Option<AnyElement> {
        let deps: &Vec<DependencyInfo> = &deps.ready()?.dependencies;
        if deps.is_empty() {
            return None;
        }
        let (border, primary) = (alpha(t.border, 0.6), t.primary);
        let (bg, bg_hover) = (alpha(t.muted, 0.3), alpha(t.muted, 0.6));
        let muted_fg = t.muted_foreground;
        Some(
            section(format!("Requires ({})", deps.len()), t)
                .child(div().flex().flex_wrap().gap(px(8.0)).children(deps.iter().enumerate().map(|(i, d)| {
                    let manifest = d.manifest.clone();
                    div()
                        .id(("dependency", i))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded_full()
                        .border_1()
                        .border_color(border)
                        .bg(bg)
                        .px(px(12.0))
                        .py(px(6.0))
                        .text_sm_tw()
                        .cursor_pointer()
                        .hover(move |s| s.border_color(alpha(primary, 0.4)).bg(bg_hover))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_manifest(manifest.clone(), cx)))
                        .child(
                            div()
                                .flex_none()
                                .size(px(8.0))
                                .rounded_full()
                                .when(d.is_installed, |d| d.bg(primary))
                                .when(!d.is_installed, |d| d.border_1().border_color(alpha(muted_fg, 0.4))),
                        )
                        .child(d.manifest.alias.clone())
                })))
                .into_any_element(),
        )
    }

    /// Link to the author's Ko-fi page, if the manifest has one.
    fn support_section(&self, t: &Tokens) -> Option<AnyElement> {
        let kofi = self.manifest.kofi.clone().filter(|k| !k.is_empty())?;
        Some(
            section("Support the Author", t)
                .child(
                    div().flex().child(
                        Button::new("kofi")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .has_icon(true)
                            .on_click(move |_, _, cx| cx.open_url(&format!("https://ko-fi.com/{kofi}")))
                            .child(Icon::new(icons::HEART).size(px(16.0)).text_color(t.primary))
                            .child(format!("Buy {} a coffee", self.manifest.author)),
                    ),
                )
                .into_any_element(),
        )
    }

    /// Bottom bar: rating buttons and the install / uninstall button, which shows progress while installing.
    fn action_bar(&self, cx: &mut Context<Self>, t: &Tokens, release: &Load<Release>, installed: bool) -> AnyElement {
        let rating = self.rating.read(cx);
        let rating_disabled = rating.busy(cx);
        let rating_value = i32::from(rating.rating(cx));
        let model = self.rating.clone();
        let processing = self.is_processing(cx);
        let loading_release = release.is_pending();
        let has_release = release.ready().is_some();
        let progress =
            self.operation(cx).and_then(|op| op.progress.as_ref()).and_then(|p| progress_label(&self.manifest.name, p));

        let label: AnyElement = if installed {
            div()
                .flex()
                .items_center()
                .child(div().mr(px(8.0)).child(Icon::new(icons::TRASH_2).size(px(16.0))))
                .child("Uninstall")
                .into_any_element()
        } else if loading_release {
            div()
                .flex()
                .items_center()
                .child(div().mr(px(8.0)).child(cam_ui::components::spinner()))
                .child("Checking...")
                .into_any_element()
        } else if !has_release {
            "Not Available".into_any_element()
        } else if processing {
            div()
                .flex()
                .items_center()
                .child(div().mr(px(8.0)).child(cam_ui::components::spinner()))
                .child("Installing...")
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .child(div().mr(px(8.0)).child(Icon::new(icons::DOWNLOAD).size(px(16.0))))
                .child("Install")
                .into_any_element()
        };

        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .border_t_1()
            .border_color(t.border)
            .bg(alpha(t.background, 0.8))
            .p(px(16.0))
            .child(
                div().flex().items_center().gap(px(4.0)).child(
                    RatingButtons::new("dialog-rating")
                        .rating(rating_value)
                        .disabled(rating_disabled)
                        .on_rate(move |value, _window, cx| model.update(cx, |m, cx| m.rate(value, cx))),
                ),
            )
            .child(match progress {
                // Same size as the install button. Fills with the download, then with the extracted files.
                Some((label, fraction)) => div()
                    .relative()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .h(px(36.0))
                    .min_w(px(150.0))
                    .px(px(12.0))
                    .overflow_hidden()
                    .rounded(radius::MD)
                    .border_1()
                    .border_color(alpha(t.primary, 0.4))
                    .bg(alpha(t.primary, 0.15))
                    .text_sm_tw()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.primary)
                    // gpui clips to a plain rectangle, so the fill rounds its own corners to stay inside the border.
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left_0()
                            .w(relative(fraction))
                            .rounded(radius::MD - px(1.0))
                            .bg(alpha(t.primary, 0.45)),
                    )
                    .child(div().relative().whitespace_nowrap().child(SharedString::from(label)))
                    .into_any_element(),
                None => Button::new("dialog-install")
                    .variant(if installed { ButtonVariant::Destructive } else { ButtonVariant::Default })
                    .has_icon(true)
                    .min_w(px(120.0))
                    .disabled((!installed && (!has_release || loading_release)) || processing)
                    .on_click(cx.listener(
                        move |this, _, _, cx| {
                            if installed { this.uninstall(cx) } else { this.install(cx) }
                        },
                    ))
                    .child(label)
                    .into_any_element(),
            })
            .into_any_element()
    }

    /// Round button that scrolls the dialog back to the top, shown once it is scrolled down.
    fn scroll_to_top(&self, cx: &mut Context<Self>, t: &Tokens) -> Option<AnyElement> {
        if !self.show_scroll_top {
            return None;
        }
        let handle = self.scroll.clone();
        let (accent, border) = (alpha(t.accent, 0.5), t.input);
        Some(
            div()
                .id("scroll-to-top")
                .absolute()
                .right(px(16.0))
                .bottom(px(80.0))
                .flex()
                .items_center()
                .justify_center()
                .size(px(36.0))
                .rounded_full()
                .border_1()
                .border_color(border)
                .bg(alpha(t.background, 0.7))
                .shadow(shadow_md())
                .cursor_pointer()
                .hover(move |s| s.bg(accent))
                .on_click(cx.listener(move |this, _, _, cx| {
                    handle.set_offset(point(px(0.0), px(0.0)));
                    this.sync_scroll_top(cx);
                }))
                .child(Icon::new(icons::ARROW_UP).size(px(16.0)))
                .into_any_element(),
        )
    }
}

impl Render for RemoteAddonDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.tokens();
        let (release, readme) = (self.release(cx).clone(), self.readme(cx).clone());
        let (deps, installed) = (self.dependencies(cx), self.installed(cx));
        let header = self.header(window, cx, &t, &release);
        let warning = self.warning();
        let stat_bar = self.stat_bar(&t, &release);
        let description = self.description_section(cx, &t, &readme);
        let changelog = self.changelog_section(cx, &t, &release);
        let requires = self.requires_section(cx, &t, &deps);
        let support = self.support_section(&t);
        let action_bar = self.action_bar(cx, &t, &release, installed);
        let scroll_top = self.scroll_to_top(cx, &t);

        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .text_color(t.foreground)
            // keep the scroll-to-top button in sync with wheel scrolling and scrollbar drags
            .on_scroll(cx.listener(|this, _, _, cx| this.sync_scroll_top(cx)))
            .on_mouse_move(cx.listener(|this, _, _, cx| this.sync_scroll_top(cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.sync_scroll_top(cx)))
            .child(
                // Not `ScrollArea`: its `size_full` viewport gets no height of its own in this auto-height card (min 500px,
                // max 90% of the window), so it grew with the content and never scrolled. This box takes the content's
                // height, grows to the card's minimum and shrinks to its maximum, the viewport inside it scrolls.
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_auto()
                    .min_h_0()
                    .child(
                        div()
                            .id("addon-dialog-scroll")
                            .flex_auto()
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(header)
                                    .children(warning)
                                    .child(stat_bar)
                                    .child(description)
                                    .child(changelog)
                                    .children(requires)
                                    .children(support),
                            ),
                    )
                    .child(scrollbar_overlay("addon-dialog-scrollbar", &self.scroll, ScrollbarAxis::Vertical)),
            )
            .child(action_bar)
            .children(scroll_top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(body: &str) -> Release {
        Release { body: body.into(), tag_name: "v1".into(), ..Default::default() }
    }

    #[test]
    fn changelog_text_cases() {
        assert_eq!(changelog_text(&Load::Pending), "");
        assert_eq!(changelog_text(&Load::Failed(anyhow::anyhow!("x").into())), "Error loading change log");
        assert_eq!(changelog_text(&Load::Ready(release(""))), "No change log was provided");
        assert_eq!(changelog_text(&Load::Ready(release("notes"))), "notes");
    }

    #[test]
    fn readme_falls_back_to_description() {
        let mut m = AddonManifest::default();
        assert_eq!(readme_text(&Load::Ready(None), &m), "No description provided.");
        m.description = "desc".into();
        assert_eq!(readme_text(&Load::Ready(None), &m), "desc");
        assert_eq!(readme_text(&Load::Ready(Some(String::new())), &m), "desc");
        assert_eq!(readme_text(&Load::Failed(anyhow::anyhow!("e").into()), &m), "desc");
        assert_eq!(readme_text(&Load::Ready(Some("# hi".into())), &m), "# hi");
    }

    #[test]
    fn install_errors_pick_the_no_release_message() {
        let m = AddonManifest { name: "n".into(), alias: "A".into(), ..Default::default() };
        let not_found = anyhow::Error::new(cam_core::api::ApiError::NotFound).context("resolving dependencies");
        assert_eq!(install_error_message(&m, &not_found), "No release found for n");
        assert_eq!(install_error_message(&m, &anyhow::anyhow!("boom")), "Failed to install A: Boom");
    }
}
