use std::{cell::RefCell, rc::Rc};

use hypereffect::{
    Value as Json,
    client::{Client, ResolvedTree},
};

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
    error: Option<String>,
}

impl ClientHost {
    /// `send` writes a line to the server. It is called with the `hello` first.
    pub fn new(send: impl FnMut(String) + 'static) -> Self {
        Self::with_hello(send, &Default::default())
    }

    /// Like [`ClientHost::new`], with the parameters of the session in the `hello`.
    pub fn with_hello(
        mut send: impl FnMut(String) + 'static,
        params: &std::collections::BTreeMap<String, String>,
    ) -> Self {
        let mut client = Client::new();
        send(client.hello_with(params));
        Self {
            client,
            send: Box::new(send),
            open: Box::new(|_| {}),
            error: None,
        }
    }

    /// Decide what a button asking to open a URL does. Without this they do nothing: the host
    /// owns that decision (a server must not be able to start a payment this way).
    pub fn on_open(mut self, open: impl FnMut(String) + 'static) -> Self {
        self.open = Box::new(open);
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
        // Diagnostics are the server's mistakes: not for the user.
        let _ = self.client.take_diagnostics();
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
        self.client.write(path, value);
    }

    fn act(&mut self, id: &str, event: &str) {
        if let Some(line) = self.client.trigger(id, event) {
            (self.send)(line);
        }
        self.report();
    }

    fn error(&self) -> Option<String> {
        self.error.clone()
    }
}
