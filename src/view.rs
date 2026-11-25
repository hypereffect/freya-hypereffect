use std::{cell::RefCell, rc::Rc};

use base64::Engine;
use freya::prelude::*;
use freya_components::http::fetch;
use hypereffect::{Component as HeComponent, Value as Json, client::ResolvedTree};

use crate::{
    SharedHost,
    controls::{Controls, Input as UserInput, slider_value},
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImagePolicy {
    /// Images written into the page itself (`data:` URIs), which fetch nothing. On by default.
    pub inline: bool,
    /// Files of this device (`file://` and absolute paths). Off by default: a server that does
    /// not own the device has no business naming its files.
    pub files: bool,
    /// Images fetched from `http:` and `https:` URLs. On by default.
    pub remote: bool,
}

impl Default for ImagePolicy {
    /// Inline and remote images, not files of the device.
    fn default() -> Self {
        Self {
            inline: true,
            files: false,
            remote: true,
        }
    }
}

impl ImagePolicy {
    /// Every source: inline, files and remote images.
    pub const fn all() -> Self {
        Self {
            inline: true,
            files: true,
            remote: true,
        }
    }

    /// Inline images only.
    pub const fn inline() -> Self {
        Self {
            inline: true,
            files: false,
            remote: false,
        }
    }

    /// No images: a placeholder with the alt text.
    pub const fn none() -> Self {
        Self {
            inline: false,
            files: false,
            remote: false,
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
            images: ImagePolicy::default(),
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
        let (tree, error, closed) = {
            let host = self.host.borrow();
            (host.tree(), host.error(), host.is_closed())
        };
        let controls = use_hook(|| Rc::new(RefCell::new(Controls::default())));
        if let Ok(mut controls) = controls.try_borrow_mut() {
            controls.retain(&tree);
        }
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
            .children(tree.into_iter().filter(visible).map(|node| {
                NodeView {
                    node,
                    host: host.clone(),
                    controls: controls.clone(),
                    rev,
                    images,
                    style,
                    skin: self.skin.clone(),
                    revision,
                    closed,
                }
                .into_element()
            }))
    }
}

#[derive(Clone)]
struct NodeView {
    node: ResolvedTree,
    host: SharedHost,
    controls: Rc<RefCell<Controls>>,
    rev: State<u64>,
    images: ImagePolicy,
    style: Style,
    skin: Option<SharedSkin>,
    revision: u64,
    closed: bool,
}

impl PartialEq for NodeView {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && Rc::ptr_eq(&self.host, &other.host)
            && Rc::ptr_eq(&self.controls, &other.controls)
            && self.rev == other.rev
            && self.images == other.images
            && self.style == other.style
            && same_skin(&self.skin, &other.skin)
            && self.revision == other.revision
            && self.closed == other.closed
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
            controls: self.controls.clone(),
            rev: self.rev,
            images: self.images,
            style: self.style,
            skin: self.skin.clone(),
            revision: self.revision,
            closed: self.closed,
        }
        .into_element()
    }

    /// Reports what the user did with this element, then redraws. Skipped if the host is
    /// already borrowed (an action triggered while the host is being read).
    fn on<T: 'static>(&self, input: impl Fn(T) -> UserInput + 'static) -> impl FnMut(T) + 'static {
        let (host, controls, rev) = (self.host.clone(), self.controls.clone(), self.rev);
        // what the controls need of it: not its children
        let node = ResolvedTree {
            children: Vec::new(),
            ..self.node.clone()
        };
        move |value| {
            if let (Ok(mut host), Ok(mut controls)) =
                (host.try_borrow_mut(), controls.try_borrow_mut())
            {
                controls.input(&mut *host, &node, input(value));
            }
            bump(rev);
        }
    }

    /// A control's value, as [`Controls::value`].
    fn value(&self) -> Option<Json> {
        self.controls
            .borrow()
            .value(&*self.host.borrow(), &self.node)
    }
}

