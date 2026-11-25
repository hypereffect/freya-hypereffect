//! Draws a recorded flow to a PNG, without a window: what a page looks like at a given size.
//!
//! ```sh
//! cop press <uri> add --record flow.txt
//! cargo run --example snapshot -- flow.txt page.png 380 1000
//! ```
//!
//! The transcript is one `cop` writes (`cop stream`, or `--record`): the server's lines, those
//! starting with `< `, are fed to a `ClientHost` as if they had just arrived, and the page they
//! leave is drawn. Nothing is sent back, so it shows a state; reach the state with `cop` first.

use std::{cell::RefCell, process::exit, rc::Rc, time::Duration};

use freya::prelude::*;
use freya_hypereffect::{ClientHost, HyperView, SharedHost};
use freya_testing::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (transcript, out) = match &args[..] {
        [transcript, out, ..] => (transcript, out),
        _ => {
            eprintln!("usage: snapshot <transcript> <out.png> [width] [height]");
            exit(2);
        }
    };
    let size = |i: usize, default: f32| {
        args.get(i).map_or(default, |s| {
            s.parse().unwrap_or_else(|_| {
                eprintln!("not a size: {s}");
                exit(2)
            })
        })
    };
    let (width, height) = (size(2, 640.), size(3, 900.));

    let text = std::fs::read_to_string(transcript).unwrap_or_else(|err| {
        eprintln!("{transcript}: {err}");
        exit(1)
    });
    let lines: String = text
        .lines()
        .filter_map(|line| line.strip_prefix("< "))
        .map(|line| format!("{line}\n"))
        .collect();

    let app = move || {
        let host: SharedHost = use_hook(|| {
            let mut host = ClientHost::new(|_| {});
            host.receive(lines.as_bytes());
            Rc::new(RefCell::new(host)) as SharedHost
        });
        let rev = use_state(|| 0u64);
        ScrollView::new().child(rect().padding(16.).child(HyperView::new(host, rev)))
    };

    let (mut runner, _) = TestingRunner::new(app, Size2D::new(width, height), |_| {}, 1.0);
    runner.sync_and_update();
    // images load in the background
    runner.poll(Duration::from_millis(50), Duration::from_millis(1500));
    runner.sync_and_update();
    runner.render_to_file(out);
}
