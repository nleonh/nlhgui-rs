/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use skia_safe::{
    Paint, PaintStyle, PathBuilder,
    colors::{BLACK, BLUE},
};

use crate::{
    primitives::PathDrawable,
    widgets::{
        Container, CursorReactiveBox, CursorReactiveBoxHandlers, TextLine,
        reactive::{ReactiveUI, UIBuildArg},
    },
};

use crate::widgets::ex::*;

/// Group of radio buttons (select exactly one out of multiple options)
pub struct RadioButtonGroup {
    child: ReactiveUI<RadioButtonGroupState, Container, Rc<RadioButtonGroupCtrl>>,
}

struct RadioButtonGroupState {
    hovered: Option<usize>,
}

#[derive(PartialEq, Copy, Clone)]
struct SelectableItemState {
    selected: bool,
    hovered: bool,
}

struct RadioClickField(SelectableItemState);

impl Widget for RadioClickField {
    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn build(&mut self, target: &mut Vec<Box<dyn crate::primitives::Drawable>>, ctx: BuildingContext, _glb_ctx: &Rc<GlobalBuildingContext>) {
        let mut p = PathBuilder::new();
        let x0 = ctx.x_begin;
        let y0 = ctx.y_begin;
        p.move_to((x0, y0));
        p.line_to((x0 + 16., y0));
        p.line_to((x0 + 16., y0 + 16.));
        p.line_to((x0, y0 + 16.));

        if self.0.selected {
            p.line_to((x0, y0));
            p.line_to((x0 + 16., y0 + 16.));
            p.move_to((x0 + 16., y0));
            p.line_to((x0, y0 + 16.));
        } else {
            p.close();
        }

        let mut paint = Paint::new(
            // if selected, we don't want a hover animation (not unselectedable)
            if !self.0.selected && self.0.hovered {
                BLUE
            } else {
                BLACK
            },
            None,
        );
        paint.set_stroke_width(2.);
        paint.set_style(PaintStyle::Stroke);

        target.push(Box::new(PathDrawable(p.snapshot(), paint)));
    }

    fn layout(&mut self, avl_sp: AvailableSpace, _glb_ctx: &Rc<GlobalBuildingContext>) -> LayoutingResult {
        LayoutingResult::fix(20., 20.).checked(avl_sp)
    }
}

fn build_radio_click_field(
    arg: &UIBuildArg<RadioButtonGroupState, Rc<RadioButtonGroupCtrl>>,
    state: SelectableItemState,
    index: usize,
) -> CursorReactiveBox<RadioClickField> {
    CursorReactiveBox::new(
        RadioClickField(state),
        CursorReactiveBoxHandlers {
            on_clicked: Box::new(arg.handler(move |arg, _| {
                arg.config().selected.set(index);
                arg.request_rebuild();
            })),
            on_hover: Box::new(arg.handler(move |arg, _| {
                arg.state_mut().hovered = Some(index);
                arg.request_rebuild();
            })),
            on_hover_end: Box::new(arg.handler(move |arg, _| {
                arg.state_mut().hovered = None;
                arg.request_rebuild();
            })),
            on_mv_inside: None,
        },
        state.hovered,
    )
}

fn build_radio_item(
    arg: &UIBuildArg<RadioButtonGroupState, Rc<RadioButtonGroupCtrl>>,
    item_state: SelectableItemState,
    txt: String,
    index: usize,
) -> Container {
    let mut c = Container::new([]).horizontal();
    c.add(build_radio_click_field(arg, item_state, index));
    c.add(TextLine::new(txt));
    c
}

fn build_radio_btn_group(
    arg: UIBuildArg<RadioButtonGroupState, Rc<RadioButtonGroupCtrl>>,
) -> Container {
    let hovered = arg.state().hovered.unwrap_or(usize::MAX);
    let selected = arg.config().selected.get();
    let n_items = arg.config().texts.borrow().len();
    assert!(selected <= n_items);
    let mut container = Container::new([]);
    for (i, txt) in arg.config().texts.borrow().iter().enumerate() {
        let selected = selected == i;
        // If selected, we don't want a hover animation
        let hovered = hovered == i;
        container.add(build_radio_item(
            &arg,
            SelectableItemState { selected, hovered },
            txt.clone(),
            i,
        ));
    }
    container
}

impl RadioButtonGroup {
    pub fn new(ctrl: Rc<RadioButtonGroupCtrl>) -> Self {
        Self {
            child: ReactiveUI::new_with_config(
                RadioButtonGroupState { hovered: None },
                build_radio_btn_group,
                ctrl,
            ),
        }
    }
}

