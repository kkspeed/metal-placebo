#[macro_use]
extern crate rswm;
extern crate x11;

use std::io;
use std::io::{Read, Write};
use std::os::raw::{c_uchar, c_uint};
use std::process;
use x11::{keysym, xlib};

use rswm::client::ClientW;
use rswm::config::*;
use rswm::core;
use rswm::loggers;
use rswm::layout::{Tile, FullScreen, Overview, Layout};
use rswm::util::spawn;

const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))] =
    &[(MOD_MASK, keysym::XK_r, &|_| spawn("dmenu_run", &[])),
      (MOD_MASK, keysym::XK_t, &|_| spawn("urxvt", &[])),
      (MOD_MASK, keysym::XK_Print, &|_| spawn("scrot", &["-e", "mv $f ~/"])),
      (MOD_MASK | xlib::Mod1Mask,
       keysym::XK_Print,
       &|_| spawn("scrot", &["-s", "-e", "mv $f ~/"])),
      (MOD_MASK, keysym::XK_f, &|_| spawn("pcmanfm", &[])),
      (MOD_MASK, keysym::XK_l, &|_| spawn("i3lock", &["-c", "000000", "-n"])),
      (0, keysym::XF86XK_MonBrightnessUp, &|_| spawn("xbrightness", &["+10000"])),
      (0, keysym::XF86XK_MonBrightnessDown, &|_| spawn("xbrightness", &["-10000"])),
      (0, keysym::XF86XK_AudioRaiseVolume, &|_| spawn("amixer", &["set", "Master", "5000+"])),
      (0, keysym::XF86XK_AudioLowerVolume, &|_| spawn("amixer", &["set", "Master", "5000-"])),
      (0, keysym::XF86XK_AudioMute, &|_| spawn("amixer", &["set", "Master", "toggle"])),
      (0, keysym::XF86XK_AudioMicMute, &|_| spawn("amixer", &["set", "Capture", "toggle"])),
      (MOD_MASK,
       keysym::XK_w,
       &|w| {
        let clients = w.all_clients();
        let contents: Vec<String> =
            clients.iter().map(|c| format!("[{}] {}", c.get_class(), c.get_title())).collect();
        match dmenu_helper(contents.iter(),
                           &["-p", "window", "-i", "-l", "7", "-sb", "#000000", "-sf",
                             "#00ff00", "-nb", "#000000", "-nf", "#dddddd"]) {
            Ok(result) => {
                if let Some(position) = contents.iter().position(|s| *s == result.trim()) {
                    let c = clients[position].clone();
                    w.select_tag(c.tag());
                    w.set_focus(c);
                }
            }
            Err(_) => return,
        }
    })];

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
       })];

const TAG_KEYS: &'static (&'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))],
          &'static [c_uchar]) = &define_tags!(MOD_MASK,
                                              xlib::ShiftMask,
                                              ['1', '2', '3', '4', '5', '6', '7', '8', '9']);

const TAG_DESCRIPTION: &'static [(c_uchar, &'static str)] = &[('1' as c_uchar, "网页"),
                                                              ('2' as c_uchar, "代码")];

fn main() {
    let config = Config::default()
        .border_width(2)
        .bar_height(19)
        .addtional_keys(KEYS)
        .start_programs(START_PROGRAMS)
        .tag_keys(TAG_KEYS)
        .tag_default('1' as c_uchar)
        .rules(RULES)
        .tag_description(TAG_DESCRIPTION)
        .tag_layout(vec![('9' as c_uchar, Box::new(FullScreen)),
                         (TAG_OVERVIEW as c_uchar, Box::new(Overview))]);
    let xmobar_logger = loggers::XMobarLogger::new(loggers::LoggerConfig::default(), &[]);
    let mut window_manager = core::WindowManager::new(config);
    window_manager.set_logger(Box::new(xmobar_logger));
    window_manager.run();
}

fn dmenu_helper<'a, I>(strings: I, args: &[&str]) -> Result<String, io::Error>
    where I: Iterator<Item = &'a String>
{
    let mut child = try!(process::Command::new("dmenu")
        .args(args)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn());
    for c in strings {
        try!(writeln!(child.stdin.as_mut().unwrap(), "{}", c));
    }
    try!(child.wait());
    let mut result = String::new();
    try!(child.stdout.unwrap().read_to_string(&mut result));
    Ok(result)
}