use base64::Engine;
use freya::prelude::*;
use hypereffect::{Component as HeComponent, Value as Json, client::ResolvedTree, json};

use crate::{
    SharedHost,
    skin::{
        ButtonProps, ButtonVariant, CardProps, CheckboxProps, InputProps, ProgressProps,
        SharedSkin, Skin, StockSkin,
    },
};

/// The measures the view lays out with. The default is what the view always drew; change what
/// does not fit the app around it (compact on a phone, rounder or squarer).
#[derive(Clone, Copy, PartialEq)]
pub struct Style {
    /// Between the pieces of one control, and a `gap` of `small`.
    pub space_small: f32,
    /// Between elements (a `gap` of `medium`, the default), and inside cards.
    pub space_medium: f32,
    /// A `gap` of `large`, and the padding of an empty list.
    pub space_large: f32,
    /// Corner radius of cards and boxes.
    pub radius: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            space_small: 6.,
            space_medium: 12.,
            space_large: 20.,
            radius: 12.,
        }
    }
}

/// Which images a server may make the renderer show.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ImagePolicy {
    /// Images written into the page itself (`data:` URIs), which fetch nothing. On by default.
    pub inline: bool,
    /// Files of this device (`file://` and absolute paths). Off by default: a server that does
    /// not own the device has no business naming its files.
    pub files: bool,
}

impl ImagePolicy {
    /// Inline images only.
    pub const fn inline() -> Self {
        Self {
            inline: true,
            files: false,
        }
    }

    /// No images: a placeholder with the alt text.
    pub const fn none() -> Self {
        Self {
            inline: false,
            files: false,
        }
    }
}

/// The interface of a [`HyperHost`](crate::HyperHost), drawn with Freya's components in the
/// colors of the current theme.
#[derive(Clone)]
pub struct HyperView {
    host: SharedHost,
    rev: State<u64>,
    images: ImagePolicy,
    style: Style,
    skin: Option<SharedSkin>,
    /// The host's [`revision`](crate::HyperHost::revision) when the view was made.
    revision: u64,
}

impl HyperView {
    /// `rev` is bumped by the view after the user acts; bump it too when the host changes from
    /// outside, so the view redraws.
    pub fn new(host: SharedHost, rev: State<u64>) -> Self {
        let revision = host.try_borrow().map_or(0, |host| host.revision());
        Self {
            host,
            rev,
            images: ImagePolicy::inline(),
            style: Style::default(),
            skin: None,
            revision,
        }
    }

    pub fn images(mut self, images: ImagePolicy) -> Self {
        self.images = images;
        self
    }

    /// Draw the controls with `skin` instead of Freya's own components. Keep the `Rc` and hand
    /// out clones: a view with another skin than before redraws everything.
    pub fn skin(mut self, skin: SharedSkin) -> Self {
        self.skin = Some(skin);
        self
    }

    /// Lay out with other spacing and corner radius than the default.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

fn same_skin(a: &Option<SharedSkin>, b: &Option<SharedSkin>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl PartialEq for HyperView {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.host, &other.host)
            && self.rev == other.rev
            && self.images == other.images
            && self.style == other.style
            && same_skin(&self.skin, &other.skin)
            && self.revision == other.revision
    }
}

impl Component for HyperView {
    fn render(&self) -> impl IntoElement {
        let _ = self.rev.read();
        let (tree, error) = {
            let host = self.host.borrow();
            (host.tree(), host.error())
        };
        let colors = palette();
        let host = self.host.clone();
        let rev = self.rev;
        let images = self.images;
        let style = self.style;
        let revision = self.revision;

        rect()
            .vertical()
            .width(Size::fill())
            .spacing(style.space_medium)
            .map(error, |el, error| {
                el.child(
                    rect()
                        .width(Size::fill())
                        .padding(style.space_medium)
                        .corner_radius(style.radius)
                        .background(colors.error.with_a(40))
                        .child(txt(error, 14., colors.error, FontWeight::MEDIUM)),
                )
            })
            .children(tree.into_iter().map(|node| {
                NodeView {
                    node,
                    host: host.clone(),
                    rev,
                    images,
                    style,
                    skin: self.skin.clone(),
                    revision,
                }
                .into_element()
            }))
    }
}

