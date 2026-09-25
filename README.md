# Desktop GUI library for Rust
This library is supposed to help you build powerful cross-platform GUI applications in Rust.

> [!WARNING]
> This library is not ready for production! macOS hasn't been tested at all. So far, there are only a few widgets.

## Example
```rust
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

fn main() {
    // Prepare your UI state.
    let init_state = MyState { count: 0 };

    // Create a root widget
    let root = ReactiveUI::new(init_state, build_ui);

    // Launch the application.
    LaunchConfig::default()
        .with_title("simple example")
        .launch(root);
}
```

So far, this library is not available on crates.io. To use it, reference this repository in your Cargo.toml:

`nlhgui = { git = "https://github.com/nleonh/nlhgui-rs.git" }`
## Features
The following lists are non-exhaustive.
### Features without known bugs
Please not that these features should not be considered "stable" (tested on Fedora Linux mostly).
- **Basics**
    - Compilation (tested on Fedora and Windows)
    - Window creation
    - Rendering using skia's OpenGL backend
- **Simplicity**
    - Code is very easy to read
    - No callback hell
    - No macros, just plain Rust
- **Widgets**
    - Vertical and horiztonal containers (see [example 4](example/src/e4_container.rs))
    - Layouting (center child, padding)
    - Text rendering, TextButton
    - Very basic text editing (missing some important features)
- **Reactive UI**
    - Triggering UI updates in event handlers: click/hover (see [quickstart](example/src/quickstart.rs))
    - Interacting with worker threads (see [example 3](example/src/e3_block_good.rs))
### Planned
- Use more modern skia backends, if available: Vulkan, Apple's Metal, maybe DirectX
- Common widgets: checkbox, menu bar, context menu
- SVG rendering
### Limitations
- Error handling: many `unwrap`s and `panic`s so far
## License

The library `nlhgui` is licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0/).

See the [LICENSE](LICENSE) file. This includes all the source files in `nlhgui/`, unless otherwise specified at the top of a file.

### License of examples

However, all the examples (in `example/`) are licensed under ["The Unlicense"](https://choosealicense.com/licenses/unlicense/) ([read terms here](example/UNLICENSE)).

This means, you are completely free to use the examples as a starting point for any project.

## Contribute

If youre interested in contributing head over to [CONTRIBUTING](CONTRIBUTING.md).
