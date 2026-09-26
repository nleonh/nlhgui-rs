/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{cell::RefCell, rc::Rc};

use crate::{
    events::GenericMouseButton,
    maxf,
    primitives::{self, Point, TextBlobDrawable, create_border_path},
    widgets::{
        ex::{
            AvailableSpace, BuildingContext, CursorEvent, GlobalBuildingContext, LayoutingResult,
            SelectedLayout, Widget, WrapperWidget,
        },
        reactive::{ReactiveUI, UIBuildArg},
    },
};
use primitives::Drawable;
use skia_safe::{Font, TextBlob};

/// This module allows you to make your UI reactive. This usually includes handling
/// events (e.g. button click), changing state and requesting UI rebuilds.
pub mod reactive;

/// This is needed for creating your own custom widgets by implementing the
/// `Widget` trait.
pub mod ex;

/// Used for preloading fonts. **You usually don't need to use this module
/// directly.**
pub mod fonts;

pub(crate) mod wtp;

mod input;
pub use input::*;

mod select;
pub use select::*;

/// Literally just a rect.
pub struct Rect {
    width: f32,
    height: f32,
    color: Color,
}

impl Rect {
    pub fn new(width: f32, height: f32, color: Color) -> Self {
        Self {
            width,
            height,
            color,
        }
    }
}

impl Widget for Rect {
    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        let rect = primitives::Rect::new(
            ctx.x_begin,
            ctx.y_begin,
            self.width,
            self.height,
            self.color,
        );
        target.push(Box::new(rect));
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        LayoutingResult::fix(self.width, self.height).checked(avl_sp)
    }

    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn hello(&mut self, _ctx: &Rc<GlobalBuildingContext>) {}
}

struct ContainerChildLayout {
    selected_height: f32,
    selected_width: f32,
    layout: LayoutingResult,
}

/// Arranges multiple child widgets.
pub struct Container {
    children: Vec<Box<dyn Widget>>,
    child_layout_data: Vec<ContainerChildLayout>,
    spacing: f32,
    horizontal: bool,
}

impl Container {
    /// default spacing: 5, default alignment: horizontally
    pub fn new<const N: usize>(children: [Box<dyn Widget>; N]) -> Self {
        Self {
            child_layout_data: vec![],
            children: Vec::from(children),
            spacing: 5.,
            horizontal: false,
        }
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.set_spacing(spacing);
        self
    }

    pub fn set_spacing(&mut self, spacing: f32) {
        self.spacing = spacing;
    }

    pub fn add<T: 'static + Widget>(&mut self, new_child: T) {
        self.children.push(Box::new(new_child));
    }

    pub fn make_horizontal(&mut self) {
        self.horizontal = true;
    }

    pub fn horizontal(mut self) -> Self {
        self.make_horizontal();
        self
    }
}

impl Widget for Container {
    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        for c in &mut self.children {
            c.hello(ctx);
        }
    }

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        let mut align_count = 0.;
        for (i, c) in self.children.iter().enumerate() {
            let mut ctx = BuildingContext {
                x_begin: ctx.x_begin,
                y_begin: ctx.y_begin,
            };
            if self.horizontal {
                ctx.x_begin += align_count;
            } else {
                ctx.y_begin += align_count;
            }
            
            c.build(target, ctx);
            align_count += self.child_layout_data[i].selected_height + self.spacing;
        }
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        self.child_layout_data.clear();
        let mut minw = 0.;
        let mut minh = 0.;
        let mut maxw = Some(0.);
        let mut maxh = Some(0.);
        for c in &mut self.children {
            let c_layout = c.layout(avl_sp.clone());

            if self.horizontal {
                minh = maxf(minh, c_layout.min_height);
                minw += c_layout.min_width + self.spacing;
            } else {
                minw = maxf(minw, c_layout.min_width);
                minh += c_layout.min_height + self.spacing;
            }

            if let Some(maxw_c) = c_layout.max_width {
                if let Some(maxw) = &mut maxw {
                    if self.horizontal {
                        *maxw += maxw_c + self.spacing;
                    } else {
                        *maxw = maxf(*maxw, maxw_c)
                    }
                }
            } else {
                maxw = None;
            }

            if let Some(maxh_c) = c_layout.max_height {
                if let Some(maxh) = &mut maxh {
                    if self.horizontal {
                        *maxh = maxf(*maxh, maxh_c);
                    } else {
                        *maxh += maxh_c + self.spacing;
                    }
                }
            } else {
                maxh = None;
            }

            self.child_layout_data.push(ContainerChildLayout {
                // initialized in apply_layout
                selected_height: 0.,
                selected_width: 0.,
                layout: c_layout,
            });
        }

        if let Some(maxh) = &mut maxh {
            *maxh -= self.spacing;
        }
        minh -= self.spacing;

        LayoutingResult {
            min_width: minw,
            min_height: minh,
            max_width: maxw,
            max_height: maxh,
        }
        .checked(avl_sp)
    }

    fn apply_layout(&mut self, _layout: SelectedLayout) {
        // todo

        for (i, child) in &mut self.children.iter_mut().enumerate() {
            let cl = &mut self.child_layout_data[i];
            cl.selected_height = cl.layout.min_height;
            cl.selected_width = cl.layout.min_width;
            child.apply_layout(SelectedLayout {
                width: cl.selected_width,
                height: cl.selected_height,
            });
        }
    }
}

