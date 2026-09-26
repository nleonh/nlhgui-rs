/*
 Copyright (C) 2026 Nils L. Hake

 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use log::debug;
use skia_safe::{FontMgr, FontStyle, FontStyleSet, Typeface};

pub struct FontsModule {
    sk_mgr: FontMgr,
    typeface: Option<Typeface>,
}

impl FontsModule {
    pub fn create() -> Self {
        let mut m = Self {
            sk_mgr: FontMgr::new(),
            typeface: None,
        };
        m.load_default_font();
        m
    }

    fn try_load_font(&mut self, name: &str) -> bool {
        let mut system_font = self.match_family(name);
        if let Some(typeface) = system_font.match_style(FontStyle::normal()) {
            self.typeface = Some(typeface);
            debug!("Using font {}", name);
            true
        } else {
            debug!("Could not load font {}", name);
            false
        }
    }

    fn load_known_sans_font(&mut self) -> bool {
        let fonts = [
            "Segoe UI",
            "Arial",
            "San Francisco",
            "Helvetica",
            "Aptos",
            "Calibri",
        ];
        for font in &fonts {
            if self.try_load_font(font) {
                return true;
            }
        }
        false
    }

    fn load_any_font(&mut self) -> bool {
        todo!()
    }

    fn load_default_font(&mut self) {
        if !(self.try_load_font("system-ui") || self.load_known_sans_font() || self.load_any_font())
        {
            panic!("Could not load any font");
        }
    }

    pub fn match_family(&self, name: &str) -> FontStyleSet {
        self.sk_mgr.match_family(name)
    }

    pub fn std_typeface(&self) -> &Typeface {
        self.typeface.as_ref().unwrap()
    }

    pub fn mgr(&self) -> &FontMgr {
        &self.sk_mgr
    }
}
