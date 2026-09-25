// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use std::{f64::consts::PI, thread::sleep, time::Duration};

use nlhgui::{
    LaunchConfig,
    widgets::{
        Container, TextButton, TextLine,
        ex::Widget,
        reactive::{ConcurrentTaskArg, ReactiveUI, UIBuildArg},
    },
};

async fn heavy_calc_pi(ctx: ConcurrentTaskArg<MyTaskEvent>) {
    let half_second = Duration::from_millis(500);
    sleep(half_second);
    ctx.send(MyTaskEvent::UpdatePercentage(33));
    sleep(half_second);
    ctx.send(MyTaskEvent::UpdatePercentage(66));
    sleep(half_second);
    ctx.send(MyTaskEvent::UpdatePercentage(99));
    sleep(half_second);
    ctx.send(MyTaskEvent::Done(PI));
}

enum MyTaskEvent {
    UpdatePercentage(u8),
    Done(f64),
}

enum CalcState {
    NotStarted,
    Done(f64),
    Ongoing(u8),
}

fn handle_event(ev: MyTaskEvent, s: UIBuildArg<MyState, (), MyTaskEvent>) {
    match ev {
        MyTaskEvent::Done(result) => {
            s.state_mut().calc = CalcState::Done(result);
        }
        MyTaskEvent::UpdatePercentage(percent) => {
            s.state_mut().calc = CalcState::Ongoing(percent);
        }
    }
    s.request_rebuild();
}

fn build_ui(arg: UIBuildArg<MyState, (), MyTaskEvent>) -> Container {
    let mut container = Container::new([]);
    container.add(TextLine::new("Heavy calculation, status below".to_string()));
    container.add(TextButton::new(
        "start calculation".to_string(),
        arg.handler(|arg, _ev| {
            arg.state_mut().calc = CalcState::Ongoing(0);
            arg.request_rebuild();
            let ctx = arg.get_concurrent_task_ctx();
            arg.run_blocking(heavy_calc_pi(ctx));
        }),
    ));
    let status_str = match &arg.state().calc {
        CalcState::NotStarted => "not started".to_string(),
        CalcState::Done(result) => format!("done, result: {}", result),
        CalcState::Ongoing(percent) => format!("ongoing: {} %", percent),
    };
    container.add(TextLine::new(status_str));
    container
}

struct MyState {
    calc: CalcState,
}

pub fn run_block_good() {
    let init_state = MyState {
        calc: CalcState::NotStarted,
    };
    let root = ReactiveUI::new(init_state, build_ui).with_event_handler(handle_event);
    LaunchConfig::default().with_title(file!()).launch(root);
}
