/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::sync::Arc;

use log::{debug, error};
use winit::event_loop::EventLoopProxy;

use crate::{RenderingBackend, RenderingBackendPreference, events::EventHandling};

pub mod gl;
pub mod vk;

pub trait EventLoopAwaker {
    fn request_redraw(&self) {

    }
}

#[derive(Debug)]
enum CustomWinitEvent {
    Redraw,
}

struct WinitElAwaker {
    proxy: EventLoopProxy<CustomWinitEvent>
}

impl EventLoopAwaker for WinitElAwaker {
    fn request_redraw(&self) {
        self.proxy.send_event(CustomWinitEvent::Redraw).unwrap();
    }
}

pub trait WindowBackend<'a> {
    fn run(self: Box<Self>);

    /// returns `(width, height)`
    fn get_size(&self) -> (u32, u32);

    fn use_events(&mut self, events: EventHandling<'a>);

    fn get_el_awaker(&self) -> Arc<dyn Send + Sync + EventLoopAwaker>;
}

pub fn create_window(
    title: &String,
    config: &RenderingBackendPreference,
) -> Option<Box<dyn WindowBackend<'static>>> {
    for backend in &config.0 {
        match backend {
            RenderingBackend::Vulkan => match vk::create_vk_window(title.clone()) {
                Ok(vk) => {
                    debug!("Backend: Vulkan");
                    return Some(vk);
                }
                Err(e) => error!("Failed to create Vulkan windwo: {}", e),
            },
            RenderingBackend::OpenGL => match gl::create_gl_window(title) {
                Ok(gl) => {
                    debug!("Backend: OpenGL");
                    return Some(gl);
                }
                Err(e) => error!("Failed to create OpenGL window: {}", e),
            },
        }
    }
    error!("no backend working: window creation failed");
    None
}