impl Component for NodeView {
    fn render(&self) -> impl IntoElement {
        let _ = self.rev.read();
        let node = &self.node;
        let colors = palette();
        if !attr_bool(node, "visible", true) {
            return rect().into_element();
        }

        let st = self.style;
        let is_row = node.component == HeComponent::Row;
        let wraps = attr_bool(node, "wrap", false);
        let shown = node.children.iter().filter(|child| visible(child)).count();
        // A field has a label above it, so a row with one lines its children up on the bottom
        // edge (the field and the button beside it); other rows center theirs.
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
        // A wrapping row cannot align its lines (Freya aligns against the whole row, before the
        // lines are made), so it starts them at the top and puts what has no label of its own
        // below an empty one: it lines up with the fields beside it all the same.
        let label_slot = is_row && wraps && form && cross == Alignment::End;
        let children: Vec<Element> = node
            .children
            .iter()
            .filter(|child| visible(child))
            .cloned()
            .map(|child| {
                let grows = is_row && grows_in_row(&child);
                let key = DiffKey::from(&child.id);
                let child = self.child(child);
                if grows && wraps {
                    // A wrapping row has no flex, and a filling child would take the whole line
                    // and crush the next one instead of wrapping it: give it a share of the line.
                    rect()
                        .key(key)
                        .width(wrap_share(shown, gap(node, st)))
                        .child(child)
                        .into_element()
                } else if grows {
                    // Text and fields share what the buttons leave of the row.
                    rect()
                        .key(key)
                        .width(Size::flex(1.))
                        .min_width(Size::px(96.))
                        .child(child)
                        .into_element()
                } else if label_slot {
                    rect()
                        .key(key)
                        .vertical()
                        .spacing(2.)
                        .child(txt(" ", 13., colors.text_secondary, FontWeight::MEDIUM))
                        .child(child)
                        .into_element()
                } else {
                    child
                }
            })
            .collect();

        match node.component {
            HeComponent::Column => rect()
                .vertical()
                .width(Size::fill())
                .spacing(gap(node, st))
                .cross_align(align(node, "align"))
                .children(children)
                .into_element(),
            HeComponent::Row => {
                let (content, cross) = if wraps {
                    (Content::wrap_spacing(gap(node, st)), Alignment::Start)
                } else {
                    (Content::flex(), cross)
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
                    .filter(|t| t.component == HeComponent::Tab && visible(t))
                    .collect();
                let current =
                    self.controls
                        .borrow()
                        .selected_tab(&*self.host.borrow(), node, &tabs);
                let Some(current) = current else {
                    return rect().into_element();
                };

                let mut bar = rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::wrap_spacing(st.space_small))
                    .spacing(st.space_small);
                for tab in &tabs {
                    let pick = tab.id.clone();
                    bar = bar.child(choice(
                        attr_str(tab, "label"),
                        tab.id == current.id,
                        !self.closed,
                        self.on(move |_: Event<PressEventData>| UserInput::Tab(pick.clone())),
                    ));
                }
                rect()
                    .vertical()
                    .width(Size::fill())
                    .spacing(st.space_medium)
                    .child(bar)
                    .child(self.child(current.clone()))
                    .into_element()
            }
            HeComponent::Tab => rect()
                .vertical()
                .width(Size::fill())
                .spacing(gap(node, st))
                .children(children)
                .into_element(),
            HeComponent::Dialog => {
                let dismissable = !self.closed
                    && node
                        .attrs
                        .get("onDismiss")
                        .is_some_and(|handler| !handler.is_null());
                let mut dismiss = self.on(|()| UserInput::Dismiss);
                let title = attr_str(node, "title");
                // Escape and a press outside ask to close; the server decides, by `onDismiss`.
                Popup::new()
                    .key(&node.id)
                    .on_close_request(move |()| {
                        if dismissable {
                            dismiss(())
                        }
                    })
                    .maybe(attr_bool(node, "open", true), |popup| {
                        popup
                            .maybe(!title.is_empty(), |popup| {
                                popup.child(PopupTitle::new(title.clone()))
                            })
                            .child(
                                PopupContent::new().child(
                                    rect()
                                        .vertical()
                                        .width(Size::fill())
                                        .spacing(st.space_medium)
                                        .children(children),
                                ),
                            )
                    })
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
                let variant = attr_str(node, "variant");
                if let Some(media) = media_source(&src, self.images) {
                    let (width, height) = match variant.as_str() {
                        "avatar" => (Size::px(48.), Size::px(48.)),
                        "thumbnail" => (Size::px(120.), Size::px(120.)),
                        // as wide as the page, as tall as that makes it
                        "full" => (Size::fill(), Size::auto()),
                        _ => (Size::fill(), Size::px(160.)),
                    };
                    if media.svg {
                        return SvgPicture {
                            source: media.source,
                            width,
                            height,
                            radius: st.radius,
                            alt,
                            full: variant == "full",
                        }
                        .into_element();
                    }
                    let viewer = match media.source {
                        PictureSource::Bytes(bytes) => {
                            let fallback =
                                image_placeholder(&alt, st.radius, width.clone(), height.clone());
                            ImageViewer::new((src, bytes))
                                .width(width)
                                .height(height)
                                .error_renderer(move |_| fallback.clone())
                                .into_element()
                        }
                        PictureSource::Url(url) => {
                            // A URL may serve an SVG without saying so in its path. If it is not
                            // a raster image, let the SVG viewer try the same source.
                            let fallback = SvgPicture {
                                source: PictureSource::Url(url.clone()),
                                width: width.clone(),
                                height: height.clone(),
                                radius: st.radius,
                                alt,
                                full: variant == "full",
                            }
                            .into_element();
                            ImageViewer::new(url)
                                .width(width)
                                .height(height)
                                .error_renderer(move |_| fallback.clone())
                                .into_element()
                        }
                    };
                    return viewer;
                }
                // As wide as the image would be: a small one keeps its place in a row.
                let (width, height) = match variant.as_str() {
                    "avatar" => (Size::px(48.), Size::px(48.)),
                    "thumbnail" => (Size::px(120.), Size::px(120.)),
                    _ => (Size::fill(), Size::px(120.)),
                };
                image_placeholder(&alt, st.radius, width, height)
            }
            HeComponent::Divider => rect()
                .width(Size::fill())
                .height(Size::px(1.))
                .background(colors.border)
                .into_element(),
            HeComponent::Progress => {
                let fraction = node
                    .attrs
                    .get("value")
                    .and_then(Json::as_f64)
                    .filter(|_| attr_str(node, "variant") != "spinner");
                self.skin().progress(ProgressProps {
                    label: Some(attr_str(node, "label")).filter(|label| !label.is_empty()),
                    fraction: fraction.map(|f| f as f32),
                    style: st,
                })
            }
            HeComponent::Button => {
                let enabled = !self.closed && !attr_bool(node, "disabled", false);
                let variant = match attr_str(node, "variant").as_str() {
                    "primary" | "success" => ButtonVariant::Primary,
                    "failure" => ButtonVariant::Danger,
                    "plain" => ButtonVariant::Plain,
                    _ => ButtonVariant::Secondary,
                };
                self.skin().button(ButtonProps {
                    label: attr_str(node, "label"),
                    variant,
                    enabled,
                    on_press: EventHandler::new(self.on(|()| UserInput::Press)),
                })
            }
            HeComponent::Input => {
                let variant = attr_str(node, "variant");
                let value = self.controls.borrow().text(&*self.host.borrow(), node);
                let field = self.skin().input(InputProps {
                    value,
                    placeholder: attr_str(node, "placeholder"),
                    multiline: variant == "multiline",
                    password: variant == "password",
                    enabled: !self.closed && !attr_bool(node, "disabled", false),
                    on_change: EventHandler::new(self.on(UserInput::Text)),
                    on_commit: EventHandler::new(self.on(|()| UserInput::Commit)),
                    on_submit: EventHandler::new(self.on(|_: String| UserInput::Submit)),
                });
                // keyed: its editor and focus stay its own as others come and go
                labeled(
                    node,
                    rect().key(&node.id).width(Size::fill()).child(field).into(),
                )
            }
            HeComponent::Checkbox => {
                let checked = self.value().and_then(|v| v.as_bool()).unwrap_or(false);
                self.skin().checkbox(CheckboxProps {
                    label: attr_str(node, "label"),
                    checked,
                    switch: attr_str(node, "variant") == "switch",
                    enabled: !self.closed && !attr_bool(node, "disabled", false),
                    on_toggle: EventHandler::new(self.on(move |()| UserInput::Checked(!checked))),
                    style: st,
                })
            }
            HeComponent::Select => {
                let options = select_options(node);
                let current = self
                    .value()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .filter(|value| options.iter().any(|(v, _)| v == value));
                let is = |value: &str| current.as_deref() == Some(value);
                let enabled = !self.closed && !attr_bool(node, "disabled", false);
                let pick = |value: &str| {
                    let value = value.to_owned();
                    self.on(move |_: Event<PressEventData>| UserInput::Select(value.clone()))
                };
                let control: Element = match attr_str(node, "variant").as_str() {
                    "radio" => rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(st.space_small)
                        .children(options.iter().map(|(value, label)| {
                            radio_row(label.clone(), is(value), enabled, pick(value))
                        }))
                        .into_element(),
                    "segmented" => SegmentedButton::new()
                        .children(options.iter().map(|(value, label)| {
                            ButtonSegment::new()
                                .key(value)
                                .selected(is(value))
                                .enabled(enabled)
                                .cursor(CursorIcon::Pointer)
                                .on_press(pick(value))
                                .child(label.clone())
                                .into_element()
                        }))
                        .into_element(),
                    _ => {
                        let shown = options
                            .iter()
                            .find(|(value, _)| is(value))
                            .map_or("Choose…".to_string(), |(_, label)| label.clone());
                        if enabled {
                            Select::new()
                                .selected_item(txt(
                                    shown,
                                    15.,
                                    colors.text_primary,
                                    FontWeight::NORMAL,
                                ))
                                .children(options.iter().map(|(value, label)| {
                                    MenuItem::new()
                                        .selected(is(value))
                                        .on_press(pick(value))
                                        .child(txt(
                                            label.clone(),
                                            15.,
                                            colors.text_primary,
                                            FontWeight::NORMAL,
                                        ))
                                        .into_element()
                                }))
                                .into_element()
                        } else {
                            Button::new()
                                .outline()
                                .enabled(false)
                                .child(shown)
                                .into_element()
                        }
                    }
                };
                labeled(node, control)
            }
            HeComponent::Slider => {
                let min = attr_num(node, "min").unwrap_or(0.);
                let max = attr_num(node, "max").unwrap_or(100.).max(min);
                let step = attr_num(node, "step").unwrap_or(1.);
                let value = self
                    .value()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(min)
                    .clamp(min, max);
                let percent = if max > min {
                    (value - min) / (max - min) * 100.
                } else {
                    0.
                };
                let control = Slider::new(self.on(move |percent: f64| {
                    UserInput::Slide(slider_value(percent, min, max, step))
                }))
                .value(percent)
                .enabled(!self.closed && !attr_bool(node, "disabled", false))
                .size(Size::fill())
                .into_element();
                labeled(node, control)
            }
        }
    }

