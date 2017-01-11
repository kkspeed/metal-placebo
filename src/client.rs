use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::ffi::CString;
use std::io::Write;
use std::os::raw::{c_long, c_int, c_uchar, c_uint, c_ulong, c_void};
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::slice;

use x11::xlib;

use atoms;
use config::Config;
use util;

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

impl Default for Rect {
    fn default() -> Rect {
        Rect {
            x: -1,
            y: -1,
            width: -1,
            height: -1,
        }
    }
}

impl Rect {
    pub fn new(x: c_int, y: c_int, width: c_int, height: c_int) -> Rect {
        Rect {
            x: x,
            y: y,
            width: width,
            height: height,
        }
    }

    pub fn intersect(&self, other: &Rect) -> bool {
        self.contains_point(other.x, other.y) ||
        self.contains_point(other.x + other.width, other.y) ||
        self.contains_point(other.x, other.y + other.height) ||
        self.contains_point(other.x + other.width, other.y + other.height) ||
        other.contains_point(self.x, self.y) ||
        other.contains_point(self.x + self.width, self.y) ||
        other.contains_point(self.x, self.y + self.height) ||
        other.contains_point(self.x + self.width, self.y + self.height)
    }

    pub fn contains_point(&self, x: c_int, y: c_int) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

pub struct Client {
    anchor_window: xlib::Window,
    config: Rc<Config>,
    pub tag: c_uchar,
    pub title: String,
    root: xlib::Window,
    class: String,
    was_floating: bool,
    is_floating: bool,
    is_sticky: bool,
    is_dialog: bool,
    is_maximized: bool,
    is_fullscreen: bool,
    is_dock: bool,
    is_above: bool,
    normal_border_color: c_ulong,
    focused_border_color: c_ulong,
    pub display: *mut xlib::Display,
    pub window: c_ulong,
    pub old_rect: Rect,
    pub rect: Rect,
    pub border: c_int,
    pub old_border: c_int,
    pub weight: i32,
    extras: HashMap<String, Rc<String>>,
}

impl Client {
    pub fn new(config: Rc<Config>,
               display: *mut xlib::Display,
               root: xlib::Window,
               window: c_ulong,
               anchor_window: xlib::Window,
               tag: c_uchar)
               -> Client {
        let mut class_hint: xlib::XClassHint = unsafe { zeroed() };
        let mut client = Client {
            anchor_window: anchor_window,
            config: config,
            display: display,
            root: root,
            window: window,
            tag: tag,
            title: "broken".to_string(),
            class: "broken".to_string(),
            was_floating: false,
            is_floating: false,
            is_dialog: false,
            is_sticky: false,
            is_maximized: false,
            is_fullscreen: false,
            is_dock: false,
            is_above: false,
            focused_border_color: 0,
            normal_border_color: 0,
            old_rect: Rect::default(),
            rect: Rect::default(),
            border: 0,
            old_border: 0,
            weight: -1,
            extras: HashMap::new(),
        };
        unsafe {
            xlib::XGetClassHint(display, window, &mut class_hint);
            if !class_hint.res_class.is_null() {
                client.class =
                    CString::from_raw(class_hint.res_class).to_string_lossy().into_owned();
            }
        }
        client
    }

    pub fn save_window_size(&mut self) {
        self.old_rect = self.rect.clone();
        log!("Save window size: old rect: {:?}", self.old_rect);
    }

    pub fn set_size(&mut self, x: c_int, y: c_int, width: c_int, height: c_int) {
        self.rect = Rect {
            x: x,
            y: y,
            width: width,
            height: height,
        };
    }
}

#[derive(Clone)]
pub struct ClientW(Rc<RefCell<Client>>);
pub type ClientL = Vec<ClientW>;

impl ClientW {
    pub fn new(config: Rc<Config>,
               display: *mut xlib::Display,
               root: xlib::Window,
               window: c_ulong,
               anchor_window: xlib::Window,
               tag: c_uchar)
               -> ClientW {
        ClientW(Rc::new(RefCell::new(Client::new(config,
                                                 display,
                                                 root,
                                                 window,
                                                 anchor_window,
                                                 tag))))
    }

