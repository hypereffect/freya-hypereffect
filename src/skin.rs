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
    /// Hide what is typed.
    pub password: bool,
    pub enabled: bool,
    /// The text, as it is typed.
    pub on_change: EventHandler<String>,
    /// The user is done with the field: the focus left it.
    pub on_commit: EventHandler<()>,
    /// Enter, in a single-line field.
    pub on_submit: EventHandler<String>,
}

pub struct CheckboxProps {
    pub label: String,
    pub checked: bool,
    /// Drawn as a switch rather than a box (the `switch` variant).
    pub switch: bool,
    pub enabled: bool,
    /// Flip it; the view writes the new value.
    pub on_toggle: EventHandler<()>,
    pub style: Style,
}

pub struct ProgressProps {
    pub label: Option<String>,
    /// How far along, from 0 to 1; `None` when it is not known (a spinner).
    pub fraction: Option<f32>,
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
    StockInput(Rc::new(props)).into_element()
}

/// Freya's field, which also tells when the focus leaves it.
#[derive(Clone)]
struct StockInput(Rc<InputProps>);

impl PartialEq for StockInput {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (&*self.0, &*other.0);
        a.value == b.value
            && a.placeholder == b.placeholder
            && a.multiline == b.multiline
            && a.password == b.password
            && a.enabled == b.enabled
            && a.on_change == b.on_change
            && a.on_commit == b.on_commit
            && a.on_submit == b.on_submit
    }
}

impl Component for StockInput {
    fn render(&self) -> impl IntoElement {
        let props = &*self.0;
        let a11y_id = use_a11y();
        let focus = use_focus(a11y_id);
        let mut was_focused = use_state(|| false);
        let on_commit = use_reactive(&props.on_commit);
        use_side_effect(move || {
            let focused = focus().is_focused();
            if *was_focused.peek() && !focused {
                on_commit.peek().call(());
            }
            if *was_focused.peek() != focused {
                was_focused.set(focused);
            }
        });

        let on_submit = props.on_submit.clone();
        Input::new(WritableSetter::new(
            props.value.clone(),
            props.on_change.clone(),
        ))
        .a11y_id(a11y_id)
        .placeholder(props.placeholder.clone())
        .multiline(props.multiline)
        .mode(if props.password {
            InputMode::new_password()
        } else {
            InputMode::Shown
        })
        .enabled(props.enabled)
        .maybe(!props.multiline, |input| {
            input.on_submit(move |text: String| on_submit.call(text))
        })
        .width(Size::fill())
    }
}

fn stock_checkbox(props: CheckboxProps) -> Element {
    let colors = palette();
    let CheckboxProps {
        label,
        checked,
        switch,
        enabled,
        on_toggle,
        style,
    } = props;
    let mark: Element = if switch {
        // the switch takes the press itself
        let on_toggle = on_toggle.clone();
        Switch::new(WritableSetter::new(checked, move |_| on_toggle.call(())))
            .enabled(enabled)
            .cursor(CursorIcon::Pointer)
            .into()
    } else {
        Checkbox::new().selected(checked).into()
    };
    rect()
        .horizontal()
        .spacing(style.space_medium)
        .cross_align(Alignment::Center)
        .maybe(!enabled, |el| el.opacity(0.45))
        .a11y_role(if switch {
            AccessibilityRole::Switch
        } else {
            AccessibilityRole::CheckBox
        })
        .a11y_focusable(enabled && !switch)
        // all of the row, label included, ticks a box
        .maybe(enabled && !switch, |el| {
            el.on_press(move |_: Event<PressEventData>| on_toggle.call(()))
        })
        .child(mark)
        .maybe(!label.is_empty(), |el| {
            el.child(txt(label, 15., colors.text_primary, FontWeight::NORMAL))
        })
        .into_element()
}

fn stock_progress(props: ProgressProps) -> Element {
    let colors = palette();
    let bar: Element = match props.fraction {
        Some(fraction) => {
            let percent = fraction.clamp(0., 1.) * 100.;
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
                ))
                .into_element()
        }
        None => CircularLoader::new().size(28.).into_element(),
    };
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(props.style.space_small)
        .map(props.label, |el, label| {
            el.child(txt(label, 13., colors.text_secondary, FontWeight::NORMAL))
        })
        .child(bar)
        .into_element()
}
