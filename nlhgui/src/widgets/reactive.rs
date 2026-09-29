/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    rc::Rc,
    sync::mpsc::{Receiver, Sender, channel},
};

use log::{debug, warn};

use crate::{
    primitives::Drawable,
    widgets::{
        BuildingContext, GlobalBuildingContext, LayoutingResult, SelectedLayout, Widget,
        ex::{AvailableSpace, GlobalRebuildTrigger},
    },
};

type UIBuildType<StateDataType, W, C = (), E = ()> = dyn Fn(UIBuildArg<StateDataType, C, E>) -> W;

struct ConcurrencyData<E> {
    receiver: Receiver<E>,
    sender: Sender<E>,
}

impl<E> ConcurrencyData<E> {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<E>();
        Self { receiver, sender }
    }
}

struct UIBuildArgsData<S, C, E> {
    state: RefCell<S>,
    config: C,
    needs_rebuild: Cell<bool>,
    glb_ctx: RefCell<Option<Rc<GlobalBuildingContext>>>,
    concurrency: ConcurrencyData<E>,
}

pub struct ConcurrentTaskArg<EventType> {
    sender: Sender<EventType>,
    glb_build: GlobalRebuildTrigger,
}

impl<T> ConcurrentTaskArg<T> {
    /// Sending data automatically triggers UI updates
    pub fn send(&self, data: T) {
        self.glb_build.trigger_rebuild();
        self.sender.send(data).unwrap();
    }
}

/// UI build argument type: \
/// If your build function relies on a state of type `S` it gets an argument
/// of type `UIBuildArg<S>` instead of `&S` or `&mut S`. This also allows you
/// to explicitly specify whether a UI rebuild is necessary. If you change the
/// state (`state_mut()`) you usually also want to request a UI rebuild using
/// `request_rebuild()`.
#[derive(Clone)]
pub struct UIBuildArg<State, Config = (), Events = ()>(
    Rc<UIBuildArgsData<State, Config, Events>>,
);

impl<S: 'static, C: 'static, E: 'static> UIBuildArg<S, C, E> {
    pub fn handler<CallbackArgument, F: 'static + Fn(Self, CallbackArgument)>(
        &self,
        handler: F,
    ) -> Box<dyn 'static + Fn(CallbackArgument)> {
        let copy = self.0.clone();
        let handler = Box::new(handler);
        Box::new(move |d| {
            (*handler)(Self(copy.clone()), d);
        })
    }

    fn recv_task_event(&self) -> Option<E> {
        self.0.concurrency.receiver.try_recv().ok()
    }

    pub fn get_concurrent_task_ctx(&self) -> ConcurrentTaskArg<E> {
        ConcurrentTaskArg {
            sender: self.0.concurrency.sender.clone(),
            glb_build: self
                .0
                .glb_ctx
                .borrow()
                .as_ref()
                .unwrap()
                .create_global_rebuild_trigger(),
        }
    }

    pub fn run_blocking<F>(&self, future: F)
    where
        F: 'static + Send + Future,
        F::Output: Send,
    {
        self.glb_ctx().spawn_blocking(future);
    }

    /// Note that you **may not already have a mutable reference to the state**
    /// (acquired by calling `state()` or `state_mut()`), as mutable borrows are
    /// exclusive in Rust. This is checked at runtime.
    pub fn state<'a>(&'a self) -> Ref<'a, S> {
        self.0.state.borrow()
    }

    pub fn config(&self) -> &C {
        &self.0.config
    }

    pub fn glb_ctx(&self) -> Rc<GlobalBuildingContext> {
        self.0.glb_ctx.borrow().as_ref().unwrap().clone()
    }

    /// Changing the state using this function **does not request a UI rebuild!** To
    /// do that you need to call `request_rebuild()`. Note that you **may not
    /// already have a reference to the state** (acquired by calling `state()` or
    /// `state_mut()`), as mutable borrows are exclusive in Rust. This is checked
    /// at runtime.
    pub fn state_mut<'a>(&'a self) -> RefMut<'a, S> {
        self.0.state.borrow_mut()
    }

    pub fn request_rebuild(&self) {
        let x = self.0.glb_ctx.borrow();
        let glb_ctx = x.as_ref().unwrap();
        glb_ctx.request_rebuild();
        self.0.needs_rebuild.set(true);
    }
}

struct ReactiveUIImpl<S, C, E> {
    args: UIBuildArg<S, C, E>,
}

impl<S, C, E> ReactiveUIImpl<S, C, E> {
    pub(crate) fn new(state: S, config: C) -> Self {
        Self {
            args: UIBuildArg(Rc::new(UIBuildArgsData {
                needs_rebuild: Cell::new(true),
                state: RefCell::new(state),
                config,
                glb_ctx: RefCell::new(None),
                concurrency: ConcurrencyData::new(),
            })),
        }
    }