#[derive(Clone)]
struct NodeView {
    node: ResolvedTree,
    host: SharedHost,
    rev: State<u64>,
    images: ImagePolicy,
    style: Style,
    skin: Option<SharedSkin>,
    revision: u64,
}

impl PartialEq for NodeView {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && std::rc::Rc::ptr_eq(&self.host, &other.host)
            && self.rev == other.rev
            && self.images == other.images
            && self.style == other.style
            && same_skin(&self.skin, &other.skin)
            && self.revision == other.revision
    }
}

impl NodeView {
    fn skin(&self) -> &dyn Skin {
        match &self.skin {
            Some(skin) => &**skin,
            None => &StockSkin,
        }
    }

    fn child(&self, node: ResolvedTree) -> Element {
        NodeView {
            node,
            host: self.host.clone(),
            rev: self.rev,
            images: self.images,
            style: self.style,
            skin: self.skin.clone(),
            revision: self.revision,
        }
        .into_element()
    }

    /// Run `f` on the host, then redraw. Skipped if the host is already borrowed (an action
    /// triggered while the host is being read).
    fn with_host(host: &SharedHost, rev: State<u64>, f: impl FnOnce(&mut dyn crate::HyperHost)) {
        if let Ok(mut host) = host.try_borrow_mut() {
            f(&mut *host);
        }
        bump(rev);
    }
}

