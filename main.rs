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

const MOD_MASK: c_uint = xlib::Mod4Mask;

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

    let keys: Vec<(c_uint, c_uint, WmAction)> =
        vec![(MOD_MASK, keysym::XK_r, Box::new(|_| spawn("dmenu_run", &[]))),
             (MOD_MASK, keysym::XK_t, Box::new(|_| spawn("urxvt", &[]))),
             (MOD_MASK, keysym::XK_Print, Box::new(|_| spawn("scrot", &["-e", "mv $f ~/"]))),
             (MOD_MASK | xlib::Mod1Mask,
              keysym::XK_Print,
              Box::new(|_| spawn("scrot", &["-s", "-e", "mv $f ~/"]))),
             (MOD_MASK, keysym::XK_f, Box::new(|_| spawn("pcmanfm", &[]))),
             (MOD_MASK, keysym::XK_l, Box::new(|_| spawn("i3lock", &["-c", "000000", "-n"]))),
             (0, keysym::XF86XK_MonBrightnessUp, Box::new(|_| spawn("xbrightness", &["+10000"]))),
             (0,
              keysym::XF86XK_MonBrightnessDown,
              Box::new(|_| spawn("xbrightness", &["-10000"]))),
             (0,
              keysym::XF86XK_AudioRaiseVolume,
              Box::new(|_| spawn("amixer", &["set", "Master", "5000+"]))),
             (0,
              keysym::XF86XK_AudioLowerVolume,
              Box::new(|_| spawn("amixer", &["set", "Master", "5000-"]))),
             (0,
              keysym::XF86XK_AudioMute,
              Box::new(|_| spawn("amixer", &["set", "Master", "toggle"]))),
             (0,
              keysym::XF86XK_AudioMicMute,
              Box::new(|_| spawn("amixer", &["set", "Capture", "toggle"]))),
             (MOD_MASK | xlib::ShiftMask,
              keysym::XK_t,
              Box::new(extra::add_workspace_user_tag_dmenu)),
             (MOD_MASK | xlib::ShiftMask,
              keysym::XK_w,
              Box::new(extra::add_window_user_tag_dmenu)),
             (MOD_MASK, keysym::XK_w, Box::new(extra::select_window_dmenu)),
             (MOD_MASK, keysym::XK_y, Box::new(|w| w.set_focus_index(Some(0)))),
             (MOD_MASK, keysym::XK_u, Box::new(|w| w.set_focus_index(Some(1)))),
             (MOD_MASK, keysym::XK_i, Box::new(|w| w.set_focus_index(Some(2)))),
             (MOD_MASK, keysym::XK_o, Box::new(|w| w.set_focus_index(Some(3)))),
             (MOD_MASK, keysym::XK_p, Box::new(|w| w.set_focus_index(None))),
             (MOD_MASK, keysym::XK_Tab, Box::new(|w| w.toggle_back()))];

    let start_programs: Vec<StartAction> =
        vec![Box::new(|| spawn("xcompmgr", &[])),
             Box::new(|| spawn("fcitx", &[])),
             Box::new(|| spawn("tilda", &["--hidden"])),
             Box::new(|| spawn("/usr/lib/polkit-kde/polkit-kde-authentication-agent-1", &[]))];

    let rules: Vec<(ClientPredicate, ClientAction)> =
        vec![(Box::new(|c| c.get_class().as_str() == "Gimp"), Box::new(|c| c.set_floating(true))),
             (Box::new(|c| c.is_dialog()), Box::new(|c| c.set_floating(true))),
             (Box::new(|c| c.get_class().as_str() == "VirtualBox"),
              Box::new(|c| c.set_floating(true))),
             (Box::new(|c| c.get_class().as_str() == "Tilda"),
              Box::new(|c| {
                  c.set_floating(true);
                  c.set_sticky(true);
              }))];

    let tag_description: Vec<(c_uchar, String)> = vec![('1' as c_uchar, "web".into()),
                                                       ('2' as c_uchar, "code".into())];

    let config = Config::new(MOD_MASK)
        .border_width(2)
        .bar_height(18)
        .addtional_keys(keys)
        .start_programs(start_programs)
        .tag_keys(define_tags!(MOD_MASK,
                               xlib::ShiftMask,
                               ['1', '2', '3', '4', '5', '6', '7', '8', '9']))
        .tag_default('1' as c_uchar)
        .rules(rules)
        .tag_description(tag_description)
        .tag_layout(vec![('3' as c_uchar, Box::new(Tile13 { layout: Box::new(FullScreen) })),
                         ('9' as c_uchar, Box::new(FullScreen)),
                         (TAG_OVERVIEW as c_uchar, Box::new(Overview))]);
    let logger_config = loggers::LoggerConfig::default()
        .client_title_length(8)
        .client_template("<fc=#CCCCCC,#006048> <{{& tag }}> {{& content }} </fc>")
        .client_selected_template("<fc=#2f2f2f,#00BFFF> <{{& tag }}> {{& content }} </fc>")
        .tag_selected_template("<fc=#FFFFFF,#D81D4E> {{& content }} </fc>")
        .tag_template("<fc=#66595C,#FAF6EC> <action=`xdotool key super+{{& tag }}` button=1>{{& \
                       content }}</action> </fc>")
        .separator("<fc=#000000,#00FA9A> </fc>");
    let xmobar_logger = loggers::XMobarLogger::new(logger_config, &[]);
    let mut window_manager = core::WindowManager::new(config);
    window_manager.set_logger(Box::new(xmobar_logger));
    window_manager.run();
}
