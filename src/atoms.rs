use std::ffi::CString;
use std::os::raw::c_ulong;
use x11::xlib;

#[derive(Debug)]
pub struct Atoms {
    pub wm_protocols: c_ulong,
    pub wm_delete: c_ulong,
    pub wm_state: c_ulong,
    pub wm_take_focus: c_ulong,
    pub net_active_window: c_ulong,
    pub net_supported: c_ulong,
    pub net_wm_name: c_ulong,
    pub net_wm_state: c_ulong,
    pub net_wm_state_above: c_ulong,
    pub net_wm_state_fullscreen: c_ulong,
    pub net_wm_state_sticky: c_ulong,
    pub net_wm_window_type: c_ulong,
    pub net_wm_window_type_dialog: c_ulong,
    pub net_wm_window_type_dock: c_ulong,
    pub net_client_list: c_ulong,
}

impl Atoms {
    pub fn create_atom(display: *mut xlib::Display) -> Atoms {
        Atoms {
            wm_protocols: Atoms::intern_atom(display, "WM_PROTOCOLS"),
            wm_delete: Atoms::intern_atom(display, "WM_DELETE_WINDOW"),
            wm_state: Atoms::intern_atom(display, "WM_STATE"),
            wm_take_focus: Atoms::intern_atom(display, "WM_TAKE_FOCUS"),
            net_active_window: Atoms::intern_atom(display, "_NET_ACTIVE_WINDOW"),
            net_supported: Atoms::intern_atom(display, "_NET_SUPPORTED"),
            net_wm_name: Atoms::intern_atom(display, "_NET_SUPPORTED"),
            net_wm_state: Atoms::intern_atom(display, "_NET_WM_STATE"),
            net_wm_state_above: Atoms::intern_atom(display, "_NET_WM_STATE_ABOVE"),
            net_wm_state_sticky: Atoms::intern_atom(display, "_NET_WM_STATE_STICKY"),
            net_wm_state_fullscreen: Atoms::intern_atom(display, "_NET_WM_STATE_FULLSCREEN"),
            net_wm_window_type: Atoms::intern_atom(display, "_NET_WM_WINDOW_TYPE"),
            net_wm_window_type_dialog: Atoms::intern_atom(display, "_NET_WM_WINDOW_TYPE_DIALOG"),
            net_wm_window_type_dock: Atoms::intern_atom(display, "_NET_WM_WINDOW_TYPE_DOCK"),
            net_client_list: Atoms::intern_atom(display, "_NET_CLIENT_LIST"),
        }
    }

    fn intern_atom(display: *mut xlib::Display, atom: &str) -> c_ulong {
        unsafe { xlib::XInternAtom(display, CString::new(atom).unwrap().as_ptr(), 0) }
    }
}