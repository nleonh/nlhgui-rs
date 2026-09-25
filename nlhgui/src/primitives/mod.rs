/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

mod shapes;

pub use shapes::*;

use skia_safe::{
    Canvas, Paint, PaintStyle, PathBuilder, TextBlob, colors::{BLACK, BLUE}, textlayout,
};

#[derive(Debug)]
pub struct Point(pub f32, pub f32);

impl From<&Point> for skia_safe::Point {
    fn from(other: &Point) -> Self {
        skia_safe::Point {
            x: other.0,
            y: other.1,
        }
    }
}

pub trait Drawable {
    fn draw(&self, canvas: &Canvas);
}

pub struct TextBlobDrawable {
    blob: TextBlob,
    point: Point,
}

impl TextBlobDrawable {
    pub fn new(blob: TextBlob, point: Point) -> Self {
        Self { blob, point }
    }
}

impl Drawable for TextBlobDrawable {
    fn draw(&self, canvas: &Canvas) {
        let paint = skia_safe::Paint::new(BLACK, None);
        canvas.draw_text_blob(&self.blob, &self.point, &paint);
    }
}

pub struct ParagraphDrawable(pub textlayout::Paragraph, pub Point);

impl Drawable for ParagraphDrawable {
    fn draw(&self, canvas: &Canvas) {
        self.0.paint(canvas, &self.1);
    }
}

pub fn create_cursor_drawable(pos: Point, size: f32) -> Box<dyn Drawable> {
    let mut path = PathBuilder::new();
    path.move_to(&pos);
    path.line_to(&Point(pos.0, pos.1 + size));
    let path = path.snapshot();
    let mut paint = Paint::new(BLUE, None);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.);
    Box::new(PathDrawable(path, paint))
}
