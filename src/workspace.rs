use std::collections::VecDeque;
use std::io::Write;
use std::mem::zeroed;
use std::os::raw::c_uchar;
use std::rc::Rc;

use client::{ClientW, Rect};
use config::Config;
use layout::Layout;
use util;

use x11::xlib;

pub enum FocusShift {
    Forward,
    Backward,
}

pub struct Workspace {
    pub config: Rc<Config>,
    client_current: Option<ClientW>,
    clients_prev: VecDeque<ClientW>,
    clients_next: VecDeque<ClientW>,
    layout: Box<Layout + 'static>,
    pub tag: c_uchar,
}

impl Workspace {
    pub fn new(config: Rc<Config>, tag: c_uchar, layout: Box<Layout + 'static>) -> Self {
        Workspace {
            client_current: None,
            clients_prev: VecDeque::new(),
            clients_next: VecDeque::new(),
            config: config,
            layout: layout,
            tag: tag,
        }
    }

    pub fn circle_focus(&mut self, direction: FocusShift) {
        match direction {
            FocusShift::Forward => {
                if self.clients_next.len() > 0 {
                    self.shift_focus(direction);
                } else {
                    while self.clients_prev.len() > 0 {
                        self.shift_focus(FocusShift::Backward);
                    }
                }
            }
            FocusShift::Backward => {
                if self.clients_prev.len() > 0 {
                    self.shift_focus(direction);
                } else {
                    while self.clients_next.len() > 0 {
                        self.shift_focus(FocusShift::Forward);
                    }
                }
            }
        }
        self.restack();
    }

    pub fn clear(&mut self) {
        self.clients_prev.clear();
        self.clients_next.clear();
        self.client_current = None;
    }

    pub fn detach_current(&mut self) -> Option<ClientW> {
        if self.client_current.is_some() {
            if self.clients_next.len() > 0 {
                self.shift_focus(FocusShift::Forward);
                return self.clients_prev.pop_back();
            } else if self.clients_prev.len() > 0 {
                self.shift_focus(FocusShift::Backward);
                return self.clients_next.pop_front();
            }
        }
        self.client_current.take()
    }

    pub fn get_client_by_window(&self, window: xlib::Window) -> Option<ClientW> {
        if let &Some(ref c) = &self.client_current {
            if c.window() == window {
                return Some(c.clone());
            }
        }

        for c in &self.clients_prev {
            if c.window() == window {
                return Some(c.clone());
            }
        }

        for c in &self.clients_next {
            if c.window() == window {
                return Some(c.clone());
            }
        }

        None
    }

    pub fn get_current_focused(&self) -> Option<ClientW> {
        if let &Some(ref c) = &self.client_current {
            return Some(c.clone());
        }
        None
    }

    pub fn get_layout(&self, rect: Rect) -> Vec<(ClientW, Rect)> {
        self.layout.layout(self, rect)
    }

    pub fn kill_client(&mut self) {
        if let Some(client) = self.detach_current() {
            let atom = client.atoms().wm_delete;
            if !client.send_event(atom) {
                x_disable_error_unsafe!(client.display(), {
                    xlib::XSetCloseDownMode(client.display(), xlib::DestroyAll);
                    xlib::XKillClient(client.display(), client.borrow().window);
                });
            }
        }
    }

    pub fn new_client(&mut self, client: ClientW, at_focus: bool) {
        if !at_focus {
            self.clients_prev.push_front(client);
            while self.clients_prev.len() > 0 {
                self.shift_focus(FocusShift::Backward);
            }
        } else {
            self.clients_prev.push_back(client);
            self.shift_focus(FocusShift::Backward);
        }
    }

    fn push_next(&mut self, client: Option<ClientW>) {
        if let Some(c) = client {
            self.clients_next.push_front(c);
        }
    }

    fn push_prev(&mut self, client: Option<ClientW>) {
        if let Some(c) = client {
            self.clients_prev.push_back(c);
        }
    }

