/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::cell::Cell;

use skia_safe::{
    Color,
    colors::{BLACK, TRANSPARENT},
    textlayout,
};

use crate::{
    primitives::{ParagraphDrawable, create_cursor_drawable},
    widgets::ex::{AvailableSpace, TextReceiverEvent, WrapperWidget},
};

use super::*;

/// Usually, you just need [`TextButton`].
pub type TextButtonContent = BoxWidget<TextLine>;

fn on_btn_hover(p: UIBuildArg<TextButtonState>, _: ()) {
    p.state_mut().btn_state.hovered = true;
    p.request_rebuild();
}

fn on_btn_click(p: UIBuildArg<TextButtonState>, c: ClickEvent) {
    (*p.state().btn_state.click_handler)(c);
}

fn on_btn_hover_stop(p: UIBuildArg<TextButtonState>, _: ()) {
    p.state_mut().btn_state.hovered = false;
    p.request_rebuild();
}

fn build_txt_button(p: UIBuildArg<TextButtonState>) -> CursorReactiveBox<TextButtonContent> {
    let hovered = p.state().btn_state.hovered;
    let ctx = p.glb_ctx();
    let styles = &ctx.builtin_style;
    return CursorReactiveBox::new(
        BoxWidget::new(TextLine::new(p.state().text.clone()))
            .with_bg_color(if hovered {
                styles.dyn_hover_col
            } else {
                styles.dyn_std_col
            })
            .with_padding(BoxPadding::all(10.))
            .with_border(BoxBorder(1., BLACK)),
        CursorReactiveBoxHandlers {
            on_clicked: p.handler(on_btn_click),
            on_hover: p.handler(on_btn_hover),
            on_hover_end: p.handler(on_btn_hover_stop),
        },
        hovered,
    );
}

/// Generic Button type. Use [`TextButton`] for simply creating a button holding
/// a text widget.
pub struct Button<C: Widget> {
    child: ReactiveUI<TextButtonState, CursorReactiveBox<C>>,
}

/// Shortcut for the most common Button type
pub type TextButton = Button<TextButtonContent>;

impl TextButton {
    pub fn new<H: 'static + Fn(ClickEvent) -> ()>(text: String, handler: H) -> Self {
        Self {
            child: ReactiveUI::new(
                TextButtonState {
                    text,
                    btn_state: ButtonState {
                        click_handler: Rc::new(handler),
                        hovered: false,
                    },
                },
                build_txt_button,
            ),
        }
    }
}

impl<C: 'static + Widget> WrapperWidget for Button<C> {
    fn child<'a>(&'a self) -> &'a dyn Widget {
        &self.child
    }

    fn child_mut<'a>(&'a mut self) -> &'a mut dyn Widget {
        &mut self.child
    }
}

struct KeyboardInputWrapper<C: Widget> {
    child: C,
    want_listen: bool,
    glb_ctx: Option<Rc<GlobalBuildingContext>>,
    handler: Rc<dyn Fn(TextReceiverEvent) + 'static>,
}

impl<T: Widget> KeyboardInputWrapper<T> {
    pub fn new<H: Fn(TextReceiverEvent) + 'static>(
        child: T,
        want_listen: bool,
        handler: H,
    ) -> Self {
        Self {
            child,
            want_listen,
            glb_ctx: None,
            handler: Rc::new(handler),
        }
    }
}

impl<T: Widget> Widget for KeyboardInputWrapper<T> {
    fn apply_layout(&mut self, layout: SelectedLayout) {
        self.child.apply_layout(layout);

        if self.want_listen {
            let x = self.handler.clone();
            self.glb_ctx
                .as_ref()
                .unwrap()
                .register_text_receiver(move |a| {
                    (*x)(a);
                });
        }
    }

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        self.child.build(target, ctx)
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.glb_ctx = Some(ctx.clone());
        self.child.hello(ctx)
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        self.child.layout(avl_sp)
    }
}

struct TextFieldContent {
    content: String,
    prg: Cell<Option<textlayout::Paragraph>>,
    glb_ctx: Option<Rc<GlobalBuildingContext>>,
    cursor_pos: Option<usize>,
    font_size: f32,
}

impl Widget for TextFieldContent {
    fn apply_layout(&mut self, _layout: SelectedLayout) {}

    fn build(&self, target: &mut Vec<Box<dyn Drawable>>, ctx: BuildingContext) {
        let mut prg = self.prg.take().take().unwrap();

        if let Some(cursor_pos) = &self.cursor_pos {
            let mut offset = None;
            if !self.content.is_empty() {
                offset = prg.get_glyph_info_at_utf16_offset(cursor_pos - 1);
            }
            let (cpx, cpy) = offset
                .map(|cp| {
                    (
                        cp.grapheme_layout_bounds.right,
                        cp.grapheme_layout_bounds.top,
                    )
                })
                .unwrap_or((0., 0.));
            target.push(create_cursor_drawable(
                Point(cpx + ctx.x_begin, cpy + ctx.y_begin),
                self.font_size,
            ));
        }

        target.push(Box::new(ParagraphDrawable(
            prg,
            Point(ctx.x_begin, ctx.y_begin),
        )));
    }

    fn hello(&mut self, ctx: &Rc<GlobalBuildingContext>) {
        self.glb_ctx = Some(ctx.clone());
    }