impl WrapperWidget for Box<dyn Widget> {
    fn child<'a>(&'a self) -> &'a dyn Widget {
        self.as_ref()
    }

    fn child_mut<'a>(&'a mut self) -> &'a mut dyn Widget {
        self.as_mut()
    }
}

/// Currently, this is just a placeholder
pub struct ClickEvent {}

/// Callback functions for [`CursorReactiveBox`]
pub struct CursorReactiveBoxHandlers {
    pub on_hover: Box<dyn Fn(())>,
    pub on_hover_end: Box<dyn Fn(())>,
    pub on_clicked: Box<dyn Fn(ClickEvent)>,
}

struct CursorReactiveInnerState {
    handlers: CursorReactiveBoxHandlers,
    is_down_inside: RefCell<bool>,
}

/// In most cases, [`TextButton`] or [`Button<T>`] will get the job done.
pub struct CursorReactiveBox<T: Widget> {
    child: T,
    state: Rc<CursorReactiveInnerState>,
    glb_ctx: Option<Rc<GlobalBuildingContext>>,
    layout: SelectedLayout,
}

impl<T: Widget> CursorReactiveBox<T> {
    // todo replace curr_down_inside with state: either hovered or clicked
    pub fn new(child: T, handlers: CursorReactiveBoxHandlers, curr_down_inside: bool) -> Self {
        Self {
            child,
            state: Rc::new(CursorReactiveInnerState {
                handlers,
                is_down_inside: RefCell::new(curr_down_inside),
            }),
            glb_ctx: None,
            layout: SelectedLayout {
                width: 0.,
                height: 0.,
            },
        }
    }
}

impl<T: Widget> Widget for CursorReactiveBox<T> {
    fn apply_layout(&mut self, layout: SelectedLayout) {
        self.layout = layout.clone();
        self.child.apply_layout(layout)
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.glb_ctx = Some(ctx.clone());
        self.child.hello(ctx)
    }

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        let glb_ctx = self.glb_ctx.as_ref().unwrap();
        let statec = self.state.clone();
        glb_ctx.register_cursor_sensitive_area(
            ctx.x_begin,
            ctx.y_begin,
            ctx.x_begin + self.layout.width,
            ctx.y_begin + self.layout.height,
            move |event| match event {
                CursorEvent::Entered => {
                    // Important: If a rebuild is triggered by a handler, this CursorEvent
                    // might be emitted multiple times. maybe TODO
                    if !*statec.is_down_inside.borrow() {
                        (*statec.handlers.on_hover)(())
                    }
                }
                CursorEvent::Left => {
                    *statec.is_down_inside.borrow_mut() = false;
                    (*statec.handlers.on_hover_end)(())
                }
                CursorEvent::Down(b) => {
                    if *b == GenericMouseButton::Left {
                        *statec.is_down_inside.borrow_mut() = true;
                    }
                }
                CursorEvent::Up(b) => {
                    if *b == GenericMouseButton::Left {
                        if *statec.is_down_inside.borrow() {
                            (*statec.handlers.on_clicked)(ClickEvent {});
                        }
                    }
                }
            },
        );
        self.child.build(target, ctx)
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        self.child.layout(avl_sp)
    }
}

