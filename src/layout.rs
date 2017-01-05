use std::io::Write;
use std::rc::Rc;

use client::{ClientW, Rect};
use config::Config;
use core;
use workspace::Workspace;

pub trait Layout: LayoutClone {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)>;
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
        let mut result = Vec::new();
        if clients.len() > 0 {
            let mut direction = 1;
            result.push((clients[0].clone(), rect));
            for c in &clients.as_slice()[1..] {
                let (c_prev, r) = result.pop().unwrap();
                if direction == 1 {
                    let r1 = Rect::new(r.x,
                                       r.y,
                                       r.width / 2 - current_workspace.config.border_width,
                                       r.height);
                    let r2 = Rect::new(r.x + r.width / 2 + current_workspace.config.border_width,
                                       r.y,
                                       r.width / 2 - current_workspace.config.border_width,
                                       r.height);
                    if r1.width < 0 {
                        result.push((c_prev, r.clone()));
                        result.push((c.clone(), r));
                    } else {
                        result.push((c_prev, r1));
                        result.push((c.clone(), r2));
                    }
                } else {
                    let r2 = Rect::new(r.x,
                                       r.y + r.height / 2 + current_workspace.config.border_width,
                                       r.width,
                                       r.height / 2 - current_workspace.config.border_width);
                    let r1 = Rect::new(r.x,
                                       r.y,
                                       r.width,
                                       r.height / 2 - current_workspace.config.border_width);
                    if r2.height < 0 {
                        result.push((c_prev, r.clone()));
                        result.push((c.clone(), r));
                    } else {
                        result.push((c_prev, r1));
                        result.push((c.clone(), r2));
                    }
                }
                direction ^= 1;
            }
        }
        result
    }
}

#[derive(Clone)]
pub struct FullScreen;

impl Layout for FullScreen {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_floating());
        clients.iter().map(|c| (c.clone(), rect.clone())).collect()
    }
}

#[derive(Clone)]
pub struct Overview;

impl Layout for Overview {
    fn layout(&self, current_workspace: &Workspace, rect: Rect) -> Vec<(ClientW, Rect)> {
        let clients = current_workspace.select_clients(&|c| !c.is_sticky());
        let mut result = Vec::new();
        let rects = overview_rects(current_workspace.config.clone(), clients.len(), rect);
        for i in 0..clients.len() {
            result.push((clients[i].clone(), rects[i].clone()));
        }
        log!("Got overview clients: {}", result.len());
        result
    }
}

fn overview_rects(config: Rc<Config>, n: usize, rect: Rect) -> Vec<Rect> {
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
