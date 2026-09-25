/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use skia_safe::{Paint, PaintStyle};

use crate::{
    primitives::{Drawable, Point},
    widgets::Color,
};

pub struct Rect {
    rect_impl: skia_safe::Rect,
    color: Color,
}

impl Rect {
    pub fn new(pos_x: f32, pos_y: f32, width: f32, height: f32, color: Color) -> Self {
        Self {
            rect_impl: skia_safe::Rect {
                left: pos_x,
                top: pos_y,
                right: pos_x + width,
                bottom: pos_y + height,
            },
            color,
        }
    }
}

impl Drawable for Rect {
    fn draw(&self, canvas: &skia_safe::Canvas) {
        let paint = skia_safe::Paint::new(&self.color, None);
        canvas.draw_rect(self.rect_impl, &paint);
    }
}

pub struct PathDrawable(pub skia_safe::Path, pub Paint);

impl Drawable for PathDrawable {
    fn draw(&self, canvas: &skia_safe::Canvas) {
        canvas.draw_path(&self.0, &self.1);
    }
}

/// Utility for creating a border. Used by [`crate::widgets::BoxWidget`] (see
/// [`crate::widgets::BoxWidget::with_border`]). `point` has to be the upper left
/// corner of the area inside of the border.
pub fn create_border_path(
    inner_width: f32,
    inner_height: f32,
    thickness: f32,
    point: Point,
    color: Color,
) -> PathDrawable {
    let mv_out = thickness / 2.;
    let mut pathb = skia_safe::PathBuilder::new();
    let a = Point(point.0 - mv_out, point.1 - mv_out);
    let b = Point(point.0 - mv_out, point.1 + inner_height + mv_out);
    let c = Point(
        point.0 + inner_width + mv_out,
        point.1 + inner_height + mv_out,
    );
    let d = Point(point.0 + mv_out + inner_width, point.1 - mv_out);
    pathb.move_to(&a);
    pathb.line_to(&b);
    pathb.line_to(&c);
    pathb.line_to(&d);
    pathb.line_to(&a);
    pathb.close();

    let mut paint = Paint::new(color, None);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(thickness as f32);

    let path = pathb.snapshot();
    PathDrawable(path, paint)
}
