/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{cell::RefCell, rc::Rc, sync::{Arc, atomic::{self, AtomicBool}}};

use log::{debug, warn};
use skia_safe::{Color4f};
use winit::keyboard::NamedKey;

use crate::{
    events::{GenericKeyEvent, GenericMouseButton},
    primitives::{Drawable, Point},
    widgets::fonts::FontsModule,
};

#[derive(Clone, Debug)]
pub struct SelectedLayout {
    pub width: f32,
    pub height: f32,
}

pub enum CursorEvent<'a> {
    Entered,
    Left,
    /// inside the rect/area.
    /// Note that after Down and Left there will be no Up
    Down(&'a GenericMouseButton),
    /// (inside the rect/area)
    Up(&'a GenericMouseButton),
}

pub(crate) struct CursorSensitiveArea {
    handler: Box<dyn Fn(CursorEvent) -> ()>,
    begin_x: f32,
    begin_y: f32,
    end_x: f32,
    end_y: f32,
    currently_inside: bool,
}

impl CursorSensitiveArea {
    pub fn is_inside(&self, x: f32, y: f32) -> bool {
        x >= self.begin_x && x <= self.end_x && y >= self.begin_y && y <= self.end_y
    }
}

/// See [`GlobalBuildingContext::register_text_receiver`]
pub enum TextReceiverEvent {
    /// esc key or clicking outside the area
    LostFocus,
    /// Got some input
    Text(String),
    /// backspace or del key
    Delete,
}

pub(crate) enum ConcurrencySolution {
    None,
    Tokio(tokio::runtime::Runtime),
}

impl ConcurrencySolution {
    pub fn spawn_blocking<F>(&self, future: F) where F: 'static + Future + Send, F::Output: Send {
        match self {
            Self::None => panic!("No concurrency runtime"),
            Self::Tokio(t) => t.spawn(future)
        };
    }
}

pub(crate) struct GlobalBuildingContextData {
    pub cursor_areas: Vec<CursorSensitiveArea>,
    pub cursor_pos: Point,
    pub text_receiver: Option<Box<dyn Fn(TextReceiverEvent) -> ()>>,
    pub concurrency: ConcurrencySolution
}

impl GlobalBuildingContextData {
    pub fn renew_cursor_pos(&mut self) {
        for area in &mut self.cursor_areas {
            if area.is_inside(self.cursor_pos.0, self.cursor_pos.1) {
                if area.currently_inside {
                    // todo moved inside
                } else {
                    area.currently_inside = true;
                    (*area.handler)(CursorEvent::Entered);
                }
            } else if area.currently_inside {
                area.currently_inside = false;
                (*area.handler)(CursorEvent::Left);
            }
        }
    }
}

pub struct BuiltinStyleData {
    pub std_font_size: f32,
    pub dyn_hover_col: Color4f,
    pub dyn_std_col: Color4f,
    pub dyn_act_col: Color4f,
}

pub struct GlobalRebuildTrigger {
    rebuild_flag: Arc<AtomicBool>
}

impl GlobalRebuildTrigger {
    pub fn trigger_rebuild(&self) {
        self.rebuild_flag.store(true, atomic::Ordering::Relaxed);
    }
}

pub struct GlobalBuildingContext {
    pub(crate) data: RefCell<GlobalBuildingContextData>,
    pub(crate) fonts_mod: Option<FontsModule>,
    pub(crate) need_rebuild: Arc<AtomicBool>,
    pub builtin_style: BuiltinStyleData,
}

impl Default for GlobalBuildingContext {
    fn default() -> Self {
        debug!("Creating global building context");
        Self {
            need_rebuild: Arc::new(AtomicBool::new(false)),
            fonts_mod: Some(FontsModule::create()),
            data: RefCell::new(GlobalBuildingContextData {
                concurrency: ConcurrencySolution::None,
                cursor_areas: vec![],
                cursor_pos: Point(f32::MAX, f32::MAX), // outside actual window
                text_receiver: None,
            }),
            builtin_style: BuiltinStyleData {
                std_font_size: 16.,
                dyn_hover_col: Color4f {
                    r: 0.8,
                    b: 1.0,
                    g: 0.95,
                    a: 1.,
                },
                dyn_std_col: Color4f {
                    r: 0.85,
                    g: 0.85,
                    b: 0.85,
                    a: 0.85,
                },
                dyn_act_col: Color4f {
                    r: 0.8,
                    b: 1.0,
                    g: 0.8,
                    a: 1.,
                },
            },
        }
    }
}

impl GlobalBuildingContext {
    pub fn request_rebuild(&self) {
        self.need_rebuild.store(true, atomic::Ordering::Relaxed);
    }

    pub fn create_global_rebuild_trigger(&self) -> GlobalRebuildTrigger {
        GlobalRebuildTrigger { rebuild_flag: self.need_rebuild.clone() }
    }

    pub fn with_tokio_rt(self, rt: tokio::runtime::Runtime) -> Self {
        self.data.borrow_mut().concurrency = ConcurrencySolution::Tokio(rt);
        self
    }

    pub fn fonts(&self) -> &FontsModule {
        self.fonts_mod.as_ref().unwrap()
    }

