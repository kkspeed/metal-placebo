use std::ffi::CString;
use x11::xlib;

static mut WM_PROTOCOLS: xlib::Atom = 0;
pub fn wm_protocols() -> xlib::Atom {
    unsafe { WM_PROTOCOLS }
}

static mut WM_DELETE: xlib::Atom = 0;
pub fn wm_delete() -> xlib::Atom {
    unsafe { WM_DELETE }
}

static mut WM_STATE: xlib::Atom = 0;
pub fn wm_state() -> xlib::Atom {
    unsafe { WM_STATE }
}

static mut WM_TAKE_FOCUS: xlib::Atom = 0;
pub fn wm_take_focus() -> xlib::Atom {
    unsafe { WM_TAKE_FOCUS }
}

static mut NET_ACTIVE_WINDOW: xlib::Atom = 0;
pub fn net_active_window() -> xlib::Atom {
    unsafe { NET_ACTIVE_WINDOW }
}

static mut NET_SUPPORTED: xlib::Atom = 0;
pub fn net_supported() -> xlib::Atom {
    unsafe { NET_SUPPORTED }
}

static mut NET_WM_NAME: xlib::Atom = 0;
pub fn net_wm_name() -> xlib::Atom {
    unsafe { NET_WM_NAME }
}

static mut NET_WM_STATE: xlib::Atom = 0;
pub fn net_wm_state() -> xlib::Atom {
    unsafe { NET_WM_STATE }
}

static mut NET_WM_STATE_ABOVE: xlib::Atom = 0;
pub fn net_wm_state_above() -> xlib::Atom {
    unsafe { NET_WM_STATE_ABOVE }
}

static mut NET_WM_STATE_FULLSCREEN: xlib::Atom = 0;
pub fn net_wm_state_fullscreen() -> xlib::Atom {
    unsafe { NET_WM_STATE_FULLSCREEN }
}

static mut NET_WM_STATE_STICKY: xlib::Atom = 0;
pub fn net_wm_state_sticky() -> xlib::Atom {
    unsafe { NET_WM_STATE_STICKY }
}

static mut NET_WM_WINDOW_TYPE: xlib::Atom = 0;
pub fn net_wm_window_type() -> xlib::Atom {
    unsafe { NET_WM_WINDOW_TYPE }
}

static mut NET_WM_WINDOW_TYPE_DIALOG: xlib::Atom = 0;
pub fn net_wm_window_type_dialog() -> xlib::Atom {
    unsafe { NET_WM_WINDOW_TYPE_DIALOG }
}

static mut NET_WM_WINDOW_TYPE_DOCK: xlib::Atom = 0;
pub fn net_wm_window_type_dock() -> xlib::Atom {
    unsafe { NET_WM_WINDOW_TYPE_DOCK }
}

static mut NET_CLIENT_LIST: xlib::Atom = 0;
pub fn net_client_list() -> xlib::Atom {
    unsafe { NET_CLIENT_LIST }
}

pub fn create_atoms(display: *mut xlib::Display) {
    unsafe {
        WM_PROTOCOLS = intern_atom(display, "WM_PROTOCOLS");
        WM_DELETE = intern_atom(display, "WM_DELETE_WINDOW");
        WM_STATE = intern_atom(display, "WM_STATE");
        WM_TAKE_FOCUS = intern_atom(display, "WM_TAKE_FOCUS");
        NET_ACTIVE_WINDOW = intern_atom(display, "_NET_ACTIVE_WINDOW");
        NET_SUPPORTED = intern_atom(display, "_NET_SUPPORTED");
        NET_WM_NAME = intern_atom(display, "_NET_SUPPORTED");
        NET_WM_STATE = intern_atom(display, "_NET_WM_STATE");
        NET_WM_STATE_ABOVE = intern_atom(display, "_NET_WM_STATE_ABOVE");
        NET_WM_STATE_STICKY = intern_atom(display, "_NET_WM_STATE_STICKY");
        NET_WM_STATE_FULLSCREEN = intern_atom(display, "_NET_WM_STATE_FULLSCREEN");
        NET_WM_WINDOW_TYPE = intern_atom(display, "_NET_WM_WINDOW_TYPE");
        NET_WM_WINDOW_TYPE_DIALOG = intern_atom(display, "_NET_WM_WINDOW_TYPE_DIALOG");
        NET_WM_WINDOW_TYPE_DOCK = intern_atom(display, "_NET_WM_WINDOW_TYPE_DOCK");
        NET_CLIENT_LIST = intern_atom(display, "_NET_CLIENT_LIST");
    }
}

fn intern_atom(display: *mut xlib::Display, atom: &str) -> xlib::Atom {
    unsafe { xlib::XInternAtom(display, CString::new(atom).unwrap().as_ptr(), 0) }
}
