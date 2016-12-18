extern crate libc;
extern crate x11;

use std::cell::RefCell;
use std::ffi::CString;
use std::io::Write;
use std::mem::zeroed;
use std::os::raw::{c_long, c_int, c_uchar, c_uint, c_ulong, c_void};
use std::process;
use std::ptr::null;
use std::rc::Rc;

use x11::{xlib, keysym};

mod client;

macro_rules! log(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

const XC_LEFT_PTR: c_uint = 68;
const WITHDRAWN_STATE: c_ulong = 0;

const FOCUSED_BORDER_COLOR: &'static str = "RGBi:1.0/0.0/0.0";
const NORMAL_BORDER_COLOR: &'static str = "RGBi:0.0/1.0/0.0";

const BORDER_WIDTH: c_int = 5;

const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager) -> ())] =
    &[(xlib::Mod1Mask, keysym::XK_r, &|w| spawn("dmenu_run")),
      (xlib::Mod1Mask, keysym::XK_q, &|w| process::exit(0)),
      (xlib::Mod1Mask, keysym::XK_t, &|w| spawn("xterm")),
      (xlib::Mod1Mask, keysym::XK_j, &|w| w.shift_focus(1)),
      (xlib::Mod1Mask, keysym::XK_k, &|w| w.shift_focus(-1)),
      (xlib::Mod1Mask, keysym::XK_Return, &|w| w.zoom()),
      (xlib::Mod1Mask, keysym::XK_1, &|w| w.select_tag(1 as c_uchar)),
      (xlib::Mod1Mask, keysym::XK_2, &|w| w.select_tag(2 as c_uchar)),
      (xlib::Mod1Mask, keysym::XK_3, &|w| w.select_tag(3 as c_uchar)),
      (xlib::Mod1Mask, keysym::XK_0, &|w| w.select_tag(0 as c_uchar)),
      (xlib::Mod1Mask | xlib::ShiftMask, keysym::XK_1, &|w| w.add_tag(1 as c_uchar)),
      (xlib::Mod1Mask | xlib::ShiftMask, keysym::XK_2, &|w| w.add_tag(2 as c_uchar)),
      (xlib::Mod1Mask | xlib::ShiftMask, keysym::XK_3, &|w| w.add_tag(3 as c_uchar)),
      (xlib::Mod1Mask | xlib::ShiftMask, keysym::XK_0, &|w| w.add_tag(0 as c_uchar)),
      (xlib::Mod1Mask, keysym::XK_F4, &|w| w.kill_client())];

const TAG_DEFAULT: c_uchar = 1;

fn spawn(command: &str) {
    process::Command::new(command).spawn();
}

fn tile(clients: &ClientList,
        pane_x: c_int,
        pane_y: c_int,
        pane_width: c_int,
        pane_height: c_int)
        -> Vec<(c_int, c_int, c_int, c_int)> {
    let mut result =
        vec![(pane_x, pane_y, pane_width - 2 * BORDER_WIDTH, pane_height - 2 * BORDER_WIDTH)];
    let mut count = clients.len();
    let mut direction = 1;
    log!("Tiling count: {}", count);
    while count > 1 {
        let (last_x, last_y, last_width, last_height) = result.pop().unwrap();
        if direction == 1 {
            // Horizontal split
            let p1 = (last_x, last_y, last_width / 2 - BORDER_WIDTH, last_height);
            let p2 = (last_x + last_width / 2 + BORDER_WIDTH,
                      last_y,
                      last_width / 2 - BORDER_WIDTH,
                      last_height);
            result.push(p1);
            result.push(p2);
        } else {
            // Vertical split
            let p2 = (last_x,
                      last_y + last_height / 2 + BORDER_WIDTH,
                      last_width,
                      last_height / 2 - BORDER_WIDTH);
            let p1 = (last_x, last_y, last_width, last_height / 2 - BORDER_WIDTH);
            result.push(p1);
            result.push(p2);
        }
        direction = direction ^ 1;
        count = count - 1;
    }
    result
}