impl Component for NodeView {
    fn render(&self) -> impl IntoElement {
        let _ = self.rev.read();
        let mut selected = use_state(String::new);
        let mut local = use_state(String::new);
        let node = &self.node;
        let colors = palette();
        if !attr_bool(node, "visible", true) {
            return rect().into_element();
        }

        let st = self.style;
        let host = self.host.clone();
        let rev = self.rev;
        let in_row = node.component == HeComponent::Row && !attr_bool(node, "wrap", false);
        let children: Vec<Element> = node
            .children
            .iter()
            .cloned()
            .map(|child| {
                let grows = in_row && grows_in_row(&child);
                let child = self.child(child);
                if grows {
                    // Text and fields share what the buttons leave of the row.
                    rect()
                        .width(Size::flex(1.))
                        .min_width(Size::px(96.))
                        .child(child)
                        .into_element()
                } else {
                    child
                }
            })
            .collect();

        let id = node.id.clone();
        match node.component {
            HeComponent::Column => rect()
                .vertical()
                .width(Size::fill())
                .spacing(gap(node, st))
                .cross_align(align(node, "align"))
                .children(children)
                .into_element(),
            HeComponent::Row => {
                // A field has a label above it, so a row with one lines its children up on the
                // bottom edge (the field and the button beside it); other rows center theirs.
                let form = node.children.iter().any(|c| {
                    matches!(
                        c.component,
                        HeComponent::Input | HeComponent::Select | HeComponent::Slider
                    )
                });
                let cross = match attr_str(node, "align").as_str() {
                    "center" if form => Alignment::End,
                    _ => align(node, "align"),
                };
                let content = if attr_bool(node, "wrap", false) {
                    Content::wrap_spacing(gap(node, st))
                } else {
                    Content::flex()
                };
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(content)
                    .spacing(gap(node, st))
                    .main_align(justify(node))
                    .cross_align(cross)
                    .children(children)
                    .into_element()
            }
            HeComponent::Card => self.skin().card(CardProps {
                title: Some(attr_str(node, "title")).filter(|title| !title.is_empty()),
                children,
                style: st,
            }),
            HeComponent::List => {
                let empty = attr_str(node, "empty");
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(st.space_small)
                    .maybe(children.is_empty() && !empty.is_empty(), |el| {
                        el.child(txt(empty, 14., colors.text_secondary, FontWeight::NORMAL))
                    })
                    .children(children)
                    .into_element()
            }
            HeComponent::Tabs => {
                let tabs: Vec<&ResolvedTree> = node
                    .children
                    .iter()
                    .filter(|t| t.component == HeComponent::Tab && attr_bool(t, "visible", true))
                    .collect();
                if tabs.is_empty() {
                    return rect().into_element();
                }
                let bind = attr_str(node, "bind");
                let bound = if bind.is_empty() {
                    None
                } else {
                    host.borrow()
                        .value(&bind)
                        .and_then(|v| v.as_str().map(str::to_owned))
                };
                let current = bound
                    .filter(|id| tabs.iter().any(|t| &t.id == id))
                    .or_else(|| {
                        let local = selected.read().clone();
                        tabs.iter().any(|t| t.id == local).then_some(local)
                    })
                    .unwrap_or_else(|| tabs[0].id.clone());

                let mut bar = rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::wrap_spacing(st.space_small))
                    .spacing(st.space_small);
                for tab in &tabs {
                    let pickup = tab.id.clone();
                    let host = host.clone();
                    let bind = bind.clone();
                    let tabs_id = id.clone();
                    let on = move |_: Event<PressEventData>| {
                        Self::with_host(&host, rev, |host| {
                            if !bind.is_empty() {
                                host.write(&bind, Json::String(pickup.clone()));
                                host.act(&tabs_id, "onChange");
                            }
                        });
                        selected.set(pickup.clone());
                    };
                    bar = bar.child(choice(attr_str(tab, "label"), tab.id == current, true, on));
                }
                let body = tabs
                    .iter()
                    .find(|t| t.id == current)
                    .map(|t| self.child((*t).clone()))
                    .unwrap_or_else(|| rect().into_element());
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(st.space_medium)
                    .child(bar)
                    .child(body)
                    .into_element()
            }
            HeComponent::Tab => rect()
                .vertical()
                .width(Size::fill())
                .spacing(gap(node, st))
                .children(children)
                .into_element(),
            HeComponent::Dialog => {
                if !attr_bool(node, "open", true) {
                    return rect().into_element();
                }
                let dismiss = node
                    .attrs
                    .get("onDismiss")
                    .is_some_and(|handler| !handler.is_null());
                let host = host.clone();
                let dismiss_id = id.clone();
                rect()
                    .layer(Layer::Overlay)
                    .position(Position::new_global().top(0.).left(0.))
                    .width(Size::window_percent(100.))
                    .height(Size::window_percent(100.))
                    .center()
                    .background(Color::BLACK.with_a(120))
                    .child(
                        rect()
                            .vertical()
                            .width(Size::px(440.))
                            .max_width(Size::window_percent(92.))
                            .padding(st.space_large)
                            .spacing(st.space_medium)
                            .corner_radius(st.radius)
                            .background(colors.surface_secondary)
                            .maybe(!attr_str(node, "title").is_empty(), |el| {
                                el.child(txt(
                                    attr_str(node, "title"),
                                    18.,
                                    colors.text_primary,
                                    FontWeight::BOLD,
                                ))
                            })
                            .children(children)
                            .maybe(dismiss, |el| {
                                el.child(
                                    Button::new()
                                        .outline()
                                        .cursor(CursorIcon::Pointer)
                                        .on_press(move |_| {
                                            Self::with_host(&host, rev, |host| {
                                                host.act(&dismiss_id, "onDismiss")
                                            })
                                        })
                                        .child("Close"),
                                )
                            }),
                    )
                    .into_element()
            }
            HeComponent::Text => {
                let (size, weight, color) = match attr_str(node, "variant").as_str() {
                    "h1" => (24., FontWeight::BOLD, colors.text_primary),
                    "h2" => (18., FontWeight::BOLD, colors.text_primary),
                    "h3" => (16., FontWeight::BOLD, colors.text_primary),
                    "caption" => (12., FontWeight::NORMAL, colors.text_secondary),
                    "code" => (13., FontWeight::MEDIUM, colors.text_primary),
                    _ => (15., FontWeight::NORMAL, colors.text_primary),
                };
                txt(attr_str(node, "text"), size, color, weight)
                    .width(Size::fill())
                    .into_element()
            }
            HeComponent::Notice => {
                let color = tone_color(attr_str(node, "variant").as_str(), &colors);
                rect()
                    .width(Size::fill())
                    .corner_radius(st.radius)
                    .background(color.with_a(40))
                    .padding(st.space_medium)
                    .spacing(2.)
                    .maybe(!attr_str(node, "title").is_empty(), |el| {
                        el.child(txt(
                            attr_str(node, "title"),
                            15.,
                            color,
                            FontWeight::SEMI_BOLD,
                        ))
                    })
                    .child(txt(attr_str(node, "text"), 13., color, FontWeight::NORMAL))
                    .into_element()
            }
            HeComponent::Badge => {
                let color = tone_color(attr_str(node, "variant").as_str(), &colors);
                rect()
                    .padding((3., 8.))
                    .corner_radius(99.)
                    .background(color.with_a(40))
                    .child(txt(attr_str(node, "text"), 11., color, FontWeight::BOLD))
                    .into_element()
            }
            HeComponent::Image => {
                let alt = attr_str(node, "alt");
                let src = attr_str(node, "src");
                if let Some(bytes) = image_bytes(&src, self.images) {
                    let (width, height) = match attr_str(node, "variant").as_str() {
                        "avatar" => (Size::px(48.), Size::px(48.)),
                        "thumbnail" => (Size::px(120.), Size::px(120.)),
                        "full" => (Size::fill(), Size::px(300.)),
                        _ => (Size::fill(), Size::px(160.)),
                    };
                    return ImageViewer::new((src, bytes::Bytes::from(bytes)))
                        .width(width)
                        .height(height)
                        .into_element();
                }
                rect()
                    .width(Size::fill())
                    .height(Size::px(120.))
                    .corner_radius(st.radius)
                    .background(colors.surface_tertiary)
                    .center()
                    .child(txt(
                        // A remote source is shown as what it is called, never fetched.
                        alt,
                        13.,
                        colors.text_secondary,
                        FontWeight::NORMAL,
                    ))
                    .into_element()
            }
            HeComponent::Divider => rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(colors.border)
                .into_element(),
            HeComponent::Progress => {
                let value = node.attrs.get("value").and_then(Json::as_f64).unwrap_or(0.);
                self.skin().progress(ProgressProps {
                    label: Some(attr_str(node, "label")).filter(|label| !label.is_empty()),
                    fraction: value as f32,
                    style: st,
                })
            }
            HeComponent::Button => {
                let enabled = !attr_bool(node, "disabled", false);
                let host = host.clone();
                let button_id = id.clone();
                let on = move |()| {
                    Self::with_host(&host, rev, |host| host.act(&button_id, "onPress"));
                };
                let label = attr_str(node, "label");
                let variant = match attr_str(node, "variant").as_str() {
                    "primary" | "success" => ButtonVariant::Primary,
                    "failure" => ButtonVariant::Danger,
                    "plain" => ButtonVariant::Plain,
                    _ => ButtonVariant::Secondary,
                };
                self.skin().button(ButtonProps {
                    label,
                    variant,
                    enabled,
                    on_press: EventHandler::new(on),
                })
            }
            HeComponent::Input => {
                let bind = attr_str(node, "bind");
                let path = (!bind.is_empty()).then_some(bind);
                let number = attr_str(node, "variant") == "number";
                let multiline = attr_str(node, "variant") == "multiline";
                let bound = path.as_deref().and_then(|path| host.borrow().value(path));
                // A number being typed ("1.", "-", "1e") is not yet the number it will become, so
                // the raw text stays while it still means the bound value.
                let value = match bound {
                    Some(value) => {
                        let typed = local.read().clone();
                        if number && typed_number(&typed, &value) {
                            typed
                        } else {
                            json_text(&value)
                        }
                    }
                    None => local.read().clone(),
                };
                let enabled = !attr_bool(node, "disabled", false);
                let input_id = id.clone();

                let h = host.clone();
                let p = path.clone();
                let change_id = input_id.clone();
                let on_change = move |text: String| {
                    local.set(text.clone());
                    if let Some(path) = &p {
                        let value = if number {
                            number_json(&text)
                        } else {
                            Json::String(text)
                        };
                        Self::with_host(&h, rev, |host| {
                            host.write(path, value);
                            host.act(&change_id, "onChange");
                        });
                    } else {
                        bump(rev);
                    }
                };
                let h = host.clone();
                let p = path;
                let submit_id = input_id;
                let on_submit = move |text: String| {
                    if let Some(path) = &p {
                        let value = if number {
                            number_json(&text)
                        } else {
                            Json::String(text)
                        };
                        Self::with_host(&h, rev, |host| {
                            host.write(path, value);
                            host.act(&submit_id, "onSubmit");
                        });
                    }
                };
                labeled(
                    node,
                    self.skin().input(InputProps {
                        value,
                        placeholder: attr_str(node, "placeholder"),
                        multiline,
                        enabled,
                        on_change: EventHandler::new(on_change),
                        on_submit: EventHandler::new(on_submit),
                    }),
                )
            }
            HeComponent::Checkbox => {
                let bind = attr_str(node, "bind");
                let value = host
                    .borrow()
                    .value(&bind)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let label = attr_str(node, "label");
                let enabled = !attr_bool(node, "disabled", false);
                let host = host.clone();
                let box_id = id.clone();
                let on = move |()| {
                    if !bind.is_empty() {
                        Self::with_host(&host, rev, |host| {
                            host.write(&bind, Json::Bool(!value));
                            host.act(&box_id, "onChange");
                        });
                    }
                };
                self.skin().checkbox(CheckboxProps {
                    label,
                    checked: value,
                    enabled,
                    on_toggle: EventHandler::new(on),
                    style: st,
                })
            }
            HeComponent::Select => {
                let bind = attr_str(node, "bind");
                let options = select_options(node);
                let current = host
                    .borrow()
                    .value(&bind)
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                let control: Element = if attr_str(node, "variant") == "dropdown" {
                    let shown = options
                        .iter()
                        .find(|(value, _)| *value == current)
                        .map(|(_, label)| label.clone())
                        .unwrap_or_default();
                    Select::new()
                        .selected_item(txt(shown, 15., colors.text_primary, FontWeight::NORMAL))
                        .children(options.into_iter().map(|(value, label)| {
                            let host = host.clone();
                            let bind = bind.clone();
                            let select_id = id.clone();
                            MenuItem::new()
                                .on_press(move |_| {
                                    Self::with_host(&host, rev, |host| {
                                        host.write(&bind, Json::String(value.clone()));
                                        host.act(&select_id, "onChange");
                                    });
                                })
                                .child(txt(label, 15., colors.text_primary, FontWeight::NORMAL))
                                .into_element()
                        }))
                        .into_element()
                } else {
                    let mut row = rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::wrap_spacing(st.space_small))
                        .spacing(st.space_small);
                    for (value, label) in options {
                        let is_current = value == current;
                        let host = host.clone();
                        let bind = bind.clone();
                        let select_id = id.clone();
                        row = row.child(choice(label, is_current, true, move |_| {
                            Self::with_host(&host, rev, |host| {
                                host.write(&bind, Json::String(value.clone()));
                                host.act(&select_id, "onChange");
                            });
                        }));
                    }
                    row.into_element()
                };
                labeled(node, control)
            }
            HeComponent::Slider => {
                let bind = attr_str(node, "bind");
                let min = attr_num(node, "min").unwrap_or(0.);
                let max = attr_num(node, "max").unwrap_or(100.);
                let step = attr_num(node, "step").unwrap_or(1.);
                let value = host
                    .borrow()
                    .value(&bind)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(min);
                let percent = if max > min {
                    ((value - min) / (max - min) * 100.).clamp(0., 100.)
                } else {
                    0.
                };
                let enabled = !attr_bool(node, "disabled", false);
                let host = host.clone();
                let slider_id = id.clone();
                let step_percent = if max > min {
                    step / (max - min) * 100.
                } else {
                    1.
                };
                let control = Slider::new(move |v: f64| {
                    let actual = min + (max - min) * v / 100.;
                    Self::with_host(&host, rev, |host| {
                        host.write(&bind, json!(actual));
                        host.act(&slider_id, "onChange");
                    });
                })
                .value(percent)
                .enabled(enabled)
                .step(step_percent)
                .size(Size::fill())
                .into_element();
                labeled(node, control)
            }
        }
    }
}

