/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use log::{debug, error};

use crate::events::EventHandling;

pub mod gl;
pub mod vk;

pub trait WindowBackend<'a> {
    fn run(self: Box<Self>);

    /// returns `(width, height)`
    fn get_size(&self) -> (u32, u32);

    fn use_events(&mut self, events: EventHandling<'a>);
}

pub fn create_window(title: &String) -> Option<Box<dyn WindowBackend<'static>>> {
    if true {
        match vk::create_vk_window() {
            Ok(vk) => {
                debug!("backend: Vulkan");
                return Some(vk);
            }
            Err(e) => error!("failed to create Vulkan windwo: {}", e),
        }
    }
    match gl::create_gl_window(title) {
        Ok(gl) => {
            debug!("backend: OpenGL");
            return Some(gl);
        }
        Err(e) => error!("failed to create OpenGL window: {}", e),
    }
    error!("no backend working: window creation failed");
    None
}