    pub(crate) fn to_build_arg(&self) -> UIBuildArg<S, C, E> {
        UIBuildArg(self.args.0.clone())
    }
}

type HandleEvFnT<E, S, C> = dyn Fn(E, UIBuildArg<S, C, E>);

/// # Reactive UI
/// Example:
/// ```
/// use nlhgui::Launcher;
/// use nlhgui::widgets::reactive::*;
/// use nlhgui::widgets::*;
///
/// struct MyState {
///     count: u32,
/// }
///
/// fn handle_click(arg: UIBuildArg<MyState>, _: ClickEvent) {
///     arg.state_mut().count += 1;
///
///     // The build_ui function is not being called on every frame. Instead, we explicitly state
///     // if a rebuild is necessary.
///     arg.request_rebuild();
/// }
///
/// fn build_ui(arg: UIBuildArg<MyState>) -> BoxWidget<Container> {
///     BoxWidget::new(Container::new([
///         Box::new(TextLine::new(format!(
///             "Hello there! Count: {}",
///             arg.state().count
///         ))),
///         Box::new(TextButton::new(
///             "Click".to_string(),
///             arg.handler(handle_click),
///         )),
///     ]))
///     .with_padding(BoxPadding::all(10.))
///     .with_alignment(Alignment::Center, Alignment::Center)
/// }
///
/// // This would be your main function.
/// fn main() {
///     // Prepare your UI state.
///     let init_state = MyState { count: 0 };
///
///     // Create a root widget
///     let root = ReactiveUI::new(init_state, build_ui);
///
///     // Launch the application.
///     Launcher::default()
///         .with_title("quickstart example")
///         .launch(root);
/// }
/// ```
pub struct ReactiveUI<StateType, W: Widget, ConfigType = (), EventType = ()> {
    inner: ReactiveUIImpl<StateType, ConfigType, EventType>,
    build: Box<UIBuildType<StateType, W, ConfigType, EventType>>,
    handle_ev: Box<HandleEvFnT<EventType, StateType, ConfigType>>,
    widget: Option<W>,
}

impl<S, W: Widget, E> ReactiveUI<S, W, (), E> {
    /// Creates a reactive widget using a build function and an initial state
    pub fn new<B: 'static + Fn(UIBuildArg<S, (), E>) -> W>(state: S, build: B) -> Self {
        Self::new_with_config(state, build, ())
    }
}

impl<S, C, W: Widget, E> ReactiveUI<S, W, C, E> {
    /// Creates a reactive widget using a build function and an initial state
    pub fn new_with_config<B: 'static + Fn(UIBuildArg<S, C, E>) -> W>(
        state: S,
        build: B,
        config: C,
    ) -> Self {
        let inner = ReactiveUIImpl::<S, C, E>::new(state, config);
        let build = Box::new(build);
        Self {
            build,
            widget: None,
            inner,
            handle_ev: Box::new(|_, _| warn!("Task event not handled")),
        }
    }

    pub fn with_event_handler<H: 'static + Fn(E, UIBuildArg<S, C, E>)>(
        mut self,
        handler: H,
    ) -> Self {
        self.handle_ev = Box::new(handler);
        self
    }

    /// **Borrow checking is done at runtime**
    pub fn state(&self) -> Ref<'_, S> {
        self.inner.args.0.state.borrow()
    }

    /// **Borrow checking is done at runtime**
    pub fn state_mut(&self) -> RefMut<'_, S> {
        self.inner.args.0.state.borrow_mut()
    }
}

impl<S: 'static, C: 'static, W: 'static + Widget, E: 'static> Widget for ReactiveUI<S, W, C, E> {
    fn apply_layout(&mut self, layout: SelectedLayout) {
        self.widget.as_mut().unwrap().apply_layout(layout)
    }

    fn build(
        &mut self,
        target: &mut Vec<Box<dyn Drawable>>,
        ctx: BuildingContext,
        glb_ctx: &Rc<GlobalBuildingContext>,
    ) {
        self.widget.as_mut().unwrap().build(target, ctx, glb_ctx)
    }

    fn layout(
        &mut self,
        avl_sp: AvailableSpace,
        glb_ctx: &Rc<GlobalBuildingContext>,
    ) -> LayoutingResult {
        self.inner.args.0.glb_ctx.replace(Some(glb_ctx.clone()));
        while let Some(ev) = self.inner.args.recv_task_event() {
            (*self.handle_ev)(ev, self.inner.to_build_arg());
        }
        if self.inner.args.0.needs_rebuild.get() {
            let tmp = &self.inner.args.0;
            // Don't delete: Helps detecting bugs related to too many redraws
            debug!("Rebuilding reactive UI tree");
            let widget = (*self.build)(self.inner.to_build_arg());
            self.widget = Some(widget);
            tmp.needs_rebuild.set(false);
        }
        self.widget.as_mut().unwrap().layout(avl_sp, glb_ctx)
    }
}
