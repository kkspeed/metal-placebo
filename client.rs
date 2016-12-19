use std::cell::{Ref, RefCell, RefMut};
use std::io::Write;
use std::os::raw::{c_long, c_int, c_uchar, c_uint, c_ulong, c_void};
use std::mem::zeroed;
use std::rc::Rc;
use std::slice;

use x11::xlib;

use atoms::Atoms;

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
    atoms: Rc<Atoms>,
    pub tag: c_uchar,
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
            display: unsafe { zeroed() },
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
               window: c_ulong,
               tag: c_uchar,
               atoms: Rc<Atoms>)
               -> Client {
        Client {
            display: display,
            window: window,
            tag: tag,
            atoms: atoms,
            ..Default::default()
        }
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
               window: c_ulong,
               tag: c_uchar,
               atoms: Rc<Atoms>)
               -> ClientW {
        ClientW(Rc::new(RefCell::new(Client::new(display, window, tag, atoms))))
    }

    pub fn borrow(&self) -> Ref<Client> {
        self.0.borrow()
    }

    pub fn borrow_mut(&mut self) -> RefMut<Client> {
        self.0.borrow_mut()
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

    pub fn resize(&mut self, rect: Rect) {
        self.borrow_mut().save_window_size();
        self.borrow_mut().set_size(rect.x, rect.y, rect.width, rect.height);
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
        self.sort_by_key(|c| (c.tag(), -c.borrow().weight));
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