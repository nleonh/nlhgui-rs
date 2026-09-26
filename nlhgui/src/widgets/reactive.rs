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

use log::warn;

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
pub struct UIBuildArg<S, C = (), E = ()>(Rc<UIBuildArgsData<S, C, E>>);

impl<S: 'static, C: 'static, E: 'static> UIBuildArg<S, C, E> {
    pub fn handler<CallbackArgument, F: 'static + Fn(Self, CallbackArgument) -> ()>(
        &self,
        handler: F,
    ) -> Box<dyn 'static + Fn(CallbackArgument) -> ()> {
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

struct ReactiveUIImpl<StateDataType, ConfigDataType, EventType> {
    // build: Box<BuildType<StateDataType, WidgetType>>,
    args: UIBuildArg<StateDataType, ConfigDataType, EventType>,
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

    pub(crate) fn to_build_arg<W: Widget>(&self) -> UIBuildArg<S, C, E> {
        UIBuildArg(self.args.0.clone())
    }
}

/// # Reactive UI
/// Example:
/// ```
/// use nlhgui::{*, widgets::{reactive::*, *}};
///
/// struct MyUIState {}
///
/// fn build_ui(arg: UIBuildArg<MyUIState>) -> Container {
///     Container::new([])
/// }
///
/// fn main() {
///     let root = ReactiveUI::new(MyUIState {}, build_ui);
///     LaunchConfig::default()
///         .launch(root);
/// }
/// ```
pub struct ReactiveUI<StateType, W: Widget, ConfigType = (), EventType = ()> {
    inner: ReactiveUIImpl<StateType, ConfigType, EventType>,
    widget: Option<W>,
    build: Box<UIBuildType<StateType, W, ConfigType, EventType>>,
    handle_ev: Box<dyn Fn(EventType, UIBuildArg<StateType, ConfigType, EventType>)>,
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

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        self.widget.as_ref().unwrap().build(target, ctx)
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        while let Some(ev) = self.inner.args.recv_task_event() {
            (*self.handle_ev)(ev, self.inner.to_build_arg::<W>());
        }
        if self.inner.args.0.needs_rebuild.get() {
            let tmp = &self.inner.args.0;
            let tmp2 = tmp.glb_ctx.borrow();
            let glb_ctx = tmp2.as_ref().unwrap();
            let mut widget = (*self.build)(self.inner.to_build_arg::<W>());
            widget.hello(glb_ctx);
            self.widget = Some(widget);
            tmp.needs_rebuild.set(false);
        }
        self.widget.as_mut().unwrap().layout(avl_sp)
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        *self.inner.args.0.glb_ctx.borrow_mut() = Some(ctx.clone());
    }
}