type ClientWindow = Rc<RefCell<Client>>;
type ClientList = Vec<ClientWindow>;

struct Client {
    tag: c_uchar,
    window: c_ulong,
    old_x: c_int,
    old_y: c_int,
    old_width: c_int,
    old_height: c_int,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border: c_int,
    old_border: c_int,
    weight: i32,
}

impl Default for Client {
    fn default() -> Client {
        Client {
            tag: TAG_DEFAULT,
            window: 0,
            old_x: -1,
            old_y: -1,
            old_width: -1,
            old_height: -1,
            x: -1,
            y: -1,
            width: -1,
            height: -1,
            border: 0,
            old_border: 0,
            weight: -1,
        }
    }
}

impl Client {
    fn new(window: c_ulong) -> Client {
        Client { window: window, ..Default::default() }
    }

    fn save_window_size(&mut self) {
        self.old_x = self.x;
        self.old_y = self.y;
        self.old_width = self.width;
        self.old_height = self.height;
    }

    fn set_size(&mut self, x: c_int, y: c_int, width: c_int, height: c_int) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
}

struct Atoms {
    wm_protocols: c_ulong,
    wm_delete: c_ulong,
    wm_state: c_ulong,
    wm_take_focus: c_ulong,
    net_active_window: c_ulong,
    net_supported: c_ulong,
    net_wm_name: c_ulong,
    net_wm_state: c_ulong,
    net_wm_fullscreen: c_ulong,
    net_wm_window_type: c_ulong,
    net_wm_window_type_dialog: c_ulong,
    net_client_list: c_ulong,
}

impl Atoms {
    fn create_atom(display: *mut xlib::Display) -> Atoms {
        Atoms {
            wm_protocols: Atoms::intern_atom(display, "WM_PROTOCOLS"),
            wm_delete: Atoms::intern_atom(display, "WM_DELETE_WINDOW"),
            wm_state: Atoms::intern_atom(display, "WM_STATE"),
            wm_take_focus: Atoms::intern_atom(display, "WM_TAKE_FOCUS"),
            net_active_window: Atoms::intern_atom(display, "_NET_ACTIVE_WINDOW"),
            net_supported: Atoms::intern_atom(display, "_NET_SUPPORTED"),
            net_wm_name: Atoms::intern_atom(display, "_NET_SUPPORTED"),
            net_wm_state: Atoms::intern_atom(display, "_NET_WM_STATE"),
            net_wm_fullscreen: Atoms::intern_atom(display, "_NET_WM_STATE_FULLSCREEN"),
            net_wm_window_type: Atoms::intern_atom(display, "_NET_WM_WINDOW_TYPE"),
            net_wm_window_type_dialog: Atoms::intern_atom(display, "_NET_WM_WINDOW_TYPE_DIALOG"),
            net_client_list: Atoms::intern_atom(display, "_NET_CLIENT_LIST"),
        }
    }

    fn intern_atom(display: *mut xlib::Display, atom: &str) -> c_ulong {
        unsafe { xlib::XInternAtom(display, CString::new(atom).unwrap().as_ptr(), 0) }
    }
}

struct Colors {
    normal_border_color: c_ulong,
    focused_border_color: c_ulong,
}

impl Colors {
    fn new(display: *mut xlib::Display, window: c_ulong) -> Colors {
        unsafe {
            let normal_color = Colors::create_color(display, window, NORMAL_BORDER_COLOR);
            let focused_color = Colors::create_color(display, window, FOCUSED_BORDER_COLOR);
            Colors {
                normal_border_color: normal_color.pixel,
                focused_border_color: focused_color.pixel,
            }
        }
    }

