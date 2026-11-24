use std::{cell::RefCell, rc::Rc};

use hypereffect::{
    Map, Value as Json,
    client::{Client, ResolvedTree},
};
use serde::Serialize;

/// What a [`HyperView`](crate::HyperView) draws and talks to.
pub trait HyperHost {
    /// The resolved element tree, roots first.
    fn tree(&self) -> Vec<ResolvedTree>;

    /// The signal at `path`.
    fn value(&self, path: &str) -> Option<Json>;

    /// Write a signal, as the user editing a field does.
    fn write(&mut self, path: &str, value: Json);

    /// Run the handler of element `id` for `event` (`onPress`, `onChange`, ...), and send the
    /// action it makes, if any.
    fn act(&mut self, id: &str, event: &str);

    /// Counts the changes the host made on its own, signals included: a signal may change (a
    /// checkbox ticked from elsewhere) while the tree does not, and the view redraws when this
    /// number does. Hosts whose signals only change through the view can leave it at 0.
    fn revision(&self) -> u64 {
        0
    }

    /// Whether the session is over: the view keeps the last tree on screen, but the user cannot
    /// act on it. Hosts that never end a session can leave it at `false`.
    fn is_closed(&self) -> bool {
        false
    }

    /// What went wrong, to show above the interface.
    fn error(&self) -> Option<String> {
        None
    }
}

/// A host shared between the component that draws it and whatever feeds it.
pub type SharedHost = Rc<RefCell<dyn HyperHost>>;

/// A [`Client`] and the way to the server.
pub struct ClientHost {
    client: Client,
    send: Box<dyn FnMut(String)>,
    open: Box<dyn FnMut(String)>,
    diagnostic: Box<dyn FnMut(String)>,
    error: Option<String>,
}

impl ClientHost {
    /// `send` writes a line to the server. It is called with the `hello` first.
    pub fn new(send: impl FnMut(String) + 'static) -> Self {
        Self::with_hello(send, &Map::new())
    }

    /// Like [`ClientHost::new`], with `hello` as the object of the `hello` line. Its members
    /// mean nothing to hypereffect: the protocol on top of it gives them their meaning.
    pub fn with_hello<T: Serialize + ?Sized>(
        mut send: impl FnMut(String) + 'static,
        hello: &T,
    ) -> Self {
        let mut client = Client::new();
        send(client.hello_with(hello));
        Self {
            client,
            send: Box::new(send),
            open: Box::new(|_| {}),
            diagnostic: Box::new(|diagnostic| eprintln!("hypereffect: {diagnostic}")),
            error: None,
        }
    }

    /// Decide what a button asking to open a URL does. Without this they do nothing: the host
    /// owns that decision (a server must not be able to start a payment this way).
    pub fn on_open(mut self, open: impl FnMut(String) + 'static) -> Self {
        self.open = Box::new(open);
        self
    }

    /// Decide where the server's mistakes go (a message it could not apply, an expression that
    /// failed). They are for whoever wrote the server, not the user: by default they are
    /// printed to stderr.
    pub fn on_diagnostic(mut self, diagnostic: impl FnMut(String) + 'static) -> Self {
        self.diagnostic = Box::new(diagnostic);
        self
    }

    /// The underlying client, to register message handlers on or read its metadata.
    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Bytes from the server.
    pub fn receive(&mut self, bytes: &[u8]) {
        self.client.receive(bytes);
        self.report();
    }

    /// The server went away: the interface stays, but says so.
    pub fn close(&mut self, reason: Option<String>) {
        self.client.close();
        self.error = reason;
    }

    fn report(&mut self) {
        for url in self.client.take_opens() {
            (self.open)(url);
        }
        // the server's mistakes: for its developer, not on screen
        for diagnostic in self.client.take_diagnostics() {
            (self.diagnostic)(diagnostic);
        }
        if let Some(close) = self.client.closed() {
            let reason = match close.reason.as_deref() {
                Some(reason) => format!("Disconnected: {reason}"),
                None => "Disconnected. The server closed the connection.".to_string(),
            };
            self.error = Some(reason);
        }
    }
}

impl HyperHost for ClientHost {
    fn tree(&self) -> Vec<ResolvedTree> {
        self.client.tree()
    }

    fn value(&self, path: &str) -> Option<Json> {
        self.client.value(path).cloned()
    }

    fn write(&mut self, path: &str, value: Json) {
        if self.client.is_connected() {
            self.client.write(path, value);
        }
    }

    fn act(&mut self, id: &str, event: &str) {
        if self.client.is_connected()
            && let Some(line) = self.client.trigger(id, event)
        {
            (self.send)(line);
        }
        self.report();
    }

    fn is_closed(&self) -> bool {
        !self.client.is_connected()
    }

    fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

#[cfg(test)]
mod tests {
    use hypereffect::{RendererMessage, json};

    use super::*;

    #[test]
    fn hello_carries_an_object() {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let log = sent.clone();
        let _host = ClientHost::with_hello(
            move |line| log.borrow_mut().push(line),
            &json!({"room": "abc", "n": 1}),
        );

        let sent = sent.borrow();
        assert_eq!(sent.len(), 1);
        let Ok(RendererMessage::Hello(body)) = RendererMessage::parse(sent[0].trim_end()) else {
            panic!("not a hello");
        };
        assert_eq!(body["room"], json!("abc"));
        assert_eq!(body["n"], json!(1));
    }

    #[test]
    fn diagnostics_are_reported() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let mut host = ClientHost::new(|_| {}).on_diagnostic(move |d| log.borrow_mut().push(d));
        host.receive(b"element {\"id\":\"x\",\"type\":\"nonsense\"}\n");
        assert!(!seen.borrow().is_empty());
    }

    #[test]
    fn server_close_is_reported_and_stops_writes() {
        let mut host = ClientHost::new(|_| {});
        host.receive(b"signal {\"count\":1}\n");
        host.receive(b"close {\"reason\":\"session_not_found\"}\n");

        assert!(host.is_closed());
        assert_eq!(
            host.error().as_deref(),
            Some("Disconnected: session_not_found")
        );

        host.write("count", json!(2));
        assert_eq!(host.value("count"), Some(json!(1)));
    }
}