    fn render_key(&self) -> DiffKey {
        // its state (a field's focus) stays its own as siblings come and go
        DiffKey::from(&self.node.id)
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

/// One option of a set of which one is chosen, all of the row pressable.
fn radio_row(
    text: String,
    selected: bool,
    enabled: bool,
    on: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    let colors = palette();
    rect()
        .horizontal()
        .spacing(10.)
        .cross_align(Alignment::Center)
        .maybe(!enabled, |el| el.opacity(0.45))
        .a11y_role(AccessibilityRole::RadioButton)
        .a11y_focusable(enabled)
        .maybe(enabled, |el| el.on_press(on))
        .child(RadioItem::new().selected(selected))
        .child(txt(text, 15., colors.text_primary, FontWeight::NORMAL))
        .into_element()
}

fn visible(node: &ResolvedTree) -> bool {
    attr_bool(node, "visible", true)
}

/// Whether a child of a row takes the row's spare width instead of its own size. Everything
/// drawn as wide as its parent does: placed as it is, it would take the whole row and push what
/// follows it out.
fn grows_in_row(node: &ResolvedTree) -> bool {
    attr_bool(node, "visible", true)
        && match node.component {
            HeComponent::Text
            | HeComponent::Input
            | HeComponent::Select
            | HeComponent::Slider
            | HeComponent::Progress
            | HeComponent::Checkbox
            | HeComponent::Column
            | HeComponent::Row
            | HeComponent::Card
            | HeComponent::List
            | HeComponent::Tabs
            | HeComponent::Tab
            | HeComponent::Notice
            | HeComponent::Divider => true,
            HeComponent::Image => {
                !matches!(attr_str(node, "variant").as_str(), "avatar" | "thumbnail")
            }
            HeComponent::Badge | HeComponent::Button | HeComponent::Dialog => false,
        }
}

/// The width of a growing child of a wrapping row of `children`: as many to a line as can be
/// at least 140 wide, each an equal part of it, and the whole line once that is just one.
fn wrap_share(children: usize, gap: f32) -> Size {
    Size::func_data(
        move |ctx| {
            let mut per_line = children.max(1);
            let part = |n: usize| (ctx.parent - (n - 1) as f32 * gap) / n as f32;
            while per_line > 1 && part(per_line) < 140. {
                per_line -= 1;
            }
            // rounded down, so the parts add up to no more than the line and none wraps early
            Some(part(per_line).floor().max(0.))
        },
        &(children, gap.to_bits()),
    )
}

/// Where the renderer gets the bytes of a picture.
#[derive(Clone, PartialEq)]
enum PictureSource {
    Bytes(Bytes),
    Url(Url),
}

/// A picture the policy lets through.
struct Media {
    source: PictureSource,
    svg: bool,
}

/// The picture at `src`, if [`ImagePolicy`] lets it through.
fn media_source(src: &str, policy: ImagePolicy) -> Option<Media> {
    if let Some(rest) = src.strip_prefix("data:") {
        if !policy.inline {
            return None;
        }
        let (meta, data) = rest.split_once(',')?;
        let meta = meta.to_ascii_lowercase();
        let mime = meta.split(';').next().unwrap_or("");
        let bytes = if meta.ends_with(";base64") {
            base64::engine::general_purpose::STANDARD
                .decode(data)
                .ok()?
        } else {
            percent_decode(data)
        };
        return Some(Media {
            source: PictureSource::Bytes(Bytes::from(bytes)),
            svg: mime == "image/svg+xml",
        });
    }

    if let Ok(url) = Url::parse(src) {
        match url.scheme() {
            "http" | "https" => {
                if !policy.remote {
                    return None;
                }
                return Some(Media {
                    svg: url.path().to_ascii_lowercase().ends_with(".svg"),
                    source: PictureSource::Url(url),
                });
            }
            "file" => {
                if !policy.files {
                    return None;
                }
                return read_picture(url.to_file_path().ok()?);
            }
            _ => {}
        }
    }

    if policy.files && src.starts_with('/') {
        return read_picture(src.into());
    }
    None
}

/// A picture read from a file of this device.
fn read_picture(path: std::path::PathBuf) -> Option<Media> {
    let bytes = std::fs::read(&path).ok()?;
    Some(Media {
        svg: path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
            || looks_like_svg(&bytes),
        source: PictureSource::Bytes(Bytes::from(bytes)),
    })
}

/// Whether the start of `bytes` is an SVG document.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    String::from_utf8_lossy(head)
        .to_ascii_lowercase()
        .contains("<svg")
}

/// Decode the `%XX` escapes of a data URI that is not base64.
fn percent_decode(data: &str) -> Vec<u8> {
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
        {
            out.push(high * 16 + low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    out
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The width/height ratio an SVG declares for itself.
fn svg_ratio(bytes: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(bytes).ok()?;
    let start = text.find("<svg")?;
    let end = start + text[start..].find('>')?;
    let tag = &text[start..end];
    let width = svg_attr(tag, "width").and_then(svg_length);
    let height = svg_attr(tag, "height").and_then(svg_length);
    if let (Some(width), Some(height)) = (width, height) {
        return Some(width / height);
    }
    let mut view_box = svg_attr(tag, "viewBox")?
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty());
    view_box.next()?;
    view_box.next()?;
    let width: f32 = view_box.next()?.parse().ok()?;
    let height: f32 = view_box.next()?.parse().ok()?;
    (width > 0. && height > 0.).then_some(width / height)
}

/// An SVG length attribute in pixels (or unitless), the units an intrinsic size can use.
fn svg_length(value: &str) -> Option<f32> {
    let value = value.trim();
    let number = value.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
    let unit = &value[number.len()..];
    if !unit.is_empty() && !unit.eq_ignore_ascii_case("px") {
        return None;
    }
    number.parse().ok().filter(|number| *number > 0.)
}

/// The raw value of `name` in an SVG tag such as `<svg width="10" viewBox="...">`.
fn svg_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(index) = rest.find(name) {
        let before = &rest[..index];
        let after = &rest[index + name.len()..];
        if before.ends_with(|c: char| c.is_whitespace())
            && let Some(after) = after.trim_start().strip_prefix('=')
        {
            let after = after.trim_start();
            if let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') {
                let value = &after[quote.len_utf8()..];
                if let Some(end) = value.find(quote) {
                    return Some(&value[..end]);
                }
            }
        }
        rest = after;
    }
    None
}

/// What an image that cannot be shown leaves: its alt text, in its place.
fn image_placeholder(alt: &str, radius: f32, width: Size, height: Size) -> Element {
    let colors = palette();
    rect()
        .width(width)
        .height(height)
        .corner_radius(radius)
        .background(colors.surface_tertiary)
        .center()
        .child(txt(alt, 13., colors.text_secondary, FontWeight::NORMAL))
        .into_element()
}

/// An SVG, drawn with [`SvgViewer`]. Remote bytes are fetched here, because a `full` SVG needs
/// its own aspect ratio to make a height, and a raster viewer with an auto height never gets one.
#[derive(Clone, PartialEq)]
struct SvgPicture {
    source: PictureSource,
    width: Size,
    height: Size,
    radius: f32,
    alt: String,
    full: bool,
}

impl Component for SvgPicture {
    fn render(&self) -> impl IntoElement {
        let loaded = use_state::<Option<(String, Result<Bytes, String>)>>(|| None);
        let remote = match &self.source {
            PictureSource::Url(url) => Some(url.clone()),
            PictureSource::Bytes(_) => None,
        };
        let needs_fetch = match (&remote, loaded.peek().as_ref()) {
            (Some(url), Some((cached, _))) => cached.as_str() != url.as_str(),
            (Some(_), None) => true,
            (None, _) => false,
        };
        use_side_effect_with_deps(
            &(remote.clone(), needs_fetch),
            move |(remote, needs_fetch)| {
                if *needs_fetch && let Some(url) = remote {
                    let mut loaded = loaded;
                    let url = (*url).clone();
                    spawn_in_window(async move {
                        let result = fetch(url.clone()).await.map_err(|err| err.to_string());
                        loaded.set(Some((url.to_string(), result)));
                    });
                }
            },
        );

        let bytes = match &self.source {
            PictureSource::Bytes(bytes) => Some(bytes.clone()),
            PictureSource::Url(url) => loaded.read().as_ref().and_then(|(cached, result)| {
                (cached.as_str() == url.as_str())
                    .then(|| result.as_ref().ok().cloned())
                    .flatten()
            }),
        };
        let remote = remote.is_some();

        // `full` is as wide as the page: measure that width, then give the SVG the height its
        // own aspect ratio asks for.
        let mut measured = use_state(|| 0.0f32);
        if self.full {
            let Some(bytes) = bytes else {
                return image_placeholder(&self.alt, self.radius, Size::fill(), Size::px(80.));
            };
            let ratio = svg_ratio(&bytes).unwrap_or(16. / 9.);
            let width = *measured.read();
            let content = if width > 0.5 {
                let fallback =
                    image_placeholder(&self.alt, self.radius, Size::fill(), Size::px(80.));
                SvgViewer::new(bytes)
                    .width(Size::px(width))
                    .height(Size::px((width / ratio).max(1.)))
                    .async_rasterization(remote)
                    .error_renderer(move |_| fallback.clone())
                    .into_element()
            } else {
                rect().into_element()
            };
            return rect()
                .width(Size::fill())
                .on_sized(move |event: Event<SizedEventData>| {
                    measured.set_if_modified(event.visible_area.width());
                })
                .child(content)
                .into_element();
        }

        match bytes {
            Some(bytes) => {
                let fallback = image_placeholder(
                    &self.alt,
                    self.radius,
                    self.width.clone(),
                    self.height.clone(),
                );
                SvgViewer::new(bytes)
                    .width(self.width.clone())
                    .height(self.height.clone())
                    .async_rasterization(remote)
                    .error_renderer(move |_| fallback.clone())
                    .into_element()
            }
            None => image_placeholder(
                &self.alt,
                self.radius,
                self.width.clone(),
                self.height.clone(),
            ),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"></svg>"#;

    fn data_uri(mime: &str, svg: &str) -> String {
        let encoded = base64::engine::general_purpose::STANDARD.encode(svg);
        format!("data:{mime};base64,{encoded}")
    }

    #[test]
    fn the_default_policy_is_inline_and_remote() {
        assert_eq!(
            ImagePolicy::default(),
            ImagePolicy {
                inline: true,
                files: false,
                remote: true,
            }
        );
    }

    #[test]
    fn svg_data_uris_are_recognized() {
        let media = media_source(&data_uri("image/svg+xml", SVG), ImagePolicy::default())
            .expect("inline SVG");
        assert!(media.svg);
        match media.source {
            PictureSource::Bytes(bytes) => assert_eq!(&bytes[..], SVG.as_bytes()),
            PictureSource::Url(_) => panic!("inline image is not remote"),
        }
    }

    #[test]
    fn percent_encoded_svg_data_uris_are_decoded() {
        let encoded = "%3Csvg%20viewBox%3D%220%200%202%201%22%3E%3C%2Fsvg%3E";
        let media = media_source(
            &format!("data:image/svg+xml,{encoded}"),
            ImagePolicy::default(),
        )
        .expect("inline SVG");
        assert!(media.svg);
        match media.source {
            PictureSource::Bytes(bytes) => {
                assert_eq!(&bytes[..], b"<svg viewBox=\"0 0 2 1\"></svg>")
            }
            PictureSource::Url(_) => panic!("inline image is not remote"),
        }
    }

    #[test]
    fn remote_sources_follow_their_extension() {
        let policy = ImagePolicy::default();
        assert!(
            media_source("https://example.com/a.svg", policy)
                .unwrap()
                .svg
        );
        assert!(
            !media_source("https://example.com/a.png", policy)
                .unwrap()
                .svg
        );
        assert!(
            media_source("http://example.com/a.svg?v=1", policy)
                .unwrap()
                .svg
        );
        assert!(media_source("https://example.com/a.png", ImagePolicy::inline()).is_none());
    }

    #[test]
    fn files_follow_the_content() {
        let path = std::env::temp_dir().join("freya-hypereffect-picture");
        std::fs::write(&path, SVG).unwrap();
        let src = format!("file://{}", path.display());
        assert!(media_source(&src, ImagePolicy::default()).is_none());
        let media = media_source(&src, ImagePolicy::all()).expect("file image");
        assert!(media.svg);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn an_svg_ratio_comes_from_its_size_or_view_box() {
        assert_eq!(svg_ratio(SVG.as_bytes()), Some(2.));
        assert_eq!(
            svg_ratio(br#"<svg width="30px" height="10px"></svg>"#),
            Some(3.)
        );
        assert_eq!(
            svg_ratio(br#"<svg width="100%" height="100%" viewBox="0 0 4 3"></svg>"#),
            Some(4. / 3.)
        );
        assert_eq!(svg_ratio(b"not an svg"), None);
        // `width` must be an attribute of its own, not the end of `stroke-width`.
        assert_eq!(
            svg_ratio(br#"<svg stroke-width="8" viewBox="0 0 5 1"></svg>"#),
            Some(5.)
        );
    }
}