    fn create_color(display: *mut xlib::Display, window: c_ulong, name: &str) -> xlib::XColor {
        unsafe {
            let mut color_def_screen: xlib::XColor = zeroed();
            let mut color_def_exact: xlib::XColor = zeroed();
            let visual = xlib::XDefaultVisual(display, 0);
            let color_map = xlib::XCreateColormap(display, window, visual, xlib::AllocNone);
            xlib::XAllocNamedColor(display,
                                   color_map,
                                   name.as_ptr() as *const i8,
                                   &mut color_def_screen,
                                   &mut color_def_exact);
            color_def_screen
        }
    }
}

struct WindowManager {
    display: *mut xlib::Display,
    screen: c_int,
    root: c_ulong,
    screen_width: c_int,
    screen_height: c_int,
    atoms: Atoms,
    current_tag: c_uchar,
    current_stack: ClientList,
    clients: ClientList,
    current_focus: Option<usize>,
    colors: Colors,
}

impl WindowManager {
    fn new() -> WindowManager {
        let display = unsafe { xlib::XOpenDisplay(null()) };
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let root = unsafe { xlib::XRootWindow(display, screen) };
        let width = unsafe { xlib::XDisplayWidth(display, screen) };
        let height = unsafe { xlib::XDisplayHeight(display, screen) };
        let atoms = Atoms::create_atom(display);
        let mut wm = WindowManager {
            display: display,
            screen: screen,
            root: root,
            screen_width: width,
            screen_height: height,
            atoms: atoms,
            current_tag: TAG_DEFAULT,
            current_stack: Vec::new(),
            current_focus: None,
            clients: Vec::new(),
            colors: Colors::new(display, root),
        };

        let net_atom_list = vec![wm.atoms.net_active_window,
                                 wm.atoms.net_client_list,
                                 wm.atoms.net_supported,
                                 wm.atoms.net_wm_fullscreen,
                                 wm.atoms.net_wm_name,
                                 wm.atoms.net_wm_state,
                                 wm.atoms.net_wm_state,
                                 wm.atoms.net_wm_window_type,
                                 wm.atoms.net_wm_window_type_dialog];
        unsafe {
            xlib::XChangeProperty(display,
                                  root,
                                  wm.atoms.net_supported,
                                  xlib::XA_ATOM,
                                  32,
                                  xlib::PropModeReplace,
                                  net_atom_list.as_ptr() as *mut u8,
                                  net_atom_list.len() as c_int);
            xlib::XDeleteProperty(display, root, wm.atoms.net_client_list);
            let mut xattr: xlib::XSetWindowAttributes = zeroed();
            xattr.cursor = xlib::XCreateFontCursor(display, XC_LEFT_PTR);
            xattr.event_mask =
                xlib::SubstructureNotifyMask | xlib::SubstructureRedirectMask |
                xlib::ButtonPressMask | xlib::PointerMotionMask |
                xlib::EnterWindowMask | xlib::LeaveWindowMask |
                xlib::StructureNotifyMask | xlib::PropertyChangeMask;
            xlib::XChangeWindowAttributes(display,
                                          root,
                                          xlib::CWEventMask | xlib::CWCursor,
                                          &mut xattr);
            xlib::XSelectInput(display, root, xattr.event_mask);
        }
        wm.grab_keys();
        wm
    }

    fn handle_event(&mut self, event: &xlib::XEvent) {
        match event.get_type() {
            xlib::ButtonPress => self.on_button_press(event),
            xlib::ClientMessage => self.on_client_message(event),
            xlib::ConfigureRequest => self.on_configure_request(event),
            xlib::DestroyNotify => self.on_destroy_notify(event),
            xlib::EnterNotify => self.on_enter_notify(event),
            xlib::Expose => self.on_expose_notify(event),
            xlib::FocusIn => self.on_focus_in(event),
            xlib::KeyPress => self.on_key_press(event),
            xlib::MappingNotify => self.on_mapping_notify(event),
            xlib::MapRequest => self.on_map_request(event),
            xlib::MotionNotify => self.on_motion_notify(event),
            xlib::PropertyNotify => self.on_property_notify(event),
            xlib::UnmapNotify => self.on_unmap_notify(event),
            _ => (),
        }
    }