pub(crate) fn palette() -> ColorsSheet {
    get_theme_or_default().read().colors.clone()
}

pub(crate) fn txt(text: impl Into<String>, size: f32, color: Color, weight: FontWeight) -> Label {
    label()
        .font_size(size)
        .font_weight(weight)
        .color(color)
        .text(text.into())
}

/// A button that is highlighted while it is the current choice of a set.
fn choice(
    text: String,
    current: bool,
    enabled: bool,
    on: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let button = Button::new()
        .enabled(enabled)
        .cursor(CursorIcon::Pointer)
        .on_press(on);
    if current {
        button.filled().child(text).into_element()
    } else {
        button.outline().child(text).into_element()
    }
}

/// Whether a child of a row takes the row's spare width instead of its own size.
fn grows_in_row(node: &ResolvedTree) -> bool {
    attr_bool(node, "visible", true)
        && matches!(
            node.component,
            HeComponent::Text
                | HeComponent::Input
                | HeComponent::Select
                | HeComponent::Slider
                | HeComponent::Progress
                | HeComponent::Checkbox
                | HeComponent::Column
                | HeComponent::Card
                | HeComponent::List
        )
}

/// The bytes of an image the policy lets through.
fn image_bytes(src: &str, policy: ImagePolicy) -> Option<Vec<u8>> {
    if let Some(rest) = src.strip_prefix("data:") {
        if !policy.inline {
            return None;
        }
        let (meta, data) = rest.split_once(',')?;
        if !meta.ends_with(";base64") {
            return None;
        }
        return base64::engine::general_purpose::STANDARD.decode(data).ok();
    }
    let path = src.strip_prefix("file://").unwrap_or(src);
    if policy.files && path.starts_with('/') {
        return std::fs::read(path).ok();
    }
    None
}

