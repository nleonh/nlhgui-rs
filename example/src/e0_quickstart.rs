// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use nlhgui::LaunchConfig;
use nlhgui::widgets::reactive::*;
use nlhgui::widgets::*;

struct MyState {
    count: u32,
}

fn handle_click(arg: UIBuildArg<MyState>, _: ClickEvent) {
    arg.state_mut().count += 1;

    // The build_ui function is not being called on every frame. Instead, we explicitly state
    // if a rebuild is necessary.
    arg.request_rebuild();
}

fn build_ui(arg: UIBuildArg<MyState>) -> BoxWidget<Container> {
    BoxWidget::new(Container::new([
        Box::new(TextLine::new(format!(
            "Hello there! Count: {}",
            arg.state().count
        ))),
        Box::new(TextButton::new(
            "Click".to_string(),
            arg.handler(handle_click),
        )),
    ]))
    .with_padding(BoxPadding::all(10.))
    .with_alignment(Alignment::Center, Alignment::Center)
}

// This would be your main function.
pub fn run_quickstart() {
    // Prepare your UI state.
    let init_state = MyState { count: 0 };

    // Create a root widget
    let root = ReactiveUI::new(init_state, build_ui);

    // Launch the application.
    LaunchConfig::default()
        .with_title("quickstart example")
        .launch(root);
}
