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

pub const TAG_OVERVIEW: c_uchar = 0 as c_uchar;

pub type WmAction = Box<Fn(&mut WindowManager)>;
pub type ClientPredicate = Box<Fn(&ClientW) -> bool>;
pub type ClientAction = Box<Fn(&mut ClientW)>;
pub type StartAction = Box<Fn()>;

pub struct Config {
    pub mod_key: c_uint,
    pub add_keys: Vec<(c_uint, c_uint, WmAction)>,
    pub bar_height: c_int,
    pub border_width: c_int,
    pub focused_border_color: &'static str,
    pub keys: Vec<(c_uint, c_uint, WmAction)>,
    pub normal_border_color: &'static str,
    pub overview_inset: c_int,
    pub rules: Vec<(ClientPredicate, ClientAction)>,
    pub start_programs: Vec<StartAction>,
    pub tags: Vec<c_uchar>,
    pub tag_default: c_uchar,
    pub tag_description: Vec<(c_uchar, String)>,
    pub tag_keys: Vec<(c_uint, c_uint, WmAction)>,
    pub tag_layout: Vec<(c_uchar, Box<Layout + 'static>)>,
    pub window_expand_delta: c_int,
    pub window_move_delta: c_int,
}

impl Config {
    pub fn new(mod_mask: c_uint) -> Config {
        let keys: Vec<(c_uint, c_uint, WmAction)> =
            vec![(mod_mask, keysym::XK_q, Box::new(|w| process::exit(0))),
                 (mod_mask, keysym::XK_j, Box::new(|w| w.shift_focus(1))),
                 (mod_mask, keysym::XK_k, Box::new(|w| w.shift_focus(-1))),
                 (mod_mask, keysym::XK_F4, Box::new(|w| w.kill_client())),
                 (mod_mask, keysym::XK_m, Box::new(|w| w.toggle_maximize())),
                 (mod_mask, keysym::XK_e, Box::new(|w| w.toggle_floating())),
                 (mod_mask, keysym::XK_Left, Box::new(|w| w.shift_window(-WINDOW_MOVE_DELTA, 0))),
                 (mod_mask, keysym::XK_Right, Box::new(|w| w.shift_window(WINDOW_MOVE_DELTA, 0))),
                 (mod_mask, keysym::XK_Up, Box::new(|w| w.shift_window(0, -WINDOW_MOVE_DELTA))),
                 (mod_mask, keysym::XK_Down, Box::new(|w| w.shift_window(0, WINDOW_MOVE_DELTA))),
                 (mod_mask | xlib::ShiftMask,
                  keysym::XK_Up,
                  Box::new(|w| w.expand_height(-WINDOW_EXPAND_DELTA))),
                 (mod_mask | xlib::ShiftMask,
                  keysym::XK_Down,
                  Box::new(|w| w.expand_height(WINDOW_EXPAND_DELTA))),
                 (mod_mask | xlib::ShiftMask,
                  keysym::XK_Right,
                  Box::new(|w| w.expand_width(WINDOW_EXPAND_DELTA))),
                 (mod_mask | xlib::ShiftMask,
                  keysym::XK_Left,
                  Box::new(|w| w.expand_width(-WINDOW_EXPAND_DELTA))),
                 (mod_mask, keysym::XK_F2, Box::new(|w| w.select_tag(TAG_OVERVIEW))),
                 (mod_mask,
                  keysym::XK_Return,
                  Box::new(|w| {
                if w.current_tag != TAG_OVERVIEW {
                    w.zoom();
                } else {
                    if let Some(current_client) = w.current_focused() {
                        w.select_tag(current_client.tag());
                        w.set_focus(current_client);
                    }
                }
            }))];
        let (tag_keys, tags) = define_tags!(mod_mask,
                                            xlib::ShiftMask,
                                            ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);
        Config {
            mod_key: mod_mask,
            add_keys: Vec::new(),
            bar_height: BAR_HEIGHT,
            border_width: BORDER_WIDTH,
            focused_border_color: FOCUSED_BORDER_COLOR,
            normal_border_color: NORMAL_BORDER_COLOR,
            keys: keys,
            overview_inset: OVERVIEW_INSET,
            rules: vec![],
            start_programs: vec![],
            tag_default: tags[0],
            tags: tags,
            tag_description: vec![],
            tag_keys: tag_keys,
            tag_layout: Vec::new(),
            window_expand_delta: WINDOW_EXPAND_DELTA,
            window_move_delta: WINDOW_MOVE_DELTA,
        }
    }

    pub fn addtional_keys(mut self, keys: Vec<(c_uint, c_uint, WmAction)>) -> Config {
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

    pub fn keys(mut self, keys: Vec<(c_uint, c_uint, WmAction)>) -> Config {
        self.keys = keys;
        self
    }

    pub fn no_default_keys(mut self) -> Config {
        self.tag_keys = Vec::new();
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

    pub fn rules(mut self, rules: Vec<(ClientPredicate, ClientAction)>) -> Config {
        self.rules = rules;
        self
    }

    pub fn start_programs(mut self, start: Vec<StartAction>) -> Config {
        self.start_programs = start;
        self
    }

    pub fn tag_default(mut self, tag: c_uchar) -> Config {
        self.tag_default = tag;
        self
    }

    pub fn tag_description(mut self, description: Vec<(c_uchar, String)>) -> Config {
        self.tag_description = description;
        self
    }

    pub fn tag_keys(mut self, keys: (Vec<(c_uint, c_uint, WmAction)>, Vec<c_uchar>)) -> Config {
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

    pub fn get_description(&self, tag: c_uchar) -> Option<&str> {
        for c in self.tag_description.iter() {
            if c.0 == tag {
                return Some(&c.1);
            }
        }
        None
    }
}
