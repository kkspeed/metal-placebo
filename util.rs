use std::ffi::CString;
use std::io::Write;
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::mem::zeroed;
use std::process;

use x11::xlib;
use xproto;

use client::{ClientL, ClientW};

pub struct LoggerConfig {
    pub selected_tag_color: &'static str,
    pub tag_color: &'static str,
    pub separator_color: &'static str,
    pub selected_client_color: &'static str,
    pub client_color: &'static str,
}

pub trait Logger {
    fn new(config: &'static LoggerConfig) -> Self;
    fn dump(&mut self,
            all_clients: &ClientL,
            current_tag: c_uchar,
            current_stack: &ClientL,
            focus: &Option<usize>);
}

pub struct XMobarLogger {
    config: &'static LoggerConfig,
    child_stdin: process::ChildStdin,
}

impl Logger for XMobarLogger {
    fn new(config: &'static LoggerConfig) -> XMobarLogger {
        let process::Child { stdin: child_stdin, .. } = process::Command::new("xmobar")
            .stdin(process::Stdio::piped())
            .spawn()
            .expect("cannot spawn xmobar");
        XMobarLogger {
            config: config,
            child_stdin: child_stdin.unwrap(),
        }
    }

    fn dump(&mut self,
            all_clients: &ClientL,
            current_tag: c_uchar,
            current_stack: &ClientL,
            focus: &Option<usize>) {
        let mut tags: Vec<char> = all_clients.iter().map(|c| c.tag() as char).collect();
        tags.push(current_tag as char);
        tags.sort();
        tags.dedup();
        let mut result = String::new();
        for t in &tags {
            if *t == current_tag as char {
                if current_tag == 0 {
                    result += &format!("<fc={}> Overview </fc> |", self.config.selected_tag_color);
                } else {
                    result += &format!("<fc={}> {} </fc> |", self.config.selected_tag_color, t);
                }
            } else {
                result += &format!("<fc={}> {} </fc> |", self.config.tag_color, t);
            }
        }

        result += " :: ";
        let mut index = 99999;
        if let &Some(ref i) = focus {
            index = *i;
        }

        let mut color;
        for i in 0..current_stack.len() {
            if i == index {
                color = self.config.selected_client_color;
            } else {
                color = self.config.client_color;
            }
            result += &format!("[<fc={}>{1:.5}</fc>] ", color, current_stack[i].get_title());
        }
        writeln!(self.child_stdin, "{}", result).unwrap();
    }
}

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
        log!("Free done!");
    }
    result
}

pub fn clean_mask(keycode: u32) -> u32 {
    keycode & !xlib::LockMask &
    (xlib::Mod1Mask | xlib::Mod2Mask | xlib::Mod3Mask | xlib::Mod4Mask | xlib::Mod5Mask |
     xlib::ShiftMask | xlib::ControlMask)
}

pub fn spawn(command: &str) {
    process::Command::new(command).spawn().unwrap();
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
