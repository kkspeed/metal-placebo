use std::io::Write;
use std::os::raw::c_int;
use std::process;

use x11::xlib;

#[macro_export]
macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

macro_rules! x_disable_error_unsafe {
    ( $display: expr, $s: block ) => {
        unsafe {
            xlib::XGrabServer($display);
            xlib::XSetErrorHandler(Some(util::xerror_dummy));
            $s
            xlib::XSync($display, 0);
            xlib::XSetErrorHandler(None);
            xlib::XUngrabServer($display);
        }
    }
}

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