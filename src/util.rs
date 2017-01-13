use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};
use std::mem::zeroed;
use std::process;

use x11::xlib;

use xproto;

#[macro_export]
macro_rules! x_disable_error_unsafe {
    ( $display: expr, $s: block ) => {
        unsafe {
            debug!("grab server");
            xlib::XGrabServer($display);
            xlib::XSetErrorHandler(Some(util::xerror_dummy));
            $s
            xlib::XSync($display, 0);
            xlib::XSetErrorHandler(Some(util::xerror));
            xlib::XUngrabServer($display);
            debug!("ungrab server");
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
    debug!("[WARN] Got error {} from request {}",
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
            let st = CString::from_raw(name.value as *mut c_char)
                .to_string_lossy()
                .to_string();
            result = Some(st);
        } else {
            if xlib::Xutf8TextPropertyToTextList(display, &mut name, &mut list, &mut n) >=
               xlib::Success as c_int && n > 0 && !(*list).is_null() {
                let st = CString::from_raw(*list).to_string_lossy().to_string();
                result = Some(st);
                // xlib::XFreeStringList(list);
            }
        }
        // TODO: Possible memory leak but adding the following statement will result in double
        // free.
        // xlib::XFree(name.value as *mut c_void);
    }
    result
}

pub fn get_root_pointer(display: *mut xlib::Display, root: xlib::Window) -> Option<(c_int, c_int)> {
    let mut x = 0;
    let mut y = 0;
    let mut di = 0;
    let mut dui = 0;
    let mut ddui = 0;
    let mut dummy: xlib::Window = 0;

    let result = unsafe {
        xlib::XQueryPointer(display,
                            root,
                            &mut dummy,
                            &mut dummy,
                            &mut x,
                            &mut y,
                            &mut dui,
                            &mut di,
                            &mut ddui)
    };

    if result == 0 { None } else { Some((x, y)) }
}

pub fn clean_mask(keycode: c_uint) -> c_uint {
    keycode & !xlib::LockMask &
    (xlib::Mod1Mask | xlib::Mod2Mask | xlib::Mod3Mask | xlib::Mod4Mask | xlib::Mod5Mask |
     xlib::ShiftMask | xlib::ControlMask)
}

pub fn spawn(command: &str, args: &[&str]) {
    match process::Command::new(command).args(args).spawn() {
        Ok(_) => (),
        Err(s) => debug!("fail to spawn process {}, error: {}", command, s),
    }
}

pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
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
        debug!("rswm: error: request code={}, error code={}", ee.request_code, ee.error_code);
        return 0;
    }
    error!("rswm: fatal error: request code={}, error code={}\n",
			ee.request_code, ee.error_code);
    process::exit(1);
}
