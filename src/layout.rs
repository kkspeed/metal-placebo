use std::io::Write;
use std::rc::Rc;

use client::{ClientW, Rect};
use config::Config;
use core;
use workspace::Workspace;

pub trait Layout: LayoutClone {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)>;
    fn layout_rects(&self, config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect>;
    fn post(&self, message: &str, window_manager: &mut core::WindowManager) {}
}

pub trait LayoutClone {
    fn clone_layout(&self) -> Box<Layout>;
}

impl<T> LayoutClone for T
    where T: 'static + Layout + Clone
{
    fn clone_layout(&self) -> Box<Layout> {
        Box::new(self.clone())
    }
}

impl Clone for Box<Layout> {
    fn clone(&self) -> Box<Layout> {
        self.clone_layout()
    }
}

#[derive(Clone)]
pub struct Tile;

impl Layout for Tile {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_floating());
        let rects = self.layout_rects(current_workspace.config.clone(), clients.len(), rect);
        clients.into_iter().zip(rects).collect()
    }

    fn layout_rects(&self, config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect> {
        let mut result = vec![rect];
        let mut direction = 1;
        while result.len() < n {
            let r = result.pop().unwrap();
            if direction == 1 {
                let r1 = Rect::new(r.x, r.y, r.width / 2 - config.border_width, r.height);
                let r2 = Rect::new(r.x + r.width / 2 + config.border_width,
                                   r.y,
                                   r.width / 2 - config.border_width,
                                   r.height);
                if r1.width < 0 {
                    result.push(r.clone());
                    result.push(r);
                } else {
                    result.push(r1);
                    result.push(r2);
                }
            } else {
                let r2 = Rect::new(r.x,
                                   r.y + r.height / 2 + config.border_width,
                                   r.width,
                                   r.height / 2 - config.border_width);
                let r1 = Rect::new(r.x, r.y, r.width, r.height / 2 - config.border_width);
                if r2.height < 0 {
                    result.push(r.clone());
                    result.push(r);
                } else {
                    result.push(r1);
                    result.push(r2);
                }
            }
            direction ^= 1;
        }
        result
    }
}

#[derive(Clone)]
pub struct FullScreen;

impl Layout for FullScreen {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_floating());
        let rects = self.layout_rects(current_workspace.config.clone(), clients.len(), rect);
        clients.into_iter().zip(rects).collect()
    }

    fn layout_rects(&self, config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect> {
        vec![rect; n]
    }
}

#[derive(Clone)]
pub struct Overview;

impl Layout for Overview {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_sticky());
        let mut result = Vec::new();
        let rects = self.layout_rects(current_workspace.config.clone(), clients.len(), rect);
        for i in 0..clients.len() {
            result.push((clients[i].clone(), rects[i].clone()));
        }
        debug!("overview # of clients: {}", result.len());
        result
    }

    fn layout_rects(&self, config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect> {
        let mut result = vec![rect];
        let mut direction = 0;
        while result.len() < n {
            let mut tmp = Vec::new();
            for r in &result {
                if direction == 0 {
                    tmp.push(Rect::new(r.x,
                                       r.y,
                                       r.width / 2 - config.overview_inset,
                                       r.height - config.overview_inset));
                    tmp.push(Rect::new(r.x + r.width / 2 + config.overview_inset,
                                       r.y,
                                       r.width / 2 - config.overview_inset,
                                       r.height - config.overview_inset));
                } else {
                    tmp.push(Rect::new(r.x,
                                       r.y,
                                       r.width - config.overview_inset,
                                       r.height / 2 - config.overview_inset));
                    tmp.push(Rect::new(r.x,
                                       r.y + r.height / 2 + config.overview_inset,
                                       r.width - config.overview_inset,
                                       r.height / 2 - config.overview_inset));
                }
            }
            direction = direction ^ 1;
            result = tmp;
        }

        result
    }
}

#[derive(Clone)]
pub struct Tile13 {
    pub layout: Box<Layout>,
}

impl Layout for Tile13 {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_floating());
        let result = self.layout_rects(current_workspace.config.clone(), clients.len(), rect);
        clients.into_iter().zip(result).collect()
    }

    fn layout_rects(&self, config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect> {
        let mut result = Vec::new();
        if n == 1 {
            result.push(rect);
            return result;
        }
        if n > 1 {
            let first_rect = Rect::new(rect.x,
                                       rect.y,
                                       rect.width / 3 - config.border_width,
                                       rect.height);
            let other_rect = Rect::new(first_rect.x + first_rect.width + config.border_width,
                                       rect.y,
                                       rect.width - first_rect.width - config.border_width,
                                       rect.height);
            result.push(first_rect);
            result.extend(self.layout.layout_rects(config.clone(), n - 1, other_rect));
        }
        result
    }
}