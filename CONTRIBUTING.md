# Contributing
Thank you for your interest in contributing! Here are some examples on what you could do:

## Test the library
This project needs to be tested a lot before it can be considered *reliable*. On macOS, it hasn't even been tested at all. Whether you have a Mac or not, I would really appreciate if you just tested the library by running the "example" crate (`/example`) on your machine. Please report any difficulties or bugs.

## Implement features
Also have a look at the issues (those marked "good first issue").

You could, for example, write your own widget.

*How to do that?*

**Non-reactive widget**: Create a struct (e.g. in `/nlhgui/src/widgets/mod.rs`) and implement the `Widget` trait. Have a look at other widgets for inspiration. The `build` function just creates a bunch of *primitives* that can be rendered easily. Of course you can also create your own primitives by implementing the `Drawable` trait and accessing `skia_safe` directly.

**Reactive widget**: If your widget needs to react to events (click/hover) and update itself, you need to create your own `build` function and pass it to `ReactiveUI`. Have a look at the implementation of `TextButton` for inspiration.

## More
You could improve the documentation by just adding or modifying the code comments.
