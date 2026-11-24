//! What the controls mean, apart from how they look: which signal a control shows and writes,
//! which handler what the user did runs, and what a control remembers on its own. Nothing here
//! knows about Freya, so it is tested without drawing anything.
//!
//! A control with `bind` reads and writes its signal. One without keeps its value here, by
//! element id, and still runs its handlers: an unbound checkbox ticks and fires `onChange`.

use std::collections::HashMap;

use hypereffect::{Component, Value as Json, client::ResolvedTree};

use crate::HyperHost;

/// What the user did with an element.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Input {
    Press,
    /// A field's text, as it is typed: written to the signal, no handler.
    Text(String),
    /// The focus left a field: `onChange`.
    Commit,
    /// Enter in a field: `onSubmit`.
    Submit,
    Checked(bool),
    Select(String),
    Slide(f64),
    Tab(String),
    Dismiss,
}

/// The values of unbound controls, and the text typed into fields.
#[derive(Default)]
pub(crate) struct Controls {
    /// values of controls that aren't bound to a signal, by id
    local: HashMap<String, Json>,
    /// what was typed into fields, by id, shown as long as it still means the value
    drafts: HashMap<String, String>,
}

impl Controls {
    /// A control's value: its signal's, or what is remembered for it.
    pub fn value(&self, host: &dyn HyperHost, node: &ResolvedTree) -> Option<Json> {
        match bind(node) {
            Some(path) => host.value(path),
            None => self.local.get(&node.id).cloned(),
        }
    }

    /// The text a field shows: what was typed, while it still says the same as the value ("1."
    /// is 1), or else the value.
    pub fn text(&self, host: &dyn HyperHost, node: &ResolvedTree) -> String {
        let value = self.value(host, node).unwrap_or(Json::Null);
        if is_number(node)
            && let Some(draft) = self.drafts.get(&node.id)
            && parse_number(draft).as_f64() == value.as_f64()
        {
            return draft.clone();
        }
        match value {
            Json::String(s) => s,
            Json::Number(n) => n.to_string(),
            Json::Bool(b) => b.to_string(),
            _ => String::new(),
        }
    }

    /// The tab of `tabs` to show: the chosen one if it is still there, or else the first.
    pub fn selected_tab<'a>(
        &self,
        host: &dyn HyperHost,
        node: &ResolvedTree,
        tabs: &[&'a ResolvedTree],
    ) -> Option<&'a ResolvedTree> {
        let chosen = self.value(host, node);
        let chosen = chosen.as_ref().and_then(Json::as_str);
        tabs.iter()
            .find(|tab| Some(tab.id.as_str()) == chosen)
            .or(tabs.first())
            .copied()
    }

    /// Takes what the user did with `node`: writes the control's value and runs its handler.
    /// Nothing happens once the session is over.
    pub fn input(&mut self, host: &mut dyn HyperHost, node: &ResolvedTree, input: Input) {
        if host.is_closed() {
            return;
        }
        let event = match (node.component, input) {
            (Component::Button, Input::Press) => "onPress",
            (Component::Input, Input::Text(text)) => {
                let value = if is_number(node) {
                    parse_number(&text)
                } else {
                    Json::String(text.clone())
                };
                self.drafts.insert(node.id.clone(), text);
                self.write(host, node, value);
                return;
            }
            (Component::Input, Input::Commit) => "onChange",
            (Component::Input, Input::Submit) => "onSubmit",
            (Component::Checkbox, Input::Checked(checked)) => {
                self.write(host, node, Json::Bool(checked));
                "onChange"
            }
            (Component::Select, Input::Select(value)) => {
                self.write(host, node, Json::String(value));
                "onChange"
            }
            (Component::Slider, Input::Slide(value)) => {
                let value = number(value);
                // a drag reports every move: only a new value is a change
                if self.value(host, node).and_then(|v| v.as_f64()) == value.as_f64() {
                    return;
                }
                self.write(host, node, value);
                "onChange"
            }
            (Component::Tabs, Input::Tab(id)) => {
                self.write(host, node, Json::String(id));
                "onChange"
            }
            (Component::Dialog, Input::Dismiss) => "onDismiss",
            _ => return,
        };
        host.act(&node.id, event);
    }

    /// Forgets the elements that are gone.
    pub fn retain(&mut self, tree: &[ResolvedTree]) {
        fn ids<'a>(tree: &'a [ResolvedTree], out: &mut Vec<&'a str>) {
            for node in tree {
                out.push(&node.id);
                ids(&node.children, out);
            }
        }
        let mut present = Vec::new();
        ids(tree, &mut present);
        self.local.retain(|id, _| present.contains(&id.as_str()));
        self.drafts.retain(|id, _| present.contains(&id.as_str()));
    }

    fn write(&mut self, host: &mut dyn HyperHost, node: &ResolvedTree, value: Json) {
        match bind(node) {
            Some(path) => host.write(path, value),
            None => {
                self.local.insert(node.id.clone(), value);
            }
        }
    }
}

fn bind(node: &ResolvedTree) -> Option<&str> {
    node.attrs
        .get("bind")
        .and_then(Json::as_str)
        .filter(|path| !path.is_empty())
}

fn is_number(node: &ResolvedTree) -> bool {
    node.attrs.get("variant").and_then(Json::as_str) == Some("number")
}

