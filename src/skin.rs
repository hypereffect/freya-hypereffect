//! What the view draws its controls with. The [`StockSkin`] uses Freya's own components in the
//! colors of the theme; implement [`Skin`] to draw some or all of them yourself and hand it to
//! [`HyperView::skin`](crate::HyperView::skin). Every method has the stock drawing as its default,
//! so a skin only writes the controls it wants to change.
//!
//! The view keeps what the controls mean (binding them to signals, running the handlers, labels
//! and errors around fields); a skin only gives them a look and calls the handlers it is given.

use std::rc::Rc;

use freya::prelude::*;

use crate::view::{Style, palette, txt};

/// Which kind of button an element asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonVariant {
    /// The main action (`primary` and `success`).
    Primary,
    /// Any other action.
    Secondary,
    /// Quiet, no frame (`plain`).
    Plain,
    /// Destructive (`failure`).
    Danger,
}

pub struct ButtonProps {
    pub label: String,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub on_press: EventHandler<()>,
}

pub struct CardProps {
    pub title: Option<String>,
    pub children: Vec<Element>,
    pub style: Style,
}

pub struct InputProps {
    /// The text to show.
    pub value: String,
    pub placeholder: String,
    pub multiline: bool,
    pub enabled: bool,
    pub on_change: EventHandler<String>,
    pub on_submit: EventHandler<String>,
}

pub struct CheckboxProps {
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
    /// Flip it; the view writes the new value.
    pub on_toggle: EventHandler<()>,
    pub style: Style,
}

pub struct ProgressProps {
    pub label: Option<String>,
    /// How far along, from 0 to 1.
    pub fraction: f32,
    pub style: Style,
}

/// How the view's controls look.
pub trait Skin {
    fn button(&self, props: ButtonProps) -> Element {
        stock_button(props)
    }

    fn card(&self, props: CardProps) -> Element {
        stock_card(props)
    }

    /// The field itself: its label and error are drawn around it by the view.
    fn input(&self, props: InputProps) -> Element {
        stock_input(props)
    }

    fn checkbox(&self, props: CheckboxProps) -> Element {
        stock_checkbox(props)
    }

    fn progress(&self, props: ProgressProps) -> Element {
        stock_progress(props)
    }
}

/// Freya's own components, in the colors of the theme.
pub struct StockSkin;

impl Skin for StockSkin {}

/// A skin shared between the view and the app that made it.
pub type SharedSkin = Rc<dyn Skin>;

fn stock_button(props: ButtonProps) -> Element {
    let colors = palette();
    let on = {
        let on_press = props.on_press;
        move |_: Event<PressEventData>| on_press.call(())
    };
    let button = Button::new()
        .enabled(props.enabled)
        .cursor(CursorIcon::Pointer)
        .on_press(on);
    match props.variant {
        ButtonVariant::Primary => button.filled().child(props.label).into_element(),
        ButtonVariant::Danger => button
            .filled()
            .theme_colors(
                ButtonColorsThemePartial::new()
                    .background(colors.error)
                    .hover_background(colors.error.with_a(200)),
            )
            .child(props.label)
            .into_element(),
        ButtonVariant::Plain => button.flat().child(props.label).into_element(),
        ButtonVariant::Secondary => button.outline().child(props.label).into_element(),
    }
}

fn stock_card(props: CardProps) -> Element {
    let colors = palette();
    let style = props.style;
    rect()
        .vertical()
        .width(Size::fill())
        .padding(style.space_medium)
        .spacing(style.space_medium)
        .corner_radius(style.radius)
        .background(colors.surface_secondary)
        .map(props.title, |el, title| {
            el.child(txt(title, 18., colors.text_primary, FontWeight::BOLD))
        })
        .children(props.children)
        .into_element()
}

fn stock_input(props: InputProps) -> Element {
    Input::new(WritableSetter::new(props.value, props.on_change))
        .placeholder(props.placeholder)
        .multiline(props.multiline)
        .enabled(props.enabled)
        .on_submit(move |text: String| props.on_submit.call(text))
        .width(Size::fill())
        .into_element()
}

fn stock_checkbox(props: CheckboxProps) -> Element {
    let colors = palette();
    let on_toggle = props.on_toggle;
    rect()
        .horizontal()
        .spacing(props.style.space_medium)
        .cross_align(Alignment::Center)
        .maybe(props.enabled, |el| {
            el.on_pointer_down(move |_: Event<PointerEventData>| on_toggle.call(()))
        })
        .child(Checkbox::new().selected(props.checked))
        .maybe(!props.label.is_empty(), |el| {
            el.child(txt(
                props.label,
                15.,
                colors.text_primary,
                FontWeight::NORMAL,
            ))
        })
        .into_element()
}

fn stock_progress(props: ProgressProps) -> Element {
    let colors = palette();
    let percent = props.fraction.clamp(0., 1.) * 100.;
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(props.style.space_small)
        .map(props.label, |el, label| {
            el.child(txt(label, 13., colors.text_secondary, FontWeight::NORMAL))
        })
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(props.style.space_medium)
                .child(
                    // Freya's bar takes 0 to 100 and prints it with decimals; the view prints
                    // the whole percent itself.
                    ProgressBar::new(percent)
                        .show_progress(false)
                        .width(Size::flex(1.)),
                )
                .child(txt(
                    format!("{}%", percent.round()),
                    13.,
                    colors.text_secondary,
                    FontWeight::NORMAL,
                )),
        )
        .into_element()
}
