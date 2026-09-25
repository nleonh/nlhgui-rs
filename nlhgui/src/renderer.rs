/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use skia_safe::{Canvas, colors::WHITE};

use crate::primitives::Drawable;

pub(crate) struct Renderer {
    items: Vec<Box<dyn Drawable>>,
}

impl Renderer {
    pub fn create() -> Self {
        Self { items: vec![] }
    }

    pub fn replace_items(&mut self, new_items: Vec<Box<dyn Drawable>>) {
        self.items = new_items;
    }

    pub fn render(&mut self, canvas: &Canvas) {
        canvas.clear(WHITE);

        for item in &self.items {
            item.draw(canvas);
        }
    }
}
