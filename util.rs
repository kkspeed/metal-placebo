use std::io::Write;
use std::os::raw::c_int;
use std::process;

use x11::xlib;
use xproto;

#[macro_export]
macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

#[macro_export]
macro_rules! x_disable_error_unsafe {
    ( $display: expr, $s: block ) => {
        unsafe {
            xlib::XGrabServer($display);
            xlib::XSetErrorHandler(Some(util::xerror_dummy));
            $s
            xlib::XSync($display, 0);
            xlib::XSetErrorHandler(Some(util::xerror));
            xlib::XUngrabServer($display);
        }
    }
}

#[macro_export]
macro_rules! define_tags (
    ( $modkey: expr, $mod_mask: expr, [$($x: expr), *]) => {
        [
            $(($modkey, $x as c_uint, &|w| w.select_tag($x as c_uchar)), )*
            $(($modkey | $mod_mask, $x as c_uint, &|w| w.add_tag($x as c_uchar)),)*
        ]
    };
);

#[allow(unused_variables)]
pub extern "C" fn xerror_dummy(display: *mut xlib::Display,
                               event: *mut xlib::XErrorEvent)
                               -> c_int {
    let e: xlib::XErrorEvent = unsafe { *event };
    log!("[WARN] Got error {} from request {}",
         e.error_code,
         e.request_code);
    0
}

pub fn clean_mask(keycode: u32) -> u32 {
    keycode & !xlib::LockMask &
    (xlib::Mod1Mask | xlib::Mod2Mask | xlib::Mod3Mask | xlib::Mod4Mask | xlib::Mod5Mask |
     xlib::ShiftMask | xlib::ControlMask)
}

#[allow(unused_must_use)]
pub fn spawn(command: &str) {
    process::Command::new(command).spawn();
}

#[allow(unused_variables)]
pub extern "C" fn xerror(dpy: *mut xlib::Display, err: *mut xlib::XErrorEvent) -> c_int {
    let ee = unsafe { *err };
    if ee.error_code == xlib::BadWindow ||
       (ee.request_code == xproto::X_SetInputFocus && ee.error_code == xlib::BadMatch) ||
       (ee.request_code == xproto::X_PolyText8 && ee.error_code == xlib::BadDrawable) ||
       (ee.request_code == xproto::X_PolyFillRectangle && ee.error_code == xlib::BadDrawable) ||
       (ee.request_code == xproto::X_PolySegment && ee.error_code == xlib::BadDrawable) ||
       (ee.request_code == xproto::X_ConfigureWindow && ee.error_code == xlib::BadMatch) ||
       (ee.request_code == xproto::X_GrabButton && ee.error_code == xlib::BadAccess) ||
       (ee.request_code == xproto::X_GrabKey && ee.error_code == xlib::BadAccess) ||
       (ee.request_code == xproto::X_CopyArea && ee.error_code == xlib::BadDrawable) {
        return 0;
    }
    log!("rswm: fatal error: request code={}, error code={}\n",
			ee.request_code, ee.error_code);
    process::exit(1);
}