fn labeled(node: &ResolvedTree, control: Element) -> Element {
    let colors = palette();
    let label = attr_str(node, "label");
    let error = attr_str(node, "error");
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(2.)
        .maybe(!label.is_empty(), |el| {
            el.child(txt(label, 13., colors.text_secondary, FontWeight::MEDIUM))
        })
        .child(control)
        .maybe(!error.is_empty(), |el| {
            el.child(txt(error, 12., colors.error, FontWeight::NORMAL))
        })
        .into_element()
}

fn bump(mut rev: State<u64>) {
    let next = *rev.read() + 1;
    rev.set(next);
}

fn attr_str(node: &ResolvedTree, name: &str) -> String {
    node.attrs
        .get(name)
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_owned()
}

fn attr_bool(node: &ResolvedTree, name: &str, default: bool) -> bool {
    node.attrs
        .get(name)
        .and_then(Json::as_bool)
        .unwrap_or(default)
}

fn attr_num(node: &ResolvedTree, name: &str) -> Option<f64> {
    node.attrs.get(name).and_then(Json::as_f64)
}

fn gap(node: &ResolvedTree, st: Style) -> f32 {
    match attr_str(node, "gap").as_str() {
        "none" => 0.,
        "small" => st.space_small,
        "large" => st.space_large,
        _ => st.space_medium,
    }
}

