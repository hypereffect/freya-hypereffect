//! A [Freya](https://freyaui.dev) renderer for [hypereffect](https://hypereffect.dev).
//!
//! [`hypereffect::client::Client`] keeps the element tree, applies signals, evaluates CEL and
//! resolves every attribute; this crate draws the resolved tree with Freya's own components, in
//! the colors of the app's Freya theme, and sends the user's actions back.
//!
//! The renderer does not know where the interface comes from: it draws a [`HyperHost`]. A
//! [`ClientHost`] is the ready-made one, a `Client` and a function that sends it the lines to
//! write to the server; a host of your own can drive a server in-process (a WebAssembly app, say).
//!
//! ```ignore
//! let host = Rc::new(RefCell::new(ClientHost::new(|line| send_to_server(line))));
//! host.borrow_mut().receive(bytes_from_server);
//! HyperView::new(host, rev)
//! ```
//!
//! Freya redraws a component when state it read changes, and the host is not state, so a
//! [`HyperView`] takes a `rev` counter: bump it whenever the host changes from outside (a message
//! arrived). The renderer bumps it itself after the user acts.

mod host;
mod skin;
mod view;

pub use host::{ClientHost, HyperHost, SharedHost};
pub use skin::{
    ButtonProps, ButtonVariant, CardProps, CheckboxProps, InputProps, ProgressProps, SharedSkin,
    Skin, StockSkin,
};
pub use view::{HyperView, ImagePolicy, Style};
