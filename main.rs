extern crate libc;
extern crate x11;

use std::io::Write;
use std::mem::zeroed;
use std::os::raw::{c_int, c_uchar, c_uint, c_ulong};
use std::process;
use std::ptr::null;
use std::rc::Rc;

use x11::{xlib, keysym};

#[macro_use]
mod util;
mod atoms;
mod client;
mod xproto;

use atoms::Atoms;
use client::{ClientL, ClientList, ClientW, Rect};
use util::{clean_mask, spawn, Logger};

const FOCUSED_BORDER_COLOR: &'static str = "RGBi:0.0/1.0/1.0";
const NORMAL_BORDER_COLOR: &'static str = "RGBi:0.0/0.3/0.3";

const BORDER_WIDTH: c_int = 3;
const OVERVIEW_INSET: c_int = 15;
const BAR_HEIGHT: c_int = 15;

const WINDOW_MOVE_DELTA: c_int = 15;
const WINDOW_EXPAND_DELTA: c_int = 10;

const TRACE: bool = true;

type MyLogger = util::XMobarLogger;
const LOGGER_CONFIG: &'static util::LoggerConfig = &util::LoggerConfig {
    selected_tag_color: "#00FF00",
    tag_color: "#FFFFFF",
    separator_color: "#000000",
    selected_client_color: "#FFFF00",
    client_color: "#FFFFFF",
};

#[allow(unused_variables)]
const KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))] =
    &[(xlib::Mod1Mask, keysym::XK_r, &|w| spawn("dmenu_run", &[])),
      (xlib::Mod1Mask, keysym::XK_q, &|w| process::exit(0)),
      (xlib::Mod1Mask, keysym::XK_t, &|w| spawn("xterm", &[])),
      (xlib::Mod1Mask, keysym::XK_j, &|w| w.shift_focus(1)),
      (xlib::Mod1Mask, keysym::XK_k, &|w| w.shift_focus(-1)),
      (xlib::Mod1Mask, keysym::XK_F4, &|w| w.kill_client()),
      (xlib::Mod1Mask, keysym::XK_Left, &|w| w.shift_window(-WINDOW_MOVE_DELTA, 0)),
      (xlib::Mod1Mask, keysym::XK_Right, &|w| w.shift_window(WINDOW_MOVE_DELTA, 0)),
      (xlib::Mod1Mask, keysym::XK_Up, &|w| w.shift_window(0, -WINDOW_MOVE_DELTA)),
      (xlib::Mod1Mask, keysym::XK_Down, &|w| w.shift_window(0, WINDOW_MOVE_DELTA)),
      (xlib::Mod1Mask | xlib::ShiftMask,
       keysym::XK_Up,
       &|w| w.expand_height(-WINDOW_EXPAND_DELTA)),
      (xlib::Mod1Mask | xlib::ShiftMask,
       keysym::XK_Down,
       &|w| w.expand_height(WINDOW_EXPAND_DELTA)),
      (xlib::Mod1Mask | xlib::ShiftMask,
       keysym::XK_Right,
       &|w| w.expand_width(WINDOW_EXPAND_DELTA)),
      (xlib::Mod1Mask | xlib::ShiftMask,
       keysym::XK_Left,
       &|w| w.expand_width(-WINDOW_EXPAND_DELTA)),
      (xlib::Mod1Mask, keysym::XK_F2, &|w| w.select_tag(TAG_OVERVIEW)),
      (xlib::Mod1Mask,
       keysym::XK_Return,
       &|w| {
        if let Some(focus) = w.current_focus {
            if w.current_tag == TAG_OVERVIEW && w.current_stack.len() > focus {
                let client = w.current_stack[focus].clone();
                w.select_tag(client.tag());
            } else {
                w.zoom();
            }
        }
    })];

const TAG_KEYS: &'static [(c_uint, c_uint, &'static Fn(&mut WindowManager))] =
    &define_tags!(xlib::Mod1Mask,
                  xlib::ShiftMask,
                  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);