    pub fn spawn_blocking<F>(&self, future: F) where F: 'static + Future + Send, F::Output: Send {
        self.data.borrow().concurrency.spawn_blocking(future);
    }

    pub fn register_cursor_sensitive_area<H: 'static + Fn(CursorEvent) -> ()>(
        &self,
        begin_x: f32,
        begin_y: f32,
        end_x: f32,
        end_y: f32,
        handler: H,
    ) {
        self.data
            .borrow_mut()
            .cursor_areas
            .push(CursorSensitiveArea {
                handler: Box::new(handler),
                begin_x,
                begin_y,
                end_x,
                end_y,
                currently_inside: false,
            });
    }

    pub fn handle_key_event(&self, ev: GenericKeyEvent) {
        if !(ev.alt || ev.ctrl) {
            if ev.key == NamedKey::Backspace || ev.key == NamedKey::Delete {
                let d = self.data.borrow();
                if let Some(recv) = d.text_receiver.as_ref() {
                    (**recv)(TextReceiverEvent::Delete);
                }
            }
        }
    }

    /// Registers a "text receiver", e.g. a text input widget.
    /// There can only be one receiver at a time. If there's one at the moment of this
    /// call, it will be replaced safely.
    pub fn register_text_receiver<H: Fn(TextReceiverEvent) + 'static>(&self, handler: H) {
        let mut x = self
            .data
            .borrow_mut()
            .text_receiver
            .replace(Box::new(handler));
        if let Some(x) = x.take() {
            (*x)(TextReceiverEvent::LostFocus);
        }
    }

    fn remove_current_text_receiver(&self) {
        if let Some(x) = self.data.borrow_mut().text_receiver.take() {
            (*x)(TextReceiverEvent::LostFocus);
        }
    }

    pub fn handle_mouse_btn(&self, down: bool, btn: GenericMouseButton) {
        let mut did = 0_u32;
        for a in &self.data.borrow().cursor_areas {
            if a.currently_inside {
                let event = if down {
                    CursorEvent::Down(&btn)
                } else {
                    CursorEvent::Up(&btn)
                };
                (*a.handler)(event);
                did += 1;
            }
        }
        if did == 0 {
            self.remove_current_text_receiver();
        }
    }

    pub fn handle_text_input(&self, input: String) {
        if let Some(h) = self.data.borrow().text_receiver.as_ref() {
            (**h)(TextReceiverEvent::Text(input));
        }
    }

    pub fn handle_cursor_movement(&self, n: Option<Point>) {
        let mut data_mut = self.data.borrow_mut();
        if let Some(n) = n {
            data_mut.cursor_pos = n;
        }
        data_mut.renew_cursor_pos();
    }

    pub fn before_build(&self) {
        self.data.borrow_mut().cursor_areas.clear();
        self.data.borrow_mut().text_receiver = None;
    }

    pub fn after_build(&self) {
        self.need_rebuild.store(false, atomic::Ordering::Relaxed);
        self.handle_cursor_movement(None);
    }
}

/// Specifies where to put the Drawables
#[derive(Debug)]
pub struct BuildingContext {
    pub x_begin: f32,
    pub y_begin: f32,
}

/// Returned by [`Widget::layout`]
#[derive(Debug, Clone)]
pub struct LayoutingResult {
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

impl LayoutingResult {
    pub fn fix(width: f32, height: f32) -> Self {
        Self {
            min_height: height,
            min_width: width,
            max_height: Some(height),
            max_width: Some(width),
        }
    }

    /// Check computed layout.
    /// If there's not enough space, it will produce a warning and shrink it.
    /// **This does not change the actual layout of the `Drawable`s!**
    pub fn checked(mut self, avl_sp: AvailableSpace) -> Self {
        let height_avl = avl_sp.height.unwrap_or(f32::MAX);
        let width_avl = avl_sp.width.unwrap_or(f32::MAX);
        if self.min_height > height_avl {
            warn!(
                "Minimum height ({}) too large ({} available)",
                self.min_height, height_avl
            );
            self.min_height = height_avl;
        }
        if self.min_width > width_avl {
            warn!(
                "Minimum width ({}) too large ({} available)",
                self.min_width, width_avl
            );
            self.min_width = width_avl;
        }
        self
    }
}

/// Used by [`Widget::layout`], e.g. in [`crate::widgets::TextField`]
#[derive(Clone, Debug)]
pub struct AvailableSpace {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl AvailableSpace {
    /// doesn't do anything if self has no width limit
    pub fn subtract_width(&mut self, x: f32) {
        if let Some(width) = self.width.as_mut() {
            *width -= x;
        }
    }

    /// doesn't do anything if self has no height limit
    pub fn subtract_height(&mut self, x: f32) {
        if let Some(height) = self.height.as_mut() {
            *height -= x;
        }
    }
}

pub trait Widget {
    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext);

    fn apply_layout(&mut self, layout: SelectedLayout);

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult;

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>);
}

/// less boilerplate, auto-implements Widget by just calling the child's methods
pub trait WrapperWidget {
    fn child<'a>(&'a self) -> &'a dyn Widget;
    fn child_mut<'a>(&'a mut self) -> &'a mut dyn Widget;
}

impl<T> Widget for T
where
    T: WrapperWidget,
{
    fn apply_layout(&mut self, layout: SelectedLayout) {
        self.child_mut().apply_layout(layout)
    }

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        self.child().build(target, ctx)
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.child_mut().hello(ctx)
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        self.child_mut().layout(avl_sp)
    }
}
