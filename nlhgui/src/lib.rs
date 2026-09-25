/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

//! # nlhgui
//! Cross-platform desktop GUI development using pure Rust \
//! Example:
//! ```
//! use nlhgui::LaunchConfig;
//! use nlhgui::widgets::reactive::*;
//! use nlhgui::widgets::*;
//! 
//! struct MyState {
//!     count: u32,
//! }
//! 
//! fn handle_click(arg: UIBuildArg<MyState>, _: ClickEvent) {
//!     arg.state_mut().count += 1;
//! 
//!     // The build_ui function is not being called on every frame. Instead, we explicitly state
//!     // if a rebuild is necessary.
//!     arg.request_rebuild();
//! }
//! 
//! fn build_ui(arg: UIBuildArg<MyState>) -> BoxWidget<Container> {
//!     BoxWidget::new(Container::new([
//!         Box::new(TextLine::new(format!(
//!             "Hello there! Count: {}",
//!             arg.state().count
//!         ))),
//!         Box::new(TextButton::new(
//!             "Click".to_string(),
//!             arg.handler(handle_click),
//!         )),
//!     ]))
//!     .with_padding(BoxPadding::all(10.))
//!     .with_alignment(Alignment::Center, Alignment::Center)
//! }
//! 
//! fn main() {
//!     // Prepare your UI state.
//!     let init_state = MyState { count: 0 };
//! 
//!     // Create a root widget
//!     let root = ReactiveUI::new(init_state, build_ui);
//! 
//!     // Launch the application.
//!     LaunchConfig::default()
//!         .with_title("simple example")
//!         .launch(root);
//! }
//! ```
//!
//! ## Where to start?
//! The example above shows how to launch an application. In the [`widgets`] module
//! you will find all the builtin widgets.

use std::{cell::RefCell, rc::Rc};

use log::debug;
use skia_safe::Canvas;

use crate::{
    events::EventHandling,
    primitives::Point,
    widgets::{
        ex::{GlobalBuildingContext, Widget},
        wtp::WidgetsToPrimitivesInterface,
    },
};

/// This module provides all the built-in widgets.
pub mod widgets;

/// This module provides a generic event handling utility so that
/// the core application logic does not depend on a specific backend
/// (OpenGL/Vulkan/Metal/DirectX) or window system.
/// **You usually don't need to access this module directly.**
pub mod events;

/// This module provides primitives that can be easily drawed. They
/// are usually being created by build (see trait [`widgets::ex::Widget`]).
/// **You usually don't need to access this module directly.**
pub mod primitives;

/// rendering utility, calls the `draw(...)` functions of the `Drawable` items
pub(crate) mod renderer;

/// This module handles the window and rendering backend (OpenGL, Vulkan ...).
/// **You usually don't need to access this module directly.**
pub mod wnd;

/// Color constants and helpers
pub mod colors;

fn maxf(a: f32, b: f32) -> f32 {
    assert!(a == a && b == b);
    if a > b { a } else { b }
}

fn minf(a: f32, b: f32) -> f32 {
    assert!(a == a && b == b);
    if a < b { a } else { b }
}

struct Launched {
    renderer: renderer::Renderer,
    wtp: WidgetsToPrimitivesInterface,
}

impl Launched {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.wtp.resize(width as f32, height as f32);
        self.rebuild();
    }

    pub fn render(&mut self, canvas: &Canvas) {
        if self.wtp.wants_rebuild() {
            self.rebuild();
        }
        self.renderer.render(canvas);
    }

    pub fn rebuild(&mut self) {
        let render_items = self.wtp.build();
        self.renderer.replace_items(render_items);
    }

    pub fn glb_ctx(&self) -> &GlobalBuildingContext {
        self.wtp.glb_ctx()
    }
}

/// This configuration struct is needed for launching an application.
/// Create it using [`LaunchConfig::default`]. Call [`LaunchConfig::launch`] after you are
/// done configuring.
pub struct LaunchConfig {
    wnd_title: String,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            wnd_title: String::from("nlhgui window"),
        }
    }
}

impl LaunchConfig {
    /// changes the title, e.g.
    /// `LaunchConfig::default().with_title("custom title").launch(...)"`
    pub fn with_title(mut self, title: &str) -> Self {
        self.wnd_title = title.to_string();
        self
    }

    /// Launches the application.
    /// - Creates the window
    /// - Loads renderer
    /// - Builds UI
    /// - Runs the event loop
    /// - **Note that this function might not return.**
    pub fn launch<R: Widget + 'static>(self, root: R) {
        debug!("Launching application");

        let mut window = wnd::create_window(&self.wnd_title).unwrap();

        let (width, height) = window.get_size();
        let wtp = widgets::wtp::WidgetsToPrimitivesInterface::new(
            width as f32,
            height as f32,
            Box::new(root),
        );

        let renderer = renderer::Renderer::create();

        let mut state = Launched { renderer, wtp };
        state.rebuild();

        let stater = Rc::new(RefCell::new(state));
        let stater1 = stater.clone();
        let stater2 = stater.clone();
        let stater3 = stater.clone();
        let stater4 = stater.clone();
        let stater5 = stater.clone();
        let stater6 = stater.clone();

        let mut events = EventHandling::new();

        events.on_render(move |canvas| {
            stater.borrow_mut().render(canvas);
        });
        events.on_resize(move |width, height| {
            stater1.borrow_mut().resize(width, height);
        });
        events.on_cursor_move(move |x, y| {
            stater2
                .borrow()
                .glb_ctx()
                .handle_cursor_movement(Some(Point(x as f32, y as f32)));
        });
        events.on_mouse_btn(move |down, btn| {
            stater3.borrow().glb_ctx().handle_mouse_btn(down, btn);
        });
        events.on_text_input(move |text| {
            stater4.borrow().glb_ctx().handle_text_input(text);
        });
        events.set_wants_redraw_answer(move || stater5.borrow().wtp.wants_rebuild());
        events.on_key_event(move |ev| {
            stater6.borrow().glb_ctx().handle_key_event(ev);
        });

        window.use_events(events);

        window.run();
    }
}
