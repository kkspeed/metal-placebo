use std::collections::VecDeque;
use std::io::Write;
use std::mem::zeroed;
use std::os::raw::{c_int, c_uchar, c_uint};
use std::rc::Rc;

use atoms;
use client::{ClientW, Rect};
use config::{Config, TAG_OVERVIEW};
use layout::Layout;
use util;

use x11::xlib;

pub enum FocusShift {
    Forward,
    Backward,
}

pub struct Workspace {
    anchor_window: xlib::Window,
    pub config: Rc<Config>,
    client_current: Option<ClientW>,
    clients_prev: VecDeque<ClientW>,
    clients_next: VecDeque<ClientW>,
    description: Option<String>,
    layout: Box<Layout + 'static>,
    pub rect: Rect,
    pub tag: c_uchar,
}

impl Workspace {
    pub fn new(
        config: Rc<Config>,
        anchor_window: xlib::Window,
        tag: c_uchar,
        description: Option<String>,
        layout: Box<Layout + 'static>,
        rect: Rect,
    ) -> Self {
        Workspace {
            anchor_window: anchor_window,
            client_current: None,
            clients_prev: VecDeque::new(),
            clients_next: VecDeque::new(),
            description: description,
            config: config,
            layout: layout,
            rect: rect,
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

    pub fn update_rect(&mut self, rect: Rect) {
        self.rect = rect;
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
        if let Some(c) = self.client_current.as_ref() {
            if c.window() == window {
                return Some(c.clone());
            }
        }
        self.clients_prev
            .iter()
            .find(|c| c.window() == window)
            .map(|c| c.clone())
            .or_else(|| {
                self.clients_next
                    .iter()
                    .find(|c| c.window() == window)
                    .map(|c| c.clone())
            })
    }

    pub fn get_current_focused(&self) -> Option<ClientW> {
        self.client_current.as_ref().map(|c| c.clone())
    }

    pub fn get_description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub fn get_layout(&self, rect: Rect) -> Vec<(ClientW, Rect)> {
        self.layout.layout(self, rect)
    }

    pub fn kill_client(&mut self) {
        self.client_current.as_mut().map(|client| {
            let atom = atoms::wm_delete();
            if !client.send_event(atom) {
                x_disable_error_unsafe!(client.display(), {
                    xlib::XSetCloseDownMode(client.display(), xlib::DestroyAll);
                    xlib::XKillClient(client.display(), client.window());
                });
            }
        });
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
        if let Some(position) = self.clients_prev.iter().position(
            |c| c.window() == client.window(),
        )
        {
            self.clients_prev.remove(position);
            return;
        }
        if let Some(position) = self.clients_next.iter().position(
            |c| c.window() == client.window(),
        )
        {
            self.clients_next.remove(position);
            return;
        }
        let cmp = self.client_current.clone().map(
            |c| c.window() == client.window(),
        );
        if cmp.is_some() && cmp.unwrap() {
            self.detach_current();
        }
    }

    pub fn restack(&mut self) {
        if let Some(focus) = self.get_current_focused() {
            focus.focus(true);
            if focus.is_floating() {
                focus.raise_window();
                return;
            }

            unsafe {
                let mut wc: xlib::XWindowChanges = zeroed();
                wc.stack_mode = xlib::Below;
                wc.sibling = self.anchor_window;
                xlib::XConfigureWindow(
                    focus.display(),
                    focus.window(),
                    xlib::CWSibling as c_uint | xlib::CWStackMode as c_uint,
                    &mut wc,
                );
                let mut xevent: xlib::XEvent = zeroed();
                xlib::XSync(focus.display(), 0);
                while xlib::XCheckMaskEvent(
                    focus.display(),
                    xlib::EnterWindowMask,
                    &mut xevent,
                ) != 0
                {}
            }
        }
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into());
    }

    pub fn set_focus(&mut self, client: ClientW) {
        if let &mut Some(ref mut c) = &mut self.client_current {
            if c.window() == client.window() {
                // c.focus(true);
                c.grab_buttons(true);
                return;
            }
        }
        if let Some(position) = self.clients_prev.iter().position(
            |c| c.window() == client.window(),
        )
        {
            for _ in 0..self.clients_prev.len() - position {
                self.shift_focus(FocusShift::Backward);
            }
            return;
        }
        if let Some(position) = self.clients_next.iter().position(
            |c| c.window() == client.window(),
        )
        {
            for _ in 0..position + 1 {
                self.shift_focus(FocusShift::Forward);
            }
            return;
        }
    }

    pub fn select_clients(&self, pred: &Fn(&ClientW) -> bool) -> Vec<ClientW> {
        self.iter().cloned().filter(pred).collect()
    }

    pub fn show(&mut self, visible: bool) {
        for c in self.iter_mut() {
            c.show(visible);
        }
    }

    pub fn shift_focus(&mut self, shift: FocusShift) {
        match shift {
            FocusShift::Forward => {
                if let Some(mut next_client) = self.clients_next.pop_front() {
                    let current = self.client_current.take();
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
        self.detach_current().map(|c| self.new_client(c, false));
    }

    pub fn arrange(&mut self) {
        let bar_height = if self.rect.x == 0 {
            self.config.bar_height
        } else {
            self.rect.y
        };

        // TODO: 1) Handle sticky windows as well
        //       2) Handle other multiple screen layout
        let strategy = self.get_layout(Rect::new(
            self.rect.x,
            bar_height,
            self.rect.width - 2 * self.config.border_width,
            self.rect.height - bar_height - 2 * self.config.border_width,
        ));
        for (mut c, r) in strategy {
            if self.tag == TAG_OVERVIEW {
                c.resize(r, true);
                continue;
            }
            let target_rect = if c.is_maximized() {
                Rect::new(
                    self.rect.x,
                    bar_height,
                    self.rect.width - 2 * self.config.border_width,
                    self.rect.height - bar_height - 2 * self.config.border_width,
                )
            } else {
                r
            };
            c.resize(target_rect, false);
        }

        if self.tag != TAG_OVERVIEW {
            let mut floating_clients = self.select_clients(&|c| c.is_floating() == true);
            for fc in floating_clients.iter_mut() {
                let rect = fc.get_rect();
                fc.resize(rect, false);
                fc.raise_window();
            }
        }

        for c in self.iter_mut() {
            if c.is_fullscreen() {
                c.raise_window();
            }
        }
    }

    pub fn iter(&self) -> WSIter {
        WSIter {
            current_client: self.client_current.as_ref(),
            prev_iter: Box::new(self.clients_prev.iter()),
            next_iter: Box::new(self.clients_next.iter()),
            current_returned: false,
        }
    }

    pub fn iter_mut(&mut self) -> WSIterMut {
        WSIterMut {
            current_client: self.client_current.as_mut(),
            prev_iter: Box::new(self.clients_prev.iter_mut()),
            next_iter: Box::new(self.clients_next.iter_mut()),
            current_returned: false,
        }
    }
}

pub struct WSIter<'a> {
    current_client: Option<&'a ClientW>,
    prev_iter: Box<Iterator<Item = &'a ClientW> + 'a>,
    next_iter: Box<Iterator<Item = &'a ClientW> + 'a>,
    current_returned: bool,
}

impl<'a> Iterator for WSIter<'a> {
    type Item = &'a ClientW;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_client) = self.prev_iter.next() {
            Some(next_client)
        } else if let Some(next_client) = self.next_iter.next() {
            Some(next_client)
        } else {
            self.current_client.take()
        }
    }
}

pub struct WSIterMut<'a> {
    current_client: Option<&'a mut ClientW>,
    prev_iter: Box<Iterator<Item = &'a mut ClientW> + 'a>,
    next_iter: Box<Iterator<Item = &'a mut ClientW> + 'a>,
    current_returned: bool,
}

impl<'a> Iterator for WSIterMut<'a> {
    type Item = &'a mut ClientW;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_client) = self.prev_iter.next() {
            Some(next_client)
        } else if let Some(next_client) = self.next_iter.next() {
            Some(next_client)
        } else {
            self.current_returned = true;
            self.current_client.take()
        }
    }
}