    fn grab_keys(&mut self) {
        unsafe {
            let modifiers = vec![0, xlib::LockMask];
            xlib::XUngrabKey(self.display, xlib::AnyKey, xlib::AnyModifier, self.root);
            for &key in KEYS {
                let code = xlib::XKeysymToKeycode(self.display, key.1 as u64);
                for modifier in modifiers.iter() {
                    xlib::XGrabKey(self.display,
                                   code as i32,
                                   key.0 | modifier,
                                   self.root,
                                   1,
                                   xlib::GrabModeAsync,
                                   xlib::GrabModeAsync);
                }

            }
        }
    }

    fn grab_buttons(&mut self, client: ClientWindow, focused: bool) {
        unsafe {
            xlib::XUngrabButton(self.display,
                                xlib::AnyButton as c_uint,
                                xlib::AnyModifier,
                                client.borrow().window);
            if !focused {
                xlib::XGrabButton(self.display,
                                  xlib::AnyButton as c_uint,
                                  xlib::AnyModifier,
                                  client.borrow().window,
                                  0,
                                  (xlib::ButtonPressMask | xlib::ButtonReleaseMask) as c_uint,
                                  xlib::GrabModeAsync,
                                  xlib::GrabModeAsync,
                                  0,
                                  0);
            }
        }
    }

    fn send_event(&mut self, client: ClientWindow, proto: xlib::Atom) -> bool {
        let mut exists = false;
        unsafe {
            let mut n: c_int = 0;
            let mut p: *mut xlib::Atom = zeroed();
            if xlib::XGetWMProtocols(self.display, client.borrow().window, &mut p, &mut n) != 0 {
                let protocols: &[xlib::Atom] = std::slice::from_raw_parts(p, n as usize);
                exists = protocols.iter().any(|c| *c == proto);
                xlib::XFree(p as *mut c_void);
            }
            if exists {
                log!("Send event: {}", proto);
                let mut ev: xlib::XClientMessageEvent = zeroed();
                ev.type_ = xlib::ClientMessage; // wtf?
                ev.window = client.borrow().window;
                ev.message_type = self.atoms.wm_protocols;
                ev.format = 32;
                ev.data.set_long(0, proto as c_long);
                ev.data.set_long(1, xlib::CurrentTime as c_long);
                let status = xlib::XSendEvent(self.display,
                                              client.borrow().window,
                                              0,
                                              xlib::NoEventMask,
                                              &mut xlib::XEvent::from(ev));
                log!("Status: {}", status);
            }
        }
        exists
    }

    fn get_client(&mut self, window: c_ulong) -> Option<ClientWindow> {
        for c in &self.clients {
            if c.borrow().window == window {
                return Some(c.clone());
            }
        }
        None
    }

    fn get_client_index(&self, client: ClientWindow) -> usize {
        self.clients
            .iter()
            .position(|n| n.borrow().window == client.borrow().window)
            .unwrap()
    }

    fn update_client_list(&mut self) {
        unsafe {
            xlib::XDeleteProperty(self.display, self.root, self.atoms.net_client_list);
        }
        for c in &self.clients {
            unsafe {
                xlib::XChangeProperty(self.display,
                                      self.root,
                                      self.atoms.net_client_list,
                                      xlib::XA_WINDOW,
                                      32,
                                      xlib::PropModeAppend,
                                      &mut c.borrow_mut().window as *mut c_ulong as *mut c_uchar,
                                      1);
            }
        }
    }

    fn add_tag(&mut self, tag: c_uchar) {
        if let Some(focused) = self.current_focus {
            log!("Add tag: {}", tag);
            self.current_stack[focused].borrow_mut().tag = tag;
            self.select_tag(tag);
        }
    }