    pub fn borrow(&self) -> Ref<Client> {
        self.0.borrow()
    }

    pub fn borrow_mut(&mut self) -> RefMut<Client> {
        self.0.borrow_mut()
    }

    pub fn get_title(&self) -> String {
        self.borrow().title.clone()
    }

    pub fn update_title(&mut self) {
        self.borrow_mut().title =
            util::get_text_prop(self.display(), self.window(), atoms::net_wm_name())
                .or(util::get_text_prop(self.display(), self.window(), xlib::XA_WM_NAME))
                .unwrap_or("Unknown".to_string());
    }

    pub fn get_class<'a>(&self) -> String {
        // TODO: Revisit: unnecessary clone.
        self.borrow().class.clone()
    }

    pub fn is_fullscreen(&self) -> bool {
        self.borrow().is_fullscreen
    }

    pub fn is_floating(&self) -> bool {
        self.borrow().is_floating
    }

    pub fn is_above(&self) -> bool {
        self.borrow().is_above
    }

    pub fn is_sticky(&self) -> bool {
        self.borrow().is_sticky
    }

    pub fn is_dock(&self) -> bool {
        self.borrow().is_dock
    }

    pub fn is_dialog(&self) -> bool {
        if let Some(atom) = self.get_atom(atoms::net_wm_window_type()) {
            atom == atoms::net_wm_window_type_dialog()
        } else {
            false
        }
    }

    pub fn is_maximized(&self) -> bool {
        self.borrow().is_maximized
    }

    pub fn get_extra(&self, key: &str) -> Option<Rc<String>> {
        self.borrow().extras.get(key).map(|c| c.clone())
    }

    pub fn put_extra(&mut self, key: String, val: String) {
        self.borrow_mut().extras.insert(key, Rc::new(val));
    }

    pub fn get_atom(&self, atom: xlib::Atom) -> Option<xlib::Atom> {
        let mut da: xlib::Atom = 0;
        let mut di: c_int = 0;
        let mut dl: c_ulong = 0;
        let mut c: *mut c_uchar = null_mut();
        unsafe {
            let ret = xlib::XGetWindowProperty(self.display(),
                                               self.window(),
                                               atom,
                                               0,
                                               size_of::<xlib::Atom>() as c_long,
                                               0,
                                               xlib::XA_ATOM,
                                               &mut da,
                                               &mut di,
                                               &mut dl,
                                               &mut dl,
                                               &mut c);
            if ret == xlib::Success as c_int && !c.is_null() {
                log!("Success atom!");
                let result = *(c as *mut xlib::Atom);
                log!("Success deref!");
                xlib::XFree(c as *mut c_void);
                log!("Return!");
                return Some(result);
            }
            None
        }
    }

    pub fn set_floating(&mut self, floating: bool) {
        let was_floating = self.borrow().is_floating;
        self.borrow_mut().was_floating = was_floating;
        self.borrow_mut().is_floating = floating;
    }

    pub fn set_sticky(&mut self, sticky: bool) {
        self.borrow_mut().is_sticky = sticky;
    }

    pub fn set_dock(&mut self, dock: bool) {
        self.borrow_mut().is_dock = dock;
    }

    pub fn set_above(&mut self, above: bool) {
        self.borrow_mut().is_above = above;
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        self.borrow_mut().is_maximized = maximized;
    }

    pub fn set_fullscreen(&mut self, rect: Rect, fullscreen: bool) {
        if fullscreen {
            unsafe {
                xlib::XChangeProperty(self.display(),
                                      self.window(),
                                      atoms::net_wm_state(),
                                      xlib::XA_ATOM,
                                      32,
                                      xlib::PropModeReplace,
                                      (&atoms::net_wm_state_fullscreen() as *const u64) as *const u8,
                                      1);
            }
            self.borrow_mut().is_fullscreen = true;
            self.set_floating(true);
            self.resize(rect, false);
            self.raise_window();
        } else {
            unsafe {
                xlib::XChangeProperty(self.display(),
                                      self.window(),
                                      atoms::net_wm_state(),
                                      xlib::XA_ATOM,
                                      32,
                                      xlib::PropModeReplace,
                                      null(),
                                      0);
            }
            let was_floating = self.borrow().was_floating;
            self.set_floating(was_floating);
            self.borrow_mut().is_fullscreen = false;
            let old_rect = self.borrow().old_rect.clone();
            self.resize(old_rect, false);
        }
    }

