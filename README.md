# Desktop GUI library for Rust
This library is supposed to help you build powerful cross-platform GUI applications in Rust.

> [!WARNING]
> This library is not ready for production! macOS hasn't been tested at all. So far, there are only a few widgets.

## Example
The following code creates a window with a line of text containing the value of a mutable counter and a button. Clicking the button
will increase the counter and update the status text.

```rust
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
fn main() {
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

```

So far, this library is not available on crates.io. To use it, reference this repository in your Cargo.toml:

`nlhgui = { git = "https://github.com/nleonh/nlhgui-rs.git" }`
## Features
Please not that these features should **not** be considered *stable* (tested on Fedora Linux mostly).
- ***Basics***
    - Compilation (tested on Fedora and Windows)
    - Window creation ([all examples](example/src))
    - Rendering using skia's **OpenGL** and **Vulkan** backends
- ***Simplicity***
    - Code is **very easy to read**
    - No callback hell
    - No macros, **pure Rust** (on top of **Skia** which is written in C++)
    - **Easy to extend** (especially writing your own widgets)
- ***Widgets***
    - Vertical and horiztonal **containers** (see [example 4](example/src/e4_container.rs))
    - **Layouting** (center child, padding)
    - **Text rendering** (including multi-line with automatic line wraps, see [example 6](example/src/e6_text.rs)), TextButton
    - **Radio buttons, checkboxes** (see [example 5](example/src/e5_input.rs))
    - Very basic **text editing**, though missing some important features (see [example 5](example/src/e5_input.rs))
- ***Reactive UI***
    - Triggering UI updates in event handlers: click/hover (see [quickstart](example/src/quickstart.rs))
    - Interacting with worker threads (see [example 3](example/src/e3_block_good.rs))
### Planned
- Avoid OpenGL, use Metal/DirectX if possible, improve Vulkan support
- Theming
- Common widgets: menu bar, context menu
- SVG rendering
### Current limitations
- Error handling: many `unwrap`s and `panic`s so far
- No accessibilty support
- No support for managing multiple windows
## License

The library `nlhgui` is licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0/).

See the [LICENSE](LICENSE) file. This includes all the source files in `nlhgui/`, unless otherwise specified at the top of a file.

### License of examples

However, all the examples (in `example/`) are licensed under ["The Unlicense"](https://choosealicense.com/licenses/unlicense/) ([read terms here](example/UNLICENSE)).

This means, you are completely free to use the examples as a starting point for any project.

## Contribute

If you are interested in contributing head over to [CONTRIBUTING](CONTRIBUTING.md).