struct ButtonState {
    click_handler: Rc<dyn Fn(ClickEvent) -> ()>,
    hovered: bool,
}

struct TextButtonState {
    text: String,
    btn_state: ButtonState,
}

/// Use [`BoxWidget::with_padding`]
pub struct BoxPadding {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl BoxPadding {
    pub fn all(all: f32) -> Self {
        Self {
            top: all,
            bottom: all,
            left: all,
            right: all,
        }
    }

    pub fn no_padding() -> Self {
        Self::all(0.)
    }
}

/// Use [`BoxWidget::with_border`]
pub struct BoxBorder(pub f32, pub Color);

/// RGBA color
pub type Color = skia_safe::Color4f;

/// Use [`BoxWidget`] to align widgets
#[derive(PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

/// Contains one widget. Use this wrapper for styling and layouting.
pub struct BoxWidget<C: Widget> {
    child: C,
    padding: BoxPadding,
    own_layout: Option<SelectedLayout>,
    child_layouting_res: Option<LayoutingResult>,
    bg_color: Option<Color>,
    x_align: Alignment,
    y_align: Alignment,
    max_own_height: bool,
    max_own_width: bool,
    child_offset: Option<Point>,
    child_layout: Option<SelectedLayout>,
    border: Option<BoxBorder>,
}

impl<C: Widget> BoxWidget<C> {
    /// no padding, no background color, no border so far.
    /// Aligned at top & left
    pub fn new(child: C) -> Self {
        Self {
            child,
            padding: BoxPadding::no_padding(),
            child_layouting_res: None,
            child_layout: None,
            own_layout: None,
            bg_color: None,
            x_align: Alignment::Start,
            y_align: Alignment::Start,
            max_own_width: false,
            max_own_height: false,
            child_offset: None,
            border: None,
        }
    }
}

impl<C: Widget> BoxWidget<C> {
    pub fn set_alignment(&mut self, x_align: Alignment, y_align: Alignment) {
        self.x_align = x_align;
        if self.x_align != Alignment::Start {
            self.max_own_width = true;
        }

        self.y_align = y_align;
        if self.x_align != Alignment::Start {
            self.max_own_height = true;
        }
    }

    pub fn with_alignment(mut self, x_align: Alignment, y_align: Alignment) -> Self {
        self.set_alignment(x_align, y_align);
        self
    }

    pub fn set_padding(&mut self, padding: BoxPadding) {
        self.padding = padding;
    }

    pub fn with_padding(mut self, padding: BoxPadding) -> Self {
        self.set_padding(padding);
        self
    }

    pub fn set_bg_color(&mut self, color: Color) {
        self.bg_color = Some(color)
    }

    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.set_bg_color(color);
        self
    }

    pub fn set_border(&mut self, border: BoxBorder) {
        self.border = Some(border);
    }

    pub fn with_border(mut self, border: BoxBorder) -> Self {
        self.set_border(border);
        self
    }

    fn compute_offset(
        a: &Alignment,
        min_before: f32,
        min_after: f32,
        child_size: f32,
        own_size: f32,
    ) -> f32 {
        match a {
            Alignment::Center => (own_size - child_size) / 2.,
            Alignment::End => min_after,
            Alignment::Start => min_before,
        }
    }
}

