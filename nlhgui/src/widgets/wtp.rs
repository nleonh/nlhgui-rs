/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use std::{cmp::min, rc::Rc, sync::atomic};

use log::{debug, warn};

use crate::{
    minf, primitives::Drawable, widgets::ex::{
        AvailableSpace, BuildingContext, GlobalBuildingContext, LayoutingResult, SelectedLayout,
        Widget,
    },
};

struct WtpConfig {
    height: f32,
    width: f32,
}

pub struct WidgetsToPrimitivesInterface {
    config: WtpConfig,
    glb_ctx: Rc<GlobalBuildingContext>,
    root: Box<dyn Widget>,
}

pub struct LayoutingContext {
    pub max_width: f32,
    pub max_height: f32,
}

impl LayoutingContext {
    /// `max_width` or `max_height` specifiy whether to maximize (true)
    /// or minimize (false) the size
    ///
    /// returns `None` if layouting is not possible (e.g. not enough
    /// space)
    pub fn select_layout_simple(
        &self,
        layout: &LayoutingResult,
        max_width: bool,
        max_height: bool,
    ) -> Option<SelectedLayout> {
        if layout.min_height > self.max_height || layout.min_width > self.max_width {
            return None;
        }

        let height;
        if max_height {
            if let Some(mh_layout) = layout.max_height {
                height = minf(mh_layout, self.max_height);
            } else {
                height = self.max_height;
            }
        } else {
            height = layout.min_height;
        }

        let width;
        if max_width {
            if let Some(mw_layout) = layout.max_width {
                width = minf(mw_layout, self.max_width);
            } else {
                width = self.max_width;
            }
        } else {
            width = layout.min_width;
        }

        Some(SelectedLayout { width, height })
    }
}

impl WidgetsToPrimitivesInterface {
    pub fn new(width: f32, height: f32, root: Box<dyn Widget>) -> Self {
        let glb_ctx = Rc::new(
            GlobalBuildingContext::default().with_tokio_rt(tokio::runtime::Runtime::new().unwrap()),
        );
        let mut x = Self {
            root,
            config: WtpConfig { height, width },
            glb_ctx,
        };
        debug!("Created wtp interface");
        x.hello();
        debug!("Widgets are ready");
        x
    }

    fn hello(&mut self) {
        self.root.hello(&self.glb_ctx);
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.config.width = width;
        self.config.height = height;
    }

    pub fn glb_ctx(&self) -> &GlobalBuildingContext {
        self.glb_ctx.as_ref()
    }

    pub fn wants_rebuild(&self) -> bool {
        self.glb_ctx.need_rebuild.load(atomic::Ordering::Relaxed)
    }

    pub fn build(&mut self) -> Vec<Box<dyn Drawable>> {
        self.glb_ctx.before_build();

        let layout_ctx = LayoutingContext {
            max_height: self.config.height,
            max_width: self.config.width,
        };
        let possible_layouts = self.root.layout(AvailableSpace {
            width: Some(self.config.width),
            height: Some(self.config.height),
        });
        let selected_layout = match layout_ctx.select_layout_simple(&possible_layouts, true, true) {
            None => {
                warn!("Layouting failed");
                SelectedLayout {
                    width: self.config.width,
                    height: self.config.height,
                }
            }
            Some(selected_layout) => selected_layout,
        };
        self.root.apply_layout(selected_layout);

        let mut result = vec![];
        let building_ctx = BuildingContext {
            x_begin: 0.,
            y_begin: 0.,
        };

        self.root.build(&mut result, building_ctx);

        self.glb_ctx.after_build();
        return result;
    }
}
