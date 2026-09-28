// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use nlhgui::Launcher;
use nlhgui::widgets::reactive::*;
use nlhgui::widgets::*;

struct MyState {
    count: u32,
}

fn handle_click(arg: UIBuildArg<MyState>, _: ClickEvent) {
    arg.state_mut().count += 1;

    // This tells nlhgui that build_ui has to be invoked again.
    arg.request_rebuild();
}

// This function is called on every rebuild. Hoever, it is *not* being called each time a new frame
// is being rendered to the screen.
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
    .with_alignment(Alignment::Center, Alignment::Center)
}

// This would be your main function.
pub fn run_quickstart() {
    // Prepare your UI state.
    let init_state = MyState { count: 0 };

    // Create a root widget. ReactiveUI should be the root of every UI that needs to update
    // itself. You can also wrap multiple instances of ReactiveUI's so that a little change
    // doesn't trigger a full rebuild.
    let root = ReactiveUI::new(init_state, build_ui);

    // Launch the application.
    Launcher::default()
        .with_title("quickstart example")
        .launch(root);
}