impl<C: Widget> Widget for BoxWidget<C> {
    fn apply_layout(&mut self, layout: SelectedLayout) {
        let border_thickness = self.border.as_ref().map(|x| x.0).unwrap_or(0.);
        let dx = self.padding.left + self.padding.right + border_thickness * 2.;
        let dy = self.padding.top + self.padding.bottom + border_thickness * 2.;

        let mut c_sel_layout = SelectedLayout {
            width: layout.width - dx,
            height: layout.height - dy,
        };

        let c_layout = self.child_layouting_res.take().unwrap();
        assert!(
            !(c_sel_layout.width < c_layout.min_width || c_sel_layout.height < c_layout.min_height)
        );
        if c_sel_layout.width > c_layout.max_width.unwrap_or(f32::MAX) {
            c_sel_layout.width = c_layout.max_width.unwrap();
        }
        if c_sel_layout.height > c_layout.max_height.unwrap_or(f32::MAX) {
            c_sel_layout.height = c_layout.max_height.unwrap();
        }

        let offset_x = Self::compute_offset(
            &self.x_align,
            self.padding.left + border_thickness,
            self.padding.right + border_thickness,
            c_sel_layout.width,
            layout.width,
        );
        let offset_y = Self::compute_offset(
            &self.y_align,
            self.padding.top + border_thickness,
            self.padding.bottom + border_thickness,
            c_sel_layout.height,
            layout.height,
        );

        self.child_layout = Some(c_sel_layout.clone());
        self.child.apply_layout(c_sel_layout);
        self.child_offset = Some(Point(offset_x, offset_y));
        self.own_layout = Some(layout);
    }

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, mut ctx: BuildingContext) {
        let offset = self.child_offset.as_ref().unwrap();
        ctx.x_begin += offset.0;
        ctx.y_begin += offset.1;

        let child_layout = self.child_layout.as_ref().unwrap();
        let inner_width = child_layout.width + self.padding.left + self.padding.right;
        let inner_height = child_layout.height + self.padding.top + self.padding.bottom;
        if let Some(bg_col) = &self.bg_color {
            target.push(Box::new(primitives::Rect::new(
                ctx.x_begin - self.padding.left,
                ctx.y_begin - self.padding.right,
                inner_width,
                inner_height,
                *bg_col,
            )));
        }

        if let Some(border) = &self.border {
            target.push(Box::new(create_border_path(
                inner_width,
                inner_height,
                border.0,
                Point(
                    ctx.x_begin - self.padding.left,
                    ctx.y_begin - self.padding.top,
                ),
                border.1,
            )));
        }

        self.child.build(
            target,
            BuildingContext {
                x_begin: ctx.x_begin,
                y_begin: ctx.y_begin,
            },
        );
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.child.hello(ctx);
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        let border = self.border.as_ref().map(|x| x.0).unwrap_or(0.);
        let own_y = self.padding.bottom + self.padding.top + border * 2.;
        let own_x = self.padding.left + self.padding.right + border * 2.;
        let mut avl_sp_c = avl_sp.clone();
        avl_sp_c.subtract_height(own_y);
        avl_sp_c.subtract_width(own_x);

        let mut layout = self.child.layout(avl_sp_c);
        self.child_layouting_res = Some(layout.clone());
        layout.min_width += own_x;
        if self.max_own_width {
            layout.max_width = Some(avl_sp.width.unwrap());
        } else {
            if let Some(max_width) = &mut layout.max_width {
                *max_width += own_x;
            }
        }
        layout.min_height += own_y;
        if self.max_own_height {
            layout.max_height = Some(avl_sp.height.unwrap());
        } else {
            if let Some(max_height) = &mut layout.max_height {
                *max_height += own_y;
            }
        }
        layout.checked(avl_sp)
    }
}

/// Just a single line of text (no auto-wrapping).
/// Don't pass strings containing \n as they will
/// not be rendered correctly.
pub struct TextLine {
    data: String,
    blob: Option<TextBlob>,
    ctx: Option<Rc<GlobalBuildingContext>>,
    offset: Option<(f32, f32)>,
}

impl TextLine {
    pub fn new(data: String) -> Self {
        Self {
            data,
            blob: None,
            ctx: None,
            offset: None,
        }
    }
}

impl Widget for TextLine {
    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        let blob = self.blob.as_ref().unwrap();
        let offset = self.offset.as_ref().unwrap();
        let pos = Point(ctx.x_begin - offset.0, ctx.y_begin - offset.1);
        target.push(Box::new(TextBlobDrawable::new(blob.clone(), pos)));
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.ctx = Some(ctx.clone());
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        let typeface = self.ctx.as_ref().unwrap().fonts().std_typeface();
        let font = Font::from_typeface(
            typeface,
            Some(self.ctx.as_ref().unwrap().builtin_style.std_font_size as f32),
        );
        let tb = TextBlob::from_str(&self.data, &font).unwrap();

        let bounds = tb.bounds();
        let width = bounds.width();
        let height = bounds.height();
        self.offset = Some((bounds.left, bounds.top));
        self.blob = Some(tb);
        LayoutingResult {
            min_width: width,
            min_height: height,
            max_width: Some(width),
            max_height: Some(height),
        }
        .checked(avl_sp)
    }
}
