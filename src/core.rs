use std::cmp;
use std::collections::HashMap;
use std::io::Write;
use std::mem::zeroed;
use std::os::raw::{c_int, c_long, c_uchar, c_uint, c_ulong};
use std::ptr::null;
use std::rc::Rc;

use x11::xlib;

use atoms;
use client::{ClientL, ClientW, Rect};
use config::*;
use util;
use util::clean_mask;
use layout::{FullScreen, Layout, Overview, Tile};
use loggers;
use loggers::Logger;
use workspace::{FocusShift, Workspace};
use xproto;

const TRACE: bool = true;

fn lookup_layout(config: Rc<Config>, tag: c_uchar) -> Box<Layout + 'static> {
    for &(ref t, ref l) in &config.tag_layout {
        if *t == tag {
            return l.clone();
        }
    }
    Box::new(Tile)
}

struct Colors {
    normal_border_color: c_ulong,
    focused_border_color: c_ulong,
}

impl Colors {
    fn new(config: Rc<Config>, display: *mut xlib::Display, window: c_ulong) -> Colors {
        let normal_color = Colors::create_color(display, window, config.normal_border_color);
        let focused_color = Colors::create_color(display, window, config.focused_border_color);
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

pub struct WindowManager {
    config: Rc<Config>,
    display: *mut xlib::Display,
    anchor_window: c_ulong,
    screen: c_int,
    root: c_ulong,
    screen_width: c_int,
    screen_height: c_int,
    pub current_tag: c_uchar,
    pub special_windows: ClientL,
    colors: Colors,
    pub workspaces: HashMap<c_uchar, Workspace>,
    logger: Box<Logger + 'static>,
}

impl WindowManager {
    pub fn new(cfg: Config) -> WindowManager {
        let config = Rc::new(cfg);
        let display = unsafe { xlib::XOpenDisplay(null()) };
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let root = unsafe { xlib::XRootWindow(display, screen) };
        let width = unsafe { xlib::XDisplayWidth(display, screen) };
        let height = unsafe { xlib::XDisplayHeight(display, screen) };
        atoms::create_atoms(display);
        let mut wm = WindowManager {
            config: config.clone(),
            display: display,
            anchor_window: 0,
            screen: screen,
            root: root,
            screen_width: width,
            screen_height: height,
            current_tag: config.tag_default,
            special_windows: Vec::new(),
            colors: Colors::new(config.clone(), display, root),
            logger: Box::new(loggers::DummyLogger::new(loggers::LoggerConfig::default())),
            workspaces: HashMap::new(),
        };

        wm.anchor_window = unsafe {
            xlib::XCreateSimpleWindow(wm.display,
                                      wm.root,
                                      0,
                                      0,
                                      1,
                                      1,
                                      0,
                                      wm.colors.normal_border_color,
                                      wm.colors.normal_border_color)
        };
        // Add workspaces.
        for tag in wm.config.tags {
            let w = Workspace::new(config.clone(),
                                   wm.anchor_window,
                                   *tag,
                                   config.get_description(*tag).map(|c| c.into()),
                                   lookup_layout(config.clone(), *tag));
            wm.workspaces.insert(*tag, w);
        }

        wm.workspaces.insert(TAG_OVERVIEW,
                             Workspace::new(config.clone(),
                                            wm.anchor_window,
                                            TAG_OVERVIEW,
                                            None,
                                            lookup_layout(config.clone(), TAG_OVERVIEW)));

        let net_atom_list = vec![atoms::net_active_window(),
                                 atoms::net_client_list(),
                                 atoms::net_supported(),
                                 atoms::net_wm_state_fullscreen(),
                                 atoms::net_wm_state_modal(),
                                 atoms::net_wm_state_above(),
                                 atoms::net_wm_name(),
                                 atoms::net_wm_state(),
                                 atoms::net_wm_state(),
                                 atoms::net_wm_window_type(),
                                 atoms::net_wm_window_type_dialog(),
                                 atoms::net_wm_window_type_dock()];
        unsafe {
            xlib::XChangeProperty(display,
                                  root,
                                  atoms::net_supported(),
                                  xlib::XA_ATOM,
                                  32,
                                  xlib::PropModeReplace,
                                  net_atom_list.as_ptr() as *mut u8,
                                  net_atom_list.len() as c_int);
            xlib::XDeleteProperty(display, root, atoms::net_client_list());
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
        wm.grab_keys();
        wm
    }

    fn handle_event(&mut self, event: &xlib::XEvent) {
        match event.get_type() {
            xlib::ButtonPress => self.on_button_press(event),
            xlib::ClientMessage => self.on_client_message(event),
            xlib::ConfigureRequest => self.on_configure_request(event),
            xlib::ConfigureNotify => self.on_configure_notify(event),
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
        let grab = |keys: &[(c_uint, c_uint, &Fn(&mut WindowManager))]| {
            let modifiers = vec![0, xlib::LockMask];
            for &key in keys {
                unsafe {
                    let code = xlib::XKeysymToKeycode(self.display, key.1 as c_ulong);
                    for modifier in modifiers.iter() {
                        xlib::XGrabKey(self.display,
                                       code as c_int,
                                       key.0 | modifier,
                                       self.root,
                                       1,
                                       xlib::GrabModeAsync,
                                       xlib::GrabModeAsync);
                    }
                }
            }
        };
        unsafe {
            xlib::XUngrabKey(self.display, xlib::AnyKey, xlib::AnyModifier, self.root);
        }
        grab(self.config.keys);
        grab(self.config.tag_keys);
        grab(self.config.add_keys);
    }

    fn update_client_list(&mut self) {
        unsafe {
            xlib::XDeleteProperty(self.display, self.root, atoms::net_client_list());
        }

        for (_, v) in self.workspaces.iter_mut() {
            if v.tag == TAG_OVERVIEW {
                continue;
            }
            for c in v.iter_mut() {
                unsafe {
                    xlib::XChangeProperty(self.display,
                                      self.root,
                                      atoms::net_client_list(),
                                      xlib::XA_WINDOW,
                                      32,
                                      xlib::PropModeAppend,
                                      &mut c.borrow_mut().window as *mut c_ulong as *mut c_uchar,
                                      1);
                }
            }
        }
    }

    pub fn set_logger(&mut self, logger: Box<Logger + 'static>) {
        self.logger = logger;
    }

    pub fn add_tag(&mut self, tag: c_uchar) {
        if self.current_tag != TAG_OVERVIEW {
            let current_client = {
                self.current_workspace_mut().detach_current()
            };
            if let Some(mut c) = current_client {
                let workspace = self.workspaces.get_mut(&tag).unwrap();
                c.borrow_mut().tag = tag;
                workspace.new_client(c, false);
            }
            self.select_tag(tag);
        }
    }

    pub fn select_tag(&mut self, tag: c_uchar) {
        if tag == TAG_OVERVIEW {
            self.current_tag = tag;
        } else {
            let mut sticky_clients = self.current_workspace().select_clients(&|c| c.is_sticky());
            for c in sticky_clients.iter_mut() {
                self.current_workspace_mut().remove_client(c.clone());
            }
            self.current_tag = tag;
            for c in sticky_clients.iter_mut() {
                c.borrow_mut().tag = tag;
                self.current_workspace_mut().new_client(c.clone(), true);
            }
        }
        unsafe {
            xlib::XSetInputFocus(self.display,
                                 self.root,
                                 xlib::RevertToPointerRoot,
                                 xlib::CurrentTime);
        }
        self.arrange_windows();
        if let Some(c) = self.current_focused() {
            self.set_focus(c);
        }
        self.do_log();
    }

    pub fn toggle_maximize(&mut self) {
        if self.current_tag != TAG_OVERVIEW {
            if let Some(mut c) = self.current_focused() {
                let maximized = c.is_maximized();
                c.set_maximized(!maximized);
                self.arrange_windows();
            }
        }
    }

    pub fn set_fullscreen(&mut self, client: ClientW, fullscreen: bool) {
        client.clone().set_fullscreen(Rect::new(0, 0, self.screen_width, self.screen_height),
                                      fullscreen);
        if !fullscreen {
            self.arrange_windows();
        }
    }

    pub fn set_focus_index(&mut self, index: Option<usize>) {
        let clients = self.current_clients();
        let i = index.unwrap_or(if clients.len() == 0 {
            0
        } else {
            clients.len() - 1
        });
        if let Some(c) = clients.get(i) {
            self.set_focus(c.clone());
        }
    }

    pub fn set_focus(&mut self, client: ClientW) {
        {
            let workspace = self.current_workspace_mut();
            workspace.set_focus(client.clone());
            workspace.restack();
        }
        //        client.send_event(atoms::wm_take_focus());
        self.do_log();
    }

    pub fn shift_focus(&mut self, inc: c_int) {
        {
            let workspace = self.current_workspace_mut();
            workspace.circle_focus(if inc > 0 {
                FocusShift::Forward
            } else {
                FocusShift::Backward
            });
        }
        self.do_log();
    }

    pub fn kill_client(&mut self) {
        {
            let workspace = self.current_workspace_mut();
            workspace.kill_client();
        }
        self.arrange_windows();
    }

    pub fn shift_window(&mut self, delta_x: c_int, delta_y: c_int) {
        if let Some(mut client) = self.current_focused() {
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

    pub fn expand_width(&self, delta: c_int) {
        if let Some(mut client) = self.current_focused() {
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

    pub fn expand_height(&self, delta: c_int) {
        if let Some(mut client) = self.current_focused() {
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

    pub fn zoom(&mut self) {
        {
            let workspace = {
                self.current_workspace_mut()
            };
            workspace.zoom();
        }
        self.arrange_windows();
    }

    pub fn current_focused(&self) -> Option<ClientW> {
        self.current_workspace().get_current_focused()
    }

    pub fn current_workspace(&self) -> &Workspace {
        self.workspaces.get(&self.current_tag).unwrap()
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace {
        self.workspaces.get_mut(&self.current_tag).unwrap()
    }

    pub fn all_clients(&self) -> Vec<ClientW> {
        let mut result = Vec::new();
        for (_, w) in &self.workspaces {
            if w.tag == TAG_OVERVIEW {
                continue;
            }
            result.extend(w.iter().cloned());
        }
        result
    }

    pub fn current_clients(&self) -> Vec<ClientW> {
        self.current_workspace().iter().cloned().collect()
    }

    pub fn get_client_by_window(&self, window: xlib::Window) -> Option<ClientW> {
        for (_, w) in &self.workspaces {
            if w.tag == TAG_OVERVIEW {
                continue;
            }
            if let Some(c) = w.get_client_by_window(window) {
                return Some(c);
            }
        }
        None
    }

    pub fn move_mouse(&mut self, client: &mut ClientW) {
        if client.is_fullscreen() {
            return;
        }

        let rect = client.get_rect();
        let mouse_mask = xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::PointerMotionMask;
        if unsafe {
            xlib::XGrabPointer(self.display,
                               self.root,
                               0,
                               mouse_mask as c_uint,
                               xlib::GrabModeAsync,
                               xlib::GrabModeAsync,
                               0,
                               0,
                               xlib::CurrentTime) == xlib::GrabSuccess
        } {
            if let Some((x, y)) = util::get_root_pointer(self.display, self.root) {
                let mut event: xlib::XEvent = unsafe { zeroed() };
                let mut last_time: xlib::Time = 0;
                loop {
                    unsafe {
                        xlib::XMaskEvent(self.display, mouse_mask, &mut event);
                        match event.get_type() {
                            xlib::Expose => self.on_expose_notify(&event),
                            xlib::MapRequest => self.on_map_request(&event),
                            xlib::ConfigureRequest => self.on_configure_request(&event),
                            xlib::MotionNotify => {
                                let me: xlib::XMotionEvent = xlib::XMotionEvent::from(event);
                                if me.time - last_time < 1000 / 60 {
                                    continue;
                                }
                                last_time = me.time;
                                let nx = rect.x + me.x - x;
                                let ny = rect.y + me.y - y;
                                if client.is_floating() {
                                    client.move_window(nx, ny, true);
                                }
                            }
                            xlib::ButtonRelease => break,
                            _ => (),
                        }
                    }
                }
            }
            unsafe {
                xlib::XUngrabPointer(self.display, xlib::CurrentTime);
            }
        }
    }

    fn resize_mouse(&mut self, client: &mut ClientW) {
        if client.is_fullscreen() {
            return;
        }

        let rect = client.get_rect();
        let mouse_mask = xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::PointerMotionMask;
        if unsafe {
            xlib::XGrabPointer(self.display,
                               self.root,
                               0,
                               mouse_mask as c_uint,
                               xlib::GrabModeAsync,
                               xlib::GrabModeAsync,
                               0,
                               0,
                               xlib::CurrentTime) == xlib::GrabSuccess
        } {
            unsafe {
                xlib::XWarpPointer(self.display,
                                   0,
                                   client.window(),
                                   0,
                                   0,
                                   0,
                                   0,
                                   rect.width + self.config.border_width - 1,
                                   rect.height + self.config.border_width - 1);
                xlib::XSync(self.display, 0);
            }
            let mut event: xlib::XEvent = unsafe { zeroed() };
            let mut last_time: xlib::Time = 0;
            loop {
                unsafe {
                    xlib::XMaskEvent(self.display, mouse_mask, &mut event);
                    match event.get_type() {
                        xlib::Expose => self.on_expose_notify(&event),
                        xlib::MapRequest => self.on_map_request(&event),
                        xlib::ConfigureRequest => self.on_configure_request(&event),
                        xlib::MotionNotify => {
                            let me: xlib::XMotionEvent = xlib::XMotionEvent::from(event);
                            if me.time - last_time < 1000 / 60 {
                                continue;
                            }
                            last_time = me.time;
                            let nw = cmp::max(me.x - 2 * self.config.border_width - rect.x + 1, 1);
                            let nh = cmp::max(me.y - 2 * self.config.border_width - rect.y + 1, 1);
                            if client.is_floating() {
                                client.resize(Rect::new(rect.x, rect.y, nw, nh), false);
                            }
                        }
                        xlib::ButtonRelease => break,
                        _ => (),
                    }
                }
            }
            unsafe {
                let rect = client.get_rect();
                xlib::XWarpPointer(self.display,
                                   0,
                                   client.window(),
                                   0,
                                   0,
                                   0,
                                   0,
                                   rect.width + self.config.border_width - 1,
                                   rect.height + self.config.border_width - 1);
                xlib::XUngrabPointer(self.display, xlib::CurrentTime);
            }
        }
    }

    fn manage_window(&mut self, window: c_ulong, xa: &xlib::XWindowAttributes) {
        let tag = if self.current_tag == TAG_OVERVIEW {
            self.config.tag_default
        } else {
            self.current_tag
        };
        let mut client = ClientW::new(self.config.clone(),
                                      self.display,
                                      self.root,
                                      window,
                                      self.anchor_window,
                                      self.current_tag);
        client.update_title();
        client.borrow_mut().tag = tag;
        client.borrow_mut().set_size(xa.x, xa.y, xa.width, xa.height);
        client.borrow_mut().save_window_size();
        client.set_border_color(self.colors.normal_border_color,
                                self.colors.focused_border_color);
        debug!("start to managing client: {}, window {}",
               client.get_title(),
               client.window());
        unsafe {
            xlib::XChangeProperty(self.display,
                                  self.root,
                                  atoms::net_client_list(),
                                  xlib::XA_WINDOW,
                                  32,
                                  xlib::PropModeAppend,
                                  &mut client.borrow_mut().window as *mut c_ulong as *mut u8,
                                  1);
            let mut wc: xlib::XWindowChanges = zeroed();
            wc.border_width = self.config.border_width;
            self.update_window_type(client.clone());
            if !(client.is_dock()) {
                xlib::XConfigureWindow(self.display,
                                       window,
                                       xlib::CWBorderWidth as c_uint,
                                       &mut wc);
                xlib::XSetWindowBorder(self.display, window, self.colors.normal_border_color);
            }
            xlib::XSelectInput(self.display,
                               window,
                               xlib::EnterWindowMask | xlib::FocusChangeMask |
                               xlib::PropertyChangeMask |
                               xlib::StructureNotifyMask);
            client.grab_buttons(false);
            xlib::XMapWindow(self.display, window);
        }

        for r in self.config.rules {
            if r.0(&client) {
                r.1(&mut client);
            }
        }

        if client.is_dock() {
            self.special_windows.push(client.clone());
        } else {
            {
                let workspace = self.workspaces.get_mut(&tag).unwrap();
                workspace.new_client(client.clone(), client.is_floating());
            }
            self.arrange_windows();
        }
    }

    fn unmanage(&mut self, client: ClientW, destroy: bool) {
        if let Some(c) = self.get_client_by_window(client.window()) {
            if !destroy {
                x_disable_error_unsafe!(self.display, {
                    client.clone().set_state(xproto::WITHDRAWN_STATE);
                });
            }
            {
                if self.current_tag == TAG_OVERVIEW {
                    let real_workspace = self.workspaces.get_mut(&c.tag()).unwrap();
                    real_workspace.remove_client(c.clone());
                }
                let workspace = self.current_workspace_mut();
                workspace.remove_client(c);
            }
            self.update_client_list();
            self.arrange_windows();
        }
    }

    fn do_log(&mut self) {
        let all_clients = self.all_clients();
        let current_clients = self.current_clients();
        let current_focused = self.current_focused();
        self.logger.dump(&self.config,
                         &self.workspaces,
                         &all_clients,
                         self.current_tag,
                         &current_clients,
                         current_focused);
    }

    fn arrange_windows(&mut self) {
        for (_, mut w) in self.workspaces.iter_mut() {
            let tag = w.tag;
            if tag == TAG_OVERVIEW {
                continue;
            }
            if tag != self.current_tag && self.current_tag != TAG_OVERVIEW {
                w.show(false);
            }
        }

        if self.current_tag == TAG_OVERVIEW {
            let clients = {
                let mut c = self.all_clients();
                c.sort_by_key(|c| -(c.tag() as c_int));
                c
            };
            let next_workspace = self.workspaces.get_mut(&self.current_tag).unwrap();
            next_workspace.clear();
            for c in clients {
                next_workspace.new_client(c.clone(), false);
            }
        }

        // TODO: Handle sticky windows as well
        let strategy = self.current_workspace()
            .get_layout(Rect::new(0,
                                  self.config.bar_height,
                                  self.screen_width - 2 * self.config.border_width,
                                  self.screen_height - self.config.bar_height -
                                  2 * self.config.border_width));
        for (mut c, r) in strategy {
            if self.current_tag == TAG_OVERVIEW {
                c.resize(r, true);
                continue;
            }
            let target_rect = if c.is_maximized() {
                Rect::new(0,
                          self.config.bar_height,
                          self.screen_width - 2 * self.config.border_width,
                          self.screen_height - self.config.bar_height -
                          2 * self.config.border_width)
            } else {
                r
            };
            c.resize(target_rect, false);
        }

        if self.current_tag != TAG_OVERVIEW {
            let mut floating_clients = self.current_workspace_mut()
                .select_clients(&|c| c.is_floating() == true);
            for fc in floating_clients.iter_mut() {
                let rect = fc.get_rect();
                fc.resize(rect, false);
                fc.raise_window();
            }
        }

        for c in self.current_clients() {
            if c.is_fullscreen() {
                c.raise_window();
            }
        }

        self.current_workspace_mut().restack();
    }

    fn update_window_type(&mut self, client: ClientW) {
        if let Some(state) = client.get_atom(atoms::net_wm_state()) {
            if state == atoms::net_wm_state_fullscreen() {
                debug!("update window {} to full screen.", client.get_title());
                self.set_fullscreen(client.clone(), true);
            }
            if state == atoms::net_wm_state_above() {
                debug!("update window {} above.", client.get_title());
                client.clone().set_above(true);
            }
            if state == atoms::net_wm_state_modal() {
                debug!("update window {} to modal.", client.get_title());
                client.clone().set_floating(true);
            }
        }
        if let Some(tp) = client.get_atom(atoms::net_wm_window_type()) {
            if tp == atoms::net_wm_window_type_dock() {
                debug!("found a dock window {}.", client.get_title());
                let mut c = client.clone();
                c.set_dock(true);
            }
        }
    }

    fn on_button_press(&mut self, event: &xlib::XEvent) {
        let button_event: xlib::XButtonPressedEvent = xlib::XButtonPressedEvent::from(*event);
        {
            let workspace = self.current_workspace_mut();
            if let Some(c) = workspace.get_client_by_window(button_event.window) {
                workspace.set_focus(c.clone());
                workspace.restack();
            }
        }
        if button_event.button == xlib::Button1 && button_event.state & MOD_MASK != 0 {
            if let Some(mut c) = self.current_workspace()
                .get_client_by_window(button_event.window) {
                self.move_mouse(&mut c);
            }
        }
        if button_event.button == xlib::Button3 && button_event.state & MOD_MASK != 0 {
            if let Some(mut c) = self.current_workspace()
                .get_client_by_window(button_event.window) {
                self.resize_mouse(&mut c);
            }
        }
    }

    fn on_client_message(&mut self, event: &xlib::XEvent) {
        let client_message: xlib::XClientMessageEvent = xlib::XClientMessageEvent::from(*event);
        debug!("client message atom {}, window: {}, state: {}, {}, {}",
               atoms::get_atom(client_message.message_type),
               client_message.window,
               client_message.data.get_long(0),
               client_message.data.get_long(1),
               client_message.data.get_long(2));
        if let Some(title) = util::get_text_prop(self.display,
                                                 client_message.window,
                                                 atoms::net_wm_name()) {
            debug!(" window {}, title: {}, state: {}, {}, {}",
                   client_message.window,
                   title,
                   client_message.data.get_long(0),
                   client_message.data.get_long(1),
                   client_message.data.get_long(2));
        }
        if let Some(c) = self.get_client_by_window(client_message.window) {
            if client_message.message_type == atoms::net_wm_state() {
                if client_message.data.get_long(1) == atoms::net_wm_state_fullscreen() as c_long ||
                   client_message.data.get_long(2) == atoms::net_wm_state_fullscreen() as c_long {
                    let fullscreen = client_message.data.get_long(0) == 1 ||
                                     (client_message.data.get_long(0) == 2 && !c.is_fullscreen());
                    debug!("client message: set_fullscreen: {}", fullscreen);
                    self.set_fullscreen(c.clone(), fullscreen);
                }
                if client_message.data.get_long(1) == atoms::net_wm_state_modal() as c_long ||
                   client_message.data.get_long(2) == atoms::net_wm_state_modal() as c_long {
                    debug!("set modal for client: {}", c.window());
                    c.clone().set_floating(true);
                    self.arrange_windows();
                }
            }
        }
    }

    fn on_configure_request(&mut self, event: &xlib::XEvent) {
        let mut xa: xlib::XWindowChanges = unsafe { zeroed() };
        let mut configure_request_event = xlib::XConfigureRequestEvent::from(*event);
        if let Some(mut c) = self.get_client_by_window(configure_request_event.window) {
            debug!("on_configure_request for window: {} ", c.get_title());
            if self.current_tag == TAG_OVERVIEW {
                xa.sibling = configure_request_event.above;
                xa.stack_mode = configure_request_event.detail;
                configure_request_event.value_mask &=
                    !((xlib::CWX | xlib::CWY | xlib::CWWidth | xlib::CWHeight) as c_ulong);
                unsafe {
                    xlib::XConfigureWindow(self.display,
                                           configure_request_event.window,
                                           configure_request_event.value_mask as c_uint,
                                           &mut xa);
                }
            } else if (c.is_sticky() || c.tag() == self.current_tag) && c.is_floating() {
                let mut rect = c.get_rect();
                if configure_request_event.value_mask & xlib::CWX as c_ulong != 0 {
                    rect.x = configure_request_event.x;
                }
                if configure_request_event.value_mask & xlib::CWY as c_ulong != 0 {
                    rect.y = configure_request_event.y;
                }
                if configure_request_event.value_mask & xlib::CWWidth as c_ulong != 0 {
                    rect.width = configure_request_event.width;
                }
                if configure_request_event.value_mask & xlib::CWHeight as c_ulong != 0 {
                    rect.height = configure_request_event.height;
                }
                c.resize(rect, false);
            } else {
                c.configure();
                let show = c.tag() == self.current_tag;
                c.show(show);
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

    fn on_configure_notify(&mut self, event: &xlib::XEvent) {
        let ce: xlib::XConfigureEvent = xlib::XConfigureEvent::from(*event);
        if ce.window == self.root &&
           (ce.width != self.screen_width || ce.height != self.screen_height) {
            self.screen_width = ce.width;
            self.screen_height = ce.height;
            self.arrange_windows();
        }
    }

    fn on_destroy_notify(&mut self, event: &xlib::XEvent) {
        let destroy_window_event = xlib::XDestroyWindowEvent::from(*event);
        if let Some(c) = self.get_client_by_window(destroy_window_event.window) {
            debug!("destroy window: {:x}, title: {}", c.window(), c.get_title());
            self.unmanage(c.clone(), true);
        }
    }

    fn on_enter_notify(&mut self, event: &xlib::XEvent) {
        debug!("[on_enter_notify]: not implemented!");
    }

    fn on_expose_notify(&mut self, event: &xlib::XEvent) {
        debug!("[on_expose_notify]: not implemented!");
    }

    fn on_focus_in(&mut self, event: &xlib::XEvent) {
        if let Some(client) = self.current_focused() {
            debug!("focus in for: {:x} title: {}",
                   client.window(),
                   client.get_title());
            if !client.is_floating() {
                self.current_workspace_mut().restack();
            }
            self.current_workspace_mut().set_focus(client);
            self.do_log();
        }
    }

    fn on_key_press(&mut self, event: &xlib::XEvent) {
        unsafe {
            let key_event = xlib::XKeyEvent::from(*event);
            let keysym = xlib::XKeycodeToKeysym(self.display, key_event.keycode as u8, 0);
            for &key in self.config.keys {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
            for &key in self.config.tag_keys {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
            for &key in self.config.add_keys {
                if key.1 == keysym as c_uint && clean_mask(key_event.state) == key.0 {
                    key.2(self);
                }
            }
        }
    }

    fn on_mapping_notify(&mut self, event: &xlib::XEvent) {
        debug!("[on_mapping_notify]");
        let mut mapping_event = xlib::XMappingEvent::from(*event);
        unsafe {
            xlib::XRefreshKeyboardMapping(&mut mapping_event);
        }
        if mapping_event.request == xlib::MappingKeyboard {
            self.grab_keys();
        }
    }

    fn on_map_request(&mut self, event: &xlib::XEvent) {
        unsafe {
            let map_request_event = xlib::XMapRequestEvent::from(*event);
            debug!("map request for window {}", map_request_event.window);
            let mut xa: xlib::XWindowAttributes = zeroed();
            if xlib::XGetWindowAttributes(self.display, map_request_event.window, &mut xa) == 0 ||
               xa.override_redirect != 0 {
                debug!("map request got override redirect");
                return;
            }
            if self.get_client_by_window(map_request_event.window).is_none() {
                self.manage_window(map_request_event.window, &xa);
            }
        }
    }

    fn on_motion_notify(&mut self, event: &xlib::XEvent) {
        // log!("[on_motion_notify() Not implemented]");
    }

    fn on_property_notify(&mut self, event: &xlib::XEvent) {
        let property_event = xlib::XPropertyEvent::from(*event);
        if let Some(mut c) = self.get_client_by_window(property_event.window).as_mut() {
            if property_event.atom == xlib::XA_WM_NAME ||
               property_event.atom == atoms::net_wm_name() {
                c.update_title();
                self.do_log();
            } else if property_event.atom == xlib::XA_WM_NORMAL_HINTS {
                if self.current_tag != TAG_OVERVIEW {
                    let tag = c.tag();
                    // ignore size hint for overview since the window sizes are
                    // temporary.
                    c.invalidate();
                    c.show(tag == self.current_tag);
                }
            } else if property_event.atom == atoms::net_wm_window_type() {
                self.update_window_type(c.clone());
            } else if property_event.atom == xlib::XA_WM_SIZE_HINTS {
                debug!("on_property_notify: received size hints from {}",
                       c.get_title());
            }
        }
    }

    fn on_unmap_notify(&mut self, event: &xlib::XEvent) {
        let unmap_event = xlib::XUnmapEvent::from(*event);
        if let Some(c) = self.get_client_by_window(unmap_event.window) {
            if unmap_event.send_event != 0 {
                c.clone().set_state(xproto::WITHDRAWN_STATE);
            } else {
                debug!("unmap notify: unmanage {}, window {}",
                       c.get_title(),
                       c.window());
                self.unmanage(c.clone(), false);
            }
        }
        self.do_log();
    }

    pub fn run(&mut self) {
        self.do_log();
        for prog in self.config.start_programs {
            prog();
        }
        unsafe {
            xlib::XSetErrorHandler(Some(util::xerror));
            let mut event: xlib::XEvent = zeroed();
            let display = self.display;
            while xlib::XNextEvent(display, &mut event) == 0 {
                {
                    self.handle_event(&event);
                }
            }
        }
    }
}