    pub fn display(&self) -> *mut xlib::Display {
        self.borrow().display
    }
    pub fn tag(&self) -> c_uchar {
        self.borrow().tag
    }

    pub fn window(&self) -> c_ulong {
        self.borrow().window
    }

    pub fn show(&mut self, visible: bool) {
        let rect = self.get_rect();
        if visible {
            self.move_window(rect.x, rect.y, false);
            self.invalidate();
        } else {
            self.move_window(-10 * rect.width, rect.y, false);
        }
    }

    pub fn get_rect(&self) -> Rect {
        self.borrow().rect.clone()
    }

    pub fn move_window(&mut self, x: c_int, y: c_int, save: bool) {
        if save {
            self.borrow_mut().save_window_size();
            self.borrow_mut().rect.x = x;
            self.borrow_mut().rect.y = y;
        }
        unsafe {
            xlib::XMoveWindow(self.borrow().display, self.borrow().window, x, y);
        }
    }

    pub fn invalidate(&mut self) {
        // TODO: This is a very ugly solution to invalidate the window that requires
        // resize to repaint, especially gtk 2 windows: emacs, lxterminal etc.
        let mut rect = self.get_rect();
        rect.width = rect.width + 1;
        self.resize(rect.clone(), false);
        rect.width = rect.width - 1;
        self.resize(rect, false);
    }

    pub fn resize(&mut self, rect: Rect, temporary: bool) {
        if !temporary {
            self.borrow_mut().save_window_size();
            self.borrow_mut().set_size(rect.x, rect.y, rect.width, rect.height);
        }
        let mut xc: xlib::XWindowChanges = unsafe { zeroed() };
        xc.x = rect.x;
        xc.y = rect.y;
        xc.width = rect.width;
        xc.height = rect.height;
        unsafe {
            xlib::XConfigureWindow(self.display(),
                                   self.window(),
                                   (xlib::CWX | xlib::CWY | xlib::CWWidth | xlib::CWHeight) as u32,
                                   &mut xc);
            xlib::XSync(self.display(), 0);
        }
    }

    pub fn configure(&mut self) {
        let mut ce: xlib::XConfigureEvent = unsafe { zeroed() };
        let rect = self.get_rect();
        ce.type_ = xlib::ConfigureNotify;
        ce.display = self.display();
        ce.event = self.window();
        ce.x = rect.x;
        ce.y = rect.y;
        ce.width = rect.width;
        ce.height = rect.height;
        ce.border_width = self.borrow().border;
        ce.above = 0;
        ce.override_redirect = 0;
        unsafe {
            xlib::XSendEvent(self.display(),
                             self.window(),
                             0,
                             xlib::StructureNotifyMask,
                             &mut xlib::XEvent::from(ce));
        }
    }

    pub fn set_state(&self, state: c_ulong) {
        let data = vec![state, 0];
        unsafe {
            xlib::XChangeProperty(self.borrow().display,
                                  self.borrow().window,
                                  atoms::wm_state(),
                                  atoms::wm_state(),
                                  32,
                                  xlib::PropModeReplace,
                                  data.as_ptr() as *const u8,
                                  2);
        }
    }

    pub fn send_event(&self, proto: xlib::Atom) -> bool {
        let mut exists = false;
        unsafe {
            let mut n: c_int = 0;
            let mut p: *mut xlib::Atom = zeroed();
            if xlib::XGetWMProtocols(self.display(), self.window(), &mut p, &mut n) != 0 {
                let protocols: &[xlib::Atom] = slice::from_raw_parts(p, n as usize);
                exists = protocols.iter().any(|c| *c == proto);
                xlib::XFree(p as *mut c_void);
            }
            if exists {
                log!("Send event: {}", proto);
                let mut ev: xlib::XClientMessageEvent = zeroed();
                ev.type_ = xlib::ClientMessage; // wtf?
                ev.window = self.window();
                ev.message_type = atoms::wm_protocols();
                ev.format = 32;
                ev.data.set_long(0, proto as c_long);
                ev.data.set_long(1, xlib::CurrentTime as c_long);
                let status = xlib::XSendEvent(self.display(),
                                              self.window(),
                                              0,
                                              xlib::NoEventMask,
                                              &mut xlib::XEvent::from(ev));
                log!("Status: {}", status);
            }
        }
        exists
    }