fn align(node: &ResolvedTree, name: &str) -> Alignment {
    match attr_str(node, name).as_str() {
        "center" => Alignment::Center,
        "end" => Alignment::End,
        _ => Alignment::Start,
    }
}

fn justify(node: &ResolvedTree) -> Alignment {
    match attr_str(node, "justify").as_str() {
        "center" => Alignment::Center,
        "end" => Alignment::End,
        "between" => Alignment::SpaceBetween,
        _ => Alignment::Start,
    }
}

fn tone_color(variant: &str, colors: &ColorsSheet) -> Color {
    match variant {
        "success" => colors.success,
        "warning" => colors.warning,
        "failure" => colors.error,
        "info" => colors.info,
        _ => colors.text_secondary,
    }
}

fn select_options(node: &ResolvedTree) -> Vec<(String, String)> {
    node.attrs
        .get("options")
        .and_then(Json::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    Some((
                        option.get("value")?.as_str()?.to_owned(),
                        option.get("label")?.as_str()?.to_owned(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What a number input holds for the text typed in it: a number, or null when it is not one.
fn number_json(text: &str) -> Json {
    match text.trim().parse::<f64>() {
        Ok(n) if n.is_finite() => match text.trim().parse::<i64>() {
            Ok(i) => json!(i),
            Err(_) => json!(n),
        },
        _ => Json::Null,
    }
}

/// Whether `typed` is still a way of writing the bound `value`.
fn typed_number(typed: &str, value: &Json) -> bool {
    match (number_json(typed).as_f64(), value.as_f64()) {
        (Some(typed), Some(value)) => typed == value,
        (None, None) => number_json(typed).is_null() && value.is_null(),
        _ => false,
    }
}

fn json_text(value: &Json) -> String {
    match value {
        Json::String(s) => s.clone(),
        Json::Number(n) => n.to_string(),
        Json::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
