use std::os::raw::{c_int, c_uchar, c_uint};
use std::process;

use x11::{xlib, keysym};

use core::WindowManager;
use client::{ClientL, ClientW};
use layout::Layout;

const FOCUSED_BORDER_COLOR: &'static str = "RGBi:0.0/1.0/1.0";
const NORMAL_BORDER_COLOR: &'static str = "RGBi:0.0/0.3/0.3";

const BORDER_WIDTH: c_int = 3;
const OVERVIEW_INSET: c_int = 15;
const BAR_HEIGHT: c_int = 15;

const WINDOW_MOVE_DELTA: c_int = 15;
const WINDOW_EXPAND_DELTA: c_int = 10;

pub const MOD_MASK: c_uint = xlib::Mod4Mask;

#[allow(unused_variables)]
const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))] =
    &[(MOD_MASK, keysym::XK_q, &|w| process::exit(0)),
      (MOD_MASK, keysym::XK_j, &|w| w.shift_focus(1)),
      (MOD_MASK, keysym::XK_k, &|w| w.shift_focus(-1)),
      (MOD_MASK, keysym::XK_F4, &|w| w.kill_client()),
      (MOD_MASK, keysym::XK_m, &|w| w.toggle_maximize()),
      (MOD_MASK, keysym::XK_e, &|w| w.toggle_floating()),
      (MOD_MASK, keysym::XK_Left, &|w| w.shift_window(-WINDOW_MOVE_DELTA, 0)),
      (MOD_MASK, keysym::XK_Right, &|w| w.shift_window(WINDOW_MOVE_DELTA, 0)),
      (MOD_MASK, keysym::XK_Up, &|w| w.shift_window(0, -WINDOW_MOVE_DELTA)),
      (MOD_MASK, keysym::XK_Down, &|w| w.shift_window(0, WINDOW_MOVE_DELTA)),
      (MOD_MASK | xlib::ShiftMask, keysym::XK_Up, &|w| w.expand_height(-WINDOW_EXPAND_DELTA)),
      (MOD_MASK | xlib::ShiftMask, keysym::XK_Down, &|w| w.expand_height(WINDOW_EXPAND_DELTA)),
      (MOD_MASK | xlib::ShiftMask, keysym::XK_Right, &|w| w.expand_width(WINDOW_EXPAND_DELTA)),
      (MOD_MASK | xlib::ShiftMask, keysym::XK_Left, &|w| w.expand_width(-WINDOW_EXPAND_DELTA)),
      (MOD_MASK, keysym::XK_F2, &|w| w.select_tag(TAG_OVERVIEW)),
      (MOD_MASK,
       keysym::XK_Return,
       &|w| {
        if w.current_tag != TAG_OVERVIEW {
            w.zoom();
        } else {
            if let Some(current_client) = w.current_focused() {
                w.select_tag(current_client.tag());
                w.set_focus(current_client);
            }
        }
    })];

const TAG_KEYS: &'static (&'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))],
          &'static [c_uchar]) = &define_tags!(MOD_MASK,
                                              xlib::ShiftMask,
                                              ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);

pub const TAG_OVERVIEW: c_uchar = 0 as c_uchar;

pub struct Config {
    pub add_keys: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))],
    pub bar_height: c_int,
    pub border_width: c_int,
    pub focused_border_color: &'static str,
    pub keys: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))],
    pub normal_border_color: &'static str,
    pub overview_inset: c_int,
    pub rules: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))],
    pub start_programs: &'static [&'static Fn()],
    pub tags: &'static [c_uchar],
    pub tag_default: c_uchar,
    pub tag_description: &'static [(c_uchar, &'static str)],
    pub tag_keys: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))],
    pub tag_layout: Vec<(c_uchar, Box<Layout + 'static>)>,
    pub window_expand_delta: c_int,
    pub window_move_delta: c_int,
}

impl Config {
    pub fn addtional_keys(mut self,
                          keys: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))])
                          -> Config {
        self.add_keys = keys;
        self
    }

    pub fn bar_height(mut self, bar_height: c_int) -> Config {
        self.bar_height = bar_height;
        self
    }

    pub fn border_width(mut self, border_width: c_int) -> Config {
        self.border_width = border_width;
        self
    }

    pub fn focused_border_color(mut self, color: &'static str) -> Config {
        self.focused_border_color = color;
        self
    }

    pub fn keys(mut self,
                keys: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))])
                -> Config {
        self.keys = keys;
        self
    }

    pub fn no_default_keys(mut self) -> Config {
        self.tag_keys = &[];
        self
    }

    pub fn normal_border_color(mut self, color: &'static str) -> Config {
        self.normal_border_color = color;
        self
    }

    pub fn overview_inset(mut self, inset: c_int) -> Config {
        self.overview_inset = inset;
        self
    }

    pub fn rules(mut self,
                 rules: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))])
                 -> Config {
        self.rules = rules;
        self
    }

    pub fn start_programs(mut self, start: &'static [&'static Fn()]) -> Config {
        self.start_programs = start;
        self
    }

    pub fn tag_default(mut self, tag: c_uchar) -> Config {
        self.tag_default = tag;
        self
    }

    pub fn tag_description(mut self, description: &'static [(c_uchar, &'static str)]) -> Config {
        self.tag_description = description;
        self
    }

    pub fn tag_keys(mut self,
                    keys: &'static (&'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))],
                                    &'static [c_uchar]))
                    -> Config {
        self.tag_keys = keys.0;
        self.tag_default = keys.1[0];
        self
    }

    pub fn tag_layout(mut self, layout: Vec<(c_uchar, Box<Layout + 'static>)>) -> Config {
        self.tag_layout = layout;
        self
    }

    pub fn window_expand_delta(mut self, delta: c_int) -> Config {
        self.window_expand_delta = delta;
        self
    }

    pub fn window_move_delta(mut self, delta: c_int) -> Config {
        self.window_move_delta = delta;
        self
    }

    pub fn get_description(&self, tag: c_uchar) -> Option<&'static str> {
        for c in self.tag_description {
            if c.0 == tag {
                return Some(c.1);
            }
        }
        None
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            add_keys: &[],
            bar_height: BAR_HEIGHT,
            border_width: BORDER_WIDTH,
            focused_border_color: FOCUSED_BORDER_COLOR,
            normal_border_color: NORMAL_BORDER_COLOR,
            keys: KEYS,
            overview_inset: OVERVIEW_INSET,
            rules: &[],
            start_programs: &[],
            tags: TAG_KEYS.1,
            tag_default: TAG_KEYS.1[0],
            tag_description: &[],
            tag_keys: TAG_KEYS.0,
            tag_layout: Vec::new(),
            window_expand_delta: WINDOW_EXPAND_DELTA,
            window_move_delta: WINDOW_MOVE_DELTA,
        }
    }
}
