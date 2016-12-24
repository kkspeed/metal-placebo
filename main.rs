extern crate x11;
extern crate rswm;

use std::os::raw::c_uint;
use x11::keysym;
use rswm::client::ClientW;
use rswm::config::*;
use rswm::core;
use rswm::util::spawn;

const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))] =
    &[(MOD_MASK, keysym::XK_r, &|w| spawn("dmenu_run", &[])),
      (MOD_MASK, keysym::XK_t, &|w| spawn("xterm", &[]))];

const START_PROGRAMS: &'static [&'static Fn()] =
    &[&|| spawn("xcompmgr", &[]), &|| spawn("fcitx", &[]), &|| spawn("tilda", &["--hidden"])];

const RULES: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))] =
    &[(&|c| c.get_class() == "Gimp", &|c| c.set_floating(true)),
      (&|c| c.is_dialog(), &|c| c.set_floating(true)),
      (&|c| c.get_class() == "Tilda", &|c| c.set_floating(true))];

fn main() {
    let config = Config::default()
        .border_width(5)
        .addtional_keys(KEYS)
        .start_programs(START_PROGRAMS)
        .rules(RULES);
    let mut window_manager = core::WindowManager::new(config);
    window_manager.run();
}
