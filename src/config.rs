use std::os::raw::{c_int, c_uchar, c_uint};
use std::process;

use x11::{xlib, keysym};

use core::{WindowManager, overview, fullscreen};
use client::{ClientL, ClientW};
use util;
use util::spawn;

pub const FOCUSED_BORDER_COLOR: &'static str = "RGBi:0.0/1.0/1.0";
pub const NORMAL_BORDER_COLOR: &'static str = "RGBi:0.0/0.3/0.3";

pub const BORDER_WIDTH: c_int = 3;
pub const OVERVIEW_INSET: c_int = 15;
pub const BAR_HEIGHT: c_int = 15;

pub const WINDOW_MOVE_DELTA: c_int = 15;
pub const WINDOW_EXPAND_DELTA: c_int = 10;

pub type MyLogger = util::XMobarLogger;

pub const LOGGER_CONFIG: &'static util::LoggerConfig = &util::LoggerConfig {
    selected_tag_color: "#00FF00",
    tag_color: "#FFFFFF",
    separator_color: "#000000",
    selected_client_color: "#FFFF00",
    client_color: "#FFFFFF",
};

pub const MOD_MASK: c_uint = xlib::Mod1Mask;

#[allow(unused_variables)]
pub const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))] =
    &[(MOD_MASK, keysym::XK_r, &|w| spawn("dmenu_run", &[])),
      (MOD_MASK, keysym::XK_q, &|w| process::exit(0)),
      (MOD_MASK, keysym::XK_t, &|w| spawn("xterm", &[])),
      (MOD_MASK, keysym::XK_j, &|w| w.shift_focus(1)),
      (MOD_MASK, keysym::XK_k, &|w| w.shift_focus(-1)),
      (MOD_MASK, keysym::XK_F4, &|w| w.kill_client()),
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
        if let Some(focus) = w.current_focus {
            if w.current_tag == TAG_OVERVIEW && w.current_stack.len() > focus {
                let client = w.current_stack[focus].clone();
                w.select_tag(client.tag());
            } else {
                w.zoom();
            }
        }
    })];

pub const TAG_KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))] =
    &define_tags!(xlib::Mod1Mask,
                  xlib::ShiftMask,
                  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);

pub type LayoutFn = &'static Fn(&ClientL, usize, c_int, c_int, c_int, c_int)
                                -> Vec<(c_int, c_int, c_int, c_int)>;
pub const TAG_LAYOUT: &'static [(c_uchar, LayoutFn)] = &[('9' as c_uchar, &fullscreen),
                                                         (TAG_OVERVIEW, &overview)];

pub const TAG_DEFAULT: c_uchar = '1' as c_uchar;
pub const TAG_OVERVIEW: c_uchar = 0 as c_uchar;

pub const RULES: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))] =
    &[(&|c| c.get_class() == "Gimp", &|c| c.set_floating(true)),
      (&|c| c.is_dialog(), &|c| c.set_floating(true)),
      (&|c| c.get_class() == "Tilda", &|c| c.set_floating(true))];

pub const START_PROGRAMS: &'static [&'static Fn()] =
    &[&|| spawn("xcompmgr", &[]), &|| spawn("fcitx", &[]), &|| spawn("tilda", &["--hidden"])];