    fn select_tag(&mut self, tag: c_uchar) {
        self.current_tag = tag;
        self.set_focus(None);
        self.arrange_windows();
    }

    fn set_focus(&mut self, i: Option<usize>) {
        if let Some(prev_focused) = self.current_focus {
            if prev_focused >= self.current_stack.len() {
                return;
            }
            unsafe {
                let prev_client = self.current_stack[prev_focused].clone();
                xlib::XSetWindowBorder(self.display,
                                       prev_client.borrow().window,
                                       self.colors.normal_border_color);
                self.grab_buttons(prev_client, false);
            }
        }
        if let Some(c) = i {
            unsafe {
                let window = self.current_stack[c].borrow().window;
                xlib::XSetWindowBorder(self.display, window, self.colors.focused_border_color);
                xlib::XSetInputFocus(self.display,
                                     window,
                                     xlib::RevertToPointerRoot,
                                     xlib::CurrentTime);

                xlib::XChangeProperty(self.display,
                                      self.root,
                                      self.atoms.net_active_window,
                                      xlib::XA_WINDOW,
                                      32,
                                      xlib::PropModeReplace,
                                      &window as *const u64 as *const u8,
                                      1);
            }
            let client = self.current_stack[c].clone();
            let atom = self.atoms.wm_take_focus;
            self.grab_buttons(client.clone(), true);
            self.send_event(client, atom);
        }
        self.current_focus = i;
    }

    fn shift_focus(&mut self, inc: c_int) {
        let len = self.current_stack.len();
        if len == 0 {
            return;
        }
        if let Some(c) = self.current_focus {
            let next = (c as i32 + inc + len as i32) % len as i32;
            self.set_focus(Some(next as usize));
        } else {
            self.set_focus(None);
        }
    }

    fn kill_client(&mut self) {
        if let Some(c) = self.current_focus {
            let client: ClientWindow = self.current_stack[c].clone();
            let atom = self.atoms.wm_delete;
            log!("Killing client");
            if !self.send_event(client.clone(), atom) {
                log!("Not succeeded!");
                unsafe {
                    xlib::XGrabServer(self.display);
                    xlib::XSetErrorHandler(Some(xerror_dummy));
                    xlib::XSetCloseDownMode(self.display, xlib::DestroyAll);
                    xlib::XKillClient(self.display, client.borrow().window);
                    xlib::XSync(self.display, 0);
                    xlib::XSetErrorHandler(None);
                    xlib::XUngrabServer(self.display);
                }
            }
        }
    }

    fn zoom(&mut self) {
        if let Some(c) = self.current_focus {
            if c < self.current_stack.len() {
                let client = self.current_stack[c].clone();
                client.borrow_mut().weight = self.current_stack
                    .iter()
                    .map(|a| a.borrow().weight)
                    .max()
                    .unwrap() + 1;
                self.arrange_windows();
            }
        }
    }

    fn manage_window(&mut self, window: c_ulong, xa: &xlib::XWindowAttributes) {
        let client = Rc::new(RefCell::new(Client::new(window)));
        client.borrow_mut().tag = self.current_tag;
        client.borrow_mut().set_size(xa.x, xa.y, xa.width, xa.height);
        client.borrow_mut().save_window_size();
        unsafe {
            xlib::XChangeProperty(self.display,
                                  self.root,
                                  self.atoms.net_client_list,
                                  xlib::XA_WINDOW,
                                  32,
                                  xlib::PropModeAppend,
                                  &mut client.borrow_mut().window as *mut c_ulong as *mut u8,
                                  1);
            let mut wc: xlib::XWindowChanges = zeroed();
            wc.border_width = BORDER_WIDTH;
            xlib::XConfigureWindow(self.display, window, xlib::CWBorderWidth as u32, &mut wc);
            xlib::XSetWindowBorder(self.display, window, self.colors.normal_border_color);
            xlib::XSelectInput(self.display,
                               window,
                               xlib::EnterWindowMask | xlib::FocusChangeMask |
                               xlib::PropertyChangeMask |
                               xlib::StructureNotifyMask);
            self.grab_buttons(client.clone(), false);
            xlib::XMapWindow(self.display, window);
        }
        self.clients.push(client);
        self.arrange_windows();
    }

