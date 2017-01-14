#[macro_use]
extern crate rswm;

extern crate log;
extern crate log4rs;
extern crate x11;

use std::os::raw::{c_uchar, c_uint};
use x11::{keysym, xlib};

use rswm::client::ClientW;
use rswm::config::*;
use rswm::core;
use rswm::extra;
use rswm::loggers;
use rswm::layout::{Tile, Tile13, FullScreen, Overview, Layout};
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
      (MOD_MASK | xlib::ShiftMask, keysym::XK_t, &extra::add_workspace_user_tag_dmenu),
      (MOD_MASK | xlib::ShiftMask, keysym::XK_w, &extra::add_window_user_tag_dmenu),
      (MOD_MASK, keysym::XK_w, &extra::select_window_dmenu),
      (MOD_MASK, keysym::XK_y, &|w| w.set_focus_index(Some(0))),
      (MOD_MASK, keysym::XK_u, &|w| w.set_focus_index(Some(1))),
      (MOD_MASK, keysym::XK_i, &|w| w.set_focus_index(Some(2))),
      (MOD_MASK, keysym::XK_o, &|w| w.set_focus_index(Some(3))),
      (MOD_MASK, keysym::XK_p, &|w| w.set_focus_index(None))];

const START_PROGRAMS: &'static [&'static Fn()] =
    &[&|| spawn("xcompmgr", &[]),
      &|| spawn("fcitx", &[]),
      &|| spawn("tilda", &["--hidden"]),
      &|| spawn("/usr/lib/polkit-kde/polkit-kde-authentication-agent-1", &[])];

const RULES: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))] =
    &[(&|c| c.get_class().as_str() == "Gimp", &|c| c.set_floating(true)),
      (&|c| c.is_dialog(), &|c| c.set_floating(true)),
      (&|c| c.get_class().as_str() == "VirtualBox", &|c| c.set_floating(true)),
      (&|c| c.get_class().as_str() == "Tilda",
       &|c| {
           c.set_floating(true);
           c.set_sticky(true);
       })];

const TAG_KEYS: &'static (&'static [(c_uint, c_uint, &'static Fn(&mut core::WindowManager))],
          &'static [c_uchar]) = &define_tags!(MOD_MASK,
                                              xlib::ShiftMask,
                                              ['1', '2', '3', '4', '5', '6', '7', '8', '9']);

const TAG_DESCRIPTION: &'static [(c_uchar, &'static str)] = &[('1' as c_uchar, "web"),
                                                              ('2' as c_uchar, "code")];

fn init_logging(filter: log::LogLevelFilter) {
    use log4rs::append::file::FileAppender;
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Root};

    let root = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%H:%M:%S)} \\({l}\\)] {M}:{L}: {m}{n}")))
        .build("rswm_error.log")
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("root", Box::new(root)))
        .build(Root::builder()
            .appender("root")
            .build(filter))
        .unwrap();
    log4rs::init_config(config).expect("fail to initialize logger");
}

fn main() {
    init_logging(log::LogLevelFilter::Debug);

    let config = Config::default()
        .border_width(2)
        .bar_height(31)
        .addtional_keys(KEYS)
        .start_programs(START_PROGRAMS)
        .tag_keys(TAG_KEYS)
        .tag_default('1' as c_uchar)
        .rules(RULES)
        .tag_description(TAG_DESCRIPTION)
        .tag_layout(vec![('3' as c_uchar, Box::new(Tile13 { layout: Box::new(FullScreen) })),
                         ('9' as c_uchar, Box::new(FullScreen)),
                         (TAG_OVERVIEW as c_uchar, Box::new(Overview))]);
    let logger_config = loggers::LoggerConfig::default()
        .client_title_length(8)
        .client_template("<fc=#CCCCCC,#006048> {{& content }} </fc>")
        .client_selected_template("<fc=#2f2f2f,#00BFFF> {{& content }} </fc>")
        .tag_selected_template("<fc=#FFFFFF,#D81D4E> {{& content }} </fc>")
        .tag_template("<fc=#66595C,#FAF6EC> {{& content }} </fc>")
        .separator("<fc=#000000,#00FA9A> </fc>");
    let xmobar_logger = loggers::XMobarLogger::new(logger_config, &[]);
    let mut window_manager = core::WindowManager::new(config);
    window_manager.set_logger(Box::new(xmobar_logger));
    window_manager.run();
}