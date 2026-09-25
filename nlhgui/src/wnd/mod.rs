/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::error::Error;

use crate::events::EventHandling;

pub mod gl;

pub trait WindowBackend<'a> {
    fn run(self: Box<Self>);

    /// returns `(width, height)`
    fn get_size(&self) -> (u32, u32);

    fn use_events(&mut self, events: EventHandling<'a>);
}

pub fn create_window(title: &String) -> Result<Box<dyn WindowBackend<'static>>, Box<dyn Error>> {
    return gl::create_window(title);
}