    pub fn remove_client(&mut self, client: ClientW) {
        if let Some(position) = self.clients_prev
            .iter()
            .position(|c| c.window() == client.window()) {
            self.clients_prev.remove(position);
            return;
        }
        if let Some(position) = self.clients_next
            .iter()
            .position(|c| c.window() == client.window()) {
            self.clients_next.remove(position);
            return;
        }
        let cmp = self.client_current.clone().map(|c| c.window() == client.window());
        if cmp.is_some() && cmp.unwrap() {
            self.detach_current();
        }
    }

    pub fn restack(&mut self) {
        let mut wc: xlib::XWindowChanges = unsafe { zeroed() };
        if let Some(focus) = self.get_current_focused() {
            if focus.is_floating() {
                return;
            }
            unsafe {
                xlib::XLowerWindow(focus.display(), focus.window());
            }
            wc.sibling = focus.window();
            wc.stack_mode = xlib::Below;
            for c in self.select_clients(&|c| c.window() != focus.window()) {
                if !c.is_floating() {
                    unsafe {
                        xlib::XConfigureWindow(focus.display(),
                                               c.window(),
                                               xlib::CWSibling as u32 | xlib::CWStackMode as u32,
                                               &mut wc);
                    }
                    wc.sibling = c.window();
                }
            }
            unsafe {
                let mut xevent: xlib::XEvent = zeroed();
                xlib::XSync(focus.display(), 0);
                while xlib::XCheckMaskEvent(focus.display(),
                                            xlib::EnterWindowMask,
                                            &mut xevent) != 0 {}
            }
        }
    }

    pub fn set_focus(&mut self, client: ClientW) {
        if let &mut Some(ref mut c) = &mut self.client_current {
            if c.window() == client.window() {
                c.focus(true);
                c.grab_buttons(true);
                return;
            }
        }
        if let Some(position) = self.clients_prev
            .iter()
            .position(|c| c.window() == client.window()) {
            for _ in 0..self.clients_prev.len() - position {
                self.shift_focus(FocusShift::Backward);
            }
            return;
        }
        if let Some(position) = self.clients_next
            .iter()
            .position(|c| c.window() == client.window()) {
            for _ in 0..position + 1 {
                self.shift_focus(FocusShift::Forward);
            }
            return;
        }
    }

    pub fn select_clients(&self, pred: &Fn(&&ClientW) -> bool) -> Vec<ClientW> {
        let mut result: Vec<ClientW> =
            self.clients_prev.iter().filter(pred).map(|c| c.clone()).collect();
        if let &Some(ref c) = &self.client_current {
            if pred(&c) {
                result.push(c.clone());
            }
        }
        result.extend(self.clients_next
            .iter()
            .filter(pred)
            .map(|c| c.clone())
            .collect::<Vec<ClientW>>());
        result
    }

    pub fn show(&mut self, visible: bool) {
        for c in &mut self.clients_prev {
            c.show(visible);
        }

        if let &mut Some(ref mut c) = &mut self.client_current {
            c.show(visible);
        }

        for c in &mut self.clients_next {
            c.show(visible);
        }
    }

    pub fn shift_focus(&mut self, shift: FocusShift) {
        match shift {
            FocusShift::Forward => {
                if let Some(mut next_client) = self.clients_next.pop_front() {
                    let current = self.client_current.take();
                    next_client.focus(true);
                    next_client.grab_buttons(true);
                    self.client_current = Some(next_client);
                    self.push_prev(current.map(|c| {
                        c.focus(false);
                        c.clone().grab_buttons(false);
                        c
                    }));
                }
            }
            FocusShift::Backward => {
                if let Some(mut prev_client) = self.clients_prev.pop_back() {
                    let current = self.client_current.take();
                    prev_client.focus(true);
                    prev_client.grab_buttons(true);
                    self.client_current = Some(prev_client);
                    self.push_next(current.map(|c| {
                        c.focus(false);
                        c.clone().grab_buttons(false);
                        c
                    }));
                }
            }
        }
    }

    pub fn zoom(&mut self) {
        if let Some(c) = self.detach_current() {
            self.new_client(c, false);
        }
    }
}