// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use std::rc::Rc;

use nlhgui::LaunchConfig;
use nlhgui::widgets::reactive::{ReactiveUI, UIBuildArg};
use nlhgui::widgets::{
    Checkbox, CheckboxController, Container, RadioButtonGroup, RadioButtonGroupCtrl, TextField,
    TextFieldController, TextFieldEditEvent, TextLine,
};

struct MyState {
    input_ctrl: Rc<TextFieldController>,
    edit_count: usize,
    radio_ctrl: Rc<RadioButtonGroupCtrl>,
    checkbox_ctrl: Rc<CheckboxController>,
}

fn on_edit(arg: UIBuildArg<MyState>, _ev: TextFieldEditEvent) {
    arg.state_mut().edit_count += 1;
    arg.request_rebuild();
}

fn build_ui(arg: UIBuildArg<MyState>) -> Container {
    let mut container = Container::new([]);

    container.add(TextLine::new("TextField".to_string()));
    let text_field =
        TextField::new(arg.state().input_ctrl.clone()).with_edit_handler(arg.handler(on_edit));
    container.add(text_field);

    let val = arg.state().input_ctrl.get_text();
    container.add(TextLine::new(format!(
        "Current value: {}, edit count: {}",
        val,
        arg.state().edit_count
    )));

    container.add(TextLine::new("RadioButtonGroup:".to_string()));
    let radio_group = RadioButtonGroup::new(arg.state().radio_ctrl.clone());
    container.add(radio_group);

    container.add(TextLine::new("Checkbox:".to_string()));
    let checkbox = Checkbox::new(arg.state().checkbox_ctrl.clone());
    container.add(checkbox);

    container
}

pub fn run_input() {
    let input_ctrl = TextFieldController::new();
    let radio_ctrl = RadioButtonGroupCtrl::new(["option a", "another option", "3rd option"], 0);
    let checkbox_ctrl = CheckboxController::new("checkbox label".to_string(), false);
    let state = MyState {
        edit_count: 0,
        input_ctrl: Rc::new(input_ctrl),
        radio_ctrl: Rc::new(radio_ctrl),
        checkbox_ctrl: Rc::new(checkbox_ctrl),
    };
    let root = ReactiveUI::new(state, build_ui);
    LaunchConfig::default().with_title(file!()).launch(root);
}
