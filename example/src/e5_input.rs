// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use std::rc::Rc;

use nlhgui::{
    LaunchConfig,
    widgets::{
        Container, TextField, TextFieldController, TextFieldEditEvent, TextLine,
        reactive::{ReactiveUI, UIBuildArg},
    },
};

struct MyState {
    input_ctrl: Rc<TextFieldController>,
    edit_count: usize,
}

fn on_edit(arg: UIBuildArg<MyState>, _ev: TextFieldEditEvent) {
    arg.state_mut().edit_count += 1;
    arg.request_rebuild();
}

fn build_ui(arg: UIBuildArg<MyState>) -> Container {
    let mut container = Container::new([]);

    let text_field =
        TextField::new(arg.state().input_ctrl.clone()).with_edit_handler(arg.handler(on_edit));
    container.add(text_field);

    let val = arg.state().input_ctrl.get_text();
    container.add(TextLine::new(format!(
        "Current value: {}, edit count: {}",
        val,
        arg.state().edit_count
    )));

    container
}

pub fn run_input() {
    let input_ctrl = TextFieldController::new();
    let state = MyState {
        edit_count: 0,
        input_ctrl: Rc::new(input_ctrl),
    };
    let root = ReactiveUI::new(state, build_ui);
    LaunchConfig::default().with_title(file!()).launch(root);
}