    fn unmanage(&mut self, client: ClientWindow, destroy: bool) {
        let index = self.get_client_index(client.clone());
        if !destroy {
            unsafe {
                xlib::XGrabServer(self.display);
                xlib::XSetErrorHandler(Some(xerror_dummy));
                self.set_client_state(client.clone(), WITHDRAWN_STATE);
                xlib::XSync(self.display, 0);
                xlib::XSetErrorHandler(None);
                xlib::XUngrabServer(self.display);
            }
        }
        self.clients.remove(index);
        self.update_client_list();
        self.arrange_windows();
    }

    fn resize_client(&mut self,
                     client: ClientWindow,
                     x: c_int,
                     y: c_int,
                     width: c_int,
                     height: c_int) {
        let mut xc: xlib::XWindowChanges = unsafe { zeroed() };
        client.borrow_mut().save_window_size();
        client.borrow_mut().set_size(x, y, width, height);
        xc.x = x;
        xc.y = y;
        xc.width = width;
        xc.height = height;
        unsafe {
            xlib::XConfigureWindow(self.display,
                                   client.borrow().window,
                                   (xlib::CWX | xlib::CWY | xlib::CWWidth | xlib::CWHeight) as u32,
                                   &mut xc);
        }
    }

    fn arrange_windows(&mut self) {
        let tag = self.current_tag;
        let clients = self.select_clients(tag);
        self.current_stack = clients;
        let focus = if self.current_stack.len() > 0 {
            Some(0)
        } else {
            None
        };
        self.set_focus(focus);
        let positions = tile(&self.current_stack,
                             0,
                             0,
                             self.screen_width,
                             self.screen_height);
        for i in 0..self.current_stack.len() {
            let client = self.current_stack[i].clone();
            self.resize_client(client,
                               positions[i].0,
                               positions[i].1,
                               positions[i].2,
                               positions[i].3);
        }

    }

    fn select_clients(&mut self, tag: c_uchar) -> ClientList {
        let tag = self.current_tag;
        let display = self.display;
        let mut result: ClientList = self.clients
            .iter()
            .filter_map(|c| {
                if c.borrow().tag == tag {
                    Some(c.clone())
                } else {
                    unsafe {
                        xlib::XMoveWindow(display,
                                          c.borrow().window,
                                          c.borrow().width * -2,
                                          c.borrow().y);
                    }
                    None
                }
            })
            .collect();

        for c in &result {
            unsafe {
                xlib::XMoveWindow(display, c.borrow().window, c.borrow().x, c.borrow().y);
            }
        }
        result.sort_by_key(|a| -a.borrow().weight);
        log!("Window length: {}", result.len());
        result
    }

    fn set_client_state(&self, client: ClientWindow, state: c_ulong) {
        let data = vec![state, 0];
        unsafe {
            xlib::XChangeProperty(self.display,
                                  client.borrow().window,
                                  self.atoms.wm_state,
                                  self.atoms.wm_state,
                                  32,
                                  xlib::PropModeReplace,
                                  data.as_ptr() as *const c_uchar,
                                  2);
        }
    }

    fn on_button_press(&mut self, event: &xlib::XEvent) {
        let button_event = xlib::XButtonPressedEvent::from(*event);
        let position =
            self.current_stack.iter().position(|x| x.borrow().window == button_event.window);
        if let Some(i) = position {
            self.set_focus(Some(i));
        }
    }

    fn on_client_message(&mut self, event: &xlib::XEvent) {
        log!("[on_client_message(): Not implemented]");
    }