    pub fn raise_window(&self) {
        unsafe {
            xlib::XRaiseWindow(self.display(), self.window());
            xlib::XSync(self.display(), 0);
        }
    }

    pub fn lower_window(&self) {
        unsafe {
            xlib::XLowerWindow(self.display(), self.window());
            xlib::XSync(self.display(), 0);
        }
    }

    pub fn set_border_color(&mut self, normal: c_ulong, focused: c_ulong) {
        self.borrow_mut().focused_border_color = focused;
        self.borrow_mut().normal_border_color = normal;
    }

    pub fn grab_buttons(&mut self, focused: bool) {
        unsafe {
            xlib::XUngrabButton(self.display(),
                                xlib::AnyButton as c_uint,
                                xlib::AnyModifier,
                                self.window());
            if !focused {
                xlib::XGrabButton(self.display(),
                                  xlib::AnyButton as c_uint,
                                  xlib::AnyModifier,
                                  self.window(),
                                  0,
                                  (xlib::ButtonPressMask | xlib::ButtonReleaseMask) as c_uint,
                                  xlib::GrabModeAsync,
                                  xlib::GrabModeAsync,
                                  0,
                                  0);
            }
        }
    }

    pub fn focus(&self, focus: bool) {
        if focus {
            unsafe {
                xlib::XSetWindowBorder(self.display(),
                                       self.window(),
                                       self.borrow().focused_border_color);
                xlib::XChangeProperty(self.display(),
                                      self.borrow().root,
                                      atoms::net_active_window(),
                                      xlib::XA_WINDOW,
                                      32,
                                      xlib::PropModeReplace,
                                      &self.window() as *const u64 as *const u8,
                                      1);
                xlib::XSetInputFocus(self.display(),
                                     self.window(),
                                     xlib::RevertToPointerRoot,
                                     xlib::CurrentTime);
            }
            self.send_event(atoms::wm_take_focus());
        } else {
            unsafe {
                xlib::XSetWindowBorder(self.display(),
                                       self.window(),
                                       self.borrow().normal_border_color);
            }
        }
    }
}


pub trait ClientList {
    type Item;
    fn new() -> Self;
    fn get_client_by_window(&self, window: c_ulong) -> Option<Self::Item>;
    fn rank(&mut self);
    fn show(&mut self);
    fn select_clients(&mut self,
                      predicate: &Fn(&Self::Item) -> bool,
                      rank: bool,
                      yes_action: &Fn(&mut Self::Item),
                      no_action: &Fn(&mut Self::Item))
                      -> Self;
}

impl ClientList for ClientL {
    type Item = ClientW;
    fn new() -> Vec<ClientW> {
        Vec::new()
    }

    fn get_client_by_window(&self, window: c_ulong) -> Option<ClientW> {
        self.iter()
            .find(|c| c.borrow().window == window)
            .map(|c| c.clone())
    }

    fn rank(&mut self) {
        self.sort_by_key(|c| (c.is_sticky(), c.tag(), c.is_floating(), -c.borrow().weight));
    }

    fn show(&mut self) {
        for c in self {
            c.show(true);
        }
    }

    fn select_clients(&mut self,
                      predicate: &Fn(&ClientW) -> bool,
                      rank: bool,
                      yes_action: &Fn(&mut ClientW),
                      no_action: &Fn(&mut ClientW))
                      -> ClientL {
        let mut result = Self::new();
        for c in self {
            if predicate(c) {
                yes_action(c);
                result.push(c.clone());
            } else {
                no_action(c);
            }
        }
        if rank {
            result.rank();
        }
        result
    }
}