/// A JSON number, whole when it can be.
pub(crate) fn number(n: f64) -> Json {
    if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        Json::from(n as i64)
    } else {
        Json::from(n)
    }
}

/// What a number field holds for what was typed: a number, or null.
fn parse_number(text: &str) -> Json {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map_or(Json::Null, number)
}

/// The value a slider from `min` to `max` stands for at `percent`, on a multiple of `step`.
pub(crate) fn slider_value(percent: f64, min: f64, max: f64, step: f64) -> f64 {
    let max = max.max(min);
    let value = min + percent.clamp(0., 100.) / 100. * (max - min);
    if step > 0. {
        (min + ((value - min) / step).round() * step).clamp(min, max)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use hypereffect::{Handler, RendererMessage, ServerMessage, cel, json, ui};

    use super::*;
    use crate::ClientHost;

    /// A host that got `messages`, and the actions it sent.
    fn host(messages: Vec<ServerMessage>) -> (ClientHost, Rc<RefCell<Vec<String>>>) {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let log = sent.clone();
        let mut host = ClientHost::new(move |line| {
            if let Ok(RendererMessage::Action(action)) = RendererMessage::parse(line.trim_end()) {
                log.borrow_mut().push(action.name);
            }
        });
        for m in &messages {
            host.client().apply(m);
        }
        (host, sent)
    }

    fn node(host: &ClientHost, id: &str) -> ResolvedTree {
        fn find(tree: Vec<ResolvedTree>, id: &str) -> Option<ResolvedTree> {
            tree.into_iter().find_map(|n| {
                if n.id == id {
                    Some(n)
                } else {
                    find(n.children, id)
                }
            })
        }
        find(host.tree(), id).unwrap()
    }

    #[test]
    fn number_fields_keep_what_was_typed() {
        let (mut host, _) = host(vec![
            ServerMessage::signal(json!({"amount": 1})),
            ui::input("amount").variant("number").bind("amount").into(),
        ]);
        let mut c = Controls::default();
        let amount = node(&host, "amount");
        assert_eq!(c.text(&host, &amount), "1");
        c.input(&mut host, &amount, Input::Text("2.".into()));
        assert_eq!(c.text(&host, &amount), "2.");
        assert_eq!(host.value("amount"), Some(json!(2)));
        c.input(&mut host, &amount, Input::Text("x".into()));
        // null is no number: the signal goes, the text stays
        assert_eq!(host.value("amount"), None);
        assert_eq!(c.text(&host, &amount), "x");
        // the server sets it: that wins over the draft
        host.client()
            .apply(&ServerMessage::signal(json!({"amount": 7})));
        assert_eq!(c.text(&host, &amount), "7");
    }

    #[test]
    fn fields_fire_on_change_when_done_typing() {
        let (mut host, sent) = host(vec![
            ui::input("name")
                .bind("name")
                .on_change(Handler::action("changed"))
                .on_submit(Handler::action("submitted"))
                .into(),
        ]);
        let mut c = Controls::default();
        let name = node(&host, "name");
        c.input(&mut host, &name, Input::Text("a".into()));
        c.input(&mut host, &name, Input::Text("ab".into()));
        assert!(sent.borrow().is_empty());
        c.input(&mut host, &name, Input::Commit);
        c.input(&mut host, &name, Input::Submit);
        assert_eq!(*sent.borrow(), ["changed", "submitted"]);
    }

    #[test]
    fn unbound_controls_remember_and_fire() {
        let (mut host, sent) = host(vec![
            ui::checkbox("c")
                .on_change(Handler::action("ticked"))
                .into(),
        ]);
        let mut c = Controls::default();
        let checkbox = node(&host, "c");
        c.input(&mut host, &checkbox, Input::Checked(true));
        assert_eq!(c.value(&host, &checkbox), Some(json!(true)));
        assert_eq!(*sent.borrow(), ["ticked"]);
        // gone, and forgotten
        host.client().apply(&ServerMessage::remove("c"));
        c.retain(&host.tree());
        assert_eq!(c.value(&host, &checkbox), None);
    }

    #[test]
    fn sliders_snap_and_report_only_changes() {
        assert_eq!(slider_value(33., 0., 10., 1.), 3.);
        assert_eq!(slider_value(120., 0., 10., 1.), 10.);
        let (mut host, sent) = host(vec![
            ui::slider("s")
                .bind("s")
                .on_change(Handler::action("slid"))
                .into(),
        ]);
        let mut c = Controls::default();
        let slider = node(&host, "s");
        c.input(&mut host, &slider, Input::Slide(3.));
        c.input(&mut host, &slider, Input::Slide(3.));
        assert_eq!(host.value("s"), Some(json!(3)));
        assert_eq!(*sent.borrow(), ["slid"]);
    }

    #[test]
    fn closed_sessions_take_no_input() {
        let (mut host, sent) = host(vec![
            ui::button("b").on_press(Handler::action("go")).into(),
            ui::checkbox("c").disabled(cel("false")).into(),
        ]);
        host.close(None);
        let mut c = Controls::default();
        let button = node(&host, "b");
        c.input(&mut host, &button, Input::Press);
        let checkbox = node(&host, "c");
        c.input(&mut host, &checkbox, Input::Checked(true));
        assert!(sent.borrow().is_empty());
        assert_eq!(c.value(&host, &checkbox), None);
    }
}