    fn on_configure_request(&mut self, event: &xlib::XEvent) {
        let mut xa: xlib::XWindowChanges = unsafe { zeroed() };
        let configure_request_event = xlib::XConfigureRequestEvent::from(*event);
        xa.x = configure_request_event.x;
        xa.y = configure_request_event.y;
        xa.width = configure_request_event.width;
        xa.height = configure_request_event.height;
        xa.sibling = configure_request_event.above;
        xa.stack_mode = configure_request_event.detail;
        unsafe {
            xlib::XConfigureWindow(self.display,
                                   configure_request_event.window,
                                   configure_request_event.value_mask as c_uint,
                                   &mut xa);
            xlib::XSync(self.display, 0);
        }

    }

    fn on_destroy_notify(&mut self, event: &xlib::XEvent) {
        let destroy_window_event = xlib::XDestroyWindowEvent::from(*event);
        if let Some(c) = self.get_client(destroy_window_event.window) {
            self.unmanage(c.clone(), true);
        }
    }

    fn on_enter_notify(&mut self, event: &xlib::XEvent) {
        log!("[on_enter_notify() Not implemented]");
    }

    fn on_expose_notify(&mut self, event: &xlib::XEvent) {
        log!("[on_expose_notify() Not implemented]");
    }

    fn on_focus_in(&mut self, event: &xlib::XEvent) {
        log!("[on_focus_in() Not implemented]");
    }

    fn on_key_press(&mut self, event: &xlib::XEvent) {
        unsafe {
            let key_event = xlib::XKeyEvent::from(*event);
            let keysym = xlib::XKeycodeToKeysym(self.display, key_event.keycode as u8, 0);
            for &key in KEYS {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
        }
    }

    fn on_mapping_notify(&mut self, event: &xlib::XEvent) {
        log!("[on_mapping_notify() Not implemented]");
    }

    fn on_map_request(&mut self, event: &xlib::XEvent) {
        unsafe {
            let map_request_event = xlib::XMapRequestEvent::from(*event);
            let mut xa: xlib::XWindowAttributes = zeroed();
            if xlib::XGetWindowAttributes(self.display, map_request_event.window, &mut xa) == 0 ||
               xa.override_redirect != 0 {
                return;
            }
            if self.get_client(map_request_event.window).is_none() {
                self.manage_window(map_request_event.window, &xa);
            }
        }
    }

    fn on_motion_notify(&mut self, event: &xlib::XEvent) {
        log!("[on_motion_notify() Not implemented]");
    }

    fn on_property_notify(&mut self, event: &xlib::XEvent) {
        log!("[on_property_notify() Not implemented]");
    }

    fn on_unmap_notify(&mut self, event: &xlib::XEvent) {
        let unmap_event = xlib::XUnmapEvent::from(*event);
        if let Some(c) = self.get_client(unmap_event.window) {
            if unmap_event.send_event != 0 {
                self.set_client_state(c.clone(), WITHDRAWN_STATE);
            } else {
                self.set_focus(None);
                self.unmanage(c.clone(), false);
            }
        }
    }

    fn run(&mut self) {
        unsafe {
            let mut event: xlib::XEvent = zeroed();
            while xlib::XNextEvent(self.display, &mut event) == 0 {
                self.handle_event(&event);
            }
        }
    }
}

extern "C" fn xerror_dummy(display: *mut xlib::Display, event: *mut xlib::XErrorEvent) -> c_int {
    let e: xlib::XErrorEvent = unsafe { *event };
    log!("Got error {} from request {}", e.error_code, e.request_code);
    0
}

fn clean_mask(keycode: u32) -> u32 {
    keycode & !xlib::LockMask &
    (xlib::Mod1Mask | xlib::Mod2Mask | xlib::Mod3Mask | xlib::Mod4Mask | xlib::Mod5Mask |
     xlib::ShiftMask | xlib::ControlMask)
}

fn main() {
    let mut window_manager = WindowManager::new();
    window_manager.run();
}
