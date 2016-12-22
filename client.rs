use std::cell::{Ref, RefCell, RefMut};
use std::ffi::CString;
use std::io::Write;
use std::os::raw::{c_long, c_int, c_uchar, c_uint, c_ulong, c_void, c_char};
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::rc::Rc;
use std::slice;

use x11::xlib;

use atoms::Atoms;
use util;

#[derive(Clone)]
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

pub struct Client {
    pub atoms: Rc<Atoms>,
    pub tag: c_uchar,
    pub title: String,
    root: xlib::Window,
    class: String,
    is_floating: bool,
    is_dialog: bool,
    normal_border_color: c_ulong,
    focused_border_color: c_ulong,
    pub display: *mut xlib::Display,
    pub window: c_ulong,
    pub old_rect: Rect,
    pub rect: Rect,
    pub border: c_int,
    pub old_border: c_int,
    pub weight: i32,
}

impl Default for Client {
    fn default() -> Client {
        Client {
            atoms: unsafe { Rc::new(zeroed()) },
            tag: 0,
            title: "broken".to_string(),
            display: unsafe { zeroed() },
            root: 0,
            class: "broken".to_string(),
            is_floating: false,
            is_dialog: false,
            focused_border_color: 0,
            normal_border_color: 0,
            window: 0,
            old_rect: Rect::default(),
            rect: Rect::default(),
            border: 0,
            old_border: 0,
            weight: -1,
        }
    }
}

impl Client {
    pub fn new(display: *mut xlib::Display,
               root: xlib::Window,
               window: c_ulong,
               tag: c_uchar,
               atoms: Rc<Atoms>)
               -> Client {
        let mut class_hint: xlib::XClassHint = unsafe { zeroed() };
        let mut client = Client {
            display: display,
            root: root,
            window: window,
            tag: tag,
            atoms: atoms,
            ..Default::default()
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
    pub fn new(display: *mut xlib::Display,
               root: xlib::Window,
               window: c_ulong,
               tag: c_uchar,
               atoms: Rc<Atoms>)
               -> ClientW {
        ClientW(Rc::new(RefCell::new(Client::new(display, root, window, tag, atoms))))
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
            util::get_text_prop(self.display(), self.window(), self.atoms().net_wm_name)
                .or(util::get_text_prop(self.display(), self.window(), xlib::XA_WM_NAME))
                .unwrap_or("Unknown".to_string());
    }

    pub fn get_class<'a>(&self) -> String {
        // TODO: Revisit: unnecessary clone.
        self.borrow().class.clone()
    }

    pub fn is_floating(&self) -> bool {
        self.borrow().is_floating
    }

    pub fn atoms(&self) -> Rc<Atoms> {
        self.borrow().atoms.clone()
    }

    pub fn is_dialog(&self) -> bool {
        if let Some(atom) = self.get_atom(self.borrow().atoms.net_wm_window_type) {
            atom == self.borrow().atoms.net_wm_window_type_dialog
        } else {
            false
        }
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
        self.borrow_mut().is_floating = floating;
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
        } else {
            self.move_window(-2 * rect.width, rect.y, false);
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
                                  self.borrow().atoms.wm_state,
                                  self.borrow().atoms.wm_state,
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
                ev.message_type = self.borrow().atoms.wm_protocols;
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
        }
    }

    pub fn set_border_color(&mut self, normal: c_ulong, focused: c_ulong) {
        self.borrow_mut().focused_border_color = focused;
        self.borrow_mut().normal_border_color = normal;
    }

    pub fn focus(&self, focus: bool) {
        if focus {
            let window = self.window();
            unsafe {
                xlib::XSetWindowBorder(self.display(),
                                       self.window(),
                                       self.borrow().focused_border_color);
                xlib::XChangeProperty(self.display(),
                                      self.borrow().root,
                                      self.atoms().net_active_window,
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
        self.sort_by_key(|c| (c.tag(), c.is_floating(), -c.borrow().weight));
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
                      -> Self {
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