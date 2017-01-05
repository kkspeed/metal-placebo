use std::ffi::CString;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::mem::zeroed;
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

macro_rules! trace(
    ($($arg:tt)*) => { {
        if TRACE {
            log!($($arg)*);
        }
    } }
);

#[macro_export]
macro_rules! x_disable_error_unsafe {
    ( $display: expr, $s: block ) => {
        unsafe {
            log!("Grab server");
            xlib::XGrabServer($display);
            xlib::XSetErrorHandler(Some(util::xerror_dummy));
            $s
            xlib::XSync($display, 0);
            xlib::XSetErrorHandler(Some(util::xerror));
            xlib::XUngrabServer($display);
            log!("Ungrab server");
        }
    }
}

#[macro_export]
macro_rules! define_tags (
    ( $modkey: expr, $mod_mask: expr, [$($x: expr), *]) => {
        (&[
            $(($modkey, $x as c_uint, &|w| w.select_tag($x as c_uchar)), )*
            $(($modkey | $mod_mask, $x as c_uint, &|w| w.add_tag($x as c_uchar)),)*
        ], &[$($x as c_uchar, )*])
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

pub fn get_text_prop(display: *mut xlib::Display,
                     window: xlib::Window,
                     atom: xlib::Atom)
                     -> Option<String> {
    let mut list: *mut (*mut c_char) = unsafe { zeroed() };
    let mut name: xlib::XTextProperty = unsafe { zeroed() };
    let mut n: c_int = 0;
    let mut result = None;
    unsafe {
        xlib::XGetTextProperty(display, window, &mut name, atom);
        if name.nitems == 0 {
            return None;
        }
        if name.encoding == xlib::XA_STRING {
            log!("Get string");
            let st = CString::from_raw(name.value as *mut c_char)
                .to_string_lossy()
                .to_string();
            log!("Length: {} - {}", st.len(), st);
            result = Some(st);
            log!("Get string done");
        } else {
            if xlib::Xutf8TextPropertyToTextList(display, &mut name, &mut list, &mut n) >=
               xlib::Success as c_int && n > 0 && !(*list).is_null() {
                log!("Get text list!");
                let st = CString::from_raw(*list).to_string_lossy().to_string();
                log!("Text list length: {}", st.len());
                result = Some(st);
                // xlib::XFreeStringList(list);
            }
        }
        // TODO: Possible memory leak.
        // xlib::XFree(name.value as *mut c_void);
    }
    result
}

pub fn clean_mask(keycode: u32) -> u32 {
    keycode & !xlib::LockMask &
    (xlib::Mod1Mask | xlib::Mod2Mask | xlib::Mod3Mask | xlib::Mod4Mask | xlib::Mod5Mask |
     xlib::ShiftMask | xlib::ControlMask)
}

pub fn spawn(command: &str, args: &[&str]) {
    match process::Command::new(command).args(args).spawn() {
        Ok(_) => (),
        Err(s) => log!("Fail to spawn process {}, error: {}", command, s),
    }
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
        log!("rswm: error: request code={}, error code={}", ee.request_code, ee.error_code);
        return 0;
    }
    log!("rswm: fatal error: request code={}, error code={}\n",
			ee.request_code, ee.error_code);
    process::exit(1);
}
