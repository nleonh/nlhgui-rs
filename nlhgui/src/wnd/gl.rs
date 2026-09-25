/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

/*
 This file incorporates work covered by the following copyright and
 permission notice:

    https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/gl-window/main.rs (modified)

    MIT License

    Copyright (c) 2019 LongYinan & Armin Sander
    Copyright (c) 2019 rust-skia Contributors

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
*/

use std::{
    error::Error,
    ffi::CString,
    num::NonZeroU32,
    time::{Duration, Instant},
};

use gl::types::*;
use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    prelude::{GlSurface, NotCurrentGlContext},
    surface::{Surface as GlutinSurface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use log::debug;
use raw_window_handle::HasWindowHandle;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, Modifiers, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

use skia_safe::{
    ColorType, Surface,
    gpu::{self, SurfaceOrigin, backend_render_targets, gl::FramebufferInfo},
};

use crate::events::{EventHandling, GenericKeyEvent, GenericMouseButton};

struct App<'a> {
    gel: EventHandling<'a>,
    env: OpenGlStuff,
    fb_info: FramebufferInfo,
    num_samples: usize,
    stencil_size: usize,
    modifiers: Modifiers,
    previous_frame_start: Instant,
}

struct GlWindowBackend<'a> {
    el: EventLoop<()>,
    app: App<'a>,
}

// don't change the order
struct OpenGlStuff {
    surface: Surface,
    gl_surface: GlutinSurface<WindowSurface>,
    gr_context: skia_safe::gpu::DirectContext,
    gl_context: PossiblyCurrentContext,
    window: Window,
}

impl Drop for OpenGlStuff {
    fn drop(&mut self) {
        self.gr_context.release_resources_and_abandon();
    }
}

fn create_config(
    el: &EventLoop<()>,
    window_attributes: WindowAttributes,
) -> Result<(Window, glutin::config::Config), Box<dyn Error>> {
    let template = ConfigTemplateBuilder::new();
    let display_builder = DisplayBuilder::new().with_window_attributes(window_attributes.into());
    let (window, gl_config) = display_builder.build(el, template, |configs| {
        configs
            .reduce(|accum, config| {
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                if transparency_check || config.num_samples() < accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    })?;
    debug!("Picked a config with {} samples", gl_config.num_samples());
    Ok((window.unwrap(), gl_config))
}

fn create_surface(
    window: &Window,
    fb_info: FramebufferInfo,
    gr_context: &mut skia_safe::gpu::DirectContext,
    num_samples: usize,
    stencil_size: usize,
) -> Surface {
    let size = window.inner_size();
    let size = (
        size.width.try_into().expect("Could not convert width"),
        size.height.try_into().expect("Could not convert height"),
    );
    let backend_render_target =
        backend_render_targets::make_gl(size, num_samples, stencil_size, fb_info);

    gpu::surfaces::wrap_backend_render_target(
        gr_context,
        &backend_render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .expect("Could not create skia surface")
}

impl<'a> super::WindowBackend<'a> for GlWindowBackend<'a> {
    fn run(mut self: Box<Self>) {
        self.el.run_app(&mut self.app).unwrap();
    }

    fn get_size(&self) -> (u32, u32) {
        (
            self.app.env.gl_surface.width().unwrap(),
            self.app.env.gl_surface.height().unwrap(),
        )
    }

    fn use_events(&mut self, events: EventHandling<'a>) {
        self.app.gel = events;
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let winit::event::StartCause::ResumeTimeReached { .. } = cause {
            self.env.window.request_redraw()
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let mut draw_frame = false;
        let frame_start = Instant::now();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(physical_size) => {
                self.env.surface = create_surface(
                    &self.env.window,
                    self.fb_info,
                    &mut self.env.gr_context,
                    self.num_samples,
                    self.stencil_size,
                );
                /* First resize the opengl drawable */
                let (width, height): (u32, u32) = physical_size.into();

                self.env.gl_surface.resize(
                    &self.env.gl_context,
                    NonZeroU32::new(width.max(1)).unwrap(),
                    NonZeroU32::new(height.max(1)).unwrap(),
                );

                self.gel.handle_resize(width, height);
            }
            WindowEvent::ModifiersChanged(new_modifiers) => self.modifiers = new_modifiers,
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let (x, y): (f64, f64) = position.into();
                let x = x as u32;
                let y = y as u32;
                self.gel.handle_cursor_move(x, y);
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
                    self.gel.handle_mouse_btn(state.is_pressed(), button);
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

                    if let Some(text) = text.as_ref() {
                        if !(text.is_empty()
                            || text.chars().nth(0).unwrap().is_ascii_control()
                            || modstate.control_key()
                            || modstate.alt_key())
                        {
                            self.gel.handle_text_input(text.to_string());
                            done = true;
                        }
                    }

                    if !done {
                        self.gel.handle_key_event(GenericKeyEvent {
                            ctrl: modstate.control_key(),
                            alt: modstate.alt_key(),
                            key: logical_key,
                        });
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                draw_frame = true;
            }
            _ => (),
        }

        let expected_frame_length_seconds = 1.0 / 10.0;
        let frame_duration = Duration::from_secs_f32(expected_frame_length_seconds);

        if frame_start - self.previous_frame_start > frame_duration {
            draw_frame = true;
            self.previous_frame_start = frame_start;
        }
        if draw_frame {
            let canvas = self.env.surface.canvas();

            self.gel.render(canvas);

            self.env.gr_context.flush_and_submit();
            self.env
                .gl_surface
                .swap_buffers(&self.env.gl_context)
                .unwrap();
        }

        if self.gel.wants_redraw() {
            self.env.window.request_redraw();
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            self.previous_frame_start + frame_duration,
        ));
    }
}

pub fn create_window(
    title: &String,
) -> Result<Box<dyn super::WindowBackend<'static>>, Box<dyn Error>> {
    let el = EventLoop::new()?;

    let window_attributes = WindowAttributes::default().with_title(title);

    let (window, gl_config) = create_config(&el, window_attributes)?;

    let window_handle = window.window_handle()?;
    let raw_window_handle = window_handle.as_raw();

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));
    let not_current_gl_context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    };

    let (width, height): (u32, u32) = window.inner_size().into();

    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .expect("Could not create gl window surface")
    };

    let gl_context = not_current_gl_context
        .make_current(&gl_surface)
        .expect("Could not make GL context current when setting up skia renderer");

    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(CString::new(s).unwrap().as_c_str())
    });
    let interface = skia_safe::gpu::gl::Interface::new_load_with(|name| {
        if name == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        gl_config
            .display()
            .get_proc_address(CString::new(name).unwrap().as_c_str())
    })
    .expect("Could not create interface");

    let mut gr_context = skia_safe::gpu::direct_contexts::make_gl(interface, None)
        .expect("Could not create direct context");

    let fb_info = {
        let mut fboid: GLint = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: skia_safe::gpu::gl::Format::RGBA8.into(),
            ..Default::default()
        }
    };

    let num_samples = gl_config.num_samples() as usize;
    let stencil_size = gl_config.stencil_size() as usize;

    let surface = create_surface(&window, fb_info, &mut gr_context, num_samples, stencil_size);

    let env = OpenGlStuff {
        surface,
        gl_surface,
        gl_context,
        gr_context,
        window,
    };

    let app = App {
        // TODO don't create this dummy object for no reason (it is being replaced later)
        gel: EventHandling::new(),
        env,
        fb_info,
        num_samples,
        stencil_size,
        modifiers: Modifiers::default(),
        previous_frame_start: Instant::now(),
    };
    Ok(Box::new(GlWindowBackend { app, el }))
}
