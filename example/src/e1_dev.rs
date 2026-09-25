// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use std::rc::Rc;

use nlhgui::{
    LaunchConfig,
    colors::BLACK,
    widgets::{
        Alignment, BoxBorder, BoxPadding, BoxWidget, ClickEvent, Container, TextButton, TextField,
        TextFieldController, TextLine,
        reactive::{ReactiveUI, UIBuildArg},
    },
};

fn on_btn_click(state: UIBuildArg<SimpleState>, _event: ClickEvent) {
    state.state_mut().count += 1;
    state.request_rebuild();
}

fn build_ui(state: UIBuildArg<SimpleState>) -> BoxWidget<Container> {
    let mut x = Container::new([Box::new(TextButton::new(
        "Click me".to_string(),
        state.handler(on_btn_click),
    ))]);
    x.add(TextLine::new(format!("Count: {}", state.state().count)));
    x.add(TextLine::new("another line".to_string()));
    x.add(TextField::new(state.state().editing.clone()));
    x.add(TextButton::new(
        "A rather long test too check layout BLA BLA BLA".to_string(),
        |_| {},
    ));
    BoxWidget::new(x)
        .with_alignment(Alignment::Center, Alignment::Center)
        .with_padding(BoxPadding::all(5.))
        .with_border(BoxBorder(3., BLACK))
}

struct SimpleState {
    count: usize,
    editing: Rc<TextFieldController>,
}

pub fn run_dev() {
    let root = ReactiveUI::new(
        SimpleState {
            count: 0,
            editing: Rc::new(TextFieldController::new()),
        },
        build_ui,
    );
    LaunchConfig::default().with_title(file!()).launch(root);
}
