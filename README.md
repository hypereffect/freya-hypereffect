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
- Images: `data:` URIs and remote `http(s)` URLs by default (`ImagePolicy`); files of the device (`file://`, absolute paths) are off unless enabled. Restrict further with `ImagePolicy::inline()` or `ImagePolicy::none()`. SVG images are drawn too, inline or remote.
- Buttons that ask to open a URL do nothing unless the host decides what to do with them (`ClientHost::on_open`).
- A server ending the session (`close`) leaves the last interface on screen, with its reason above it, and the user can no longer act on it.
- The server's mistakes (`diagnostics`: a message it could not apply, an expression that failed) are printed to stderr, never shown in the interface; `ClientHost::on_diagnostic` sends them elsewhere.

## Debugging: seeing a page without a window

`examples/snapshot.rs` draws a recorded flow to a PNG, headless, at the size you give it. Record the state you want with [`copcat`](https://github.com/cassiscash/copcat) (the first page with `copcat stream`, or after a press with `--record`), then draw it:

```sh
copcat press <uri> add --record flow.txt
cargo run --example snapshot -- flow.txt page.png 380 1000   # width and height, 640×900 by default
```

It feeds the server's lines of the transcript (`< ` ones) to a `ClientHost` and renders what they leave, so it is good for:

- checking how a real merchant's page actually lays out, rather than how the tree reads: wrapping rows, image sizes, alignment;
- comparing a phone width against a desktop one in one go;
- looking at a layout change before and after, on the same recording.

Nothing is sent back to the server: it shows one state. Use `copcat` to reach it first.