impl WrapperWidget for RadioButtonGroup {
    fn child(&self) -> &dyn super::ex::Widget {
        &self.child
    }

    fn child_mut(&mut self) -> &mut dyn super::ex::Widget {
        &mut self.child
    }
}

/// Controller for [`RadioButtonGroup`] (saves state across rebuilds)
pub struct RadioButtonGroupCtrl {
    selected: Cell<usize>,
    texts: RefCell<Vec<String>>,
}

impl RadioButtonGroupCtrl {
    /// Expects a minimum of two options and `selected < options.len()`
    pub fn new<T: ToString, const NUM_OPTIONS: usize>(
        options: [T; NUM_OPTIONS],
        selected: usize,
    ) -> Self {
        let texts: Vec<String> = options.map(|x| x.to_string()).to_vec();
        assert!(texts.len() > 1 && selected <= texts.len());
        Self {
            texts: RefCell::new(texts),
            selected: Cell::new(selected),
        }
    }

    pub fn get_selected(&self) -> usize {
        self.selected.get()
    }
}

struct CheckboxState {
    hovered: bool,
}

/// Controller for [`Checkbox`] (remembers state across rebuilds)
pub struct CheckboxController {
    ticked: Cell<bool>,
    label: String,
}

impl CheckboxController {
    pub fn new(label: String, ticked: bool) -> Self {
        Self {
            label,
            ticked: Cell::new(ticked),
        }
    }

    pub fn is_ticked(&self) -> bool {
        self.ticked.get()
    }
}

/// Options to tick ("yes") or don't tick ("no")
pub struct Checkbox {
    child: ReactiveUI<CheckboxState, Container, Rc<CheckboxController>>,
}

impl WrapperWidget for Checkbox {
    fn child(&self) -> &dyn Widget {
        &self.child
    }

    fn child_mut(&mut self) -> &mut dyn Widget {
        &mut self.child
    }
}

struct CheckboxButton(SelectableItemState);

impl Widget for CheckboxButton {
    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn build(&mut self, target: &mut Vec<Box<dyn crate::primitives::Drawable>>, ctx: BuildingContext, _glb_ctx: &Rc<GlobalBuildingContext>) {
        let mut p = PathBuilder::new();
        let x0 = ctx.x_begin;
        let y0 = ctx.y_begin;
        p.move_to((x0, y0));
        p.line_to((x0 + 16., y0));
        p.line_to((x0 + 16., y0 + 16.));
        p.line_to((x0, y0 + 16.));

        if self.0.selected {
            p.line_to((x0, y0));
            p.move_to((x0 + 1., y0 + 9.));
            p.line_to((x0 + 8., y0 + 14.));
            p.line_to((x0 + 15., y0));
        } else {
            p.close();
        }

        let mut paint = Paint::new(if self.0.hovered { BLUE } else { BLACK }, None);
        paint.set_stroke_width(2.);
        paint.set_style(PaintStyle::Stroke);

        paint.set_anti_alias(true);

        target.push(Box::new(PathDrawable(p.snapshot(), paint)));
    }

    fn layout(&mut self, avl_sp: AvailableSpace, _glb_ctx: &Rc<GlobalBuildingContext>) -> LayoutingResult {
        LayoutingResult::fix(20., 20.).checked(avl_sp)
    }
}

fn build_checkbox(arg: UIBuildArg<CheckboxState, Rc<CheckboxController>>) -> Container {
    Container::new([
        Box::new(CursorReactiveBox::new(
            CheckboxButton(SelectableItemState {
                selected: arg.config().ticked.get(),
                hovered: arg.state().hovered,
            }),
            CursorReactiveBoxHandlers {
                on_clicked: arg.handler(|arg, _| {
                    let c = arg.config();
                    c.ticked.set(!c.ticked.get());
                    arg.request_rebuild();
                }),
                on_hover: arg.handler(|arg, _| {
                    arg.state_mut().hovered = true;
                    arg.request_rebuild();
                }),
                on_hover_end: arg.handler(|arg, _| {
                    arg.state_mut().hovered = false;
                    arg.request_rebuild();
                }),
                on_mv_inside: None,
            },
            arg.state().hovered,
        )),
        Box::new(TextLine::new(arg.config().label.clone())),
    ])
    .horizontal()
}

impl Checkbox {
    /// Create a checkbox. Label and state (ticked or not) are specified through `ctrl`
    pub fn new(ctrl: Rc<CheckboxController>) -> Self {
        Self {
            child: ReactiveUI::new_with_config(
                CheckboxState { hovered: false },
                build_checkbox,
                ctrl,
            ),
        }
    }
}
