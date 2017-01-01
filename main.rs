#[macro_use]
extern crate rswm;
extern crate x11;

use std::os::raw::{c_uchar, c_uint};
use x11::{keysym, xlib};

use rswm::client::ClientW;
use rswm::config::*;
use rswm::core;
use rswm::loggers;
use rswm::layout::{Tile, FullScreen, Layout};
use rswm::util::spawn;

const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))] =
    &[(MOD_MASK, keysym::XK_r, &|_| spawn("dmenu_run", &[])),
      (MOD_MASK, keysym::XK_t, &|_| spawn("urxvt", &[])),
      (MOD_MASK, keysym::XK_Print, &|_| spawn("scrot", &["-e", "mv $f ~/"])),
      (MOD_MASK | xlib::Mod1Mask,
       keysym::XK_Print,
       &|_| spawn("scrot", &["-s", "-e", "mv $f ~/"])),
      (MOD_MASK, keysym::XK_f, &|_| spawn("pcmanfm", &[])),
      (MOD_MASK, keysym::XK_l, &|_| spawn("i3lock", &[])),
      (0, keysym::XF86XK_MonBrightnessUp, &|_| spawn("xbrightness", &["+10000"])),
      (0, keysym::XF86XK_MonBrightnessDown, &|_| spawn("xbrightness", &["-10000"])),
      (0, keysym::XF86XK_AudioRaiseVolume, &|_| spawn("amixer", &["set", "Master", "5000+"])),
      (0, keysym::XF86XK_AudioLowerVolume, &|_| spawn("amixer", &["set", "Master", "5000-"])),
      (0, keysym::XF86XK_AudioMute, &|_| spawn("amixer", &["set", "Master", "toggle"])),
      (0, keysym::XF86XK_AudioMicMute, &|_| spawn("amixer", &["set", "Capture", "toggle"]))];

const START_PROGRAMS: &'static [&'static Fn()] =
    &[&|| spawn("xcompmgr", &[]),
      &|| spawn("fcitx", &[]),
      &|| spawn("tilda", &["--hidden"]),
      &|| spawn("/usr/lib/polkit-kde/polkit-kde-authentication-agent-1", &[])];

const TAG_LAYOUT: &'static [(c_uchar, &'static str)] = &[('9' as c_uchar, "fullscreen"),
                                                         (TAG_OVERVIEW as c_uchar, "overview")];

const RULES: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))] =
    &[(&|c| c.get_class() == "Gimp", &|c| c.set_floating(true)),
      (&|c| c.is_dialog(), &|c| c.set_floating(true)),
      (&|c| c.get_class() == "VirtualBox", &|c| c.set_floating(true)),
      (&|c| c.get_class() == "Tilda",
       &|c| {
           c.set_floating(true);
           c.set_sticky(true);
       }),
      (&|c| c.get_class() == "Wicd-client.py", &|c| c.set_floating(true))];

const TAG_KEYS: &'static (&'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))],
          &'static [c_uchar]) = &define_tags!(MOD_MASK,
                                              xlib::ShiftMask,
                                              ['1', '2', '3', '4', '5', '6', '7', '8', '9']);

const TAG_DESCRIPTION: &'static [(c_uchar, &'static str)] = &[('1' as c_uchar, "网页"),
                                                              ('2' as c_uchar, "代码")];

fn main() {
    let config = Config::default()
        .border_width(3)
        .bar_height(31)
        .addtional_keys(KEYS)
        .start_programs(START_PROGRAMS)
        .tag_keys(TAG_KEYS)
        .tag_default('1' as c_uchar)
        .rules(RULES)
        .tag_description(TAG_DESCRIPTION)
        .tag_layout(TAG_LAYOUT);
    let xmobar_logger = loggers::XMobarLogger::new(loggers::LoggerConfig::default(), &[]);
    let mut window_manager = core::WindowManager::new(config);
    window_manager.set_logger(Box::new(xmobar_logger));
    window_manager.run();
}