type LayoutFn = &'static Fn(&ClientL, usize, c_int, c_int, c_int, c_int)
                            -> Vec<(c_int, c_int, c_int, c_int)>;

const TAG_LAYOUT: &'static [(c_uchar, LayoutFn)] = &[('9' as c_uchar, &fullscreen),
                                                     (TAG_OVERVIEW, &overview)];

const TAG_DEFAULT: c_uchar = '1' as c_uchar;
const TAG_OVERVIEW: c_uchar = 0 as c_uchar;

const RULES: &'static [(&'static Fn(&ClientW) -> bool, &'static Fn(&mut ClientW))] =
    &[(&|c| c.get_class() == "Gimp", &|c| c.set_floating(true)),
      (&|c| c.is_dialog(), &|c| c.set_floating(true)),
      (&|c| c.get_class() == "Tilda", &|c| c.set_floating(true))];

const START_PROGRAMS: &'static [&'static Fn()] =
    &[&|| spawn("xcompmgr", &[]), &|| spawn("fcitx", &[]), &|| spawn("tilda", &["--hidden"])];

fn tile(clients: &ClientL,
        floating_len: usize,
        pane_x: c_int,
        pane_y: c_int,
        pane_width: c_int,
        pane_height: c_int)
        -> Vec<(c_int, c_int, c_int, c_int)> {
    let mut result =
        vec![(pane_x, pane_y, pane_width - 2 * BORDER_WIDTH, pane_height - 2 * BORDER_WIDTH)];
    let mut count = clients.len() - floating_len;
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

fn fullscreen(clients: &ClientL,
              floating_len: usize,
              pane_x: c_int,
              pane_y: c_int,
              pane_width: c_int,
              pane_height: c_int)
              -> Vec<(c_int, c_int, c_int, c_int)> {
    vec![(pane_x , pane_y , pane_width - BORDER_WIDTH, pane_height - BORDER_WIDTH); 
        clients.len() - floating_len]
}

fn overview(clients: &ClientL,
            floating_len: usize,
            pane_x: c_int,
            pane_y: c_int,
            pane_width: c_int,
            pane_height: c_int)
            -> Vec<(c_int, c_int, c_int, c_int)> {
    let l = clients.len();
    let mut result = vec![(pane_x, pane_y, pane_width, pane_height)];
    let mut direction = 0;
    while result.len() < l {
        let mut tmp = Vec::new();
        for &(x, y, width, height) in &result {
            if direction == 0 {
                tmp.push((x, y, width / 2 - OVERVIEW_INSET, height - OVERVIEW_INSET));
                tmp.push((x + width / 2 + OVERVIEW_INSET,
                          y,
                          width / 2 - OVERVIEW_INSET,
                          height - OVERVIEW_INSET));
            } else {
                tmp.push((x, y, width - OVERVIEW_INSET, height / 2 - OVERVIEW_INSET));
                tmp.push((x,
                          y + height / 2 + OVERVIEW_INSET,
                          width - OVERVIEW_INSET,
                          height / 2 - OVERVIEW_INSET));
            }
        }
        tmp.sort_by_key(|c| (c.1, c.0));
        direction = direction ^ 1;
        result = tmp;
    }

    result
}

fn lookup_layout(tag: c_uchar) -> Option<LayoutFn> {
    for &(t, f) in TAG_LAYOUT {
        if t == tag {
            return Some(f);
        }
    }
    None
}

struct Colors {
    normal_border_color: c_ulong,
    focused_border_color: c_ulong,
}

impl Colors {
    fn new(display: *mut xlib::Display, window: c_ulong) -> Colors {
        let normal_color = Colors::create_color(display, window, NORMAL_BORDER_COLOR);
        let focused_color = Colors::create_color(display, window, FOCUSED_BORDER_COLOR);
        Colors {
            normal_border_color: normal_color.pixel,
            focused_border_color: focused_color.pixel,
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
    atoms: Rc<Atoms>,
    current_tag: c_uchar,
    current_stack: ClientL,
    clients: ClientL,
    current_focus: Option<usize>,
    colors: Colors,
    logger: MyLogger,
}

impl WindowManager {
    fn new() -> WindowManager {
        let display = unsafe { xlib::XOpenDisplay(null()) };
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let root = unsafe { xlib::XRootWindow(display, screen) };
        let width = unsafe { xlib::XDisplayWidth(display, screen) };
        let height = unsafe { xlib::XDisplayHeight(display, screen) };
        let atoms = Rc::new(Atoms::create_atom(display));
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
            logger: MyLogger::new(&LOGGER_CONFIG),
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
            xattr.cursor = xlib::XCreateFontCursor(display, xproto::XC_LEFT_PTR);
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
        wm.logger.dump(&wm.clients,
                       wm.current_tag,
                       &wm.current_stack,
                       &wm.current_focus);
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
            for &key in TAG_KEYS {
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

    fn grab_buttons(&mut self, client: ClientW, focused: bool) {
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

    fn get_client_index(&self, client: ClientW) -> usize {
        self.clients
            .iter()
            .position(|n| n.borrow().window == client.borrow().window)
            .unwrap()
    }

    fn update_client_list(&mut self) {
        unsafe {
            xlib::XDeleteProperty(self.display, self.root, self.atoms.net_client_list);
        }
        for c in &mut self.clients {
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
        self.logger.dump(&self.clients,
                         self.current_tag,
                         &self.current_stack,
                         &self.current_focus);
    }

    fn set_focus(&mut self, i: Option<usize>) {
        if let Some(prev_focused) = self.current_focus {
            if prev_focused < self.current_stack.len() {
                let prev_client = self.current_stack[prev_focused].clone();
                prev_client.focus(false);
                self.grab_buttons(prev_client, false);
            }
        }
        if let Some(c) = i {
            let client = self.current_stack[c].clone();
            client.focus(true);
            client.raise_window();
            let atom = self.atoms.wm_take_focus;
            self.grab_buttons(client.clone(), true);
            client.send_event(atom);
        }
        self.current_focus = i;
        self.logger.dump(&self.clients,
                         self.current_tag,
                         &self.current_stack,
                         &self.current_focus);
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
            let client: ClientW = self.current_stack[c].clone();
            let atom = self.atoms.wm_delete;
            // TODO: REMOVE client record here already!
            if !client.send_event(atom) {
                log!("Force kill!");
                x_disable_error_unsafe!(self.display, {
                    xlib::XSetCloseDownMode(self.display, xlib::DestroyAll);
                    xlib::XKillClient(self.display, client.borrow().window);
                });
                log!("Force kill done!");
            }
        }
    }

    fn shift_window(&mut self, delta_x: c_int, delta_y: c_int) {
        if let Some(index) = self.current_focus {
            let mut client = self.current_stack[index].clone();
            if !client.is_floating() {
                return;
            }
            let rect = client.get_rect();
            let mut target_x = rect.x + delta_x;
            let mut target_y = rect.y + delta_y;
            if target_x < 0 {
                target_x = 0;
            }
            if target_x > self.screen_width - rect.width {
                target_x = self.screen_width - rect.width;
            }
            if target_y < 0 {
                target_y = 0;
            }
            if target_y > self.screen_height - rect.height {
                target_y = self.screen_height - rect.height;
            }
            client.move_window(target_x, target_y, true);
            unsafe {
                xlib::XSync(self.display, 0);
            }
        }
    }

    fn expand_width(&self, delta: c_int) {
        if let Some(index) = self.current_focus {
            let mut client = self.current_stack[index].clone();
            if !client.is_floating() {
                return;
            }
            let mut rect = client.get_rect();
            rect.width = delta + rect.width;

            if rect.width > self.screen_width {
                rect.width = self.screen_width;
            }
            if rect.width < 10 {
                return;
            }
            client.resize(rect, false);
        }
    }

    fn expand_height(&self, delta: c_int) {
        if let Some(index) = self.current_focus {
            let mut client = self.current_stack[index].clone();
            if !client.is_floating() {
                return;
            }
            let mut rect = client.get_rect();
            rect.height = delta + rect.height;

            if rect.height > self.screen_height {
                rect.height = self.screen_height;
            }

            if rect.height < 10 {
                return;
            }

            client.resize(rect, false);
        }
    }

    fn zoom(&mut self) {
        if let Some(c) = self.current_focus {
            if c < self.current_stack.len() {
                let mut client = self.current_stack[c].clone();
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
        let tag = if self.current_tag == TAG_OVERVIEW {
            TAG_DEFAULT
        } else {
            self.current_tag
        };
        let mut client = ClientW::new(self.display,
                                      self.root,
                                      window,
                                      self.current_tag,
                                      self.atoms.clone());
        client.update_title();
        client.borrow_mut().tag = tag;
        client.borrow_mut().set_size(xa.x, xa.y, xa.width, xa.height);
        client.borrow_mut().save_window_size();
        client.set_border_color(self.colors.normal_border_color,
                                self.colors.focused_border_color);
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

        for r in RULES {
            if r.0(&client) {
                r.1(&mut client);
            }
        }

        self.clients.push(client.clone());
        self.arrange_windows();
        client.raise_window();
        if let Some(index) = self.current_stack.iter().position(|c| c.window() == client.window()) {
            self.set_focus(Some(index));
        }
    }

    fn unmanage(&mut self, client: ClientW, destroy: bool) {
        let index = self.get_client_index(client.clone());
        if !destroy {
            x_disable_error_unsafe!(self.display, {
                client.clone().set_state(xproto::WITHDRAWN_STATE);
            });
        }
        self.clients.remove(index);
        self.update_client_list();
        self.arrange_windows();
    }

    fn arrange_windows(&mut self) {
        let tag = self.current_tag;
        self.current_stack = self.clients
            .select_clients(&|c| c.tag() == tag || tag == TAG_OVERVIEW,
                            true,
                            &|c| c.show(true),
                            &|c| c.show(false));
        let focus = if self.current_stack.len() > 0 {
            log!("Set focus to: 0, stack_len: {}", self.current_stack.len());
            Some(0)
        } else {
            log!("Set focus to: None, stack_len: {}",
                 self.current_stack.len());
            None
        };
        self.set_focus(focus);
        let floating_windows: ClientL = self.current_stack
            .iter()
            .filter_map(|c| if c.is_floating() {
                Some(c.clone())
            } else {
                None
            })
            .collect();
        let t = &tile;
        let layout_fn = lookup_layout(tag).unwrap_or(t);
        let positions = layout_fn(&self.current_stack,
                                  floating_windows.len(),
                                  0,
                                  BAR_HEIGHT,
                                  self.screen_width,
                                  self.screen_height - BAR_HEIGHT);

        for i in 0..self.current_stack.len() {
            let mut client = self.current_stack[i].clone();
            if self.current_tag == TAG_OVERVIEW {
                client.resize(Rect {
                                  x: positions[i].0,
                                  y: positions[i].1,
                                  width: positions[i].2,
                                  height: positions[i].3,
                              },
                              true);
            } else {
                if !client.is_floating() {
                    client.resize(Rect {
                                      x: positions[i].0,
                                      y: positions[i].1,
                                      width: positions[i].2,
                                      height: positions[i].3,
                                  },
                                  false);
                } else {
                    let rect = client.get_rect();
                    client.resize(rect, false);
                    unsafe {
                        client.raise_window();
                        xlib::XSync(self.display, 0);
                    }
                }
            }
        }
    }

    fn on_button_press(&mut self, event: &xlib::XEvent) {
        trace!("[on_button_press]");
        let button_event = xlib::XButtonPressedEvent::from(*event);
        let position =
            self.current_stack.iter().position(|x| x.borrow().window == button_event.window);
        if let Some(i) = position {
            self.set_focus(Some(i));
        }
    }

    fn on_client_message(&mut self, event: &xlib::XEvent) {
        trace!("[on_client_message]: not implemented!");
    }

    fn on_configure_request(&mut self, event: &xlib::XEvent) {
        trace!("[on_configure_request]");
        let mut xa: xlib::XWindowChanges = unsafe { zeroed() };
        let configure_request_event = xlib::XConfigureRequestEvent::from(*event);
        if let Some(mut c) = self.clients.get_client_by_window(configure_request_event.window) {
            if c.tag() == self.current_tag && c.is_floating() {
                c.show(true);
            } else {
                c.configure();
            }
        } else {
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
            }
        }

        unsafe {
            xlib::XSync(self.display, 0);
        }
    }

    fn on_destroy_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_destroy_notify]");
        let destroy_window_event = xlib::XDestroyWindowEvent::from(*event);
        if let Some(c) = self.clients.get_client_by_window(destroy_window_event.window) {
            self.unmanage(c.clone(), true);
        }
    }

    fn on_enter_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_enter_notify]: not implemented!");
    }

    fn on_expose_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_expose_notify]: not implemented!");
    }

    fn on_focus_in(&mut self, event: &xlib::XEvent) {
        trace!("[on_focus_in]");
        let focus = self.current_focus;
        self.set_focus(focus);
    }

    fn on_key_press(&mut self, event: &xlib::XEvent) {
        trace!("[on_key_press]");
        unsafe {
            let key_event = xlib::XKeyEvent::from(*event);
            let keysym = xlib::XKeycodeToKeysym(self.display, key_event.keycode as u8, 0);
            for &key in KEYS {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
            for &key in TAG_KEYS {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
        }
    }

    fn on_mapping_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_mapping_notify]");
        let mut mapping_event = xlib::XMappingEvent::from(*event);
        unsafe {
            xlib::XRefreshKeyboardMapping(&mut mapping_event);
        }
        if mapping_event.request == xlib::MappingKeyboard {
            self.grab_keys();
        }
    }

    fn on_map_request(&mut self, event: &xlib::XEvent) {
        trace!("[on_map_request]");
        unsafe {
            let map_request_event = xlib::XMapRequestEvent::from(*event);
            let mut xa: xlib::XWindowAttributes = zeroed();
            if xlib::XGetWindowAttributes(self.display, map_request_event.window, &mut xa) == 0 ||
               xa.override_redirect != 0 {
                return;
            }
            if self.clients.get_client_by_window(map_request_event.window).is_none() {
                self.manage_window(map_request_event.window, &xa);
            }
        }
    }

    fn on_motion_notify(&mut self, event: &xlib::XEvent) {
        // log!("[on_motion_notify() Not implemented]");
    }

    fn on_property_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_property_notify]");
        let property_event = xlib::XPropertyEvent::from(*event);
        if let Some(c) = self.clients.get_client_by_window(property_event.window) {
            if property_event.atom == xlib::XA_WM_NAME ||
               property_event.atom == self.atoms.net_wm_name {
                c.clone().update_title();
                self.logger.dump(&self.clients,
                                 self.current_tag,
                                 &self.current_stack,
                                 &self.current_focus);
            }
        }
    }

    fn on_unmap_notify(&mut self, event: &xlib::XEvent) {
        trace!("[on_unmap_notify]");
        let unmap_event = xlib::XUnmapEvent::from(*event);
        if let Some(c) = self.clients.get_client_by_window(unmap_event.window) {
            if unmap_event.send_event != 0 {
                c.clone().set_state(xproto::WITHDRAWN_STATE);
            } else {
                log!("From unmap notify!");
                self.set_focus(None);
                self.unmanage(c.clone(), false);
            }
        }
    }

    fn run(&mut self) {
        for prog in START_PROGRAMS {
            prog();
        }
        unsafe {
            let mut event: xlib::XEvent = zeroed();
            while xlib::XNextEvent(self.display, &mut event) == 0 {
                self.handle_event(&event);
            }
        }
    }
}

fn main() {
    let mut window_manager = WindowManager::new();
    window_manager.run();
}
