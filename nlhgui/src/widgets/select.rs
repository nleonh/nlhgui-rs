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
    Paint, PaintStyle, PathBuilder, colors::{BLACK, BLUE},
};

use crate::{
    primitives::PathDrawable,
    widgets::{
        Container, CursorReactiveBox, CursorReactiveBoxHandlers, TextLine,
        reactive::{ReactiveUI, UIBuildArg},
    },
};

use crate::widgets::ex::*;

pub struct RadioButtonGroup {
    child: ReactiveUI<RadioButtonGroupState, Container, Rc<RadioButtonGroupCtrl>>,
}

struct RadioButtonGroupState {
    hovered: Option<usize>,
}

#[derive(PartialEq, Copy, Clone)]
enum RadioItemState {
    Selected,
    Hovered,
    Nothing,
}

struct RadioClickField(RadioItemState);

impl Widget for RadioClickField {
    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn build(&self, target: &mut Vec<Box<dyn crate::primitives::Drawable>>, ctx: BuildingContext) {
        let mut p = PathBuilder::new();
        let x0 = ctx.x_begin;
        let y0 = ctx.y_begin;
        p.move_to((x0, y0));
        p.line_to((x0 + 16., y0));
        p.line_to((x0 + 16., y0 + 16.));
        p.line_to((x0, y0 + 16.));

        if self.0 == RadioItemState::Selected {
            p.line_to((x0, y0));
            p.line_to((x0 + 16., y0 + 16.));
            p.move_to((x0 + 16., y0));
            p.line_to((x0, y0 + 16.));
        } else {
            p.close();
        }

        let mut paint = Paint::new(
            if self.0 == RadioItemState::Hovered {
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

    fn hello(&mut self, _ctx: &Rc<GlobalBuildingContext>) {}

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        LayoutingResult::fix(20., 20.).checked(avl_sp)
    }
}

fn build_radio_click_field(
    arg: &UIBuildArg<RadioButtonGroupState, Rc<RadioButtonGroupCtrl>>,
    state: RadioItemState,
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
        },
        state != RadioItemState::Nothing,
    )
}

fn build_radio_item(
    arg: &UIBuildArg<RadioButtonGroupState, Rc<RadioButtonGroupCtrl>>,
    item_state: RadioItemState,
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
        container.add(build_radio_item(
            &arg,
            if i == selected {
                RadioItemState::Selected
            } else if i == hovered {
                RadioItemState::Hovered
            } else {
                RadioItemState::Nothing
            },
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
    fn child<'a>(&'a self) -> &'a dyn super::ex::Widget {
        &self.child
    }

    fn child_mut<'a>(&'a mut self) -> &'a mut dyn super::ex::Widget {
        &mut self.child
    }
}

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
}
