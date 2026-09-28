/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use log::error;
use skia_safe::Canvas;

#[derive(PartialEq)]
pub enum GenericMouseButton {
    Left,
    Right,
}

pub struct GenericKeyEvent {
    pub ctrl: bool,
    pub alt: bool,
    pub key: winit::keyboard::Key,
}

pub use winit::keyboard::NamedKey;

pub struct EventHandling<'a> {
    render_fn: Box<dyn FnMut(&Canvas) + 'a>,
    resize_fn: Box<dyn FnMut(u32, u32) + 'a>,
    cursor_move_fn: Box<dyn FnMut(u32, u32) + 'a>,
    mouse_btn_fn: Box<dyn FnMut(bool, GenericMouseButton) + 'a>,
    text_input_fn: Box<dyn Fn(String) + 'a>,
    wants_redraw_fn: Box<dyn Fn() -> bool + 'a>,
    key_event_fn: Box<dyn Fn(GenericKeyEvent) + 'a>,
    modifiers: winit::event::Modifiers,
}

impl<'a> Default for EventHandling<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> EventHandling<'a> {
    pub fn new() -> Self {
        Self {
            render_fn: Box::new(|_| {
                error!("dummy render function called");
            }),
            resize_fn: Box::new(|_, _| {
                error!("dummy resize function called");
            }),
            cursor_move_fn: Box::new(|_, _| {
                error!("dummy cursor move function called");
            }),
            mouse_btn_fn: Box::new(|_, _| error!("dummy mouse btn function called")),
            text_input_fn: Box::new(|_| error!("dummy text input fn called")),
            wants_redraw_fn: Box::new(|| {
                error!("dummy wants redraw fn called");
                false
            }),
            key_event_fn: Box::new(|_| error!("dummy key event fn called")),
            modifiers: winit::event::Modifiers::default()
        }
    }

    pub fn render(&mut self, canvas: &Canvas) {
        (*self.render_fn)(canvas);
    }

    pub fn on_render<H: FnMut(&Canvas) + 'a>(&mut self, handler: H) {
        self.render_fn = Box::new(handler);
    }

    pub fn on_resize<H: 'a + FnMut(u32, u32)>(&mut self, handler: H) {
        self.resize_fn = Box::new(handler);
    }

    pub fn handle_resize(&mut self, width: u32, height: u32) {
        (*self.resize_fn)(width, height);
    }

    pub fn handle_cursor_move(&mut self, x: u32, y: u32) {
        (*self.cursor_move_fn)(x, y);
    }

    pub fn on_cursor_move<H: 'a + FnMut(u32, u32)>(&mut self, handler: H) {
        self.cursor_move_fn = Box::new(handler);
    }

    pub fn on_mouse_btn<H: 'a + FnMut(bool, GenericMouseButton)>(&mut self, handler: H) {
        self.mouse_btn_fn = Box::new(handler);
    }

    pub fn handle_mouse_btn(&mut self, down: bool, btn: GenericMouseButton) {
        (*self.mouse_btn_fn)(down, btn);
    }

    pub fn handle_text_input(&self, input: String) {
        (*self.text_input_fn)(input);
    }

    pub fn on_text_input<H: 'a + Fn(String)>(&mut self, handler: H) {
        self.text_input_fn = Box::new(handler);
    }

    pub fn wants_redraw(&self) -> bool {
        (*self.wants_redraw_fn)()
    }

    pub fn set_wants_redraw_answer<H: Fn() -> bool + 'a>(&mut self, handler: H) {
        self.wants_redraw_fn = Box::new(handler);
    }

    pub fn handle_key_event(&self, key_event: GenericKeyEvent) {
        (*self.key_event_fn)(key_event);
    }

    pub fn on_key_event<H: Fn(GenericKeyEvent) + 'a>(&mut self, handler: H) {
        self.key_event_fn = Box::new(handler);
    }

    pub fn handle_winit_event(&mut self, event: winit::event::WindowEvent) {
        use winit::event::{WindowEvent, MouseButton, KeyEvent};

        match event {
            WindowEvent::ModifiersChanged(new_modifiers) => self.modifiers = new_modifiers,
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let (x, y): (f64, f64) = position.into();
                let x = x as u32;
                let y = y as u32;
                self.handle_cursor_move(x, y);
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let button = match button {
                    MouseButton::Left => Some(GenericMouseButton::Left),
                    MouseButton::Right => Some(GenericMouseButton::Right),
                    _ => None,
                };
                if let Some(button) = button {
                    self.handle_mouse_btn(state.is_pressed(), button);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        text,
                        state,
                        logical_key,
                        ..
                    },
                ..
            } => {
                if state.is_pressed() {
                    let modstate = self.modifiers.state();
                    let mut done = false;

                    if let Some(text) = text.as_ref()
                        && !(text.is_empty()
                            || text.chars().nth(0).unwrap().is_ascii_control()
                            || modstate.control_key()
                            || modstate.alt_key())
                    {
                        self.handle_text_input(text.to_string());
                        done = true;
                    }

                    if !done {
                        self.handle_key_event(GenericKeyEvent {
                            ctrl: modstate.control_key(),
                            alt: modstate.alt_key(),
                            key: logical_key,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}