    fn layout(&mut self, avl_sp: AvailableSpace) -> LayoutingResult {
        if avl_sp.width.is_none() {
            todo!();
        }

        let width = avl_sp.width.unwrap();

        use skia_safe::textlayout::*;
        let mut f = FontCollection::new();
        f.enable_font_fallback();
        f.set_default_font_manager(
            Some(self.glb_ctx.as_ref().unwrap().fonts().mgr().clone()),
            "system-ui",
        );
        let mut style = ParagraphStyle::new();
        style.set_height(100.);
        let mut builder = ParagraphBuilder::new(&style, f);

        let mut text_style = TextStyle::new();
        text_style.set_font_size(self.font_size);
        text_style.set_color(Color::from_rgb(0, 0, 0));
        builder.push_style(&text_style);

        builder.add_text(&self.content);

        let mut prg = builder.build();
        prg.layout(width as f32);
        let mut height = prg.height();
        if height == 0. {
            height = 30.;
        }

        self.prg.set(Some(prg));

        LayoutingResult::fix(width, height)
    }
}

struct TextFieldControllerMut {
    cur_down: bool,
    value: String,
    insert_at: usize,
}

pub struct TextFieldEditEvent {}

pub struct TextFieldController {
    inner: RefCell<TextFieldControllerMut>,
}

impl TextFieldController {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(TextFieldControllerMut {
                cur_down: false,
                value: String::new(),
                insert_at: 0,
            }),
        }
    }

    pub fn get_text(&self) -> String {
        self.inner.borrow().value.clone()
    }
}

fn text_field_click(state: UIBuildArg<TextFieldState, Rc<TextFieldController>>, _: ClickEvent) {
    if state.config().inner.borrow().cur_down {
        return;
    }
    state.config().inner.borrow_mut().cur_down = true;

    state.request_rebuild();
}

fn text_field_event(
    state: UIBuildArg<TextFieldState, Rc<TextFieldController>>,
    ev: TextReceiverEvent,
) {
    let mut edited = false;
    match ev {
        TextReceiverEvent::Text(x) => {
            let mut new_str;
            {
                let s = &state.config().inner.borrow().value;
                let pos = state.config().inner.borrow().insert_at;
                new_str = s[0..pos].to_string();
                new_str += &x;
                new_str += &s[pos..];
            }
            state.config().inner.borrow_mut().value = new_str;
            state.config().inner.borrow_mut().insert_at += x.len();
            state.request_rebuild();
            edited = true;
        }
        TextReceiverEvent::LostFocus => {
            state.config().inner.borrow_mut().cur_down = false;
            state.request_rebuild();
        }
        TextReceiverEvent::Delete => {
            let mut conf = state.config().inner.borrow_mut();
            if conf.insert_at == 0 {
                return;
            }
            conf.insert_at -= 1;
            let del = conf.insert_at;
            conf.value.remove(del);
            state.request_rebuild();
            edited = true;
        }
    }
    let tmp = state.state_mut().handle_edit.take();
    if edited && let Some(h) = tmp {
        (*h)(TextFieldEditEvent {});
        state.state_mut().handle_edit = Some(h);
    }
}

fn build_text_field_content(
    state: UIBuildArg<TextFieldState, Rc<TextFieldController>>,
) -> CursorReactiveBox<KeyboardInputWrapper<BoxWidget<TextFieldContent>>> {
    let ctx = state.glb_ctx();
    let styles = &ctx.builtin_style;
    let cfg = state.config().inner.borrow();
    CursorReactiveBox::new(
        KeyboardInputWrapper::new(
            BoxWidget::new(TextFieldContent {
                content: cfg.value.clone(),
                prg: Cell::new(None),
                glb_ctx: None,
                cursor_pos: if cfg.cur_down {
                    Some(cfg.insert_at)
                } else {
                    None
                },
                font_size: state.glb_ctx().builtin_style.std_font_size,
            })
            .with_bg_color(if cfg.cur_down {
                styles.dyn_act_col
            } else if state.state().hovered {
                styles.dyn_hover_col
            } else {
                TRANSPARENT
            })
            .with_border(BoxBorder(1., BLACK)),
            cfg.cur_down,
            state.handler(text_field_event),
        ),
        CursorReactiveBoxHandlers {
            on_clicked: state.handler(text_field_click),
            on_hover: state.handler(|s, _| {
                if s.state().hovered {
                    return;
                }
                s.state_mut().hovered = true;
                s.request_rebuild();
            }),
            on_hover_end: state.handler(|s, _| {
                s.state_mut().hovered = false;
                s.request_rebuild();
            }),
        },
        state.config().inner.borrow().cur_down,
    )
}

struct TextFieldState {
    hovered: bool,
    handle_edit: Option<Box<dyn Fn(TextFieldEditEvent)>>
}

pub struct TextField {
    child: ReactiveUI<
        TextFieldState,
        CursorReactiveBox<KeyboardInputWrapper<BoxWidget<TextFieldContent>>>,
        Rc<TextFieldController>,
    >,
}

impl TextField {
    pub fn new(ctrl: Rc<TextFieldController>) -> Self {
        Self {
            child: ReactiveUI::new_with_config(
                TextFieldState { hovered: false, handle_edit: None },
                build_text_field_content,
                ctrl,
            ),
        }
    }

    /// **Panics** if there already is one event handler
    pub fn with_edit_handler<H: 'static + Fn(TextFieldEditEvent)>(self, handler: H) -> Self {
        assert!(self.child.state().handle_edit.is_none());

        self.child.state_mut().handle_edit = Some(Box::new(handler));
        self
    }
}

impl WrapperWidget for TextField {
    fn child<'a>(&'a self) -> &'a dyn Widget {
        &self.child
    }

    fn child_mut<'a>(&'a mut self) -> &'a mut dyn Widget {
        &mut self.child
    }
}
