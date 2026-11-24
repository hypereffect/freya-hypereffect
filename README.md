# freya-hypereffect

A [Freya](https://freyaui.dev) renderer for [hypereffect](https://hypereffect.dev). The [`Client`](https://docs.rs/hypereffect/0.7.0/hypereffect/client/struct.Client.html) of the hypereffect crate keeps the element tree, signals and CEL; this crate draws the resolved tree with Freya's own components, in the colors of the app's Freya theme, and sends the user's actions back.

Built against hypereffect `0.7.0` from crates.io.

```rust
let host = Rc::new(RefCell::new(ClientHost::new(|line| send_to_server(line))));
host.borrow_mut().receive(bytes_from_server);

// in a component:
let rev = use_state(|| 0u64);
HyperView::new(host.clone(), rev)
```

- `HyperHost` is what the view draws: a tree, signals, a way to act. `ClientHost` is the ready-made one over a `Client` and a function that writes lines to the server; implement the trait to drive something else (a WebAssembly app in-process).
- Images: only `data:` URIs by default (`ImagePolicy`). A server does not get to make the app fetch URLs or read files.
- Buttons that ask to open a URL do nothing unless the host decides what to do with them (`ClientHost::on_open`).
- A server ending the session (`close`) leaves the last interface on screen, with its reason above it, and the user can no longer act on it.
- The server's mistakes (`diagnostics`: a message it could not apply, an expression that failed) are printed to stderr, never shown in the interface; `ClientHost::on_diagnostic` sends them elsewhere.
