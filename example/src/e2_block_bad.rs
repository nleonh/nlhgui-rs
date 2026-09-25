// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

// WARNING:
// This example shows you how NOT to handle blocking tasks!
// Clicking the button "start calculation" will freeze the window which is very bad!

use std::{f64::consts::PI, thread::sleep, time::Duration};

use log::warn;
use nlhgui::{
    LaunchConfig,
    widgets::{
        Container, TextButton, TextLine,
        reactive::{ReactiveUI, UIBuildArg},
    },
};

fn heavy_calc_pi() -> f64 {
    sleep(Duration::from_secs(5));
    PI
}

fn build_ui(b: UIBuildArg<MyState>) -> Container {
    let mut c = Container::new([]);
    c.add(TextLine::new("Heavy calculation, status below".to_string()));
    c.add(TextButton::new(
        "start calculation".to_string(),
        b.handler(|s, _| {
            s.state_mut().calc = CalcState::Ongoing;
            s.request_rebuild();
            let x = heavy_calc_pi();
            s.state_mut().calc = CalcState::Done(x);
        }),
    ));
    let status_str = match &b.state().calc {
        CalcState::NotStarted => "not started".to_string(),
        CalcState::Done(result) => format!("done, result: {}", result),
        CalcState::Ongoing => "ongoing".to_string(),
    };
    c.add(TextLine::new(status_str));
    c
}

enum CalcState {
    NotStarted,
    Done(f64),
    Ongoing,
}

struct MyState {
    calc: CalcState,
}

pub fn run_block_bad() {
    warn!("This example shows you how NOT to handle blocking tasks.");
    let init_state = MyState {
        calc: CalcState::NotStarted,
    };
    let root = ReactiveUI::new(init_state, build_ui);
    LaunchConfig::default().with_title(file!()).launch(root);
}
